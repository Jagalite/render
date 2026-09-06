use render_core::{document::*, render::*, *};
use std::sync::atomic::AtomicBool;
fn document(bytes: &[u8]) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(9960))).unwrap();
    let i = gltf_scene::import_pbr_glb(
        bytes,
        Id(9960),
        &gltf_scene::PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "export:gpu:fixture:01", i.commands).unwrap(),
    )
    .unwrap();
    d
}
#[test]
#[ignore = "requires a real GPU; mandatory in evaluated GLB export acceptance"]
fn exported_pbr_surfaces_match_cpu_and_metal_at_static_and_animated_times() {
    let cases: [&[u8]; 3] = [
        include_bytes!("../../../fixtures/named-uv/data/roles.glb"),
        include_bytes!("../../../fixtures/vertex-colors/data/rgba.glb"),
        include_bytes!("../../../fixtures/morph-frames/data/dense.glb"),
    ];
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let cancel = AtomicBool::new(false);
    let mut s = fixtures::settings();
    s.width = 32;
    s.height = 24;
    s.samples = 8;
    s.max_depth = 2;
    s.camera.position = [0., 0., 3.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 2.5,
        ymag: 2.5,
        near: 0.1,
        far: 8.,
    });
    for (case, bytes) in cases.into_iter().enumerate() {
        let d = document(bytes);
        let at = d
            .snapshot()
            .animation
            .as_ref()
            .map(|a| gltf_export::Sample {
                clip: *a.clips.keys().next().unwrap(),
                time: Time::new(1, 2).unwrap(),
            });
        let q = gltf_export::Request {
            revision: d.snapshot().revision().unwrap(),
            at: at.clone(),
            policy: gltf_export::Policy {
                allow_approximations: true,
                ..Default::default()
            },
        };
        let exported = gltf_export::export(d.snapshot(), &q, || false).unwrap();
        let round = document(&exported.glb);
        let mut evaluator = Evaluator::default();
        let original = if let Some(at) = at {
            evaluator
                .evaluate_at(d.snapshot(), at.clip, at.time, || false)
                .unwrap()
                .0
        } else {
            evaluator.evaluate(d.snapshot()).unwrap()
        };
        let scene = Evaluator::default().evaluate(round.snapshot()).unwrap();
        let cpu = render(&scene, &s, || false).unwrap();
        let a = pollster::block_on(gpu.render(&original, &s, 0, &cancel)).unwrap();
        let b = pollster::block_on(gpu.render(&scene, &s, 0, &cancel)).unwrap();
        for (name, reference, tolerance) in
            [("GPU source", &a, 0.00002), ("CPU roundtrip", &cpu, 0.002)]
        {
            let rmse = (reference
                .linear
                .iter()
                .flatten()
                .zip(b.linear.iter().flatten())
                .map(|(x, y)| f64::from(x - y).powi(2))
                .sum::<f64>()
                / (s.width * s.height * 3) as f64)
                .sqrt();
            assert!(rmse < tolerance, "case{case} {name}: {rmse}");
            println!("case={case} reference={name} linear_rmse={rmse}");
            assert!(
                reference
                    .depth
                    .iter()
                    .zip(&b.depth)
                    .all(|(x, y)| (x - y).abs() < 2e-5)
            );
            assert!(
                reference
                    .normals
                    .iter()
                    .flatten()
                    .zip(b.normals.iter().flatten())
                    .all(|(x, y)| (x - y).abs() < 0.002)
            );
        }
        assert_eq!(cpu.objects, b.objects);
    }
    let d = document(include_bytes!("../../../fixtures/alpha-gltf/data/mask.glb"));
    let q = gltf_export::Request {
        revision: d.snapshot().revision().unwrap(),
        at: None,
        policy: gltf_export::Policy {
            allow_approximations: true,
            ..Default::default()
        },
    };
    let round = document(&gltf_export::export(d.snapshot(), &q, || false).unwrap().glb);
    let scene = Evaluator::default().evaluate(round.snapshot()).unwrap();
    let cpu = render(&scene, &s, || false).unwrap();
    let image = pollster::block_on(gpu.render(&scene, &s, 0, &cancel)).unwrap();
    assert_eq!(cpu.objects, image.objects);
    assert!(
        cpu.linear
            .iter()
            .flatten()
            .zip(image.linear.iter().flatten())
            .all(|(a, b)| (a - b).abs() < 0.002)
    );
}
