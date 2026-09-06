use glam::{DVec3, Vec3};
use render_core::{
    render::Shading,
    scattering::{self, Model},
};
fn shading() -> Shading {
    Shading {
        color: Vec3::splat(0.3),
        emission: Vec3::ZERO,
        metallic: 0.2,
        roughness: 0.5,
        occlusion: 1.,
        normal: DVec3::Z,
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn zero_weight_coat_preserves_the_complete_base_sampling_distribution() {
    let s = shading();
    let n = DVec3::Z;
    let view = DVec3::new(0.3, 0., 1.).normalize();
    let coat = Model::Coated {
        weight: 0.,
        ior: 3.,
        roughness: 0.05,
    };
    for i in 0..1024 {
        let r = std::array::from_fn(|j| render_core::render::random(i, 0, j as u32, 311));
        let a = scattering::sample_direction(&Model::Principled, &s, n, view, r);
        let b = scattering::sample_direction(&coat, &s, n, view, r);
        assert_eq!(a, b);
        assert_eq!(
            scattering::pdf(&coat, &s, n, view, a),
            scattering::pdf(&Model::Principled, &s, n, view, a)
        );
        assert_eq!(
            scattering::brdf(&coat, &s, n, view, a),
            scattering::brdf(&Model::Principled, &s, n, view, a)
        );
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn optical_boundaries_have_independent_normal_incidence_values() {
    let mut s = shading();
    s.roughness = 1.;
    s.color = Vec3::ZERO;
    s.metallic = 1.;
    let n = DVec3::Z;
    // At normal incidence with roughness1: D=1/pi, visibility=1/4.
    for (model, reflectance) in [
        (
            Model::Conductor {
                eta: [0.; 3],
                k: [1.; 3],
            },
            1.,
        ),
        (
            Model::Conductor {
                eta: [1.; 3],
                k: [0.; 3],
            },
            0.,
        ),
        (
            Model::Coated {
                weight: 1.,
                ior: 1.5,
                roughness: 1.,
            },
            0.04,
        ),
        (
            Model::Coated {
                weight: 1.,
                ior: 1.,
                roughness: 1.,
            },
            0.,
        ),
    ] {
        let value = scattering::brdf(&model, &s, n, n, n) * 4. * std::f64::consts::PI;
        assert!(value.distance(DVec3::splat(reflectance)) < 1e-14);
    }
}
