use glam::{DVec3, Vec2};
use render_core::{document::*, geometry::*, render::*, storage::*, topology::*, *};
use std::sync::Arc;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn identity_and_rational_time() {
    let id = Id(u128::MAX);
    assert_eq!(
        serde_json::from_slice::<Id>(&canonical(&id).unwrap()).unwrap(),
        id
    );
    assert!(serde_json::from_str::<Id>("\"01\"").is_err());
    assert_eq!(
        Time::frame(30000, 30000, 1001).unwrap(),
        Time::new(1001, 1).unwrap()
    );
    assert!(Time::new(1, 0).is_err());
    assert!(Time::frame(i64::MAX, 24, u64::MAX).is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn canonical_order_independent_of_table_layout() {
    let mut a = Snapshot::empty(Id(1));
    let mut b = a.clone();
    for n in 1..10 {
        a.entities
            .insert(Entity {
                id: Id(n),
                name: "x".into(),
                parent: None,
                mesh: None,
                material: None,
                transform: Transform::default(),
            })
            .unwrap();
    }
    for e in a
        .entities
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        b.entities.insert(e).unwrap();
    }
    assert_eq!(a.revision().unwrap(), b.revision().unwrap());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn mesh_preserves_seams_and_loose_elements() {
    let m = fixtures::seam_mesh().unwrap();
    m.validate().unwrap();
    let json = canonical(&m).unwrap();
    assert_eq!(m, serde_json::from_slice::<Mesh>(&json).unwrap());
    assert_eq!(m.positions.len(), 8);
    assert_eq!(m.triangles().unwrap().len(), 6);
    assert_ne!(m.uv(0), m.uv(4));
    assert!(m.edges.contains(&[5, 6]));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn malformed_topology_rejected() {
    let original = fixtures::seam_mesh().unwrap();
    let mut m = original.clone();
    m.corners[0].vertex = 100;
    assert!(m.validate().is_err());
    let mut m = original.clone();
    m.face_offsets[1] = u32::MAX;
    assert!(m.validate().is_err());
    let mut m = original.clone();
    m.point_ids[0] = m.point_ids[1];
    assert!(m.validate().is_err());
    let mut m = original.clone();
    m.attributes.get_mut("UVMap").unwrap().values = AttributeValues::Vec2(vec![]);
    assert!(m.validate().is_err());
    let mut m = original;
    m.positions = Positions::F32(vec![[f32::NAN; 3]; 8]);
    assert!(m.validate().is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn concave_polygon_area_and_degeneracy() {
    let m = Mesh::from_polygons(
        Positions::F64(vec![
            [0., 0., 0.],
            [2., 0., 0.],
            [2., 2., 0.],
            [1., 1., 0.],
            [0., 2., 0.],
        ]),
        &[vec![0, 1, 2, 3, 4]],
        &[],
    )
    .unwrap();
    let tris = m.triangles().unwrap();
    assert_eq!(tris.len(), 3);
    let area: f64 = tris
        .iter()
        .map(|t| {
            let p = t.map(|c| m.positions.get(m.corners[c as usize].vertex as usize));
            (p[1] - p[0]).cross(p[2] - p[0]).length() / 2.
        })
        .sum();
    assert!((area - 3.).abs() < 1e-12);
    let d = Mesh::from_polygons(
        Positions::F32(vec![[0.; 3], [1., 0., 0.], [2., 0., 0.]]),
        &[vec![0, 1, 2]],
        &[],
    )
    .unwrap();
    assert_eq!(d.triangles().unwrap_err().code, "degenerate");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn radial_nonmanifold_and_generation_safety() {
    let m = fixtures::nonmanifold().unwrap();
    let mut e = EditMesh::new(&m).unwrap();
    assert!(e.radial.iter().any(|r| r.len() == 3));
    assert!(!e.manifold_candidate_accepts());
    e.move_vertex(e.vertex(2).unwrap(), DVec3::new(0., 2., 0.))
        .unwrap();
    let r = e.commit().unwrap();
    assert_eq!(r.changed_points, vec![3]);
    assert_eq!(r.preserved_points, m.point_ids);
    let mut arena = Arena::new();
    let h = arena.insert(3);
    assert_eq!(arena.remove(h).unwrap(), 3);
    let new = arena.insert(4);
    assert!(arena.get(h).is_err());
    assert_ne!(h, new);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn snapshots_copy_only_changed_entity_chunk() {
    let mut table = EntityTable::default();
    for n in 0..130 {
        table
            .insert(Entity {
                id: Id(n),
                name: "x".into(),
                parent: None,
                mesh: None,
                material: None,
                transform: Transform::default(),
            })
            .unwrap();
    }
    let mut branch = table.clone();
    branch.get_mut(Id(70)).unwrap().name = "changed".into();
    assert_eq!(table.chunk_count(), 3);
    assert_eq!(table.shared_chunks(&branch), 2);
    assert_eq!(table.get(Id(70)).unwrap().name, "x");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn negative_shear_parent_and_layer_semantics() {
    let mut d = fixtures::demo().unwrap();
    let mut transform = Transform::default();
    transform.columns[0][0] = -1.;
    transform.columns[1][0] = 0.5;
    let r = fixtures::request(
        &d,
        "transform:negative",
        vec![
            Command::SetTransform {
                entity: Id(4),
                transform,
            },
            Command::Reparent {
                entity: Id(5),
                parent: Some(Id(4)),
            },
        ],
    )
    .unwrap();
    d.execute(&fixtures::principal(), &r).unwrap();
    assert!(
        d.snapshot()
            .world_transform(Id(5))
            .unwrap()
            .matrix3
            .determinant()
            < 0.
    );
    let before = d.snapshot().revision().unwrap();
    let r = fixtures::request(
        &d,
        "transform:cycle01",
        vec![Command::Reparent {
            entity: Id(4),
            parent: Some(Id(5)),
        }],
    )
    .unwrap();
    assert_eq!(
        d.execute(&fixtures::principal(), &r).unwrap_err().code,
        "cycle"
    );
    assert_eq!(d.snapshot().revision().unwrap(), before);
    let mut singular = Transform::default();
    singular.columns[0] = [0.; 3];
    assert_eq!(singular.inverse().unwrap_err().code, "singular");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn transactions_are_atomic_and_revision_checked() {
    let mut d = fixtures::demo().unwrap();
    let before = d.snapshot().revision().unwrap();
    let r = fixtures::request(
        &d,
        "transaction:bad01",
        vec![
            Command::Rename {
                entity: Id(4),
                name: "bad".into(),
            },
            Command::SetMaterialScalar {
                material: Id(2),
                parameter: ScalarParameter::Roughness,
                value: 1.5,
            },
        ],
    )
    .unwrap();
    assert!(d.execute(&fixtures::principal(), &r).is_err());
    assert_eq!(before, d.snapshot().revision().unwrap());
    let r = fixtures::request(
        &d,
        "transaction:good1",
        vec![Command::Rename {
            entity: Id(4),
            name: "new".into(),
        }],
    )
    .unwrap();
    let c = d.prepare(&fixtures::principal(), &r).unwrap();
    let c2 = d
        .prepare(
            &fixtures::principal(),
            &Request {
                idempotency_key: "transaction:good2".into(),
                ..r.clone()
            },
        )
        .unwrap();
    d.commit(c).unwrap();
    assert_eq!(d.commit(c2).unwrap_err().code, "conflict");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn retries_wire_permissions_and_budgets() {
    let mut a = fixtures::demo().unwrap();
    let mut b = a.clone();
    let r = fixtures::request(
        &a,
        "wire:request:0001",
        vec![Command::Rename {
            entity: Id(4),
            name: "wire".into(),
        }],
    )
    .unwrap();
    let receipt = a.execute(&fixtures::principal(), &r).unwrap();
    let wire = b
        .execute_wire(&fixtures::principal(), &canonical(&r).unwrap())
        .unwrap();
    assert_eq!(receipt, serde_json::from_slice::<Receipt>(&wire).unwrap());
    assert_eq!(receipt, a.execute(&fixtures::principal(), &r).unwrap());
    let mut mismatch = r.clone();
    mismatch.max_added_bytes += 1;
    assert_eq!(
        a.execute(&fixtures::principal(), &mismatch)
            .unwrap_err()
            .code,
        "idempotency_mismatch"
    );
    assert_eq!(
        a.execute(
            &Principal {
                id: "user".into(),
                can_write: false
            },
            &r
        )
        .unwrap_err()
        .code,
        "permission"
    );
    let mut limited = fixtures::request(
        &a,
        "budget:request:01",
        vec![Command::Rename {
            entity: Id(4),
            name: "x".repeat(100),
        }],
    )
    .unwrap();
    limited.max_added_bytes = 0;
    assert_eq!(
        a.execute(&fixtures::principal(), &limited)
            .unwrap_err()
            .code,
        "budget"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn durable_failure_quota_recovery_and_idempotency() {
    let mut d = fixtures::demo().unwrap();
    let before = d.snapshot().revision().unwrap();
    let r = fixtures::request(
        &d,
        "storage:request01",
        vec![Command::Rename {
            entity: Id(4),
            name: "saved".into(),
        }],
    )
    .unwrap();
    let mut store = MemoryStore {
        quota: 1,
        ..Default::default()
    };
    assert_eq!(
        durable_execute(&mut store, &mut d, &fixtures::principal(), &r)
            .unwrap_err()
            .code,
        "quota"
    );
    assert_eq!(before, d.snapshot().revision().unwrap());
    store.quota = 0;
    store.fail_publication = true;
    assert!(durable_execute(&mut store, &mut d, &fixtures::principal(), &r).is_err());
    store.fail_publication = false;
    let receipt = durable_execute(&mut store, &mut d, &fixtures::principal(), &r).unwrap();
    assert!(receipt.durable);
    let mut recovered = recover(&store.records).unwrap().unwrap();
    assert_eq!(
        recovered.execute(&fixtures::principal(), &r).unwrap(),
        receipt
    );
    store.records[0].payload[0] ^= 1;
    assert!(recover(&store.records).is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn journal_sequence_is_checked() {
    let d = fixtures::demo().unwrap();
    let e = Envelope::new(1, None, &d).unwrap();
    assert!(recover(&[e]).is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn renderer_reuses_geometry_after_transform() {
    let mut d = fixtures::demo().unwrap();
    let mut eval = Evaluator::default();
    let a = eval.evaluate(d.snapshot()).unwrap();
    let r = fixtures::request(
        &d,
        "eval:transform:01",
        vec![Command::SetTransform {
            entity: Id(4),
            transform: Transform::translation(-2., 0., 0.),
        }],
    )
    .unwrap();
    d.execute(&fixtures::principal(), &r).unwrap();
    let b = eval.evaluate(d.snapshot()).unwrap();
    assert_eq!(a.geometry_builds, 2);
    assert_eq!(b.geometry_builds, 0);
    assert!(Arc::ptr_eq(
        &a.instances[0].geometry,
        &b.instances[0].geometry
    ));
    assert!(Arc::ptr_eq(
        &a.instances[0].geometry,
        &a.instances[1].geometry
    ));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn triangle_bvh_and_parallel_slab_analytic() {
    let t = Triangle {
        uv_sets: vec![],
        normals: None,
        tangents: None,
        positions: [
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(0., 1., 0.),
        ],
        uv: [Vec2::ZERO; 3],
    };
    let ray = Ray {
        origin: DVec3::new(0.25, 0.25, 2.),
        direction: -DVec3::Z,
    };
    assert_eq!(t.intersect(ray, 0., 100.), Some((2., 0.25, 0.25)));
    assert!(t.bounds().hit(ray, 0., 100.));
    let parallel = Ray {
        origin: DVec3::new(2., 0., 0.),
        direction: DVec3::Y,
    };
    assert!(!t.bounds().hit(parallel, 0., 100.));
    let b = Bvh::build(&[t.bounds()]);
    assert_eq!(b.candidates(ray, 0., 100.), vec![0]);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn diffuse_energy_pdf_and_sample_mean() {
    let mut mean = DVec3::ZERO;
    for sample in 0..20000 {
        let d = cosine_direction(DVec3::Z, random(0, sample, 0, 42), random(0, sample, 1, 42));
        assert!((d.length() - 1.).abs() < 1e-12);
        assert!(d.z >= 0.);
        let pdf = diffuse_pdf(d.z);
        assert!(pdf > 0.);
        let weight = (0.7 / std::f64::consts::PI) * d.z / pdf;
        assert!((weight - 0.7).abs() < 1e-12);
        mean += d;
    }
    mean /= 20000.;
    assert!((mean.z - 2. / 3.).abs() < 0.015);
    assert!(mean.x.abs() < 0.015 && mean.y.abs() < 0.015);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn empty_scene_environment_is_analytic() {
    let mut eval = Evaluator::default();
    let scene = eval.evaluate(&Snapshot::empty(Id(1))).unwrap();
    let mut s = fixtures::settings();
    s.width = 8;
    s.height = 4;
    let image = render(&scene, &s, || false).unwrap();
    for p in image.linear {
        for (a, b) in p.iter().zip(&s.environment) {
            assert!((a - b).abs() < 1e-6);
        }
    }
    assert_eq!(render(&scene, &s, || true).unwrap_err().code, "cancelled");
    s.max_bytes = 1;
    assert_eq!(render(&scene, &s, || false).unwrap_err().code, "budget");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn diffuse_plane_radiance_matches_analytic_solution() {
    let (scene, settings, expected) = fixtures::diffuse_plane().unwrap();
    let image = render(&scene, &settings, || false).unwrap();
    assert!(image.objects.iter().all(Option::is_some));
    for pixel in image.linear {
        for (actual, expected) in pixel.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-6);
        }
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unsupported_scattering_is_diagnosed() {
    for parameter in [ScalarParameter::Metallic, ScalarParameter::Roughness] {
        let mut doc = fixtures::demo().unwrap();
        let request = fixtures::request(
            &doc,
            "unsupported:material:001",
            vec![Command::SetMaterialScalar {
                material: Id(2),
                parameter,
                value: 0.5,
            }],
        )
        .unwrap();
        doc.execute(&fixtures::principal(), &request).unwrap();
        assert!(
            matches!(Evaluator::default().evaluate(doc.snapshot()), Err(error) if error.code == "unsupported_material")
        );
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn render_reproducibility_and_color_boundaries() {
    assert!((linear_to_srgb(0.0031308) - 0.04044994).abs() < 1e-6);
    assert_eq!(linear_to_srgb(0.), 0.);
    let d = fixtures::demo().unwrap();
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let mut s = fixtures::settings();
    s.width = 12;
    s.height = 8;
    s.samples = 2;
    let a = render(&scene, &s, || false).unwrap();
    let b = render(&scene, &s, || false).unwrap();
    assert_eq!(a.linear, b.linear);
    assert!(a.objects.iter().any(Option::is_some));
    assert!(a.ppm().starts_with(b"P6\n"));
    assert!(a.pfm().starts_with(b"PF\n"));
    assert!(a.linear.iter().flatten().all(|v| v.is_finite() && *v >= 0.));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn obj_and_gltf_profiles_roundtrip() {
    let m = fixtures::seam_mesh().unwrap();
    let (obj, loss) = interchange::export_obj(&m).unwrap();
    assert!(!loss.losses.is_empty());
    let (n, _) = interchange::import_obj(&obj).unwrap();
    assert_eq!(n.face_offsets, m.face_offsets);
    assert_eq!(n.corners, m.corners);
    for c in 0..m.corners.len() {
        assert_eq!(n.uv(c), m.uv(c));
    }
    let (json, bin, _) = interchange::export_gltf(&m).unwrap();
    let (g, _) = interchange::import_gltf(&json, &bin).unwrap();
    assert_eq!(g.triangles().unwrap().len(), m.triangles().unwrap().len());
    assert!(interchange::import_gltf(&json, &bin[..4]).is_err());
    assert!(interchange::import_obj("v 0 0 0\nf 1 2 3").is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn jobs_events_cancellation_and_recovery() {
    use jobs::*;
    let mut queue = Jobs::default();
    let job = Job {
        id: Id(1),
        principal: "p".into(),
        revision: "r".into(),
        state: State::Accepted,
        budget: Budget {
            max_wall_ms: 1000,
            max_bytes: 1000,
            max_samples: 1,
            max_output_bytes: 1000,
        },
        artifacts: vec![],
        diagnostic: None,
        render_input: None,
    };
    queue.submit(job.clone()).unwrap();
    assert_eq!(queue.start_next(), Some(Id(1)));
    assert!(queue.cancel("other", Id(1)).is_err());
    assert_eq!(queue.cancel("p", Id(1)).unwrap(), State::CancelRequested);
    assert_eq!(
        queue.finish(Id(1), Ok(vec!["discarded".into()])).unwrap(),
        State::Cancelled
    );
    assert!(queue.jobs[&Id(1)].artifacts.is_empty());
    assert!(
        queue
            .events_after(Some(1))
            .unwrap()
            .iter()
            .all(|e| e.sequence > 1)
    );
    let mut next = job;
    next.id = Id(2);
    queue.submit(next).unwrap();
    queue.start_next();
    let bytes = canonical(&queue).unwrap();
    let mut restored: Jobs = serde_json::from_slice(&bytes).unwrap();
    restored.recover_interrupted();
    assert_eq!(restored.jobs[&Id(2)].state, State::Failed);
    queue.event_retention = 1;
    queue.cancel("p", Id(2)).unwrap();
    assert!(queue.events_after(Some(0)).is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn shared_browser_suite() {
    assert!(fixtures::conformance().unwrap().values().all(|v| *v));
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn native_publication_fault_boundaries() {
    use storage::native::*;
    for (index, fault) in [
        Fault::AfterWrite,
        Fault::AfterFileSync,
        Fault::AfterRename,
        Fault::AfterDirectorySync,
    ]
    .into_iter()
    .enumerate()
    {
        let path =
            std::env::temp_dir().join(format!("render-fault-{}-{index}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        let mut store = NativeStore::open(&path).unwrap();
        let d = fixtures::demo().unwrap();
        let first = Envelope::new(0, None, &d).unwrap();
        store.publish(&first).unwrap();
        let mut next = d.clone();
        let r = fixtures::request(
            &next,
            "fault:request:001",
            vec![Command::Rename {
                entity: Id(4),
                name: "next".into(),
            }],
        )
        .unwrap();
        next.execute(&fixtures::principal(), &r).unwrap();
        let e = Envelope::new(1, Some(first.digest), &next).unwrap();
        store.fault = fault;
        assert!(store.publish(&e).is_err());
        let loaded = store.load().unwrap();
        let actual = recover(&loaded)
            .unwrap()
            .unwrap()
            .snapshot()
            .revision()
            .unwrap();
        let expected = if matches!(fault, Fault::AfterRename | Fault::AfterDirectorySync) {
            next.snapshot().revision().unwrap()
        } else {
            d.snapshot().revision().unwrap()
        };
        assert_eq!(actual, expected);
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn staged_upload_permissions_checksums_and_budget() {
    let mesh = fixtures::seam_mesh().unwrap();
    let bytes = canonical(&mesh).unwrap();
    let mut uploads = uploads::Uploads::new(bytes.len());
    uploads
        .begin("p", Id(1), bytes.len(), digest(&bytes))
        .unwrap();
    assert!(uploads.begin("p", Id(2), 1, "bad".into()).is_err());
    assert!(uploads.append("other", Id(1), 0, &bytes).is_err());
    assert!(uploads.append("p", Id(1), 1, &bytes).is_err());
    let half = bytes.len() / 2;
    uploads.append("p", Id(1), 0, &bytes[..half]).unwrap();
    assert!(uploads.finish_mesh("p", Id(1)).is_err());
    uploads.append("p", Id(1), half, &bytes[half..]).unwrap();
    assert_eq!(uploads.finish_mesh("p", Id(1)).unwrap(), mesh);
    uploads
        .begin("p", Id(3), bytes.len(), "wrong".into())
        .unwrap();
    uploads.append("p", Id(3), 0, &bytes).unwrap();
    assert!(uploads.finish_mesh("p", Id(3)).is_err());
    uploads.cancel("p", Id(3)).unwrap();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn ordered_transform_channels_and_external_texture() {
    let mut t = Transform {
        operations: vec![
            TransformOp::TranslationMeters([1., 0., 0.]),
            TransformOp::Scale([2., 1., 1.]),
        ],
        ..Default::default()
    };
    assert_eq!(
        t.affine().unwrap().transform_point3(DVec3::X),
        DVec3::new(3., 0., 0.)
    );
    t.operations.reverse();
    assert_eq!(
        t.affine().unwrap().transform_point3(DVec3::X),
        DVec3::new(4., 0., 0.)
    );
    let mut ppm = b"P6\n# authored external image fixture\n1 1\n255\n".to_vec();
    ppm.extend([10, 128, 255]);
    let texture = imaging::read_ppm_srgb(&ppm).unwrap();
    assert!((texture.linear_rgb[0][0] - 0.00303527).abs() < 1e-7);
    assert!((texture.linear_rgb[0][1] - 0.2158605).abs() < 1e-6);
    assert_eq!(texture.linear_rgb[0][2], 1.);
    assert!(imaging::read_ppm_srgb(&ppm[..ppm.len() - 1]).is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn durable_host_metadata_cannot_be_overwritten_by_stale_snapshot() {
    let mut d = fixtures::demo().unwrap();
    let mut stale = d.clone();
    let mut store = MemoryStore::default();
    durable_update(&mut store, &mut d, |doc| {
        doc.jobs_mut().queue_limit = 2;
        Ok(())
    })
    .unwrap();
    let r = fixtures::request(
        &stale,
        "stale:metadata:01",
        vec![Command::Rename {
            entity: Id(4),
            name: "stale".into(),
        }],
    )
    .unwrap();
    assert_eq!(
        durable_execute(&mut store, &mut stale, &fixtures::principal(), &r)
            .unwrap_err()
            .code,
        "conflict"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn storage_crash_child() {
    let Some(path) = std::env::var_os("RENDER_TEST_CRASH_DIRECTORY") else {
        return;
    };
    use storage::native::*;
    let stage = std::env::var("RENDER_TEST_CRASH_STAGE").unwrap();
    let mut store = NativeStore::open(path).unwrap();
    let records = store.load().unwrap();
    let mut doc = recover(&records).unwrap().unwrap();
    let r = fixtures::request(
        &doc,
        "process:crash:001",
        vec![Command::Rename {
            entity: Id(4),
            name: "terminated writer".into(),
        }],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &r).unwrap();
    store.set_boundary_hook(move |boundary| {
        if format!("{boundary:?}") == stage {
            std::process::exit(91);
        }
    });
    store
        .publish(
            &Envelope::new(
                records.len() as u64,
                records.last().map(|e| e.digest.clone()),
                &doc,
            )
            .unwrap(),
        )
        .unwrap();
    panic!("requested crash boundary was never reached");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn process_termination_recovers_committed_root() {
    use storage::native::*;
    for (index, stage) in [
        "AfterWrite",
        "AfterFileSync",
        "AfterRename",
        "AfterDirectorySync",
    ]
    .into_iter()
    .enumerate()
    {
        let root = std::env::temp_dir().join(format!(
            "render-process-crash-{}-{index}",
            std::process::id()
        ));
        let mut store = NativeStore::open(&root).unwrap();
        let before = fixtures::demo().unwrap();
        store
            .publish(&Envelope::new(0, None, &before).unwrap())
            .unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "storage_crash_child", "--nocapture"])
            .env("RENDER_TEST_CRASH_DIRECTORY", &root)
            .env("RENDER_TEST_CRASH_STAGE", stage)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(91));
        let records = store.load().unwrap();
        let recovered = recover(&records).unwrap().unwrap();
        assert_eq!(records.len(), if index < 2 { 1 } else { 2 });
        assert_eq!(
            recovered.snapshot().entities.get(Id(4)).unwrap().name,
            if index < 2 {
                "Pyramid A"
            } else {
                "terminated writer"
            }
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn binary_mesh_chunks_preserve_precision_and_reject_truncation() {
    let mut mesh = fixtures::seam_mesh().unwrap();
    for precise in [false, true] {
        if precise {
            mesh.positions = Positions::F64(
                (0..mesh.positions.len())
                    .map(|i| mesh.positions.get(i).to_array())
                    .collect(),
            );
            mesh.positions
                .set(0, DVec3::new(-1.00000000000001, 0., -1.))
                .unwrap();
        }
        let bytes = mesh_codec::encode(&mesh).unwrap();
        assert_eq!(&bytes[..8], b"R3DMESH0");
        assert_eq!(mesh_codec::decode(&bytes).unwrap(), mesh);
        for n in 0..bytes.len() {
            assert!(mesh_codec::decode(&bytes[..n]).is_err());
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn foreign_edit_handle_and_hostile_accessor_are_rejected() {
    let mesh = fixtures::seam_mesh().unwrap();
    let first = EditMesh::new(&mesh).unwrap();
    let mut second = EditMesh::new(&mesh).unwrap();
    assert_eq!(
        second
            .move_vertex(first.vertex(0).unwrap(), DVec3::ZERO)
            .unwrap_err()
            .code,
        "stale_handle"
    );
    let (json, bin, _) = interchange::export_gltf(&mesh).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    value["accessors"][0]["byteOffset"] = serde_json::json!(u64::MAX);
    assert!(interchange::import_gltf(&canonical(&value).unwrap(), &bin).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_mesh_chunks_deduplicate_and_detect_corruption() {
    let root = std::env::temp_dir().join(format!("render-chunk-store-{}", std::process::id()));
    let mut store = storage::native::NativeStore::open(&root).unwrap();
    let mut doc = fixtures::demo().unwrap();
    let first = Envelope::new(0, None, &doc).unwrap();
    store.publish(&first).unwrap();
    let before = std::fs::read_dir(root.join("chunks")).unwrap().count();
    assert_eq!(before, 2);
    let r = fixtures::request(
        &doc,
        "chunk:rename:001",
        vec![Command::Rename {
            entity: Id(4),
            name: "same geometry".into(),
        }],
    )
    .unwrap();
    durable_execute(&mut store, &mut doc, &fixtures::principal(), &r).unwrap();
    assert_eq!(
        std::fs::read_dir(root.join("chunks")).unwrap().count(),
        before
    );
    let chunk = std::fs::read_dir(root.join("chunks"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(chunk, b"corrupt test payload").unwrap();
    assert_eq!(store.load().unwrap_err().code, "integrity");
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn durable_retry_publishes_to_the_requested_store() {
    let mut doc = fixtures::demo().unwrap();
    let request = fixtures::request(
        &doc,
        "retry:publication:001",
        vec![Command::Rename {
            entity: Id(4),
            name: "prepared in memory".into(),
        }],
    )
    .unwrap();
    let memory_receipt = doc.execute(&fixtures::principal(), &request).unwrap();
    assert!(!memory_receipt.durable);
    let mut store = MemoryStore {
        fail_publication: true,
        ..Default::default()
    };
    assert!(durable_execute(&mut store, &mut doc, &fixtures::principal(), &request).is_err());
    assert!(store.records.is_empty());
    assert!(
        !doc.retry(&fixtures::principal(), &request)
            .unwrap()
            .unwrap()
            .durable
    );
    store.fail_publication = false;
    let durable = durable_execute(&mut store, &mut doc, &fixtures::principal(), &request).unwrap();
    assert!(durable.durable);
    assert_eq!(durable.revision, memory_receipt.revision);
    assert_eq!(store.records.len(), 1);
    let recovered = recover(&store.records).unwrap().unwrap();
    assert_eq!(recovered.snapshot().revision().unwrap(), durable.revision);
    durable_execute(&mut store, &mut doc, &fixtures::principal(), &request).unwrap();
    assert_eq!(store.records.len(), 1);
    // A durable receipt from another adapter is not proof this destination has a root.
    let mut destination = MemoryStore::default();
    durable_execute(&mut destination, &mut doc, &fixtures::principal(), &request).unwrap();
    assert_eq!(destination.records.len(), 1);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn later_layer_restores_hidden_geometry() {
    let mut doc = fixtures::demo().unwrap();
    let authored_mesh = doc.snapshot().entities.get(Id(4)).unwrap().mesh.clone();
    let layer = |name: &str, hidden| Layer {
        name: name.into(),
        overrides: [(
            Id(4),
            Override {
                transform: None,
                material: None,
                hidden,
            },
        )]
        .into(),
    };
    let request = fixtures::request(
        &doc,
        "layers:visibility:001",
        vec![
            Command::PutLayer {
                layer: layer("hide", true),
            },
            Command::PutLayer {
                layer: layer("show", false),
            },
        ],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &request).unwrap();
    assert_eq!(
        doc.snapshot()
            .composed()
            .unwrap()
            .entities
            .get(Id(4))
            .unwrap()
            .mesh,
        authored_mesh
    );
    let request = fixtures::request(
        &doc,
        "layers:visibility:002",
        vec![Command::PutLayer {
            layer: layer("show", true),
        }],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &request).unwrap();
    assert!(
        doc.snapshot()
            .composed()
            .unwrap()
            .entities
            .get(Id(4))
            .unwrap()
            .mesh
            .is_none()
    );
    assert_eq!(
        doc.snapshot().entities.get(Id(4)).unwrap().mesh,
        authored_mesh
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn cancelled_jobs_release_queue_capacity_and_retries_preserve_events() {
    use jobs::*;
    let mut queue = Jobs::default();
    queue.queue_limit = 1;
    for id in [Id(1), Id(2), Id(3)] {
        queue
            .submit(Job {
                id,
                principal: "p".into(),
                revision: "r".into(),
                state: State::Accepted,
                budget: Budget {
                    max_wall_ms: 1,
                    max_bytes: 1,
                    max_samples: 1,
                    max_output_bytes: 1,
                },
                artifacts: vec![],
                diagnostic: None,
                render_input: None,
            })
            .unwrap();
        assert_eq!(queue.cancel("p", id).unwrap(), State::Cancelled);
        let cursor = queue.events_after(None).unwrap().last().unwrap().sequence;
        assert_eq!(queue.cancel("p", id).unwrap(), State::Cancelled);
        assert!(queue.events_after(Some(cursor)).unwrap().is_empty());
    }
    assert_eq!(queue.start_next(), None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn prepared_noop_candidates_cannot_overfill_retention() {
    let mut doc = fixtures::demo().unwrap();
    let commands = vec![Command::Rename {
        entity: Id(4),
        name: "Pyramid A".into(),
    }];
    let request = fixtures::request(&doc, "retention:seed:001", commands.clone()).unwrap();
    let receipt = doc.execute(&fixtures::principal(), &request).unwrap();
    // Restore a near-capacity checkpoint without executing O(n^2) historical clones.
    let mut checkpoint = serde_json::to_value(&doc).unwrap();
    let accepted = checkpoint["accepted"].as_object_mut().unwrap();
    for n in accepted.len()..9999 {
        let request =
            fixtures::request(&doc, &format!("historical:key:{n:08}"), commands.clone()).unwrap();
        let scope = String::from_utf8(
            canonical(&(
                fixtures::principal().id,
                doc.snapshot().document_id,
                &request.idempotency_key,
            ))
            .unwrap(),
        )
        .unwrap();
        accepted.insert(
            scope,
            serde_json::to_value(Accepted {
                payload: digest(&canonical(&request).unwrap()),
                receipt: receipt.clone(),
            })
            .unwrap(),
        );
    }
    let mut doc: Document = serde_json::from_value(checkpoint).unwrap();
    let a = doc
        .prepare(
            &fixtures::principal(),
            &fixtures::request(&doc, "retention:race:001", commands.clone()).unwrap(),
        )
        .unwrap();
    let b = doc
        .prepare(
            &fixtures::principal(),
            &fixtures::request(&doc, "retention:race:002", commands).unwrap(),
        )
        .unwrap();
    doc.commit(a.clone()).unwrap();
    assert_eq!(doc.commit(b).unwrap_err().code, "admission");
    assert!(doc.commit(a).is_ok()); // A retry consumes no additional retained key.
}
