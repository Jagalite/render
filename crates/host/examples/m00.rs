//! Reproducible representation experiment; hecs is an unbundled dev dependency.
use render_core::{Id, document::*, fixtures, topology::EditMesh};
use std::{fs, hint::black_box, time::Instant};
fn timed<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let start = Instant::now();
    let value = black_box(f());
    (value, start.elapsed().as_secs_f64() * 1000.)
}
fn main() {
    let count = 10000;
    let repetitions = 20;
    // One shared quad becomes two authored triangles. Measure wide asset-reference
    // replacement separately from the common immutable mesh construction cost.
    let positions = render_core::geometry::Positions::F32(vec![
        [0., 0., 0.],
        [1., 0., 0.],
        [1., 1., 0.],
        [0., 1., 0.],
    ]);
    let original_mesh =
        render_core::geometry::Mesh::from_polygons(positions.clone(), &[vec![0, 1, 2, 3]], &[])
            .unwrap();
    let (replacement_mesh, replacement_build_ms) = timed(|| {
        render_core::geometry::Mesh::from_polygons(positions, &[vec![0, 1, 2], vec![0, 2, 3]], &[])
            .unwrap()
    });
    let original_key = original_mesh.content_id().unwrap();
    let replacement_key = replacement_mesh.content_id().unwrap();
    assert_ne!(original_key, replacement_key);
    let source = (0..count)
        .map(|n| Entity {
            id: Id(n as u128),
            name: format!("entity-{n}"),
            parent: None,
            mesh: Some(original_key.clone()),
            material: None,
            transform: Transform::translation(n as f64, 0., 0.),
        })
        .collect::<Vec<_>>();
    let (table, table_build) = timed(|| EntityTable::try_from(source.clone()).unwrap());
    let (flat, flat_build) = timed(|| source.clone());
    let ((world, _handles), ecs_build) = timed(|| {
        let mut world = hecs::World::new();
        let handles = world
            .spawn_batch(
                source
                    .iter()
                    .map(|e| (e.id, e.name.clone(), e.transform.clone(), e.mesh.clone())),
            )
            .collect::<Vec<_>>();
        (world, handles)
    });
    let mut rows = vec![];
    for _ in 0..repetitions {
        let (mut branch, table_branch) = timed(|| table.clone());
        let (_, table_edit) = timed(|| branch.get_mut(Id(5000)).unwrap().name = "edited".into());
        let (_, table_wide) = timed(|| {
            for n in 0..count {
                branch.get_mut(Id(n as u128)).unwrap().name = "wide".into();
            }
        });
        let (_, table_traverse) = timed(|| {
            table
                .iter()
                .map(|e| e.transform.operations.len())
                .sum::<usize>()
        });
        let (mut copied, flat_branch) = timed(|| flat.clone());
        let (_, flat_edit) = timed(|| copied[5000].name = "edited".into());
        let (_, flat_wide) = timed(|| {
            for e in &mut copied {
                e.name = "wide".into();
            }
        });
        let (_, flat_traverse) = timed(|| {
            flat.iter()
                .map(|e| e.transform.operations.len())
                .sum::<usize>()
        });
        // hecs deliberately has no generic World::clone. Clone registered components,
        // preserving stable authoring IDs independently of hecs runtime handles.
        let ((mut ecs_branch, branch_handles), ecs_clone) = timed(|| {
            let mut branch = hecs::World::new();
            let branch_handles = branch
                .spawn_batch(
                    world
                        .query::<(&Id, &String, &Transform, &Option<String>)>()
                        .iter()
                        .map(|(_, (&id, name, t, mesh))| {
                            (id, name.clone(), t.clone(), mesh.clone())
                        })
                        .collect::<Vec<_>>(),
                )
                .collect::<Vec<_>>();
            (branch, branch_handles)
        });
        let (_, ecs_edit) = timed(|| {
            *ecs_branch.get::<&mut String>(branch_handles[5000]).unwrap() = "edited".into()
        });
        let (_, ecs_wide) = timed(|| {
            for (_, name) in ecs_branch.query_mut::<&mut String>() {
                *name = "wide".into();
            }
        });
        let (_, ecs_traverse) = timed(|| {
            world
                .query::<&Transform>()
                .iter()
                .map(|(_, t)| t.operations.len())
                .sum::<usize>()
        });
        let (_, table_topology_ms) = timed(|| {
            for n in 0..count {
                branch.get_mut(Id(n as u128)).unwrap().mesh = Some(replacement_key.clone());
            }
        });
        let (_, flat_topology_ms) = timed(|| {
            for e in &mut copied {
                e.mesh = Some(replacement_key.clone());
            }
        });
        let (_, ecs_topology_ms) = timed(|| {
            for (_, mesh) in ecs_branch.query_mut::<&mut Option<String>>() {
                *mesh = Some(replacement_key.clone());
            }
        });
        assert!(
            branch
                .iter()
                .all(|e| e.mesh.as_ref() == Some(&replacement_key))
        );
        assert!(
            copied
                .iter()
                .all(|e| e.mesh.as_ref() == Some(&replacement_key))
        );
        assert!(
            ecs_branch
                .query::<&Option<String>>()
                .iter()
                .all(|(_, m)| m.as_ref() == Some(&replacement_key))
        );
        assert!(table.iter().all(|e| e.mesh.as_ref() == Some(&original_key)));
        rows.push(serde_json::json!({"chunked":{"wide_topology_references_ms":table_topology_ms,"branch_ms":table_branch,"edit_ms":table_edit,"wide_ms":table_wide,"traverse_ms":table_traverse},"flat":{"wide_topology_references_ms":flat_topology_ms,"branch_ms":flat_branch,"edit_ms":flat_edit,"wide_ms":flat_wide,"traverse_ms":flat_traverse},"hecs":{"wide_topology_references_ms":ecs_topology_ms,"branch_ms":ecs_clone,"edit_ms":ecs_edit,"wide_ms":ecs_wide,"traverse_ms":ecs_traverse}}));
    }
    let mesh = fixtures::nonmanifold().unwrap();
    let (edit, radial_ms) = timed(|| EditMesh::new(&mesh).unwrap());
    let (_, dense_traverse) = timed(|| {
        for _ in 0..10000 {
            black_box(mesh.corners.iter().filter(|c| c.edge == 0).count());
        }
    });
    let (_, radial_traverse) = timed(|| {
        for _ in 0..10000 {
            black_box(edit.radial[0].len());
        }
    });
    let grid = 100u32;
    let positions = (0..=grid)
        .flat_map(|y| (0..=grid).map(move |x| [x as f32, y as f32, 0.]))
        .collect();
    let polygons = (0..grid)
        .flat_map(|y| {
            (0..grid).map(move |x| {
                let i = y * (grid + 1) + x;
                vec![i, i + 1, i + grid + 2, i + grid + 1]
            })
        })
        .collect::<Vec<_>>();
    let grid_mesh = render_core::geometry::Mesh::from_polygons(
        render_core::geometry::Positions::F32(positions),
        &polygons,
        &[],
    )
    .unwrap();
    let encoded = render_core::mesh_codec::encode(&grid_mesh).unwrap();
    let (_, load_ms) = timed(|| render_core::mesh_codec::decode(&encoded).unwrap());
    let (mut editing, conversion_ms) = timed(|| EditMesh::new(&grid_mesh).unwrap());
    let capacity_bytes = editing.connectivity_capacity_bytes();
    let handle = editing.vertex(5050).unwrap();
    let (_, local_ms) = timed(|| {
        editing
            .move_vertex(handle, grid_mesh.positions.get(0))
            .unwrap()
    });
    // Move back before comparing the two complete wide position edits.
    editing
        .move_vertex(handle, grid_mesh.positions.get(5050))
        .unwrap();
    let (_, wide_ms) = timed(|| {
        for i in 0..grid_mesh.positions.len() {
            editing
                .move_vertex(editing.vertex(i).unwrap(), grid_mesh.positions.get(i) * 2.)
                .unwrap();
        }
    });
    let (committed, commit_ms) = timed(|| editing.commit().unwrap());
    let mut dense = grid_mesh.clone();
    let (_, dense_wide_ms) = timed(|| {
        for i in 0..dense.positions.len() {
            dense.positions.set(i, dense.positions.get(i) * 2.).unwrap();
        }
        dense.validate().unwrap();
    });
    assert_eq!(
        committed.mesh.content_id().unwrap(),
        dense.content_id().unwrap()
    );
    let (triangles, triangulation_ms) = timed(|| committed.mesh.triangles().unwrap());
    let topology = serde_json::json!({"faces":polygons.len(),"corners":grid_mesh.corners.len(),"triangles":triangles.len(),"binary_mesh_bytes":encoded.len(),"binary_load_ms":load_ms,"edit_conversion_ms":conversion_ms,"connectivity_capacity_bytes_excluding_mesh_and_allocator":capacity_bytes,"local_move_ms":local_ms,"wide_move_ms":wide_ms,"edit_commit_validation_ms":commit_ms,"dense_wide_move_and_validation_ms":dense_wide_ms,"evaluated_triangulation_ms":triangulation_ms,"wide_result_equal":true});
    let report = serde_json::json!({"status":"measured","profile":{"entities":count,"repetitions":repetitions,"architecture":std::env::consts::ARCH,"os":std::env::consts::OS,"hecs_version":"0.10.5","optimized":!cfg!(debug_assertions)},"build_ms":{"chunked":table_build,"flat":flat_build,"hecs":ecs_build},"samples":rows,"wide_topology":{"from_faces":1,"to_faces":2,"instances":count,"replacement_asset_build_ms":replacement_build_ms,"geometry_shared":true,"original_branch_unchanged":true,"scope":"asset replacement and reference updates, not a general modeling operator"},"topology_workload":topology,"radial":{"construction_ms":radial_ms,"radial_adjacency_ms":radial_traverse,"dense_scan_ms":dense_traverse,"nonmanifold_faces":3,"manifold_candidate_accepts":edit.manifold_candidate_accepts()},"limitations":["wall-clock measurements; no allocator instrumentation","hecs component cloning reconstructs runtime handles; persistent IDs are preserved","small nonmanifold topology fixture; not the five-million-corner stress gate"]});
    fs::create_dir_all("artifacts/evidence").unwrap();
    fs::write(
        "artifacts/evidence/representation_comparison.json",
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("representation comparison written: {count} entities, {repetitions} repetitions");
}
