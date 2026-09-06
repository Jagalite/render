use render_core::{
    curves::*,
    document::*,
    geometry::{AttributeValues, Domain},
    *,
};
fn control(id: u128, z: f64, radius: f64, color: Option<[f32; 4]>) -> Control {
    Control {
        id: Id(id),
        position: [0., 0., z],
        radius,
        tilt: 0.,
        color,
    }
}
fn curve() -> Curve {
    Curve {
        id: Id(1),
        basis: Basis::Polyline,
        controls: vec![
            control(10, 0., 0.1, Some([1., 0., 0., 0.25])),
            control(11, 2., 0.2, Some([0., 0., 1., 0.75])),
        ],
        closed: false,
    }
}
fn asset(curves: Vec<Curve>) -> Asset {
    Asset {
        shape: Shape::Curves { curves },
        tessellation: Tessellation {
            radial_error: 0.02,
            ..Default::default()
        },
    }
}
fn colors(mesh: &geometry::Mesh) -> &Vec<[f32; 4]> {
    let a = mesh.color_attribute().unwrap();
    assert_eq!(a.id, Id(200));
    assert_eq!(a.domain, Domain::Point);
    let AttributeValues::Vec4(v) = &a.values else {
        panic!()
    };
    v
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn polyline_midpoint_radius_subdivision_rings_caps_and_missing_white() {
    let c = curve();
    assert_eq!(c.sample(0.5).unwrap().color, [0.5, 0., 0.5, 0.5]);
    let a = asset(vec![c.clone()]);
    let samples = c.tessellate(&a.tessellation, &mut || false).unwrap();
    assert!(samples.len() > 2);
    for s in &samples {
        let t = s.position.z / 2.;
        assert!((f64::from(s.color[2]) - t).abs() < 1e-7);
        assert!((f64::from(s.color[3]) - (0.25 + 0.5 * t)).abs() < 1e-7);
    }
    let d = a.evaluate(|| false).unwrap();
    let rgba = colors(&d.mesh);
    assert_eq!(rgba.len(), d.mesh.positions.len());
    for (i, color) in rgba.iter().enumerate() {
        let t = d.mesh.positions.get(i).z / 2.;
        assert!((f64::from(color[2]) - t).abs() < 1e-7);
        assert!((f64::from(color[3]) - (0.25 + 0.5 * t)).abs() < 1e-7);
    }
    assert_eq!(rgba[rgba.len() - 2], [1., 0., 0., 0.25]);
    assert_eq!(rgba[rgba.len() - 1], [0., 0., 1., 0.75]);
    let mut white = c.clone();
    white.id = Id(2);
    for p in &mut white.controls {
        p.color = None;
        p.position[0] = 1.;
    }
    let mixed = asset(vec![c, white.clone()]).evaluate(|| false).unwrap();
    let range = mixed.receipt.curve_ranges[&Id(2)];
    assert!(
        colors(&mixed.mesh)[range[0] as usize..range[1] as usize]
            .iter()
            .all(|v| *v == [1.; 4])
    );
    let uncolored = asset(vec![white]);
    assert!(
        uncolored
            .evaluate(|| false)
            .unwrap()
            .mesh
            .color_attribute()
            .is_none()
    );
    assert!(
        !String::from_utf8(canonical(&uncolored).unwrap())
            .unwrap()
            .contains("color")
    );
    let closed = Curve {
        id: Id(3),
        basis: Basis::Polyline,
        controls: vec![
            Control {
                id: Id(1),
                position: [0., 0., 0.],
                radius: 0.05,
                tilt: 0.,
                color: Some([1., 0., 0., 1.]),
            },
            Control {
                id: Id(2),
                position: [1., 0., 0.],
                radius: 0.05,
                tilt: 0.,
                color: Some([0., 1., 0., 1.]),
            },
            Control {
                id: Id(3),
                position: [1., 1., 0.],
                radius: 0.05,
                tilt: 0.,
                color: None,
            },
            Control {
                id: Id(4),
                position: [0., 1., 0.],
                radius: 0.05,
                tilt: 0.,
                color: Some([0., 0., 1., 0.5]),
            },
        ],
        closed: true,
    };
    assert_eq!(
        closed.sample(0.).unwrap().color,
        closed.sample(1.).unwrap().color
    );
    let d = asset(vec![closed]).evaluate(|| false).unwrap();
    assert!(colors(&d.mesh).contains(&[0., 0., 1., 0.5]));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn color_validation_schema_transactions_and_recovery() {
    for bad in [f32::NAN, -0.1, 1.1] {
        let mut c = curve();
        c.controls[0].color = Some([bad, 0., 0., 1.]);
        assert_eq!(c.validate().unwrap_err().code, "curve");
    }
    let mut c = curve();
    c.basis = Basis::CubicBezier;
    assert_eq!(c.validate().unwrap_err().code, "unsupported_curve_color");
    let a = asset(vec![curve()]);
    assert_eq!(a.evaluate(|| true).unwrap_err().code, "cancelled");
    let mut limited = a.clone();
    limited.tessellation.max_vertices = 6;
    assert_eq!(limited.evaluate(|| false).unwrap_err().code, "budget");
    let mut d = Document::new(Snapshot::empty(Id(10300))).unwrap();
    let before = canonical(&d).unwrap();
    let q = fixtures::request(
        &d,
        "curve:rgba:author:001",
        vec![Command::PutGeometry { asset: a }],
    )
    .unwrap();
    let mut reader = fixtures::principal();
    reader.can_write = false;
    assert_eq!(d.execute(&reader, &q).unwrap_err().code, "permission");
    assert_eq!(canonical(&d).unwrap(), before);
    d.execute(&fixtures::principal(), &q).unwrap();
    assert_eq!(d.snapshot().version, 15);
    let frozen = canonical(&d).unwrap();
    d.execute(&fixtures::principal(), &q).unwrap();
    assert_eq!(canonical(&d).unwrap(), frozen);
    let mut stale = q;
    stale.idempotency_key = "curve:rgba:stale:001".into();
    assert_eq!(
        d.execute(&fixtures::principal(), &stale).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(canonical(&d).unwrap(), frozen);
    let mut old = d.snapshot().clone();
    old.version = 14;
    assert_eq!(old.validate().unwrap_err().code, "schema_version");
    let restored = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(canonical(&restored).unwrap(), frozen);
    let mut invalid = asset(vec![curve()]);
    if let Shape::Curves { curves } = &mut invalid.shape {
        curves[0].controls[1].color = Some([0., 0., 0., 2.]);
    }
    let q = fixtures::request(
        &d,
        "curve:rgba:invalid:001",
        vec![Command::PutGeometry { asset: invalid }],
    )
    .unwrap();
    assert_eq!(
        d.execute(&fixtures::principal(), &q).unwrap_err().code,
        "curve"
    );
    assert_eq!(canonical(&d).unwrap(), frozen);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn groom_children_preserve_guide_colors_and_version_boundary() {
    let mut d = feature_fixtures::groom_document().unwrap();
    let mut g = d.snapshot().grooms[&Id(7601)].clone();
    g.children.truncate(2);
    g.guides[0].curve.basis = Basis::Polyline;
    for p in &mut g.guides[0].curve.controls {
        p.color = Some([0.2, 0.4, 0.8, 0.5]);
    }
    let q = fixtures::request(
        &d,
        "groom:rgba:author:001",
        vec![Command::SetGroom {
            entity: Id(7601),
            groom: Some(g.clone()),
        }],
    )
    .unwrap();
    d.execute(&fixtures::principal(), &q).unwrap();
    assert_eq!(d.snapshot().version, 15);
    let out = g.evaluate(d.snapshot(), Id(7601), || false).unwrap();
    assert_eq!(out.receipt.curve_ranges.len(), 3);
    assert!(colors(&out.mesh).iter().all(|v| *v == [0.2, 0.4, 0.8, 0.5]));
    let mut old = d.snapshot().clone();
    old.version = 14;
    assert_eq!(old.validate().unwrap_err().code, "schema_version");
    let restored = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    let again = restored.snapshot().grooms[&Id(7601)]
        .evaluate(restored.snapshot(), Id(7601), || false)
        .unwrap();
    assert_eq!(
        canonical(&out.mesh).unwrap(),
        canonical(&again.mesh).unwrap()
    );
    let mut changed = g.clone();
    changed.guides[0].curve.controls[0].color = Some([1., 0., 0., 1.]);
    let next = changed.evaluate(d.snapshot(), Id(7601), || false).unwrap();
    assert_ne!(out.receipt.source_digest, next.receipt.source_digest);
    assert_eq!(
        canonical(&out.mesh.positions).unwrap(),
        canonical(&next.mesh.positions).unwrap()
    );
    assert_ne!(colors(&out.mesh), colors(&next.mesh));
    assert_eq!(
        changed
            .evaluate(d.snapshot(), Id(7601), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
}
