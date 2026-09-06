use render_core::{document::*, render::*, *};
use std::sync::atomic::AtomicBool;
fn fixture() -> (Scene, Settings) {
    let mut doc = Document::new(Snapshot::empty(Id(9400))).unwrap();
    let commands = serde_json::from_slice(include_bytes!(
        "../../../fixtures/multibounce-pbr/create.json"
    ))
    .unwrap();
    doc.execute(
        &fixtures::principal(),
        &fixtures::request(&doc, "transport:fixture:01", commands).unwrap(),
    )
    .unwrap();
    (
        Evaluator::default().evaluate(doc.snapshot()).unwrap(),
        doc.snapshot().render_settings.clone().unwrap(),
    )
}
#[test]
fn gpu_depth_admission_and_unsupported_materials_are_explicit() {
    let (mut scene, mut settings) = fixture();
    for depth in [1, 2, 4, 16] {
        settings.max_depth = depth;
        assert_eq!(
            render_gpu::pack(&scene, &settings, 0).unwrap().params[0][3],
            depth as f32
        );
    }
    for depth in [0, 17] {
        settings.max_depth = depth;
        assert_eq!(
            render_gpu::pack(&scene, &settings, 0).err().unwrap().code,
            "render_settings"
        );
    }
    settings.max_depth = 4;
    scene.instances[0].material.pbr.as_mut().unwrap().advanced = Some(scattering::Surface {
        model: scattering::Model::Coated {
            weight: 0.5,
            ior: 1.5,
            roughness: 0.3,
        },
        opacity: scattering::Opacity::Opaque,
    });
    assert_eq!(
        render_gpu::pack(&scene, &settings, 0).err().unwrap().code,
        "unsupported_profile"
    );
}
#[test]
#[ignore = "requires a GPU adapter; run explicitly in the multi-bounce acceptance gate"]
fn gpu_multibounce_parity_progressive_identity_and_cancellation() {
    let (scene, mut settings) = fixture();
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let cancel = AtomicBool::new(false);
    for depth in [1, 2, 4, 16] {
        settings.max_depth = depth;
        let cpu = render(&scene, &settings, || false).unwrap();
        let image = pollster::block_on(gpu.render(&scene, &settings, 0, &cancel)).unwrap();
        let error = (cpu
            .linear
            .iter()
            .flatten()
            .zip(image.linear.iter().flatten())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            / (settings.width * settings.height * 3) as f32)
            .sqrt();
        assert!(error < 0.002, "depth {depth} RMSE {error}");
        assert_eq!(cpu.objects, image.objects);
        for (a, b) in cpu.depth.iter().zip(&image.depth) {
            assert!((a - b).abs() < 1e-5);
        }
        for (a, b) in cpu
            .normals
            .iter()
            .flatten()
            .zip(image.normals.iter().flatten())
        {
            assert!((a - b).abs() < 1e-5);
        }
        if depth == 1 {
            assert!(image.linear.iter().flatten().all(|x| *x == 0.));
        } else {
            assert!(image.linear.iter().map(|x| x[0]).sum::<f32>() > 10.);
        }
    }
    assert!(pollster::block_on(gpu.cancellation_fault_probe(&scene, &settings)).unwrap());
    // Sample a textured secondary emitter at nonconstant UVs. This exercises
    // the explicit LOD0 secondary policy and half-precision GPU texture packing.
    let mut textured = scene.clone();
    let asset = textures::ImageAsset::from_encoded(
        textures::Mime::Png,
        include_bytes!("../../../fixtures/static-pbr/BoxTextured/CesiumLogoFlat.png").to_vec(),
    )
    .unwrap();
    let key = digest(&canonical(&asset).unwrap());
    let role = textures::TextureRole::SrgbColor;
    textured.images.insert(
        (key.clone(), role),
        textures::Pyramid::new(&asset.decode().unwrap(), role).into(),
    );
    let ceiling = textured
        .instances
        .iter_mut()
        .find(|i| i.id == Id(2))
        .unwrap();
    ceiling.material.pbr.as_mut().unwrap().emission = Some(textures::Binding {
        uv_attribute: None,
        image: key,
        role,
        sampler: Default::default(),
    });
    for t in &mut std::sync::Arc::make_mut(&mut ceiling.geometry).triangles {
        t.uv = [[0., 0.].into(), [1., 0.].into(), [0., 1.].into()];
    }
    // Disposable geometry has a new identity so the GPU cache must upload it.
    textured.revision = "textured-secondary-fixture".into();
    settings.max_depth = 4;
    settings.light.intensity = [1.; 3];
    let cpu = render(&textured, &settings, || false).unwrap();
    let image = pollster::block_on(gpu.render(&textured, &settings, 0, &cancel)).unwrap();
    let rmse = (cpu
        .linear
        .iter()
        .flatten()
        .zip(image.linear.iter().flatten())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / (settings.width * settings.height * 3) as f32)
        .sqrt();
    assert!(rmse < 0.002, "textured secondary RMSE {rmse}");
    settings.light.intensity = [0.; 3];
    settings.max_depth = 4;
    settings.samples = 8;
    let mut progressive = render_gpu::Progressive::default();
    pollster::block_on(progressive.step(&mut gpu, &scene, &settings, &cancel)).unwrap();
    let two = pollster::block_on(progressive.step(&mut gpu, &scene, &settings, &cancel))
        .unwrap()
        .clone();
    settings.samples = 16;
    let full = pollster::block_on(gpu.render(&scene, &settings, 0, &cancel)).unwrap();
    for (a, b) in two
        .linear
        .iter()
        .flatten()
        .zip(full.linear.iter().flatten())
    {
        assert!((a - b).abs() < 2e-6);
    }
    assert!(two.receipt.approximation.contains("finite depth 4"));
    assert!(two.receipt.approximation.contains("GGX"));
    settings.max_depth = 2;
    let reset = pollster::block_on(progressive.step(&mut gpu, &scene, &settings, &cancel)).unwrap();
    assert_eq!(reset.receipt.samples, 16);
    assert_eq!(
        pollster::block_on(gpu.render(&scene, &settings, 0, &AtomicBool::new(true)))
            .unwrap_err()
            .code,
        "cancelled"
    );
}
