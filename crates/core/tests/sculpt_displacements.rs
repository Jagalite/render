use render_core::{
    Id, canonical,
    document::*,
    fixtures,
    geometry::{Mesh, Positions},
    render::*,
    sculpt::*,
};
use std::collections::BTreeMap;
fn setup(side: usize) -> (Document, String, String, Vec<u64>) {
    setup_ids(side, false)
}
fn setup_ids(side: usize, dense_ids: bool) -> (Document, String, String, Vec<u64>) {
    let positions = (0..=side)
        .flat_map(|y| {
            (0..=side).map(move |x| {
                [
                    x as f64 / side as f64 * 2. - 1.,
                    y as f64 / side as f64 * 2. - 1.,
                    0.,
                ]
            })
        })
        .collect();
    let mut faces = Vec::new();
    for y in 0..side {
        for x in 0..side {
            let i = (y * (side + 1) + x) as u32;
            let row = (side + 1) as u32;
            faces.extend([vec![i, i + 1, i + row + 1], vec![i, i + row + 1, i + row]]);
        }
    }
    let mut mesh = Mesh::from_polygons(Positions::F64(positions), &faces, &[]).unwrap();
    mesh.point_ids = (0..mesh.positions.len())
        .map(|i| {
            if dense_ids {
                65 + i as u64
            } else {
                9007199254740993 + i as u64 * 71
            }
        })
        .collect();
    *mesh.point_ids.last_mut().unwrap() = u64::MAX;
    let ids = mesh.point_ids.clone();
    let key = mesh.content_id().unwrap();
    let asset = Asset {
        version: 0,
        base_mesh: key.clone(),
        blocks: BTreeMap::new(),
    };
    let asset_key = asset.content_id().unwrap();
    let mut settings = fixtures::settings();
    settings.width = 64;
    settings.height = 64;
    settings.samples = 2;
    settings.camera.position = [0., 0., 4.];
    settings.camera.target = [0.; 3];
    settings.camera.up = [0., 1., 0.];
    let mut doc = Document::new(Snapshot::empty(Id(16000))).unwrap();
    let q = fixtures::request(
        &doc,
        "sculpt-initialize-001",
        vec![
            Command::PutMesh { mesh },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(1),
                    name: "sculpt".into(),
                    parent: None,
                    mesh: Some(key.clone()),
                    material: None,
                    transform: Transform::default(),
                },
            },
            Command::SetRenderSettings { settings },
            Command::PutSculptAsset { asset },
            Command::SetSculpt {
                entity: Id(1),
                source_mesh: key.clone(),
                source_asset: None,
                asset: Some(asset_key.clone()),
            },
        ],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &q).unwrap();
    (doc, key, asset_key, ids)
}
fn update(
    doc: &mut Document,
    key: &str,
    old: &str,
    point: u64,
    delta: [f32; 3],
    name: &str,
) -> String {
    let block = BlockId(point >> 6);
    let chunk = Chunk {
        version: 0,
        base_mesh: key.into(),
        block,
        points: vec![Entry {
            slot: (point & 63) as u8,
            delta_meters: delta,
        }],
    };
    let ckey = chunk.content_id().unwrap();
    let mut asset = doc.snapshot().sculpt_assets[old].as_ref().clone();
    asset.blocks.insert(block, ckey);
    let asset_key = asset.content_id().unwrap();
    let q = fixtures::request(
        doc,
        name,
        vec![
            Command::PutSculptChunk { chunk },
            Command::PutSculptAsset { asset },
            Command::SetSculpt {
                entity: Id(1),
                source_mesh: key.into(),
                source_asset: Some(old.into()),
                asset: Some(asset_key.clone()),
            },
        ],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &q).unwrap();
    asset_key
}
fn same(a: &Image, b: &Image) {
    assert_eq!(a.linear, b.linear);
    assert_eq!(a.depth, b.depth);
    assert_eq!(a.normals, b.normals);
    assert_eq!(a.objects, b.objects);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn canonical_blocks_references_and_version_gate() {
    for value in ["0", "1", "288230376151711743"] {
        let block: BlockId = serde_json::from_str(&format!("\"{value}\"")).unwrap();
        assert_eq!(
            serde_json::to_string(&block).unwrap(),
            format!("\"{value}\"")
        );
    }
    for value in [
        "00",
        "+1",
        "-1",
        "288230376151711744",
        "18446744073709551616",
    ] {
        assert!(serde_json::from_str::<BlockId>(&format!("\"{value}\"")).is_err());
    }
    assert!(serde_json::from_str::<BlockId>("1").is_err());
    let (doc, _, _, _) = setup(2);
    assert_eq!(doc.snapshot().version, 19);
    let mut old = doc.snapshot().clone();
    old.version = 18;
    assert_eq!(old.validate().unwrap_err().code, "schema_version");
    let mut future = doc.snapshot().clone();
    future.version = 20;
    assert_eq!(future.validate().unwrap_err().code, "schema_version");
    let reopened: Document = serde_json::from_slice(&canonical(&doc).unwrap()).unwrap();
    assert_eq!(canonical(&doc).unwrap(), canonical(&reopened).unwrap());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn local_refit_matches_fresh_renders_and_undo() {
    let (mut doc, key, original, ids) = setup(16);
    let settings = doc.snapshot().render_settings.clone().unwrap();
    let mut evaluator = Evaluator::default();
    let before = evaluator.evaluate(doc.snapshot()).unwrap();
    let before_image = render(&before, &settings, || false).unwrap();
    let changed = update(
        &mut doc,
        &key,
        &original,
        ids[8 * 17 + 8],
        [0., 0., 0.25],
        "sculpt-local-change-001",
    );
    let scene = evaluator.evaluate(doc.snapshot()).unwrap();
    let report = &scene.sculpt_evaluations[0].1;
    assert!(report.basis_reused);
    assert_eq!(report.changed_points, 1);
    assert_eq!(report.changed_triangles, 6);
    assert!(report.refit_nodes < before.instances[0].geometry.bvh.nodes.len());
    assert!(report.triangle_elements_cloned < 512);
    assert!(
        before.instances[0]
            .geometry
            .triangles
            .shared_chunks(&scene.instances[0].geometry.triangles)
            > 0
    );
    let image = render(&scene, &settings, || false).unwrap();
    assert_ne!(image.depth, before_image.depth);
    let fresh = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    same(&image, &render(&fresh, &settings, || false).unwrap());
    for (a, b) in scene.instances[0]
        .geometry
        .bvh
        .nodes
        .iter()
        .zip(&fresh.instances[0].geometry.bvh.nodes)
    {
        assert_eq!(a.bounds.min.to_array(), b.bounds.min.to_array());
        assert_eq!(a.bounds.max.to_array(), b.bounds.max.to_array());
        assert_eq!(a.items, b.items);
        assert_eq!(a.children, b.children);
    }
    let repeat = evaluator.evaluate(doc.snapshot()).unwrap();
    assert!(repeat.sculpt_evaluations[0].1.exact_geometry_reused);
    let q = fixtures::request(
        &doc,
        "sculpt-undo-local-001",
        vec![Command::SetSculpt {
            entity: Id(1),
            source_mesh: key,
            source_asset: Some(changed),
            asset: Some(original),
        }],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &q).unwrap();
    let undo = evaluator.evaluate(doc.snapshot()).unwrap();
    same(&before_image, &render(&undo, &settings, || false).unwrap());
    assert_eq!(
        before.instances[0].geometry.triangles[0].positions,
        undo.instances[0].geometry.triangles[0].positions
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn cancelled_evaluation_preserves_old_cache() {
    let (mut doc, key, old, ids) = setup(2);
    let initial = doc.snapshot().clone();
    update(
        &mut doc,
        &key,
        &old,
        ids[4],
        [0., 0., 0.2],
        "sculpt-cancel-input-001",
    );
    let mut probe = Evaluator::default();
    probe.evaluate(&initial).unwrap();
    let mut count = 0;
    probe
        .evaluate_with_cancel(doc.snapshot(), || {
            count += 1;
            false
        })
        .unwrap();
    for stop in 1..=count {
        let mut e = Evaluator::default();
        e.evaluate(&initial).unwrap();
        let mut calls = 0;
        assert_eq!(
            e.evaluate_with_cancel(doc.snapshot(), || {
                calls += 1;
                calls == stop
            })
            .unwrap_err()
            .code,
            "cancelled"
        );
        assert!(
            e.evaluate(&initial).unwrap().sculpt_evaluations[0]
                .1
                .exact_geometry_reused
        );
    }
}

fn author_request(doc: &Document, source: &str, point: u64) -> render_core::sculpt::Request {
    render_core::sculpt::Request {
        version: 0,
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "sculpt-authoring-point-001".into(),
        entity: Id(1),
        source_asset: source.into(),
        points: vec![PointUpdate {
            point: render_core::geometry_query::ElementId(point),
            delta_meters: [0., 0., 0.25],
        }],
        budget: Budget::default(),
        max_added_bytes: 1 << 20,
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn public_authoring_durable_retry_and_warm_session_render() {
    use render_core::{agent, storage::*};
    let (doc, _, source, ids) = setup(4);
    let q = author_request(&doc, &source, ids[12]);
    let p = fixtures::principal();
    let mut session = agent::Session::new(doc);
    let revision = session.document().snapshot().revision().unwrap();
    let baseline = session.render_root_cpu(&revision, || false).unwrap();
    assert_eq!(baseline["sculpt_evaluations"][0][1]["basis_reused"], false);
    let request = agent::Request {
        version: 0,
        operation: agent::Operation::AuthorSculpt { request: q },
    };
    let before = canonical(session.document()).unwrap();
    let mut store = MemoryStore {
        fail_publication: true,
        ..Default::default()
    };
    assert_eq!(
        session
            .dispatch_durable(&mut store, &p, request.clone())
            .unwrap_err()
            .code,
        "storage"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    store.fail_publication = false;
    let out = session
        .dispatch_durable(&mut store, &p, request.clone())
        .unwrap();
    assert_eq!(out["receipt"]["durable"], true);
    session.dispatch_durable(&mut store, &p, request).unwrap();
    assert_eq!(store.records.len(), 1);
    let revision = session.document().snapshot().revision().unwrap();
    let image = session.render_root_cpu(&revision, || false).unwrap();
    assert_eq!(image["sculpt_evaluations"][0][1]["basis_reused"], true);
    assert_eq!(image["sculpt_evaluations"][0][1]["changed_points"], 1);
    assert_ne!(
        image["passes"]["depth_meters"],
        baseline["passes"]["depth_meters"]
    );
    let reopened: Document =
        serde_json::from_slice(&canonical(session.document()).unwrap()).unwrap();
    let mut cold = agent::Session::new(reopened);
    assert_eq!(
        image["passes"],
        cold.render_root_cpu(&revision, || false).unwrap()["passes"]
    );
    assert_eq!(
        session.render_root_cpu("stale", || false).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(
        session
            .render_root_cpu(&revision, || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert_eq!(
        session.render_root_cpu(&revision, || false).unwrap()["sculpt_evaluations"][0][1]["exact_geometry_reused"],
        true
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn authoring_rejects_invalid_stale_and_cancelled_without_mutation() {
    let (doc, _, source, ids) = setup(2);
    let p = fixtures::principal();
    let q = author_request(&doc, &source, ids[4]);
    let before = canonical(&doc).unwrap();
    let mut denied = p.clone();
    denied.can_write = false;
    assert_eq!(
        prepare(&doc, &denied, &q, || false).err().unwrap().code,
        "permission"
    );
    for (bad, code) in [
        {
            let mut v = q.clone();
            v.base_revision = "stale".into();
            (v, "stale_revision")
        },
        {
            let mut v = q.clone();
            v.points.push(v.points[0].clone());
            (v, "sculpt_point")
        },
        {
            let mut v = q.clone();
            v.points[0].point.0 = 1;
            (v, "sculpt_point")
        },
        {
            let mut v = q.clone();
            v.points[0].delta_meters[2] = f32::NAN;
            (v, "sculpt_metric")
        },
        {
            let mut v = q.clone();
            v.budget.max_basis_bytes = 1;
            (v, "budget")
        },
        {
            let mut v = q.clone();
            v.budget.max_copy_bytes = 1;
            (v, "budget")
        },
        {
            let mut v = q.clone();
            v.max_added_bytes = 1;
            (v, "budget")
        },
    ] {
        assert_eq!(prepare(&doc, &p, &bad, || false).err().unwrap().code, code);
    }
    let mut count = 0;
    prepare(&doc, &p, &q, || {
        count += 1;
        false
    })
    .unwrap();
    for stop in 1..=count {
        let mut n = 0;
        assert_eq!(
            prepare(&doc, &p, &q, || {
                n += 1;
                n == stop
            })
            .err()
            .unwrap()
            .code,
            "cancelled"
        );
        assert_eq!(canonical(&doc).unwrap(), before);
    }
    let mut changed = doc.clone();
    let prepared = prepare(&changed, &p, &q, || false).unwrap();
    changed.execute(&p, &prepared.transaction).unwrap();
    let mut stale = q.clone();
    stale.base_revision = changed.snapshot().revision().unwrap();
    stale.idempotency_key = "sculpt-stale-selection-001".into();
    assert_eq!(
        prepare(&changed, &p, &stale, || false).err().unwrap().code,
        "stale_selection"
    );
    let mut undo = author_request(&changed, &prepared.report.output, ids[4]);
    undo.idempotency_key = "sculpt-clear-point-001".into();
    undo.points[0].delta_meters = [-0., 0., -0.];
    let cleared = prepare(&changed, &p, &undo, || false).unwrap();
    assert_eq!(cleared.report.output, source);
    changed.execute(&p, &cleared.transaction).unwrap();
    assert_eq!(changed.snapshot().sculpt_bindings[&Id(1)], source);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn shared_blocks_keep_other_points_and_reject_collapsed_triangles() {
    let (mut doc, _, source, ids) = setup_ids(2, true);
    let p = fixtures::principal();
    let q = author_request(&doc, &source, ids[0]);
    let first = prepare(&doc, &p, &q, || false).unwrap();
    doc.execute(&p, &first.transaction).unwrap();
    let mut q = author_request(&doc, &first.report.output, ids[1]);
    q.idempotency_key = "sculpt-same-block-002".into();
    let second = prepare(&doc, &p, &q, || false).unwrap();
    assert_eq!(second.report.changed_chunks, 1);
    doc.execute(&p, &second.transaction).unwrap();
    let asset = &doc.snapshot().sculpt_assets[&second.report.output];
    let chunk = &doc.snapshot().sculpt_chunks[&asset.blocks[&BlockId(1)]];
    assert_eq!(chunk.points.len(), 2);
    assert_eq!(chunk.points[0].slot, 1);
    assert_eq!(chunk.points[1].slot, 2);
    assert_eq!(
        doc.snapshot().sculpt_assets[&first.report.output]
            .blocks
            .len(),
        1
    );
    let mut q = author_request(&doc, &second.report.output, ids[0]);
    q.idempotency_key = "sculpt-same-block-clear".into();
    q.points[0].delta_meters = [0.; 3];
    let cleared = prepare(&doc, &p, &q, || false).unwrap();
    doc.execute(&p, &cleared.transaction).unwrap();
    let asset = &doc.snapshot().sculpt_assets[&cleared.report.output];
    let chunk = &doc.snapshot().sculpt_chunks[&asset.blocks[&BlockId(1)]];
    assert_eq!(chunk.points.len(), 1);
    assert_eq!(chunk.points[0].slot, 2);
    // Point 0 collapses onto displaced point 1: publication must reject it.
    let mut bad = author_request(&doc, &cleared.report.output, ids[0]);
    bad.idempotency_key = "sculpt-collapse-reject".into();
    bad.points[0].delta_meters = [1., 0., 0.25];
    assert_eq!(
        prepare(&doc, &p, &bad, || false).err().unwrap().code,
        "sculpt_degenerate"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn cache_retains_old_chunks_across_checkpoint_pruning_and_mesh_eviction() {
    let (mut doc, key, source, ids) = setup(2);
    let mut evaluator = Evaluator::default();
    let first = update(
        &mut doc,
        &key,
        &source,
        ids[4],
        [0., 0., 0.1],
        "sculpt-cache-first-001",
    );
    evaluator.evaluate(doc.snapshot()).unwrap();
    let second = update(
        &mut doc,
        &key,
        &first,
        ids[4],
        [0., 0., 0.3],
        "sculpt-cache-second-002",
    );
    let mut checkpoint = doc.snapshot().clone();
    checkpoint.sculpt_assets.retain(|k, _| k == &second);
    let retained = checkpoint.sculpt_assets[&second]
        .blocks
        .values()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    checkpoint.sculpt_chunks.retain(|k, _| retained.contains(k));
    checkpoint.validate().unwrap();
    let warm = evaluator.evaluate(&checkpoint).unwrap();
    assert!(warm.sculpt_evaluations[0].1.basis_reused);
    let fresh = Evaluator::default().evaluate(&checkpoint).unwrap();
    let settings = checkpoint.render_settings.as_ref().unwrap();
    same(
        &render(&warm, settings, || false).unwrap(),
        &render(&fresh, settings, || false).unwrap(),
    );
    let other = setup(3).0;
    evaluator.evaluate(other.snapshot()).unwrap();
    let evicted = evaluator.evaluate(&checkpoint).unwrap();
    assert!(!evicted.sculpt_evaluations[0].1.basis_reused);
    same(
        &render(&warm, settings, || false).unwrap(),
        &render(&evicted, settings, || false).unwrap(),
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn flat_normals_corner_seams_and_source_profile_are_explicit() {
    use render_core::geometry::{Attribute, AttributeValues, Domain, Transfer};
    use std::sync::Arc;
    let (doc, key, source, ids) = setup(1);
    let mut mesh = doc.snapshot().meshes[&key].as_ref().clone();
    mesh.attributes.insert(
        "uv".into(),
        Attribute {
            id: Id(21),
            domain: Domain::Corner,
            semantic: "uv".into(),
            transfer: Transfer::Linear,
            values: AttributeValues::Vec2(vec![
                [0., 0.],
                [1., 0.],
                [1., 1.],
                [0.25, 0.25],
                [0.75, 0.75],
                [0., 1.],
            ]),
        },
    );
    let mut snapshot = doc.snapshot().clone();
    let new_key = mesh.content_id().unwrap();
    snapshot
        .meshes
        .insert(new_key.clone(), Arc::new(mesh.clone()));
    let asset = Asset {
        version: 0,
        base_mesh: new_key.clone(),
        blocks: BTreeMap::new(),
    };
    let new_source = asset.content_id().unwrap();
    snapshot
        .sculpt_assets
        .insert(new_source.clone(), Arc::new(asset));
    snapshot.sculpt_bindings.insert(Id(1), new_source.clone());
    snapshot.entities.get_mut(Id(1)).unwrap().mesh = Some(new_key.clone());
    let mut doc = Document::new(snapshot.clone()).unwrap();
    let mut q = author_request(&doc, &new_source, ids[0]);
    q.points = ids
        .iter()
        .map(|id| PointUpdate {
            point: render_core::geometry_query::ElementId(*id),
            delta_meters: [0., 0., 0.5],
        })
        .collect();
    let prepared = prepare(&doc, &fixtures::principal(), &q, || false).unwrap();
    doc.execute(&fixtures::principal(), &prepared.transaction)
        .unwrap();
    let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    let triangles = &scene.instances[0].geometry.triangles;
    assert_ne!(triangles[0].uv[0], triangles[1].uv[0]);
    for t in triangles {
        assert!(t.normals.is_none());
        assert!(t.positions.iter().all(|p| p.z == 0.5));
        assert_eq!(
            (t.positions[1] - t.positions[0])
                .cross(t.positions[2] - t.positions[0])
                .normalize(),
            glam::DVec3::Z
        );
        assert_eq!(t.uv_sets[0], t.uv);
    }
    assert_eq!(snapshot.sculpt_assets[&source].base_mesh, key);
    for semantic in ["normal", "tangent", "tangent_sign"] {
        let mut bad = snapshot.clone();
        let mut shaped = mesh.clone();
        shaped.attributes.insert(
            semantic.into(),
            Attribute {
                id: Id(22),
                domain: Domain::Corner,
                semantic: semantic.into(),
                transfer: Transfer::Normalize,
                values: if semantic == "tangent_sign" {
                    AttributeValues::Scalar(vec![1.; 6])
                } else {
                    AttributeValues::Vec3(vec![[0., 0., 1.]; 6])
                },
            },
        );
        let key = shaped.content_id().unwrap();
        bad.meshes.insert(key.clone(), Arc::new(shaped));
        let asset = Asset {
            version: 0,
            base_mesh: key.clone(),
            blocks: BTreeMap::new(),
        };
        let hash = asset.content_id().unwrap();
        bad.sculpt_assets.insert(hash.clone(), Arc::new(asset));
        bad.sculpt_bindings.insert(Id(1), hash);
        bad.entities.get_mut(Id(1)).unwrap().mesh = Some(key);
        assert_eq!(bad.validate().unwrap_err().code, "sculpt_profile");
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sculpt_refit_resource_counts_and_late_agent_cancellation() {
    let (mut doc, key, source, ids) = setup(32);
    let mut evaluator = Evaluator::default();
    let original = evaluator.evaluate(doc.snapshot()).unwrap();
    update(
        &mut doc,
        &key,
        &source,
        ids[16 * 33 + 16],
        [0., 0., 0.25],
        "sculpt-resource-change-001",
    );
    let edited = evaluator.evaluate(doc.snapshot()).unwrap();
    let report = &edited.sculpt_evaluations[0].1;
    assert_eq!(report.changed_points, 1);
    assert_eq!(report.changed_triangles, 6);
    assert_eq!(report.built_triangles, 0);
    assert_eq!(report.instance_bound_vertices_scanned, 6144);
    assert!(report.refit_nodes < edited.instances[0].geometry.bvh.nodes.len());
    assert!(report.triangle_elements_cloned < 2048);
    assert_eq!(report.triangle_root_references_copied, 32);
    assert_eq!(report.retained_chunk_references_copied, 1);
    assert!(
        report.basis_geometry_layout_bytes
            + report.basis_nested_layout_bytes
            + report.basis_mapping_layout_bytes
            < report.basis_byte_charge as usize
    );
    let resources = serde_json::json!({"cold":original.sculpt_evaluations[0].1,"warm":report,
        "base_current_shared_triangle_chunks":original.instances[0].geometry.triangles.shared_chunks(&edited.instances[0].geometry.triangles),
        "base_current_shared_node_chunks":original.instances[0].geometry.bvh.nodes.shared_chunks(&edited.instances[0].geometry.bvh.nodes),
        "scope":"separate basis/current layouts can share chunks; excludes allocator and Arc headers, private map allocations, source and global admission temporaries"});
    #[cfg(not(target_arch = "wasm32"))]
    println!("SCULPT_RESOURCE {}", resources);
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test::console_log!("SCULPT_RESOURCE {}", resources);
    use render_core::agent;
    let (doc, _, source, ids) = setup(2);
    let q = agent::Request {
        version: 0,
        operation: agent::Operation::AuthorSculpt {
            request: author_request(&doc, &source, ids[4]),
        },
    };
    let p = fixtures::principal();
    let mut count = 0;
    agent::Session::new(doc.clone())
        .dispatch_cancellable(&p, q.clone(), || {
            count += 1;
            false
        })
        .unwrap();
    for stop in 1..=count {
        let mut session = agent::Session::new(doc.clone());
        let before = canonical(session.document()).unwrap();
        let mut n = 0;
        assert_eq!(
            session
                .dispatch_cancellable(&p, q.clone(), || {
                    n += 1;
                    n == stop
                })
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert_eq!(canonical(session.document()).unwrap(), before);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn agent_root_render_forwards_cancellation_and_preserves_previous_cache() {
    use render_core::agent;
    let (mut doc, key, source, ids) = setup(2);
    let revision = doc.snapshot().revision().unwrap();
    let mut session = agent::Session::new(doc.clone());
    session.render_root_cpu(&revision, || false).unwrap();
    update(
        &mut doc,
        &key,
        &source,
        ids[4],
        [0., 0., 0.25],
        "sculpt-root-cancel-edit",
    );
    // Author the same edit through the public operation on the warmed session.
    let q = author_request(session.document(), &source, ids[4]);
    session
        .dispatch(
            &fixtures::principal(),
            agent::Request {
                version: 0,
                operation: agent::Operation::AuthorSculpt { request: q },
            },
        )
        .unwrap();
    let revision = session.document().snapshot().revision().unwrap();
    let request = agent::Request {
        version: 0,
        operation: agent::Operation::RenderRootCpu {
            revision: revision.clone(),
        },
    };
    let before = canonical(session.document()).unwrap();
    let mut probe = session.clone();
    let mut count = 0;
    probe
        .dispatch_cancellable(&fixtures::principal(), request.clone(), || {
            count += 1;
            false
        })
        .unwrap();
    assert!(count > 3);
    for stop in [2, count - 1, count] {
        let mut candidate = session.clone();
        let mut n = 0;
        assert_eq!(
            candidate
                .dispatch_cancellable(&fixtures::principal(), request.clone(), || {
                    n += 1;
                    n == stop
                })
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert_eq!(canonical(candidate.document()).unwrap(), before);
        let next = candidate.render_root_cpu(&revision, || false).unwrap();
        assert_eq!(next["sculpt_evaluations"][0][1]["changed_points"], 1);
        assert_eq!(next["sculpt_evaluations"][0][1]["basis_reused"], true);
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn empty_surface_and_invalid_sparse_chunks_are_rejected() {
    let (doc, key, _, ids) = setup(1);
    let chunk = Chunk {
        version: 0,
        base_mesh: key,
        block: BlockId(ids[0] >> 6),
        points: vec![Entry {
            slot: (ids[0] & 63) as u8,
            delta_meters: [0., 0., 1.],
        }],
    };
    for bad in [
        {
            let mut c = chunk.clone();
            c.points.clear();
            c
        },
        {
            let mut c = chunk.clone();
            c.points[0].delta_meters = [0.; 3];
            c
        },
        {
            let mut c = chunk.clone();
            c.points.push(c.points[0].clone());
            c
        },
        {
            let mut c = chunk.clone();
            c.points[0].slot = 64;
            c
        },
        {
            let mut c = chunk.clone();
            c.block = BlockId(0);
            c.points[0].slot = 0;
            c
        },
        {
            let mut c = chunk.clone();
            c.block = BlockId(u64::MAX);
            c
        },
    ] {
        assert_eq!(bad.content_id().unwrap_err().code, "sculpt_chunk");
    }
    let mesh = Mesh::from_polygons(Positions::F64(vec![[0.; 3]]), &[], &[]).unwrap();
    let key = mesh.content_id().unwrap();
    let q = fixtures::request(
        &doc,
        "sculpt-empty-surface-reject",
        vec![
            Command::PutMesh { mesh },
            Command::PutSculptAsset {
                asset: Asset {
                    version: 0,
                    base_mesh: key,
                    blocks: BTreeMap::new(),
                },
            },
        ],
    )
    .unwrap();
    assert_eq!(
        doc.prepare(&fixtures::principal(), &q).unwrap_err().code,
        "sculpt_profile"
    );
}
