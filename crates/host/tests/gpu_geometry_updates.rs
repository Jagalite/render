use render_core::{document::*, geometry::Mesh, render::*, *};
use render_gpu::{GeometryUploadKind, Renderer};
use std::{collections::BTreeMap, sync::atomic::AtomicBool};

fn bind(mesh: Mesh) -> Vec<Command> {
    let asset = mesh.content_id().unwrap();
    vec![
        Command::PutMesh { mesh },
        Command::SetProcedural {
            entity: Id(2),
            graph: Some(procedural::Graph {
                root: procedural::Group {
                    nodes: BTreeMap::from([(Id(10), procedural::Node::Mesh { asset })]),
                    output: Id(10),
                },
                groups: BTreeMap::new(),
                budget: modeling::Budget::default(),
            }),
        },
    ]
}
fn apply(d: &mut Document, name: &str, commands: Vec<Command>) {
    let q = fixtures::request(d, name, commands).unwrap();
    d.execute(&fixtures::principal(), &q).unwrap();
}
fn same(a: &Image, b: &Image) {
    assert_eq!(a.linear, b.linear);
    assert_eq!(a.depth, b.depth);
    assert_eq!(a.normals, b.normals);
    assert_eq!(a.objects, b.objects);
    assert_eq!(
        canonical(&a.receipt).unwrap(),
        canonical(&b.receipt).unwrap()
    );
}
fn cpu_error(cpu: &Image, gpu: &Image) -> f64 {
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
        .map(|(a, b)| f64::from(a - b).powi(2))
        .sum::<f64>()
        / (cpu.linear.len() * 3) as f64)
        .sqrt();
    assert!(rmse < 0.002, "CPU/GPU RMSE {rmse}");
    rmse
}
#[test]
#[ignore = "requires real GPU; mandatory for packed geometry updates qualification"]
fn persistent_cache_matches_fresh_renderers_and_reports_local_transfers() {
    pollster::block_on(async {
        let input: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/gpu-geometry-updates/data/grid-local-edit.json"
        ))
        .unwrap();
        let mesh = |name: &str| serde_json::from_value::<Mesh>(input[name].clone()).unwrap();
        let mut d = Document::new(Snapshot::empty(Id(11000))).unwrap();
        let mut s = fixtures::settings();
        s.width = 16;
        s.height = 16;
        s.samples = 4;
        s.camera.position = [0., 0., 6.];
        s.camera.target = [0.; 3];
        s.camera.up = [0., 1., 0.];
        s.camera.lens = Some(cameras::Lens::Orthographic {
            xmag: 2.,
            ymag: 2.,
            near: 0.01,
            far: 10.,
        });
        let mut commands = vec![
            Command::PutMaterial {
                material: Box::new(Material::diffuse(Id(1), [0.4, 0.6, 0.8])),
            },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(2),
                    name: "Local edit grid".into(),
                    parent: None,
                    mesh: None,
                    material: Some(Id(1)),
                    transform: Transform::default(),
                },
            },
            Command::SetRenderSettings {
                settings: s.clone(),
            },
        ];
        commands.extend(bind(mesh("base")));
        apply(&mut d, "cache:fixture:initial", commands);
        let base = d.clone();
        let stale = fixtures::request(&d, "cache:stale:request", bind(mesh("resized"))).unwrap();
        apply(&mut d, "cache:fixture:local", bind(mesh("local")));
        let local = d.clone();
        let mut dense = d.clone();
        apply(&mut dense, "cache:fixture:dense", bind(mesh("dense")));
        let before = canonical(&d).unwrap();
        assert_eq!(
            d.execute(&fixtures::principal(), &stale).unwrap_err().code,
            "stale_revision"
        );
        assert_eq!(before, canonical(&d).unwrap());
        apply(&mut d, "cache:fixture:resize", bind(mesh("resized")));
        let resized = d.clone();
        let mut camera = resized.clone();
        let mut moved = s.clone();
        moved.camera.position[0] = 0.1;
        moved.camera.target[0] = 0.1;
        apply(
            &mut camera,
            "cache:fixture:camera",
            vec![Command::SetRenderSettings { settings: moved }],
        );
        let mut material = camera.clone();
        apply(
            &mut material,
            "cache:fixture:material",
            vec![Command::PutMaterial {
                material: Box::new(Material::diffuse(Id(1), [0.8, 0.2, 0.1])),
            }],
        );
        let cases = [
            ("base", &base, GeometryUploadKind::Allocate),
            ("repeat", &base, GeometryUploadKind::Reuse),
            ("local", &local, GeometryUploadKind::Patch),
            ("local-repeat", &local, GeometryUploadKind::Reuse),
            ("undo", &base, GeometryUploadKind::Patch),
            ("dense", &dense, GeometryUploadKind::Rewrite),
            ("dense-repeat", &dense, GeometryUploadKind::Reuse),
            ("dense-undo", &base, GeometryUploadKind::Rewrite),
            ("resize", &resized, GeometryUploadKind::Allocate),
            ("resize-repeat", &resized, GeometryUploadKind::Reuse),
            ("camera", &camera, GeometryUploadKind::Reuse),
            ("material", &material, GeometryUploadKind::Reuse),
        ];
        let mut gpu = Renderer::new().await.unwrap();
        let cancel = AtomicBool::new(false);
        let mut reports = vec![];
        let mut base_depth = None;
        let mut local_change = 0.0_f32;
        let out = std::env::var_os("RENDER_GPU_UPDATE_OUTPUT").map(std::path::PathBuf::from);
        if let Some(out) = &out {
            std::fs::create_dir_all(out).unwrap();
        }
        for (name, document, expected) in cases {
            let before = canonical(document).unwrap();
            let scene = Evaluator::default().evaluate(document.snapshot()).unwrap();
            let settings = document.snapshot().render_settings.as_ref().unwrap();
            let image = gpu.render(&scene, settings, 0, &cancel).await.unwrap();
            let mut fresh = Renderer::new().await.unwrap();
            let reference = fresh.render(&scene, settings, 0, &cancel).await.unwrap();
            same(&image, &reference);
            let cpu = render(&scene, settings, || false).unwrap();
            let rmse = cpu_error(&cpu, &image);
            let statistics = gpu.geometry_upload_statistics();
            let last = statistics.last.as_ref().unwrap();
            assert_eq!(last.kind, expected, "{name}: {statistics:?}");
            if name == "local" {
                assert!(
                    last.uploaded_bytes * 10 < last.packed_buffer_bytes,
                    "{statistics:?}"
                );
                local_change = base_depth
                    .as_ref()
                    .map(|depth: &Vec<f32>| {
                        depth
                            .iter()
                            .zip(&image.depth)
                            .map(|(a, b)| (a - b).abs())
                            .fold(0.0_f32, f32::max)
                    })
                    .unwrap();
                assert!(local_change > 0.01);
            }
            if name == "base" {
                base_depth = Some(image.depth.clone());
            }
            assert_eq!(before, canonical(document).unwrap());
            reports.push(serde_json::json!({"name":name,"revision":scene.revision,"statistics":statistics,"cpu_gpu_rmse":rmse,"fresh_gpu_exact":true,"document_unchanged":true}));
            if let Some(out) = &out {
                let folder = out.join(name);
                std::fs::create_dir_all(&folder).unwrap();
                std::fs::write(folder.join("document.json"), canonical(document).unwrap()).unwrap();
                std::fs::write(folder.join("image.pfm"), image.pfm()).unwrap();
                std::fs::write(folder.join("cpu.pfm"), cpu.pfm()).unwrap();
                std::fs::write(
                    folder.join("passes.json"),
                    canonical(&(
                        image.width,
                        image.height,
                        &image.depth,
                        &image.normals,
                        &image.objects,
                    ))
                    .unwrap(),
                )
                .unwrap();
                std::fs::write(
                    folder.join("receipt.json"),
                    canonical(&image.receipt).unwrap(),
                )
                .unwrap();
            }
        }
        let scene = Evaluator::default().evaluate(local.snapshot()).unwrap();
        let before = canonical(&gpu.geometry_upload_statistics()).unwrap();
        assert_eq!(
            gpu.render(&scene, &s, 0, &AtomicBool::new(true))
                .await
                .unwrap_err()
                .code,
            "cancelled"
        );
        let mut invalid = s.clone();
        invalid.max_bytes = 1;
        assert_eq!(
            gpu.render(&scene, &invalid, 0, &cancel)
                .await
                .unwrap_err()
                .code,
            "budget"
        );
        assert_eq!(
            before,
            canonical(&gpu.geometry_upload_statistics()).unwrap()
        );
        assert!(gpu.cancellation_fault_probe(&scene, &s).await.unwrap());
        let after = gpu.render(&scene, &s, 0, &cancel).await.unwrap();
        assert_eq!(
            gpu.geometry_upload_statistics().last.unwrap().kind,
            GeometryUploadKind::Reuse
        );
        let mut fresh = Renderer::new().await.unwrap();
        same(&after, &fresh.render(&scene, &s, 0, &cancel).await.unwrap());
        gpu.destroy();
        assert_eq!(gpu.geometry_upload_statistics().retained_shadow_bytes, 0);
        assert_eq!(
            gpu.render(&scene, &s, 0, &cancel).await.unwrap_err().code,
            "device_lost"
        );
        let mut recreated = Renderer::new().await.unwrap();
        same(
            &after,
            &recreated.render(&scene, &s, 0, &cancel).await.unwrap(),
        );
        let report = serde_json::json!({"status":"passed","profile":"gpu-buffer-updates-v1","cases":reports,"local_depth_change_meters":local_change,"stale_transaction_atomic":true,"pre_admission_failures_preserve_counters":true,"post_submit_cancel_cache_valid":true,"destroy_clears_shadow":true,"recreated_exact":true,"device":recreated.capabilities});
        if let Some(out) = out {
            std::fs::write(
                out.join("native_report.json"),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        println!("{report}");
    });
}
