use render_core::{document::*, gltf_scene::*, render::*, sequence::*, *};
use std::sync::atomic::AtomicBool;
fn compare(cpu: &Image, gpu: &Image, normal_reference: &Image) {
    assert_eq!(cpu.objects, gpu.objects);
    for (a, b) in cpu.depth.iter().zip(&gpu.depth) {
        assert!(
            (a - b).abs() < 2e-5,
            "pass mismatch CPU {a} GPU {b} absolute {}",
            (a - b).abs()
        );
    }
    for (a, b) in normal_reference
        .normals
        .iter()
        .flatten()
        .zip(gpu.normals.iter().flatten())
    {
        assert!(
            (a - b).abs() < 2e-5,
            "pass mismatch CPU {a} GPU {b} absolute {}",
            (a - b).abs()
        );
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
#[ignore = "requires a real GPU; mandatory in morph frame acceptance"]
fn authored_morph_frames_match_metal_at_random_times_and_shutters() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let cancelled = AtomicBool::new(false);
    for bytes in [
        include_bytes!("../../../fixtures/morph-frames/data/dense.glb").as_slice(),
        include_bytes!("../../../fixtures/morph-frames/data/sparse.glb").as_slice(),
    ] {
        let mut d = Document::new(Snapshot::empty(Id(10000))).unwrap();
        let imported = import_pbr_glb(
            bytes,
            Id(10000),
            &PbrPolicy {
                allow_approximations: true,
            },
        )
        .unwrap();
        let clip = imported.report.source_clips[&0];
        let mut commands = imported.commands;
        let mut s = fixtures::settings();
        s.width = 24;
        s.height = 32;
        s.samples = 4;
        s.max_depth = 2;
        s.camera.position = [0., 0., 5.];
        s.camera.target = [0.; 3];
        s.camera.lens = Some(cameras::Lens::Orthographic {
            xmag: 2.,
            ymag: 2.5,
            near: 0.1,
            far: 10.,
        });
        commands.push(Command::SetRenderSettings { settings: s });
        d.execute(
            &fixtures::principal(),
            &fixtures::request(&d, "morph:gpu:import:01", commands).unwrap(),
        )
        .unwrap();
        let before = canonical(&d).unwrap();
        for time in [
            Time::new(1, 1).unwrap(),
            Time::new(1, 2).unwrap(),
            Time::new(0, 1).unwrap(),
        ] {
            for shutter in [
                Shutter::instant(),
                Shutter {
                    open: Time::new(-1, 8).unwrap(),
                    close: Time::new(1, 8).unwrap(),
                    samples: 3,
                },
            ] {
                let request = FrameRequest {
                    revision: d.snapshot().revision().unwrap(),
                    clip,
                    time,
                    shutter,
                };
                let cpu = render_frame(d.snapshot(), &request, || false).unwrap();
                let actual =
                    pollster::block_on(gpu.render_frame(d.snapshot(), &request, &cancelled))
                        .unwrap();
                assert!(cpu.image.depth.iter().any(|v| *v > 0.));
                eprintln!("time {:?}, shutter {:?}", time, request.shutter);
                let (mut nominal, _) = Evaluator::default()
                    .evaluate_at(d.snapshot(), clip, time, || false)
                    .unwrap();
                // Independent binary16 oracle for this constant normal map. All
                // RGB components lie in [0.5,1), whose binary16 spacing is 2^-11.
                // Preserve the strict 2e-5 direction check against that storage
                // profile, and compare original f32 CPU radiance separately.
                let normal_images = nominal
                    .instances
                    .iter()
                    .filter_map(|i| i.material.pbr.as_ref()?.normal.as_ref())
                    .map(|b| (b.image.clone(), b.role))
                    .collect::<std::collections::BTreeSet<_>>();
                for (key, pyramid) in &mut nominal.images {
                    if normal_images.contains(key) {
                        for level in &mut std::sync::Arc::make_mut(pyramid).levels {
                            for pixel in &mut level.rgba {
                                for value in &mut pixel[..3] {
                                    assert!((0.5..1.).contains(value));
                                    *value = (*value * 2048.).round() / 2048.;
                                }
                            }
                        }
                    }
                }
                let normal_reference = render(
                    &nominal,
                    d.snapshot().render_settings.as_ref().unwrap(),
                    || false,
                )
                .unwrap();
                compare(&cpu.image, &actual.image, &normal_reference);
            }
        }
        assert_eq!(canonical(&d).unwrap(), before);
    }
}
