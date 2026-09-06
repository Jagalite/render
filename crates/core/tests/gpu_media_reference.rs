#[path = "support/gpu_media.rs"]
mod fixture;
use render_core::{render::*, *};
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn analytic_sparse_emission_extinction_camera_and_affine_corpus() {
    for case in fixture::cases() {
        let scene = Evaluator::default()
            .evaluate(case.document.snapshot())
            .unwrap();
        let image = render(&scene, &case.settings, || false).unwrap();
        for (a, b) in image.linear[0].iter().zip(case.expected) {
            assert!(
                (a - b).abs() < 2e-6,
                "{}: {:?} != {:?}",
                case.name,
                image.linear[0],
                case.expected
            );
        }
        let recovered = storage::Envelope::new(0, None, &case.document)
            .unwrap()
            .decode()
            .unwrap();
        assert_eq!(
            canonical(&case.document).unwrap(),
            canonical(&recovered).unwrap()
        );
    }
}
