use render_core::{geometry::*, modeling::*, *};
fn volume(m: &Mesh) -> f64 {
    m.triangles()
        .unwrap()
        .iter()
        .map(|t| {
            let p = t.map(|c| m.positions.get(m.corners[c as usize].vertex as usize));
            p[0].dot(p[1].cross(p[2])) / 6.
        })
        .sum()
}
fn closed(m: &Mesh) {
    let mut edges = std::collections::BTreeMap::new();
    for w in m.face_offsets.windows(2) {
        for i in w[0]..w[1] {
            let a = m.corners[i as usize].vertex;
            let b = m.corners[if i + 1 == w[1] { w[0] } else { i + 1 } as usize].vertex;
            let entry = edges.entry((a.min(b), a.max(b))).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(edges.values().all(|v| *v == (2, 0)), "{edges:?}");
}
fn apply(m: &Mesh, op: Operation) -> (Mesh, Receipt) {
    render_core::modeling::apply(m, &op, &Budget::default(), || false).unwrap()
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn primitives_extrude_inset_split_weld_and_correspondence() {
    let cube = box_mesh([-1.; 3], [1.; 3]).unwrap();
    assert!((volume(&cube) - 8.).abs() < 1e-12);
    closed(&cube);
    let (extruded, receipt) = apply(
        &cube,
        Operation::ExtrudeFace {
            face: cube.face_ids[1],
            distance_meters: 0.5,
        },
    );
    closed(&extruded);
    assert!((volume(&extruded) - 10.).abs() < 1e-12);
    assert_eq!(receipt.points.created.len(), 4);
    assert!(!receipt.faces.split.is_empty());
    let (inset, _) = apply(
        &cube,
        Operation::InsetFace {
            face: cube.face_ids[1],
            fraction: 0.25,
        },
    );
    closed(&inset);
    assert!((volume(&inset) - 8.).abs() < 1e-12);
    let (split, receipt) = apply(
        &cube,
        Operation::SplitEdge {
            edge: cube.edge_ids[0],
            fraction: 0.5,
        },
    );
    closed(&split);
    assert!((volume(&split) - 8.).abs() < 1e-12);
    assert_eq!(receipt.points.created.len(), 1);
    assert!(receipt.points.merged.contains(&receipt.points.created[0]));
    let id = receipt.points.created[0];
    let (welded, _) = apply(
        &split,
        Operation::Weld {
            points: vec![id, cube.point_ids[cube.edges[0][0] as usize]],
        },
    );
    closed(&welded);
    assert!(volume(&welded) > 0.);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn subdivision_mirror_array_solidify_and_box_csg_numeric() {
    let cube = box_mesh([-1.; 3], [1.; 3]).unwrap();
    let (sub, _) = apply(&cube, Operation::Subdivide);
    closed(&sub);
    assert_eq!(sub.positions.len(), 26);
    assert_eq!(sub.faces(), 48);
    for i in 0..8 {
        let p = sub.positions.get(i);
        assert!((p.abs().max_element() - 5. / 9.).abs() < 1e-12);
    }
    let (mirrored, _) = apply(
        &cube,
        Operation::Mirror {
            axis: 0,
            offset_meters: 2.,
        },
    );
    closed(&mirrored);
    assert!((volume(&mirrored) - 16.).abs() < 1e-12);
    let (array, _) = apply(
        &cube,
        Operation::Array {
            count: 3,
            step: [3., 0., 0.],
        },
    );
    closed(&array);
    assert!((volume(&array) - 24.).abs() < 1e-12);
    let plane = Mesh::from_polygons(
        Positions::F64(vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]),
        &[vec![0, 1, 2, 3]],
        &[],
    )
    .unwrap();
    let (shell, _) = apply(
        &plane,
        Operation::Solidify {
            thickness_meters: 0.2,
        },
    );
    closed(&shell);
    assert!((volume(&shell) - 0.2).abs() < 1e-12);
    for (operation, expected) in [
        (Boolean::Union, 12.),
        (Boolean::Intersection, 4.),
        (Boolean::Difference, 4.),
    ] {
        let (result, receipt) = apply(
            &cube,
            Operation::BooleanBox {
                operation,
                min: [0., -1., -1.],
                max: [2., 1., 1.],
            },
        );
        closed(&result);
        assert!((volume(&result) - expected).abs() < 1e-12);
        assert!(!receipt.faces.ambiguous.is_empty());
    }
    let (bevel, _) = apply(
        &cube,
        Operation::BevelBox {
            distance_meters: 0.2,
        },
    );
    closed(&bevel);
    assert_eq!(bevel.faces(), 26);
    assert!(volume(&bevel) < 8. && volume(&bevel) > 7.);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn bridge_dissolve_attribute_transfer_invalid_selection_budget_and_cancel() {
    let m = Mesh::from_polygons(
        Positions::F64(vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ]),
        &[vec![0, 3, 2, 1], vec![4, 5, 6, 7]],
        &[],
    )
    .unwrap();
    let (bridged, _) = apply(
        &m,
        Operation::Bridge {
            first: vec![1, 4, 3, 2],
            second: vec![5, 8, 7, 6],
        },
    );
    closed(&bridged);
    assert!((volume(&bridged) - 1.).abs() < 1e-12);
    let mut plane = Mesh::from_polygons(
        Positions::F64(vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]),
        &[vec![0, 1, 2], vec![0, 2, 3]],
        &[],
    )
    .unwrap();
    plane.attributes.insert(
        "temperature".into(),
        Attribute {
            id: Id(1),
            domain: Domain::Point,
            semantic: "temperature".into(),
            transfer: Transfer::Linear,
            values: AttributeValues::Scalar(vec![0., 1., 2., 1.]),
        },
    );
    let edge = plane
        .edges
        .iter()
        .position(|e| e == &[0, 2] || e == &[2, 0])
        .unwrap();
    let (dissolved, _) = apply(
        &plane,
        Operation::DissolveEdge {
            edge: plane.edge_ids[edge],
        },
    );
    assert_eq!(dissolved.faces(), 1);
    assert_eq!(
        dissolved.attributes["temperature"],
        plane.attributes["temperature"]
    );
    let (split, receipt) = apply(
        &plane,
        Operation::SplitEdge {
            edge: plane.edge_ids[0],
            fraction: 0.5,
        },
    );
    let AttributeValues::Scalar(values) = &split.attributes["temperature"].values else {
        panic!()
    };
    let i = split
        .point_ids
        .iter()
        .position(|id| receipt.points.created.contains(id))
        .unwrap();
    assert!((values[i] - 0.5).abs() < 1e-7);
    let bad = Operation::SplitEdge {
        edge: 999,
        fraction: 0.5,
    };
    assert_eq!(
        render_core::modeling::apply(&plane, &bad, &Budget::default(), || false)
            .unwrap_err()
            .code,
        "stale_selection"
    );
    assert_eq!(
        render_core::modeling::apply(&plane, &bad, &Budget::default(), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    let before = canonical(&plane).unwrap();
    let b = Budget {
        vertices: 4,
        faces: 4,
        bytes: 1024,
    };
    assert_eq!(
        render_core::modeling::apply(
            &plane,
            &Operation::Array {
                count: 100,
                step: [1., 0., 0.]
            },
            &b,
            || false
        )
        .unwrap_err()
        .code,
        "budget"
    );
    assert_eq!(canonical(&plane).unwrap(), before);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn procedural_groups_fields_direct_parity_bake_recovery_and_stale_edits() {
    use render_core::{document::*, procedural::*, render::Evaluator};
    let mut d = feature_fixtures::modeling_document().unwrap();
    let before = canonical(&d).unwrap();
    let key = d.snapshot().procedural_bindings[&Id(8720)].clone();
    let graph = d.snapshot().procedural_assets[&key].clone();
    let evaluated = graph.evaluate(&d.snapshot().meshes, || false).unwrap();
    let cube = box_mesh([-0.3; 3], [0.3; 3]).unwrap();
    let (bevel, _) = apply(
        &cube,
        Operation::BevelBox {
            distance_meters: 0.06,
        },
    );
    let (mut direct, _) = apply(
        &bevel,
        Operation::Array {
            count: 3,
            step: [0.8, 0., 0.],
        },
    );
    for i in 0..direct.positions.len() {
        let p = direct.positions.get(i);
        direct
            .positions
            .set(i, p + glam::DVec3::new(-0.8, 0.2, 0.))
            .unwrap();
    }
    assert_eq!(
        direct.content_id().unwrap(),
        evaluated.mesh.content_id().unwrap()
    );
    assert_eq!(evaluated.receipt.operations.len(), 2);
    let mut evaluator = Evaluator::default();
    let scene = evaluator.evaluate(d.snapshot()).unwrap();
    assert_eq!(scene.procedures.len(), 1);
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
    let req = fixtures::request(
        &d,
        "m06:bake:00000001",
        vec![Command::BakeProcedural {
            entity: Id(8720),
            source_graph: key.clone(),
        }],
    )
    .unwrap();
    d.execute(&fixtures::principal(), &req).unwrap();
    assert!(d.snapshot().procedural_bindings.is_empty());
    assert!(d.snapshot().procedural_assets.contains_key(&key));
    assert_eq!(
        d.snapshot()
            .entities
            .get(Id(8720))
            .unwrap()
            .mesh
            .as_ref()
            .unwrap(),
        &direct.content_id().unwrap()
    );
    let current = canonical(&d).unwrap();
    let req = fixtures::request(
        &d,
        "m06:stale:000001",
        vec![Command::ModelMesh {
            entity: Id(8720),
            source_mesh: "stale".into(),
            operation: Operation::Triangulate,
            budget: Budget::default(),
        }],
    )
    .unwrap();
    assert_eq!(
        d.execute(&fixtures::principal(), &req).unwrap_err().code,
        "stale_selection"
    );
    assert_eq!(canonical(&d).unwrap(), current);
    let req = fixtures::request(
        &d,
        "m06:readonly:001",
        vec![Command::ModelMesh {
            entity: Id(8720),
            source_mesh: direct.content_id().unwrap(),
            operation: Operation::Triangulate,
            budget: Budget::default(),
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
    assert_eq!(canonical(&d).unwrap(), current);
    let mut bad = (*graph).clone();
    bad.root.nodes.insert(Id(5), Node::Scalar { value: 1. });
    assert_eq!(
        bad.validate(&d.snapshot().meshes).unwrap_err().code,
        "port_type"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn direct_modeling_transactions_and_field_cycle_domain_resource_diagnostics() {
    use render_core::{document::*, procedural::*};
    use std::collections::BTreeMap;
    let mut d = Document::new(Snapshot::empty(Id(8900))).unwrap();
    let req = fixtures::request(
        &d,
        "model:primitive:1",
        vec![
            Command::CreateEntity {
                entity: Entity {
                    id: Id(8901),
                    name: "direct cube".into(),
                    parent: None,
                    mesh: None,
                    material: None,
                    transform: Transform::default(),
                },
            },
            Command::CreateBox {
                entity: Id(8901),
                min: [-1.; 3],
                max: [1.; 3],
            },
        ],
    )
    .unwrap();
    d.execute(&fixtures::principal(), &req).unwrap();
    let source = d
        .snapshot()
        .entities
        .get(Id(8901))
        .unwrap()
        .mesh
        .clone()
        .unwrap();
    let req = fixtures::request(
        &d,
        "model:extrude:001",
        vec![Command::ModelMesh {
            entity: Id(8901),
            source_mesh: source,
            operation: Operation::ExtrudeFace {
                face: 2,
                distance_meters: 0.5,
            },
            budget: Budget::default(),
        }],
    )
    .unwrap();
    d.execute(&fixtures::principal(), &req).unwrap();
    assert_eq!(d.snapshot().modeling_receipts.len(), 1);
    let recovered = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(canonical(&d).unwrap(), canonical(&recovered).unwrap());
    let graph = Graph {
        root: Group {
            nodes: BTreeMap::from([
                (
                    Id(1),
                    Node::Box {
                        min: [0.; 3],
                        max: [1.; 3],
                    },
                ),
                (
                    Id(2),
                    Node::Position {
                        domain: Domain::Point,
                    },
                ),
                (
                    Id(3),
                    Node::Vector {
                        value: [1., 0., 0.],
                    },
                ),
                (Id(4), Node::AddVector { a: Id(2), b: Id(3) }),
                (
                    Id(5),
                    Node::SetPositions {
                        geometry: Id(1),
                        positions: Id(4),
                    },
                ),
            ]),
            output: Id(5),
        },
        groups: BTreeMap::new(),
        budget: Budget::default(),
    };
    assert_eq!(
        graph.evaluate(&BTreeMap::new(), || true).unwrap_err().code,
        "cancelled"
    );
    let mut bad = graph.clone();
    bad.root
        .nodes
        .insert(Id(2), Node::AddVector { a: Id(4), b: Id(3) });
    let e = bad.validate(&BTreeMap::new()).unwrap_err();
    assert_eq!(e.code, "graph_cycle");
    assert!(e.message.contains(" -> "));
    let mut bad = graph.clone();
    bad.root.nodes.insert(
        Id(2),
        Node::Position {
            domain: Domain::Face,
        },
    );
    assert_eq!(
        bad.validate(&BTreeMap::new()).unwrap_err().code,
        "field_domain"
    );
    let mut bad = graph;
    bad.budget.vertices = 4;
    assert_eq!(
        bad.evaluate(&BTreeMap::new(), || false).unwrap_err().code,
        "budget"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn box_operators_reject_inward_shells() {
    let cube = box_mesh([-1.; 3], [1.; 3]).unwrap();
    let polygons: Vec<Vec<u32>> = cube
        .face_offsets
        .windows(2)
        .map(|w| {
            cube.corners[w[0] as usize..w[1] as usize]
                .iter()
                .rev()
                .map(|c| c.vertex)
                .collect()
        })
        .collect();
    let inward = Mesh::from_polygons(cube.positions.clone(), &polygons, &[]).unwrap();
    assert_eq!(
        render_core::modeling::apply(
            &inward,
            &Operation::BevelBox {
                distance_meters: 0.2
            },
            &Budget::default(),
            || false
        )
        .unwrap_err()
        .code,
        "topology_profile"
    );
}
