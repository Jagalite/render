use render_core::{blend::*, document::*, render::*, *};
const FILES: [&[u8]; 4] = [
    include_bytes!("../../../fixtures/blend-static/data/quad-32-little.blend"),
    include_bytes!("../../../fixtures/blend-static/data/quad-32-big.blend"),
    include_bytes!("../../../fixtures/blend-static/data/quad-64-little.blend"),
    include_bytes!("../../../fixtures/blend-static/data/quad-64-big.blend"),
];
const LAYOUTS: [&str; 4] = [
    include_str!("../../../fixtures/blend-static/data/quad-32-little.blend.layout.json"),
    include_str!("../../../fixtures/blend-static/data/quad-32-big.blend.layout.json"),
    include_str!("../../../fixtures/blend-static/data/quad-64-little.blend.layout.json"),
    include_str!("../../../fixtures/blend-static/data/quad-64-big.blend.layout.json"),
];
fn policy() -> Policy {
    Policy {
        scene: "Scene".into(),
        meters_per_unit: 1.,
        allow_principled_approximation: true,
        allow_point_light_approximation: true,
    }
}
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 16;
    s.height = 16;
    s.samples = 4;
    s
}
fn load(bytes: &[u8], p: &Policy) -> Imported {
    import(bytes, Id(293), p, settings(), || false).unwrap()
}
fn document(i: Imported) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(293))).unwrap();
    let r = fixtures::request(&d, "blend-static-test-001", i.commands).unwrap();
    d.execute(&fixtures::principal(), &r).unwrap();
    d
}
fn location(index: usize, label: &str, kind: &str, field: &str) -> usize {
    let j: serde_json::Value = serde_json::from_str(LAYOUTS[index]).unwrap();
    let token = j["addresses"][label].as_u64().unwrap();
    j["data_offsets"][token.to_string()].as_u64().unwrap() as usize
        + j["fields"][kind][field].as_u64().unwrap() as usize
}
fn mutate(index: usize, label: &str, kind: &str, field: &str, value: u64, width: usize) -> Vec<u8> {
    let mut bytes = FILES[index].to_vec();
    let at = location(index, label, kind, field);
    let packed = if index.is_multiple_of(2) {
        value.to_le_bytes().to_vec()
    } else {
        value.to_be_bytes()[8 - width..].to_vec()
    };
    bytes[at..at + width].copy_from_slice(&packed[..width]);
    bytes
}
fn token(index: usize, label: &str) -> u64 {
    serde_json::from_str::<serde_json::Value>(LAYOUTS[index]).unwrap()["addresses"][label]
        .as_u64()
        .unwrap()
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn analytic_scene_hierarchy_uv_camera_light_and_source_roundtrip() {
    let mut reference = None;
    for (index, bytes) in FILES.iter().enumerate() {
        let i = load(bytes, &policy());
        let report = i.report.clone();
        let s = i.settings.clone();
        let d = document(i);
        assert_eq!(d.snapshot().version, 16);
        assert_eq!(report.entities.len(), 4);
        assert_eq!(report.materials.len(), 1);
        assert_eq!(
            d.snapshot().source_assets[&report.source_asset].bytes(),
            *bytes
        );
        assert_eq!(report.source_digest, digest(bytes));
        assert_eq!(report.pointer_bits, if index < 2 { 32 } else { 64 });
        let e = d.snapshot().entities.get(report.entities["Quad"]).unwrap();
        assert_eq!(e.parent, Some(report.entities["Parent"]));
        assert_eq!(
            d.snapshot().world_transform(e.id).unwrap(),
            glam::DAffine3::IDENTITY
        );
        let m = &d.snapshot().meshes[e.mesh.as_ref().unwrap()];
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.face_offsets, [0, 4]);
        assert_eq!(m.corners.len(), 4);
        assert_eq!(m.triangles().unwrap().len(), 2);
        assert_eq!(m.uv(2).to_array(), [1., 1.]);
        assert_eq!(m.positions.get(0).to_array(), [-1., -1., 0.]);
        assert_eq!(s.camera.position, [0., 0., 4.]);
        assert_eq!(s.camera.up, [0., 1., 0.]);
        assert!((s.camera.fov() - 2. * 0.36f64.atan()).abs() < 1e-14);
        assert_eq!(s.light.position, [0., 0., 3.]);
        for c in 0..3 {
            assert!(
                (f64::from(s.light.intensity[c])
                    - 32. * [1., 0.5, 0.25][c] / (4. * std::f64::consts::PI))
                    .abs()
                    < 2e-7
            );
        }
        assert_eq!(s.environment, [0.0625, 0.125, 0.25]);
        let meshbytes = canonical(m).unwrap();
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let image = render(&scene, &s, || false).unwrap();
        assert!(image.objects.iter().any(Option::is_some));
        assert!(image.linear.iter().flatten().all(|x| x.is_finite()));
        if let Some((mesh, linear)) = &reference {
            assert_eq!(&meshbytes, mesh);
            assert_eq!(&image.linear, linear);
        } else {
            reference = Some((meshbytes, image.linear));
        }
        let archive = canonical(&d).unwrap();
        let restored: Document = serde_json::from_slice(&archive).unwrap();
        restored.snapshot().validate().unwrap();
        assert_eq!(canonical(&restored).unwrap(), archive);
        assert_eq!(
            restored.snapshot().source_assets[&report.source_asset].bytes(),
            *bytes
        );
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn typed_source_transactions_permissions_versions_and_durability() {
    let p = fixtures::principal();
    let mut d = Document::new(Snapshot::empty(Id(293))).unwrap();
    let before = canonical(&d).unwrap();
    let root = d.snapshot().revision().unwrap();
    assert!(
        !String::from_utf8(canonical(d.snapshot()).unwrap())
            .unwrap()
            .contains("source_assets")
    );
    let i = load(FILES[2], &policy());
    let report = i.report.clone();
    let q = fixtures::request(&d, "blend-source-transaction", i.commands).unwrap();
    let denied = Principal {
        id: "reader".into(),
        can_write: false,
    };
    assert_eq!(d.execute(&denied, &q).unwrap_err().code, "permission");
    let mut low = q.clone();
    low.max_added_bytes = 1;
    assert_eq!(d.execute(&p, &low).unwrap_err().code, "budget");
    let mut stale = q.clone();
    stale.base_revision = "sha256:wrong".into();
    assert_eq!(d.execute(&p, &stale).unwrap_err().code, "stale_revision");
    assert_eq!(canonical(&d).unwrap(), before);
    for (quota, fail, code) in [(1, false, "quota"), (0, true, "storage")] {
        let mut store = storage::MemoryStore {
            quota,
            fail_publication: fail,
            ..Default::default()
        };
        assert_eq!(
            storage::durable_execute(&mut store, &mut d, &p, &q)
                .unwrap_err()
                .code,
            code
        );
        assert!(store.records.is_empty());
        assert_eq!(canonical(&d).unwrap(), before);
    }
    let mut store = storage::MemoryStore::default();
    let receipt = storage::durable_execute(&mut store, &mut d, &p, &q).unwrap();
    assert_eq!(receipt.base_revision, root);
    assert!(receipt.durable);
    assert_eq!(
        storage::durable_execute(&mut store, &mut d, &p, &q).unwrap(),
        receipt
    );
    assert_eq!(store.records.len(), 1);
    let recovered = storage::recover(&store.records).unwrap().unwrap();
    assert_eq!(canonical(&recovered).unwrap(), canonical(&d).unwrap());
    assert_eq!(
        recovered.snapshot().source_assets[&report.source_asset].bytes(),
        FILES[2]
    );
    let mut old = d.snapshot().clone();
    old.version = 15;
    assert_eq!(old.validate().unwrap_err().code, "schema_version");
    let mut future = d.snapshot().clone();
    future.version = 19;
    assert_eq!(future.validate().unwrap_err().code, "schema_version");
    let mut corrupt = d.snapshot().clone();
    let asset = corrupt.source_assets.remove(&report.source_asset).unwrap();
    corrupt.source_assets.insert("sha256:wrong".into(), asset);
    assert_eq!(corrupt.validate().unwrap_err().code, "integrity");
    let bad = source::Asset::Blend {
        version: 292,
        bytes: FILES[2].to_vec(),
    };
    assert_eq!(bad.validate().unwrap_err().code, "integrity");
    let committed = canonical(&d).unwrap();
    let mut rename = q.clone();
    rename.idempotency_key = "blend-rename-independent".into();
    assert_eq!(d.execute(&p, &rename).unwrap_err().code, "stale_revision");
    assert_eq!(canonical(&d).unwrap(), committed);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn hostile_links_dependencies_numeric_and_cancellation_fail_before_commands() {
    for (index, bytes) in FILES.iter().enumerate() {
        let width = if index < 2 { 4 } else { 8 };
        for (label, kind, field, value, n, code) in [
            (
                "material",
                "Material",
                "blend_flag",
                1,
                1,
                "unsupported_blend",
            ),
            ("scene", "Scene", "r", (-1f32).to_bits() as u64, 4, "blend"),
            ("lamp", "Lamp", "r", (-1f32).to_bits() as u64, 4, "blend"),
            ("mesh-object", "Object", "mode", 1, 4, "unsupported_blend"),
            ("view", "ViewLayer", "flag", 0, 2, "unsupported_blend"),
            ("camera", "Camera", "dof", 1, 2, "unsupported_blend"),
            (
                "mesh-object",
                "Object",
                "parent",
                token(index, "mesh-object"),
                width,
                "cycle",
            ),
            ("mesh-object", "Object", "data", 0x123456, width, "blend"),
            (
                "mesh-object",
                "Object",
                "adt",
                1,
                width,
                "unsupported_blend",
            ),
            (
                "mesh-object",
                "Object",
                "modifiers",
                1,
                width,
                "unsupported_blend",
            ),
            (
                "member-mesh",
                "CollectionObject",
                "next",
                token(index, "member-parent"),
                width,
                "blend",
            ),
            ("mesh", "Mesh", "totvert", 0xffff_ffff, 4, "budget"),
            ("polygons", "MPoly", "loopstart", 1, 4, "blend"),
            ("polygons", "MPoly", "flag", 1, 1, "unsupported_blend"),
            (
                "camera",
                "Camera",
                "shiftx",
                0x3f80_0000,
                4,
                "unsupported_blend",
            ),
            (
                "inactive-field",
                "PartDeflect",
                "forcefield",
                1,
                2,
                "unsupported_blend",
            ),
            ("vertices", "MVert", "co", 0x7fc0_0000, 4, "blend"),
            (
                "material-link",
                "bNodeLink",
                "fromnode",
                0x123456,
                width,
                "blend",
            ),
            (
                "layer",
                "LayerCollection",
                "flag",
                16,
                2,
                "unsupported_blend",
            ),
        ] {
            let b = mutate(index, label, kind, field, value, n);
            let error = import(&b, Id(293), &policy(), settings(), || false)
                .err()
                .unwrap();
            assert_eq!(error.code, code, "{label}.{field}: {error}");
        }
        let mut wrong = bytes.to_vec();
        wrong[9..12].copy_from_slice(b"292");
        assert_eq!(
            import(&wrong, Id(293), &policy(), settings(), || false)
                .err()
                .unwrap()
                .code,
            "unsupported_blend"
        );
        let mut total = 0;
        import(bytes, Id(293), &policy(), settings(), || {
            total += 1;
            false
        })
        .unwrap();
        assert!(total > 10);
        let threshold = total - 3;
        let mut calls = 0;
        let error = import(bytes, Id(293), &policy(), settings(), || {
            calls += 1;
            calls > threshold
        })
        .err()
        .unwrap();
        assert_eq!(error.code, "cancelled");
        assert_eq!(calls, threshold + 1);
        assert_eq!(
            import(bytes, Id(293), &policy(), settings(), || true)
                .err()
                .unwrap()
                .code,
            "cancelled"
        );
    }
    for material in [true, false] {
        let mut p = policy();
        if material {
            p.allow_principled_approximation = false;
        } else {
            p.allow_point_light_approximation = false;
        }
        assert_eq!(
            import(FILES[2], Id(293), &p, settings(), || false)
                .err()
                .unwrap()
                .code,
            "unsupported_blend"
        );
    }
    let mut p = policy();
    p.scene = "Missing".into();
    assert_eq!(
        import(FILES[2], Id(293), &p, settings(), || false)
            .err()
            .unwrap()
            .code,
        "blend"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn public_agent_import_export_cancellation_and_invalid_wire_are_atomic() {
    let p = fixtures::principal();
    let mut session = agent::Session::new(Document::new(Snapshot::empty(Id(293))).unwrap());
    let before = canonical(session.document()).unwrap();
    let operation = agent::Operation::ImportBlend {
        bytes: FILES[2].to_vec(),
        policy: policy(),
        settings: settings(),
        base_revision: session.document().snapshot().revision().unwrap(),
        idempotency_key: "agent-blend-import-001".into(),
    };
    let request = agent::Request {
        version: 0,
        operation,
    };
    let denied = Principal {
        id: "reader".into(),
        can_write: false,
    };
    assert_eq!(
        session.dispatch(&denied, request.clone()).unwrap_err().code,
        "permission"
    );
    assert_eq!(
        session
            .dispatch_cancellable(&p, request.clone(), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    let imported = session.dispatch(&p, request.clone()).unwrap();
    let root = canonical(session.document()).unwrap();
    let revision = session.document().snapshot().revision().unwrap();
    let asset = imported["report"]["source_asset"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        session.dispatch(&p, request.clone()).unwrap()["receipt"],
        imported["receipt"]
    );
    let q = source::ExportRequest {
        revision: revision.clone(),
        asset: asset.clone(),
    };
    let out = source::export(session.document().snapshot(), &q, || false).unwrap();
    assert_eq!(out.bytes, FILES[2]);
    assert_eq!(out.source_digest, digest(FILES[2]));
    let op = agent::Request {
        version: 0,
        operation: agent::Operation::ExportSource { request: q.clone() },
    };
    let wire = canonical(&op).unwrap();
    let result: source::Exported =
        serde_json::from_slice(&session.dispatch_wire(&p, &wire).unwrap()).unwrap();
    assert_eq!(result, out);
    let mut calls = 0;
    assert_eq!(
        source::export(session.document().snapshot(), &q, || {
            calls += 1;
            calls == 2
        })
        .unwrap_err()
        .code,
        "cancelled"
    );
    let mut stale = q.clone();
    stale.revision = "sha256:stale".into();
    assert_eq!(
        source::export(session.document().snapshot(), &stale, || false)
            .unwrap_err()
            .code,
        "stale_revision"
    );
    let mut missing = q;
    missing.asset = "sha256:missing".into();
    assert_eq!(
        source::export(session.document().snapshot(), &missing, || false)
            .unwrap_err()
            .code,
        "reference"
    );
    let mut wire = serde_json::to_value(&op).unwrap();
    wire["operation"]["request"]["execute_script"] = serde_json::Value::Bool(true);
    assert_eq!(
        session
            .dispatch_wire(&p, &canonical(&wire).unwrap())
            .unwrap_err()
            .code,
        "encoding"
    );
    assert_eq!(canonical(session.document()).unwrap(), root);
    let mut modified = request;
    let agent::Operation::ImportBlend {
        idempotency_key, ..
    } = &mut modified.operation
    else {
        panic!()
    };
    *idempotency_key = "agent-blend-import-stale".into();
    assert_eq!(
        session.dispatch(&p, modified).unwrap_err().code,
        "import_target"
    );
    assert_eq!(canonical(session.document()).unwrap(), root);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn metric_units_authored_rotations_reflections_and_orthographic_lens() {
    let mut p = policy();
    p.meters_per_unit = 2.;
    let i = load(FILES[2], &p);
    let ids = i.report.entities.clone();
    let s = i.settings.clone();
    let d = document(i);
    assert_eq!(
        d.snapshot()
            .world_transform(ids["Parent"])
            .unwrap()
            .translation
            .to_array(),
        [2., 0., 0.]
    );
    assert_eq!(s.camera.position, [0., 0., 8.]);
    assert_eq!(s.light.position, [0., 0., 6.]);
    let m = d.snapshot().meshes.values().next().unwrap();
    assert_eq!(m.positions.get(0).to_array(), [-2., -2., 0.]);
    for (field, value, expected) in [
        (
            "rot",
            (std::f32::consts::FRAC_PI_2).to_bits() as u64,
            [0., 0., 1.],
        ),
        ("size", (-1f32).to_bits() as u64, [0., 1., 0.]),
    ] {
        let bytes = mutate(2, "mesh-object", "Object", field, value, 4);
        let i = load(&bytes, &policy());
        let id = i.report.entities["Quad"];
        let d = document(i);
        let t = d.snapshot().entities.get(id).unwrap();
        assert!(!t.transform.operations.is_empty());
        let world = d.snapshot().world_transform(id).unwrap();
        let actual = world.transform_vector3(glam::DVec3::Y);
        assert!((actual - glam::DVec3::from_array(expected)).length() < 5e-8);
        if field == "size" {
            assert!(world.matrix3.determinant() < 0.);
        }
    }
    let bytes = mutate(2, "camera", "Camera", "type", 1, 1);
    let i = load(&bytes, &policy());
    let cameras::Lens::Orthographic {
        xmag,
        ymag,
        near,
        far,
    } = i.settings.camera.lens.unwrap()
    else {
        panic!()
    };
    assert_eq!((xmag, ymag, far), (2., 2., 100.));
    assert!((near - 0.1).abs() < 2e-9);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn source_asset_count_and_byte_limits_are_transactional() {
    let mut d = Document::new(Snapshot::empty(Id(293))).unwrap();
    let p = fixtures::principal();
    let commands = FILES
        .iter()
        .map(|b| Command::PutSource {
            asset: source::Asset::Blend {
                version: 293,
                bytes: b.to_vec(),
            },
        })
        .collect();
    let q = fixtures::request(&d, "source-four-containers", commands).unwrap();
    d.execute(&p, &q).unwrap();
    assert_eq!(d.snapshot().source_assets.len(), 4);
    let before = canonical(&d).unwrap();
    let mut bytes = FILES[2].to_vec();
    bytes[9..12].copy_from_slice(b"292");
    let q = fixtures::request(
        &d,
        "source-fifth-container",
        vec![Command::PutSource {
            asset: source::Asset::Blend {
                version: 292,
                bytes,
            },
        }],
    )
    .unwrap();
    assert_eq!(d.execute(&p, &q).unwrap_err().code, "budget");
    assert_eq!(canonical(&d).unwrap(), before);
    let asset = source::Asset::Blend {
        version: 293,
        bytes: vec![0; MAX_INPUT_BYTES + 1],
    };
    assert_eq!(asset.validate().unwrap_err().code, "budget");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn distinct_material_blocks_cannot_alias_one_native_identity() {
    // A second mesh instance overrides its slot with a distinct Material block
    // bearing the same source name. It must not overwrite the first native material.
    let layout: serde_json::Value = serde_json::from_str(LAYOUTS[2]).unwrap();
    let mut bytes = FILES[2].to_vec();
    let material_token = 0xeeee_1000u64;
    let array_token = 0xeeee_2000u64;
    let original = token(2, "material");
    let data = layout["data_offsets"][original.to_string()]
        .as_u64()
        .unwrap() as usize;
    let size = layout["sizes"]["Material"].as_u64().unwrap() as usize;
    let mut block = bytes[data - 24..data + size].to_vec();
    block[8..16].copy_from_slice(&material_token.to_le_bytes());
    let end = bytes.len() - 24;
    bytes.splice(end..end, block);
    let mut array = Vec::new();
    array.extend(b"DATA");
    array.extend(8u32.to_le_bytes());
    array.extend(array_token.to_le_bytes());
    array.extend(0u32.to_le_bytes());
    array.extend(1u32.to_le_bytes());
    array.extend(material_token.to_le_bytes());
    let end = bytes.len() - 24;
    bytes.splice(end..end, array);
    for (field, value, width) in [
        ("type", 1, 2),
        ("totcol", 1, 4),
        ("data", token(2, "mesh"), 8),
        ("matbits", token(2, "matbits"), 8),
        ("mat", array_token, 8),
    ] {
        let at = location(2, "parent-object", "Object", field);
        bytes[at..at + width].copy_from_slice(&value.to_le_bytes()[..width]);
    }
    let at = layout["data_offsets"][token(2, "matbits").to_string()]
        .as_u64()
        .unwrap() as usize;
    bytes[at] = 1;
    let error = import(&bytes, Id(293), &policy(), settings(), || false)
        .err()
        .unwrap();
    assert_eq!(error.code, "blend");
    assert!(error.message.contains("duplicate material datablock name"));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn invalid_source_parameters_do_not_hide_behind_positive_ratios_or_zero_power() {
    let layout: serde_json::Value = serde_json::from_str(LAYOUTS[2]).unwrap();
    let mut bytes = FILES[2].to_vec();
    let at = location(2, "scene", "Scene", "r");
    for field in ["xasp", "yasp"] {
        let offset = layout["fields"]["RenderData"][field].as_u64().unwrap() as usize;
        bytes[at + offset..at + offset + 4].copy_from_slice(&(-1f32).to_le_bytes());
    }
    let e = import(&bytes, Id(293), &policy(), settings(), || false)
        .err()
        .unwrap();
    assert_eq!(e.code, "blend");
    assert!(e.message.contains("must be positive"));
    let mut bytes = mutate(2, "lamp", "Lamp", "r", (-1f32).to_bits() as u64, 4);
    let at = location(2, "lamp", "Lamp", "energy");
    bytes[at..at + 4].copy_from_slice(&0f32.to_le_bytes());
    let e = import(&bytes, Id(293), &policy(), settings(), || false)
        .err()
        .unwrap();
    assert_eq!(e.code, "blend");
    assert!(e.message.contains("negative source light color"));
    let mut bytes = mutate(
        2,
        "world-input-0-value",
        "bNodeSocketValueRGBA",
        "value",
        (-1f32).to_bits() as u64,
        4,
    );
    let at = location(2, "world-input-1-value", "bNodeSocketValueFloat", "value");
    bytes[at..at + 4].copy_from_slice(&0f32.to_le_bytes());
    let e = import(&bytes, Id(293), &policy(), settings(), || false)
        .err()
        .unwrap();
    assert_eq!(e.code, "blend");
    assert!(e.message.contains("negative source environment color"));
    let mut bytes = mutate(
        2,
        "material-input-3-value",
        "bNodeSocketValueRGBA",
        "value",
        (-1f32).to_bits() as u64,
        4,
    );
    let at = location(
        2,
        "material-input-18-value",
        "bNodeSocketValueFloat",
        "value",
    );
    bytes[at..at + 4].copy_from_slice(&0f32.to_le_bytes());
    let e = import(&bytes, Id(293), &policy(), settings(), || false)
        .err()
        .unwrap();
    assert_eq!(e.code, "blend");
    assert!(
        e.message
            .contains("negative source material emission color")
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn shared_mesh_instances_do_not_expand_asset_commands_or_geometry_builds() {
    let mut bytes = FILES[2].to_vec();
    for (field, value, width) in [
        ("type", 1, 2),
        ("totcol", 1, 4),
        ("data", token(2, "mesh"), 8),
        ("matbits", token(2, "matbits"), 8),
        ("mat", token(2, "materials"), 8),
    ] {
        let at = location(2, "parent-object", "Object", field);
        bytes[at..at + width].copy_from_slice(&value.to_le_bytes()[..width]);
    }
    let i = load(&bytes, &policy());
    assert_eq!(
        i.commands
            .iter()
            .filter(|c| matches!(c, Command::PutMesh { .. }))
            .count(),
        1
    );
    assert_eq!(
        i.commands
            .iter()
            .filter(|c| matches!(c, Command::PutMaterial { .. }))
            .count(),
        1
    );
    let d = document(i);
    assert_eq!(d.snapshot().meshes.len(), 1);
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    assert_eq!(scene.instances.len(), 2);
    assert_eq!(scene.geometry_builds, 1);
    assert_ne!(scene.instances[0].transform, scene.instances[1].transform);
}
