use glam::{DAffine3, DQuat, DVec3};
use render_core::{animation::*, document::*, rigging::*, *};
use std::collections::BTreeMap;
fn time(n: i64, d: u64) -> Time {
    Time::new(n, d).unwrap()
}
fn key(n: i64, value: Value) -> Key {
    Key {
        time: time(n, 1),
        value,
        incoming: None,
        outgoing: None,
    }
}
fn vector() -> Track {
    Track {
        id: Id(1),
        target: Target::Entity { entity: Id(1) },
        property: Property::Translation,
        interpolation: Interpolation::Linear,
        keys: vec![
            key(0, Value::Vector([0.; 3])),
            key(2, Value::Vector([8.; 3])),
        ],
    }
}
fn transform(a: DAffine3) -> Transform {
    Transform {
        columns: a.to_cols_array_2d(),
        operations: vec![],
    }
}
fn rig() -> Rig {
    Rig {
        joints: vec![
            Joint {
                id: Id(1),
                parent: None,
                rest: Transform::default(),
                inverse_bind: Transform::default(),
            },
            Joint {
                id: Id(2),
                parent: Some(Id(1)),
                rest: Transform::translation(1., 0., 0.),
                inverse_bind: Transform::translation(-1., 0., 0.),
            },
            Joint {
                id: Id(3),
                parent: Some(Id(2)),
                rest: Transform::translation(1., 0., 0.),
                inverse_bind: Transform::translation(-2., 0., 0.),
            },
        ],
        constraints: vec![],
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn rational_time_and_interpolation_preserve_authored_modes() {
    assert_eq!(time(-1, 3).modulo(time(1, 1)).unwrap(), time(2, 3));
    assert_eq!(
        time(i64::MIN, 1).subtract(time(i64::MIN, 1)).unwrap(),
        time(0, 1)
    );
    assert_eq!(Time::frame(i64::MAX, 2, 2).unwrap(), time(i64::MAX, 1));
    assert!(time(i64::MAX, 1).add_time(time(1, 1)).is_err());
    let mut t = vector();
    assert_eq!(t.sample(time(1, 1)).unwrap(), Value::Vector([4.; 3]));
    t.interpolation = Interpolation::Step;
    assert_eq!(t.sample(time(1, 1)).unwrap(), Value::Vector([0.; 3]));
    assert_eq!(t.sample(time(2, 1)).unwrap(), Value::Vector([8.; 3]));
    t.interpolation = Interpolation::Cubic;
    t.keys[0].incoming = Some(Value::Vector([0.; 3]));
    t.keys[0].outgoing = Some(Value::Vector([0.; 3]));
    t.keys[1].incoming = Some(Value::Vector([12.; 3]));
    t.keys[1].outgoing = Some(Value::Vector([12.; 3]));
    assert_eq!(t.sample(time(1, 1)).unwrap(), Value::Vector([1.; 3]));
    let q = DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2);
    let mut t = vector();
    t.property = Property::Quaternion;
    t.keys = vec![
        key(0, Value::Quaternion(DQuat::IDENTITY.to_array())),
        key(2, Value::Quaternion((-q).to_array())),
    ];
    let Value::Quaternion(mid) = t.sample(time(1, 1)).unwrap() else {
        panic!()
    };
    let p = DQuat::from_array(mid) * DVec3::X;
    assert!((p - DVec3::new(0.5f64.sqrt(), 0.5f64.sqrt(), 0.)).length() < 1e-12);
    t.property = Property::Euler {
        order: RotationOrder::Zyx,
    };
    t.keys = vec![
        key(0, Value::Vector([0.; 3])),
        key(2, Value::Vector([4. * std::f64::consts::PI, 0., 0.])),
    ];
    assert_eq!(
        t.sample(time(1, 2)).unwrap(),
        Value::Vector([std::f64::consts::PI, 0., 0.])
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn clip_random_access_retime_conflicts_and_cancellation() {
    let mut c = Clip {
        id: Id(1),
        start: time(0, 1),
        end: time(2, 1),
        extrapolation: Extrapolation::Repeat,
        remap: TimeMap {
            rate: time(-1, 1),
            offset: time(0, 1),
        },
        tracks: vec![vector()],
    };
    for n in [3, -1, 7, 0, 1, -3, 3] {
        let sample = c.sample(time(n, 1), || false).unwrap();
        let expected = if n % 2 == 0 { 0. } else { 4. };
        assert_eq!(
            sample.poses[&Target::Entity { entity: Id(1) }].translation,
            Some([expected; 3])
        );
    }
    assert_eq!(c.sample(time(0, 1), || true).unwrap_err().code, "cancelled");
    c.tracks.push(c.tracks[0].clone());
    assert_eq!(c.validate().unwrap_err().code, "channel");
    let mut t = vector();
    t.keys[1].time = t.keys[0].time;
    assert_eq!(t.validate().unwrap_err().code, "channel");
    t = vector();
    t.property = Property::Quaternion;
    t.interpolation = Interpolation::Cubic;
    t.keys = vec![
        Key {
            time: time(0, 1),
            value: Value::Quaternion([0., 0., 0., 1.]),
            incoming: Some(Value::Quaternion([0.; 4])),
            outgoing: Some(Value::Quaternion([0.; 4])),
        },
        Key {
            time: time(2, 1),
            value: Value::Quaternion([0., 0., 0., -1.]),
            incoming: Some(Value::Quaternion([0.; 4])),
            outgoing: Some(Value::Quaternion([0.; 4])),
        },
    ];
    assert_eq!(t.sample(time(1, 1)).unwrap_err().code, "quaternion");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn bind_affine_scale_shear_and_two_bone_solver_diagnostics() {
    let mut r = rig();
    let plan = r.compile().unwrap();
    let pose = plan
        .evaluate(Id(1), &BTreeMap::new(), |_| panic!(), || false)
        .unwrap();
    for (a, b) in pose.iter().zip(&plan.inverse_bind) {
        assert!((*a * *b).abs_diff_eq(DAffine3::IDENTITY, 1e-12));
    }
    r.constraints.push(Constraint::TwoBoneIk {
        id: Id(11),
        root: Id(1),
        mid: Id(2),
        tip: Id(3),
        target: Goal::Point {
            position: [1., 1., 0.],
        },
        pole: [0., 1., 0.],
        tolerance: 1e-8,
    });
    let solved = r
        .compile()
        .unwrap()
        .evaluate(Id(1), &BTreeMap::new(), |_| panic!(), || false)
        .unwrap();
    assert!(solved[2].translation.distance(DVec3::new(1., 1., 0.)) < 1e-8);
    assert!((solved[1].translation.distance(solved[0].translation) - 1.).abs() < 1e-12);
    let Constraint::TwoBoneIk { target, .. } = &mut r.constraints[0] else {
        panic!()
    };
    *target = Goal::Point {
        position: [5., 0., 0.],
    };
    let e = r
        .compile()
        .unwrap()
        .evaluate(Id(1), &BTreeMap::new(), |_| panic!(), || false)
        .unwrap_err();
    assert_eq!(e.code, "ik_unreachable");
    assert!(e.context.contains_key("constraint"));
    let Constraint::TwoBoneIk { target, .. } = &mut r.constraints[0] else {
        panic!()
    };
    *target = Goal::Joint { joint: Id(3) };
    assert_eq!(r.compile().unwrap_err().code, "constraint_cycle");
    r = rig();
    r.joints[0].parent = Some(Id(3));
    assert_eq!(r.compile().unwrap_err().code, "rig_cycle");
    r = rig();
    r.joints[1].inverse_bind = Transform::default();
    assert_eq!(r.compile().unwrap_err().code, "bind_pose");
    r = rig();
    let mut a = DAffine3::from_scale(DVec3::new(-2., 1., 0.5));
    a.matrix3.y_axis.x = 0.3;
    r.joints[0].rest = transform(a);
    for i in 0..3 {
        let global = a * DAffine3::from_translation(DVec3::X * i as f64);
        r.joints[i].inverse_bind = transform(global.inverse());
    }
    let p = r.compile().unwrap();
    let posed = p
        .evaluate(Id(1), &BTreeMap::new(), |_| panic!(), || false)
        .unwrap();
    for (a, b) in posed.iter().zip(&p.inverse_bind) {
        assert!((*a * *b).abs_diff_eq(DAffine3::IDENTITY, 1e-12));
    }
    assert_eq!(
        p.evaluate(Id(1), &BTreeMap::new(), |_| panic!(), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn character_skin_morph_transaction_random_access_and_native_state_recovery() {
    let mut d = feature_fixtures::character_document().unwrap();
    let before = canonical(&d).unwrap();
    let mut hashes = BTreeMap::new();
    for frame in [0, 1, 2, 3, 4, 3, 1, 0, 2] {
        let e = evaluate(d.snapshot(), Id(8400), time(frame, 4), || false).unwrap();
        assert_eq!(e.receipt.deformed_points, 56);
        let key = e
            .snapshot
            .entities
            .get(Id(8201))
            .unwrap()
            .mesh
            .clone()
            .unwrap();
        if let Some(old) = hashes.insert(frame, key.clone()) {
            assert_eq!(key, old);
        }
    }
    assert_ne!(hashes[&0], hashes[&2]);
    assert_eq!(canonical(&d).unwrap(), before);
    let recovered = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(canonical(&recovered).unwrap(), before);
    // With constraints disabled and no pose/morph deltas, every bind vertex is preserved.
    let mut rest = d.snapshot().clone();
    let state = rest.animation.as_mut().unwrap();
    state.rigs.get_mut(&Id(8200)).unwrap().constraints.clear();
    state
        .clips
        .get_mut(&Id(8400))
        .unwrap()
        .tracks
        .retain(|t| matches!(t.target, Target::Morph { .. }));
    let original = rest.meshes[rest.entities.get(Id(8201)).unwrap().mesh.as_ref().unwrap()].clone();
    let e = evaluate(&rest, Id(8400), time(0, 1), || false).unwrap();
    let bound = &e.snapshot.meshes[e
        .snapshot
        .entities
        .get(Id(8201))
        .unwrap()
        .mesh
        .as_ref()
        .unwrap()];
    for i in 0..original.positions.len() {
        assert!((original.positions.get(i) - bound.positions.get(i)).length() < 1e-12);
    }
    let mut invalid = d.snapshot().animation.clone().unwrap();
    invalid
        .skins
        .get_mut(&Id(8201))
        .unwrap()
        .weights
        .values_mut()
        .next()
        .unwrap()[0]
        .weight = 0.5;
    let req = fixtures::request(
        &d,
        "anim:invalid:0001",
        vec![Command::SetAnimation {
            animation: Some(invalid),
        }],
    )
    .unwrap();
    assert_eq!(
        d.execute(&fixtures::principal(), &req).unwrap_err().code,
        "skin_weights"
    );
    assert_eq!(canonical(&d).unwrap(), before);
    let mut req = fixtures::request(
        &d,
        "anim:stale:000001",
        vec![Command::SetAnimation { animation: None }],
    )
    .unwrap();
    req.base_revision = "stale".into();
    assert_eq!(
        d.execute(&fixtures::principal(), &req).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(canonical(&d).unwrap(), before);
    assert_eq!(
        evaluate(d.snapshot(), Id(8400), time(0, 1), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    let mut calls = 0;
    assert_eq!(
        evaluate(d.snapshot(), Id(8400), time(0, 1), || {
            calls += 1;
            calls == 12
        })
        .unwrap_err()
        .code,
        "cancelled"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn weighted_affine_skinning_and_morph_have_independent_numeric_expectations() {
    let d = feature_fixtures::character_document().unwrap();
    let mut s = d.snapshot().clone();
    let state = s.animation.as_mut().unwrap();
    state.rigs.get_mut(&Id(8200)).unwrap().constraints.clear();
    let mesh = &s.meshes[s.entities.get(Id(8201)).unwrap().mesh.as_ref().unwrap()];
    let point = mesh.point_ids[0];
    let p = mesh.positions.get(0);
    state.skins.get_mut(&Id(8201)).unwrap().weights.insert(
        point,
        vec![
            Influence {
                joint: Id(8100),
                weight: 0.25,
            },
            Influence {
                joint: Id(8101),
                weight: 0.75,
            },
        ],
    );
    state.clips.get_mut(&Id(8400)).unwrap().tracks = vec![Track {
        id: Id(1),
        target: Target::Joint {
            rig: Id(8200),
            joint: Id(8101),
        },
        property: Property::Quaternion,
        interpolation: Interpolation::Step,
        keys: vec![key(
            0,
            Value::Quaternion(DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2).to_array()),
        )],
    }];
    let e = evaluate(&s, Id(8400), time(0, 1), || false).unwrap();
    let output = &e.snapshot.meshes[e
        .snapshot
        .entities
        .get(Id(8201))
        .unwrap()
        .mesh
        .as_ref()
        .unwrap()];
    let pivot = DVec3::new(0.35, 0.65, 0.);
    let expected = p * 0.25
        + (pivot + DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2) * (p - pivot)) * 0.75;
    assert!(output.positions.get(0).distance(expected) < 1e-12);
    let state = s.animation.as_mut().unwrap();
    state.clips.get_mut(&Id(8400)).unwrap().tracks = vec![Track {
        id: Id(1),
        target: Target::Morph {
            entity: Id(8201),
            target: Id(8300),
        },
        property: Property::MorphWeight,
        interpolation: Interpolation::Step,
        keys: vec![key(0, Value::Scalar(0.5))],
    }];
    let mesh = &s.meshes[s.entities.get(Id(8201)).unwrap().mesh.as_ref().unwrap()];
    let p = mesh.positions.get(8);
    let delta = DVec3::from_array(state.morphs[&Id(8201)][0].offsets[&mesh.point_ids[8]]);
    let e = evaluate(&s, Id(8400), time(0, 1), || false).unwrap();
    let output = &e.snapshot.meshes[e
        .snapshot
        .entities
        .get(Id(8201))
        .unwrap()
        .mesh
        .as_ref()
        .unwrap()];
    assert!(output.positions.get(8).distance(p + delta * 0.5) < 1e-12);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn shutter_sequence_pinned_revisions_partial_output_and_agent_preview() {
    use render_core::sequence::*;
    let d = feature_fixtures::character_document().unwrap();
    let mut s = d.snapshot().clone();
    let settings = s.render_settings.as_mut().unwrap();
    settings.width = 24;
    settings.height = 24;
    settings.samples = 1;
    let revision = s.revision().unwrap();
    let shutter = Shutter {
        open: time(-1, 4),
        close: time(1, 4),
        samples: 4,
    };
    assert_eq!(
        shutter.times(time(1, 2)).unwrap(),
        vec![time(5, 16), time(7, 16), time(9, 16), time(11, 16)]
    );
    let mut req = FrameRequest {
        revision: revision.clone(),
        clip: Id(8400),
        time: time(1, 2),
        shutter,
    };
    let blur = render_frame(&s, &req, || false).unwrap();
    req.shutter = Shutter::instant();
    let instant = render_frame(&s, &req, || false).unwrap();
    assert_eq!(blur.image.objects, instant.image.objects);
    assert_ne!(
        blur.image.receipt.output_digest,
        instant.image.receipt.output_digest
    );
    assert_eq!(blur.evaluation.authored_revision, revision);
    req.revision = "stale".into();
    assert_eq!(
        render_frame(&s, &req, || false).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(
        render_frame(&s, &req, || true).unwrap_err().code,
        "cancelled"
    );
    let request = SequenceRequest {
        revision: revision.clone(),
        clip: Id(8400),
        times: vec![time(0, 1), time(1, 2), time(1, 1)],
        shutter: Shutter::instant(),
    };
    let emitted = std::cell::Cell::new(0);
    let e = render_sequence(
        &s,
        &request,
        |_, _| {
            emitted.set(emitted.get() + 1);
            Ok(())
        },
        || emitted.get() > 0,
    )
    .unwrap_err();
    assert_eq!(e.code, "cancelled");
    assert_eq!(emitted.get(), 1);
    let mut agent = agent::Session::new(Document::new(s.clone()).unwrap());
    let principal = fixtures::principal();
    agent
        .dispatch(
            &principal,
            agent::Request {
                version: 0,
                operation: agent::Operation::Branch {
                    branch: "clip".into(),
                    base_revision: revision.clone(),
                },
            },
        )
        .unwrap();
    req.revision = revision;
    let rendered = agent
        .preview_at(&principal, "clip", &req, || false)
        .unwrap();
    assert!(rendered["inspection"]["visible_pixels"].as_u64().unwrap() > 20);
    assert_eq!(
        canonical(agent.document().snapshot()).unwrap(),
        canonical(&s).unwrap()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn quaternion_layer_schema_gate_and_transaction_upgrade() {
    let mut d = fixtures::demo().unwrap();
    let entity = d.snapshot().entities.iter().next().unwrap().id;
    let layer = Layer {
        name: "quaternion-layer".into(),
        overrides: BTreeMap::from([(
            entity,
            Override {
                transform: Some(Transform {
                    operations: vec![TransformOp::Quaternion([0., 0., 0., 1.])],
                    ..Transform::default()
                }),
                material: None,
                hidden: false,
            },
        )]),
    };
    let mut old = d.snapshot().clone();
    old.layers.push(layer.clone());
    assert!(old.validate().is_err());
    let request =
        fixtures::request(&d, "quaternion:layer:01", vec![Command::PutLayer { layer }]).unwrap();
    d.execute(&fixtures::principal(), &request).unwrap();
    assert_eq!(d.snapshot().version, 6);
    d.snapshot().validate().unwrap();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sequence_preflights_all_exact_times_and_rejects_invalid_dimensions_without_overflow() {
    use render_core::sequence::*;
    let doc = feature_fixtures::character_document().unwrap();
    let mut snapshot = doc.snapshot().clone();
    let mut request = SequenceRequest {
        revision: snapshot.revision().unwrap(),
        clip: Id(8400),
        times: vec![time(0, 1), time(i64::MAX, 1)],
        shutter: Shutter {
            open: time(0, 1),
            close: time(2, 1),
            samples: 1,
        },
    };
    assert_eq!(
        render_sequence(
            &snapshot,
            &request,
            |_, _| panic!("invalid later time must fail before any publication"),
            || false
        )
        .unwrap_err()
        .code,
        "overflow"
    );
    let settings = snapshot.render_settings.as_mut().unwrap();
    settings.width = u32::MAX;
    settings.height = u32::MAX;
    settings.max_bytes = u64::MAX;
    request.times = vec![time(0, 1)];
    assert_eq!(
        request.validate(&snapshot).unwrap_err().code,
        "render_settings"
    );
}
