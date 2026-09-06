use render_core::{document::*, gltf_scene::*, render::*, scattering::*, *};
use serde_json::{Value, json};

const SOURCE: &[u8] = include_bytes!("../../../fixtures/alpha-gltf/data/mask.gltf");
const BIN: &[u8] = include_bytes!("../../../fixtures/alpha-gltf/data/mask.bin");
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 16;
    s.height = 8;
    s.samples = 128;
    s.max_depth = 1;
    s.camera.position = [0., 0., 3.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 1.,
        ymag: 1.,
        near: 0.1,
        far: 5.,
    });
    s.light.intensity = [0.; 3];
    s.environment = [0.; 3];
    s
}
fn imported(root: &Value) -> Result<Imported> {
    import_pbr_scene(
        &serde_json::to_vec(root).unwrap(),
        &[BIN.to_vec()],
        &[],
        Id(9800),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
}
fn document(root: &Value) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(9800))).unwrap();
    let mut commands = imported(root).unwrap().commands;
    commands.push(Command::SetRenderSettings {
        settings: settings(),
    });
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "alpha:fixture:0001", commands).unwrap(),
    )
    .unwrap();
    d
}
fn draw(d: &Document, s: &Settings) -> Image {
    render_core::render::render(
        &Evaluator::default().evaluate(d.snapshot()).unwrap(),
        s,
        || false,
    )
    .unwrap()
}
fn source() -> Value {
    serde_json::from_slice(SOURCE).unwrap()
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn imported_linear_alpha_mask_blend_and_opaque_have_analytic_coverage() {
    for mode in ["MASK", "BLEND", "OPAQUE"] {
        let mut root = source();
        root["materials"][0]["alphaMode"] = json!(mode);
        if mode == "BLEND" {
            root["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(0.5);
        }
        let d = document(&root);
        let before = canonical(&d).unwrap();
        let image = draw(&d, &settings());
        for stripe in 0..4 {
            let pixels: Vec<_> = image
                .linear
                .iter()
                .enumerate()
                .filter(|(i, _)| (i % 16) / 4 == stripe)
                .map(|(_, p)| p)
                .collect();
            let expected = match mode {
                "MASK" => {
                    if stripe >= 2 {
                        1.
                    } else {
                        0.
                    }
                }
                "BLEND" => stripe as f64 / 6.,
                _ => 1.,
            };
            let mean = pixels.iter().map(|p| f64::from(p[1])).sum::<f64>() / pixels.len() as f64;
            assert!(
                (mean - expected).abs() < 0.03,
                "{mode} stripe {stripe}: {mean} vs {expected}"
            );
            for p in pixels {
                assert_eq!(p[0] + p[1], 1.);
                assert_eq!(p[2], 0.);
            }
        }
        if mode != "BLEND" {
            for (i, &depth) in image.depth.iter().enumerate() {
                assert_eq!(
                    depth,
                    if mode == "MASK" && (i % 16) < 8 {
                        3.
                    } else {
                        2.
                    }
                );
            }
        }
        let restored = storage::Envelope::new(0, None, &d)
            .unwrap()
            .decode()
            .unwrap();
        assert_eq!(canonical(&restored).unwrap(), before);
        assert_eq!(draw(&restored, &settings()).linear, image.linear);
        assert_eq!(canonical(&d).unwrap(), before);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn alpha_factor_cutoff_boundaries_and_defaults_are_preserved() {
    for (mode, factor, cutoff, coverage) in [
        ("MASK", 0., 0., 1.),
        ("MASK", 0.5, 0.5, 1.),
        ("MASK", 0.5, 0.50001, 0.),
        ("MASK", 1., 1., 1.),
        ("MASK", 1., 2., 0.),
        ("MASK", 1., f64::MAX, 0.),
        ("BLEND", 0., 2., 0.),
        ("BLEND", 1., 2., 1.),
        ("OPAQUE", 0., 2., 1.),
    ] {
        let mut root = source();
        let m = &mut root["materials"][0];
        m["alphaMode"] = json!(mode);
        m["alphaCutoff"] = json!(cutoff);
        m["pbrMetallicRoughness"]
            .as_object_mut()
            .unwrap()
            .remove("baseColorTexture");
        m["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(factor);
        let d = document(&root);
        assert!(
            draw(&d, &settings())
                .linear
                .iter()
                .all(|p| *p == [1. - coverage, coverage, 0.])
        );
        if mode == "MASK" {
            assert!(d.snapshot().materials.values().any(|m| {
                m.pbr
                    .as_ref()
                    .and_then(|p| p.advanced.as_ref())
                    .is_some_and(|s| s.opacity == Opacity::Mask { factor, cutoff })
            }));
        }
    }
    let mut root = source();
    root["materials"][0]["pbrMetallicRoughness"]
        .as_object_mut()
        .unwrap()
        .remove("baseColorFactor");
    let d = document(&root);
    assert!(d.snapshot().materials.values().any(|m| {
        m.pbr
            .as_ref()
            .and_then(|p| p.advanced.as_ref())
            .is_some_and(|s| {
                s.opacity
                    == Opacity::Mask {
                        factor: 1.,
                        cutoff: 0.5,
                    }
            })
    }));
    root["materials"][0]
        .as_object_mut()
        .unwrap()
        .remove("alphaMode");
    assert!(
        document(&root).snapshot().materials.values().all(|m| m
            .pbr
            .as_ref()
            .unwrap()
            .advanced
            .is_none())
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn alpha_continuation_preserves_camera_clip_and_orthographic_primary_depth() {
    let mut root = source();
    root["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(0.);
    let d = document(&root);
    let mut s = settings();
    s.samples = 1;
    assert!(draw(&d, &s).depth.iter().all(|d| *d == 3.));
    for lens in [
        cameras::Lens::Orthographic {
            xmag: 0.8,
            ymag: 0.8,
            near: 0.1,
            far: 2.5,
        },
        cameras::Lens::Perspective {
            vertical_fov_radians: 0.5,
            aspect_ratio: Some(1.),
            near: 0.1,
            far: Some(2.5),
        },
    ] {
        s.camera.lens = Some(lens);
        let image = draw(&d, &s);
        assert!(image.objects.iter().all(Option::is_none));
        assert!(image.linear.iter().all(|p| *p == [0.; 3]));
        assert!(image.depth.iter().all(|d| *d == 0.));
    }
    // A plane before the near clip is ignored even if its material is opaque.
    root["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(1.);
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 0.8,
        ymag: 0.8,
        near: 2.5,
        far: 5.,
    });
    assert!(
        draw(&document(&root), &s)
            .linear
            .iter()
            .all(|p| *p == [1., 0., 0.])
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn alpha_invalid_inputs_and_failed_transactions_leave_document_unchanged() {
    let mut d = document(&source());
    let before = canonical(&d).unwrap();
    for (field, value) in [
        ("alphaMode", json!(null)),
        ("alphaMode", json!(2)),
        ("alphaMode", json!("OTHER")),
        ("alphaCutoff", json!(-0.1)),
        ("alphaCutoff", json!("0.5")),
        ("alphaCutoff", json!(null)),
        ("doubleSided", json!(1)),
    ] {
        let mut root = source();
        root["materials"][0][field] = value;
        assert!(imported(&root).is_err(), "{field}");
    }
    let mut root = source();
    root["materials"][0]
        .as_object_mut()
        .unwrap()
        .remove("alphaMode");
    root["materials"][0]["alphaCutoff"] = json!(0.5);
    assert!(imported(&root).is_err());
    for factor in [-0.01, 1.01] {
        let mut root = source();
        root["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(factor);
        assert!(imported(&root).is_err());
    }
    for cutoff in [-0.1, f64::INFINITY, f64::NAN] {
        assert!(
            Surface {
                model: Model::Principled,
                opacity: Opacity::Mask { factor: 1., cutoff }
            }
            .validate()
            .is_err()
        );
    }
    let mut material = d
        .snapshot()
        .materials
        .values()
        .find(|m| m.pbr.as_ref().unwrap().advanced.is_some())
        .unwrap()
        .clone();
    material
        .pbr
        .as_mut()
        .unwrap()
        .advanced
        .as_mut()
        .unwrap()
        .opacity = Opacity::Blend { factor: 2. };
    let request = fixtures::request(
        &d,
        "alpha:invalid:0001",
        vec![Command::PutMaterial {
            material: Box::new(material),
        }],
    )
    .unwrap();
    assert_eq!(
        d.execute(&fixtures::principal(), &request)
            .unwrap_err()
            .code,
        "material"
    );
    assert_eq!(canonical(&d).unwrap(), before);
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let mut calls = 0;
    assert_eq!(
        render_core::render::render(&scene, &settings(), || {
            calls += 1;
            calls > 4
        })
        .unwrap_err()
        .code,
        "cancelled"
    );
    assert_eq!(canonical(&d).unwrap(), before);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn alpha_layers_and_single_double_sided_backfaces_preserve_coverage() {
    let mut root = source();
    root["materials"][0]["alphaMode"] = json!("BLEND");
    root["materials"][0]["pbrMetallicRoughness"]
        .as_object_mut()
        .unwrap()
        .remove("baseColorTexture");
    root["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(0.5);
    root["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"mesh":0,"translation":[0,0,0.5]}));
    root["scenes"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!(2));
    let image = draw(&document(&root), &settings());
    let green =
        image.linear.iter().map(|p| f64::from(p[1])).sum::<f64>() / image.linear.len() as f64;
    assert!((green - 0.75).abs() < 0.02, "two layers over red: {green}");
    // Turn both green quads around: culling ignores them; double-sided restores them.
    for i in [0, 2] {
        root["nodes"][i]["rotation"] = json!([0, 1, 0, 0]);
    }
    assert!(
        draw(&document(&root), &settings())
            .linear
            .iter()
            .all(|p| *p == [1., 0., 0.])
    );
    root["materials"][0]["doubleSided"] = json!(true);
    let image = draw(&document(&root), &settings());
    let green =
        image.linear.iter().map(|p| f64::from(p[1])).sum::<f64>() / image.linear.len() as f64;
    assert!((green - 0.75).abs() < 0.02);
    assert!(image.normals.iter().all(|n| *n == [0., 0., 1.]));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exhausted_alpha_camera_interval_with_media_returns_environment() {
    let mut root = source();
    root["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(0.);
    let d = document(&root);
    let mut scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let medium = volumes::Asset {
        origin: [10.; 3],
        voxel_size: [1.; 3],
        cells: vec![volumes::Cell {
            coordinate: [0; 3],
            density: 1.,
            emission: [0.; 3],
        }],
        absorption: [1.; 3],
        scattering: [0.; 3],
        anisotropy: 0.,
        max_step_meters: 0.1,
    };
    scene.media =
        volumes::Media::build([(Id(99), &medium, glam::DAffine3::IDENTITY)], || false).unwrap();
    let mut s = settings();
    s.samples = 1;
    s.environment = [0.25; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 0.8,
        ymag: 0.8,
        near: 0.1,
        far: 2.,
    });
    let image = render_core::render::render(&scene, &s, || false).unwrap();
    assert!(image.linear.iter().all(|p| *p == [0.25; 3]));
    assert!(image.objects.iter().all(Option::is_none));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn initial_perspective_near_clip_is_not_replaced_by_continuation_epsilon() {
    let mut root = source();
    root["materials"][0]["alphaCutoff"] = json!(0.);
    let d = document(&root);
    let mut s = settings();
    s.samples = 1;
    s.camera.position = [0., 0., 1.000002];
    s.camera.lens = Some(cameras::Lens::Perspective {
        vertical_fov_radians: 0.5,
        aspect_ratio: Some(1.),
        near: 1e-6,
        far: Some(2.),
    });
    let image = draw(&d, &s);
    assert!(image.linear.iter().all(|p| *p == [0., 1., 0.]));
    assert!(image.depth.iter().all(|d| *d < 1e-5));
}
