//! Complete public-API asset workflow; validation never modifies reference images.
use super::{save_image, write};
use render_core::{
    agent::{self, Operation, Session},
    document::*,
    gltf_scene::PbrPolicy,
    render::*,
    storage::{native::NativeStore, *},
    *,
};
use serde_json::json;
use std::{path::Path, sync::atomic::AtomicBool, time::Instant};
pub fn run(root: &Path) -> Result<()> {
    if root.exists() {
        return Err(Error::new(
            "output_exists",
            "choose fresh PBR evidence directory",
        ));
    }
    std::fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
    let started = Instant::now();
    let mut gpu = pollster::block_on(render_gpu::Renderer::new())?;
    let mut rows = vec![];
    for (index, name) in ["BoxTextured", "NormalTangentMirrorTest"]
        .iter()
        .enumerate()
    {
        let folder = Path::new("fixtures/static-pbr").join(name);
        let source = std::fs::read(folder.join(format!("{name}.glb")))
            .map_err(|e| Error::new("io", e.to_string()))?;
        let mut settings = fixtures::settings();
        settings.width = 64;
        settings.height = 64;
        settings.samples = 32;
        settings.max_depth = 1;
        settings.max_bytes = 256 * 1024 * 1024;
        settings.environment = [0.25; 3];
        settings.light = PointLight {
            position: [2., 3., 4.],
            intensity: [20.; 3],
        };
        settings.camera = Camera {
            position: if index == 0 {
                [2., 1.5, 3.]
            } else {
                [0., 0., 5.]
            },
            target: [0.; 3],
            up: [0., 1., 0.],
            vertical_fov_radians: 0.6,
            lens: None,
        };
        let mut session = Session::new(Document::new(Snapshot::empty(Id(800 + index as u128)))?);
        let base = session.document().snapshot().revision()?;
        let mut store = NativeStore::open(root.join(name).join("project"))?;
        let op = agent::Request {
            version: 0,
            operation: Operation::ImportPbrGlb {
                bytes: source.clone(),
                policy: PbrPolicy {
                    allow_approximations: true,
                },
                settings: settings.clone(),
                base_revision: base,
                idempotency_key: format!("pbr:import:{name}:01"),
            },
        };
        let import = session.dispatch_durable(&mut store, &fixtures::principal(), op.clone())?;
        if import != session.dispatch_durable(&mut store, &fixtures::principal(), op)? {
            return Err(Error::new("conformance", "PBR durable retry"));
        }
        let reopened = recover(&store.load()?)?
            .ok_or_else(|| Error::new("recovery", "missing PBR project"))?;
        if canonical(&reopened)? != canonical(session.document())? {
            return Err(Error::new("conformance", "PBR exact durable recovery"));
        }
        let scene = Evaluator::default().evaluate(reopened.snapshot())?;
        let at = Instant::now();
        let cpu = render(&scene, &settings, || false)?;
        let cpu_seconds = at.elapsed().as_secs_f64();
        let at = Instant::now();
        let image = pollster::block_on(gpu.render(&scene, &settings, 0, &AtomicBool::new(false)))?;
        let gpu_seconds = at.elapsed().as_secs_f64();
        save_image(root, &format!("{name}-cpu"), &cpu)?;
        save_image(root, &format!("{name}-gpu"), &image)?;
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
        let visible = cpu.objects.iter().filter(|id| id.is_some()).count();
        let packed = render_gpu::pack(&scene, &settings, 0)?;
        let row = json!({"asset":name,"source_sha256":digest(&source),"import":import,"revision":scene.revision,"encoded_snapshot_bytes":canonical(reopened.snapshot())?.len(),"image_count":reopened.snapshot().images.len(),"triangles":scene.instances.iter().map(|i|i.geometry.triangles.len()).sum::<usize>(),"linear_rmse":rmse,"normal_rmse":normal_rmse,"object_mismatches":mismatches,"visible_pixels":visible,"cpu_seconds":cpu_seconds,"gpu_seconds":gpu_seconds,"gpu_packed_bytes":16*(packed.geometry.len()+packed.texels.len()+packed.instances.len()+packed.params.len()),"cpu_receipt":cpu.receipt,"gpu_receipt":image.receipt});
        write(
            root.join(format!("{name}-comparison.json")),
            serde_json::to_vec_pretty(&row)?,
        )?;
        // Predeclared same-sample f64/f32 gates, including visibility boundaries.
        if rmse > 0.025 || normal_rmse > 0.025 || mismatches > 4 || visible < 100 {
            return Err(Error::new(
                "conformance",
                format!("PBR CPU/GPU comparison: {row}"),
            ));
        }
        if !pollster::block_on(gpu.cancellation_fault_probe(&scene, &settings))? {
            return Err(Error::new("conformance", "GPU cancellation"));
        }
        if render(&scene, &settings, || true).is_ok() {
            return Err(Error::new("conformance", "CPU cancellation"));
        }
        if index == 1 {
            let mut back = settings.clone();
            back.camera.position = [0., 0., -5.];
            let cpu = render(&scene, &back, || false)?;
            let image = pollster::block_on(gpu.render(&scene, &back, 0, &AtomicBool::new(false)))?;
            save_image(root, "mirror-back-cpu", &cpu)?;
            save_image(root, "mirror-back-gpu", &image)?;
            let rmse = (cpu
                .linear
                .iter()
                .flatten()
                .zip(image.linear.iter().flatten())
                .map(|(a, b)| f64::from(a - b).powi(2))
                .sum::<f64>()
                / (cpu.linear.len() * 3) as f64)
                .sqrt();
            let normals = (cpu
                .normals
                .iter()
                .flatten()
                .zip(image.normals.iter().flatten())
                .map(|(a, b)| f64::from(a - b).powi(2))
                .sum::<f64>()
                / (cpu.normals.len() * 3) as f64)
                .sqrt();
            if rmse > 0.025 || normals > 0.025 {
                return Err(Error::new(
                    "conformance",
                    format!("back-face PBR comparison {rmse}, {normals}"),
                ));
            }
            write(
                root.join("back_face_report.json"),
                serde_json::to_vec_pretty(
                    &json!({"status":"passed","linear_rmse":rmse,"normal_rmse":normals}),
                )?,
            )?;
        }
        if index == 0 {
            write(
                root.join("sampler_camera_report.json"),
                serde_json::to_vec_pretty(&sampler_camera_matrix(&mut gpu, &scene, &settings)?)?,
            )?;
        }
        rows.push(row);
    }
    write(
        root.join("workflow_report.json"),
        serde_json::to_vec_pretty(
            &json!({"status":"passed","profile":"gltf2-static-pbr-v0","seconds":started.elapsed().as_secs_f64(),"gpu":gpu.capabilities,"assets":rows,"durable_recovery":true,"cancellation":true}),
        )?,
    )?;
    println!("PBR workflow passed: {}", root.display());
    Ok(())
}

/// Cross-backend probe of disposable evaluated geometry; authored fixtures remain immutable.
fn sampler_camera_matrix(
    gpu: &mut render_gpu::Renderer,
    scene: &Scene,
    settings: &Settings,
) -> Result<serde_json::Value> {
    use render_core::textures::{Filter, MinFilter, Wrap};
    let mut s = settings.clone();
    s.width = 16;
    s.height = 16;
    s.samples = 4;
    let mut scene = scene.clone();
    for instance in &mut scene.instances {
        let geometry = std::sync::Arc::make_mut(&mut instance.geometry);
        for triangle in &mut geometry.triangles {
            for uv in &mut triangle.uv {
                uv.x = uv.x * 4. - 1.;
                uv.y = uv.y * 4. - 1.;
            }
        }
    }
    let mut cases = vec![];
    for (lens_index, lens) in [
        None,
        Some(cameras::Lens::Perspective {
            vertical_fov_radians: 0.6,
            aspect_ratio: Some(1.25),
            near: 0.1,
            far: Some(20.),
        }),
        Some(cameras::Lens::Orthographic {
            xmag: 1.,
            ymag: 1.,
            near: 0.,
            far: 20.,
        }),
    ]
    .into_iter()
    .enumerate()
    {
        s.camera.lens = lens;
        for wrap in [Wrap::Repeat, Wrap::Clamp, Wrap::Mirror] {
            for min in [
                MinFilter::Nearest,
                MinFilter::Linear,
                MinFilter::NearestMipNearest,
                MinFilter::LinearMipNearest,
                MinFilter::NearestMipLinear,
                MinFilter::LinearMipLinear,
            ] {
                for instance in &mut scene.instances {
                    let b = instance
                        .material
                        .pbr
                        .as_mut()
                        .and_then(|p| p.base_color.as_mut())
                        .expect("textured fixture");
                    b.sampler.wrap_s = wrap;
                    b.sampler.wrap_t = wrap;
                    b.sampler.min = min;
                    b.sampler.mag = Filter::Linear;
                }
                let cpu = render(&scene, &s, || false)?;
                let image = pollster::block_on(gpu.render(&scene, &s, 0, &AtomicBool::new(false)))?;
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
                if rmse > 0.025 || mismatches > 2 {
                    return Err(Error::new(
                        "conformance",
                        format!(
                            "sampler/lens {lens_index}/{wrap:?}/{min:?}: rmse {rmse}, IDs {mismatches}"
                        ),
                    ));
                }
                cases.push(json!({"lens":lens_index,"wrap":wrap,"min_filter":min,"linear_rmse":rmse,"object_mismatches":mismatches}));
            }
        }
    }
    // The same scene disappears beyond the far plane in both lens implementations.
    for lens in [
        cameras::Lens::Perspective {
            vertical_fov_radians: 0.6,
            aspect_ratio: None,
            near: 0.1,
            far: Some(1.),
        },
        cameras::Lens::Orthographic {
            xmag: 1.,
            ymag: 1.,
            near: 0.,
            far: 1.,
        },
    ] {
        s.camera.lens = Some(lens);
        let cpu = render(&scene, &s, || false)?;
        let image = pollster::block_on(gpu.render(&scene, &s, 0, &AtomicBool::new(false)))?;
        if cpu
            .objects
            .iter()
            .chain(&image.objects)
            .any(Option::is_some)
        {
            return Err(Error::new("conformance", "lens far-plane clipping"));
        }
    }
    Ok(
        json!({"status":"passed","cases":cases,"far_plane_clipping":true,"fixture":"BoxTextured evaluated UVs scaled by four and offset by minus one"}),
    )
}
