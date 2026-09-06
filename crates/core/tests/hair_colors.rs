use render_core::{document::*, hair_import::*, *};
const GRADIENT: &[u8] = include_bytes!("../../../fixtures/curve-colors/data/gradient-le.hair");
const BIG: &[u8] = include_bytes!("../../../fixtures/curve-colors/data/gradient-be.hair");
const RGB: &[u8] = include_bytes!("../../../fixtures/curve-colors/data/rgb-only.hair");
fn policy() -> Policy {
    Policy {
        byte_order: ByteOrder::LittleEndian,
        meters_per_unit: 1.,
        thickness: Thickness::Diameter,
        color_space: ColorSpace::LinearSrgb,
        transparency: Transparency::CoverageOneMinusTransparency,
        representation: Representation::SweptPolylineSurface,
        roughness: 0.6,
        tessellation: curves::Tessellation {
            radial_error: 0.01,
            ..Default::default()
        },
        point_attributes: Some(PointAttributes::LinearRgbaF32),
    }
}
fn load(bytes: &[u8], p: &Policy) -> Imported {
    import(bytes, Id(10400), p, || false).unwrap()
}
fn document(i: Imported) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(10400))).unwrap();
    let q = fixtures::request(&d, "curve:hair:rgba:001", i.commands).unwrap();
    d.execute(&fixtures::principal(), &q).unwrap();
    d
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn explicit_mode_preserves_controls_alpha_precision_and_opaque_branch() {
    let p = policy();
    let i = load(GRADIENT, &p);
    assert_eq!(i.report.profile, RGBA_PROFILE);
    let conversion = i.report.point_attribute_conversion.as_ref().unwrap();
    assert_eq!(conversion.control_count, 7);
    assert!(
        conversion.max_alpha_absolute_error > 0.
            && conversion.max_alpha_absolute_error <= f64::from(f32::EPSILON) / 4.
    );
    let mut strict = p.clone();
    strict.point_attributes = None;
    assert_eq!(
        import(GRADIENT, Id(10400), &strict, || false)
            .err()
            .unwrap()
            .code,
        "unsupported_hair"
    );
    let r = i.report.clone();
    let d = document(i);
    assert_eq!(d.snapshot().version, 15);
    let row = &r.strands[0];
    let asset = &d.snapshot().geometry_assets[&row.geometry_asset];
    let curves::Shape::Curves { curves } = &asset.shape else {
        panic!()
    };
    let curve = curves.iter().find(|c| c.id == row.curve).unwrap();
    assert_eq!(curve.controls[0].color, Some([1., 0., 0., 1.]));
    assert_eq!(
        curve.controls[1].color,
        Some([0., 1., 0., (1. - f64::from(0.1f32)) as f32])
    );
    assert_eq!(curve.controls[2].color, Some([0., 0., 1., 0.25]));
    assert!(
        d.snapshot()
            .materials
            .values()
            .all(|m| m.base_color == [1.; 3]
                && matches!(
                    m.pbr.as_ref().unwrap().advanced.as_ref().unwrap().opacity,
                    scattering::Opacity::Blend { factor: 1. }
                ))
    );
    let opaque = document(load(RGB, &p));
    assert!(
        opaque
            .snapshot()
            .materials
            .values()
            .all(|m| m.pbr.as_ref().unwrap().advanced.is_none())
    );
    let mut be = p.clone();
    be.byte_order = ByteOrder::BigEndian;
    let be = document(load(BIG, &be));
    let mut left = d
        .snapshot()
        .geometry_assets
        .values()
        .map(|a| canonical(&a.evaluate(|| false).unwrap().mesh).unwrap())
        .collect::<Vec<_>>();
    let mut right = be
        .snapshot()
        .geometry_assets
        .values()
        .map(|a| canonical(&a.evaluate(|| false).unwrap().mesh).unwrap())
        .collect::<Vec<_>>();
    left.sort();
    right.sort();
    assert_eq!(left, right);
    let recovered = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(canonical(&d).unwrap(), canonical(&recovered).unwrap());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn colored_import_agent_failures_and_source_unit_validation() {
    let p = policy();
    let d = Document::new(Snapshot::empty(Id(10400))).unwrap();
    let before = canonical(&d).unwrap();
    let mut session = agent::Session::new(d);
    let request = agent::Request {
        version: 0,
        operation: agent::Operation::ImportHair {
            bytes: GRADIENT.to_vec(),
            policy: p.clone(),
            settings: fixtures::settings(),
            base_revision: session.document().snapshot().revision().unwrap(),
            idempotency_key: "curve:hair:agent:001".into(),
        },
    };
    let mut calls = 0;
    assert_eq!(
        session
            .dispatch_cancellable(&fixtures::principal(), request.clone(), || {
                calls += 1;
                calls > 10
            })
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    let mut store = storage::MemoryStore {
        fail_publication: true,
        ..Default::default()
    };
    assert_eq!(
        session
            .dispatch_durable(&mut store, &fixtures::principal(), request.clone())
            .unwrap_err()
            .code,
        "storage"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    store.fail_publication = false;
    session
        .dispatch_durable(&mut store, &fixtures::principal(), request.clone())
        .unwrap();
    let saved = canonical(session.document()).unwrap();
    session
        .dispatch_durable(&mut store, &fixtures::principal(), request)
        .unwrap();
    assert_eq!(canonical(session.document()).unwrap(), saved);
    for at in [244, 272] {
        let mut bad = GRADIENT.to_vec();
        bad[at..at + 4].copy_from_slice(&1.1f32.to_le_bytes());
        assert_eq!(
            import(&bad, Id(10400), &p, || false).err().unwrap().code,
            "hair"
        );
    }
    let mut srgb = p;
    srgb.color_space = ColorSpace::Srgb;
    let i = load(GRADIENT, &srgb);
    assert_eq!(
        i.report.point_attribute_conversion.unwrap().control_count,
        7
    );
}
