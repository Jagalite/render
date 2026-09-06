use glam::DVec3;
use render_core::{curves::*, document::*, render::*, storage::*, *};
fn control(i: u128, p: [f64; 3]) -> Control {
    Control {
        id: Id(i),
        position: p,
        radius: 0.03,
        tilt: 0.,
        color: None,
    }
}
fn bezier() -> Curve {
    Curve {
        id: Id(1),
        basis: Basis::CubicBezier,
        controls: vec![
            control(1, [-0.7, 0., 0.]),
            control(2, [-0.7, 1., 0.]),
            control(3, [0.7, 1., 0.]),
            control(4, [0.7, 0., 0.]),
        ],
        closed: false,
    }
}
fn asset() -> Asset {
    Asset {
        shape: Shape::Curves {
            curves: vec![bezier()],
        },
        tessellation: Tessellation::default(),
    }
}
fn doc() -> Document {
    let mut doc = Document::new(Snapshot::empty(Id(1100))).unwrap();
    let asset = asset();
    let key = asset.content_id().unwrap();
    let request = fixtures::request(
        &doc,
        "curves:author:001",
        vec![
            Command::PutGeometry { asset },
            Command::PutMaterial {
                material: Box::new(Material::diffuse(Id(2), [0.4, 0.8, 0.2])),
            },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(3),
                    name: "authored Bezier".into(),
                    parent: None,
                    mesh: None,
                    material: Some(Id(2)),
                    transform: Transform::default(),
                },
            },
            Command::SetGeometry {
                entity: Id(3),
                asset: Some(key),
            },
        ],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &request).unwrap();
    doc
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn spline_basis_and_rational_knot_insertion_match_analytic_geometry() {
    let curve = bezier();
    assert_eq!(curve.sample(0.).unwrap().position, DVec3::new(-0.7, 0., 0.));
    assert_eq!(curve.sample(1.).unwrap().position, DVec3::new(0.7, 0., 0.));
    assert!((curve.sample(0.5).unwrap().position - DVec3::new(0., 0.75, 0.)).length() < 1e-12);
    let circle = Curve {
        id: Id(10),
        basis: Basis::RationalBSpline {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        },
        controls: vec![
            control(1, [1., 0., 0.]),
            control(2, [1., 1., 0.]),
            control(3, [0., 1., 0.]),
        ],
        closed: false,
    };
    for i in 0..101 {
        let p = circle.sample(f64::from(i) / 100.).unwrap().position;
        assert!((p.length() - 1.).abs() < 1e-12);
    }
    let samples = circle
        .tessellate(&Tessellation::default(), &mut || false)
        .unwrap();
    assert!(samples.len() > 8);
    for s in samples {
        assert!((s.position.length() - 1.).abs() < 1e-12);
    }
    // A quadratic B-spline with unsaturated interior knots exercises actual insertion.
    let spline = Curve {
        id: Id(11),
        basis: Basis::RationalBSpline {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
            weights: vec![1., 2., 3., 1.],
        },
        controls: vec![
            control(1, [0., 0., 0.]),
            control(2, [1., 2., 0.]),
            control(3, [2., -1., 0.]),
            control(4, [3., 0., 0.]),
        ],
        closed: false,
    };
    let samples = spline
        .tessellate(&Tessellation::default(), &mut || false)
        .unwrap();
    for i in 0..1001 {
        let p = spline.sample(f64::from(i) / 1000.).unwrap().position;
        let distance = samples
            .windows(2)
            .map(|w| {
                let d = w[1].position - w[0].position;
                let t = ((p - w[0].position).dot(d) / d.length_squared()).clamp(0., 1.);
                p.distance(w[0].position + t * d)
            })
            .fold(f64::INFINITY, f64::min);
        assert!(distance <= 0.001001, "{distance}");
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn generated_tubes_and_point_spheres_are_closed_and_outward() {
    let straight = Curve {
        id: Id(5),
        basis: Basis::Polyline,
        controls: vec![control(1, [0., 0., 0.]), control(2, [0., 0., 1.])],
        closed: false,
    };
    let tube = Asset {
        shape: Shape::Curves {
            curves: vec![straight],
        },
        tessellation: Tessellation {
            radial_error: 0.0001,
            ..Default::default()
        },
    }
    .evaluate(|| false)
    .unwrap();
    let sphere = Asset {
        shape: Shape::Points {
            points: vec![Point {
                id: Id(4),
                position: [0.; 3],
                radius: 1.,
            }],
        },
        tessellation: Tessellation {
            radial_error: 0.002,
            ..Default::default()
        },
    }
    .evaluate(|| false)
    .unwrap();
    for (evaluated, expected) in [
        (&tube, std::f64::consts::PI * 0.03 * 0.03),
        (&sphere, 4. * std::f64::consts::PI / 3.),
    ] {
        let mesh = &evaluated.mesh;
        let mut edges = std::collections::BTreeMap::new();
        let mut volume = 0.;
        for tri in mesh.triangles().unwrap() {
            let p = tri.map(|c| mesh.positions.get(mesh.corners[c as usize].vertex as usize));
            volume += p[0].dot(p[1].cross(p[2])) / 6.;
            for j in 0..3 {
                let a = mesh.corners[tri[j] as usize].vertex;
                let b = mesh.corners[tri[(j + 1) % 3] as usize].vertex;
                let count = edges.entry((a.min(b), a.max(b))).or_insert((0, 0));
                count.0 += 1;
                count.1 += if a < b { 1 } else { -1 };
            }
        }
        assert!(edges.values().all(|v| *v == (2, 0)));
        assert!(volume > 0.);
        assert!(
            (volume / expected - 1.).abs() < 0.015,
            "{volume}/{expected}"
        );
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn authored_geometry_transactions_recovery_visibility_and_render() {
    let mut doc = doc();
    assert_eq!(doc.snapshot().version, 3);
    let before = canonical(doc.snapshot()).unwrap();
    let mut evaluator = Evaluator::default();
    let scene = evaluator.evaluate(doc.snapshot()).unwrap();
    assert_eq!(scene.conversions.len(), 1);
    assert_eq!(
        evaluator.evaluate(doc.snapshot()).unwrap().geometry_builds,
        0
    );
    assert_eq!(before, canonical(doc.snapshot()).unwrap());
    let envelope = Envelope::new(0, None, &doc).unwrap();
    let restored = envelope.decode().unwrap();
    assert_eq!(canonical(&doc).unwrap(), canonical(&restored).unwrap());
    let mut settings = fixtures::settings();
    settings.width = 24;
    settings.height = 24;
    settings.samples = 2;
    settings.camera = Camera {
        position: [0., 0.4, 2.5],
        target: [0., 0.4, 0.],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.7,
        lens: None,
    };
    settings.environment = [0.5; 3];
    let rendered = render(&scene, &settings, || false).unwrap();
    assert!(rendered.objects.iter().filter(|v| v.is_some()).count() > 10);
    assert!(rendered.receipt.approximation.contains("polygon sweeps"));
    let req = fixtures::request(
        &doc,
        "curves:visibility1",
        vec![Command::PutLayer {
            layer: Layer {
                name: "hide".into(),
                overrides: std::collections::BTreeMap::from([(
                    Id(3),
                    Override {
                        transform: None,
                        material: None,
                        hidden: true,
                    },
                )]),
            },
        }],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &req).unwrap();
    assert!(
        evaluator
            .evaluate(doc.snapshot())
            .unwrap()
            .instances
            .is_empty()
    );
    let before = canonical(&doc).unwrap();
    let mut req = fixtures::request(
        &doc,
        "curves:stale:0001",
        vec![Command::SetGeometry {
            entity: Id(3),
            asset: None,
        }],
    )
    .unwrap();
    req.base_revision = "stale".into();
    assert_eq!(
        doc.execute(&fixtures::principal(), &req).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(canonical(&doc).unwrap(), before);
    let req = fixtures::request(
        &doc,
        "curves:missing:01",
        vec![Command::SetGeometry {
            entity: Id(3),
            asset: Some("missing".into()),
        }],
    )
    .unwrap();
    assert_eq!(
        doc.execute(&fixtures::principal(), &req).unwrap_err().code,
        "reference"
    );
    assert_eq!(canonical(&doc).unwrap(), before);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn curve_failures_budgets_cancellation_and_schema_versions() {
    let mut a = asset();
    let Shape::Curves { curves } = &mut a.shape else {
        unreachable!()
    };
    curves[0].controls[1].radius = 0.;
    assert!(a.validate().is_err());
    let mut a = asset();
    a.tessellation.max_vertices = 6;
    assert_eq!(a.evaluate(|| false).unwrap_err().code, "budget");
    assert_eq!(asset().evaluate(|| true).unwrap_err().code, "cancelled");
    let mut calls = 0;
    assert_eq!(
        asset()
            .evaluate(|| {
                calls += 1;
                calls == 4
            })
            .unwrap_err()
            .code,
        "cancelled"
    );
    let mut a = asset();
    a.tessellation.max_depth = 1;
    a.tessellation.chord_error = 1e-12;
    assert_eq!(a.evaluate(|| false).unwrap_err().code, "convergence");
    let mut snapshot = doc().snapshot().clone();
    snapshot.version = 2;
    assert_eq!(snapshot.validate().unwrap_err().code, "schema_version");
    let mut evaluator = Evaluator::default();
    assert_eq!(
        evaluator
            .evaluate_with_cancel(doc().snapshot(), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    let mut doc = doc();
    let before = canonical(&doc).unwrap();
    let req = fixtures::request(
        &doc,
        "curves:readonly:1",
        vec![Command::SetGeometry {
            entity: Id(3),
            asset: None,
        }],
    )
    .unwrap();
    assert_eq!(
        doc.execute(
            &Principal {
                id: "reader".into(),
                can_write: false
            },
            &req
        )
        .unwrap_err()
        .code,
        "permission"
    );
    assert_eq!(canonical(&doc).unwrap(), before);
}
