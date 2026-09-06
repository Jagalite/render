use render_core::{document::*, hair_import::*, *};
const LE: &[u8] = include_bytes!("../../../fixtures/hair-import/data/default-le.hair");
const BE: &[u8] = include_bytes!("../../../fixtures/hair-import/data/default-be.hair");
const ARRAY: &[u8] = include_bytes!("../../../fixtures/hair-import/data/arrays-le.hair");
const ARRAY_BE: &[u8] = include_bytes!("../../../fixtures/hair-import/data/arrays-be.hair");
fn policy() -> Policy {
    Policy {
        byte_order: ByteOrder::LittleEndian,
        point_attributes: None,
        meters_per_unit: 1.,
        thickness: Thickness::Diameter,
        color_space: ColorSpace::LinearSrgb,
        transparency: Transparency::CoverageOneMinusTransparency,
        representation: Representation::SweptPolylineSurface,
        roughness: 0.6,
        tessellation: curves::Tessellation {
            chord_error: 0.001,
            radial_error: 0.01,
            max_samples: 8192,
            max_vertices: 65536,
            max_depth: 18,
        },
    }
}
fn read(bytes: &[u8], p: &Policy) -> Imported {
    import(bytes, Id(10200), p, || false).unwrap()
}
fn assets(i: &Imported) -> Vec<&curves::Asset> {
    i.commands
        .iter()
        .filter_map(|c| {
            if let Command::PutGeometry { asset } = c {
                Some(asset)
            } else {
                None
            }
        })
        .collect()
}
fn fail(bytes: &[u8], p: &Policy, code: &str) {
    let e = import(bytes, Id(10200), p, || false).err().unwrap();
    assert_eq!(e.code, code, "{e}");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn endian_defaults_arrays_units_materials_and_native_geometry_oracle() {
    let p = policy();
    let le = read(LE, &p);
    let mut big = p.clone();
    big.byte_order = ByteOrder::BigEndian;
    let be = read(BE, &big);
    assert_eq!(le.report.source_strands, 2);
    assert_eq!(le.report.source_points, 6);
    assert_eq!(le.report.material_groups, 1);
    let a = assets(&le)[0];
    let b = assets(&be)[0];
    assert_eq!(
        canonical(&a.evaluate(|| false).unwrap().mesh).unwrap(),
        canonical(&b.evaluate(|| false).unwrap().mesh).unwrap()
    );
    let positions = [
        [[-0.4f32, -0.75, 0.], [-0.35, 0., 0.1], [-0.25, 0.75, 0.]],
        [[0.35, -0.7, 0.], [0.4, 0., 0.], [0.45, 0.7, 0.]],
    ];
    let oracle = curves::Asset {
        shape: curves::Shape::Curves {
            curves: positions
                .iter()
                .enumerate()
                .map(|(i, points)| curves::Curve {
                    id: Id(i as u128 + 1),
                    basis: curves::Basis::Polyline,
                    controls: points
                        .iter()
                        .enumerate()
                        .map(|(j, xyz)| curves::Control {
                            id: Id(j as u128 + 10),
                            position: xyz.map(f64::from),
                            radius: f64::from(0.12f32) / 2.,
                            tilt: 0.,
                            color: None,
                        })
                        .collect(),
                    closed: false,
                })
                .collect(),
        },
        tessellation: p.tessellation.clone(),
    };
    assert_eq!(
        canonical(&a.evaluate(|| false).unwrap().mesh).unwrap(),
        canonical(&oracle.evaluate(|| false).unwrap().mesh).unwrap()
    );
    for (bytes, p) in [(ARRAY, p.clone()), (ARRAY_BE, big)] {
        let i = read(bytes, &p);
        assert_eq!(i.report.material_groups, 2);
        assert_eq!(i.report.source_points, 7);
        let mut points = 0;
        for asset in assets(&i) {
            let curves::Shape::Curves { curves } = &asset.shape else {
                panic!()
            };
            for c in curves {
                points += c.controls.len();
                assert!(c.controls.windows(2).any(|w| w[0].radius != w[1].radius));
            }
        }
        assert_eq!(points, 7);
        let material = i
            .commands
            .iter()
            .find_map(|c| {
                if let Command::PutMaterial { material } = c {
                    if material.base_color[0] > 0.5 {
                        Some(material)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap();
        assert!(matches!(
            material
                .pbr
                .as_ref()
                .unwrap()
                .advanced
                .as_ref()
                .unwrap()
                .opacity,
            scattering::Opacity::Blend { factor: 0.75 }
        ));
    }
    let mut scaled = p.clone();
    scaled.meters_per_unit = 2.;
    scaled.thickness = Thickness::Radius;
    scaled.color_space = ColorSpace::Srgb;
    let i = read(LE, &scaled);
    let curves::Shape::Curves { curves } = &assets(&i)[0].shape else {
        panic!()
    };
    assert_eq!(
        curves[0].controls[0].position,
        positions[0][0].map(|v| f64::from(v) * 2.)
    );
    assert_eq!(curves[0].controls[0].radius, f64::from(0.12f32) * 2.);
    let material = i
        .commands
        .iter()
        .find_map(|c| {
            if let Command::PutMaterial { material } = c {
                Some(material)
            } else {
                None
            }
        })
        .unwrap();
    assert!((material.base_color[0] - 0.21404114).abs() < 1e-7);
    assert_eq!(
        canonical(&le.commands).unwrap(),
        canonical(&read(LE, &p).commands).unwrap()
    );
    // The source information block is hashed but never evaluated as commands.
    let mut meta = LE.to_vec();
    meta[40..128].fill(0xff);
    assert_eq!(
        assets(&read(&meta, &p))[0]
            .evaluate(|| false)
            .unwrap()
            .mesh
            .positions
            .len(),
        a.evaluate(|| false).unwrap().mesh.positions.len()
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn malformed_unsupported_and_bounded_sweep_failures() {
    let p = policy();
    for n in [0, 4, 127, 128, LE.len() - 1] {
        fail(&LE[..n], &p, "hair");
    }
    let mut b = LE.to_vec();
    b.push(0);
    fail(&b, &p, "hair");
    for (at, value, code) in [
        (0, 0, "hair"),
        (12, 0, "unsupported_hair"),
        (12, 34, "unsupported_hair"),
        (4, 0, "budget"),
        (16, 0, "hair"),
        (16, 1, "hair"),
    ] {
        let mut b = LE.to_vec();
        b[at] = value;
        fail(&b, &p, code);
    }
    for at in [20, 24, 28, 128] {
        let mut b = LE.to_vec();
        b[at..at + 4].copy_from_slice(&f32::NAN.to_le_bytes());
        fail(
            &b,
            &p,
            if [20, 128].contains(&at) {
                "curve"
            } else {
                "hair"
            },
        );
    }
    let mut b = LE.to_vec();
    b[20..24].copy_from_slice(&0f32.to_le_bytes());
    fail(&b, &p, "curve");
    let mut b = LE.to_vec();
    let point = b[128..140].to_vec();
    b[140..152].copy_from_slice(&point);
    fail(&b, &p, "curve");
    // Array offsets: 128-byte header, two u16 counts, seven XYZ points, thickness,
    // transparency, RGB. Variation within a strand is explicitly unsupported.
    for at in [132 + 84 + 28 + 4, 132 + 84 + 56 + 12] {
        let mut b = ARRAY.to_vec();
        b[at..at + 4].copy_from_slice(&0.5f32.to_le_bytes());
        fail(&b, &p, "unsupported_hair");
    }
    let mut b = ARRAY.to_vec();
    b[20..40].fill(0xff);
    assert_eq!(read(&b, &p).report.source_points, 7);
    let mut q = p.clone();
    q.meters_per_unit = 0.;
    fail(LE, &q, "hair_policy");
    q = p.clone();
    q.roughness = 2.;
    fail(LE, &q, "hair_policy");
    q = p.clone();
    q.tessellation.max_vertices = 6;
    fail(LE, &q, "budget");
    q = p.clone();
    q.tessellation.radial_error = 1e-20;
    fail(LE, &q, "budget");
    fail(&vec![0; MAX_INPUT_BYTES + 1], &p, "budget");
    for groups in [128_u32, 129] {
        let points = groups * 3;
        let mut bytes = LE[..128].to_vec();
        for (at, value) in [(4, groups), (8, points), (12, 18), (16, 2)] {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        for strand in 0..groups {
            for y in [0f32, 0.5, 1.] {
                for value in [strand as f32, y, 0.] {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
        for strand in 0..groups {
            for _ in 0..3 {
                for value in [strand as f32 / 128., 0., 0.] {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
        let mut q = p.clone();
        q.tessellation.radial_error = 0.000005;
        let e = import(&bytes, Id(10200), &q, || false).err().unwrap();
        assert_eq!(e.code, "budget");
        assert!(
            e.message.contains(if groups == 128 {
                "aggregate sweep"
            } else {
                "material groups"
            }),
            "{e}"
        );
    }
    for at in [0, 2, 5, 10, 20] {
        let mut count = 0;
        let e = import(ARRAY, Id(10200), &p, || {
            count += 1;
            count > at
        })
        .err()
        .unwrap();
        assert_eq!(e.code, "cancelled");
    }
}
fn request(d: &Document) -> agent::Request {
    agent::Request {
        version: 0,
        operation: agent::Operation::ImportHair {
            bytes: ARRAY.to_vec(),
            policy: policy(),
            settings: fixtures::settings(),
            base_revision: d.snapshot().revision().unwrap(),
            idempotency_key: "hair:import:0001".into(),
        },
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn transactions_permissions_stale_cancel_durable_recovery_and_geometry() {
    let d = Document::new(Snapshot::empty(Id(10200))).unwrap();
    let before = canonical(&d).unwrap();
    let p = fixtures::principal();
    let q = request(&d);
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
    let mut reader = p.clone();
    reader.can_write = false;
    assert_eq!(
        session.dispatch(&reader, q.clone()).unwrap_err().code,
        "permission"
    );
    let mut stale = q.clone();
    if let agent::Operation::ImportHair { base_revision, .. } = &mut stale.operation {
        *base_revision = "stale".into();
    }
    assert_eq!(
        session.dispatch(&p, stale).unwrap_err().code,
        "stale_revision"
    );
    let mut calls = 0;
    assert_eq!(
        session
            .dispatch_cancellable(&p, q.clone(), || {
                calls += 1;
                calls > 8
            })
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    store.fail_publication = false;
    session.dispatch_durable(&mut store, &p, q.clone()).unwrap();
    let saved = canonical(session.document()).unwrap();
    session.dispatch_durable(&mut store, &p, q).unwrap();
    assert_eq!(saved, canonical(session.document()).unwrap());
    let restored = storage::recover(&store.records).unwrap().unwrap();
    assert_eq!(saved, canonical(&restored).unwrap());
    let scene = render::Evaluator::default()
        .evaluate(restored.snapshot())
        .unwrap();
    assert!(!scene.instances.is_empty());
    let mut tx =
        fixtures::request(&d, "hair:budget:0001", read(ARRAY, &policy()).commands).unwrap();
    tx.max_added_bytes = 1;
    let mut d = d;
    assert_eq!(d.execute(&p, &tx).unwrap_err().code, "budget");
    assert_eq!(canonical(&d).unwrap(), before);
}
