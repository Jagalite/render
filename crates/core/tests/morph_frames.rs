use glam::DVec3;
use render_core::{animation, document::*, geometry::*, gltf_scene::*, render::*, *};
use serde_json::{Value, json};
const JSON: &[u8] = include_bytes!("../../../fixtures/morph-frames/data/dense.gltf");
const BIN: &[u8] = include_bytes!("../../../fixtures/morph-frames/data/dense.bin");
const SPARSE_JSON: &[u8] = include_bytes!("../../../fixtures/morph-frames/data/sparse.gltf");
const SPARSE_BIN: &[u8] = include_bytes!("../../../fixtures/morph-frames/data/sparse.bin");
fn imported(v: &Value, bin: &[u8]) -> Result<Imported> {
    import_pbr_scene(
        &serde_json::to_vec(v).unwrap(),
        &[bin.to_vec()],
        &[],
        Id(10000),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
}
fn source() -> Value {
    serde_json::from_slice(JSON).unwrap()
}
fn document(v: &Value, bin: &[u8]) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(10000))).unwrap();
    let commands = imported(v, bin).unwrap().commands;
    let r = fixtures::request(&d, "morph:frames:import:01", commands).unwrap();
    d.execute(&fixtures::principal(), &r).unwrap();
    d
}
fn mesh(s: &Snapshot) -> &Mesh {
    let e = s.entities.iter().find(|e| e.mesh.is_some()).unwrap();
    &s.meshes[e.mesh.as_ref().unwrap()]
}
fn vectors(m: &Mesh, name: &str) -> Vec<DVec3> {
    let AttributeValues::Vec3(v) = &m.attributes[name].values else {
        panic!("vec3")
    };
    v.iter()
        .map(|v| DVec3::from_array(v.map(f64::from)))
        .collect()
}
fn clip(d: &Document) -> Id {
    *d.snapshot()
        .animation
        .as_ref()
        .unwrap()
        .clips
        .keys()
        .next()
        .unwrap()
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn morph_directions_and_affine_skin_match_analytic_frames_and_recovery() {
    for (v, bin) in [
        (source(), BIN),
        (serde_json::from_slice(SPARSE_JSON).unwrap(), SPARSE_BIN),
    ] {
        let d = document(&v, bin);
        assert_eq!(d.snapshot().version, 13);
        let before = canonical(&d).unwrap();
        for (n, den) in [(1, 1), (0, 1), (1, 2), (1, 1), (0, 1)] {
            let t = n as f64 / den as f64;
            let out =
                animation::evaluate(d.snapshot(), clip(&d), Time::new(n, den).unwrap(), || false)
                    .unwrap();
            let m = mesh(&out.snapshot);
            let entity = out
                .snapshot
                .entities
                .iter()
                .find(|e| e.mesh.is_some())
                .unwrap();
            let world = out.snapshot.world_transform(entity.id).unwrap();
            for (i, [x, y]) in [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]]
                .iter()
                .enumerate()
            {
                assert!(
                    world
                        .transform_point3(m.positions.get(i))
                        .distance(DVec3::new(1.5 * (x + 0.1 * t), 2. * y, 0.))
                        < 1e-7
                );
            }
            for v in vectors(m, "normal") {
                assert!(
                    v.distance(DVec3::new(t / 3., 0., 1.).normalize()) < 1e-7,
                    "{v:?}"
                );
            }
            for v in vectors(m, "tangent") {
                assert!(
                    v.distance(DVec3::new(1.5, 0.5 * t, 0.).normalize()) < 1e-7,
                    "{v:?}"
                );
            }
            assert!(out.receipt.approximation.contains("authored-frames-v1"));
        }
        let restored = storage::Envelope::new(0, None, &d)
            .unwrap()
            .decode()
            .unwrap();
        assert_eq!(canonical(&restored).unwrap(), before);
        let mut settings = fixtures::settings();
        settings.width = 16;
        settings.height = 16;
        settings.samples = 4;
        let a = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let b = Evaluator::default().evaluate(restored.snapshot()).unwrap();
        assert_eq!(
            render(&a, &settings, || false).unwrap().linear,
            render(&b, &settings, || false).unwrap().linear
        );
        assert_eq!(canonical(&d).unwrap(), before);
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn morph_reflection_and_singular_blends_are_explicit() {
    let mut v = source();
    v["nodes"][2]["scale"] = json!([-2, 1, 1]);
    v["nodes"][3]["scale"] = json!([-1, 3, 1]);
    let d = document(&v, BIN);
    let out =
        animation::evaluate(d.snapshot(), clip(&d), Time::new(1, 1).unwrap(), || false).unwrap();
    let m = mesh(&out.snapshot);
    for n in vectors(m, "normal") {
        assert!(n.distance(DVec3::new(-1. / 3., 0., 1.).normalize()) < 1e-7);
    }
    for t in vectors(m, "tangent") {
        assert!(t.distance(DVec3::new(-1.5, 0.5, 0.).normalize()) < 1e-7);
    }
    let AttributeValues::Scalar(sign) = &m.attributes["tangent_sign"].values else {
        panic!("sign")
    };
    assert!(sign.iter().all(|v| *v == -1.));
    v["nodes"][3]["scale"] = json!([2, 3, 1]);
    let d = document(&v, BIN);
    let before = canonical(&d).unwrap();
    assert_eq!(
        animation::evaluate(d.snapshot(), clip(&d), Time::new(1, 1).unwrap(), || false)
            .err()
            .unwrap()
            .code,
        "singular_pose"
    );
    assert_eq!(canonical(&d).unwrap(), before);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn morph_frame_import_rejects_missing_base_unknown_semantics_and_bad_counts() {
    let mut v = source();
    v["meshes"][0]["primitives"][0]["attributes"]
        .as_object_mut()
        .unwrap()
        .remove("NORMAL");
    assert!(imported(&v, BIN).is_err());
    let mut v = source();
    v["meshes"][0]["primitives"][0]["targets"][1] = json!({"COLOR_0":0});
    assert!(imported(&v, BIN).is_err());
    let mut v = source();
    let a = v["meshes"][0]["primitives"][0]["targets"][1]["NORMAL"]
        .as_u64()
        .unwrap() as usize;
    v["accessors"][a]["count"] = json!(3);
    assert!(imported(&v, BIN).is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn morph_frame_versions_invalid_edits_zero_directions_and_cancellation_are_atomic() {
    let mut d = document(&source(), BIN);
    let before = canonical(&d).unwrap();
    let mut old = d.snapshot().clone();
    old.version = 12;
    assert!(old.validate().is_err());
    let mut state = d.snapshot().animation.clone().unwrap();
    state.shading_frames.clear();
    let request = fixtures::request(
        &d,
        "morph:frames:invalid:01",
        vec![Command::SetAnimation {
            animation: Some(state),
        }],
    )
    .unwrap();
    assert!(d.execute(&fixtures::principal(), &request).is_err());
    assert_eq!(canonical(&d).unwrap(), before);
    let mut total_checks = 0;
    animation::evaluate(d.snapshot(), clip(&d), Time::new(1, 1).unwrap(), || {
        total_checks += 1;
        false
    })
    .unwrap();
    assert!(total_checks > 16);
    for stop in 0..total_checks {
        let mut count = 0;
        let r = animation::evaluate(d.snapshot(), clip(&d), Time::new(1, 1).unwrap(), || {
            count += 1;
            count > stop
        });
        assert_eq!(r.err().unwrap().code, "cancelled");
    }
    let mut state = d.snapshot().animation.clone().unwrap();
    let morph = state
        .morphs
        .values_mut()
        .next()
        .unwrap()
        .iter_mut()
        .find(|m| m.normal_offsets.is_some())
        .unwrap();
    for v in morph.normal_offsets.as_mut().unwrap().offsets.values_mut() {
        *v = [0., 0., -1.];
    }
    let mut s = d.snapshot().clone();
    s.animation = Some(state);
    assert_eq!(
        animation::evaluate(&s, clip(&d), Time::new(1, 1).unwrap(), || false)
            .err()
            .unwrap()
            .code,
        "deformation_frame"
    );
    assert_eq!(canonical(&d).unwrap(), before);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn native_corner_direction_offsets_preserve_seam_domain_and_stable_ids() {
    let d = document(&source(), BIN);
    let mut snapshot = d.snapshot().clone();
    let entity = *snapshot
        .animation
        .as_ref()
        .unwrap()
        .morphs
        .keys()
        .next()
        .unwrap();
    let mut m = mesh(&snapshot).clone();
    for name in ["normal", "tangent", "tangent_sign"] {
        let a = m.attributes.get_mut(name).unwrap();
        a.domain = Domain::Corner;
        a.values = match &a.values {
            AttributeValues::Vec3(v) => {
                AttributeValues::Vec3(m.corners.iter().map(|c| v[c.vertex as usize]).collect())
            }
            AttributeValues::Scalar(v) => {
                AttributeValues::Scalar(m.corners.iter().map(|c| v[c.vertex as usize]).collect())
            }
            _ => unreachable!(),
        };
    }
    let state = snapshot.animation.as_mut().unwrap();
    for morph in state.morphs.get_mut(&entity).unwrap() {
        for offsets in [&mut morph.normal_offsets, &mut morph.tangent_offsets]
            .into_iter()
            .flatten()
        {
            offsets.offsets = m
                .corner_ids
                .iter()
                .copied()
                .zip(
                    m.corners
                        .iter()
                        .map(|c| offsets.offsets[&m.point_ids[c.vertex as usize]]),
                )
                .collect();
        }
    }
    let key = m.content_id().unwrap();
    snapshot
        .meshes
        .insert(key.clone(), std::sync::Arc::new(m.clone()));
    snapshot.entities.get_mut(entity).unwrap().mesh = Some(key);
    snapshot.validate().unwrap();
    let out = animation::evaluate(&snapshot, clip(&d), Time::new(1, 1).unwrap(), || false).unwrap();
    let actual = mesh(&out.snapshot);
    assert_eq!(actual.corner_ids, m.corner_ids);
    assert_eq!(actual.attributes["normal"].domain, Domain::Corner);
    for n in vectors(actual, "normal") {
        assert!(n.distance(DVec3::new(1. / 3., 0., 1.).normalize()) < 1e-7);
    }
    let mut invalid = snapshot.clone();
    invalid
        .animation
        .as_mut()
        .unwrap()
        .shading_frames
        .get_mut(&entity)
        .unwrap()
        .tangent = None;
    assert_eq!(invalid.validate().unwrap_err().code, "deformation_frame");
}
