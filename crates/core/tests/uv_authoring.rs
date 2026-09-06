use render_core::{Id, canonical, geometry::*, modeling, uv::*};
fn quad() -> Mesh {
    Mesh::from_polygons(
        Positions::F64(vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]),
        &[vec![0, 1, 2, 3]],
        &[],
    )
    .unwrap()
}
fn unwrap_request(mesh: &Mesh) -> Unwrap {
    Unwrap {
        attribute: "PaintUV".into(),
        attribute_id: Id(500),
        seams: Default::default(),
        pins: vec![
            Pin {
                corner: mesh.corner_ids[0],
                uv: [0., 0.],
            },
            Pin {
                corner: mesh.corner_ids[1],
                uv: [1., 0.],
            },
        ],
    }
}
fn coords(mesh: &Mesh) -> &[[f32; 2]] {
    let AttributeValues::Vec2(v) = &mesh.attributes["PaintUV"].values else {
        panic!("UV vec2")
    };
    v
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn planar_and_developable_charts_match_independent_coordinates() {
    let source = quad();
    let before = canonical(&source).unwrap();
    let q = unwrap_request(&source);
    let result = unwrap(&source, &q, &Budget::default(), || false).unwrap();
    for (a, b) in coords(&result.mesh)
        .iter()
        .zip([[0., 0.], [1., 0.], [1., 1.], [0., 1.]])
    {
        for (a, b) in a.iter().zip(b) {
            assert!((*a - b).abs() < 1e-6);
        }
    }
    assert_eq!(coords(&result.mesh)[0], q.pins[0].uv);
    assert_eq!(coords(&result.mesh)[1], q.pins[1].uv);
    assert_eq!(result.mesh.positions, source.positions);
    assert_eq!(result.mesh.corners, source.corners);
    assert_eq!(result.mesh.edge_ids, source.edge_ids);
    assert_eq!(result.mesh.corner_ids, source.corner_ids);
    assert_eq!(canonical(&source).unwrap(), before);
    assert!((result.report.charts[0].uv_area - 1.).abs() < 1e-6);
    assert_eq!(result.report.charts[0].surface_area_square_meters, 1.);
    let bent = Mesh::from_polygons(
        Positions::F64(vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [1., 0., 1.],
            [1., 1., 1.],
        ]),
        &[vec![0, 1, 2, 3], vec![1, 4, 5, 2]],
        &[],
    )
    .unwrap();
    let result = unwrap(&bent, &unwrap_request(&bent), &Budget::default(), || false).unwrap();
    let expected = [
        [0., 0.],
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [1., 0.],
        [2., 0.],
        [2., 1.],
        [1., 1.],
    ];
    for (a, b) in coords(&result.mesh).iter().zip(expected) {
        for (a, b) in a.iter().zip(b) {
            assert!((*a - b).abs() < 1e-6, "{a} != {b}");
        }
    }
    assert_eq!(result.report.charts.len(), 1);
    assert_eq!(result.report.charts[0].boundary_vertices, 6);
    assert!((result.report.charts[0].uv_area - 2.).abs() < 1e-6);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn closed_surfaces_need_seams_and_keep_other_attributes() {
    let mut mesh = modeling::box_mesh([0.; 3], [1.; 3]).unwrap();
    let mut q = unwrap_request(&mesh);
    q.pins.clear();
    assert_eq!(
        unwrap(&mesh, &q, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "uv_topology"
    );
    q.seams = mesh.edge_ids.iter().copied().collect();
    mesh.attributes.insert(
        "other".into(),
        Attribute {
            id: Id(600),
            domain: Domain::Point,
            semantic: "weight".into(),
            transfer: Transfer::Linear,
            values: AttributeValues::Scalar(vec![0.5; mesh.positions.len()]),
        },
    );
    let result = unwrap(&mesh, &q, &Budget::default(), || false).unwrap();
    assert_eq!(result.report.charts.len(), 6);
    assert_eq!(result.mesh.attributes["other"], mesh.attributes["other"]);
    assert_eq!(result.layout.seams, q.seams);
    result.layout.validate(&result.mesh).unwrap();
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn invalid_constraints_budgets_and_cancellation_are_bounded() {
    let mesh = quad();
    let q = unwrap_request(&mesh);
    let before = canonical(&mesh).unwrap();
    let mut one = q.clone();
    one.pins.pop();
    assert_eq!(
        unwrap(&mesh, &one, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "uv"
    );
    let mut stale = q.clone();
    stale.seams.insert(99999);
    assert_eq!(
        unwrap(&mesh, &stale, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "stale_selection"
    );
    let mut duplicate = q.clone();
    duplicate.pins.push(q.pins[0].clone());
    assert_eq!(
        unwrap(&mesh, &duplicate, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "uv"
    );
    let low = Budget {
        max_solver_work: 1,
        ..Default::default()
    };
    assert_eq!(
        unwrap(&mesh, &q, &low, || false).err().unwrap().code,
        "budget"
    );
    let low = Budget {
        max_output_bytes: 1,
        ..Default::default()
    };
    assert_eq!(
        unwrap(&mesh, &q, &low, || false).err().unwrap().code,
        "budget"
    );
    assert_eq!(
        unwrap(&mesh, &q, &Budget::default(), || true)
            .err()
            .unwrap()
            .code,
        "cancelled"
    );
    let mut total = 0;
    unwrap(&mesh, &q, &Budget::default(), || {
        total += 1;
        false
    })
    .unwrap();
    for stop in [3, total - 1] {
        let mut calls = 0;
        assert_eq!(
            unwrap(&mesh, &q, &Budget::default(), || {
                calls += 1;
                calls >= stop
            })
            .err()
            .unwrap()
            .code,
            "cancelled"
        );
    }
    assert_eq!(canonical(&mesh).unwrap(), before);
    let mut framed = mesh.clone();
    framed.attributes.insert(
        "tangent".into(),
        Attribute {
            id: Id(601),
            domain: Domain::Point,
            semantic: "tangent".into(),
            transfer: Transfer::Normalize,
            values: AttributeValues::Vec3(vec![[1., 0., 0.]; 4]),
        },
    );
    assert_eq!(
        unwrap(&framed, &q, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "uv_frame"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn atlas_packing_is_bounded_deterministic_and_pin_explicit() {
    let mesh = modeling::box_mesh([0.; 3], [1.; 3]).unwrap();
    let mut q = unwrap_request(&mesh);
    q.pins.clear();
    q.seams = mesh.edge_ids.iter().copied().collect();
    let unwrapped = unwrap(&mesh, &q, &Budget::default(), || false).unwrap();
    let request = Pack {
        atlas: Atlas {
            width: 64,
            height: 64,
            padding_pixels: 1,
            pixels_per_meter: 12.,
        },
        pins: PinPacking::Preserve,
    };
    let packed = pack(
        &unwrapped.mesh,
        &unwrapped.layout,
        &request,
        &Budget::default(),
        || false,
    )
    .unwrap();
    packed.layout.validate(&packed.mesh).unwrap();
    assert_eq!(packed.report.charts.len(), 6);
    assert_eq!(packed.mesh.positions, mesh.positions);
    assert_eq!(packed.mesh.corner_ids, mesh.corner_ids);
    let again = pack(
        &unwrapped.mesh,
        &unwrapped.layout,
        &request,
        &Budget::default(),
        || false,
    )
    .unwrap();
    assert_eq!(
        canonical(&packed.mesh).unwrap(),
        canonical(&again.mesh).unwrap()
    );
    assert_eq!(packed.layout, again.layout);
    for chart in &packed.report.charts {
        let density = (chart.uv_area * 64. * 64. / chart.surface_area_square_meters).sqrt();
        assert!((density - 12.).abs() < 0.0002);
    }
    let too_dense = Pack {
        atlas: Atlas {
            pixels_per_meter: 64.,
            ..request.atlas.clone()
        },
        pins: PinPacking::Preserve,
    };
    assert_eq!(
        pack(
            &unwrapped.mesh,
            &unwrapped.layout,
            &too_dense,
            &Budget::default(),
            || false
        )
        .err()
        .unwrap()
        .code,
        "uv_atlas"
    );
    let source = quad();
    let mut q = unwrap_request(&source);
    q.pins[0].uv = [0.125, 0.125];
    q.pins[1].uv = [0.375, 0.125];
    let unwrapped = unwrap(&source, &q, &Budget::default(), || false).unwrap();
    let request = Pack {
        atlas: Atlas {
            width: 64,
            height: 64,
            padding_pixels: 1,
            pixels_per_meter: 16.,
        },
        pins: PinPacking::Preserve,
    };
    let packed = pack(
        &unwrapped.mesh,
        &unwrapped.layout,
        &request,
        &Budget::default(),
        || false,
    )
    .unwrap();
    assert_eq!(packed.layout.pins, q.pins);
    assert_eq!(coords(&packed.mesh), coords(&unwrapped.mesh));
    let change = Pack {
        atlas: Atlas {
            pixels_per_meter: 8.,
            ..request.atlas.clone()
        },
        pins: PinPacking::Preserve,
    };
    assert_eq!(
        pack(
            &unwrapped.mesh,
            &unwrapped.layout,
            &change,
            &Budget::default(),
            || false
        )
        .err()
        .unwrap()
        .code,
        "uv_pins"
    );
    let change = Pack {
        pins: PinPacking::TransformWithChart,
        ..change
    };
    let changed = pack(
        &unwrapped.mesh,
        &unwrapped.layout,
        &change,
        &Budget::default(),
        || false,
    )
    .unwrap();
    assert_ne!(changed.layout.pins, q.pins);
    changed.layout.validate(&changed.mesh).unwrap();
    assert_eq!(
        pack(
            &unwrapped.mesh,
            &unwrapped.layout,
            &request,
            &Budget::default(),
            || true
        )
        .err()
        .unwrap()
        .code,
        "cancelled"
    );
    let mut bad = packed.layout.clone();
    bad.atlas.as_mut().unwrap().width = 0;
    assert_eq!(bad.validate(&packed.mesh).unwrap_err().code, "budget");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn named_sets_keep_legacy_default_and_version_the_explicit_metadata() {
    use render_core::{document::*, fixtures, mesh_codec};
    let mut mesh = quad();
    let legacy = mesh_codec::encode(&mesh).unwrap();
    assert_eq!(&legacy[..8], b"R3DMESH0");
    assert!(
        !String::from_utf8(canonical(&mesh).unwrap())
            .unwrap()
            .contains("default_uv_attribute")
    );
    mesh.attributes.insert(
        "ZExisting".into(),
        Attribute {
            id: Id(700),
            domain: Domain::Corner,
            semantic: "uv".into(),
            transfer: Transfer::Linear,
            values: AttributeValues::Vec2(vec![[0., 0.], [2., 0.], [2., 2.], [0., 2.]]),
        },
    );
    let original = mesh.clone();
    let result = unwrap(&mesh, &unwrap_request(&mesh), &Budget::default(), || false).unwrap();
    assert_eq!(result.mesh.default_uv_attribute, Some(Id(700)));
    assert_eq!(
        result.mesh.attributes["ZExisting"],
        original.attributes["ZExisting"]
    );
    for c in 0..4 {
        assert_eq!(result.mesh.uv(c), original.uv(c));
    }
    let encoded = mesh_codec::encode(&result.mesh).unwrap();
    assert_eq!(&encoded[..8], b"R3DMESH1");
    assert_eq!(mesh_codec::decode(&encoded).unwrap(), result.mesh);
    let mut mismatch = encoded.clone();
    mismatch[7] = b'0';
    assert_eq!(
        mesh_codec::decode(&mismatch).unwrap_err().code,
        "mesh_chunk"
    );
    let mut mismatch = legacy;
    mismatch[7] = b'1';
    assert_eq!(
        mesh_codec::decode(&mismatch).unwrap_err().code,
        "mesh_chunk"
    );
    let (triangulated, _) = modeling::apply(
        &result.mesh,
        &modeling::Operation::Triangulate,
        &modeling::Budget::default(),
        || false,
    )
    .unwrap();
    assert_eq!(triangulated.default_uv_attribute, Some(Id(700)));
    let mut invalid = result.mesh.clone();
    invalid.default_uv_attribute = Some(Id(701));
    assert_eq!(invalid.validate().unwrap_err().code, "reference");
    let mut document = Document::new(Snapshot::empty(Id(17))).unwrap();
    let q = fixtures::request(
        &document,
        "uv-default-version17",
        vec![Command::PutMesh { mesh: result.mesh }],
    )
    .unwrap();
    document.execute(&fixtures::principal(), &q).unwrap();
    assert_eq!(document.snapshot().version, 17);
    let mut old = document.snapshot().clone();
    old.version = 16;
    assert_eq!(old.validate().unwrap_err().code, "schema_version");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn fully_pinned_flips_and_global_overlaps_reject() {
    let mesh = quad();
    let mut q = unwrap_request(&mesh);
    q.pins = mesh
        .corner_ids
        .iter()
        .zip([[0., 0.], [0., 1.], [1., 1.], [1., 0.]])
        .map(|(&corner, uv)| Pin { corner, uv })
        .collect();
    assert_eq!(
        unwrap(&mesh, &q, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "uv_fold"
    );
    let mesh = Mesh::from_polygons(
        Positions::F64(vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 2., 0.],
            [-1., 2., 0.],
            [-2., 1., 0.],
            [-2., 0., 0.],
        ]),
        &(1..6).map(|i| vec![0, i, i + 1]).collect::<Vec<_>>(),
        &[],
    )
    .unwrap();
    let uv = [
        [0., 0.],
        [1., 0.],
        [0., 1.],
        [-1., 0.],
        [0., -1.],
        [1., 0.],
        [0., 1.],
    ];
    let mut q = unwrap_request(&mesh);
    q.pins = mesh
        .corners
        .iter()
        .zip(&mesh.corner_ids)
        .map(|(c, &corner)| Pin {
            corner,
            uv: uv[c.vertex as usize],
        })
        .collect();
    assert_eq!(
        unwrap(&mesh, &q, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "uv_overlap"
    );
}

fn authored_document() -> (render_core::document::Document, Request) {
    use render_core::{document::*, fixtures};
    let mesh = quad();
    let source = mesh.content_id().unwrap();
    let mut doc = Document::new(Snapshot::empty(Id(950))).unwrap();
    let mut commands = vec![Command::PutMesh { mesh: mesh.clone() }];
    for id in [Id(951), Id(952)] {
        commands.push(Command::CreateEntity {
            entity: Entity {
                id,
                name: format!("UV instance {id:?}"),
                parent: None,
                mesh: Some(source.clone()),
                material: None,
                transform: Transform::default(),
            },
        });
    }
    let init = fixtures::request(&doc, "uv-initialize-instances", commands).unwrap();
    doc.execute(&fixtures::principal(), &init).unwrap();
    let request = render_core::uv::Request {
        version: 0,
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "uv-author-first-layout".into(),
        entity: Id(951),
        source_mesh: source,
        source_uv_asset: None,
        operation: Operation::Unwrap {
            settings: unwrap_request(&mesh),
        },
        budget: Budget::default(),
        max_added_bytes: 4 * 1024 * 1024,
    };
    (doc, request)
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn transactions_preserve_instances_multiple_sets_and_retry_identity() {
    use render_core::{document::*, fixtures, storage::*};
    let (mut doc, first) = authored_document();
    let principal = fixtures::principal();
    let old = canonical(doc.snapshot()).unwrap();
    let prepared = prepare(&doc, &principal, &first, || false).unwrap();
    assert_eq!(canonical(doc.snapshot()).unwrap(), old);
    let mut store = MemoryStore::default();
    let receipt = durable_execute(&mut store, &mut doc, &principal, &prepared.transaction).unwrap();
    assert!(receipt.durable);
    assert_eq!(doc.snapshot().version, 17);
    assert_eq!(
        doc.snapshot().entities.get(Id(952)).unwrap().mesh.as_ref(),
        Some(&first.source_mesh)
    );
    assert_ne!(
        doc.snapshot().entities.get(Id(951)).unwrap().mesh.as_ref(),
        Some(&first.source_mesh)
    );
    let retry = prepare(&doc, &principal, &first, || false).unwrap();
    assert_eq!(
        canonical(&retry.transaction).unwrap(),
        canonical(&prepared.transaction).unwrap()
    );
    assert_eq!(
        canonical(&receipt).unwrap(),
        canonical(&durable_execute(&mut store, &mut doc, &principal, &retry.transaction).unwrap())
            .unwrap()
    );
    assert_eq!(store.records.len(), 1);
    let first_set = doc.snapshot().uv_assets[&prepared.uv_asset].sets[&Id(500)].clone();
    let source = prepared.report.output_mesh;
    let mut settings = unwrap_request(&doc.snapshot().meshes[&source]);
    settings.attribute = "AnotherUV".into();
    settings.attribute_id = Id(501);
    settings.pins[1].uv = [2., 0.];
    let second = render_core::uv::Request {
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "uv-author-second-layout".into(),
        source_mesh: source,
        source_uv_asset: Some(prepared.uv_asset),
        operation: Operation::Unwrap { settings },
        ..first.clone()
    };
    let next = prepare(&doc, &principal, &second, || false).unwrap();
    durable_execute(&mut store, &mut doc, &principal, &next.transaction).unwrap();
    let asset = &doc.snapshot().uv_assets[&next.uv_asset];
    assert_eq!(asset.sets.len(), 2);
    let mut carried = first_set;
    carried.mesh = asset.mesh.clone();
    assert_eq!(asset.sets[&Id(500)], carried);
    assert_eq!(
        doc.snapshot().meshes[&asset.mesh].default_uv_id(),
        Some(Id(500))
    );
    assert_eq!(
        next.report.output_bytes as usize,
        canonical(asset.as_ref()).unwrap().len()
            + canonical(doc.snapshot().meshes[&asset.mesh].as_ref())
                .unwrap()
                .len()
    );
    let recovered = recover(&store.records).unwrap().unwrap();
    assert_eq!(canonical(&doc).unwrap(), canonical(&recovered).unwrap());
    let previous = store.records[0].decode().unwrap();
    assert_eq!(
        previous.snapshot().uv_assets[&second.source_uv_asset.clone().unwrap()]
            .sets
            .len(),
        1
    );
    // Replay after later authoring still resolves the retained original sources.
    assert!(
        doc.retry(
            &principal,
            &prepare(&doc, &principal, &first, || false)
                .unwrap()
                .transaction
        )
        .unwrap()
        .is_some()
    );
    let mut too_small = second.clone();
    too_small.budget.max_output_bytes = next.report.output_bytes - 1;
    assert_eq!(
        prepare(&previous, &principal, &too_small, || false)
            .err()
            .unwrap()
            .code,
        "budget"
    );
    let mut corrupt = doc.snapshot().clone();
    corrupt.version = 16;
    assert_eq!(corrupt.validate().unwrap_err().code, "schema_version");
    let mut corrupt = asset.as_ref().clone();
    corrupt.sets.get_mut(&Id(500)).unwrap().mesh = first.source_mesh;
    assert!(
        corrupt
            .validate(&doc.snapshot().meshes[&asset.mesh])
            .is_err()
    );
    // Ordinary geometry changes must explicitly clear constraints before changing their mesh.
    let clear = fixtures::request(
        &doc,
        "uv-explicitly-clear-layout",
        vec![Command::SetUvAsset {
            entity: Id(951),
            source_mesh: asset.mesh.clone(),
            source_asset: Some(next.uv_asset),
            asset: None,
        }],
    );
    doc.execute(&principal, &clear.unwrap()).unwrap();
    assert!(!doc.snapshot().uv_bindings.contains_key(&Id(951)));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn failed_publication_permissions_stale_selection_and_cancellation_are_atomic() {
    use render_core::{document::*, fixtures, storage::*};
    let (mut doc, request) = authored_document();
    let principal = fixtures::principal();
    let before = canonical(&doc).unwrap();
    let prepared = prepare(&doc, &principal, &request, || false).unwrap();
    for (quota, fail_publication, code) in [(1, false, "quota"), (0, true, "storage")] {
        let mut store = MemoryStore {
            quota,
            fail_publication,
            ..Default::default()
        };
        assert_eq!(
            durable_execute(&mut store, &mut doc, &principal, &prepared.transaction)
                .unwrap_err()
                .code,
            code
        );
        assert!(store.records.is_empty());
        assert_eq!(canonical(&doc).unwrap(), before);
    }
    let denied = Principal {
        can_write: false,
        ..principal.clone()
    };
    assert_eq!(
        prepare(&doc, &denied, &request, || false)
            .err()
            .unwrap()
            .code,
        "permission"
    );
    let mut stale = request.clone();
    stale.base_revision = "0".repeat(64);
    assert_eq!(
        prepare(&doc, &principal, &stale, || false)
            .err()
            .unwrap()
            .code,
        "stale_revision"
    );
    let mut calls = 0;
    prepare(&doc, &principal, &request, || {
        calls += 1;
        false
    })
    .unwrap();
    for stop in [1, calls / 2, calls] {
        let mut at = 0;
        assert_eq!(
            prepare(&doc, &principal, &request, || {
                at += 1;
                at >= stop
            })
            .err()
            .unwrap()
            .code,
            "cancelled"
        );
        assert_eq!(canonical(&doc).unwrap(), before);
    }
    doc.execute(&principal, &prepared.transaction).unwrap();
    let after = canonical(&doc).unwrap();
    let mut stale = request;
    stale.base_revision = doc.snapshot().revision().unwrap();
    stale.idempotency_key = "uv-stale-source-selection".into();
    assert_eq!(
        prepare(&doc, &principal, &stale, || false)
            .err()
            .unwrap()
            .code,
        "stale_selection"
    );
    assert_eq!(canonical(&doc).unwrap(), after);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn boundary_touch_and_authored_packing_frames_reject() {
    let mesh = Mesh::from_polygons(
        Positions::F64(vec![
            [0., 0., 0.],
            [2., 0., 0.],
            [2., 1., 0.],
            [1., 2., 0.],
            [0., 2., 0.],
            [-1., 2., 0.],
        ]),
        &[vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 4], vec![0, 4, 5]],
        &[],
    )
    .unwrap();
    let positions = [[0., 0.], [1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]];
    let mut settings = unwrap_request(&mesh);
    settings.pins = mesh
        .corners
        .iter()
        .zip(&mesh.corner_ids)
        .map(|(c, &corner)| Pin {
            corner,
            uv: positions[c.vertex as usize],
        })
        .collect();
    assert_eq!(
        unwrap(&mesh, &settings, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "uv_overlap"
    );
    let mesh = quad();
    let mut result = unwrap(&mesh, &unwrap_request(&mesh), &Budget::default(), || false).unwrap();
    result.mesh.attributes.insert(
        "authored_tangent".into(),
        Attribute {
            id: Id(999),
            domain: Domain::Corner,
            semantic: "tangent".into(),
            transfer: Transfer::Normalize,
            values: AttributeValues::Vec3(vec![[1., 0., 0.]; 4]),
        },
    );
    result.layout.mesh = result.mesh.content_id().unwrap();
    assert_eq!(
        pack(
            &result.mesh,
            &result.layout,
            &Pack {
                atlas: Atlas {
                    width: 64,
                    height: 64,
                    padding_pixels: 2,
                    pixels_per_meter: 16.
                },
                pins: PinPacking::TransformWithChart
            },
            &Budget::default(),
            || false
        )
        .err()
        .unwrap()
        .code,
        "uv_frame"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn public_agent_uv_workflow_checks_permission_cancellation_and_durability() {
    use render_core::{agent, document::*, fixtures, storage::*};
    let mut doc = Document::new(Snapshot::empty(Id(950))).unwrap();
    let commands = serde_json::from_str(include_str!(
        "../../../fixtures/uv-authoring/data/create.json"
    ))
    .unwrap();
    let create = fixtures::request(&doc, "uv-workflow-create", commands).unwrap();
    doc.execute(&fixtures::principal(), &create).unwrap();
    let source = doc
        .snapshot()
        .entities
        .get(Id(1))
        .unwrap()
        .mesh
        .clone()
        .unwrap();
    let settings = serde_json::from_str(include_str!(
        "../../../fixtures/uv-authoring/data/unwrap.json"
    ))
    .unwrap();
    let request = render_core::uv::Request {
        version: 0,
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "uv-public-agent-unwrap".into(),
        entity: Id(1),
        source_mesh: source,
        source_uv_asset: None,
        operation: Operation::Unwrap { settings },
        budget: Budget::default(),
        max_added_bytes: 4 * 1024 * 1024,
    };
    let request = agent::Request {
        version: 0,
        operation: agent::Operation::AuthorUv { request },
    };
    let p = fixtures::principal();
    let mut session = agent::Session::new(doc);
    let before = canonical(session.document()).unwrap();
    assert_eq!(
        session
            .dispatch(
                &Principal {
                    can_write: false,
                    ..p.clone()
                },
                request.clone()
            )
            .unwrap_err()
            .code,
        "permission"
    );
    let mut probe = session.clone();
    let mut calls = 0;
    probe
        .dispatch_cancellable(&p, request.clone(), || {
            calls += 1;
            false
        })
        .unwrap();
    for stop in [1, calls / 2, calls] {
        let mut at = 0;
        assert_eq!(
            session
                .dispatch_cancellable(&p, request.clone(), || {
                    at += 1;
                    at >= stop
                })
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert_eq!(canonical(session.document()).unwrap(), before);
    }
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
    let result = session
        .dispatch_durable(&mut store, &p, request.clone())
        .unwrap();
    assert_eq!(result["receipt"]["durable"], true);
    assert_eq!(
        session
            .dispatch_durable(&mut store, &p, request.clone())
            .unwrap(),
        result
    );
    assert_eq!(store.records.len(), 1);
    let mut wire = serde_json::to_value(request).unwrap();
    wire["operation"]["request"]["operation"]["settings"]["runtime_index"] = serde_json::json!(0);
    assert!(
        session
            .dispatch_wire(&p, &canonical(&wire).unwrap())
            .is_err()
    );
    assert_eq!(
        canonical(&recover(&store.records).unwrap().unwrap()).unwrap(),
        canonical(session.document()).unwrap()
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn evaluated_animation_keeps_authored_uv_constraints_on_the_source_only() {
    use render_core::{animation, document::*, feature_fixtures, fixtures};
    let mut doc = feature_fixtures::character_document().unwrap();
    let source = doc
        .snapshot()
        .entities
        .get(Id(8201))
        .unwrap()
        .mesh
        .clone()
        .unwrap();
    let mesh = &doc.snapshot().meshes[&source];
    let request = render_core::uv::Request {
        version: 0,
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "uv-character-authoring".into(),
        entity: Id(8201),
        source_mesh: source.clone(),
        source_uv_asset: None,
        operation: Operation::Unwrap {
            settings: Unwrap {
                attribute: "CharacterUV".into(),
                attribute_id: Id(9900),
                // Cut box edges but retain coplanar triangulation diagonals: 42 charts.
                seams: mesh
                    .edges
                    .iter()
                    .zip(&mesh.edge_ids)
                    .filter_map(|(edge, &id)| {
                        let delta = mesh.positions.get(edge[1] as usize)
                            - mesh.positions.get(edge[0] as usize);
                        (delta.to_array().iter().filter(|&&x| x != 0.).count() == 1).then_some(id)
                    })
                    .collect(),
                pins: vec![],
            },
        },
        budget: Budget::default(),
        max_added_bytes: 4 * 1024 * 1024,
    };
    let prepared = prepare(&doc, &fixtures::principal(), &request, || false).unwrap();
    doc.execute(&fixtures::principal(), &prepared.transaction)
        .unwrap();
    let before = canonical(&doc).unwrap();
    let posed = animation::evaluate(
        doc.snapshot(),
        Id(8400),
        render_core::Time::new(1, 1).unwrap(),
        || false,
    )
    .unwrap();
    posed.snapshot.validate().unwrap();
    assert!(!posed.snapshot.uv_bindings.contains_key(&Id(8201)));
    assert!(doc.snapshot().uv_bindings.contains_key(&Id(8201)));
    assert_eq!(canonical(&doc).unwrap(), before);
    let mut hidden = doc.snapshot().clone();
    hidden.layers.push(Layer {
        name: "hide authored mesh".into(),
        overrides: std::collections::BTreeMap::from([(
            Id(8201),
            Override {
                transform: None,
                material: None,
                hidden: true,
            },
        )]),
    });
    // The legacy composition view is tested after discarding animation state.
    hidden.animation = None;
    let composed = hidden.composed().unwrap();
    assert!(!composed.uv_bindings.contains_key(&Id(8201)));
    assert!(hidden.uv_bindings.contains_key(&Id(8201)));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn scale_normalization_and_named_set_limit_are_explicit() {
    let mesh = quad();
    let settings = unwrap_request(&mesh);
    let reference = unwrap(&mesh, &settings, &Budget::default(), || false).unwrap();
    for scale in [1e-6, 1e6] {
        let mut scaled = mesh.clone();
        for i in 0..scaled.positions.len() {
            scaled
                .positions
                .set(i, scaled.positions.get(i) * scale)
                .unwrap();
        }
        let result = unwrap(&scaled, &settings, &Budget::default(), || false).unwrap();
        for (a, b) in coords(&result.mesh)
            .iter()
            .flatten()
            .zip(coords(&reference.mesh).iter().flatten())
        {
            assert!((a - b).abs() < 1e-6);
        }
        assert!(
            (result.report.charts[0].surface_area_square_meters / (scale * scale) - 1.).abs()
                < 1e-12
        );
    }
    let (mut doc, mut request) = authored_document();
    let p = render_core::fixtures::principal();
    for n in 0..8 {
        let mut settings = unwrap_request(&doc.snapshot().meshes[&request.source_mesh]);
        settings.attribute = format!("Set{n}");
        settings.attribute_id = Id(1000 + n);
        request.operation = Operation::Unwrap { settings };
        request.idempotency_key = format!("uv-named-limit-{n:04}");
        let prepared = prepare(&doc, &p, &request, || false).unwrap();
        doc.execute(&p, &prepared.transaction).unwrap();
        request.source_mesh = prepared.report.output_mesh;
        request.source_uv_asset = Some(prepared.uv_asset);
        request.base_revision = doc.snapshot().revision().unwrap();
    }
    let before = canonical(&doc).unwrap();
    request.operation = Operation::Unwrap {
        settings: unwrap_request(&doc.snapshot().meshes[&request.source_mesh]),
    };
    request.idempotency_key = "uv-named-limit-nine".into();
    assert_eq!(
        prepare(&doc, &p, &request, || false).err().unwrap().code,
        "budget"
    );
    assert_eq!(canonical(&doc).unwrap(), before);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn stale_topology_and_retained_asset_budget_reject_atomically() {
    use render_core::{document::*, fixtures};
    use std::{collections::BTreeMap, sync::Arc};
    let (mut doc, request) = authored_document();
    let p = fixtures::principal();
    let result = prepare(&doc, &p, &request, || false).unwrap();
    doc.execute(&p, &result.transaction).unwrap();
    let before = canonical(&doc).unwrap();
    let edit = Command::ModelMesh {
        entity: request.entity,
        source_mesh: result.report.output_mesh.clone(),
        operation: modeling::Operation::Triangulate,
        budget: modeling::Budget::default(),
    };
    let mutate = fixtures::request(&doc, "uv-reject-stale-topology", vec![edit.clone()]).unwrap();
    assert_eq!(
        doc.execute(&p, &mutate).unwrap_err().code,
        "stale_selection"
    );
    assert_eq!(canonical(&doc).unwrap(), before);
    let clear = Command::SetUvAsset {
        entity: request.entity,
        source_mesh: result.report.output_mesh.clone(),
        source_asset: Some(result.uv_asset.clone()),
        asset: None,
    };
    let mutate = fixtures::request(&doc, "uv-clear-before-topology", vec![clear, edit]).unwrap();
    doc.execute(&p, &mutate).unwrap();
    doc.snapshot().validate().unwrap();
    let base = doc.snapshot().uv_assets[&result.uv_asset].sets[&Id(500)].clone();
    let mesh = &doc.snapshot().meshes[&base.mesh];
    let mut snapshot = doc.snapshot().clone();
    snapshot.uv_assets.clear();
    for n in 0..65 {
        let mut layout = base.clone();
        layout.seams = mesh
            .edge_ids
            .iter()
            .enumerate()
            .filter_map(|(bit, id)| (n & (1 << bit) != 0).then_some(*id))
            .collect();
        layout.pins = (0..3)
            .filter(|i| n & (1 << (4 + i)) != 0)
            .map(|i| Pin {
                corner: mesh.corner_ids[i],
                uv: coords(mesh)[i],
            })
            .collect();
        let asset = Asset {
            version: 0,
            mesh: base.mesh.clone(),
            sets: BTreeMap::from([(base.attribute_id, layout)]),
        };
        asset.validate(mesh).unwrap();
        snapshot
            .uv_assets
            .insert(asset.content_id().unwrap(), Arc::new(asset));
        assert_eq!(snapshot.uv_assets.len(), n + 1);
        if n == 63 {
            snapshot.validate().unwrap();
        }
    }
    assert_eq!(snapshot.validate().unwrap_err().code, "budget");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn evaluation_and_interchange_keep_the_explicit_default_before_other_names() {
    use render_core::{document::*, fixtures, gltf_export, interchange, render::Evaluator};
    let (mut doc, request) = authored_document();
    let p = fixtures::principal();
    let first = prepare(&doc, &p, &request, || false).unwrap();
    doc.execute(&p, &first.transaction).unwrap();
    let mut settings = unwrap_request(&doc.snapshot().meshes[&first.report.output_mesh]);
    settings.attribute = "AnotherUV".into();
    settings.attribute_id = Id(501);
    settings.pins[1].uv = [2., 0.];
    let second = render_core::uv::Request {
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "uv-default-export-two".into(),
        source_mesh: first.report.output_mesh,
        source_uv_asset: Some(first.uv_asset),
        operation: Operation::Unwrap { settings },
        ..request
    };
    let second = prepare(&doc, &p, &second, || false).unwrap();
    doc.execute(&p, &second.transaction).unwrap();
    // The second shared instance stays on the original mesh; materialize a
    // separate export snapshot with one untextured surface to isolate UV semantics.
    let mut snapshot = doc.snapshot().clone();
    let mut material = Material::diffuse(Id(2), [0.5; 3]);
    material.pbr = Some(render_core::pbr::Surface::default());
    snapshot.materials.insert(Id(2), material);
    for id in [Id(951), Id(952)] {
        snapshot.entities.get_mut(id).unwrap().material = Some(Id(2));
    }
    snapshot.render_settings = Some(fixtures::settings());
    let scene = Evaluator::default().evaluate(&snapshot).unwrap();
    let geometry = &scene
        .instances
        .iter()
        .find(|i| i.id == Id(951))
        .unwrap()
        .geometry;
    assert_eq!(geometry.uv_attributes, [Id(500), Id(501)]);
    for triangle in &geometry.triangles {
        assert_eq!(triangle.uv, triangle.uv_sets[0]);
        assert_ne!(triangle.uv, triangle.uv_sets[1]);
    }
    let mesh = &snapshot.meshes[&second.report.output_mesh];
    let (obj, report) = interchange::export_obj(mesh).unwrap();
    assert!(report.losses.iter().any(|s| s.contains("AnotherUV")));
    assert!(!report.losses.iter().any(|s| s.contains("PaintUV")));
    let imported = interchange::import_obj(&obj).unwrap().0;
    for i in 0..mesh.corners.len() {
        assert_eq!(mesh.uv(i), imported.uv(i));
    }
    let exported = gltf_export::export(
        &snapshot,
        &gltf_export::Request {
            revision: snapshot.revision().unwrap(),
            at: None,
            policy: gltf_export::Policy {
                allow_approximations: true,
                ..Default::default()
            },
        },
        || false,
    )
    .unwrap();
    assert!(!exported.glb.is_empty());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn nondevelopable_disk_has_the_analytic_symmetric_interior_solution() {
    let mesh = Mesh::from_polygons(
        Positions::F64(vec![
            [0., 0., 1.],
            [-1., -1., 0.],
            [1., -1., 0.],
            [1., 1., 0.],
            [-1., 1., 0.],
        ]),
        &[vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 4], vec![0, 4, 1]],
        &[],
    )
    .unwrap();
    let expected = [[0.5, 0.5], [0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut pinned = std::collections::BTreeMap::new();
    for (c, &corner) in mesh.corners.iter().zip(&mesh.corner_ids) {
        if c.vertex != 0 {
            pinned.entry(c.vertex).or_insert(Pin {
                corner,
                uv: expected[c.vertex as usize],
            });
        }
    }
    let settings = Unwrap {
        attribute: "PaintUV".into(),
        attribute_id: Id(500),
        seams: Default::default(),
        pins: pinned.into_values().collect(),
    };
    let result = unwrap(&mesh, &settings, &Budget::default(), || false).unwrap();
    for (corner, actual) in mesh.corners.iter().zip(coords(&result.mesh)) {
        for (a, b) in actual.iter().zip(expected[corner.vertex as usize]) {
            assert!((a - b).abs() < 1e-6);
        }
    }
    let report = &result.report.charts[0];
    assert_eq!(report.vertices, 5);
    assert!((report.surface_area_square_meters - 4. * 2f64.sqrt()).abs() < 1e-12);
    assert!((report.uv_area - 1.).abs() < 1e-6);
    assert!(report.least_squares_residual.unwrap() > 0.01);
    assert!(report.qr_pivot_ratio.unwrap() > 0.9);
}
