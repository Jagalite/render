use render_core::{document::*, render::*, sequence::*, *};
use std::sync::atomic::{AtomicBool, Ordering};
fn time(n: i64, d: u64) -> Time {
    Time::new(n, d).unwrap()
}
fn request(snapshot: &Snapshot, clip: Id) -> FrameRequest {
    FrameRequest {
        revision: snapshot.revision().unwrap(),
        clip,
        time: time(1, 2),
        shutter: Shutter {
            open: time(-1, 4),
            close: time(1, 4),
            samples: 4,
        },
    }
}
fn compare(cpu: &Image, gpu: &Image) {
    assert_eq!(cpu.objects, gpu.objects);
    for (a, b) in cpu.depth.iter().zip(&gpu.depth) {
        assert!((a - b).abs() < 2e-5);
    }
    for (a, b) in cpu
        .normals
        .iter()
        .flatten()
        .zip(gpu.normals.iter().flatten())
    {
        assert!((a - b).abs() < 2e-5);
    }
    let rmse = (cpu
        .linear
        .iter()
        .flatten()
        .zip(gpu.linear.iter().flatten())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / (cpu.width * cpu.height * 3) as f32)
        .sqrt();
    assert!(rmse < 0.002, "shutter RMSE {rmse}");
}
#[test]
#[ignore = "requires a real GPU; mandatory in GPU shutter acceptance"]
fn gpu_shutter_analytic_average_random_access_sequences_and_faults() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let cancelled = AtomicBool::new(false);
    let mut rigid = Document::new(Snapshot::empty(Id(9700))).unwrap();
    let mut commands: Vec<Command> =
        serde_json::from_slice(include_bytes!("../../../fixtures/project-cli/create.json"))
            .unwrap();
    let mut animation: Vec<Command> =
        serde_json::from_slice(include_bytes!("../../../fixtures/project-cli/animate.json"))
            .unwrap();
    animation.retain(|c| matches!(c, Command::SetAnimation { .. }));
    commands.extend(animation);
    rigid
        .execute(
            &fixtures::principal(),
            &fixtures::request(&rigid, "shutter:fixture:01", commands).unwrap(),
        )
        .unwrap();
    let character = feature_fixtures::character_document().unwrap();
    for (document, clip) in [(rigid, Id(10)), (character, Id(8400))] {
        let mut s = document.snapshot().clone();
        let settings = s.render_settings.as_mut().unwrap();
        settings.width = 24;
        settings.height = 24;
        settings.samples = 4;
        settings.max_depth = 2;
        let before = canonical(&s).unwrap();
        let req = request(&s, clip);
        let cpu = render_frame(&s, &req, || false).unwrap();
        let frame = pollster::block_on(gpu.render_frame(&s, &req, &cancelled)).unwrap();
        compare(&cpu.image, &frame.image);
        assert_eq!(cpu.temporal_times, frame.temporal_times);
        assert_eq!(
            canonical(&cpu.evaluation).unwrap(),
            canonical(&frame.evaluation).unwrap()
        );
        assert_eq!(cpu.image.receipt.revision, frame.image.receipt.revision);
        assert!(
            frame
                .image
                .receipt
                .backend
                .starts_with("gpu-f32-animation-v0/")
        );
        let mut instant = req.clone();
        instant.shutter = Shutter::instant();
        let (center, _) = Evaluator::default()
            .evaluate_at(&s, clip, req.time, || false)
            .unwrap();
        let static_image = pollster::block_on(gpu.render(
            &center,
            s.render_settings.as_ref().unwrap(),
            0,
            &cancelled,
        ))
        .unwrap();
        let sharp = pollster::block_on(gpu.render_frame(&s, &instant, &cancelled)).unwrap();
        assert_eq!(sharp.image.linear, static_image.linear);
        assert_eq!(frame.image.objects, sharp.image.objects);
        assert_eq!(frame.image.depth, sharp.image.depth);
        assert_eq!(frame.image.normals, sharp.image.normals);
        assert_ne!(
            frame.image.receipt.output_digest,
            sharp.image.receipt.output_digest
        );
        // Independent arithmetic reference: separate complete GPU images, f64
        // averaging on the host, compared to f32 accumulation on the device.
        let mut sums = vec![[0f64; 3]; frame.image.linear.len()];
        for at in &frame.temporal_times {
            let (scene, _) = Evaluator::default()
                .evaluate_at(&s, clip, *at, || false)
                .unwrap();
            let image = pollster::block_on(gpu.render(
                &scene,
                s.render_settings.as_ref().unwrap(),
                0,
                &cancelled,
            ))
            .unwrap();
            for (sum, p) in sums.iter_mut().zip(image.linear) {
                for c in 0..3 {
                    sum[c] += f64::from(p[c]) / 4.;
                }
            }
        }
        for (expected, actual) in sums
            .iter()
            .flatten()
            .zip(frame.image.linear.iter().flatten())
        {
            assert!((*expected - f64::from(*actual)).abs() < 2e-6);
        }
        for at in [time(1, 1), time(0, 1), time(1, 2)] {
            let mut sample = req.clone();
            sample.time = at;
            let image = pollster::block_on(gpu.render_frame(&s, &sample, &cancelled)).unwrap();
            if at == req.time {
                assert_eq!(image.image.linear, frame.image.linear);
            }
        }
        assert!(pollster::block_on(gpu.frame_cancellation_fault_probe(&s, &req)).unwrap());
        assert_eq!(
            pollster::block_on(gpu.render_frame(&s, &req, &cancelled))
                .unwrap()
                .image
                .linear,
            frame.image.linear
        );
        let seq = SequenceRequest {
            revision: req.revision.clone(),
            clip,
            times: vec![time(0, 1), time(1, 2), time(1, 1)],
            shutter: req.shutter.clone(),
        };
        let mut emitted = 0;
        let receipts = pollster::block_on(gpu.render_sequence(
            &s,
            &seq,
            |i, f| {
                assert_eq!(i, emitted);
                emitted += 1;
                if i == 1 {
                    assert_eq!(f.image.linear, frame.image.linear);
                }
                Ok(())
            },
            &cancelled,
        ))
        .unwrap();
        assert_eq!(emitted, 3);
        assert_eq!(receipts.len(), 3);
        emitted = 0;
        let error = pollster::block_on(gpu.render_sequence(
            &s,
            &seq,
            |_, _| {
                emitted += 1;
                cancelled.store(true, Ordering::Release);
                Ok(())
            },
            &cancelled,
        ))
        .unwrap_err();
        assert_eq!(error.code, "cancelled");
        assert_eq!(emitted, 1);
        assert_eq!(
            pollster::block_on(gpu.render_frame(&s, &req, &cancelled))
                .unwrap_err()
                .code,
            "cancelled"
        );
        cancelled.store(false, Ordering::Release);
        let e = pollster::block_on(gpu.render_sequence(
            &s,
            &seq,
            |_, _| Err(Error::new("consumer", "publication rejected")),
            &cancelled,
        ))
        .unwrap_err();
        assert_eq!(e.code, "consumer");
        let mut invalid = req.clone();
        invalid.revision = "stale".into();
        assert_eq!(
            pollster::block_on(gpu.render_frame(&s, &invalid, &cancelled))
                .unwrap_err()
                .code,
            "stale_revision"
        );
        invalid = req.clone();
        invalid.shutter.samples = 33;
        assert_eq!(
            pollster::block_on(gpu.render_frame(&s, &invalid, &cancelled))
                .unwrap_err()
                .code,
            "budget"
        );
        let mut invalid_seq = seq.clone();
        invalid_seq.times.reverse();
        assert_eq!(
            pollster::block_on(gpu.render_sequence(
                &s,
                &invalid_seq,
                |_, _| panic!("invalid sequence emitted"),
                &cancelled
            ))
            .unwrap_err()
            .code,
            "time"
        );
        let mut small = s.clone();
        small.render_settings.as_mut().unwrap().max_bytes = 24 * 24 * 128 - 1;
        let invalid = request(&small, clip);
        assert_eq!(
            pollster::block_on(gpu.render_frame(&small, &invalid, &cancelled))
                .unwrap_err()
                .code,
            "budget"
        );
        assert_eq!(canonical(&s).unwrap(), before);
    }
}
