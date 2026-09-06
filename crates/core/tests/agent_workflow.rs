use render_core::{
    agent::{self, *},
    document::*,
    gltf_scene::*,
    render::*,
    storage::*,
    *,
};
use serde_json::{Value, json};
const GLB: &[u8] = include_bytes!("../../../fixtures/khronos-box/Box.glb");
const GLTF: &[u8] = include_bytes!("../../../fixtures/khronos-box/Box.gltf");
const BIN: &[u8] = include_bytes!("../../../fixtures/khronos-box/Box0.bin");
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 16;
    s.height = 16;
    s.samples = 2;
    s.max_depth = 1;
    s.camera = Camera {
        lens: None,
        position: [2., 1.5, 3.],
        target: [0.; 3],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.6,
    };
    s
}
fn call(s: &mut Session, operation: Operation) -> Result<Value> {
    s.dispatch(
        &fixtures::principal(),
        agent::Request {
            version: 0,
            operation,
        },
    )
}
fn imported() -> Session {
    let mut session = Session::new(Document::new(Snapshot::empty(Id(5))).unwrap());
    let base = session.document().snapshot().revision().unwrap();
    let op = Operation::ImportGlb {
        bytes: GLB.to_vec(),
        policy: Policy {
            allow_lambertian: true,
        },
        settings: settings(),
        base_revision: base,
        idempotency_key: "import:fixture:0001".into(),
    };
    let first = call(&mut session, op.clone()).unwrap();
    assert_eq!(first, call(&mut session, op).unwrap());
    assert!(!first["report"]["losses"].as_array().unwrap().is_empty());
    session
}
fn branch(s: &mut Session, name: &str) -> String {
    let base = s.document().snapshot().revision().unwrap();
    call(
        s,
        Operation::Branch {
            branch: name.into(),
            base_revision: base.clone(),
        },
    )
    .unwrap();
    base
}
fn modify(s: &mut Session, name: &str, base: String, color: [f32; 3]) -> (Operation, String) {
    let material = *s.document().snapshot().materials.keys().next().unwrap();
    let op = Operation::Modify {
        branch: name.into(),
        base_revision: base,
        idempotency_key: format!("modify:variant:{name}:0001"),
        edits: Edits {
            materials: vec![MaterialEdit {
                material,
                base_color: color,
                emission: [0.; 3],
            }],
            light: settings().light,
            environment: [0.25; 3],
        },
        max_added_bytes: 1024 * 1024,
    };
    let receipt = call(s, op.clone()).unwrap();
    (op, receipt["revision"].as_str().unwrap().into())
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn real_glb_and_gltf_preserve_indexed_geometry_and_hierarchy() {
    let session = imported();
    let s = session.document().snapshot();
    assert_eq!(s.entities.iter().count(), 3);
    assert_eq!(s.meshes.len(), 1);
    let mesh = s.meshes.values().next().unwrap();
    assert_eq!(mesh.positions.len(), 24);
    assert_eq!(mesh.triangles().unwrap().len(), 12);
    assert!(mesh.attributes.contains_key("normal"));
    let scene = Evaluator::default().evaluate(s).unwrap();
    assert_eq!(scene.instances.len(), 1);
    assert_eq!(scene.instances[0].bounds.min.to_array(), [-0.5; 3]);
    assert_eq!(scene.instances[0].bounds.max.to_array(), [0.5; 3]);
    let imported = import_scene(
        GLTF,
        &[BIN.to_vec()],
        Id(5),
        &Policy {
            allow_lambertian: true,
        },
    )
    .unwrap();
    let mut doc = Document::new(Snapshot::empty(Id(5))).unwrap();
    let req = fixtures::request(&doc, "scene:import:00001", imported.commands).unwrap();
    doc.execute(&fixtures::principal(), &req).unwrap();
    assert_eq!(
        doc.snapshot().meshes.keys().collect::<Vec<_>>(),
        s.meshes.keys().collect::<Vec<_>>()
    );
    let mut json: Value = serde_json::from_slice(GLTF).unwrap();
    json["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"mesh":0,"translation":[2,0,0],"scale":[-1,2,1]}));
    json["nodes"][0]["children"] = json!([1, 2]);
    let import = import_scene(
        &serde_json::to_vec(&json).unwrap(),
        &[BIN.to_vec()],
        Id(6),
        &Policy {
            allow_lambertian: true,
        },
    )
    .unwrap();
    let mut doc = Document::new(Snapshot::empty(Id(6))).unwrap();
    let req = fixtures::request(&doc, "scene:instances:01", import.commands).unwrap();
    doc.execute(&fixtures::principal(), &req).unwrap();
    let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    assert_eq!(scene.instances.len(), 2);
    assert_eq!(scene.geometry_builds, 1);
    assert!(std::sync::Arc::ptr_eq(
        &scene.instances[0].geometry,
        &scene.instances[1].geometry
    ));
    assert!(
        scene
            .instances
            .iter()
            .any(|i| i.transform.matrix3.determinant() < 0.)
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn importer_rejects_truncation_unsupported_features_and_invalid_accessors() {
    let policy = Policy {
        allow_lambertian: true,
    };
    for length in 0..GLB.len() {
        assert!(
            import_glb(&GLB[..length], Id(1), &policy).is_err(),
            "{length}"
        );
    }
    assert!(
        import_glb(
            GLB,
            Id(1),
            &Policy {
                allow_lambertian: false
            }
        )
        .is_err()
    );
    for (pointer, value) in [
        ("/nodes/0/children", json!([0])),
        ("/accessors/0/count", json!(37)),
        ("/accessors/0/byteOffset", json!(u64::MAX)),
        ("/bufferViews/1/byteStride", json!(8)),
        (
            "/materials/0/pbrMetallicRoughness/metallicFactor",
            json!(0.5),
        ),
    ] {
        let mut j: Value = serde_json::from_slice(GLTF).unwrap();
        *j.pointer_mut(pointer).unwrap() = value;
        assert!(
            import_scene(
                &serde_json::to_vec(&j).unwrap(),
                &[BIN.to_vec()],
                Id(1),
                &policy
            )
            .is_err(),
            "{pointer}"
        );
    }
    for (key, value) in [
        ("animations", json!([{}])),
        ("extensionsRequired", json!(["KHR_materials_transmission"])),
        (
            "images",
            json!([{"uri":"https://untrusted.invalid/asset.png"}]),
        ),
    ] {
        let mut j: Value = serde_json::from_slice(GLTF).unwrap();
        j[key] = value;
        assert_eq!(
            import_scene(
                &serde_json::to_vec(&j).unwrap(),
                &[BIN.to_vec()],
                Id(1),
                &policy
            )
            .err()
            .unwrap()
            .code,
            "unsupported_gltf"
        );
    }
    let mut bin = BIN.to_vec();
    bin[576..578].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(import_scene(GLTF, &[bin], Id(1), &policy).is_err());
    let mut j: Value = serde_json::from_slice(GLTF).unwrap();
    j["nodes"][0]["translation"] = json!([1, 0, 0]);
    assert!(
        import_scene(
            &serde_json::to_vec(&j).unwrap(),
            &[BIN.to_vec()],
            Id(1),
            &policy
        )
        .is_err()
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn three_restricted_variants_render_and_commit_durably_with_lost_ack_retry() {
    let mut session = imported();
    let root = session.document().snapshot().revision().unwrap();
    let protected = protected_digest(session.document().snapshot()).unwrap();
    let mut digests = std::collections::BTreeSet::new();
    let mut selected = String::new();
    for (name, color) in [
        ("red", [0.8, 0.1, 0.1]),
        ("green", [0.1, 0.8, 0.1]),
        ("blue", [0.1, 0.1, 0.8]),
    ] {
        let base = branch(&mut session, name);
        let (op, revision) = modify(&mut session, name, base, color);
        assert_eq!(call(&mut session, op).unwrap()["revision"], revision);
        let preview = call(
            &mut session,
            Operation::Preview {
                branch: name.into(),
                revision: revision.clone(),
            },
        )
        .unwrap();
        assert_eq!(preview["protected_digest"], protected);
        assert!(preview["inspection"]["visible_pixels"].as_u64().unwrap() > 10);
        let normals = preview["passes"]["normals_world"].as_array().unwrap();
        let depths = preview["passes"]["depth_meters"].as_array().unwrap();
        for (i, object) in preview["passes"]["object_ids"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            if !object.is_null() {
                assert!(depths[i].as_f64().unwrap() > 0.);
                let n = normals[i].as_array().unwrap();
                assert!(
                    (n.iter().map(|x| x.as_f64().unwrap().powi(2)).sum::<f64>() - 1.).abs() < 1e-6
                );
            }
        }
        digests.insert(
            preview["receipt"]["output_digest"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
        if name == "green" {
            selected = revision;
        }
    }
    assert_eq!(digests.len(), 3);
    assert_eq!(session.document().snapshot().revision().unwrap(), root);
    let req = session
        .selection_request(
            &fixtures::principal(),
            "green",
            &selected,
            "select:green:00001",
            1024 * 1024,
        )
        .unwrap();
    let mut doc = session.document().clone();
    let mut store = MemoryStore {
        fail_publication: true,
        ..Default::default()
    };
    assert!(durable_execute(&mut store, &mut doc, &fixtures::principal(), &req).is_err());
    assert_eq!(doc.snapshot().revision().unwrap(), root);
    store.fail_publication = false;
    let receipt = durable_execute(&mut store, &mut doc, &fixtures::principal(), &req).unwrap();
    let mut restored = recover(&store.load().unwrap()).unwrap().unwrap();
    assert_eq!(
        receipt,
        durable_execute(&mut store, &mut restored, &fixtures::principal(), &req).unwrap()
    );
    assert!(receipt.durable);
    assert_eq!(protected_digest(restored.snapshot()).unwrap(), protected);
    assert_eq!(
        restored
            .snapshot()
            .render_settings
            .as_ref()
            .unwrap()
            .environment,
        [0.25; 3]
    );
    assert_eq!(restored.snapshot().revision().unwrap(), selected);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn restricted_variants_reject_scope_budget_conflicts_cancellation_and_unrendered_selection() {
    let mut s = imported();
    let base = branch(&mut s, "v");
    assert!(
        s.selection_request(
            &fixtures::principal(),
            "v",
            &base,
            "select:invalid:0001",
            99999
        )
        .is_err()
    );
    let (op, revision) = modify(&mut s, "v", base.clone(), [0.1, 0.5, 0.7]);
    let mut changed = op.clone();
    if let Operation::Modify { edits, .. } = &mut changed {
        edits.materials[0].base_color = [1., 0., 0.];
    }
    assert_eq!(
        call(&mut s, changed).unwrap_err().code,
        "idempotency_mismatch"
    );
    let mut stale = op.clone();
    if let Operation::Modify {
        idempotency_key, ..
    } = &mut stale
    {
        *idempotency_key = "different:key:0001".into();
    }
    assert_eq!(call(&mut s, stale).unwrap_err().code, "stale_revision");
    assert_eq!(
        s.preview(&fixtures::principal(), "v", &revision, || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert!(
        s.selection_request(
            &fixtures::principal(),
            "v",
            &revision,
            "select:cancelled:01",
            99999
        )
        .is_err()
    );
    let mut checks = 0;
    assert!(
        s.preview(&fixtures::principal(), "v", &revision, || {
            checks += 1;
            checks > 2
        })
        .is_err()
    );
    assert!(s.render_input(&fixtures::principal(), "v", &base).is_err());
    let other = Principal {
        id: "other".into(),
        can_write: true,
    };
    assert_eq!(
        s.render_input(&other, "v", &revision).unwrap_err().code,
        "permission"
    );
    let reader = Principal {
        id: fixtures::principal().id,
        can_write: false,
    };
    assert_eq!(
        s.dispatch(
            &reader,
            agent::Request {
                version: 0,
                operation: op.clone()
            }
        )
        .unwrap_err()
        .code,
        "permission"
    );
    let mut unknown = serde_json::to_value(agent::Request {
        version: 0,
        operation: op.clone(),
    })
    .unwrap();
    unknown["operation"]["edits"]["geometry"] = json!({});
    assert!(
        s.dispatch_wire(
            &fixtures::principal(),
            &serde_json::to_vec(&unknown).unwrap()
        )
        .is_err()
    );
    for name in ["b", "c"] {
        branch(&mut s, name);
    }
    let current = s.document().snapshot().revision().unwrap();
    assert_eq!(
        call(
            &mut s,
            Operation::Branch {
                branch: "overflow".into(),
                base_revision: current
            }
        )
        .unwrap_err()
        .code,
        "budget"
    );
    s.preview(&fixtures::principal(), "v", &revision, || false)
        .unwrap();
    let entity = s.document().snapshot().entities.iter().next().unwrap().id;
    let root_edit = fixtures::request(
        s.document(),
        "root:conflict:0001",
        vec![Command::Rename {
            entity,
            name: "concurrent".into(),
        }],
    )
    .unwrap();
    s.execute_root(&fixtures::principal(), &root_edit).unwrap();
    assert_eq!(
        call(
            &mut s,
            Operation::Commit {
                branch: "v".into(),
                revision,
                idempotency_key: "select:conflict:001".into(),
                max_added_bytes: 1000000
            }
        )
        .unwrap_err()
        .code,
        "stale_revision"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn importer_unsigned_index_widths_multiple_primitives_and_invalid_transforms() {
    let policy = Policy {
        allow_lambertian: true,
    };
    let original = imported();
    let expected = original.document().snapshot().meshes.keys().next().unwrap();
    for component in [5121, 5125] {
        let mut j: Value = serde_json::from_slice(GLTF).unwrap();
        let mut bin = BIN[..576].to_vec();
        for i in BIN[576..].chunks_exact(2) {
            let n = u16::from_le_bytes(i.try_into().unwrap());
            if component == 5121 {
                bin.push(n as u8);
            } else {
                bin.extend_from_slice(&u32::from(n).to_le_bytes());
            }
        }
        j["accessors"][0]["componentType"] = json!(component);
        j["bufferViews"][0]["byteLength"] = json!(bin.len() - 576);
        j["buffers"][0]["byteLength"] = json!(bin.len());
        let mut p = j["meshes"][0]["primitives"][0].clone();
        p["material"] = json!(1);
        j["meshes"][0]["primitives"].as_array_mut().unwrap().push(p);
        let mut m = j["materials"][0].clone();
        m["pbrMetallicRoughness"]["baseColorFactor"] = json!([0, 1, 0, 1]);
        j["materials"].as_array_mut().unwrap().push(m);
        let imported =
            import_scene(&serde_json::to_vec(&j).unwrap(), &[bin], Id(10), &policy).unwrap();
        let mut doc = Document::new(Snapshot::empty(Id(10))).unwrap();
        let req = fixtures::request(&doc, "index:width:test:01", imported.commands).unwrap();
        doc.execute(&fixtures::principal(), &req).unwrap();
        assert_eq!(doc.snapshot().meshes.keys().next().unwrap(), expected);
        let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
        assert_eq!(scene.instances.len(), 2);
        assert_eq!(scene.geometry_builds, 1);
        assert_ne!(
            scene.instances[0].material.id,
            scene.instances[1].material.id
        );
    }
    let mut j: Value = serde_json::from_slice(GLTF).unwrap();
    j["nodes"][1]["rotation"] = json!([0, 0, 0, 2]);
    assert!(
        import_scene(
            &serde_json::to_vec(&j).unwrap(),
            &[BIN.to_vec()],
            Id(1),
            &policy
        )
        .is_err()
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn agent_admission_and_durable_dispatch_are_atomic_and_versioned() {
    let mut s = Session::new(Document::new(Snapshot::empty(Id(1))).unwrap());
    let initial = s.document().snapshot().revision().unwrap();
    let mut oversized = settings();
    oversized.width = 129;
    let mut op = Operation::ImportGlb {
        bytes: GLB.to_vec(),
        policy: Policy {
            allow_lambertian: true,
        },
        settings: oversized,
        base_revision: initial.clone(),
        idempotency_key: "budget:import:0001".into(),
    };
    assert_eq!(call(&mut s, op.clone()).unwrap_err().code, "budget");
    assert_eq!(s.document().snapshot().revision().unwrap(), initial);
    if let Operation::ImportGlb {
        settings: value, ..
    } = &mut op
    {
        *value = settings();
    }
    let mut store = MemoryStore {
        quota: 1,
        ..Default::default()
    };
    let request = agent::Request {
        version: 0,
        operation: op,
    };
    assert_eq!(
        s.dispatch_durable(&mut store, &fixtures::principal(), request.clone())
            .unwrap_err()
            .code,
        "quota"
    );
    assert_eq!(s.document().snapshot().revision().unwrap(), initial);
    store.quota = 0;
    let receipt = s
        .dispatch_durable(&mut store, &fixtures::principal(), request.clone())
        .unwrap();
    assert_eq!(receipt["receipt"]["durable"], true);
    assert_eq!(
        s.dispatch_durable(&mut store, &fixtures::principal(), request)
            .unwrap(),
        receipt
    );
    assert_eq!(store.records.len(), 1);
    assert_eq!(s.document().snapshot().version, 1);
    let mut incompatible = s.document().snapshot().clone();
    incompatible.version = 0;
    assert!(incompatible.validate().is_err());
    let base = branch(&mut s, "budget");
    let (mut op, rev) = modify(&mut s, "budget", base, [0.2, 0.4, 0.6]);
    if let Operation::Modify {
        idempotency_key,
        base_revision,
        edits,
        max_added_bytes,
        ..
    } = &mut op
    {
        *idempotency_key = "unknown:material:01".into();
        *base_revision = rev.clone();
        edits.materials[0].material = Id(u128::MAX);
        *max_added_bytes = 1;
    }
    assert_eq!(call(&mut s, op.clone()).unwrap_err().code, "permission");
    assert_eq!(
        s.render_input(&fixtures::principal(), "budget", &rev)
            .unwrap()
            .0
            .revision()
            .unwrap(),
        rev
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn durable_agent_retry_checks_target_and_retained_metadata() {
    let mut session = Session::new(Document::new(Snapshot::empty(Id(20))).unwrap());
    let base = session.document().snapshot().revision().unwrap();
    let request = agent::Request {
        version: 0,
        operation: Operation::ImportGlb {
            bytes: GLB.to_vec(),
            policy: Policy {
                allow_lambertian: true,
            },
            settings: settings(),
            base_revision: base,
            idempotency_key: "retry:import:0001".into(),
        },
    };
    let mut first = MemoryStore::default();
    let receipt = session
        .dispatch_durable(&mut first, &fixtures::principal(), request.clone())
        .unwrap();
    let mut second = MemoryStore::default();
    assert_eq!(
        session
            .dispatch_durable(&mut second, &fixtures::principal(), request.clone())
            .unwrap(),
        receipt
    );
    assert_eq!(second.records.len(), 1);
    let mut writer = recover(&first.load().unwrap()).unwrap().unwrap();
    let noop = fixtures::request(
        &writer,
        "external:noop:0001",
        vec![Command::SetRenderSettings {
            settings: settings(),
        }],
    )
    .unwrap();
    durable_execute(&mut first, &mut writer, &fixtures::principal(), &noop).unwrap();
    assert_eq!(
        writer.snapshot().revision().unwrap(),
        session.document().snapshot().revision().unwrap()
    );
    assert_eq!(
        session
            .dispatch_durable(&mut first, &fixtures::principal(), request)
            .unwrap_err()
            .code,
        "conflict"
    );
    assert_eq!(first.records.len(), 2);
    let mut wire = serde_json::to_value(settings()).unwrap();
    wire["unrecognized"] = json!(1);
    assert!(serde_json::from_value::<Settings>(wire).is_err());
}
