use render_core::{
    document::*,
    render::*,
    scattering::{Model, Opacity, Surface},
    *,
};
use std::sync::atomic::{AtomicBool, Ordering};
fn document(bytes: &[u8]) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(10080))).unwrap();
    let imported = gltf_scene::import_pbr_glb(
        bytes,
        Id(10080),
        &gltf_scene::PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "surfaces:gpu:import", imported.commands).unwrap(),
    )
    .unwrap();
    d
}
fn configure(d: &mut Document, model: Option<Model>) {
    let commands = d
        .snapshot()
        .materials
        .values()
        .filter(|m| m.pbr.is_some())
        .map(|m| {
            let mut m = m.clone();
            let p = m.pbr.as_mut().unwrap();
            let opacity = p
                .advanced
                .as_ref()
                .map_or(Opacity::Opaque, |a| a.opacity.clone());
            p.advanced = model.clone().map(|model| Surface { model, opacity });
            Command::PutMaterial {
                material: Box::new(m),
            }
        })
        .collect();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(d, "surfaces:gpu:configure", commands).unwrap(),
    )
    .unwrap();
}
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 8;
    s.height = 8;
    s.samples = 32;
    s.max_depth = 2;
    s.camera.position = [0., 0., 1.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 0.9,
        ymag: 0.9,
        near: 0.01,
        far: 10.,
    });
    s.light.intensity = [0.; 3];
    s.environment = [0.; 3];
    s
}
fn compare(gpu: &mut render_gpu::Renderer, scene: &Scene, s: &Settings) -> Image {
    let cpu = render(scene, s, || false).unwrap();
    let image = pollster::block_on(gpu.render(scene, s, 0, &AtomicBool::new(false))).unwrap();
    let rmse = (cpu
        .linear
        .iter()
        .flatten()
        .zip(image.linear.iter().flatten())
        .map(|(x, y)| f64::from(x - y).powi(2))
        .sum::<f64>()
        / (s.width * s.height * 3) as f64)
        .sqrt();
    println!(
        "depth={} backend={} rmse={rmse}",
        s.max_depth, image.receipt.backend
    );
    assert!(rmse < 0.002, "{rmse}");
    assert_eq!(cpu.objects, image.objects);
    assert!(
        cpu.depth
            .iter()
            .zip(&image.depth)
            .all(|(a, b)| (a - b).abs() < 1e-4)
    );
    assert!(
        image
            .linear
            .iter()
            .flatten()
            .all(|x| x.is_finite() && *x >= 0.)
    );
    image
}
#[test]
#[ignore = "requires actual GPU; mandatory conductor/coat acceptance"]
fn conductor_coat_secondary_transport_and_three_pipeline_cache_switching() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let mut s = settings();
    let mut old = None;
    let models = [
        None,
        Some(Model::Principled),
        Some(Model::Conductor {
            eta: [0.2, 0.9, 1.1],
            k: [3., 2., 1.5],
        }),
        Some(Model::Coated {
            weight: 0.7,
            ior: 1.5,
            roughness: 0.25,
        }),
        Some(Model::Principled),
        None,
    ];
    for model in models {
        let mut d = document(include_bytes!(
            "../../../fixtures/secondary-textures/data/mask-nearest.glb"
        ));
        configure(&mut d, model.clone());
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        for depth in [1, 2, 4, 16] {
            s.max_depth = depth;
            let image = compare(&mut gpu, &scene, &s);
            if depth == 2 {
                assert!(image.linear.iter().flatten().any(|x| *x > 0.01));
                if model.is_none() {
                    if let Some(ref old) = old {
                        assert_eq!(old, &image.linear);
                    } else {
                        old = Some(image.linear);
                    }
                }
            }
        }
    }
}
#[test]
#[ignore = "requires actual GPU; mandatory analytic conductor/coat acceptance"]
fn normal_incidence_analytic_energy_and_coat_limits() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let mut s = settings();
    s.width = 1;
    s.height = 1;
    s.samples = 1;
    s.max_depth = 1;
    s.light.position = [0., 0., 1.];
    s.light.intensity = [4. * std::f32::consts::PI; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 1e-7,
        ymag: 1e-7,
        near: 0.01,
        far: 10.,
    });
    for (model, expected) in [
        (
            Model::Conductor {
                eta: [0.; 3],
                k: [1.; 3],
            },
            1.,
        ),
        (
            Model::Conductor {
                eta: [1.; 3],
                k: [0.; 3],
            },
            0.,
        ),
        (
            Model::Coated {
                weight: 1.,
                ior: 1.5,
                roughness: 1.,
            },
            0.04,
        ),
        (
            Model::Coated {
                weight: 0.,
                ior: 1.5,
                roughness: 1.,
            },
            0.,
        ),
        (
            Model::Coated {
                weight: 1.,
                ior: 1.,
                roughness: 1.,
            },
            0.,
        ),
    ] {
        let mut d = document(include_bytes!(
            "../../../fixtures/secondary-textures/data/mask-nearest.glb"
        ));
        configure(&mut d, Some(model.clone()));
        let commands = d
            .snapshot()
            .materials
            .values()
            .map(|m| {
                let mut m = m.clone();
                m.base_color = [0.; 3];
                m.emission = [0.; 3];
                m.roughness = 1.;
                m.metallic = 1.;
                if let Some(p) = m.pbr.as_mut() {
                    p.emission = None;
                }
                Command::PutMaterial {
                    material: Box::new(m),
                }
            })
            .collect();
        d.execute(
            &fixtures::principal(),
            &fixtures::request(&d, "surfaces:analytic:material", commands).unwrap(),
        )
        .unwrap();
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let image = compare(&mut gpu, &scene, &s);
        println!(
            "analytic {model:?} expected={expected} actual={:?}",
            image.linear[0]
        );
        assert!(image.linear[0].iter().all(|x| (*x - expected).abs() < 1e-5));
        let cancel = AtomicBool::new(true);
        assert_eq!(
            pollster::block_on(gpu.render(&scene, &s, 0, &cancel))
                .unwrap_err()
                .code,
            "cancelled"
        );
        cancel.store(false, Ordering::Release);
        let mut limited = s.clone();
        limited.max_bytes = 1;
        assert_eq!(
            pollster::block_on(gpu.render(&scene, &limited, 0, &cancel))
                .unwrap_err()
                .code,
            "budget"
        );
    }
}
#[test]
#[ignore = "requires actual GPU; mandatory texture and alpha integration acceptance"]
fn surface_models_keep_named_uv_vertex_color_normal_and_alpha_consumers() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let mut s = settings();
    s.width = 16;
    s.height = 16;
    s.samples = 16;
    s.max_depth = 4;
    s.camera.position = [0., 0., 3.];
    s.environment = [0.125, 0.25, 0.5];
    s.light.position = [1., 2., 3.];
    s.light.intensity = [1.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 2.5,
        ymag: 2.5,
        near: 0.1,
        far: 10.,
    });
    let cases: [&[u8]; 4] = [
        include_bytes!("../../../fixtures/named-uv/data/roles.glb"),
        include_bytes!("../../../fixtures/vertex-colors/data/rgba.glb"),
        include_bytes!("../../../fixtures/gpu-alpha/data/named-mask.glb"),
        include_bytes!("../../../fixtures/gpu-alpha/data/named-blend.glb"),
    ];
    for bytes in cases {
        for model in [
            Model::Conductor {
                eta: [0.2, 0.9, 1.1],
                k: [3., 2., 1.5],
            },
            Model::Coated {
                weight: 0.7,
                ior: 1.5,
                roughness: 0.25,
            },
        ] {
            let mut d = document(bytes);
            configure(&mut d, Some(model));
            let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
            compare(&mut gpu, &scene, &s);
        }
    }
}
