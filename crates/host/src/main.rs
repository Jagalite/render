use render_core::{
    document::*,
    render::*,
    storage::{native::*, *},
    *,
};
mod agent_workflow;
mod job_host;
mod milestone_workflow;
mod project_cli;
mod static_pbr;
use std::{
    fs,
    io::{self, BufRead, Read, Write},
    path::Path,
    sync::atomic::AtomicBool,
    time::Instant,
};

fn control_line(reader: &mut impl BufRead) -> Result<Option<String>> {
    const LIMIT: u64 = 16 * 1024 * 1024;
    let mut bytes = Vec::new();
    let read = reader
        .take(LIMIT + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(|e| Error::new("io", e.to_string()))?;
    if read == 0 {
        return Ok(None);
    }
    if read as u64 > LIMIT {
        return Err(Error::new(
            "budget",
            "control stream exceeds 16 MiB; connection closed",
        ));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|e| Error::new("encoding", e.to_string()))
}

fn write(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> Result<()> {
    fs::write(path, bytes).map_err(|e| Error::new("io", e.to_string()))
}
fn save_image(root: &Path, name: &str, image: &Image) -> Result<()> {
    write(root.join(format!("{name}.ppm")), image.ppm())?;
    write(root.join(format!("{name}.pfm")), image.pfm())?;
    write(
        root.join(format!("{name}.receipt.json")),
        canonical(&image.receipt)?,
    )?;
    write(
        root.join(format!("{name}.passes.json")),
        canonical(&(
            image.width,
            image.height,
            &image.depth,
            &image.normals,
            &image.objects,
        ))?,
    )?;
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!(
            "{}",
            serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
        );
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("project") => {
            if args.len() == 2 && args[1] == "--help" {
                print!("{}", project_cli::help());
                return Ok(());
            }
            if args.len() != 3 {
                return Err(Error::new(
                    "usage",
                    "render-host project <project-directory> <request.json|->",
                ));
            }
            let value = project_cli::run(Path::new(&args[1]), &args[2])?;
            println!("{}", serde_json::to_string(&value)?);
            Ok(())
        }
        Some("agent") => {
            let root = Path::new(
                args.get(1)
                    .map(String::as_str)
                    .unwrap_or("artifacts/agent-project"),
            );
            let mut store = NativeStore::open(root)?;
            let document =
                recover(&store.load()?)?.unwrap_or(Document::new(Snapshot::empty(Id(1)))?);
            let mut session = agent::Session::new(document);
            let mut input = io::stdin().lock();
            while let Some(line) = control_line(&mut input)? {
                let result = if line.len() > 16 * 1024 * 1024 {
                    Err(Error::new("budget", "control limit 16 MiB"))
                } else {
                    serde_json::from_str(&line)
                        .map_err(Error::from)
                        .and_then(|r| {
                            session.dispatch_durable(
                                &mut store,
                                &Principal {
                                    id: "local-stdin".into(),
                                    can_write: true,
                                },
                                r,
                            )
                        })
                };
                println!("{}", serde_json::to_string(&result)?);
                io::stdout()
                    .flush()
                    .map_err(|e| Error::new("io", e.to_string()))?;
            }
            Ok(())
        }
        Some("modeling-workflow") => milestone_workflow::modeling(Path::new(
            args.get(1)
                .map(String::as_str)
                .unwrap_or("artifacts/m06/modeling"),
        )),
        Some("surfaces-workflow") => milestone_workflow::surfaces(Path::new(
            args.get(1)
                .map(String::as_str)
                .unwrap_or("artifacts/m07/surfaces"),
        )),
        Some("imaging-workflow") => milestone_workflow::imaging(Path::new(
            args.get(1)
                .map(String::as_str)
                .unwrap_or("artifacts/m07/imaging"),
        )),
        Some("animation-workflow") => milestone_workflow::animation(Path::new(
            args.get(1)
                .map(String::as_str)
                .unwrap_or("artifacts/m08/character"),
        )),
        Some("groom-workflow") => milestone_workflow::groom(Path::new(
            args.get(1)
                .map(String::as_str)
                .unwrap_or("artifacts/m07/groom"),
        )),
        Some("volume-workflow") => milestone_workflow::volume(Path::new(
            args.get(1)
                .map(String::as_str)
                .unwrap_or("artifacts/m07/volume"),
        )),
        Some("geometry-workflow") => milestone_workflow::geometry(Path::new(
            args.get(1)
                .map(String::as_str)
                .unwrap_or("artifacts/m07/geometry"),
        )),
        Some("pbr-workflow") => static_pbr::run(Path::new(
            args.get(1)
                .map(String::as_str)
                .unwrap_or("artifacts/static-pbr/native"),
        )),
        Some("agent-workflow") => agent_workflow::run(
            Path::new(
                args.get(1)
                    .map(String::as_str)
                    .unwrap_or("fixtures/khronos-box/Box.glb"),
            ),
            Path::new(
                args.get(2)
                    .map(String::as_str)
                    .unwrap_or("artifacts/m05/native"),
            ),
            args.iter().any(|a| a == "--gpu"),
        ),
        Some("demo") => demo(
            Path::new(args.get(1).map(String::as_str).unwrap_or("artifacts/demo")),
            args.iter().any(|a| a == "--gpu"),
        ),
        Some("kernel") => {
            let kernel = match args.get(1).map(String::as_str) {
                None => render_kernel::path::kernel(),
                Some("--alpha") if args.len() == 2 => render_kernel::path::alpha_kernel(),
                Some("--surfaces") if args.len() == 2 => render_kernel::path::surface_kernel(),
                Some("--dielectric") if args.len() == 2 => render_kernel::path::dielectric_kernel(),
                _ => {
                    return Err(Error::new(
                        "arguments",
                        "kernel accepts only --alpha, --surfaces or --dielectric",
                    ));
                }
            };
            print!("{}", kernel.generate()?);
            Ok(())
        }
        Some("verify") => {
            let root = Path::new(
                args.get(1)
                    .map(String::as_str)
                    .unwrap_or("artifacts/evidence"),
            );
            fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
            write(
                root.join("core_conformance.json"),
                canonical(&fixtures::conformance()?)?,
            )?;
            benchmark(root)?;
            if args.iter().any(|a| a == "--gpu") {
                pollster::block_on(gpu_verify(root))?;
            }
            Ok(())
        }
        Some("api") => {
            api(Path::new(args.get(1).ok_or_else(|| {
                Error::new("usage", "api requires a project directory")
            })?))
        }
        Some("jobs") => {
            jobs_api(Path::new(args.get(1).ok_or_else(|| {
                Error::new("usage", "jobs requires a project directory")
            })?))
        }
        Some("serve") => serve(
            Path::new(args.get(1).map(String::as_str).unwrap_or("web")),
            args.get(2).map(String::as_str).unwrap_or("8765"),
        ),
        _ => Err(Error::new(
            "usage",
            "render-host project <project> <request.json|-> | agent <project> | agent-workflow [GLB] [output] [--gpu] | demo [directory] [--gpu] | verify [directory] [--gpu] | kernel [--alpha] | api <project> | jobs <project> | serve [web-root] [port]",
        )),
    }
}
#[derive(serde::Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
enum JobRequest {
    SubmitInput {
        id: Id,
        input: Box<jobs::RenderInput>,
        budget: jobs::Budget,
    },
    Submit {
        id: Id,
        settings: Settings,
        budget: jobs::Budget,
    },
    Cancel {
        id: Id,
    },
    Status {
        id: Id,
    },
    Events {
        cursor: Option<u64>,
    },
}
fn jobs_api(root: &Path) -> Result<()> {
    let host = job_host::JobHost::open(root, fixtures::demo()?)?;
    let mut input = io::stdin().lock();
    while let Some(line) = control_line(&mut input)? {
        let result = (|| -> Result<serde_json::Value> {
            match serde_json::from_str::<JobRequest>(&line)? {
                JobRequest::Submit {
                    id,
                    settings,
                    budget,
                } => Ok(serde_json::to_value(host.submit(
                    "local-stdin",
                    id,
                    settings,
                    budget,
                )?)?),
                JobRequest::SubmitInput { id, input, budget } => Ok(serde_json::to_value(
                    host.submit_input("local-stdin", id, *input, budget)?,
                )?),
                JobRequest::Cancel { id } => {
                    Ok(serde_json::to_value(host.cancel("local-stdin", id)?)?)
                }
                JobRequest::Status { id } => {
                    Ok(serde_json::to_value(host.status("local-stdin", id)?)?)
                }
                JobRequest::Events { cursor } => {
                    Ok(serde_json::to_value(host.events("local-stdin", cursor)?)?)
                }
            }
        })();
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}
fn demo(root: &Path, gpu: bool) -> Result<()> {
    fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
    let doc = fixtures::demo()?;
    write(root.join("scene.json"), canonical(&doc)?)?;
    let loaded: Document = serde_json::from_slice(
        &fs::read(root.join("scene.json")).map_err(|e| Error::new("io", e.to_string()))?,
    )?;
    let mut store = NativeStore::open(root.join("project"))?;
    if store.load()?.is_empty() {
        store.publish(&Envelope::new(0, None, &doc)?)?;
    }
    let scene = Evaluator::default().evaluate(loaded.snapshot())?;
    let settings = fixtures::settings();
    let start = Instant::now();
    let image = render(&scene, &settings, || false)?;
    save_image(root, "cpu", &image)?;
    let cpu_ms = start.elapsed().as_secs_f64() * 1000.;
    let mesh = fixtures::seam_mesh()?;
    let (obj, obj_loss) = interchange::export_obj(&mesh)?;
    write(root.join("mesh.obj"), obj)?;
    write(root.join("obj.loss.json"), canonical(&obj_loss)?)?;
    let (gltf, bin, gltf_loss) = interchange::export_gltf(&mesh)?;
    write(root.join("mesh.gltf"), gltf)?;
    write(root.join("mesh.bin"), bin)?;
    write(root.join("gltf.loss.json"), canonical(&gltf_loss)?)?;
    if gpu {
        let mut renderer = pollster::block_on(render_gpu::Renderer::new())?;
        let image =
            pollster::block_on(renderer.render(&scene, &settings, 0, &AtomicBool::new(false)))?;
        save_image(root, "gpu", &image)?;
        write(root.join("device.json"), canonical(&renderer.capabilities)?)?;
    }
    println!(
        "{}",
        serde_json::json!({"revision":scene.revision,"cpu_ms":cpu_ms,"instances":scene.instances.len(),"unique_geometry_builds":scene.geometry_builds,"output":root})
    );
    Ok(())
}
async fn gpu_verify(root: &Path) -> Result<()> {
    let doc = fixtures::demo()?;
    let scene = Evaluator::default().evaluate(doc.snapshot())?;
    let mut s = fixtures::settings();
    s.width = 48;
    s.height = 32;
    s.samples = 8;
    let cpu = render(&scene, &s, || false)?;
    let mut renderer = render_gpu::Renderer::new().await?;
    let start = Instant::now();
    let gpu = renderer
        .render(&scene, &s, 0, &AtomicBool::new(false))
        .await?;
    let gpu_ms = start.elapsed().as_secs_f64() * 1000.;
    let mut squared = 0.;
    let mut largest = 0f32;
    for (a, b) in cpu.linear.iter().flatten().zip(gpu.linear.iter().flatten()) {
        let e = (a - b).abs();
        largest = largest.max(e);
        squared += f64::from(e * e);
    }
    let rmse = (squared / (cpu.linear.len() * 3) as f64).sqrt();
    save_image(root, "reference", &cpu)?;
    save_image(root, "gpu", &gpu)?;
    if rmse > 0.035 {
        return Err(Error::new(
            "gpu_conformance",
            format!("CPU/GPU RMSE {rmse} > 0.035; max {largest}"),
        ));
    }
    let cancelled = renderer
        .render(&scene, &s, 0, &AtomicBool::new(true))
        .await
        .unwrap_err()
        .code
        == "cancelled";
    let mut too_large = s.clone();
    too_large.max_bytes = 1;
    let admission = renderer
        .render(&scene, &too_large, 0, &AtomicBool::new(false))
        .await
        .unwrap_err()
        .code
        == "budget";
    let mut progressive = render_gpu::Progressive::default();
    let mut batch = s.clone();
    batch.samples = 4;
    progressive
        .step(&mut renderer, &scene, &batch, &AtomicBool::new(false))
        .await?;
    let accumulated = progressive
        .step(&mut renderer, &scene, &batch, &AtomicBool::new(false))
        .await?;
    let progressive_error = accumulated
        .linear
        .iter()
        .flatten()
        .zip(gpu.linear.iter().flatten())
        .map(|(a, b)| (a - b).abs())
        .fold(0f32, f32::max);
    if progressive_error > 1e-5 {
        return Err(Error::new(
            "progressive",
            format!("batch accumulation mismatch {progressive_error}"),
        ));
    }
    let uploads = renderer.geometry_uploads;
    let mut moved = doc.clone();
    let r = fixtures::request(
        &moved,
        "gpu:transform:001",
        vec![Command::SetTransform {
            entity: Id(4),
            transform: Transform::translation(-2., 0., 0.),
        }],
    )?;
    moved.execute(&fixtures::principal(), &r)?;
    let moved_scene = Evaluator::default().evaluate(moved.snapshot())?;
    renderer
        .render(&moved_scene, &s, 0, &AtomicBool::new(false))
        .await?;
    let reuse = renderer.geometry_uploads == uploads;
    let allocation_rejected = renderer.allocation_fault_probe().await?;
    if !allocation_rejected {
        return Err(Error::new(
            "allocation_conformance",
            "over-limit GPU buffer was not rejected",
        ));
    }
    let raster = renderer.raster_preview(&scene, &s).await?;
    if raster.len() != (s.width * s.height) as usize
        || raster.iter().any(|p| p[3] != 255)
        || raster.iter().all(|p| p == &raster[0])
    {
        return Err(Error::new(
            "raster_conformance",
            "invalid raster coverage or alpha",
        ));
    }
    let mut raster_ppm = format!("P6\n{} {}\n255\n", s.width, s.height).into_bytes();
    for pixel in &raster {
        for &c in &pixel[..3] {
            raster_ppm.push((linear_to_srgb(c as f32 / 255.) * 255.).clamp(0., 255.) as u8);
        }
    }
    write(root.join("raster.ppm"), raster_ppm)?;
    let empty = Evaluator::default().evaluate(&Snapshot::empty(Id(99)))?;
    let clear = renderer.raster_preview(&empty, &s).await?;
    for p in clear {
        for (actual, expected) in p[..3].iter().zip(s.environment) {
            if (*actual as f32 - expected * 255.).abs() > 1. {
                return Err(Error::new(
                    "raster_conformance",
                    "analytic clear color mismatch",
                ));
            }
        }
    }
    let capability = renderer.capabilities.clone();
    let cancellation_start = Instant::now();
    let cancelled_after_submit = renderer.cancellation_fault_probe(&scene, &s).await?;
    let cancelled_render_ms = cancellation_start.elapsed().as_secs_f64() * 1000.;
    let (analytic_scene, analytic_settings, expected) = fixtures::diffuse_plane()?;
    let analytic = renderer
        .render(
            &analytic_scene,
            &analytic_settings,
            0,
            &AtomicBool::new(false),
        )
        .await?;
    let analytic_error = analytic
        .linear
        .iter()
        .flat_map(|p| p.iter().zip(expected).map(|(a, b)| (a - b).abs()))
        .fold(0f32, f32::max);
    if analytic_error > 1e-6
        || analytic.objects.iter().any(Option::is_none)
        || !cancelled_after_submit
    {
        return Err(Error::new(
            "gpu_conformance",
            "analytic plane or submitted cancellation failed",
        ));
    }
    renderer.destroy();
    let lost_rejected = renderer
        .render(&scene, &s, 0, &AtomicBool::new(false))
        .await
        .unwrap_err()
        .code
        == "device_lost";
    let mut recreated = render_gpu::Renderer::new().await?;
    let recovered = recreated
        .render(&scene, &s, 0, &AtomicBool::new(false))
        .await?;
    let recovered_equal = gpu.linear == recovered.linear;
    if !cancelled || !admission || !reuse || !lost_rejected || !recovered_equal {
        return Err(Error::new(
            "gpu_fault_conformance",
            "GPU cancellation, admission, reuse or device recovery failed",
        ));
    }
    let report = serde_json::json!({"status":"passed","gpu_over_limit_allocation_rejected":allocation_rejected,"cancel_after_submit":cancelled_after_submit,"cancelled_render_ms_including_pack_submit_drain":cancelled_render_ms,"analytic_plane_max_error":analytic_error,"analytic_plane_tolerance":1e-6,"cpu_gpu_rmse":rmse,"max_channel_error":largest,"rmse_tolerance":0.035,"gpu_ms_including_upload_readback":gpu_ms,"progressive_max_error":progressive_error,"cancel_before_submit":cancelled,"memory_admission_rejected":admission,"transform_reuses_geometry_upload":reuse,"explicit_device_destroy_rejected":lost_rejected,"device_recreated_equal":recovered_equal,"device":capability,"driver_loss_injection":"not performed","raster_coverage_and_clear_color":true,"shader_digest":digest(render_kernel::path::kernel().generate()?.as_bytes())});
    write(root.join("gpu_conformance.json"), canonical(&report)?)?;
    println!("{report}");
    Ok(())
}
fn benchmark(root: &Path) -> Result<()> {
    let count = 10000;
    let mut entities = Vec::new();
    for n in 0..count {
        entities.push(Entity {
            id: Id(n),
            name: format!("Entity {n}"),
            parent: None,
            mesh: None,
            material: None,
            transform: Transform::default(),
        });
    }
    let start = Instant::now();
    let table = EntityTable::try_from(entities.clone())?;
    let build_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let mut branch = table.clone();
    let branch_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    branch.get_mut(Id(5000)).expect("fixture").name = "edited".into();
    let edit_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    let mut flat = entities.clone();
    let flat_branch_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    flat[5000].name = "edited".into();
    let flat_edit_us = start.elapsed().as_secs_f64() * 1e6;
    let mesh = fixtures::nonmanifold()?;
    let start = Instant::now();
    let edit = render_core::topology::EditMesh::new(&mesh)?;
    let radial_us = start.elapsed().as_secs_f64() * 1e6;
    let report = serde_json::json!({"profile":{"entities":count,"host":std::env::consts::ARCH,"os":std::env::consts::OS,"build":"see validation command; debug unless --release"},"typed_chunked":{"build_ms":build_ms,"branch_ms":branch_ms,"single_edit_us":edit_us,"chunks":table.chunk_count(),"shared_chunks_after_edit":table.shared_chunks(&branch)},"flat":{"branch_ms":flat_branch_ms,"single_edit_us":flat_edit_us},"radial":{"construction_us":radial_us,"max_incident_faces":edit.radial.iter().map(Vec::len).max(),"manifold_only_accepts":edit.manifold_candidate_accepts()},"generic_ecs_comparison":"run cargo run --release -p render-host --example m00; representation_comparison.json","memory_allocated_bytes":"not measured; sharing counts only","status":"supplemental sharing measurement"});
    write(root.join("benchmark_results.json"), canonical(&report)?)?;
    println!("{report}");
    Ok(())
}
fn api(root: &Path) -> Result<()> {
    let mut store = NativeStore::open(root)?;
    let mut doc = match recover(&store.load()?)? {
        Some(d) => d,
        None => Document::new(Snapshot::empty(Id(1)))?,
    };
    let principal = Principal {
        id: "local-stdin".into(),
        can_write: true,
    };
    let mut input = io::stdin().lock();
    while let Some(line) = control_line(&mut input)? {
        let result = (|| {
            let request: Request = serde_json::from_str(&line)?;
            durable_execute(&mut store, &mut doc, &principal, &request)
        })();
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}
fn serve(root: &Path, port: &str) -> Result<()> {
    use std::net::TcpListener;
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .map_err(|e| Error::new("network", e.to_string()))?;
    println!("http://127.0.0.1:{port}");
    for stream in listener.incoming() {
        let mut stream = stream.map_err(|e| Error::new("network", e.to_string()))?;
        let mut first = String::new();
        io::BufReader::new(&mut stream)
            .read_line(&mut first)
            .map_err(|e| Error::new("network", e.to_string()))?;
        let path = first
            .split_whitespace()
            .nth(1)
            .unwrap_or("/")
            .split('?')
            .next()
            .unwrap_or("/");
        if path.contains("..") || path.contains('%') {
            continue;
        }
        let relative = if path == "/" {
            "index.html"
        } else {
            path.trim_start_matches('/')
        };
        let file = root.join(relative);
        let (status, data) = match fs::read(&file) {
            Ok(b) => ("200 OK", b),
            Err(_) => ("404 Not Found", b"Not found".to_vec()),
        };
        let mime = match file.extension().and_then(|s| s.to_str()) {
            Some("wasm") => "application/wasm",
            Some("js") => "text/javascript",
            Some("json") => "application/json",
            _ => "text/html",
        };
        write!(stream,"HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: {mime}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",data.len()).map_err(|e|Error::new("network",e.to_string()))?;
        stream
            .write_all(&data)
            .map_err(|e| Error::new("network", e.to_string()))?;
    }
    Ok(())
}
