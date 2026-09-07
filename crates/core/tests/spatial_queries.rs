use render_core::{
    Id, canonical,
    document::{Document, Principal, Snapshot},
    geometry::{Mesh, Positions},
    geometry_query::*,
};
use std::sync::Arc;
fn setup(mesh: Mesh) -> (Snapshot, Request) {
    let mut snapshot = Snapshot::empty(Id(300));
    let hash = mesh.content_id().unwrap();
    snapshot.meshes.insert(hash.clone(), Arc::new(mesh));
    let request = Request {
        version: 0,
        base_revision: snapshot.revision().unwrap(),
        mesh: hash,
        query: Query::NearestSurface {
            point: [0.25, 0.25, 1.],
            max_distance_meters: 10.,
        },
        budget: Budget::default(),
    };
    (snapshot, request)
}
fn triangle() -> Mesh {
    let mut mesh = Mesh::from_polygons(
        Positions::F64(vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]),
        &[vec![0, 1, 2]],
        &[],
    )
    .unwrap();
    mesh.point_ids = vec![u64::MAX, 9007199254740993, 1];
    mesh.face_ids = vec![u64::MAX - 1];
    mesh.corner_ids = vec![9007199254740997, 9007199254740995, 9007199254740999];
    mesh
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn analytic_surface_features_winding_and_inclusive_distance() {
    let (s, mut q) = setup(triangle());
    let mut cache = Cache::default();
    for (point, position, weights, distance) in [
        ([0.25, 0.25, 1.], [0.25, 0.25, 0.], [0.5, 0.25, 0.25], 1.),
        (
            [1., 1., 1.],
            [0.5, 0.5, 0.],
            [0., 0.5, 0.5],
            libm::sqrt(1.5),
        ),
        ([-1., -1., 1.], [0., 0., 0.], [1., 0., 0.], libm::sqrt(3.)),
        ([0.25, 0.25, -1.], [0.25, 0.25, 0.], [0.5, 0.25, 0.25], 1.),
    ] {
        q.query = Query::NearestSurface {
            point,
            max_distance_meters: distance,
        };
        // sqrt(distance2)^2 can round below distance2: use an exact maximum
        // for the inclusive plane case and a larger bound for irrational cases.
        if distance > 1. {
            q.query = Query::NearestSurface {
                point,
                max_distance_meters: 2.,
            };
        }
        let result = cache.query(&s, &q, || false).unwrap();
        let Hits::NearestSurface {
            surface: Some(hit),
            exact_tied_triangles: 1,
        } = result.hits
        else {
            panic!("surface")
        };
        assert_eq!(hit.face, ElementId(u64::MAX - 1));
        assert_eq!(
            hit.corners,
            <[u64; 3]>::try_from(triangle().corner_ids)
                .unwrap()
                .map(ElementId)
        );
        for i in 0..3 {
            near(hit.position_meters[i], position[i]);
            near(hit.barycentric[i], weights[i]);
        }
        assert_eq!(hit.geometric_normal, [0., 0., 1.]);
        near(hit.distance_meters, distance);
    }
    q.query = Query::NearestSurface {
        point: [0.25, 0.25, 1.],
        max_distance_meters: 0.999,
    };
    assert!(matches!(
        cache.query(&s, &q, || false).unwrap().hits,
        Hits::NearestSurface {
            surface: None,
            exact_tied_triangles: 0
        }
    ));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sphere_boundary_loose_points_high_ids_and_empty_surfaces() {
    let mut mesh = triangle();
    mesh.face_offsets = vec![0];
    mesh.face_ids.clear();
    mesh.corners.clear();
    mesh.corner_ids.clear();
    let (s, mut q) = setup(mesh);
    let mut cache = Cache::default();
    q.query = Query::PointsInSphere {
        center: [0., 0., 0.],
        radius_meters: 1.,
    };
    let Hits::PointsInSphere { points } = cache.query(&s, &q, || false).unwrap().hits else {
        panic!()
    };
    assert_eq!(
        points.iter().map(|p| p.point.0).collect::<Vec<_>>(),
        vec![1, 9007199254740993, u64::MAX]
    );
    assert!(
        String::from_utf8(canonical(&points).unwrap())
            .unwrap()
            .contains("\"18446744073709551615\"")
    );
    q.query = Query::PointsInSphere {
        center: [0., 0., 0.],
        radius_meters: 0.,
    };
    let Hits::PointsInSphere { points } = cache.query(&s, &q, || false).unwrap().hits else {
        panic!()
    };
    assert_eq!(points.len(), 1);
    q.query = Query::NearestSurface {
        point: [0., 0., 0.],
        max_distance_meters: 100.,
    };
    assert!(matches!(
        cache.query(&s, &q, || false).unwrap().hits,
        Hits::NearestSurface { surface: None, .. }
    ));
    let empty = Mesh::from_polygons(Positions::F32(vec![]), &[], &[]).unwrap();
    let (s, q) = setup(empty);
    assert!(matches!(
        cache.query(&s, &q, || false).unwrap().hits,
        Hits::NearestSurface { surface: None, .. }
    ));
    for text in ["0", "01", "+1", "18446744073709551616", "-1", ""] {
        assert!(serde_json::from_str::<ElementId>(&format!("\"{text}\"")).is_err());
    }
    assert!(serde_json::from_str::<ElementId>("9007199254740993").is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn admission_cancellation_and_cache_are_atomic() {
    let (s, q) = setup(triangle());
    let before = canonical(&s).unwrap();
    let mut cache = Cache::default();
    let mut calls = 0;
    cache
        .query(&s, &q, || {
            calls += 1;
            false
        })
        .unwrap();
    for stop in 1..=calls {
        let mut cold = Cache::default();
        let mut count = 0;
        assert_eq!(
            cold.query(&s, &q, || {
                count += 1;
                count == stop
            })
            .unwrap_err()
            .code,
            "cancelled"
        );
        assert!(!cold.query(&s, &q, || false).unwrap().cost.cache_reused);
    }
    let mut stale = q.clone();
    stale.base_revision = "stale".into();
    assert_eq!(
        cache.query(&s, &stale, || false).unwrap_err().code,
        "stale_revision"
    );
    let mut bad = q.clone();
    bad.query = Query::PointsInSphere {
        center: [f64::NAN, 0., 0.],
        radius_meters: 1.,
    };
    assert_eq!(
        cache.query(&s, &bad, || false).unwrap_err().code,
        "query_metric"
    );
    for field in 0..6 {
        let mut bad = q.clone();
        bad.query = Query::PointsInSphere {
            center: [0., 0., 0.],
            radius_meters: 10.,
        };
        match field {
            0 => bad.budget.max_source_bytes = 1,
            1 => bad.budget.max_build_bytes = 1,
            2 => bad.budget.max_visited_nodes = 0,
            3 => bad.budget.max_item_tests = 1,
            4 => bad.budget.max_hits = 1,
            _ => bad.budget.max_result_bytes = 1,
        }
        assert_eq!(cache.query(&s, &bad, || false).unwrap_err().code, "budget");
    }
    let warm = cache.query(&s, &q, || false).unwrap();
    assert!(warm.cost.cache_reused);
    assert!(!warm.cost.additional_source_identity_validation);
    assert_eq!(warm.cost.built_triangles, 0);
    let reopened: Snapshot = serde_json::from_slice(&before).unwrap();
    let verified = cache.query(&reopened, &q, || false).unwrap();
    assert!(verified.cost.cache_reused);
    assert!(verified.cost.additional_source_identity_validation);
    let mut corrupted = s.clone();
    let mut mesh = triangle();
    mesh.positions
        .set(0, glam::DVec3::new(0., 0., 0.1))
        .unwrap();
    corrupted.meshes.insert(q.mesh.clone(), Arc::new(mesh));
    let mut changed = q.clone();
    changed.base_revision = render_core::digest(&canonical(&corrupted).unwrap());
    assert_eq!(
        cache
            .query(&corrupted, &changed, || false)
            .unwrap_err()
            .code,
        "integrity"
    );
    assert!(cache.query(&s, &q, || false).unwrap().cost.cache_reused);
    assert_eq!(canonical(&s).unwrap(), before);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn read_only_agent_query_preserves_document_and_reuses_index() {
    let (s, q) = setup(triangle());
    let doc = Document::new(s).unwrap();
    let mut session = render_core::agent::Session::new(doc);
    let reader = Principal {
        id: "reader".into(),
        can_write: false,
    };
    let inspect = render_core::agent::Request {
        version: 0,
        operation: render_core::agent::Operation::Inspect,
    };
    let before = session.dispatch(&reader, inspect.clone()).unwrap();
    for reused in [false, true] {
        let result = session
            .dispatch(
                &reader,
                render_core::agent::Request {
                    version: 0,
                    operation: render_core::agent::Operation::QueryGeometry { request: q.clone() },
                },
            )
            .unwrap();
        assert_eq!(result["cost"]["cache_reused"], reused);
    }
    assert_eq!(before, session.dispatch(&reader, inspect).unwrap());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn grid_queries_match_independent_plane_and_point_enumeration() {
    let mut positions = vec![];
    let mut faces = vec![];
    for y in 0..17 {
        for x in 0..17 {
            positions.push([x as f64, y as f64, 0.]);
        }
    }
    for y in 0..16 {
        for x in 0..16 {
            let a = y * 17 + x;
            faces.extend([vec![a, a + 1, a + 18], vec![a, a + 18, a + 17]]);
        }
    }
    let mut mesh = Mesh::from_polygons(Positions::F64(positions.clone()), &faces, &[]).unwrap();
    mesh.point_ids.reverse();
    mesh.face_ids.reverse();
    let (s, mut q) = setup(mesh.clone());
    let mut cache = Cache::default();
    for i in 0..45 {
        let point = [
            -2. + i as f64 * 0.43,
            (i * 37 % 113) as f64 * 0.17,
            1. + (i % 3) as f64,
        ];
        q.query = Query::NearestSurface {
            point,
            max_distance_meters: 100.,
        };
        let response = cache.query(&s, &q, || false).unwrap();
        let Hits::NearestSurface {
            surface: Some(hit), ..
        } = response.hits
        else {
            panic!()
        };
        let expected = [point[0].clamp(0., 16.), point[1].clamp(0., 16.), 0.];
        for (a, b) in hit.position_meters.into_iter().zip(expected) {
            near(a, b);
        }
        near(
            hit.distance_meters,
            libm::sqrt(
                point
                    .into_iter()
                    .zip(expected)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum(),
            ),
        );
        near(hit.barycentric.iter().sum(), 1.);
        assert!(hit.barycentric.iter().all(|v| *v >= 0.));
        let radius = 2.5;
        q.query = Query::PointsInSphere {
            center: point,
            radius_meters: radius,
        };
        let response = cache.query(&s, &q, || false).unwrap();
        let Hits::PointsInSphere { points } = response.hits else {
            panic!()
        };
        let mut expected: Vec<_> = positions
            .iter()
            .zip(&mesh.point_ids)
            .filter(|(p, _)| {
                p.iter()
                    .zip(point)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    <= radius * radius
            })
            .map(|(_, id)| *id)
            .collect();
        expected.sort();
        assert_eq!(
            points.iter().map(|p| p.point.0).collect::<Vec<_>>(),
            expected
        );
        assert!(response.cost.item_tests < positions.len() as u64);
    }
    // The diagonal has two exact closest triangles; authored IDs decide the hit.
    q.query = Query::NearestSurface {
        point: [0.5, 0.5, 1.],
        max_distance_meters: 1.,
    };
    let Hits::NearestSurface {
        surface: Some(hit),
        exact_tied_triangles,
    } = cache.query(&s, &q, || false).unwrap().hits
    else {
        panic!()
    };
    assert_eq!(exact_tied_triangles, 2);
    assert_eq!(hit.face.0, 511);
    let mut limited = q.clone();
    limited.budget.max_visited_nodes = 1;
    assert_eq!(
        cache.query(&s, &limited, || false).unwrap_err().code,
        "budget"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn disconnected_nonmanifold_ties_and_invalid_sources() {
    let mesh = Mesh::from_polygons(
        Positions::F64(vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., -1., 0.],
            [0., 0., 1.],
            [10., 0., 0.],
            [11., 0., 0.],
            [10., 1., 0.],
        ]),
        &[vec![0, 1, 2], vec![1, 0, 3], vec![0, 1, 4], vec![5, 6, 7]],
        &[],
    )
    .unwrap();
    let (s, mut q) = setup(mesh);
    q.query = Query::NearestSurface {
        point: [0.5, 0., 0.],
        max_distance_meters: 0.,
    };
    let Hits::NearestSurface {
        surface: Some(hit),
        exact_tied_triangles,
    } = Cache::default().query(&s, &q, || false).unwrap().hits
    else {
        panic!()
    };
    assert_eq!(hit.face.0, 1);
    assert_eq!(exact_tied_triangles, 3);
    for mesh in [
        Mesh::from_polygons(
            Positions::F64(vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]]),
            &[vec![0, 1, 2]],
            &[],
        )
        .unwrap(),
        Mesh::from_polygons(
            Positions::F64(vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 1.], [0., 1., 0.]]),
            &[vec![0, 1, 2, 3]],
            &[],
        )
        .unwrap(),
    ] {
        let (s, q) = setup(mesh);
        assert!(Cache::default().query(&s, &q, || false).is_err());
    }
    let (s, mut q) = setup(triangle());
    for (point, distance) in [
        ([0., 0., 0.], -1.),
        ([f64::INFINITY, 0., 0.], 1.),
        ([1e9 + 1., 0., 0.], 1.),
        ([0., 0., 0.], f64::NAN),
    ] {
        q.query = Query::NearestSurface {
            point,
            max_distance_meters: distance,
        };
        assert_eq!(
            Cache::default().query(&s, &q, || false).unwrap_err().code,
            "query_metric"
        );
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn aggregate_polygon_work_is_bounded_on_cold_and_warm_queries() {
    let (s, mut q) = setup(triangle());
    let mut cache = Cache::default();
    let response = cache.query(&s, &q, || false).unwrap();
    assert_eq!(response.cost.triangulation_work_charge, 27);
    q.budget.max_triangulation_work = 26;
    for mut candidate in [Cache::default(), cache.clone()] {
        assert_eq!(
            candidate.query(&s, &q, || false).unwrap_err().code,
            "budget"
        );
    }
    q.budget.max_triangulation_work = 27;
    assert!(cache.query(&s, &q, || false).unwrap().cost.cache_reused);
    let mut later = s.clone();
    later.document_id = Id(301);
    q.base_revision = later.revision().unwrap();
    assert!(cache.query(&later, &q, || false).unwrap().cost.cache_reused);
    // Both faces meet the individual 256-corner cap, but their combined cubic
    // ear-clipping charge must reject before entering the triangulator.
    let positions = (0..256)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / 256.;
            [libm::cos(angle), libm::sin(angle), 0.]
        })
        .collect();
    let ring: Vec<u32> = (0..256).collect();
    let mesh = Mesh::from_polygons(Positions::F64(positions), &[ring.clone(), ring], &[]).unwrap();
    let (s, q) = setup(mesh);
    let error = cache.query(&s, &q, || false).unwrap_err();
    assert_eq!(error.code, "budget");
    assert!(error.message.contains("triangulation work"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn scaled_triangle_frame_preserves_tiny_and_large_analytic_features() {
    for scale in [1.4e-81, 1e-40, 1e-4, 1., 1e8] {
        let mesh = Mesh::from_polygons(
            Positions::F64(vec![[0., 0., 0.], [scale, 0., 0.], [0., scale, 0.]]),
            &[vec![0, 1, 2]],
            &[],
        )
        .unwrap();
        let (s, mut q) = setup(mesh);
        let mut cache = Cache::default();
        for (p, expected, bary, distance) in [
            ([0.25, 0.25, 1.], [0.25, 0.25, 0.], [0.5, 0.25, 0.25], 1.),
            (
                [1., 1., 1.],
                [0.5, 0.5, 0.],
                [0., 0.5, 0.5],
                libm::sqrt(1.5),
            ),
            ([-1., -1., 1.], [0., 0., 0.], [1., 0., 0.], libm::sqrt(3.)),
        ] {
            q.query = Query::NearestSurface {
                point: p.map(|v| v * scale),
                max_distance_meters: 2. * scale,
            };
            let Hits::NearestSurface {
                surface: Some(hit), ..
            } = cache.query(&s, &q, || false).unwrap().hits
            else {
                panic!("scaled hit {scale}")
            };
            for i in 0..3 {
                near(hit.position_meters[i] / scale, expected[i]);
                near(hit.barycentric[i], bary[i]);
            }
            near(hit.distance_meters / scale, distance);
            assert_eq!(hit.geometric_normal, [0., 0., 1.]);
        }
    }
    let mesh = Mesh::from_polygons(
        Positions::F64(vec![[11., -7., 4.], [12., -7., 5.], [11., -6., 4.]]),
        &[vec![0, 1, 2]],
        &[],
    )
    .unwrap();
    let (s, mut q) = setup(mesh);
    let normal = glam::DVec3::new(-1., 0., 1.) / libm::sqrt(2.);
    let projected = glam::DVec3::new(11.25, -6.75, 4.25);
    q.query = Query::NearestSurface {
        point: (projected + normal * 2.).to_array(),
        max_distance_meters: 3.,
    };
    let Hits::NearestSurface {
        surface: Some(hit), ..
    } = Cache::default().query(&s, &q, || false).unwrap().hits
    else {
        panic!()
    };
    for i in 0..3 {
        near(hit.position_meters[i], projected[i]);
        near(hit.geometric_normal[i], normal[i]);
        near(hit.barycentric[i], [0.5, 0.25, 0.25][i]);
    }
    near(hit.distance_meters, 2.);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn subnormal_squared_distances_do_not_collapse_zero_radius_boundaries() {
    let (s, mut q) = setup(triangle());
    let mut cache = Cache::default();
    for epsilon in [1e-160, 1e-200, 1e-300] {
        q.query = Query::PointsInSphere {
            center: [0., 0., epsilon],
            radius_meters: 0.,
        };
        let Hits::PointsInSphere { points } = cache.query(&s, &q, || false).unwrap().hits else {
            panic!()
        };
        assert!(points.is_empty());
        q.query = Query::PointsInSphere {
            center: [0., 0., epsilon],
            radius_meters: epsilon,
        };
        let Hits::PointsInSphere { points } = cache.query(&s, &q, || false).unwrap().hits else {
            panic!()
        };
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].distance_meters, epsilon);
        for (maximum, hit_expected) in [(0., false), (epsilon / 2., false), (epsilon, true)] {
            q.query = Query::NearestSurface {
                point: [0.25, 0.25, epsilon],
                max_distance_meters: maximum,
            };
            let Hits::NearestSurface { surface, .. } = cache.query(&s, &q, || false).unwrap().hits
            else {
                panic!()
            };
            assert_eq!(surface.is_some(), hit_expected);
            if let Some(hit) = surface {
                assert_eq!(hit.distance_meters, epsilon);
                assert_eq!(hit.barycentric, [0.5, 0.25, 0.25]);
            }
        }
    }
    q.query = Query::PointsInSphere {
        center: [0., 0., 0.],
        radius_meters: -0.0,
    };
    let Hits::PointsInSphere { points } = cache.query(&s, &q, || false).unwrap().hits else {
        panic!()
    };
    assert_eq!(points.len(), 1);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unresolved_conditioned_frame_rejects_without_replacing_cached_index() {
    let (good, request) = setup(triangle());
    let mut cache = Cache::default();
    cache.query(&good, &request, || false).unwrap();
    let (x, y, u, v) = (
        4293147.105974088,
        38042555.927906096,
        7513355.533717836,
        66577557.4519595,
    );
    let mut mesh = Mesh::from_polygons(
        Positions::F64(vec![[0., 0., 0.], [-y, x, 0.], [x, y, 0.], [u, v, 0.]]),
        &[vec![0, 1, 2, 3]],
        &[],
    )
    .unwrap();
    mesh.corner_ids = vec![4, 3, 2, 1];
    assert_eq!(mesh.triangles().unwrap().len(), 2);
    let (s, mut q) = setup(mesh);
    let before = canonical(&s).unwrap();
    for query in [
        Query::NearestSurface {
            point: [0., 0., 1.],
            max_distance_meters: 2.,
        },
        Query::PointsInSphere {
            center: [0., 0., 0.],
            radius_meters: 1.,
        },
    ] {
        q.query = query;
        let error = cache.query(&s, &q, || false).unwrap_err();
        assert_eq!(error.code, "degenerate");
        assert!(error.message.contains("normalized query frame"));
        assert!(
            cache
                .query(&good, &request, || false)
                .unwrap()
                .cost
                .cache_reused
        );
    }
    assert_eq!(canonical(&s).unwrap(), before);
}
