use glam::{DQuat, DVec3};
use render_core::{animation, document::*, gltf_scene::*, render::Evaluator, *};
use serde_json::{Value, json};
const JSON: &[u8] = include_bytes!("../../../fixtures/animated-gltf/character.gltf");
const BIN: &[u8] = include_bytes!("../../../fixtures/animated-gltf/character.bin");
const GLB: &[u8] = include_bytes!("../../../fixtures/animated-gltf/character.glb");
fn import(json: &[u8], bin: &[u8]) -> Result<Imported> {
    import_pbr_scene(
        json,
        &[bin.to_vec()],
        &[],
        Id(99),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
}
fn apply(doc: &mut Document, commands: Vec<Command>, key: &str) -> Result<Receipt> {
    doc.execute(
        &fixtures::principal(),
        &Request {
            version: 0,
            base_revision: doc.snapshot().revision()?,
            idempotency_key: key.into(),
            commands,
            max_added_bytes: 16 * 1024 * 1024,
        },
    )
}
fn document() -> (Document, Report) {
    let imported = import(JSON, BIN).unwrap();
    let mut doc = Document::new(Snapshot::empty(Id(99))).unwrap();
    apply(&mut doc, imported.commands, "animated-import:01").unwrap();
    (doc, imported.report)
}
fn mesh_entity(s: &Snapshot) -> Id {
    *s.animation.as_ref().unwrap().skins.keys().next().unwrap()
}
fn points(s: &Snapshot, id: Id) -> Vec<DVec3> {
    let e = s.entities.get(id).unwrap();
    let m = &s.meshes[e.mesh.as_ref().unwrap()];
    let world = s.world_transform(id).unwrap();
    (0..m.positions.len())
        .map(|i| world.transform_point3(m.positions.get(i)))
        .collect()
}
fn reference(t: f64) -> Vec<DVec3> {
    // Independent closed form of the fixture: root cubic is 0.25+0.2*t,
    // shoulder rotates 90 degrees/sec, STEP morph is 0.25 then 1 then 0.
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
fn imported_character_matches_analytic_skin_morph_and_absolute_channels() {
    let (doc, report) = document();
    let source = doc.snapshot();
    assert_eq!(source.version, 11);
    let rev = source.revision().unwrap();
    let entity = mesh_entity(source);
    let clip = report.source_clips[&0];
    for (n, d) in [(1, 1), (0, 1), (3, 2), (1, 2), (2, 1), (0, 1)] {
        let out = animation::evaluate(source, clip, Time::new(n, d).unwrap(), || false).unwrap();
        for (actual, expected) in points(&out.snapshot, entity)
            .into_iter()
            .zip(reference(n as f64 / d as f64))
        {
            assert!(
                (actual - expected).length() < 2e-7,
                "{n}/{d}: {actual:?} vs {expected:?}"
            );
        }
        assert_eq!(source.revision().unwrap(), rev);
    }
    let bytes = canonical(&doc).unwrap();
    let restored: Document = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(canonical(&restored).unwrap(), bytes);
    let scene = Evaluator::default().evaluate(restored.snapshot()).unwrap();
    assert_eq!(scene.instances.len(), 1);
    assert!(animation::evaluate(source, clip, Time::new(1, 1).unwrap(), || true).is_err());
    let mut count = 0;
    let error = animation::evaluate(source, clip, Time::new(1, 1).unwrap(), || {
        count += 1;
        count > 12
    })
    .unwrap_err();
    assert_eq!(error.code, "cancelled");
    assert_eq!(source.revision().unwrap(), rev);
    // GLB and external-buffer glTF differ in source identity but agree numerically.
    let imported = import_pbr_glb(
        GLB,
        Id(99),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    let clip = imported.report.source_clips[&0];
    let mut glb = Document::new(Snapshot::empty(Id(99))).unwrap();
    apply(&mut glb, imported.commands, "animated-glb:0001").unwrap();
    let out =
        animation::evaluate(glb.snapshot(), clip, Time::new(1, 1).unwrap(), || false).unwrap();
    for (a, b) in points(&out.snapshot, mesh_entity(glb.snapshot()))
        .into_iter()
        .zip(reference(1.))
    {
        assert!((a - b).length() < 2e-7);
    }
}
fn changed(f: impl FnOnce(&mut Value, &mut Vec<u8>)) -> Result<Imported> {
    let mut v: Value = serde_json::from_slice(JSON).unwrap();
    let mut b = BIN.to_vec();
    f(&mut v, &mut b);
    import(&serde_json::to_vec(&v).unwrap(), &b)
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn malformed_and_unsupported_animation_inputs_fail_without_publication() {
    let mutations: Vec<fn(&mut Value, &mut Vec<u8>)> = vec![
        |v, _| v["animations"][0]["channels"][0]["target"]["node"] = json!(99),
        |v, _| v["animations"][0]["channels"][0]["sampler"] = json!(99),
        |v, _| v["animations"][0]["samplers"][0]["interpolation"] = json!(5),
        |v, _| v["animations"][0]["samplers"][0]["interpolation"] = json!("CATMULLROM"),
        |v, _| v["animations"][0]["channels"][1] = v["animations"][0]["channels"][0].clone(),
        |v, _| v["accessors"][6]["count"] = json!(2),
        |v, _| v["accessors"][7]["componentType"] = json!(5123),
        |v, _| v["accessors"][0]["sparse"] = json!({}),
        |v, _| v["skins"][0]["joints"] = json!([0, 0]),
        |v, _| v["skins"][0]["joints"] = json!([0, 99]),
        |v, _| v["skins"][0]["skeleton"] = json!(2),
        |v, _| v["accessors"][5]["count"] = json!(1),
        |v, _| v["meshes"][0]["weights"] = json!([0.2, 0.3]),
        |v, _| v["meshes"][0]["primitives"][0]["targets"][0]["NORMAL"] = json!(4),
        |v, _| {
            v["meshes"][0]["primitives"][0]["attributes"]
                .as_object_mut()
                .unwrap()
                .remove("WEIGHTS_0")
                .map(|_| ())
                .unwrap()
        },
        |v, _| v["accessors"][3]["normalized"] = json!(false),
        |v, _| v["nodes"][1]["matrix"] = json!([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
        |v, _| v["animations"][0]["samplers"][0]["extensions"] = json!({"evil":{}}),
        |v, b| {
            let off = v["bufferViews"][6]["byteOffset"].as_u64().unwrap() as usize;
            b[off + 4..off + 8].copy_from_slice(&0f32.to_le_bytes());
        },
        |v, b| {
            let off = v["bufferViews"][7]["byteOffset"].as_u64().unwrap() as usize;
            b[off..off + 4].copy_from_slice(&f32::NAN.to_le_bytes());
        },
        |v, b| {
            let off = v["bufferViews"][2]["byteOffset"].as_u64().unwrap() as usize;
            b[off] = 99;
        },
        |v, b| {
            let off = v["bufferViews"][3]["byteOffset"].as_u64().unwrap() as usize;
            b[off] = 0;
        },
    ];
    assert!(
        changed(|v, _| {
            v["skins"][0].as_object_mut().unwrap().remove("skeleton");
            v["skins"][0]["joints"] = json!([0, 2]);
        })
        .is_err()
    );
    assert!(
        changed(|v, b| {
            let off = v["bufferViews"][8]["byteOffset"].as_u64().unwrap() as usize;
            b[off..off + 4].copy_from_slice(&9f32.to_le_bytes());
        })
        .is_err()
    );
    assert!(
        changed(|v, b| {
            let off = v["bufferViews"][7]["byteOffset"].as_u64().unwrap() as usize;
            b[off + 12..off + 16].copy_from_slice(&2f32.to_le_bytes());
        })
        .is_err()
    );
    for (i, mutation) in mutations.into_iter().enumerate() {
        assert!(changed(mutation).is_err(), "case {i}");
    }
    let (mut doc, report) = document();
    let before = canonical(&doc).unwrap();
    let state = doc.snapshot().animation.clone().unwrap();
    assert!(
        apply(
            &mut doc,
            vec![Command::MergeAnimation { animation: state }],
            "animated-collision:01"
        )
        .is_err()
    );
    assert_eq!(canonical(&doc).unwrap(), before);
    let imported = import(JSON, BIN).unwrap();
    let error = doc
        .execute(
            &fixtures::principal(),
            &Request {
                version: 0,
                base_revision: "stale".into(),
                idempotency_key: "animated-stale:01".into(),
                commands: imported.commands,
                max_added_bytes: 1000000,
            },
        )
        .unwrap_err();
    assert_eq!(error.code, "stale_revision");
    assert_eq!(canonical(&doc).unwrap(), before);
    let mut old = doc.snapshot().clone();
    old.version = 10;
    assert!(old.validate().is_err());
    let mut state = doc.snapshot().animation.clone().unwrap();
    let clip = state.clips.get_mut(&report.source_clips[&0]).unwrap();
    clip.tracks[0].property = animation::Property::Quaternion;
    assert!(clip.validate().is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn skin_specific_inverse_binds_defaults_and_multiple_imports_are_preserved() {
    let mut value: Value = serde_json::from_slice(JSON).unwrap();
    // Missing inverseBindMatrices means identity, independent of rest pose.
    value["skins"][0]
        .as_object_mut()
        .unwrap()
        .remove("inverseBindMatrices");
    value["nodes"][2]["weights"] = json!([0.5]);
    let imported = import(&serde_json::to_vec(&value).unwrap(), BIN).unwrap();
    let clip = imported.report.source_clips[&0];
    let mut doc = Document::new(Snapshot::empty(Id(99))).unwrap();
    apply(&mut doc, imported.commands, "identity-bind:01").unwrap();
    let s = doc.snapshot();
    let entity = mesh_entity(s);
    let out = animation::evaluate(s, clip, Time::new(0, 1).unwrap(), || false).unwrap();
    let p = points(&out.snapshot, entity);
    assert!((p[0] - DVec3::new(-0.25, 0., 0.)).length() < 1e-9);
    assert!((p[2] - DVec3::new(-0.3, 3., 0.05)).length() < 1e-7);
    let scene = Evaluator::default().evaluate(s).unwrap();
    assert_eq!(scene.instances.len(), 1);
    assert_eq!(
        s.animation.as_ref().unwrap().morphs[&entity][0].default_weight,
        0.5
    );
    // A second distinct source must preserve existing components.
    let imported = import(JSON, BIN).unwrap();
    apply(&mut doc, imported.commands, "second-import:01").unwrap();
    assert_eq!(doc.snapshot().animation.as_ref().unwrap().clips.len(), 2);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn external_khronos_character_imports_and_renders_at_random_times() {
    let buffers = vec![
        include_bytes!(
            "../../../fixtures/animated-gltf/khronos-simple-skin/SimpleSkin_geometry.bin"
        )
        .to_vec(),
        include_bytes!(
            "../../../fixtures/animated-gltf/khronos-simple-skin/SimpleSkin_skinningData.bin"
        )
        .to_vec(),
        include_bytes!(
            "../../../fixtures/animated-gltf/khronos-simple-skin/SimpleSkin_inverseBindMatrices.bin"
        )
        .to_vec(),
        include_bytes!(
            "../../../fixtures/animated-gltf/khronos-simple-skin/SimpleSkin_animation.bin"
        )
        .to_vec(),
    ];
    let imported = import_pbr_scene(
        include_bytes!("../../../fixtures/animated-gltf/khronos-simple-skin/SimpleSkin.gltf"),
        &buffers,
        &[],
        Id(99),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    let clip = imported.report.source_clips[&0];
    let mut doc = Document::new(Snapshot::empty(Id(99))).unwrap();
    apply(&mut doc, imported.commands, "external-import:01").unwrap();
    let mut settings = fixtures::settings();
    settings.width = 16;
    settings.height = 16;
    settings.samples = 1;
    settings.camera.position = [0., 1., 5.];
    settings.camera.target = [0., 1., 0.];
    let revision = doc.snapshot().revision().unwrap();
    let mut hashes = std::collections::BTreeSet::new();
    for (n, d) in [(0, 1), (1, 1), (7, 2), (1, 2)] {
        let (scene, _) = Evaluator::default()
            .evaluate_at(doc.snapshot(), clip, Time::new(n, d).unwrap(), || false)
            .unwrap();
        let image = render::render(&scene, &settings, || false).unwrap();
        assert!(image.objects.iter().any(Option::is_some));
        hashes.insert(digest(&image.pfm()));
    }
    assert!(hashes.len() > 1);
    assert_eq!(doc.snapshot().revision().unwrap(), revision);
}

fn add_floats(v: &mut Value, b: &mut Vec<u8>, shape: &str, rows: &[Vec<f32>]) -> usize {
    while !b.len().is_multiple_of(4) {
        b.push(0);
    }
    let start = b.len();
    for x in rows.iter().flatten() {
        b.extend_from_slice(&x.to_le_bytes());
    }
    let views = v["bufferViews"].as_array_mut().unwrap();
    let view = views.len();
    views.push(json!({"buffer":0,"byteOffset":start,"byteLength":b.len()-start}));
    let accessors = v["accessors"].as_array_mut().unwrap();
    let a = accessors.len();
    accessors.push(json!({"bufferView":view,"componentType":5126,"count":rows.len(),"type":shape}));
    v["buffers"][0]["byteLength"] = json!(b.len());
    a
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn absolute_nonuniform_scale_cubic_quaternions_and_vector_morph_channels() {
    let mut v: Value = serde_json::from_slice(JSON).unwrap();
    let mut b = BIN.to_vec();
    // Two morph targets exercise glTF's scalar packing (key, tangent, target).
    v["meshes"][0]["primitives"][0]["targets"] = json!([{"POSITION":4},{"POSITION":4}]);
    v["meshes"][0]["weights"] = json!([0.25, 0.5]);
    let mw = add_floats(
        &mut v,
        &mut b,
        "SCALAR",
        &[
            vec![0.],
            vec![0.],
            vec![0.],
            vec![1.],
            vec![0.],
            vec![0.],
            vec![0.],
            vec![0.],
            vec![1.],
            vec![0.],
            vec![0.],
            vec![0.],
            vec![0.],
            vec![0.],
            vec![0.],
            vec![1.],
            vec![0.],
            vec![0.],
        ],
    );
    v["animations"][0]["samplers"][1]["output"] = json!(mw);
    v["animations"][0]["samplers"][1]["interpolation"] = json!("CUBICSPLINE");
    let q = add_floats(
        &mut v,
        &mut b,
        "VEC4",
        &[
            vec![0.; 4],
            vec![0., 0., 0., 1.],
            vec![0.; 4],
            vec![0.; 4],
            vec![0., 0., 1., 0.],
            vec![0.; 4],
            vec![0.; 4],
            vec![0., 0., 0., -1.],
            vec![0.; 4],
        ],
    );
    v["animations"][0]["samplers"][0]["output"] = json!(q);
    v["animations"][0]["samplers"][0]["interpolation"] = json!("CUBICSPLINE");
    // Negative/nonuniform authored scale must not be multiplied into replacement keys.
    v["nodes"][0]["scale"] = json!([-2., 3., 1.]);
    let scale = add_floats(
        &mut v,
        &mut b,
        "VEC3",
        &[vec![-1., 2., 1.], vec![-3., 4., 1.], vec![-1., 2., 1.]],
    );
    v["animations"][0]["samplers"]
        .as_array_mut()
        .unwrap()
        .push(json!({"input":6,"output":scale,"interpolation":"LINEAR"}));
    v["animations"][0]["channels"]
        .as_array_mut()
        .unwrap()
        .push(json!({"sampler":3,"target":{"node":0,"path":"scale"}}));
    let imported = import(&serde_json::to_vec(&v).unwrap(), &b).unwrap();
    let hip = imported.report.source_nodes[&0];
    let shoulder = imported.report.source_nodes[&1];
    let clip = imported.report.source_clips[&0];
    let mut doc = Document::new(Snapshot::empty(Id(99))).unwrap();
    apply(&mut doc, imported.commands, "cubic-import:0001").unwrap();
    let sample = doc.snapshot().animation.as_ref().unwrap().clips[&clip]
        .sample(Time::new(1, 2).unwrap(), || false)
        .unwrap();
    let weights: Vec<_> = sample.morphs.values().copied().collect();
    assert_eq!(weights, vec![0.5, 0.5]);
    let rotation = sample.poses[&animation::Target::Entity { entity: shoulder }]
        .affine()
        .unwrap();
    assert!((rotation.transform_vector3(DVec3::X) - DVec3::Y).length() < 1e-9);
    let out =
        animation::evaluate(doc.snapshot(), clip, Time::new(1, 2).unwrap(), || false).unwrap();
    let transform = out.snapshot.world_transform(hip).unwrap();
    assert!((transform.transform_vector3(DVec3::X) - DVec3::new(-2., 0., 0.)).length() < 1e-9);
    assert!((transform.transform_vector3(DVec3::Y) - DVec3::new(0., 3., 0.)).length() < 1e-9);
    // Static evaluation uses the node's default weights, not the first animation key.
    let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    let instance = &scene.instances[0];
    assert!(instance.bounds.min.z >= 0.);
    assert!((instance.bounds.max.z - 0.15).abs() < 1e-7);
}
