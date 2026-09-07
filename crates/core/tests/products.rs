use render_core::{document::*, products::*, render::*, *};
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn color_profiles_numeric_white_primaries_and_signed_roundtrip() {
    assert!(
        (convert([0.0031308; 3], ColorSpace::LinearSrgb, ColorSpace::Srgb).unwrap()[0]
            - 0.040449936)
            .abs()
            < 1e-9
    );
    let red = convert(
        [1., 0., 0.],
        ColorSpace::LinearSrgb,
        ColorSpace::LinearDisplayP3,
    )
    .unwrap();
    // Independently tabulated W3C CSS Color 4 XYZ matrices, unlike production's
    // matrix derivation from primary/white chromaticities.
    let xyz = [506752. / 1228815., 87098. / 409605., 7918. / 409605.];
    let matrix = [
        [446124. / 178915., -333277. / 357830., -72051. / 178915.],
        [-14852. / 17905., 63121. / 35810., 423. / 17905.],
        [11844. / 330415., -50337. / 660830., 316169. / 330415.],
    ];
    for c in 0..3 {
        let expected: f64 = matrix[c].iter().zip(xyz).map(|(a, b)| a * b).sum();
        assert!((red[c] - expected).abs() < 1e-12);
    }
    for space in [
        ColorSpace::LinearSrgb,
        ColorSpace::Srgb,
        ColorSpace::LinearDisplayP3,
        ColorSpace::DisplayP3,
    ] {
        let white = convert([1.; 3], ColorSpace::LinearSrgb, space).unwrap();
        assert!(white.iter().all(|x| (x - 1.).abs() < 1e-12));
        for input in [[-0.1, 0.5, 4.], [0.001, 0.002, 0.003], [0.9, 0.1, 0.4]] {
            let converted = convert(input, ColorSpace::Srgb, space).unwrap();
            let returned = convert(converted, space, ColorSpace::Srgb).unwrap();
            for c in 0..3 {
                assert!((input[c] - returned[c]).abs() < 1e-10);
            }
        }
    }
    assert_eq!(
        convert([f64::NAN, 0., 0.], ColorSpace::Srgb, ColorSpace::Srgb)
            .unwrap_err()
            .code,
        "color"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn denoiser_preserves_constant_regions_and_object_edges_reduces_variance() {
    let (scene, mut s, _) = fixtures::diffuse_plane().unwrap();
    s.width = 32;
    s.height = 16;
    s.samples = 1;
    let mut image = render(&scene, &s, || false).unwrap();
    let filter = Bilateral {
        radius: 2,
        spatial_sigma: 2.,
        luminance_sigma: 2.,
        normal_sigma: 1.,
        relative_depth_sigma: 1.,
    };
    for y in 0..16 {
        for x in 0..32 {
            let i = y * 32 + x;
            image.objects[i] = Some(Id(if x < 16 { 1 } else { 2 }));
            image.linear[i] = [if x < 16 { 0.2 } else { 0.8 }; 3];
        }
    }
    let constant = filter.filter(&image, || false).unwrap();
    for (a, b) in constant.iter().zip(&image.linear) {
        assert!((a[0] - b[0]).abs() < 1e-6);
    }
    for (i, p) in image.linear.iter_mut().enumerate() {
        let noise = (random(i as u32, 0, 20, 42) - 0.5) as f32 * 0.1;
        for c in p {
            *c += noise;
        }
    }
    let filtered = filter.filter(&image, || false).unwrap();
    let error = |pixels: &[[f32; 3]]| {
        pixels
            .iter()
            .enumerate()
            .map(|(i, p)| f64::from(p[0] - if i % 32 < 16 { 0.2 } else { 0.8 }).powi(2))
            .sum::<f64>()
    };
    assert!(error(&filtered) < error(&image.linear) * 0.2);
    assert!(filtered.iter().enumerate().all(|(i, p)| if i % 32 < 16 {
        p[0] < 0.25
    } else {
        p[0] > 0.75
    }));
    assert_eq!(
        filter.filter(&image, || true).unwrap_err().code,
        "cancelled"
    );
    image.depth.pop();
    assert_eq!(filter.filter(&image, || false).unwrap_err().code, "image");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn authored_views_passes_bakes_roundtrip_invalid_atlas_and_stale_requests() {
    let mut d = feature_fixtures::imaging_document().unwrap();
    let revision = d.snapshot().revision().unwrap();
    let before = canonical(&d).unwrap();
    let products = render_products(d.snapshot(), &revision, || false).unwrap();
    assert_eq!(products.views.len(), 2);
    assert!(products.views[0].raw.objects.contains(&Some(Id(7851))));
    assert!(!products.views[1].raw.objects.contains(&Some(Id(7851))));
    assert!(products.views[1].raw.objects.contains(&Some(Id(7850))));
    for bake in products.bakes.values() {
        assert!(bake.covered.iter().all(|x| *x));
        let expected = if bake.pass == BakePass::Albedo {
            [0.7, 0.2, 0.05]
        } else {
            [0., 0., 1.]
        };
        assert!(bake.rgb.iter().all(|p| *p == expected));
    }
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
    assert_eq!(canonical(&d).unwrap(), before);
    assert_eq!(
        render_products(d.snapshot(), "stale", || false)
            .unwrap_err()
            .code,
        "stale_revision"
    );
    assert_eq!(
        render_products(d.snapshot(), &revision, || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    let mut scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let inst = scene
        .instances
        .iter_mut()
        .find(|i| i.id == Id(7850))
        .unwrap();
    let geo = std::sync::Arc::make_mut(&mut inst.geometry);
    geo.triangles = geo
        .triangles
        .iter()
        .chain(std::iter::once(&geo.triangles[0]))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    let request = &d.snapshot().imaging.as_ref().unwrap().bakes[0];
    assert_eq!(
        bake(&scene, request, &revision, || false).unwrap_err().code,
        "bake_overlap"
    );
    let mut req = fixtures::request(
        &d,
        "imaging:stale:001",
        vec![Command::SetImaging { imaging: None }],
    )
    .unwrap();
    req.base_revision = "stale".into();
    assert_eq!(
        d.execute(&fixtures::principal(), &req).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(canonical(&d).unwrap(), before);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn world_normal_bake_tracks_reflected_winding() {
    let d = feature_fixtures::imaging_document().unwrap();
    let revision = d.snapshot().revision().unwrap();
    let mut scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let request = d
        .snapshot()
        .imaging
        .as_ref()
        .unwrap()
        .bakes
        .iter()
        .find(|b| b.pass == BakePass::WorldNormal)
        .unwrap();
    let instance = scene
        .instances
        .iter_mut()
        .find(|i| i.id == request.entity)
        .unwrap();
    instance.transform = glam::DAffine3::from_scale(glam::DVec3::new(-1., 1., 1.));
    instance.inverse = instance.transform.inverse();
    let result = bake(&scene, request, &revision, || false).unwrap();
    assert!(
        result
            .rgb
            .iter()
            .zip(result.covered)
            .filter(|(_, covered)| *covered)
            .all(|(n, _)| n[2] == -1.)
    );
}
