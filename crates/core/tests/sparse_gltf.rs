use glam::{DQuat, DVec3};
use render_core::{document::*, gltf_scene::*, render::*, *};
use serde_json::{Value, json};
const DENSE_JSON: &[u8] = include_bytes!("../../../fixtures/sparse-gltf/data/dense.gltf");
const DENSE_BIN: &[u8] = include_bytes!("../../../fixtures/sparse-gltf/data/dense.bin");
const ZERO_JSON: &[u8] = include_bytes!("../../../fixtures/sparse-gltf/data/sparse-zero-u16.gltf");
const ZERO_BIN: &[u8] = include_bytes!("../../../fixtures/sparse-gltf/data/sparse-zero-u16.bin");
const BASE_JSON: &[u8] = include_bytes!("../../../fixtures/sparse-gltf/data/sparse-base-u8.gltf");
const BASE_BIN: &[u8] = include_bytes!("../../../fixtures/sparse-gltf/data/sparse-base-u8.bin");
fn policy() -> PbrPolicy {
    PbrPolicy {
        allow_approximations: true,
    }
}
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 24;
    s.height = 24;
    s.samples = 4;
    s.max_depth = 2;
    s.camera.position = [1., 1., 5.];
    s.camera.target = [0., 1., 0.];
    s
}
fn import(json: &[u8], bin: &[u8]) -> Result<Imported> {
    import_pbr_scene(json, &[bin.to_vec()], &[], Id(99), &policy())
}
fn document(json: &[u8], bin: &[u8]) -> (Document, Report) {
    let mut doc = Document::new(Snapshot::empty(Id(99))).unwrap();
    let mut imported = import(json, bin).unwrap();
    imported.commands.push(Command::SetRenderSettings {
        settings: settings(),
    });
    doc.execute(
        &fixtures::principal(),
        &fixtures::request(&doc, "sparse:fixture:01", imported.commands).unwrap(),
    )
    .unwrap();
    (doc, imported.report)
}
fn reference(t: f64) -> Vec<DVec3> {
    let weight = if t < 1. {
        0.25
    } else if t < 2. {
        1.
    } else {
        0.
    };
    let angle = if t <= 1. { t } else { 2. - t };
    let q = DQuat::from_rotation_z(angle * std::f64::consts::FRAC_PI_2);
    let shift = DVec3::new(0.2 * t, 0., 0.);
    let pivot = DVec3::new(0.25, 1., 0.);
    vec![
        DVec3::new(-0.5, 0., 0.) + shift,
        DVec3::new(0.5, 0., 0.) + shift,
        q * (DVec3::new(-0.5 - 0.2 * weight, 2., 0.2 * weight) - pivot) + pivot + shift,
        q * (DVec3::new(0.5 + 0.2 * weight, 2., 0.2 * weight) - pivot) + pivot + shift,
    ]
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sparse_dense_and_normalized_uv_inputs_preserve_analytic_animation_and_rendering() {
    let (dense, dense_report) = document(DENSE_JSON, DENSE_BIN);
    for (json, bin, glb) in [
        (
            ZERO_JSON,
            ZERO_BIN,
            include_bytes!("../../../fixtures/sparse-gltf/data/sparse-zero-u16.glb").as_slice(),
        ),
        (
            BASE_JSON,
            BASE_BIN,
            include_bytes!("../../../fixtures/sparse-gltf/data/sparse-base-u8.glb").as_slice(),
        ),
    ] {
        let (doc, report) = document(json, bin);
        let before = canonical(&doc).unwrap();
        // Same native geometry/content identity despite different source encodings and IDs.
        assert_eq!(
            canonical(&doc.snapshot().meshes).unwrap(),
            canonical(&dense.snapshot().meshes).unwrap()
        );
        assert_eq!(
            canonical(&doc.snapshot().images).unwrap(),
            canonical(&dense.snapshot().images).unwrap()
        );
        let imported_glb = import_pbr_glb(glb, Id(99), &policy()).unwrap();
        assert_eq!(imported_glb.report.instances, report.instances);
        for (n, d) in [(0, 1), (1, 2), (1, 1), (3, 2), (2, 1), (1, 2)] {
            let time = Time::new(n, d).unwrap();
            let evaluated =
                animation::evaluate(doc.snapshot(), report.source_clips[&0], time, || false)
                    .unwrap();
            let entity = *doc
                .snapshot()
                .animation
                .as_ref()
                .unwrap()
                .skins
                .keys()
                .next()
                .unwrap();
            let mesh = &evaluated.snapshot.meshes[evaluated
                .snapshot
                .entities
                .get(entity)
                .unwrap()
                .mesh
                .as_ref()
                .unwrap()];
            let world = evaluated.snapshot.world_transform(entity).unwrap();
            for (i, expected) in reference(n as f64 / d as f64).iter().enumerate() {
                assert!(
                    world
                        .transform_point3(mesh.positions.get(i))
                        .distance(*expected)
                        < 2e-7
                );
            }
            let (actual, _) = Evaluator::default()
                .evaluate_at(doc.snapshot(), report.source_clips[&0], time, || false)
                .unwrap();
            let (expected, _) = Evaluator::default()
                .evaluate_at(
                    dense.snapshot(),
                    dense_report.source_clips[&0],
                    time,
                    || false,
                )
                .unwrap();
            let a = render(&actual, &settings(), || false).unwrap();
            let b = render(&expected, &settings(), || false).unwrap();
            assert_eq!(a.linear, b.linear);
            assert_eq!(a.depth, b.depth);
            assert_eq!(a.normals, b.normals);
            assert_eq!(
                a.objects.iter().map(Option::is_some).collect::<Vec<_>>(),
                b.objects.iter().map(Option::is_some).collect::<Vec<_>>()
            );
            assert!(a.objects.iter().filter(|v| v.is_some()).count() > 5);
        }
        assert_eq!(
            canonical(
                &storage::Envelope::new(0, None, &doc)
                    .unwrap()
                    .decode()
                    .unwrap()
            )
            .unwrap(),
            before
        );
        assert_eq!(canonical(&doc).unwrap(), before);
    }
}
fn changed(
    source: &[u8],
    bin: &[u8],
    change: impl FnOnce(&mut Value, &mut Vec<u8>),
) -> Result<Imported> {
    let mut root: Value = serde_json::from_slice(source).unwrap();
    let mut data = bin.to_vec();
    change(&mut root, &mut data);
    import(&serde_json::to_vec(&root).unwrap(), &data)
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sparse_offsets_indices_counts_extensions_and_normalized_roles_reject_atomically() {
    let mutations: Vec<fn(&mut Value, &mut Vec<u8>)> = vec![
        |v, _| v["accessors"][0]["sparse"] = json!(null),
        |v, _| v["accessors"][0]["sparse"]["count"] = json!(0),
        |v, _| v["accessors"][0]["sparse"]["count"] = json!(5),
        |v, _| v["accessors"][0]["count"] = json!(196609),
        |v, _| v["accessors"][0]["byteOffset"] = json!(0),
        |v, _| v["accessors"][0]["normalized"] = json!(true),
        |v, _| v["accessors"][0]["sparse"]["indices"]["componentType"] = json!(5126),
        |v, _| v["accessors"][0]["sparse"]["indices"]["bufferView"] = json!(999),
        |v, _| v["accessors"][0]["sparse"]["values"]["byteOffset"] = json!(u64::MAX),
        |v, _| v["accessors"][0]["sparse"]["values"]["byteOffset"] = json!(1),
        |v, _| v["accessors"][0]["sparse"]["extensions"] = json!({}),
        |v, _| v["accessors"][0]["sparse"]["indices"]["extensions"] = json!({}),
        |v, _| v["accessors"][0]["sparse"]["values"]["extensions"] = json!({}),
        |v, _| v["bufferViews"][0]["byteStride"] = json!(4),
        |v, _| v["bufferViews"][0]["target"] = json!(34963),
        |v, _| v["bufferViews"][1]["byteStride"] = json!(12),
        |v, _| v["bufferViews"][1]["target"] = json!(34962),
        |v, _| v["bufferViews"][1]["byteLength"] = json!(1),
        |v, _| v["bufferViews"][1]["extensions"] = json!({}),
        |v, b| {
            let off = v["bufferViews"][0]["byteOffset"].as_u64().unwrap() as usize;
            b[off + 1] = b[off];
        },
        |v, b| {
            let off = v["bufferViews"][0]["byteOffset"].as_u64().unwrap() as usize;
            b[off] = 4;
        },
        |v, b| {
            let off = v["bufferViews"][0]["byteOffset"].as_u64().unwrap() as usize;
            b.swap(off, off + 1);
        },
        |v, b| {
            let off = v["bufferViews"][1]["byteOffset"].as_u64().unwrap() as usize;
            b[off..off + 4].copy_from_slice(&f32::NAN.to_le_bytes());
        },
        |v, _| v["accessors"][11]["normalized"] = json!(false),
        |v, _| v["accessors"][11]["normalized"] = json!(1),
        |v, _| v["accessors"][11]["componentType"] = json!(5125),
        |v, _| v["accessors"][11]["componentType"] = json!(5126),
    ];
    for (i, change) in mutations.into_iter().enumerate() {
        let mut v: Value = serde_json::from_slice(ZERO_JSON).unwrap();
        let mut b = ZERO_BIN.to_vec();
        change(&mut v, &mut b);
        let mut session = agent::Session::new(Document::new(Snapshot::empty(Id(99))).unwrap());
        let before = canonical(session.document()).unwrap();
        let revision = session.document().snapshot().revision().unwrap();
        let error = session
            .dispatch(
                &fixtures::principal(),
                agent::Request {
                    version: 0,
                    operation: agent::Operation::ImportPbrScene {
                        json: v,
                        buffers: vec![b],
                        images: vec![],
                        policy: policy(),
                        settings: settings(),
                        base_revision: revision,
                        idempotency_key: format!("sparse:bad:{i:04}"),
                    },
                },
            )
            .unwrap_err();
        assert!(
            ["gltf_scene", "unsupported_gltf", "budget"].contains(&error.code.as_str()),
            "mutation {i}: {error:?}"
        );
        assert_eq!(
            canonical(session.document()).unwrap(),
            before,
            "mutation {i}"
        );
    }
    // Relative misalignment must reject even if absolute address happens to align.
    assert!(
        changed(BASE_JSON, BASE_BIN, |v, _| {
            let view = v["accessors"][0]["bufferView"].as_u64().unwrap() as usize;
            v["bufferViews"][view]["byteOffset"] = json!(1);
            v["accessors"][0]["byteOffset"] = json!(3);
        })
        .is_err()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn missing_base_can_represent_zero_morph_and_vertex_offsets_require_four_byte_alignment() {
    let imported = changed(ZERO_JSON, ZERO_BIN, |v, _| {
        v["accessors"][4].as_object_mut().unwrap().remove("sparse");
    })
    .unwrap();
    let mut doc = Document::new(Snapshot::empty(Id(99))).unwrap();
    doc.execute(
        &fixtures::principal(),
        &fixtures::request(&doc, "sparse:zero:morph", imported.commands).unwrap(),
    )
    .unwrap();
    assert!(
        doc.snapshot()
            .animation
            .as_ref()
            .unwrap()
            .morphs
            .values()
            .flatten()
            .flat_map(|m| m.offsets.values())
            .all(|v| *v == [0.; 3])
    );
    let error = changed(BASE_JSON, BASE_BIN, |v, b| {
        // Copy legal byte UVs to a component-aligned but non-four-byte vertex offset.
        let view = v["accessors"][11]["bufferView"].as_u64().unwrap() as usize;
        let start = v["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
        let len = v["bufferViews"][view]["byteLength"].as_u64().unwrap() as usize;
        let data = b[start..start + len].to_vec();
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
        let offset = b.len();
        b.push(0);
        b.extend(data);
        v["bufferViews"][view]["byteOffset"] = json!(offset);
        v["bufferViews"][view]["byteLength"] = json!(len + 1);
        v["accessors"][11]["byteOffset"] = json!(1);
        v["buffers"][0]["byteLength"] = json!(b.len());
    })
    .err()
    .unwrap();
    assert_eq!(error.code, "gltf_scene");
    assert!(error.message.contains("four-byte"));
}
