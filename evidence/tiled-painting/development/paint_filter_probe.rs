// Temporary development probe; archived separately after diagnosis.
use render_core::{Id, canonical, digest, document::Document, render::*};
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn actual_paint_texture_footprints() {
    let doc:Document=serde_json::from_str(include_str!("../../../artifacts/tiled-painting/run-20260906T204501Z/paint-workflow/archive/document/document.json")).unwrap();
    let settings=doc.snapshot().render_settings.as_ref().unwrap();let scene=Evaluator::default().evaluate(doc.snapshot()).unwrap();
    let image=scene.images.values().next().unwrap();
    let hashes:Vec<_>=image.levels.iter().map(|l|digest(&canonical(&l.rgba.iter().map(|v|v.map(f32::to_bits)).collect::<Vec<_>>()).unwrap())).collect();
    let dims=glam::Vec2::new(image.levels[0].width as f32,image.levels[0].height as f32);
    let mut rows=Vec::new();
    for y in 0..settings.height {for x in 0..settings.width {let pixel=y*settings.width+x;for sample in 0..settings.samples {
        let px=x as f64+random(pixel,sample,0,settings.seed);let py=y as f64+random(pixel,sample,1,settings.seed);
        let ray=settings.camera.ray(px,py,settings.width,settings.height).unwrap();let (near,far)=settings.camera.clip(ray);
        let Some(hit)=scene.intersect(ray,near,far) else{continue};let inst=&scene.instances[hit.instance];let tri=&inst.geometry.triangles[hit.triangle];let attr=Id(500);
        let values=&tri.uv_sets[inst.geometry.uv_attributes.iter().position(|v|*v==attr).unwrap()];let uv=hit.uv_for(&scene,Some(attr));
        let derivatives=[settings.camera.ray(px+1.,py,settings.width,settings.height).unwrap(),settings.camera.ray(px,py+1.,settings.width,settings.height).unwrap()].map(|r|projected_uv_values(tri,Ray{origin:inst.inverse.transform_point3(r.origin),direction:inst.inverse.transform_vector3(r.direction)},values).unwrap_or(uv)-uv);
        let rho=(derivatives[0]*dims).length().max((derivatives[1]*dims).length()).max(1e-8);
        let binding=inst.material.pbr.as_ref().unwrap().base_color.as_ref().unwrap();let sampled=image.sample(&binding.sampler,uv,derivatives[0],derivatives[1]).to_array().map(f32::to_bits);
        rows.push([pixel*settings.samples+sample,uv.x.to_bits(),uv.y.to_bits(),derivatives[0].x.to_bits(),derivatives[0].y.to_bits(),derivatives[1].x.to_bits(),derivatives[1].y.to_bits(),rho.to_bits(),rho.log2().to_bits(),libm::log2f(rho).to_bits(),(libm::log2(f64::from(rho)) as f32).to_bits(),sampled[0],sampled[1],sampled[2],sampled[3]]);
    }}}
    let result=serde_json::json!({"mip_hashes":hashes,"rows":rows});
    #[cfg(not(target_arch="wasm32"))] println!("FILTER_PROBE {}",result);
    #[cfg(target_arch="wasm32")] wasm_bindgen_test::console_log!("FILTER_PROBE {}",result);
    assert!(!result["rows"].as_array().unwrap().is_empty());
}
