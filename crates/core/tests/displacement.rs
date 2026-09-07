use render_core::{displacement::*, document::*, geometry::*, render::*, *};
use std::collections::BTreeMap;
fn plane() -> Mesh {
    let mut m = Mesh::from_polygons(
        Positions::F64(vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]),
        &[vec![0, 1, 2, 3]],
        &[],
    )
    .unwrap();
    m.attributes.insert(
        "uv".into(),
        Attribute {
            id: Id(1),
            domain: Domain::Corner,
            semantic: "uv".into(),
            transfer: Transfer::Linear,
            values: AttributeValues::Vec2(vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]),
        },
    );
    m
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn displacement_metric_height_subdivision_convergence_and_limits() {
    let mesh = plane();
    let original = canonical(&mesh).unwrap();
    let mut d = Displacement {
        height: Height::Wave {
            amplitude_meters: 0.1,
            frequency: [1., 0.],
            phase_radians: 0.,
        },
        subdivisions: 2,
        max_vertices: 65536,
    };
    let mut errors = vec![];
    for level in 2..=5 {
        d.subdivisions = level;
        let (m, receipt) = d.evaluate(&mesh, &BTreeMap::new(), || false).unwrap();
        assert_eq!(receipt.triangles, 2 * 4usize.pow(u32::from(level)));
        for i in 0..m.positions.len() {
            let p = m.positions.get(i);
            assert!((p.z - 0.1 * (std::f64::consts::TAU * p.x).sin()).abs() < 1e-12);
        }
        let mut max_error: f64 = 0.;
        for t in m.triangles().unwrap() {
            let p = t.map(|c| m.positions.get(m.corners[c as usize].vertex as usize));
            let at = (p[0] + p[1] + p[2]) / 3.;
            max_error = max_error.max((at.z - 0.1 * (std::f64::consts::TAU * at.x).sin()).abs());
        }
        errors.push(max_error);
    }
    assert!(errors.windows(2).all(|e| e[1] < e[0] * 0.35), "{errors:?}");
    assert_eq!(canonical(&mesh).unwrap(), original);
    d.max_vertices = 3;
    assert_eq!(
        d.evaluate(&mesh, &BTreeMap::new(), || false)
            .unwrap_err()
            .code,
        "budget"
    );
    d.max_vertices = 65536;
    assert_eq!(
        d.evaluate(&mesh, &BTreeMap::new(), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn surface_displacement_cache_transactions_and_recovery() {
    let mut d = feature_fixtures::surface_document().unwrap();
    let before = canonical(&d).unwrap();
    let restored = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(canonical(&restored).unwrap(), before);
    let mut evaluator = Evaluator::default();
    let scene = evaluator.evaluate(d.snapshot()).unwrap();
    assert_eq!(scene.displacements.len(), 1);
    assert_eq!(evaluator.evaluate(d.snapshot()).unwrap().geometry_builds, 0);
    let mut s = d.snapshot().render_settings.clone().unwrap();
    s.width = 24;
    s.height = 16;
    s.samples = 2;
    let image = render(&scene, &s, || false).unwrap();
    assert!(image.objects.contains(&Some(Id(8620))));
    assert_eq!(image.receipt.backend, "cpu-f64-extended-bsdf-v0");
    assert_eq!(canonical(&d).unwrap(), before);
    let mut material = d.snapshot().materials[&Id(8630)].clone();
    material
        .pbr
        .as_mut()
        .unwrap()
        .displacement
        .as_mut()
        .unwrap()
        .height = Height::Constant { meters: 0.1 };
    let req = fixtures::request(
        &d,
        "displace:change:01",
        vec![Command::PutMaterial {
            material: Box::new(material),
        }],
    )
    .unwrap();
    d.execute(&fixtures::principal(), &req).unwrap();
    assert_eq!(evaluator.evaluate(d.snapshot()).unwrap().geometry_builds, 1);
    let before = canonical(&d).unwrap();
    let mut req = fixtures::request(
        &d,
        "displace:stale:01",
        vec![Command::Rename {
            entity: Id(8631),
            name: "new".into(),
        }],
    )
    .unwrap();
    req.base_revision = "stale".into();
    assert_eq!(
        d.execute(&fixtures::principal(), &req).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(canonical(&d).unwrap(), before);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn displacement_loose_data_and_root_evaluation_cancellation() {
    let d = Displacement {
        height: Height::Constant { meters: 0. },
        subdivisions: 0,
        max_vertices: 65536,
    };
    assert_eq!(
        d.evaluate(&fixtures::seam_mesh().unwrap(), &BTreeMap::new(), || false)
            .unwrap_err()
            .code,
        "topology_profile"
    );
    let document = feature_fixtures::surface_document().unwrap();
    let revision = document.snapshot().revision().unwrap();
    let mut session = agent::Session::new(document);
    assert_eq!(
        session
            .render_root_cpu(&revision, || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert_eq!(
        session.render_root_cpu("stale", || false).unwrap_err().code,
        "stale_revision"
    );
}
