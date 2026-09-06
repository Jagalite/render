use glam::{DAffine3, DVec3};
use render_core::{document::*, render::*, volumes::*, *};
fn slab() -> Asset {
    Asset {
        origin: [0.; 3],
        voxel_size: [1.; 3],
        cells: vec![Cell {
            coordinate: [0; 3],
            density: 1.,
            emission: [0.2, 0.4, 0.6],
        }],
        absorption: [0.5, 1., 2.],
        scattering: [0.; 3],
        anisotropy: 0.,
        max_step_meters: 0.1,
    }
}
fn ray() -> Ray {
    Ray {
        origin: DVec3::new(0.5, 0.5, -1.),
        direction: DVec3::Z,
    }
}
fn media(a: &Asset) -> Media {
    Media::build([(Id(1), a, DAffine3::IDENTITY)], || false).unwrap()
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn beer_emission_overlap_scale_and_sparse_boundaries() {
    let a = slab();
    let m = media(&a);
    let result = m
        .transport(
            ray(),
            0.,
            10.,
            |_, _| panic!("absorption/emission needs no scattering samples"),
            || false,
        )
        .unwrap();
    for i in 0..3 {
        let t = (-a.absorption[i]).exp();
        assert!((result.transmittance[i] - t).abs() < 1e-13);
        assert!(
            (result.radiance[i] - a.cells[0].emission[i] * (1. - t) / a.absorption[i]).abs()
                < 1e-13
        );
    }
    let overlap = Media::build(
        [
            (Id(1), &a, DAffine3::IDENTITY),
            (Id(2), &a, DAffine3::IDENTITY),
        ],
        || false,
    )
    .unwrap();
    let tr = overlap.transmittance(ray(), 0., 10., || false).unwrap();
    assert!((tr - result.transmittance * result.transmittance).length() < 1e-13);
    let scaled = Media::build(
        [(Id(1), &a, DAffine3::from_scale(DVec3::new(1., 1., 2.)))],
        || false,
    )
    .unwrap();
    assert!((tr - scaled.transmittance(ray(), 0., 10., || false).unwrap()).length() < 1e-13);
    let mut sparse = a.clone();
    sparse.cells.push(Cell {
        coordinate: [1, 0, 0],
        density: 3.,
        emission: [0.; 3],
    });
    sparse.cells.push(Cell {
        coordinate: [0, 0, 2],
        density: 2.,
        emission: [0.; 3],
    });
    let m = media(&sparse);
    // Ray exactly on an internal face belongs to x=1, never both neighboring cells.
    let boundary = Ray {
        origin: DVec3::new(1., 0.5, -1.),
        direction: DVec3::Z,
    };
    assert!(
        (m.transmittance(boundary, 0., 10., || false).unwrap().x - (-1.5f64).exp()).abs() < 1e-13
    );
    // Empty z=1 remains vacuum; only two occupied intervals contribute.
    assert!((m.transmittance(ray(), 0., 10., || false).unwrap().z - (-6f64).exp()).abs() < 1e-13);
    let mut vacuum = slab();
    vacuum.absorption = [0.; 3];
    let v = media(&vacuum)
        .transport(ray(), 0., 10., |_, _| panic!(), || false)
        .unwrap();
    assert_eq!(v.transmittance, DVec3::ONE);
    assert!((v.radiance - DVec3::from_array(vacuum.cells[0].emission)).length() < 1e-13);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn phase_normalization_and_single_scatter_quadrature_convergence() {
    for g in [-0.8, 0., 0.8] {
        let n = 20000;
        let sum: f64 = (0..n)
            .map(|i| phase(-1. + 2. * (f64::from(i) + 0.5) / f64::from(n), g))
            .sum();
        assert!((sum * 4. * std::f64::consts::PI / f64::from(n) - 1.).abs() < 1e-5);
    }
    let mut a = slab();
    a.cells[0].emission = [0.; 3];
    a.absorption = [0.; 3];
    a.scattering = [0.5; 3];
    let isotropic = media(&a)
        .transport(
            ray(),
            0.,
            10.,
            |_, _| Ok((DVec3::splat(2.), DVec3::Z)),
            || false,
        )
        .unwrap();
    let expected = 2. * (1. - (-0.5f64).exp()) / (4. * std::f64::consts::PI);
    assert!((isotropic.radiance.x - expected).abs() < 1e-13);
    // Smooth inverse-square illumination, independently integrated by very fine
    // scalar midpoint rule (includes exact camera transmittance at the midpoint).
    let exact = (0..100000)
        .map(|i| {
            let z = (f64::from(i) + 0.5) / 100000.;
            (-0.5 * z).exp() * 0.5 / (4. * std::f64::consts::PI * (2. - z).powi(2)) / 100000.
        })
        .sum::<f64>();
    let mut errors = vec![];
    for step in [0.25, 0.125, 0.0625] {
        a.max_step_meters = step;
        let r = media(&a)
            .transport(
                ray(),
                0.,
                10.,
                |p, _| Ok((DVec3::splat(1. / (2. - p.z).powi(2)), DVec3::Z)),
                || false,
            )
            .unwrap();
        errors.push((r.radiance.x - exact).abs());
    }
    assert!(
        errors[1] < errors[0] * 0.3 && errors[2] < errors[1] * 0.3,
        "{errors:?}"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn media_invalid_input_budget_and_cancellation() {
    let mut a = slab();
    a.cells.push(a.cells[0].clone());
    assert_eq!(a.validate().unwrap_err().code, "duplicate_id");
    a = slab();
    a.cells[0].density = -1.;
    assert_eq!(a.validate().unwrap_err().code, "volume");
    a = slab();
    a.anisotropy = 1.;
    assert!(a.validate().is_err());
    assert_eq!(
        Media::build([(Id(1), &slab(), DAffine3::IDENTITY)], || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert_eq!(
        media(&slab())
            .transmittance(ray(), 0., 10., || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    let mut a = slab();
    a.scattering = [1.; 3];
    a.max_step_meters = 1e-5;
    assert_eq!(
        media(&a)
            .transport(ray(), 0., 10., |_, _| Ok((DVec3::ONE, DVec3::Z)), || false)
            .unwrap_err()
            .code,
        "budget"
    );
    a.max_step_meters = 0.01;
    let mut calls = 0;
    assert_eq!(
        media(&a)
            .transport(
                ray(),
                0.,
                10.,
                |_, _| Ok((DVec3::ONE, DVec3::Z)),
                || {
                    calls += 1;
                    calls == 8
                }
            )
            .unwrap_err()
            .code,
        "cancelled"
    );
    let bad = Ray {
        direction: DVec3::Z * 2.,
        ..ray()
    };
    assert_eq!(
        media(&a)
            .transmittance(bad, 0., 10., || false)
            .unwrap_err()
            .code,
        "ray"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn volume_document_roundtrip_transactions_visibility_and_render() {
    let mut d = feature_fixtures::volume_document().unwrap();
    assert_eq!(d.snapshot().version, 4);
    let before = canonical(&d).unwrap();
    let restored = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(before, canonical(&restored).unwrap());
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    assert!(scene.media.cell_count() > 0);
    let mut s = d.snapshot().render_settings.clone().unwrap();
    s.width = 16;
    s.height = 16;
    s.samples = 1;
    let image = render(&scene, &s, || false).unwrap();
    assert!(image.receipt.approximation.contains("Beer"));
    assert!(image.linear.iter().any(|p| (p[0] - p[2]).abs() > 0.01));
    assert_eq!(before, canonical(&d).unwrap());
    let mut req = fixtures::request(
        &d,
        "volume:stale:0001",
        vec![Command::SetVolume {
            entity: Id(7400),
            asset: None,
        }],
    )
    .unwrap();
    req.base_revision = "stale".into();
    assert_eq!(
        d.execute(&fixtures::principal(), &req).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(before, canonical(&d).unwrap());
    let req = fixtures::request(
        &d,
        "volume:missing:01",
        vec![Command::SetVolume {
            entity: Id(7400),
            asset: Some("missing".into()),
        }],
    )
    .unwrap();
    assert_eq!(
        d.execute(&fixtures::principal(), &req).unwrap_err().code,
        "reference"
    );
    assert_eq!(before, canonical(&d).unwrap());
    let req = fixtures::request(
        &d,
        "volume:hidden:01",
        vec![Command::PutLayer {
            layer: Layer {
                name: "hide media".into(),
                overrides: std::collections::BTreeMap::from([(
                    Id(7400),
                    Override {
                        transform: None,
                        material: None,
                        hidden: true,
                    },
                )]),
            },
        }],
    )
    .unwrap();
    d.execute(&fixtures::principal(), &req).unwrap();
    assert!(
        Evaluator::default()
            .evaluate(d.snapshot())
            .unwrap()
            .media
            .is_empty()
    );
    let mut old = d.snapshot().clone();
    old.version = 3;
    assert_eq!(old.validate().unwrap_err().code, "schema_version");
}
