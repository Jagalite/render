#[path = "support/dielectric.rs"]
mod fixture;
use render_core::{render::Evaluator, *};
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn analytic_ideal_interfaces_snell_tir_frames_and_mirrors_survive_recovery() {
    for case in fixture::cases() {
        let before = canonical(&case.document).unwrap();
        let scene = Evaluator::default()
            .evaluate(case.document.snapshot())
            .unwrap();
        let image = render::render(&scene, &case.settings, || false).unwrap();
        println!(
            "{} expected={:?} actual={:?}",
            case.name, case.expected, image.linear[0]
        );
        assert!(
            image.linear[0]
                .iter()
                .zip(case.expected)
                .all(|(a, b)| (a - b).abs() < 2e-6),
            "{}",
            case.name
        );
        assert_eq!(image.objects[0].is_some(), case.name != "far-clipped");
        let restored = storage::Envelope::new(0, None, &case.document)
            .unwrap()
            .decode()
            .unwrap();
        let again = render::render(
            &Evaluator::default().evaluate(restored.snapshot()).unwrap(),
            &case.settings,
            || false,
        )
        .unwrap();
        assert_eq!(image.linear, again.linear);
        assert_eq!(before, canonical(&case.document).unwrap());
    }
}
