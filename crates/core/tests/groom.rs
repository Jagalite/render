use glam::DVec3;
use render_core::{document::*, render::*, *};
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn groom_authored_children_deformation_cache_visibility_and_roundtrip() {
    let d = feature_fixtures::groom_document().unwrap();
    let before = canonical(&d).unwrap();
    let g = &d.snapshot().grooms[&Id(7601)];
    let strand_count = g.guides.len() + g.children.len();
    assert!(strand_count > 20);
    let derived = g.evaluate(d.snapshot(), Id(7601), || false).unwrap();
    assert_eq!(derived.receipt.curve_ranges.len(), strand_count);
    let mut evaluator = Evaluator::default();
    let scene = evaluator.evaluate(d.snapshot()).unwrap();
    assert_eq!(scene.instances.len(), 2);
    assert_eq!(evaluator.evaluate(d.snapshot()).unwrap().geometry_builds, 0);
    assert_eq!(
        canonical(
            &storage::Envelope::new(0, None, &d)
                .unwrap()
                .decode()
                .unwrap()
        )
        .unwrap(),
        before
    );
    // Deformation retains topology IDs; attachment frames and cache keys must follow it.
    let mut deformed = d.snapshot().clone();
    let anchor = deformed.entities.get_mut(Id(7600)).unwrap();
    let mut mesh = (*deformed.meshes[anchor.mesh.as_ref().unwrap()]).clone();
    for i in 0..mesh.positions.len() {
        let p = mesh.positions.get(i);
        mesh.positions.set(i, p + DVec3::new(0., 0.2, 0.)).unwrap();
    }
    let key = mesh.content_id().unwrap();
    deformed
        .meshes
        .insert(key.clone(), std::sync::Arc::new(mesh));
    anchor.mesh = Some(key);
    let moved = g.evaluate(&deformed, Id(7601), || false).unwrap();
    assert_ne!(moved.receipt.source_digest, derived.receipt.source_digest);
    for i in 0..derived.mesh.positions.len() {
        assert!(
            (moved.mesh.positions.get(i) - derived.mesh.positions.get(i) - DVec3::new(0., 0.2, 0.))
                .length()
                < 1e-12
        );
    }
    assert_eq!(evaluator.evaluate(&deformed).unwrap().geometry_builds, 2);
    // Invisible anchors still drive visible strands, without appearing in rendering.
    let mut hidden = d.snapshot().clone();
    hidden.layers.push(Layer {
        name: "anchor hidden".into(),
        overrides: std::collections::BTreeMap::from([(
            Id(7600),
            Override {
                transform: None,
                material: None,
                hidden: true,
            },
        )]),
    });
    let scene = evaluator.evaluate(&hidden).unwrap();
    assert_eq!(scene.instances.len(), 1);
    assert_eq!(scene.instances[0].id, Id(7601));
    let mut settings = d.snapshot().render_settings.clone().unwrap();
    settings.width = 24;
    settings.height = 24;
    settings.samples = 1;
    let image = render(&scene, &settings, || false).unwrap();
    assert!(
        image
            .objects
            .iter()
            .filter(|id| **id == Some(Id(7601)))
            .count()
            > 5
    );
    assert_eq!(canonical(&d).unwrap(), before);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn groom_root_topology_failures_permissions_cancellation_and_budgets() {
    let mut d = feature_fixtures::groom_document().unwrap();
    let before = canonical(&d).unwrap();
    let g = d.snapshot().grooms[&Id(7601)].clone();
    let mut bad = g.clone();
    bad.children[0].root.topology = "stale".into();
    assert_eq!(
        bad.validate(d.snapshot()).unwrap_err().code,
        "stale_topology"
    );
    bad = g.clone();
    bad.children[0].guide = Id(999);
    assert_eq!(bad.validate(d.snapshot()).unwrap_err().code, "reference");
    bad = g.clone();
    bad.children[0].root.barycentric = [1.; 3];
    assert_eq!(bad.validate(d.snapshot()).unwrap_err().code, "groom_root");
    bad = g.clone();
    bad.tessellation.max_vertices = 20;
    assert_eq!(
        bad.evaluate(d.snapshot(), Id(7601), || false)
            .unwrap_err()
            .code,
        "budget"
    );
    assert_eq!(
        g.evaluate(d.snapshot(), Id(7601), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    let mut calls = 0;
    assert_eq!(
        g.evaluate(d.snapshot(), Id(7601), || {
            calls += 1;
            calls == 20
        })
        .unwrap_err()
        .code,
        "cancelled"
    );
    let mut req = fixtures::request(
        &d,
        "groom:stale:00001",
        vec![Command::SetGroom {
            entity: Id(7601),
            groom: None,
        }],
    )
    .unwrap();
    req.base_revision = "stale".into();
    assert_eq!(
        d.execute(&fixtures::principal(), &req).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(canonical(&d).unwrap(), before);
    let req = fixtures::request(
        &d,
        "groom:readonly:01",
        vec![Command::SetGroom {
            entity: Id(7601),
            groom: None,
        }],
    )
    .unwrap();
    assert_eq!(
        d.execute(
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
    assert_eq!(canonical(&d).unwrap(), before);
    let mut snapshot = d.snapshot().clone();
    snapshot.version = 4;
    assert_eq!(snapshot.validate().unwrap_err().code, "schema_version");
}
