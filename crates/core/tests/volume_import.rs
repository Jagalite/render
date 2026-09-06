use glam::{DAffine3, DVec3};
use render_core::{document::*, render::*, volume_import::*, *};
const DENSITY: &[u8] = include_bytes!("../../../fixtures/volume-import/data/homogeneous.vol");
const EMISSION: &[u8] = include_bytes!("../../../fixtures/volume-import/data/emission.vol");
const AXIS: &[u8] = include_bytes!("../../../fixtures/volume-import/data/axis.vol");
const EMPTY: &[u8] = include_bytes!("../../../fixtures/volume-import/data/empty.vol");
const SPARSE: &[u8] = include_bytes!("../../../fixtures/volume-import/data/sparse.vol");
fn policy() -> Policy {
    Policy {
        bounds: BoundsPolicy::File,
        reconstruction: Reconstruction::CellConstantZeroOutside,
        meters_per_unit: 1.,
        density_scale: 1.,
        emission_scale: [1.; 3],
        absorption: [0.5, 1., 2.],
        scattering: [0.; 3],
        anisotropy: 0.,
        max_step_meters: 0.1,
    }
}
fn asset(i: &Imported) -> &volumes::Asset {
    i.commands
        .iter()
        .find_map(|c| {
            if let Command::PutVolume { asset } = c {
                Some(asset)
            } else {
                None
            }
        })
        .unwrap()
}
fn read(d: &[u8], e: Option<&[u8]>, p: &Policy) -> Imported {
    import(d, e, Id(10100), p, || false).unwrap()
}
fn fail(d: &[u8], e: Option<&[u8]>, p: &Policy, code: &str) {
    let error = import(d, e, Id(10100), p, || false).err().unwrap();
    assert_eq!(error.code, code, "{error}");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exact_axes_sparse_empty_units_and_analytic_transport() {
    let p = policy();
    let i = read(AXIS, None, &p);
    let a = asset(&i);
    for c in &a.cells {
        let [x, y, z] = c.coordinate;
        assert_eq!(c.density, f64::from(1 + x + 2 * y + 4 * z));
    }
    assert_eq!(i.report.source_dimensions, [2; 3]);
    assert_eq!(a.voxel_size, [1.; 3]);
    let m = volumes::Media::build([(Id(1), a, DAffine3::IDENTITY)], || false).unwrap();
    for (x, y, sum) in [
        (0.5, 0.5, 6.),
        (1.5, 0.5, 8.),
        (0.5, 1.5, 10.),
        (1.5, 1.5, 12.),
    ] {
        let t = m
            .transmittance(
                Ray {
                    origin: DVec3::new(x, y, -1.),
                    direction: DVec3::Z,
                },
                0.,
                10.,
                || false,
            )
            .unwrap();
        for c in 0..3 {
            assert!((t[c] - (-p.absorption[c] * sum).exp()).abs() < 1e-13);
        }
    }
    for scale in [0.5, 1., 2.] {
        for bounds in [BoundsPolicy::File, BoundsPolicy::UnitCube] {
            let mut p = p.clone();
            p.meters_per_unit = scale;
            p.bounds = bounds;
            let i = read(DENSITY, Some(EMISSION), &p);
            let a = asset(&i);
            let length = if bounds == BoundsPolicy::File {
                2. * scale
            } else {
                scale
            };
            let m = volumes::Media::build([(Id(1), a, DAffine3::IDENTITY)], || false).unwrap();
            let t = m
                .transport(
                    Ray {
                        origin: DVec3::new(length / 4., length / 4., -1.),
                        direction: DVec3::Z,
                    },
                    0.,
                    10.,
                    |_, _| panic!(),
                    || false,
                )
                .unwrap();
            for c in 0..3 {
                let expected = (-p.absorption[c] * length).exp();
                assert!((t.transmittance[c] - expected).abs() < 1e-13);
                assert!(
                    (t.radiance[c] - [0.25, 0.5, 1.][c] * (1. - expected) / p.absorption[c]).abs()
                        < 1e-13
                );
            }
            assert_eq!(
                m.transmittance(
                    Ray {
                        origin: DVec3::new(-1., -1., -1.),
                        direction: DVec3::Z
                    },
                    0.,
                    10.,
                    || false
                )
                .unwrap(),
                DVec3::ONE
            );
        }
    }
    let sparse = read(SPARSE, None, &p);
    assert_eq!(sparse.report.occupied_cells, 5);
    assert_eq!(sparse.report.source_cells, 64_u64.pow(3));
    assert!(sparse.report.asset_json_bytes < 2048);
    assert_eq!(asset(&sparse).cells.last().unwrap().coordinate, [63; 3]);
    let empty = read(EMPTY, None, &p);
    assert_eq!(empty.commands.len(), 1);
    assert_eq!(empty.report.volume_asset, None);
    assert_eq!(empty.report.asset_json_bytes, 0);
    let emitting = read(EMPTY, Some(EMISSION), &p);
    assert_eq!(emitting.report.occupied_cells, 8);
    assert!(asset(&emitting).cells.iter().all(|c| c.density == 0.));
    let again = read(AXIS, None, &p);
    assert_eq!(i.report, again.report);
    assert_eq!(
        canonical(&i.commands).unwrap(),
        canonical(&again.commands).unwrap()
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn malformed_unsupported_policy_budget_and_cancellation() {
    let p = policy();
    for n in [0, 3, 47, 48, DENSITY.len() - 1] {
        fail(&DENSITY[..n], None, &p, "vol");
    }
    let mut b = DENSITY.to_vec();
    b.push(0);
    fail(&b, None, &p, "vol");
    for (at, value, code) in [
        (0, 0, "vol"),
        (3, 2, "unsupported_vol"),
        (4, 2, "unsupported_vol"),
        (20, 3, "unsupported_vol"),
        (8, 0, "vol"),
    ] {
        let mut b = DENSITY.to_vec();
        b[at] = value;
        fail(&b, None, &p, code);
    }
    for value in [-1f32, f32::INFINITY, f32::NAN] {
        for at in [24, 48] {
            let mut b = DENSITY.to_vec();
            b[at..at + 4].copy_from_slice(&value.to_le_bytes());
            if at == 24 && value == -1. {
                continue;
            }
            fail(&b, None, &p, "vol");
        }
    }
    let mut b = DENSITY.to_vec();
    b[36..40].copy_from_slice(&0f32.to_le_bytes());
    fail(&b, None, &p, "vol");
    let mut b = EMISSION.to_vec();
    b[36..40].copy_from_slice(&3f32.to_le_bytes());
    fail(DENSITY, Some(&b), &p, "vol");
    fail(DENSITY, Some(DENSITY), &p, "unsupported_vol");
    for i in 0..9 {
        let mut q = p.clone();
        match i {
            0 => q.meters_per_unit = 0.,
            1 => q.density_scale = -1.,
            2 => q.emission_scale[0] = f64::NAN,
            3 => q.absorption[0] = -1.,
            4 => q.anisotropy = 1.,
            5 => q.max_step_meters = 0.,
            6 => q.meters_per_unit = 1e-9,
            7 => q.density_scale = 1e7,
            _ => q.meters_per_unit = f64::INFINITY,
        };
        let code = if [3, 4, 5, 6].contains(&i) {
            "volume"
        } else {
            "volume_policy"
        };
        fail(DENSITY, None, &q, code);
    }
    let mut b = DENSITY.to_vec();
    b[48..52].copy_from_slice(&(-1f32).to_le_bytes());
    let mut q = p.clone();
    q.density_scale = 0.;
    fail(&b, None, &q, "vol");
    let mut huge = SPARSE.to_vec();
    for chunk in huge[48..].chunks_exact_mut(4) {
        chunk.copy_from_slice(&1f32.to_le_bytes());
    }
    fail(&huge, None, &p, "budget");
    let mut b = DENSITY.to_vec();
    for at in [8, 12, 16] {
        b[at..at + 4].copy_from_slice(&i32::MAX.to_le_bytes());
    }
    fail(&b, None, &p, "budget");
    fail(&vec![0; MAX_INPUT_BYTES + 1], None, &p, "budget");
    let mut emission = EMISSION.to_vec();
    emission[52..56].copy_from_slice(&(-1f32).to_le_bytes());
    let mut scaled = p.clone();
    scaled.emission_scale = [0.; 3];
    fail(DENSITY, Some(&emission), &scaled, "vol");
    let mut limit = DENSITY[..48].to_vec();
    for (at, n) in [(8, 32_i32), (12, 32), (16, 16)] {
        limit[at..at + 4].copy_from_slice(&n.to_le_bytes());
    }
    for _ in 0..16384 {
        limit.extend_from_slice(&1f32.to_le_bytes());
    }
    assert_eq!(read(&limit, None, &p).report.occupied_cells, 16384);
    let mut scaled = p.clone();
    scaled.density_scale = 2.;
    scaled.emission_scale = [2., 3., 4.];
    let scaled = read(DENSITY, Some(EMISSION), &scaled);
    assert!(
        asset(&scaled)
            .cells
            .iter()
            .all(|c| c.density == 2. && c.emission == [0.5, 1.5, 4.])
    );
    for limit in [0, 1, 3, 1026] {
        let mut calls = 0;
        let e = import(SPARSE, None, Id(10100), &p, || {
            calls += 1;
            calls > limit
        })
        .err()
        .unwrap();
        assert_eq!(e.code, "cancelled");
    }
}
fn request(d: &Document) -> agent::Request {
    agent::Request {
        version: 0,
        operation: agent::Operation::ImportVol {
            bytes: DENSITY.to_vec(),
            emission_bytes: Some(EMISSION.to_vec()),
            policy: policy(),
            settings: fixtures::settings(),
            base_revision: d.snapshot().revision().unwrap(),
            idempotency_key: "volume:import:0001".into(),
        },
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn transactional_import_permissions_retry_stale_cancel_and_durable_recovery() {
    let d = Document::new(Snapshot::empty(Id(10100))).unwrap();
    let before = canonical(&d).unwrap();
    let q = request(&d);
    let p = fixtures::principal();
    let mut stale_empty = request(&d);
    if let agent::Operation::ImportVol { base_revision, .. } = &mut stale_empty.operation {
        *base_revision = "stale".into();
    }
    let mut rejected = agent::Session::new(d.clone());
    assert_eq!(
        rejected.dispatch(&p, stale_empty).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(canonical(rejected.document()).unwrap(), before);
    let mut session = agent::Session::new(d.clone());
    let mut store = storage::MemoryStore {
        fail_publication: true,
        ..Default::default()
    };
    assert_eq!(
        session
            .dispatch_durable(&mut store, &p, q.clone())
            .unwrap_err()
            .code,
        "storage"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    let mut readonly = p.clone();
    readonly.can_write = false;
    assert_eq!(
        session.dispatch(&readonly, q.clone()).unwrap_err().code,
        "permission"
    );
    let mut calls = 0;
    assert_eq!(
        session
            .dispatch_cancellable(&p, q.clone(), || {
                calls += 1;
                calls > 5
            })
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    store.fail_publication = false;
    let out = session.dispatch_durable(&mut store, &p, q.clone()).unwrap();
    assert_eq!(out["receipt"]["durable"], true);
    let after = canonical(session.document()).unwrap();
    session.dispatch_durable(&mut store, &p, q.clone()).unwrap();
    assert_eq!(after, canonical(session.document()).unwrap());
    let mut stale = q.clone();
    if let agent::Operation::ImportVol {
        idempotency_key, ..
    } = &mut stale.operation
    {
        *idempotency_key = "volume:stale:0001".into();
    }
    // Existing import target gate precedes candidate preparation for nonempty roots.
    assert_eq!(
        session.dispatch(&p, stale).unwrap_err().code,
        "import_target"
    );
    assert_eq!(after, canonical(session.document()).unwrap());
    let recovered = storage::recover(&store.records).unwrap().unwrap();
    assert_eq!(after, canonical(&recovered).unwrap());
    let scene = Evaluator::default().evaluate(recovered.snapshot()).unwrap();
    assert_eq!(scene.media.cell_count(), 8);
    let i = read(DENSITY, Some(EMISSION), &policy());
    let mut tx = fixtures::request(&d, "volume:budget:001", i.commands).unwrap();
    tx.max_added_bytes = 1;
    let mut d = d;
    assert_eq!(d.execute(&p, &tx).unwrap_err().code, "budget");
    assert_eq!(canonical(&d).unwrap(), before);
    tx.max_added_bytes = 8388608;
    tx.base_revision = "stale".into();
    assert_eq!(d.execute(&p, &tx).unwrap_err().code, "stale_revision");
}
