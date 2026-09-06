#[path = "../../core/tests/support/dielectric.rs"]
mod fixture;
use render_core::{render::Evaluator, *};
use std::sync::atomic::AtomicBool;
#[test]
#[ignore = "requires actual GPU; mandatory ideal dielectric acceptance"]
fn analytic_ideal_interfaces_match_metal_and_keep_prior_shaders_exact() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let cancel = AtomicBool::new(false);
    for case in fixture::cases() {
        let scene = Evaluator::default()
            .evaluate(case.document.snapshot())
            .unwrap();
        let cpu = render::render(&scene, &case.settings, || false).unwrap();
        let image = pollster::block_on(gpu.render(&scene, &case.settings, 0, &cancel)).unwrap();
        println!(
            "{} expected={:?} gpu={:?} cpu={:?}",
            case.name, case.expected, image.linear[0], cpu.linear[0]
        );
        assert!(
            image.linear[0]
                .iter()
                .zip(case.expected)
                .all(|(a, b)| (a - b).abs() < 2e-5),
            "{}",
            case.name
        );
        assert_eq!(cpu.objects, image.objects);
        assert!((cpu.depth[0] - image.depth[0]).abs() < 2e-5);
        assert!(
            cpu.normals[0]
                .iter()
                .zip(image.normals[0])
                .all(|(a, b)| (a - b).abs() < 2e-5)
        );
        assert!(image.receipt.backend.starts_with("gpu-f32-dielectric-v1/"));
        assert_eq!(
            pollster::block_on(gpu.render(&scene, &case.settings, 0, &AtomicBool::new(true)))
                .unwrap_err()
                .code,
            "cancelled"
        );
        let mut bad = case.settings.clone();
        bad.max_bytes = 1;
        assert_eq!(
            pollster::block_on(gpu.render(&scene, &bad, 0, &cancel))
                .unwrap_err()
                .code,
            "budget"
        );
    }
    for (kernel, expected) in [
        (
            render_kernel::path::kernel(),
            "sha256:e0bae9a561867ddc4622b68e9d52635ce3d380bbf6f4aeb109e63b46c8d89247",
        ),
        (
            render_kernel::path::alpha_kernel(),
            "sha256:82177b8262a499bad4e9997fe15d04975b9635e0817ea4caeeb38d713a0648dc",
        ),
        (
            render_kernel::path::surface_kernel(),
            "sha256:cf5b558cef24671d760df416ccd1b82b7b8166bc26dbf2dfd4e2d3d616808f07",
        ),
    ] {
        assert_eq!(digest(kernel.generate().unwrap().as_bytes()), expected);
    }
}
#[test]
#[ignore = "requires actual GPU; mandatory mixed-surface dielectric acceptance"]
fn dielectric_variant_keeps_other_bsdfs_and_pipeline_cache_working() {
    use render_core::{
        document::*,
        render::*,
        scattering::{Model, Opacity, Surface},
    };
    let mut d = Document::new(Snapshot::empty(Id(9400))).unwrap();
    let commands = serde_json::from_slice(include_bytes!(
        "../../../fixtures/multibounce-pbr/create.json"
    ))
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "dielectric:mixed:source", commands).unwrap(),
    )
    .unwrap();
    let models = [
        Model::Dielectric { ior: 1.5 },
        Model::Conductor {
            eta: [0.2, 0.9, 1.1],
            k: [3., 2., 1.5],
        },
        Model::Coated {
            weight: 0.7,
            ior: 1.5,
            roughness: 0.25,
        },
        Model::Principled,
    ];
    let mut commands: Vec<Command> = d
        .snapshot()
        .materials
        .values()
        .enumerate()
        .map(|(i, m)| {
            let mut m = m.clone();
            let model = models[i % models.len()].clone();
            if matches!(model, Model::Dielectric { .. }) {
                m.roughness = 0.;
                m.metallic = 0.;
            }
            m.pbr.get_or_insert_with(pbr::Surface::default).advanced = Some(Surface {
                model,
                opacity: Opacity::Opaque,
            });
            Command::PutMaterial {
                material: Box::new(m),
            }
        })
        .collect();
    let mut legacy = d.snapshot().materials[&Id(103)].clone();
    legacy.id = Id(11001);
    let mut legacy_entity = d
        .snapshot()
        .entities
        .iter()
        .find(|e| e.material == Some(Id(103)))
        .unwrap()
        .clone();
    legacy_entity.id = Id(11002);
    legacy_entity.material = Some(legacy.id);
    legacy_entity.name = "legacy diffuse plate".into();
    legacy_entity.transform.columns[3][2] -= 0.3;
    commands.push(Command::PutMaterial {
        material: Box::new(legacy),
    });
    commands.push(Command::CreateEntity {
        entity: legacy_entity,
    });
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "dielectric:mixed:models", commands).unwrap(),
    )
    .unwrap();
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    for model in &models {
        assert!(scene.instances.iter().any(|i| {
            i.material
                .pbr
                .as_ref()
                .and_then(|p| p.advanced.as_ref())
                .is_some_and(|a| &a.model == model)
        }));
    }
    assert!(scene.instances.iter().any(|i| i.material.pbr.is_none()));
    let mut s = d.snapshot().render_settings.clone().unwrap();
    s.width = 16;
    s.height = 16;
    s.samples = 32;
    s.max_depth = 4;
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let cancel = AtomicBool::new(false);
    let cpu = render(&scene, &s, || false).unwrap();
    let before = pollster::block_on(gpu.render(&scene, &s, 0, &cancel)).unwrap();
    let rmse = (cpu
        .linear
        .iter()
        .flatten()
        .zip(before.linear.iter().flatten())
        .map(|(a, b)| f64::from(a - b).powi(2))
        .sum::<f64>()
        / 768.)
        .sqrt();
    println!("mixed RMSE={rmse}");
    assert!(rmse < 0.002);
    assert_eq!(cpu.objects, before.objects);
    for model in [
        None,
        Some(Model::Principled),
        Some(Model::Conductor {
            eta: [0.2; 3],
            k: [3.; 3],
        }),
    ] {
        let mut snapshot = d.snapshot().clone();
        for m in snapshot.materials.values_mut() {
            if let Some(p) = m.pbr.as_mut() {
                p.advanced = model.clone().map(|model| Surface {
                    model,
                    opacity: Opacity::Opaque,
                });
            }
        }
        let other = Evaluator::default().evaluate(&snapshot).unwrap();
        pollster::block_on(gpu.render(&other, &s, 0, &cancel)).unwrap();
    }
    let after = pollster::block_on(gpu.render(&scene, &s, 0, &cancel)).unwrap();
    assert_eq!(before.linear, after.linear);
    assert_eq!(
        canonical(&before.receipt).unwrap(),
        canonical(&after.receipt).unwrap()
    );
}
