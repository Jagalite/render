//! Bounded conformance adapter for the shared Rust renderer, not an authoring path.
use render_core::{document::Document, render::*, *};
use serde::Deserialize;
use std::{collections::BTreeSet, sync::atomic::AtomicBool};
use wasm_bindgen::prelude::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    name: String,
    revision: String,
    document: Document,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    cases: Vec<Input>,
}
fn js(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
fn same(a: &Image, b: &Image) -> Result<()> {
    if a.linear != b.linear
        || a.depth != b.depth
        || a.normals != b.normals
        || a.objects != b.objects
        || canonical(&a.receipt)? != canonical(&b.receipt)?
    {
        return Err(Error::new(
            "conformance",
            "cached and fresh GPU outputs differ",
        ));
    }
    Ok(())
}
fn passes(image: &Image) -> serde_json::Value {
    serde_json::json!({"linear_rgb":image.linear,"depth_meters":image.depth,"normals_world":image.normals,"object_ids":image.objects})
}
#[wasm_bindgen]
pub async fn gpu_geometry_update_conformance(json: &str) -> std::result::Result<String, JsValue> {
    if json.len() > 16 * 1024 * 1024 {
        return Err(js("budget: conformance input exceeds 16 MiB"));
    }
    let request: Request = serde_json::from_str(json).map_err(js)?;
    if request.version != 0 || request.cases.is_empty() || request.cases.len() > 16 {
        return Err(js(
            "request: conformance requires version 0 and 1..16 cases",
        ));
    }
    let mut names = BTreeSet::new();
    for input in &request.cases {
        if input.name.is_empty() || input.name.len() > 32 || !names.insert(&input.name) {
            return Err(js("request: conformance names must be unique and bounded"));
        }
        let snapshot = input.document.snapshot();
        snapshot.validate().map_err(js)?;
        if snapshot.revision().map_err(js)? != input.revision {
            return Err(js("stale_revision: conformance snapshot changed"));
        }
        let s = snapshot
            .render_settings
            .as_ref()
            .ok_or_else(|| js("reference: render settings required"))?;
        s.validate().map_err(js)?;
        if s.width > 32
            || s.height > 32
            || s.samples > 8
            || s.max_depth > 4
            || s.max_bytes > 64 * 1024 * 1024
        {
            return Err(js("budget: conformance render exceeds bounded profile"));
        }
    }
    let cancel = AtomicBool::new(false);
    let mut gpu = render_gpu::Renderer::new().await.map_err(js)?;
    let mut cases = vec![];
    for input in &request.cases {
        let snapshot = input.document.snapshot();
        let before = canonical(snapshot).map_err(js)?;
        let scene = Evaluator::default().evaluate(snapshot).map_err(js)?;
        let s = snapshot
            .render_settings
            .as_ref()
            .expect("validated settings");
        let image = gpu.render(&scene, s, 0, &cancel).await.map_err(js)?;
        let statistics = gpu.geometry_upload_statistics();
        let mut fresh = render_gpu::Renderer::new().await.map_err(js)?;
        let reference = fresh.render(&scene, s, 0, &cancel).await.map_err(js)?;
        same(&image, &reference).map_err(js)?;
        let cpu = render(&scene, s, || false).map_err(js)?;
        if before != canonical(snapshot).map_err(js)? {
            return Err(js("conformance: snapshot changed"));
        }
        cases.push(serde_json::json!({"name":input.name,"revision":input.revision,"statistics":statistics,"fresh_gpu_exact":true,"snapshot_unchanged":true,"cpu":passes(&cpu),"gpu":passes(&image)}));
    }
    let first = &request.cases[0];
    let scene = Evaluator::default()
        .evaluate(first.document.snapshot())
        .map_err(js)?;
    let s = first
        .document
        .snapshot()
        .render_settings
        .as_ref()
        .expect("validated settings");
    let before = canonical(&gpu.geometry_upload_statistics()).map_err(js)?;
    let precancel = gpu
        .render(&scene, s, 0, &AtomicBool::new(true))
        .await
        .is_err_and(|e| e.code == "cancelled");
    let mut invalid = s.clone();
    invalid.max_bytes = 1;
    let budget = gpu
        .render(&scene, &invalid, 0, &cancel)
        .await
        .is_err_and(|e| e.code == "budget");
    if !precancel
        || !budget
        || before != canonical(&gpu.geometry_upload_statistics()).map_err(js)?
    {
        return Err(js(
            "conformance: pre-admission failures changed cache counters",
        ));
    }
    let postcancel = gpu.cancellation_fault_probe(&scene, s).await.map_err(js)?;
    let restored = gpu.render(&scene, s, 0, &cancel).await.map_err(js)?;
    let reused = gpu
        .geometry_upload_statistics()
        .last
        .as_ref()
        .is_some_and(|r| r.kind == render_gpu::GeometryUploadKind::Reuse);
    let mut fresh = render_gpu::Renderer::new().await.map_err(js)?;
    same(
        &restored,
        &fresh.render(&scene, s, 0, &cancel).await.map_err(js)?,
    )
    .map_err(js)?;
    gpu.destroy();
    let cleared = gpu.geometry_upload_statistics().retained_shadow_bytes == 0;
    let lost = gpu
        .render(&scene, s, 0, &cancel)
        .await
        .is_err_and(|e| e.code == "device_lost");
    let mut recreated = render_gpu::Renderer::new().await.map_err(js)?;
    same(
        &restored,
        &recreated.render(&scene, s, 0, &cancel).await.map_err(js)?,
    )
    .map_err(js)?;
    if !postcancel || !reused || !cleared || !lost {
        return Err(js(
            "conformance: cancelled or lost GPU cache contract failed",
        ));
    }
    serde_json::to_string(&serde_json::json!({"status":"passed","profile":"gpu-buffer-updates-v1","cases":cases,"pre_admission_failures_preserve_counters":true,"post_submit_cancel_cache_valid":true,"destroy_clears_shadow":true,"recreated_exact":true,"device":recreated.capabilities})).map_err(js)
}
