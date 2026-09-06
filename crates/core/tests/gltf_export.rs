use glam::{DVec3, Vec3};
use render_core::gltf_export::Request;
use render_core::{document::*, gltf_export::*, render::*, *};
use serde_json::Value;
const UV: &[u8] = include_bytes!("../../../fixtures/named-uv/data/roles.glb");
const COLOR: &[u8] = include_bytes!("../../../fixtures/vertex-colors/data/rgba.glb");
fn document(bytes: &[u8]) -> (Document, gltf_scene::Report) {
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
        &fixtures::request(&d, "export:fixture:0001", i.commands).unwrap(),
    )
    .unwrap();
    (d, i.report)
}
fn request(s: &Snapshot) -> Request {
    Request {
        revision: s.revision().unwrap(),
        at: None,
        policy: Policy {
            allow_approximations: true,
            ..Default::default()
        },
    }
}
fn root(bytes: &[u8]) -> Value {
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&bytes[20..20 + len]).unwrap()
}
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 32;
    s.height = 16;
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
    s
}
fn compare(s: &Snapshot, q: &Request) -> Exported {
    let before = canonical(s).unwrap();
    let out = export(s, q, || false).unwrap();
    let json = root(&out.glb);
    for (id, node) in &out.report.source_entities {
        assert_eq!(
            json["nodes"][*node as usize]["name"],
            s.entities.get(*id).unwrap().name
        );
    }
    let (d, report) = document(&out.glb);
    let mut evaluator = Evaluator::default();
    let scene = if let Some(at) = &q.at {
        evaluator
            .evaluate_at(s, at.clip, at.time, || false)
            .unwrap()
            .0
    } else {
        evaluator.evaluate(s).unwrap()
    };
    let round = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let a = render(&scene, &settings(), || false).unwrap();
    let b = render(&round, &settings(), || false).unwrap();
    let error = (a
        .linear
        .iter()
        .flatten()
        .zip(b.linear.iter().flatten())
        .map(|(x, y)| f64::from(x - y).powi(2))
        .sum::<f64>()
        / a.linear.len() as f64
        / 3.)
        .sqrt();
    assert!(error < 2e-5, "RMSE {error}");
    assert!(
        a.depth
            .iter()
            .zip(&b.depth)
            .all(|(x, y)| (x - y).abs() < 2e-5)
    );
    assert!(
        a.normals
            .iter()
            .flatten()
            .zip(b.normals.iter().flatten())
            .all(|(x, y)| (x - y).abs() < 2e-5)
    );
    for (x, y) in a.objects.iter().zip(&b.objects) {
        assert_eq!(
            x.map(|id| {
                let parent = report.source_nodes[&(out.report.source_entities[&id] as usize)];
                d.snapshot()
                    .entities
                    .iter()
                    .find(|e| e.parent == Some(parent) && e.mesh.is_some())
                    .unwrap()
                    .id
            }),
            *y
        );
    }
    assert_eq!(canonical(s).unwrap(), before);
    let restored = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(canonical(&restored).unwrap(), canonical(&d).unwrap());
    assert_eq!(export(s, q, || false).unwrap().glb, out.glb);
    out
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn named_uv_color_alpha_and_sampler_semantics_roundtrip_through_glb() {
    for bytes in [
        UV,
        COLOR,
        include_bytes!("../../../fixtures/alpha-gltf/data/mask.glb"),
        include_bytes!("../../../fixtures/alpha-gltf/data/blend.glb"),
    ] {
        let (d, _) = document(bytes);
        let out = compare(d.snapshot(), &request(d.snapshot()));
        assert_eq!(out.report.output_digest, digest(&out.glb));
        assert_eq!(out.report.bytes, out.glb.len());
        assert_eq!(out.report.max_position_error_meters, 0.);
        let (round, _) = document(&out.glb);
        assert_eq!(d.snapshot().images, round.snapshot().images);
        let j = root(&out.glb);
        assert_eq!(
            j["nodes"].as_array().unwrap().len(),
            out.report.source_entities.len()
        );
        if bytes == UV {
            let attrs = &j["meshes"][0]["primitives"][0]["attributes"];
            for i in 0..8 {
                assert!(attrs.get(format!("TEXCOORD_{i}")).is_some());
            }
            let m = &j["materials"][0];
            assert_eq!(m["emissiveTexture"]["texCoord"], 1);
            assert_eq!(m["normalTexture"]["texCoord"], 1);
            assert_eq!(m["occlusionTexture"]["texCoord"], 2);
            assert_eq!(
                m["pbrMetallicRoughness"]["metallicRoughnessTexture"]["texCoord"],
                7
            );
        }
        if bytes == COLOR {
            assert_eq!(out.report.materials, 1);
            assert_eq!(out.report.meshes, 2);
            let colored = j["meshes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["primitives"][0]["attributes"].get("COLOR_0").is_some())
                .count();
            assert_eq!(colored, 1);
        }
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn evaluated_morph_frames_and_displacement_export_the_selected_surface() {
    let (d, _) = document(include_bytes!(
        "../../../fixtures/morph-frames/data/dense.glb"
    ));
    let clip = *d
        .snapshot()
        .animation
        .as_ref()
        .unwrap()
        .clips
        .keys()
        .next()
        .unwrap();
    for n in [0, 1, 2] {
        let mut q = request(d.snapshot());
        q.at = Some(Sample {
            clip,
            time: Time::new(n, 2).unwrap(),
        });
        let out = compare(d.snapshot(), &q);
        assert_eq!(
            out.report.animation.as_ref().unwrap().time,
            q.at.unwrap().time
        );
        let j = root(&out.glb);
        assert!(j.get("animations").is_none() && j.get("skins").is_none());
        assert!(
            j["meshes"][0]["primitives"][0]["attributes"]
                .get("TANGENT")
                .is_some()
        );
    }
    let (d, _) = document(COLOR);
    let mut snapshot = d.snapshot().clone();
    for m in snapshot.materials.values_mut() {
        if let Some(p) = &mut m.pbr {
            p.displacement = Some(displacement::Displacement {
                height: displacement::Height::Constant { meters: 0.125 },
                subdivisions: 2,
                max_vertices: 256,
            });
        }
    }
    let out = compare(&snapshot, &request(&snapshot));
    assert_eq!(out.report.exported_vertices, 192);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn local_frames_and_mesh_sharing_survive_nonuniform_and_reflected_transforms() {
    let (d, _) = document(UV);
    let mut s = d.snapshot().clone();
    let mut e = s
        .entities
        .iter()
        .find(|e| e.mesh.is_some())
        .unwrap()
        .clone();
    let mut mesh = (*s.meshes[e.mesh.as_ref().unwrap()]).clone();
    mesh.attributes.get_mut("normal").unwrap().values = geometry::AttributeValues::Vec3(vec![
        [0., 0., 1.],
        [0.6, 0., 0.8],
        [0., 0.6, 0.8],
        [-0.6, 0., 0.8],
    ]);
    let key = mesh.content_id().unwrap();
    s.meshes.insert(key.clone(), std::sync::Arc::new(mesh));
    e.mesh = Some(key);
    e.transform = Transform {
        columns: [[1.5, 0., 0.], [0., 0.5, 0.], [0., 0., 2.], [0., 0., 0.]],
        operations: vec![],
    };
    *s.entities.get_mut(e.id).unwrap() = e.clone();
    compare(&s, &request(&s));
    e.id = Id(99999);
    e.transform.columns[0][0] = -1.5;
    e.transform.columns[3] = [0., 1.5, 0.];
    s.entities.insert(e).unwrap();
    let out = compare(&s, &request(&s));
    assert_eq!(out.report.meshes, 1);
    assert_eq!(out.report.materials, 1);
    assert_eq!(out.report.exported_vertices, 6);
    assert_eq!(out.report.source_entities.len(), 2);
    let j = root(&out.glb);
    assert_eq!(j["nodes"][0]["mesh"], j["nodes"][1]["mesh"]);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn precision_policy_measures_world_error_and_rejects_shear_or_collapsed_output() {
    let (d, _) = document(UV);
    let mut s = d.snapshot().clone();
    let id = s.entities.iter().find(|e| e.mesh.is_some()).unwrap().id;
    s.entities.get_mut(id).unwrap().transform = Transform::translation(1. / 3., 0., 0.);
    let q = request(&s);
    let out = export(&s, &q, || false).unwrap();
    let expected = (1. / 3. - f64::from((1. / 3.) as f32)).abs();
    assert!((out.report.max_position_error_meters - expected).abs() < 1e-15);
    let mut strict = q.clone();
    strict.policy.max_position_error_meters = 1e-10;
    assert_eq!(export(&s, &strict, || false).unwrap_err().code, "precision");
    s.entities.get_mut(id).unwrap().transform.columns[1][0] = 0.1;
    assert_eq!(
        export(&s, &request(&s), || false).unwrap_err().code,
        "unsupported_gltf"
    );
    s.entities.get_mut(id).unwrap().transform = Transform::translation(1e20, 0., 0.);
    assert_eq!(
        export(&s, &request(&s), || false).unwrap_err().code,
        "precision"
    );
    let mut mesh = (*s.meshes[s.entities.get(id).unwrap().mesh.as_ref().unwrap()]).clone();
    mesh.positions = geometry::Positions::F64(vec![
        [1e8, 0., 0.],
        [1e8 + 1., 0., 0.],
        [1e8 + 1., 1., 0.],
        [1e8, 1., 0.],
    ]);
    let key = mesh.content_id().unwrap();
    s.meshes.insert(key.clone(), std::sync::Arc::new(mesh));
    let e = s.entities.get_mut(id).unwrap();
    e.mesh = Some(key);
    e.transform = Transform::default();
    let mut q = request(&s);
    q.policy.max_position_error_meters = 1.;
    assert_eq!(export(&s, &q, || false).unwrap_err().code, "precision");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn export_public_operation_rejects_stale_cancel_budgets_and_unsupported_shading() {
    let (d, _) = document(COLOR);
    let s = d.snapshot();
    let before = canonical(&d).unwrap();
    let q = request(s);
    let mut calls = 0;
    export(s, &q, || {
        calls += 1;
        false
    })
    .unwrap();
    for stop in [1, 2, 3, calls / 2, calls - 1, calls] {
        let mut count = 0;
        assert_eq!(
            export(s, &q, || {
                count += 1;
                count == stop
            })
            .unwrap_err()
            .code,
            "cancelled"
        );
    }
    let mut bad = q.clone();
    bad.revision = "stale".into();
    assert_eq!(
        export(s, &bad, || false).unwrap_err().code,
        "stale_revision"
    );
    for case in 0..4 {
        let mut q = q.clone();
        match case {
            0 => q.policy.max_bytes = 64,
            1 => q.policy.max_vertices = 3,
            2 => q.policy.max_position_error_meters = 0.,
            _ => q.policy.max_bytes = u64::MAX,
        };
        assert_eq!(export(s, &q, || false).unwrap_err().code, "budget");
    }
    let mut denied = q.clone();
    denied.policy.allow_approximations = false;
    assert_eq!(
        export(s, &denied, || false).unwrap_err().code,
        "unsupported_gltf"
    );
    for case in 0..3 {
        let mut s = s.clone();
        for m in s.materials.values_mut() {
            match case {
                0 => m.pbr = None,
                1 => m.emission = [2.; 3],
                _ => {
                    m.pbr.as_mut().unwrap().advanced = Some(scattering::Surface {
                        model: scattering::Model::Coated {
                            weight: 0.5,
                            ior: 1.5,
                            roughness: 0.2,
                        },
                        opacity: scattering::Opacity::Opaque,
                    })
                }
            }
        }
        assert_eq!(
            export(&s, &request(&s), || false).unwrap_err().code,
            if case == 0 {
                "unsupported_profile"
            } else {
                "unsupported_gltf"
            }
        );
    }
    let mut long = s.clone();
    let id = long.entities.iter().find(|e| e.mesh.is_some()).unwrap().id;
    long.entities.get_mut(id).unwrap().name = "x".repeat(1025);
    assert_eq!(
        export(&long, &request(&long), || false).unwrap_err().code,
        "budget"
    );
    assert_eq!(canonical(&d).unwrap(), before);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn agent_export_is_the_same_pinned_read_only_rust_operation() {
    let (d, _) = document(COLOR);
    let q = request(d.snapshot());
    let expected = export(d.snapshot(), &q, || false).unwrap();
    let before = canonical(&d).unwrap();
    let mut agent = agent::Session::new(d);
    let value = agent
        .dispatch(
            &fixtures::principal(),
            agent::Request {
                version: 0,
                operation: agent::Operation::ExportGlb { request: q.clone() },
            },
        )
        .unwrap();
    let actual: Exported = serde_json::from_value(value).unwrap();
    assert_eq!(actual.glb, expected.glb);
    let mut stale = q;
    stale.revision = "stale".into();
    assert_eq!(
        agent
            .dispatch(
                &fixtures::principal(),
                agent::Request {
                    version: 0,
                    operation: agent::Operation::ExportGlb { request: stale }
                }
            )
            .unwrap_err()
            .code,
        "stale_revision"
    );
    let value = agent
        .dispatch(
            &fixtures::principal(),
            agent::Request {
                version: 0,
                operation: agent::Operation::Export,
            },
        )
        .unwrap();
    assert_eq!(canonical(&value["document"]).unwrap(), before);
    let (round, _) = document(&actual.glb);
    let scene = Evaluator::default().evaluate(round.snapshot()).unwrap();
    let ray = Ray {
        origin: DVec3::new(-1.75, 0.5, 3.),
        direction: -DVec3::Z,
    };
    let h = scene.intersect(ray, 0.1, 5.).unwrap();
    assert!(
        h.color_rgba(&scene)
            .truncate()
            .distance(Vec3::new(0.25, 0.75, 0.75))
            < 1e-7
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn persistence_layout_cannot_change_export_order_or_bytes() {
    let (d, _) = document(include_bytes!(
        "../../../fixtures/alpha-gltf/data/blend.glb"
    ));
    let restored = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(
        d.snapshot().revision().unwrap(),
        restored.snapshot().revision().unwrap()
    );
    let a = export(d.snapshot(), &request(d.snapshot()), || false).unwrap();
    let b = export(restored.snapshot(), &request(restored.snapshot()), || false).unwrap();
    assert_eq!(a.report.output_digest, b.report.output_digest);
    assert_eq!(a.glb, b.glb);
}
