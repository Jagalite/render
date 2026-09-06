use render_core::{document::*, render::*, scattering::Opacity, *};
use std::sync::atomic::AtomicBool;
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 16;
    s.height = 16;
    s.samples = 16;
    s.max_depth = 2;
    s.camera.position = [0., 0., 3.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 0.9,
        ymag: 0.9,
        near: 0.1,
        far: 8.,
    });
    s.light.intensity = [0.; 3];
    s.environment = [0.125, 0.25, 0.5];
    s
}
fn document(bytes: &[u8]) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(10060))).unwrap();
    let imported = gltf_scene::import_pbr_glb(
        bytes,
        Id(10060),
        &gltf_scene::PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "alpha:gpu:import:001", imported.commands).unwrap(),
    )
    .unwrap();
    d
}
fn planes(count: usize, opacity: Opacity) -> Document {
    let mut json: serde_json::Value =
        serde_json::from_slice(include_bytes!("../../../fixtures/named-uv/data/roles.gltf"))
            .unwrap();
    json["nodes"] = serde_json::json!([{"mesh":0}]);
    json["scenes"] = serde_json::json!([{"nodes":[0]}]);
    json["materials"] = serde_json::json!([{"pbrMetallicRoughness":{"baseColorFactor":[0.,0.,0.,1.],"metallicFactor":0,"roughnessFactor":0.5},"emissiveFactor":[1.,0.5,0.25],"doubleSided":true}]);
    let imported = gltf_scene::import_pbr_scene(
        &serde_json::to_vec(&json).unwrap(),
        &[include_bytes!("../../../fixtures/named-uv/data/roles.bin").to_vec()],
        &[],
        Id(10060),
        &gltf_scene::PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    let mut d = Document::new(Snapshot::empty(Id(10060))).unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "alpha:gpu:planes:001", imported.commands).unwrap(),
    )
    .unwrap();
    let template = d
        .snapshot()
        .entities
        .iter()
        .find(|e| e.mesh.is_some())
        .unwrap()
        .clone();
    let clones = (1..count)
        .map(|i| {
            let mut entity = template.clone();
            entity.id = Id(20000 + i as u128);
            entity.parent = None;
            entity.transform = Transform::default();
            entity.transform.columns[3][2] = -(i as f64) * 0.03;
            Command::CreateEntity { entity }
        })
        .collect::<Vec<_>>();
    if !clones.is_empty() {
        d.execute(
            &fixtures::principal(),
            &fixtures::request(&d, "alpha:gpu:copies:001", clones).unwrap(),
        )
        .unwrap();
    }
    let commands = d
        .snapshot()
        .materials
        .values()
        .filter(|m| m.emission == [1., 0.5, 0.25])
        .map(|m| {
            let mut material = m.clone();
            material.pbr.as_mut().unwrap().advanced = Some(scattering::Surface {
                model: scattering::Model::Principled,
                opacity: opacity.clone(),
            });
            Command::PutMaterial {
                material: Box::new(material),
            }
        })
        .collect();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "alpha:gpu:opacity:01", commands).unwrap(),
    )
    .unwrap();
    d
}
fn compare(gpu: &mut render_gpu::Renderer, scene: &Scene, s: &Settings, tolerance: f64) -> Image {
    let a = render_core::render::render(scene, s, || false).unwrap();
    let b = pollster::block_on(gpu.render(scene, s, 0, &AtomicBool::new(false))).unwrap();
    let rmse = (a
        .linear
        .iter()
        .flatten()
        .zip(b.linear.iter().flatten())
        .map(|(a, b)| f64::from(a - b).powi(2))
        .sum::<f64>()
        / (s.width * s.height * 3) as f64)
        .sqrt();
    println!("rmse={rmse} backend={}", b.receipt.backend);
    assert!(rmse < tolerance, "rmse={rmse}");
    assert_eq!(a.objects, b.objects);
    assert!(
        a.depth
            .iter()
            .zip(&b.depth)
            .all(|(a, b)| (a - b).abs() < 3e-5)
    );
    b
}
#[test]
#[ignore = "requires actual GPU; mandatory in alpha acceptance"]
fn coverage_textures_vertex_alpha_and_extended_schedule_match_cpu() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let mut s = settings();
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 2.5,
        ymag: 1.1,
        near: 0.1,
        far: 8.,
    });
    for bytes in [
        include_bytes!("../../../fixtures/alpha-gltf/data/mask.glb").as_slice(),
        include_bytes!("../../../fixtures/alpha-gltf/data/blend.glb").as_slice(),
        include_bytes!("../../../fixtures/gpu-alpha/data/named-mask.glb").as_slice(),
        include_bytes!("../../../fixtures/gpu-alpha/data/named-blend.glb").as_slice(),
        include_bytes!("../../../fixtures/gpu-alpha/data/ao-mask.glb").as_slice(),
    ] {
        let d = document(bytes);
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        compare(&mut gpu, &scene, &s, 0.002);
    }
    let d = document(include_bytes!(
        "../../../fixtures/vertex-colors/data/rgba.glb"
    ));
    for opacity in [
        Opacity::Opaque,
        Opacity::Mask {
            factor: 0.75,
            cutoff: 0.2,
        },
        Opacity::Blend { factor: 0.75 },
    ] {
        let mut snapshot = d.snapshot().clone();
        for m in snapshot.materials.values_mut() {
            m.pbr.as_mut().unwrap().advanced = Some(scattering::Surface {
                model: scattering::Model::Principled,
                opacity: opacity.clone(),
            });
        }
        for depth in [1, 2, 4] {
            s.max_depth = depth;
            let scene = Evaluator::default().evaluate(&snapshot).unwrap();
            compare(&mut gpu, &scene, &s, 0.002);
        }
    }
}
#[test]
#[ignore = "requires actual GPU; mandatory in alpha acceptance"]
fn mask_boundaries_camera_and_transparent_crossing_budgets() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let mut s = settings();
    for (factor, cutoff, covered) in [
        (0., 0., true),
        (0., 0.5, false),
        (1., 1., true),
        (1. - f64::EPSILON, 1., false),
        (0.5, 0.5, true),
        (1., f64::MAX, false),
    ] {
        let d = planes(1, Opacity::Mask { factor, cutoff });
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let image = compare(&mut gpu, &scene, &s, 0.00001);
        assert!(image.objects.iter().all(|o| o.is_some() == covered));
    }
    for count in [63, 64, 65] {
        let d = planes(
            count,
            Opacity::Mask {
                factor: 0.,
                cutoff: 0.5,
            },
        );
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let cpu = render_core::render::render(&scene, &s, || false);
        let actual = pollster::block_on(gpu.render(&scene, &s, 0, &AtomicBool::new(false)));
        if count > 64 {
            assert_eq!(cpu.unwrap_err().code, "budget");
            assert_eq!(actual.unwrap_err().code, "budget");
        } else {
            assert_eq!(cpu.unwrap().linear, actual.unwrap().linear);
        }
    }
    let d = planes(1, Opacity::Opaque);
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    for (near, far, covered) in [(0.1, 2., false), (3.1, 8., false), (0.1, 3., true)] {
        s.camera.lens = Some(cameras::Lens::Orthographic {
            xmag: 0.9,
            ymag: 0.9,
            near,
            far,
        });
        let image = compare(&mut gpu, &scene, &s, 0.00001);
        assert!(image.objects.iter().all(|o| o.is_some() == covered));
    }
    assert_eq!(
        pollster::block_on(gpu.render(&scene, &s, 0, &AtomicBool::new(true)))
            .unwrap_err()
            .code,
        "cancelled"
    );
    s.max_bytes = 1;
    assert_eq!(
        pollster::block_on(gpu.render(&scene, &s, 0, &AtomicBool::new(false)))
            .unwrap_err()
            .code,
        "budget"
    );
}
fn shadow_document(count: usize, opacity: Opacity) -> Document {
    let mut d = planes(count + 1, opacity);
    let mut leaves = d
        .snapshot()
        .entities
        .iter()
        .filter(|e| e.mesh.is_some())
        .cloned()
        .collect::<Vec<_>>();
    leaves.sort_by(|a, b| b.transform.columns[3][2].total_cmp(&a.transform.columns[3][2]));
    let mut receiver = d.snapshot().materials[&leaves[0].material.unwrap()].clone();
    receiver.id = Id(30001);
    receiver.base_color = [0.5; 3];
    receiver.emission = [0.; 3];
    receiver.pbr.as_mut().unwrap().advanced = Some(scattering::Surface {
        model: scattering::Model::Principled,
        opacity: Opacity::Opaque,
    });
    let mut commands = vec![
        Command::PutMaterial {
            material: Box::new(receiver.clone()),
        },
        Command::SetMaterial {
            entity: leaves[0].id,
            material: receiver.id,
        },
    ];
    for (i, e) in leaves.into_iter().enumerate().skip(1) {
        let mut transform = Transform::default();
        transform.columns[0][0] = 100.;
        transform.columns[1][1] = 100.;
        transform.columns[3][2] = i as f64 * 0.02;
        commands.push(Command::SetTransform {
            entity: e.id,
            transform,
        });
    }
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "alpha:gpu:shadows:01", commands).unwrap(),
    )
    .unwrap();
    d
}
#[test]
#[ignore = "requires actual GPU; mandatory in alpha acceptance"]
fn transparent_shadows_are_products_and_visibility_budget_errors() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let mut s = settings();
    s.width = 4;
    s.height = 4;
    s.samples = 4;
    s.max_depth = 1;
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 0.5,
        ymag: 0.5,
        near: 2.99,
        far: 8.,
    });
    s.environment = [0.; 3];
    s.light.position = [0., 0., 3.];
    s.light.intensity = [4.; 3];
    let d = shadow_document(0, Opacity::Opaque);
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let reference = compare(&mut gpu, &scene, &s, 0.00001);
    for count in [1, 2, 4] {
        let d = shadow_document(count, Opacity::Blend { factor: 0.25 });
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let image = compare(&mut gpu, &scene, &s, 0.00001);
        for (a, b) in image
            .linear
            .iter()
            .flatten()
            .zip(reference.linear.iter().flatten())
        {
            assert!((a - b * 0.75_f32.powi(count as i32)).abs() < 1e-6);
        }
    }
    for count in [63, 64] {
        let d = shadow_document(
            count,
            Opacity::Mask {
                factor: 0.,
                cutoff: 0.5,
            },
        );
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let cpu = render_core::render::render(&scene, &s, || false);
        let actual = pollster::block_on(gpu.render(&scene, &s, 0, &AtomicBool::new(false)));
        if count == 64 {
            assert_eq!(cpu.unwrap_err().code, "budget");
            assert_eq!(actual.unwrap_err().code, "budget");
        } else {
            assert_eq!(actual.unwrap().linear, reference.linear);
            cpu.unwrap();
        }
    }
}
fn animated_stack() -> Document {
    use animation::*;
    let mut d = planes(
        65,
        Opacity::Mask {
            factor: 0.,
            cutoff: 0.5,
        },
    );
    let tracks = d
        .snapshot()
        .entities
        .iter()
        .filter(|e| e.mesh.is_some())
        .enumerate()
        .map(|(i, e)| Track {
            id: Id(40000 + i as u128),
            target: Target::Entity { entity: e.id },
            property: Property::Translation,
            interpolation: Interpolation::Step,
            keys: [(0, 0.), (1, 10.), (2, 0.)]
                .map(|(t, x)| Key {
                    time: Time::new(t, 1).unwrap(),
                    value: Value::Vector([x, 0., 0.]),
                    incoming: None,
                    outgoing: None,
                })
                .to_vec(),
        })
        .collect();
    let mut state = State::default();
    state.clips.insert(
        Id(30002),
        Clip {
            id: Id(30002),
            start: Time::new(0, 1).unwrap(),
            end: Time::new(2, 1).unwrap(),
            extrapolation: Extrapolation::Clamp,
            remap: TimeMap {
                rate: Time::new(1, 1).unwrap(),
                offset: Time::new(0, 1).unwrap(),
            },
            tracks,
        },
    );
    let mut s = settings();
    s.width = 2;
    s.height = 2;
    s.samples = 1;
    d.execute(
        &fixtures::principal(),
        &fixtures::request(
            &d,
            "alpha:gpu:animated:1",
            vec![
                Command::SetAnimation {
                    animation: Some(state),
                },
                Command::SetRenderSettings { settings: s },
            ],
        )
        .unwrap(),
    )
    .unwrap();
    d
}
#[test]
#[ignore = "requires actual GPU; mandatory in alpha acceptance"]
fn shutter_failure_survives_later_valid_dispatches_and_sequence_is_partial() {
    use sequence::*;
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let d = animated_stack();
    let before = canonical(&d).unwrap();
    let cancel = AtomicBool::new(false);
    let mut q = FrameRequest {
        revision: d.snapshot().revision().unwrap(),
        clip: Id(30002),
        time: Time::new(0, 1).unwrap(),
        shutter: Shutter {
            open: Time::new(1, 1).unwrap(),
            close: Time::new(2, 1).unwrap(),
            samples: 2,
        },
    };
    // Failed nominal pass followed only by valid temporal dispatches.
    assert_eq!(
        pollster::block_on(gpu.render_frame(d.snapshot(), &q, &cancel))
            .unwrap_err()
            .code,
        "budget"
    );
    // Valid nominal pass, failed first temporal sample, valid final sample.
    q.time = Time::new(1, 1).unwrap();
    q.shutter.open = Time::new(-1, 1).unwrap();
    q.shutter.close = Time::new(1, 1).unwrap();
    assert_eq!(
        pollster::block_on(gpu.render_frame(d.snapshot(), &q, &cancel))
            .unwrap_err()
            .code,
        "budget"
    );
    q.shutter = Shutter::instant();
    pollster::block_on(gpu.render_frame(d.snapshot(), &q, &cancel)).unwrap();
    let sequence = SequenceRequest {
        revision: q.revision.clone(),
        clip: q.clip,
        times: vec![Time::new(1, 1).unwrap(), Time::new(2, 1).unwrap()],
        shutter: Shutter::instant(),
    };
    let mut emitted = 0;
    let error = pollster::block_on(gpu.render_sequence(
        d.snapshot(),
        &sequence,
        |index, _| {
            assert_eq!(index, emitted);
            emitted += 1;
            Ok(())
        },
        &cancel,
    ))
    .unwrap_err();
    assert_eq!(error.code, "budget");
    assert_eq!(emitted, 1);
    q.revision = "stale".into();
    assert_eq!(
        pollster::block_on(gpu.render_frame(d.snapshot(), &q, &cancel))
            .unwrap_err()
            .code,
        "stale_revision"
    );
    q.revision = d.snapshot().revision().unwrap();
    q.shutter.samples = 0;
    assert_eq!(
        pollster::block_on(gpu.render_frame(d.snapshot(), &q, &cancel))
            .unwrap_err()
            .code,
        "budget"
    );
    assert_eq!(canonical(&d).unwrap(), before);
}
#[test]
#[ignore = "requires actual GPU; mandatory in alpha acceptance"]
fn normalized_texture_cutoffs_and_mixed_material_bounces() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let mut s = settings();
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 1.,
        ymag: 1.,
        near: 0.1,
        far: 8.,
    });
    s.environment = [0.; 3];
    s.max_depth = 1;
    let d = document(include_bytes!("../../../fixtures/alpha-gltf/data/mask.glb"));
    for (factor, cutoff) in [
        (0.75, f64::from(85_f32 / 255.) * 0.75),
        (0.75, (f64::from(85_f32 / 255.) * 0.75).next_up()),
        (0.0031, f64::from(85_f32 / 255.) * 0.0031),
        (0.0031, (f64::from(85_f32 / 255.) * 0.0031).next_up()),
        (f64::from_bits(1), f64::from_bits(1)),
        (1., f64::from_bits(1)),
    ] {
        let mut snapshot = d.snapshot().clone();
        for m in snapshot.materials.values_mut() {
            if let Some(a) = m.pbr.as_mut().and_then(|p| p.advanced.as_mut()) {
                a.opacity = Opacity::Mask { factor, cutoff };
            }
        }
        let scene = Evaluator::default().evaluate(&snapshot).unwrap();
        compare(&mut gpu, &scene, &s, 0.000001);
    }
    let d = planes(1, Opacity::Opaque);
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    s.camera.position = [0., 0., 0.000001];
    s.camera.lens = Some(cameras::Lens::Perspective {
        vertical_fov_radians: 0.5,
        aspect_ratio: Some(1.),
        near: 0.0000001,
        far: Some(1.),
    });
    // Hash 0xffffff00 at pixel0/sample0/dimension1024: its f64 midpoint
    // is below one, while the f32 random calculation rounds to one.
    s.seed = 311516010;
    s.samples = 1;
    let image = compare(&mut gpu, &scene, &s, 0.000001);
    assert!(image.objects.iter().all(Option::is_some));
    let mut d = Document::new(Snapshot::empty(Id(10060))).unwrap();
    let commands = serde_json::from_slice(include_bytes!(
        "../../../fixtures/multibounce-pbr/create.json"
    ))
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "alpha:gpu:mixed:0001", commands).unwrap(),
    )
    .unwrap();
    let mut snapshot = d.snapshot().clone();
    for m in snapshot.materials.values_mut() {
        if let Some(p) = m.pbr.as_mut() {
            p.advanced = Some(scattering::Surface {
                model: scattering::Model::Principled,
                opacity: Opacity::Mask {
                    factor: 1.,
                    cutoff: 0.5,
                },
            });
        }
    }
    let scene = Evaluator::default().evaluate(&snapshot).unwrap();
    let mut s = snapshot.render_settings.clone().unwrap();
    assert!(scene.instances.iter().any(|i| i.material.pbr.is_none()));
    for depth in [1, 2, 4] {
        s.max_depth = depth;
        compare(&mut gpu, &scene, &s, 0.002);
    }
}
#[test]
fn mask_thresholds_preserve_cpu_multiplication_at_decoded_alpha_boundaries() {
    // Equality is defined by the CPU multiplication predicate, not its algebraic
    // rearrangement into cutoff/factor (which rounds differently).
    for factor in [0.1, 0.3, 0.7, 0.9, f64::from_bits(1)] {
        for byte in [3, 9, 27, 31, 85, 128, 170, 255] {
            let alpha = f64::from(byte as f32 / 255.);
            let cutoff = alpha * factor;
            let d = planes(1, Opacity::Mask { factor, cutoff });
            let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
            let packed = render_gpu::pack(&scene, &settings(), 0).unwrap();
            let row = packed.instances[15];
            let admitted = row[0] == 0. || alpha >= f64::from(row[2]);
            assert!(
                admitted,
                "factor={factor} byte={byte} cutoff={cutoff} threshold={}",
                row[2]
            );
            if cutoff > 0. {
                let below = (row[2]).next_down();
                assert!(
                    f64::from(below) * factor < cutoff,
                    "threshold must be the smallest accepted f32 alpha"
                );
            }
        }
    }
}
