//! Development-only reader probe; source bytes are supplied explicitly by the caller.
use render_core::{Id, blend, canonical, document::*, fixtures};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("expected source file")?;
    let bytes = std::fs::read(path)?;
    let mut settings = fixtures::settings();
    settings.width = 32;
    settings.height = 18;
    settings.samples = 4;
    let imported = blend::import(
        &bytes,
        Id(293),
        &blend::Policy {
            scene: "Scene".into(),
            meters_per_unit: 1.,
            allow_principled_approximation: true,
            allow_point_light_approximation: true,
        },
        settings,
        || false,
    )?;
    let mut doc = Document::new(Snapshot::empty(Id(293)))?;
    let request = fixtures::request(&doc, "blend-probe-native", imported.commands)?;
    let receipt = doc.execute(
        &fixtures::principal(),
        &Request {
            max_added_bytes: 16 * 1024 * 1024,
            ..request
        },
    )?;
    let scene = render_core::render::Evaluator::default().evaluate(doc.snapshot())?;
    let image = render_core::render::render(&scene, &imported.settings, || false)?;
    let out = serde_json::json!({"report":imported.report,"receipt":receipt,"entities":doc.snapshot().entities,"meshes":doc.snapshot().meshes,"camera":imported.settings.camera,"light":imported.settings.light,"environment":imported.settings.environment,"image":{"linear_rgb":image.linear,"depth":image.depth,"normal":image.normals,"object_ids":image.objects}});
    println!("{}", String::from_utf8(canonical(&out)?)?);
    Ok(())
}
