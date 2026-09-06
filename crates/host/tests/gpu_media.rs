#[path = "../../core/tests/support/gpu_media.rs"]
mod fixture;
use render_core::{render::*, *};
use std::sync::atomic::AtomicBool;
#[test]
#[ignore = "requires actual GPU; mandatory sparse media acceptance"]
fn analytic_sparse_media_matches_metal_and_preserves_cancellation() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    for case in fixture::cases() {
        let scene = Evaluator::default()
            .evaluate(case.document.snapshot())
            .unwrap();
        let cpu = render(&scene, &case.settings, || false).unwrap();
        let image =
            pollster::block_on(gpu.render(&scene, &case.settings, 0, &AtomicBool::new(false)))
                .unwrap();
        println!(
            "{} expected={:?} cpu={:?} gpu={:?}",
            case.name, case.expected, cpu.linear[0], image.linear[0]
        );
        for (a, b) in image.linear[0].iter().zip(case.expected) {
            assert!((a - b).abs() < 2e-5, "{}", case.name);
        }
        assert_eq!(image.objects, cpu.objects);
        assert_eq!(image.depth, cpu.depth);
        assert_eq!(image.normals, cpu.normals);
        assert!(
            image
                .receipt
                .backend
                .starts_with("gpu-f32-sparse-medium-v1/")
        );
        assert_eq!(
            pollster::block_on(gpu.render(&scene, &case.settings, 0, &AtomicBool::new(true)))
                .unwrap_err()
                .code,
            "cancelled"
        );
    }
}

#[test]
#[ignore = "requires actual GPU; mandatory media lifecycle and progressive identity"]
fn media_progressive_cancellation_and_device_recovery() {
    let case = fixture::cases().remove(0);
    let scene = Evaluator::default()
        .evaluate(case.document.snapshot())
        .unwrap();
    let settings = case.settings;
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    assert!(pollster::block_on(gpu.cancellation_fault_probe(&scene, &settings)).unwrap());
    let mut progressive = render_gpu::Progressive::default();
    let no = AtomicBool::new(false);
    let first = pollster::block_on(progressive.step(&mut gpu, &scene, &settings, &no))
        .unwrap()
        .clone();
    assert_eq!(
        pollster::block_on(progressive.step(&mut gpu, &scene, &settings, &AtomicBool::new(true)))
            .unwrap_err()
            .code,
        "cancelled"
    );
    let accumulated = pollster::block_on(progressive.step(&mut gpu, &scene, &settings, &no))
        .unwrap()
        .clone();
    assert_eq!(accumulated.receipt.samples, 2);
    let mut combined = settings.clone();
    combined.samples = 2;
    let direct = pollster::block_on(gpu.render(&scene, &combined, 0, &no)).unwrap();
    assert_eq!(accumulated.linear, direct.linear);
    assert_eq!(accumulated.objects, direct.objects);
    let mut changed = settings.clone();
    changed.environment = [0.; 3];
    let reset = pollster::block_on(progressive.step(&mut gpu, &scene, &changed, &no)).unwrap();
    assert_eq!(reset.receipt.samples, 1);
    assert_ne!(reset.linear, first.linear);
    gpu.destroy();
    assert!(pollster::block_on(gpu.render(&scene, &settings, 0, &no)).is_err());
    let mut recovered = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    assert_eq!(
        pollster::block_on(recovered.render(&scene, &settings, 0, &no))
            .unwrap()
            .linear,
        first.linear
    );
}

#[test]
fn sparse_media_admission_bounds_precision_and_surface_profiles() {
    use render_core::{scattering, volumes};
    let cases = fixture::cases();
    let mut scene = Evaluator::default()
        .evaluate(cases[0].document.snapshot())
        .unwrap();
    let mut settings = cases[0].settings.clone();
    let original = render_gpu::pack(&scene, &settings, 0).unwrap();
    assert_eq!(original.params[9][3], 4.);
    assert_eq!(original.params[11][1], 1.);
    let asset = cases[0]
        .document
        .snapshot()
        .volume_assets
        .values()
        .next()
        .unwrap()
        .as_ref()
        .clone();
    let transform = render_core::document::Transform::default()
        .affine()
        .unwrap();
    scene.media =
        volumes::Media::build((0..64).map(|i| (Id(i + 1), &asset, transform)), || false).unwrap();
    assert_eq!(
        render_gpu::pack(&scene, &settings, 0).unwrap().params[11][1],
        64.
    );
    scene.media =
        volumes::Media::build((0..65).map(|i| (Id(i + 1), &asset, transform)), || false).unwrap();
    assert_eq!(
        render_gpu::pack(&scene, &settings, 0).err().unwrap().code,
        "budget"
    );
    scene.media = volumes::Media::build(
        [(Id(1), &asset, transform), (Id(2), &asset, transform)],
        || false,
    )
    .unwrap();
    settings.width = 128;
    settings.height = 128;
    settings.samples = 32;
    settings.max_depth = 1;
    render_gpu::pack(&scene, &settings, 0).unwrap(); // Exactly 8,388,608 conservative visits.
    settings.width = 129;
    assert_eq!(
        render_gpu::pack(&scene, &settings, 0).err().unwrap().code,
        "budget"
    );
    settings = cases[0].settings.clone();
    let mut scattering = asset.clone();
    scattering.scattering = [0.1, 0., 0.];
    scene.media = volumes::Media::build([(Id(1), &scattering, transform)], || false).unwrap();
    assert_eq!(
        render_gpu::pack(&scene, &settings, 0).err().unwrap().code,
        "unsupported_profile"
    );
    let mut translated = render_core::document::Transform::default();
    translated.columns[3] = [100000000.3, 0., 0.];
    scene.media =
        volumes::Media::build([(Id(1), &asset, translated.affine().unwrap())], || false).unwrap();
    assert_eq!(
        render_gpu::pack(&scene, &settings, 0).err().unwrap().code,
        "precision"
    );
    let mut huge = render_core::document::Transform::default();
    for axis in 0..3 {
        huge.columns[axis][axis] = 2f64.powi(110);
    }
    scene.media =
        volumes::Media::build([(Id(1), &asset, huge.affine().unwrap())], || false).unwrap();
    assert_eq!(
        render_gpu::pack(&scene, &settings, 0).err().unwrap().code,
        "precision"
    );
    let mut tiny = asset.clone();
    tiny.absorption[0] = 1e-40;
    scene.media = volumes::Media::build([(Id(1), &tiny, transform)], || false).unwrap();
    assert_eq!(
        render_gpu::pack(&scene, &settings, 0).err().unwrap().code,
        "precision"
    );
    let (mut surfaces, _, _) = fixtures::diffuse_plane().unwrap();
    surfaces.media = Evaluator::default()
        .evaluate(cases[0].document.snapshot())
        .unwrap()
        .media;
    assert_eq!(
        render_gpu::pack(&surfaces, &settings, 0)
            .err()
            .unwrap()
            .code,
        "unsupported_profile"
    );
    surfaces.instances[0].material.pbr = Some(pbr::Surface::default());
    render_gpu::pack(&surfaces, &settings, 0).unwrap();
    surfaces.instances[0]
        .material
        .pbr
        .as_mut()
        .unwrap()
        .advanced = Some(scattering::Surface {
        model: scattering::Model::Principled,
        opacity: scattering::Opacity::Opaque,
    });
    assert_eq!(
        render_gpu::pack(&surfaces, &settings, 0)
            .err()
            .unwrap()
            .code,
        "unsupported_profile"
    );
}

#[test]
#[ignore = "requires actual GPU; mandatory surface/media integration"]
fn opaque_surfaces_light_shadows_and_finite_depth_media_match_reference() {
    use render_core::{document::*, volumes};
    let mut document = Document::new(Snapshot::empty(Id(10650))).unwrap();
    let commands = serde_json::from_slice(include_bytes!(
        "../../../fixtures/multibounce-pbr/create.json"
    ))
    .unwrap();
    document
        .execute(
            &fixtures::principal(),
            &fixtures::request(&document, "media:surface:source:001", commands).unwrap(),
        )
        .unwrap();
    let mut commands: Vec<_> = document
        .snapshot()
        .materials
        .values()
        .map(|m| {
            let mut m = m.clone();
            m.pbr.get_or_insert_with(pbr::Surface::default);
            Command::PutMaterial {
                material: Box::new(m),
            }
        })
        .collect();
    let asset = volumes::Asset {
        origin: [-2., -2., -0.2],
        voxel_size: [4., 4., 2.4],
        cells: vec![volumes::Cell {
            coordinate: [0; 3],
            density: 1.,
            emission: [0.01, 0.02, 0.03],
        }],
        absorption: [0.2, 0.4, 0.8],
        scattering: [0.; 3],
        anisotropy: 0.,
        max_step_meters: 0.1,
    };
    commands.extend([
        Command::CreateEntity {
            entity: Entity {
                id: Id(10651),
                name: "room medium".into(),
                parent: None,
                mesh: None,
                material: None,
                transform: Transform::default(),
            },
        },
        Command::SetVolume {
            entity: Id(10651),
            asset: Some(asset.content_id().unwrap()),
        },
        Command::PutVolume { asset },
    ]);
    document
        .execute(
            &fixtures::principal(),
            &fixtures::request(&document, "media:surface:author:001", commands).unwrap(),
        )
        .unwrap();
    let scene = Evaluator::default().evaluate(document.snapshot()).unwrap();
    let mut settings = document.snapshot().render_settings.clone().unwrap();
    settings.samples = 16;
    settings.environment = [0.1; 3];
    settings.light.position = [0.6, 0.3, 1.];
    settings.light.intensity = [2.; 3];
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    for depth in [1, 2, 4, 8] {
        settings.max_depth = depth;
        let cpu = render(&scene, &settings, || false).unwrap();
        let image =
            pollster::block_on(gpu.render(&scene, &settings, 0, &AtomicBool::new(false))).unwrap();
        let error = (cpu
            .linear
            .iter()
            .flatten()
            .zip(image.linear.iter().flatten())
            .map(|(a, b)| f64::from(a - b).powi(2))
            .sum::<f64>()
            / (settings.width * settings.height * 3) as f64)
            .sqrt();
        println!("surface depth{depth} RMSE {error}");
        assert!(error < 0.002, "depth {depth}");
        assert_eq!(image.objects, cpu.objects);
        let mut low = settings.clone();
        low.max_bytes = 1;
        assert_eq!(
            pollster::block_on(gpu.render(&scene, &low, 0, &AtomicBool::new(false)))
                .unwrap_err()
                .code,
            "budget"
        );
    }
}

#[test]
#[ignore = "requires actual GPU; execute occupied-cell and dispatch-work boundaries"]
fn media_maximum_cells_and_dispatch_work_execute() {
    use render_core::{document::Transform, volumes};
    let case = fixture::cases().remove(0);
    let mut scene = Evaluator::default()
        .evaluate(case.document.snapshot())
        .unwrap();
    let asset = case
        .document
        .snapshot()
        .volume_assets
        .values()
        .next()
        .unwrap()
        .as_ref()
        .clone();
    let transform = Transform::default().affine().unwrap();
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let no = AtomicBool::new(false);
    scene.media =
        volumes::Media::build((0..64).map(|i| (Id(i + 1), &asset, transform)), || false).unwrap();
    let cpu = render(&scene, &case.settings, || false).unwrap();
    let image = pollster::block_on(gpu.render(&scene, &case.settings, 0, &no)).unwrap();
    assert!(
        cpu.linear[0]
            .iter()
            .zip(image.linear[0])
            .all(|(a, b)| (a - b).abs() < 2e-5)
    );
    scene.media =
        volumes::Media::build((0..2).map(|i| (Id(i + 1), &asset, transform)), || false).unwrap();
    let expected = render(&scene, &case.settings, || false).unwrap();
    let mut settings = case.settings;
    settings.width = 128;
    settings.height = 128;
    settings.samples = 32;
    settings.max_depth = 1;
    let at = std::time::Instant::now();
    let stress = pollster::block_on(gpu.render(&scene, &settings, 0, &no)).unwrap();
    println!(
        "8388608 cell-visit dispatch seconds {}",
        at.elapsed().as_secs_f64()
    );
    assert_eq!(stress.linear.len(), 128 * 128);
    assert!(stress.linear.iter().all(|p| {
        p.iter()
            .zip(expected.linear[0])
            .all(|(a, b)| (a - b).abs() < 2e-5)
    }));
}
