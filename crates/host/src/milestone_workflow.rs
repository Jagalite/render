//! Public document and render workflows for the M07/M08 execution ledger.
use super::{save_image, write};
use render_core::{
    document::*,
    render::*,
    storage::{native::NativeStore, *},
    *,
};
use serde_json::json;
use std::{path::Path, sync::atomic::AtomicBool, time::Instant};
pub fn modeling(root: &Path) -> Result<()> {
    let document = feature_fixtures::modeling_document()?;
    surface(root, document.clone(), 1, "typed-modeling-graph-v0")?;
    let graph =
        &document.snapshot().procedural_assets[&document.snapshot().procedural_bindings[&Id(8720)]];
    let evaluated = graph.evaluate(&document.snapshot().meshes, || false)?;
    let budget = modeling::Budget::default();
    let cube = modeling::box_mesh([-0.3; 3], [0.3; 3])?;
    let (bevel, _) = modeling::apply(
        &cube,
        &modeling::Operation::BevelBox {
            distance_meters: 0.06,
        },
        &budget,
        || false,
    )?;
    let (mut direct, _) = modeling::apply(
        &bevel,
        &modeling::Operation::Array {
            count: 3,
            step: [0.8, 0., 0.],
        },
        &budget,
        || false,
    )?;
    for i in 0..direct.positions.len() {
        let mut position = direct.positions.get(i);
        position.x -= 0.8;
        position.y += 0.2;
        direct.positions.set(i, position)?;
    }
    if direct.content_id()? != evaluated.mesh.content_id()? {
        return Err(Error::new(
            "conformance",
            "direct/group/field geometry differs",
        ));
    }
    write(
        root.join("procedural_conformance.json"),
        serde_json::to_vec_pretty(
            &json!({"status":"passed","direct_graph_digest":direct.content_id()?,"receipt":evaluated.receipt,"durable_roundtrip":true}),
        )?,
    )?;
    let mut rows = vec![];
    for count in [1, 8, 32] {
        let (source, _) = modeling::apply(
            &cube,
            &modeling::Operation::Array {
                count,
                step: [0.8, 0., 0.],
            },
            &budget,
            || false,
        )?;
        for (kind, operation) in [
            (
                "local_split",
                modeling::Operation::SplitEdge {
                    edge: source.edge_ids[0],
                    fraction: 0.5,
                },
            ),
            (
                "wide_array",
                modeling::Operation::Array {
                    count: 2,
                    step: [0., 1., 0.],
                },
            ),
        ] {
            let at = Instant::now();
            let (output, receipt) = modeling::apply(&source, &operation, &budget, || false)?;
            rows.push(json!({"kind":kind,"source_boxes":count,"source_vertices":source.positions.len(),"output_vertices":output.positions.len(),"source_bytes":canonical(&source)?.len(),"output_bytes":receipt.output_bytes,"seconds":at.elapsed().as_secs_f64(),"receipt":receipt}));
        }
    }
    write(
        root.join("topology_mapping_report.json"),
        serde_json::to_vec_pretty(
            &json!({"status":"passed","policy":"global array/connectivity rebuild for local and wide edits; wall time and serialized bytes are measured, not allocator peak per operation","measurements":rows}),
        )?,
    )?;
    Ok(())
}
pub fn geometry(root: &Path) -> Result<()> {
    surface(
        root,
        feature_fixtures::geometry_document()?,
        3,
        "typed-curve-point-sweep-v0",
    )
}
pub fn groom(root: &Path) -> Result<()> {
    surface(
        root,
        feature_fixtures::groom_document()?,
        2,
        "guide-root-sweep-v0",
    )
}
fn surface(root: &Path, authored: Document, expected_objects: usize, profile: &str) -> Result<()> {
    if root.exists() {
        return Err(Error::new(
            "output_exists",
            "choose a fresh geometry evidence directory",
        ));
    }
    std::fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
    let start = Instant::now();
    let mut store = NativeStore::open(root.join("project"))?;
    store.publish(&Envelope::new(0, None, &authored)?)?;
    let restored = recover(&store.load()?)?
        .ok_or_else(|| Error::new("recovery", "geometry fixture missing"))?;
    if canonical(&authored)? != canonical(&restored)? {
        return Err(Error::new(
            "conformance",
            "typed geometry durable roundtrip",
        ));
    }
    write(root.join("document.json"), canonical(&restored)?)?;
    let mut evaluator = Evaluator::default();
    let at = Instant::now();
    let scene = evaluator.evaluate(restored.snapshot())?;
    let evaluation_seconds = at.elapsed().as_secs_f64();
    let settings = restored
        .snapshot()
        .render_settings
        .as_ref()
        .expect("fixture settings");
    let cpu = render(&scene, settings, || false)?;
    let mut gpu = pollster::block_on(render_gpu::Renderer::new())?;
    let image = pollster::block_on(gpu.render(&scene, settings, 0, &AtomicBool::new(false)))?;
    save_image(root, "geometry-cpu", &cpu)?;
    save_image(root, "geometry-gpu", &image)?;
    let rmse = (cpu
        .linear
        .iter()
        .flatten()
        .zip(image.linear.iter().flatten())
        .map(|(a, b)| f64::from(a - b).powi(2))
        .sum::<f64>()
        / (cpu.linear.len() * 3) as f64)
        .sqrt();
    let normal_rmse = (cpu
        .normals
        .iter()
        .flatten()
        .zip(image.normals.iter().flatten())
        .map(|(a, b)| f64::from(a - b).powi(2))
        .sum::<f64>()
        / (cpu.normals.len() * 3) as f64)
        .sqrt();
    let mismatches = cpu
        .objects
        .iter()
        .zip(&image.objects)
        .filter(|(a, b)| a != b)
        .count();
    let mut visible = std::collections::BTreeMap::new();
    for id in cpu.objects.iter().flatten() {
        *visible.entry(*id).or_insert(0) += 1;
    }
    if rmse > 0.025
        || normal_rmse > 0.025
        || mismatches > 4
        || visible.len() != expected_objects
        || visible.values().any(|n| *n < 10)
    {
        return Err(Error::new(
            "conformance",
            format!(
                "geometry image comparison rmse={rmse}, normal={normal_rmse}, IDs={mismatches}, visible={visible:?}"
            ),
        ));
    }
    if !pollster::block_on(gpu.cancellation_fault_probe(&scene, settings))?
        || render(&scene, settings, || true).is_ok()
        || evaluator
            .evaluate_with_cancel(restored.snapshot(), || true)
            .is_ok()
    {
        return Err(Error::new("conformance", "geometry cancellation"));
    }
    let mut stale = restored.clone();
    let mut req = fixtures::request(
        &stale,
        "m07:stale:0000001",
        vec![Command::SetGeometry {
            entity: scene.instances[0].id,
            asset: None,
        }],
    )?;
    req.base_revision = "stale".into();
    if stale.execute(&fixtures::principal(), &req).is_ok()
        || canonical(&stale)? != canonical(&restored)?
    {
        return Err(Error::new("conformance", "geometry stale transaction"));
    }
    write(
        root.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"status":"passed","profile":profile,"revision":scene.revision,"conversions":scene.conversions,"procedures":scene.procedures,"linear_rmse":rmse,"normal_rmse":normal_rmse,"object_mismatches":mismatches,"visible_pixels":visible,"evaluation_seconds":evaluation_seconds,"seconds":start.elapsed().as_secs_f64(),"source_snapshot_bytes":canonical(restored.snapshot())?.len(),"cached_geometry_builds":evaluator.evaluate(restored.snapshot())?.geometry_builds,"gpu":gpu.capabilities,"durable_roundtrip":true,"cancellation":true,"stale_revision":true}),
        )?,
    )?;
    println!("M07 geometry workflow passed: {}", root.display());
    Ok(())
}

pub fn volume(root: &Path) -> Result<()> {
    if root.exists() {
        return Err(Error::new(
            "output_exists",
            "choose a fresh volume evidence directory",
        ));
    }
    std::fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
    let at = Instant::now();
    let authored = feature_fixtures::volume_document()?;
    let mut store = NativeStore::open(root.join("project"))?;
    store.publish(&Envelope::new(0, None, &authored)?)?;
    let restored = recover(&store.load()?)?
        .ok_or_else(|| Error::new("recovery", "volume document missing"))?;
    if canonical(&authored)? != canonical(&restored)? {
        return Err(Error::new(
            "conformance",
            "volume recovery changed authored state",
        ));
    }
    write(root.join("document.json"), canonical(&restored)?)?;
    let mut evaluator = Evaluator::default();
    let scene = evaluator.evaluate(restored.snapshot())?;
    let settings = restored
        .snapshot()
        .render_settings
        .as_ref()
        .expect("fixture settings");
    let image = render(&scene, settings, || false)?;
    save_image(root, "volume-cpu", &image)?;
    let rejected = render_gpu::pack(&scene, settings, 0)
        .err()
        .ok_or_else(|| Error::new("conformance", "GPU silently accepted unsupported volume"))?;
    if rejected.code != "unsupported_profile" {
        return Err(rejected);
    }
    let mut without = scene.clone();
    without.media = volumes::Media::default();
    let clear = render(&without, settings, || false)?;
    let changed = image
        .linear
        .iter()
        .zip(&clear.linear)
        .filter(|(a, b)| a.iter().zip(*b).any(|(a, b)| (a - b).abs() > 0.01))
        .count();
    if changed < 100 {
        return Err(Error::new(
            "conformance",
            "volume fixture did not affect enough pixels",
        ));
    }
    if render(&scene, settings, || true).is_ok() {
        return Err(Error::new("conformance", "volume render cancellation"));
    }
    write(
        root.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"status":"passed","profile":"sparse-media-v0","revision":scene.revision,"occupied_cells":scene.media.cell_count(),"changed_pixels":changed,"seconds":at.elapsed().as_secs_f64(),"source_snapshot_bytes":canonical(restored.snapshot())?.len(),"durable_roundtrip":true,"gpu_rejection":rejected,"cancellation":true}),
        )?,
    )?;
    println!("M07 sparse volume workflow passed: {}", root.display());
    Ok(())
}

pub fn animation(root: &Path) -> Result<()> {
    if root.exists() {
        return Err(Error::new(
            "output_exists",
            "choose a fresh animation evidence directory",
        ));
    }
    std::fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
    let at = Instant::now();
    let document = feature_fixtures::character_document()?;
    let mut store = NativeStore::open(root.join("project"))?;
    store.publish(&Envelope::new(0, None, &document)?)?;
    let restored = recover(&store.load()?)?
        .ok_or_else(|| Error::new("recovery", "animated document missing"))?;
    if canonical(&document)? != canonical(&restored)? {
        return Err(Error::new("conformance", "animated recovery mismatch"));
    }
    write(root.join("document.json"), canonical(&restored)?)?;
    let snapshot = restored.snapshot();
    let revision = snapshot.revision()?;
    let request = sequence::SequenceRequest {
        revision: revision.clone(),
        clip: Id(8400),
        times: (0..9).map(|i| Time::new(i, 8)).collect::<Result<_>>()?,
        shutter: sequence::Shutter {
            open: Time::new(-1, 32)?,
            close: Time::new(1, 32)?,
            samples: 4,
        },
    };
    let receipts = sequence::render_sequence(
        snapshot,
        &request,
        |i, frame| {
            save_image(root, &format!("frame-{i:03}"), &frame.image)?;
            write(
                root.join(format!("frame-{i:03}.evaluation.json")),
                canonical(&frame.evaluation)?,
            )
        },
        || false,
    )?;
    let mut evaluator = Evaluator::default();
    let mut gpu = pollster::block_on(render_gpu::Renderer::new())?;
    let settings = snapshot.render_settings.as_ref().expect("fixture settings");
    let mut comparisons = vec![];
    for time in [Time::new(0, 1)?, Time::new(1, 2)?, Time::new(1, 1)?] {
        let (scene, receipt) = evaluator.evaluate_at(snapshot, Id(8400), time, || false)?;
        let cpu = render(&scene, settings, || false)?;
        let image = pollster::block_on(gpu.render(&scene, settings, 0, &AtomicBool::new(false)))?;
        let rmse = (cpu
            .linear
            .iter()
            .flatten()
            .zip(image.linear.iter().flatten())
            .map(|(a, b)| f64::from(a - b).powi(2))
            .sum::<f64>()
            / (cpu.linear.len() * 3) as f64)
            .sqrt();
        let mismatches = cpu
            .objects
            .iter()
            .zip(&image.objects)
            .filter(|(a, b)| a != b)
            .count();
        if rmse > 0.025
            || mismatches > 4
            || cpu.objects.iter().filter(|id| id.is_some()).count() < 100
        {
            return Err(Error::new(
                "conformance",
                format!("animated CPU/Metal mismatch {rmse}, IDs={mismatches}"),
            ));
        }
        comparisons.push(json!({"time":time,"evaluation":receipt,"linear_rmse":rmse,"object_mismatches":mismatches,"gpu_receipt":image.receipt}));
    }
    let evaluated = animation::evaluate(snapshot, Id(8400), Time::new(1, 2)?, || false)?;
    let mesh = &evaluated.snapshot.meshes[evaluated
        .snapshot
        .entities
        .get(Id(8201))
        .expect("fixture mesh")
        .mesh
        .as_ref()
        .expect("fixture mesh")];
    let (obj, loss) = interchange::export_obj(mesh)?;
    write(root.join("deformed-frame.obj"), obj)?;
    write(root.join("deformation-export-loss.json"), canonical(&loss)?)?;
    write(root.join("request.json"), canonical(&request)?)?;
    write(
        root.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"status":"passed","profile":"affine-lbs-shutter-v0","revision":revision,"frames":receipts,"comparisons":comparisons,"seconds":at.elapsed().as_secs_f64(),"durable_roundtrip":true,"source_snapshot_bytes":canonical(snapshot)?.len(),"gpu_shutter":"per-time native Metal evaluation validated; sequence temporal integration uses Rust CPU"}),
        )?,
    )?;
    println!("M08 character sequence workflow passed: {}", root.display());
    Ok(())
}

pub fn imaging(root: &Path) -> Result<()> {
    if root.exists() {
        return Err(Error::new(
            "output_exists",
            "choose a fresh imaging evidence directory",
        ));
    }
    std::fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
    let at = Instant::now();
    let document = feature_fixtures::imaging_document()?;
    let mut store = NativeStore::open(root.join("project"))?;
    store.publish(&Envelope::new(0, None, &document)?)?;
    let restored = recover(&store.load()?)?
        .ok_or_else(|| Error::new("recovery", "imaging document missing"))?;
    if canonical(&document)? != canonical(&restored)? {
        return Err(Error::new("conformance", "imaging recovery mismatch"));
    }
    write(root.join("document.json"), canonical(&restored)?)?;
    let revision = restored.snapshot().revision()?;
    let products = products::render_products(restored.snapshot(), &revision, || false)?;
    for view in &products.views {
        save_image(root, &view.name, &view.raw)?;
        write(
            root.join(format!("{}.display.json", view.name)),
            canonical(&view.display)?,
        )?;
        write(
            root.join(format!("{}.albedo.json", view.name)),
            canonical(&view.albedo)?,
        )?;
        let mut ppm = format!("P6\n{} {}\n255\n", view.raw.width, view.raw.height).into_bytes();
        for rgb in &view.display.rgb {
            ppm.extend(rgb.map(|x| (x.clamp(0., 1.) * 255.).round() as u8));
        }
        write(root.join(format!("{}.display.ppm", view.name)), ppm)?;
    }
    for (name, bake) in &products.bakes {
        write(root.join(format!("{name}.json")), canonical(bake)?)?;
    }
    // The same scene-linear pixels can be delivered as a tagged P3 product.
    let mut p3 = restored
        .snapshot()
        .imaging
        .as_ref()
        .expect("fixture policy")
        .pipeline
        .clone();
    p3.destination = products::ColorSpace::DisplayP3;
    let p3 = p3.process(&products.views[0].raw, || false)?;
    write(root.join("beauty.p3.json"), canonical(&p3)?)?;
    write(
        root.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"status":"passed","profile":"rust-imaging-v0","revision":revision,"views":products.views.iter().map(|v|(&v.name,&v.display.output_digest)).collect::<Vec<_>>(),"bakes":products.bakes.iter().map(|(name,b)|(name,&b.output_digest)).collect::<Vec<_>>(),"seconds":at.elapsed().as_secs_f64(),"durable_roundtrip":true,"source_snapshot_bytes":canonical(restored.snapshot())?.len(),"backends":"Rust CPU on native and browser; raw render profiles remain separately declared"}),
        )?,
    )?;
    println!("M07 imaging workflow passed: {}", root.display());
    Ok(())
}

pub fn surfaces(root: &Path) -> Result<()> {
    if root.exists() {
        return Err(Error::new(
            "output_exists",
            "choose a fresh surface evidence directory",
        ));
    }
    std::fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
    let at = Instant::now();
    let document = feature_fixtures::surface_document()?;
    let mut store = NativeStore::open(root.join("project"))?;
    store.publish(&Envelope::new(0, None, &document)?)?;
    let restored = recover(&store.load()?)?
        .ok_or_else(|| Error::new("recovery", "surface document missing"))?;
    if canonical(&document)? != canonical(&restored)? {
        return Err(Error::new("conformance", "surface recovery mismatch"));
    }
    write(root.join("document.json"), canonical(&restored)?)?;
    let snapshot = restored.snapshot();
    let scene = Evaluator::default().evaluate(snapshot)?;
    let settings = snapshot.render_settings.as_ref().expect("fixture settings");
    let cpu = render(&scene, settings, || false)?;
    save_image(root, "surfaces-cpu", &cpu)?;
    let rejection = render_gpu::pack(&scene, settings, 0)
        .err()
        .ok_or_else(|| Error::new("conformance", "GPU silently accepted extended materials"))?;
    if rejection.code != "unsupported_profile" {
        return Err(rejection);
    }
    let mut floor = snapshot.clone();
    floor.layers.push(Layer {
        name: "displacement comparison".into(),
        overrides: [Id(8620), Id(8621), Id(8622)]
            .into_iter()
            .map(|id| {
                (
                    id,
                    Override {
                        transform: None,
                        material: None,
                        hidden: true,
                    },
                )
            })
            .collect(),
    });
    let floor = Evaluator::default().evaluate(&floor)?;
    let mut one = settings.clone();
    one.max_depth = 1;
    let cpu_floor = render(&floor, &one, || false)?;
    let mut gpu = pollster::block_on(render_gpu::Renderer::new())?;
    let gpu_floor = pollster::block_on(gpu.render(&floor, &one, 0, &AtomicBool::new(false)))?;
    let rmse = (cpu_floor
        .linear
        .iter()
        .flatten()
        .zip(gpu_floor.linear.iter().flatten())
        .map(|(a, b)| f64::from(a - b).powi(2))
        .sum::<f64>()
        / (cpu_floor.linear.len() * 3) as f64)
        .sqrt();
    if rmse > 0.025 {
        return Err(Error::new(
            "conformance",
            "CPU/Metal displaced geometry mismatch",
        ));
    }
    save_image(root, "displacement-gpu", &gpu_floor)?;
    let mut visible = std::collections::BTreeMap::new();
    for id in cpu.objects.iter().flatten() {
        *visible.entry(*id).or_insert(0) += 1;
    }
    if visible.len() != 4 || visible.values().any(|count| *count < 10) {
        return Err(Error::new(
            "conformance",
            format!("surface visibility {visible:?}"),
        ));
    }
    write(
        root.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"status":"passed","profile":"extended-bsdf-displacement-v0","revision":scene.revision,"visible_pixels":visible,"displacements":scene.displacements,"gpu_extended_rejection":rejection,"displaced_cpu_metal_rmse":rmse,"seconds":at.elapsed().as_secs_f64(),"source_snapshot_bytes":canonical(snapshot)?.len(),"durable_roundtrip":true}),
        )?,
    )?;
    println!("M07 surface workflow passed: {}", root.display());
    Ok(())
}
