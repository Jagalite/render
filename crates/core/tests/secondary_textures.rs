use render_core::{document::*, render::*, textures::*, *};
fn document() -> Document {
    let mut d = Document::new(Snapshot::empty(Id(10070))).unwrap();
    let imported = gltf_scene::import_pbr_glb(
        include_bytes!("../../../fixtures/secondary-textures/data/mask-nearest.glb"),
        Id(10070),
        &gltf_scene::PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "footprint:import:001", imported.commands).unwrap(),
    )
    .unwrap();
    d
}
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 8;
    s.height = 8;
    s.samples = 32;
    s.max_depth = 2;
    s.environment = [0.; 3];
    s.light.intensity = [0.; 3];
    s.camera.position = [0., 0., 1.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 0.9,
        ymag: 0.9,
        near: 0.01,
        far: 10.,
    });
    s
}
fn configure(d: &mut Document, model: Option<scattering::Model>, min: MinFilter) {
    let commands = d
        .snapshot()
        .materials
        .values()
        .filter(|m| m.pbr.is_some())
        .map(|m| {
            let mut material = m.clone();
            let p = material.pbr.as_mut().unwrap();
            if p.advanced.is_some() {
                p.advanced = model.clone().map(|model| scattering::Surface {
                    model,
                    opacity: scattering::Opacity::Opaque,
                });
                if matches!(model, Some(scattering::Model::Dielectric { .. })) {
                    material.roughness = 0.;
                }
            }
            if let Some(binding) = p.emission.as_mut() {
                binding.sampler.min = min;
            }
            Command::PutMaterial {
                material: Box::new(material),
            }
        })
        .collect();
    let key = format!("footprint:configure:{}", d.snapshot().revision().unwrap());
    d.execute(
        &fixtures::principal(),
        &fixtures::request(d, &key, commands).unwrap(),
    )
    .unwrap();
}
fn draw(d: &Document, s: &Settings) -> Image {
    render(
        &Evaluator::default().evaluate(d.snapshot()).unwrap(),
        s,
        || false,
    )
    .unwrap()
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn secondary_only_emitter_is_invariant_to_minification_filter_in_all_surface_models() {
    let models = [
        None,
        Some(scattering::Model::Principled),
        Some(scattering::Model::Coated {
            weight: 0.5,
            ior: 1.5,
            roughness: 0.2,
        }),
        Some(scattering::Model::Conductor {
            eta: [0.2; 3],
            k: [3.; 3],
        }),
        Some(scattering::Model::Dielectric { ior: 1.5 }),
    ];
    for model in models {
        let mut nearest = document();
        configure(&mut nearest, model.clone(), MinFilter::Nearest);
        let a = draw(&nearest, &settings());
        assert!(
            a.linear.iter().flatten().any(|x| *x > 0.01),
            "nonempty oracle {model:?}"
        );
        let mut mip = document();
        configure(&mut mip, model.clone(), MinFilter::LinearMipLinear);
        let before = canonical(&mip).unwrap();
        let b = draw(&mip, &settings());
        let error = a
            .linear
            .iter()
            .flatten()
            .zip(b.linear.iter().flatten())
            .map(|(x, y)| (x - y).abs())
            .fold(0_f32, f32::max);
        assert_eq!(error, 0., "secondary LOD0 invariant for {model:?}");
        assert_eq!(a.objects, b.objects);
        assert_eq!(a.depth, b.depth);
        assert_eq!(a.normals, b.normals);
        let restored = storage::Envelope::new(0, None, &mip)
            .unwrap()
            .decode()
            .unwrap();
        assert_eq!(draw(&restored, &settings()).linear, b.linear);
        assert_eq!(canonical(&mip).unwrap(), before);
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn directly_visible_emitter_retains_primary_minification_and_cancel_budget_checks() {
    let mut a = document();
    configure(
        &mut a,
        Some(scattering::Model::Principled),
        MinFilter::Nearest,
    );
    let mut b = document();
    configure(
        &mut b,
        Some(scattering::Model::Principled),
        MinFilter::LinearMipLinear,
    );
    let mut s = settings();
    s.camera.target = [0., 0., 2.];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 3.,
        ymag: 3.,
        near: 0.01,
        far: 10.,
    });
    s.max_depth = 1;
    s.samples = 1;
    let nearest = draw(&a, &s);
    let mip = draw(&b, &s);
    assert!(
        nearest
            .linear
            .iter()
            .flatten()
            .zip(mip.linear.iter().flatten())
            .any(|(x, y)| (x - y).abs() > 0.05)
    );
    let scene = Evaluator::default().evaluate(b.snapshot()).unwrap();
    assert_eq!(render(&scene, &s, || true).unwrap_err().code, "cancelled");
    s.max_bytes = 1;
    assert_eq!(render(&scene, &s, || false).unwrap_err().code, "budget");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn discarded_primary_alpha_does_not_turn_camera_footprints_into_secondary_footprints() {
    let mut variants = vec![];
    for min in [MinFilter::Nearest, MinFilter::LinearMipLinear] {
        let mut d = document();
        configure(&mut d, Some(scattering::Model::Principled), min);
        let snapshot = d.snapshot();
        let mut commands = vec![];
        for material in snapshot.materials.values() {
            if material.pbr.as_ref().is_some_and(|p| p.advanced.is_some()) {
                let mut material = material.clone();
                material
                    .pbr
                    .as_mut()
                    .unwrap()
                    .advanced
                    .as_mut()
                    .unwrap()
                    .opacity = scattering::Opacity::Mask {
                    factor: 0.,
                    cutoff: 0.5,
                };
                commands.push(Command::PutMaterial {
                    material: Box::new(material),
                });
            }
        }
        let ceiling = snapshot
            .entities
            .iter()
            .find(|e| e.name == "checker-ceiling")
            .unwrap();
        let mut transform = ceiling.transform.clone();
        transform.columns[3][2] = -1.;
        commands.push(Command::SetTransform {
            entity: ceiling.id,
            transform,
        });
        d.execute(
            &fixtures::principal(),
            &fixtures::request(&d, "footprint:transparent", commands).unwrap(),
        )
        .unwrap();
        let mut s = settings();
        // 1.8 world meters / 4 pixels / 8 emitter meters * 32 texels =
        // 1.8 texels per primary pixel: this must select minification.
        s.width = 4;
        s.height = 4;
        s.samples = 1;
        s.max_depth = 1;
        variants.push(draw(&d, &s));
    }
    assert_eq!(variants[0].objects, variants[1].objects);
    assert!(variants[0].depth.iter().all(|d| (*d - 2.).abs() < 1e-6));
    assert!(
        variants[0]
            .linear
            .iter()
            .flatten()
            .zip(variants[1].linear.iter().flatten())
            .all(|(x, y)| (x - y).abs() > 0.4)
    );
}
