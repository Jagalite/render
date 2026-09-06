use glam::{DVec3, Vec3};
use render_core::{render::*, scattering::*, *};
fn shading() -> Shading {
    Shading {
        color: Vec3::splat(0.6),
        emission: Vec3::ZERO,
        metallic: 0.,
        roughness: 0.4,
        occlusion: 1.,
        normal: DVec3::Z,
    }
}
fn materials() -> Vec<Model> {
    vec![
        Model::Principled,
        Model::Coated {
            weight: 1.,
            ior: 1.5,
            roughness: 0.2,
        },
        Model::Conductor {
            eta: [0.2, 0.9, 1.1],
            k: [3., 2., 1.5],
        },
    ]
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn dielectric_snell_fresnel_and_total_internal_reflection() {
    assert!((dielectric_fresnel(1., 1., 1.5) - 0.04).abs() < 1e-14);
    assert_eq!(dielectric_fresnel(0.01, 1., 1.), 0.);
    assert_eq!(dielectric_fresnel(0.5, 1.5, 1.), 1.);
    let incident = DVec3::new(0.5, 0., -(0.75f64).sqrt());
    let t = refract(incident, DVec3::Z, 1. / 1.5).unwrap();
    assert!((t.x - 1. / 3.).abs() < 1e-12);
    assert!((t.length() - 1.).abs() < 1e-12);
    assert!(refract(DVec3::new(0.9, 0., -(0.19f64).sqrt()), DVec3::Z, 1.5).is_none());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn coated_conductor_reciprocity_furnace_and_pdf_sampling() {
    let s = shading();
    let n = DVec3::Z;
    let v = DVec3::new(0.3, 0., 1.).normalize();
    let l = DVec3::new(-0.1, 0.4, 1.).normalize();
    for model in materials() {
        assert!(brdf(&model, &s, n, v, l).distance(brdf(&model, &s, n, l, v)) < 1e-12);
        let count = 100000;
        let mut uniform = DVec3::ZERO;
        let mut sampled = DVec3::ZERO;
        let mut density_integral = 0.;
        let mut accepted = 0;
        for i in 0..count {
            let z = (f64::from(i) + 0.5) / f64::from(count);
            let phi = std::f64::consts::TAU * ((f64::from(i) * 0.6180339887498949).fract());
            let l = DVec3::new(
                (1. - z * z).sqrt() * phi.cos(),
                (1. - z * z).sqrt() * phi.sin(),
                z,
            );
            uniform += brdf(&model, &s, n, v, l) * z * (std::f64::consts::TAU / f64::from(count));
            density_integral += pdf(&model, &s, n, v, l) * std::f64::consts::TAU / f64::from(count);
            let r = std::array::from_fn(|d| random(i as u32, 0, d as u32, 413));
            let l = sample_direction(&model, &s, n, v, r);
            let p = pdf(&model, &s, n, v, l);
            if p > 0. {
                accepted += 1;
                sampled += brdf(&model, &s, n, v, l) * n.dot(l) / (p * f64::from(count));
            }
        }
        assert!(
            uniform.min_element() >= 0. && uniform.max_element() <= 1.001,
            "{model:?} {uniform:?}"
        );
        assert!(
            uniform.distance(sampled) < 0.015,
            "{model:?}: {uniform:?} vs {sampled:?}"
        );
        assert!((density_integral - f64::from(accepted) / f64::from(count)).abs() < 0.01);
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn alpha_coverage_and_ideal_glass_render_analytic_environment() {
    let (scene, mut settings, _) = fixtures::diffuse_plane().unwrap();
    let mut scene = scene;
    settings.width = 16;
    settings.height = 16;
    settings.samples = 64;
    settings.max_depth = 4;
    settings.light.intensity = [0.; 3];
    settings.environment = [1.; 3];
    let m = &mut scene.instances[0].material;
    m.base_color = [0.; 3];
    m.roughness = 0.;
    m.pbr = Some(pbr::Surface {
        double_sided: true,
        advanced: Some(Surface {
            model: Model::Principled,
            opacity: Opacity::Mask {
                factor: 0.,
                cutoff: 0.5,
            },
        }),
        ..Default::default()
    });
    let clear = render_core::render::render(&scene, &settings, || false).unwrap();
    assert!(clear.linear.iter().all(|p| *p == [1.; 3]));
    assert!(clear.objects.iter().all(Option::is_none));
    scene.instances[0]
        .material
        .pbr
        .as_mut()
        .unwrap()
        .advanced
        .as_mut()
        .unwrap()
        .opacity = Opacity::Blend { factor: 0.5 };
    let blend = render_core::render::render(&scene, &settings, || false).unwrap();
    let mean =
        blend.linear.iter().map(|p| f64::from(p[0])).sum::<f64>() / blend.linear.len() as f64;
    assert!((mean - 0.52).abs() < 0.025, "{mean}");
    // IOR=1 transmits a white environment exactly, without a hidden eta factor.
    scene.instances[0].material.base_color = [1.; 3];
    scene.instances[0].material.pbr.as_mut().unwrap().advanced = Some(Surface {
        model: Model::Dielectric { ior: 1. },
        opacity: Opacity::Opaque,
    });
    let glass = render_core::render::render(&scene, &settings, || false).unwrap();
    assert!(glass.linear.iter().all(|p| *p == [1.; 3]));
    assert_eq!(
        render_core::render::render(&scene, &settings, || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    let mut invalid = scene.instances[0].material.clone();
    invalid.roughness = 0.5;
    assert_eq!(invalid.validate().unwrap_err().code, "material");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn dielectric_closed_interface_furnace_and_mixed_media_integration() {
    // At normal incidence, a parallel slab has no TIR: eta factors cancel at
    // paired interfaces. The remaining reflection-tail probability at depth 16
    // is below 0.04^15. Faceted spheres can trap paths and have a nonzero finite-
    // depth deficit, so they are not an exact unit-radiance oracle.
    let d = feature_fixtures::surface_document().unwrap();
    let mut snapshot = d.snapshot().clone();
    let slab = modeling::box_mesh([-1., -1., -0.2], [1., 1., 0.2]).unwrap();
    let key = slab.content_id().unwrap();
    snapshot.geometry_bindings.remove(&Id(8620));
    snapshot
        .meshes
        .insert(key.clone(), std::sync::Arc::new(slab));
    snapshot.entities.get_mut(Id(8620)).unwrap().mesh = Some(key);
    let mut scene = Evaluator::default().evaluate(&snapshot).unwrap();
    scene.instances.retain(|i| i.id == Id(8620));
    scene.bvh = Bvh::build(&scene.instances.iter().map(|i| i.bounds).collect::<Vec<_>>());
    let mut s = d.snapshot().render_settings.clone().unwrap();
    s.camera.position = [0., 0., 3.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 0.8,
        ymag: 0.8,
        near: 0.01,
        far: 10.,
    });
    s.width = 12;
    s.height = 8;
    s.samples = 128;
    s.max_depth = 16;
    s.light.intensity = [0.; 3];
    s.environment = [1.; 3];
    let image = render_core::render::render(&scene, &s, || false).unwrap();
    assert!(
        image
            .linear
            .iter()
            .flatten()
            .all(|x| (*x - 1.).abs() < 1e-5),
        "furnace range {:?}",
        image
            .linear
            .iter()
            .flatten()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &x| (
                a.min(x),
                b.max(x)
            ))
    );
    let d = feature_fixtures::volume_document().unwrap();
    let mut snapshot = d.snapshot().clone();
    snapshot.version = 10;
    snapshot
        .materials
        .values_mut()
        .next()
        .unwrap()
        .pbr
        .as_mut()
        .unwrap()
        .advanced = Some(Surface {
        model: Model::Coated {
            weight: 0.6,
            ior: 1.5,
            roughness: 0.3,
        },
        opacity: Opacity::Opaque,
    });
    let scene = Evaluator::default().evaluate(&snapshot).unwrap();
    let mut s = snapshot.render_settings.clone().unwrap();
    s.width = 12;
    s.height = 8;
    s.samples = 2;
    let image = render_core::render::render(&scene, &s, || false).unwrap();
    assert!(
        image
            .linear
            .iter()
            .flatten()
            .all(|x| x.is_finite() && *x >= 0.)
    );
    assert!(
        image
            .receipt
            .approximation
            .contains("sparse single scattering")
    );
}
