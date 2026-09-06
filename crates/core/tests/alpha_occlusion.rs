use image::ImageEncoder;
use render_core::{document::*, render::*, scattering::*, textures::*, *};
fn document(occlusion: u8, strength: f32, model: Model, opacity: Opacity) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(10050))).unwrap();
    let imported = gltf_scene::import_pbr_glb(
        include_bytes!("../../../fixtures/named-uv/data/roles.glb"),
        Id(10050),
        &gltf_scene::PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "alpha:ao:import:0001", imported.commands).unwrap(),
    )
    .unwrap();
    let mut bytes = vec![];
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            &[occlusion, 255, 255, 255],
            1,
            1,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    let image = ImageAsset::from_encoded(Mime::Png, bytes).unwrap();
    let mut material = d
        .snapshot()
        .materials
        .values()
        .find(|m| m.pbr.as_ref().is_some_and(|p| p.base_color.is_some()))
        .unwrap()
        .clone();
    material.base_color = [0.5; 3];
    material.emission = [0.; 3];
    material.metallic = 0.;
    material.roughness = if matches!(model, Model::Dielectric { .. }) {
        0.
    } else {
        0.5
    };
    material.pbr = Some(pbr::Surface {
        advanced: Some(scattering::Surface { model, opacity }),
        occlusion: Some(Binding {
            image: image.content_id().unwrap(),
            role: TextureRole::LinearData,
            sampler: Sampler::default(),
            uv_attribute: Some(Id(2)),
        }),
        occlusion_strength: strength,
        ..Default::default()
    });
    d.execute(
        &fixtures::principal(),
        &fixtures::request(
            &d,
            "alpha:ao:material:01",
            vec![
                Command::PutImage { image },
                Command::PutMaterial {
                    material: Box::new(material),
                },
            ],
        )
        .unwrap(),
    )
    .unwrap();
    d
}
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 8;
    s.height = 8;
    s.samples = 16;
    s.max_depth = 1;
    s.camera.position = [0., 0., 3.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 1.,
        ymag: 1.,
        near: 0.1,
        far: 5.,
    });
    s.environment = [1.; 3];
    s.light.intensity = [0.; 3];
    s
}
fn draw(d: &Document, s: &Settings) -> Image {
    render_core::render::render(
        &Evaluator::default().evaluate(d.snapshot()).unwrap(),
        s,
        || false,
    )
    .unwrap()
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn extended_surface_occlusion_scales_only_indirect_transport() {
    let models = [
        Model::Principled,
        Model::Coated {
            weight: 0.5,
            ior: 1.5,
            roughness: 0.2,
        },
        Model::Conductor {
            eta: [0.2; 3],
            k: [3.; 3],
        },
        Model::Dielectric { ior: 1.5 },
    ];
    for model in models {
        let full = document(255, 1., model.clone(), Opacity::Opaque);
        let reference = draw(&full, &settings());
        assert!(reference.linear.iter().flatten().any(|v| *v > 0.05));
        for (red, strength, factor) in [
            (0, 1., 0.),
            (0, 0.5, 0.5),
            (128, 1., 128. / 255.),
            (0, 0., 1.),
        ] {
            let d = document(red, strength, model.clone(), Opacity::Opaque);
            let before = canonical(&d).unwrap();
            let image = draw(&d, &settings());
            for (a, b) in image
                .linear
                .iter()
                .flatten()
                .zip(reference.linear.iter().flatten())
            {
                assert!(
                    (a - b * factor).abs() < 3e-7,
                    "model{model:?} red{red} strength{strength}: {a} vs {}",
                    b * factor
                );
            }
            let restored = storage::Envelope::new(0, None, &d)
                .unwrap()
                .decode()
                .unwrap();
            assert_eq!(draw(&restored, &settings()).linear, image.linear);
            assert_eq!(canonical(&d).unwrap(), before);
        }
        let mut direct = settings();
        direct.environment = [0.; 3];
        direct.light.intensity = [4.; 3];
        direct.light.position = [0., 0., 3.];
        let mut black = document(0, 1., model.clone(), Opacity::Opaque)
            .snapshot()
            .clone();
        let mut white = document(255, 1., model.clone(), Opacity::Opaque)
            .snapshot()
            .clone();
        for s in [&mut black, &mut white] {
            for m in s.materials.values_mut() {
                m.emission = [0.1, 0.2, 0.3];
            }
        }
        let a = render_core::render::render(
            &Evaluator::default().evaluate(&black).unwrap(),
            &direct,
            || false,
        )
        .unwrap();
        let b = render_core::render::render(
            &Evaluator::default().evaluate(&white).unwrap(),
            &direct,
            || false,
        )
        .unwrap();
        assert_eq!(
            a.linear, b.linear,
            "AO must not tint direct light or emission"
        );
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn principled_mask_and_blend_keep_occlusion_after_coverage_admission() {
    for opacity in [
        Opacity::Mask {
            factor: 1.,
            cutoff: 0.5,
        },
        Opacity::Blend { factor: 1. },
    ] {
        let d = document(0, 1., Model::Principled, opacity);
        let image = draw(&d, &settings());
        assert!(image.linear.iter().all(|p| *p == [0.; 3]));
        assert!(image.objects.iter().all(Option::is_some));
    }
}
