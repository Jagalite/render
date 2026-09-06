//! Reproducible CLI client of public import, transaction, variant and job APIs.
use super::{job_host::JobHost, save_image, write};
use render_core::{
    agent::{self, *},
    document::*,
    gltf_scene::Policy,
    jobs::*,
    render::*,
    storage::{native::NativeStore, *},
    *,
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
fn call(s: &mut Session, operation: Operation) -> Result<Value> {
    s.dispatch(
        &fixtures::principal(),
        agent::Request {
            version: 0,
            operation,
        },
    )
}
pub fn run(input: &Path, root: &Path, gpu_enabled: bool) -> Result<()> {
    if root.exists() {
        return Err(Error::new(
            "output_exists",
            "choose a fresh evidence directory",
        ));
    }
    std::fs::create_dir_all(root).map_err(|e| Error::new("io", e.to_string()))?;
    let started = Instant::now();
    let bytes = std::fs::read(input).map_err(|e| Error::new("io", e.to_string()))?;
    let mut session = Session::new(Document::new(Snapshot::empty(Id(500)))?);
    let base = session.document().snapshot().revision()?;
    let settings = Settings {
        width: 64,
        height: 64,
        samples: 16,
        seed: 42,
        max_depth: 1,
        max_bytes: 8 * 1024 * 1024,
        environment: [0.15; 3],
        light: PointLight {
            position: [2., 3., 4.],
            intensity: [20.; 3],
        },
        camera: Camera {
            lens: None,
            position: [2., 1.5, 3.],
            target: [0.; 3],
            up: [0., 1., 0.],
            vertical_fov_radians: 0.6,
        },
    };
    let import = call(
        &mut session,
        Operation::ImportGlb {
            bytes: bytes.clone(),
            policy: Policy {
                allow_lambertian: true,
            },
            settings: settings.clone(),
            base_revision: base,
            idempotency_key: "m05:import:000001".into(),
        },
    )?;
    let authored = session.document().clone();
    let base = authored.snapshot().revision()?;
    let protected = protected_digest(authored.snapshot())?;
    let material = *authored
        .snapshot()
        .materials
        .keys()
        .next()
        .ok_or_else(|| Error::new("fixture", "material required"))?;
    let host = JobHost::open(&root.join("project"), authored.clone())?;
    let mut rows = vec![];
    let mut chosen = String::new();
    let mut best_green = -1.;
    for (i, (name, color, light)) in [
        ("warm", [0.8, 0.15, 0.08], [2., 3., 4.]),
        ("green", [0.1, 0.8, 0.15], [-2., 3., 4.]),
        ("cool", [0.1, 0.2, 0.8], [1., 4., 2.]),
    ]
    .into_iter()
    .enumerate()
    {
        call(
            &mut session,
            Operation::Branch {
                branch: name.into(),
                base_revision: base.clone(),
            },
        )?;
        let edit = Operation::Modify {
            branch: name.into(),
            base_revision: base.clone(),
            idempotency_key: format!("m05:variant:{name}:01"),
            edits: Edits {
                materials: vec![MaterialEdit {
                    material,
                    base_color: color,
                    emission: [0.; 3],
                }],
                light: PointLight {
                    position: light,
                    intensity: [20.; 3],
                },
                environment: [0.15; 3],
            },
            max_added_bytes: 1024 * 1024,
        };
        let modified = call(&mut session, edit.clone())?;
        if call(&mut session, edit)? != modified {
            return Err(Error::new("conformance", "variant retry"));
        }
        let revision = modified["revision"].as_str().expect("receipt").to_owned();
        let (snapshot, settings) = session.render_input(&fixtures::principal(), name, &revision)?;
        if protected_digest(&snapshot)? != protected {
            return Err(Error::new("conformance", "geometry protection"));
        }
        let scene = Evaluator::default().evaluate(&snapshot)?;
        let cpu = render(&scene, &settings, || false)?;
        save_image(root, &format!("{name}-cpu"), &cpu)?;
        let perception = session.record_render(&fixtures::principal(), name, &cpu)?;
        let gpu = if gpu_enabled {
            let mut gpu = pollster::block_on(render_gpu::Renderer::new())?;
            let image =
                pollster::block_on(gpu.render(&scene, &settings, 0, &AtomicBool::new(false)))?;
            save_image(root, &format!("{name}-gpu"), &image)?;
            let rmse = (image
                .linear
                .iter()
                .flatten()
                .zip(cpu.linear.iter().flatten())
                .map(|(a, b)| f64::from(a - b).powi(2))
                .sum::<f64>()
                / (image.linear.len() * 3) as f64)
                .sqrt();
            let object_mismatches = image
                .objects
                .iter()
                .zip(&cpu.objects)
                .filter(|(a, b)| a != b)
                .count();
            if rmse > 0.02 || object_mismatches > 4 {
                return Err(Error::new(
                    "conformance",
                    format!("GPU/CPU rmse {rmse}, object mismatches {object_mismatches}"),
                ));
            }
            Some(
                json!({"receipt":image.receipt,"linear_rmse":rmse,"object_mismatches":object_mismatches}),
            )
        } else {
            None
        };
        let snapshot_json_bytes = canonical(&snapshot)?.len();
        let pinned = RenderInput {
            snapshot,
            settings: settings.clone(),
        };
        let budget = Budget {
            max_wall_ms: 30000,
            max_bytes: settings.max_bytes,
            max_samples: settings.samples,
            max_output_bytes: 4 * 1024 * 1024,
        };
        let job_id = Id(1000 + i as u128);
        host.submit_input("agent", job_id, pinned.clone(), budget.clone())?;
        host.submit_input("agent", job_id, pinned, budget)?;
        let cursor = host.events("agent", None)?.last().map(|e| e.sequence);
        let deadline = Instant::now() + Duration::from_secs(35);
        let job = loop {
            let job = host.status("agent", job_id)?;
            if job.state.terminal() {
                break job;
            }
            if Instant::now() > deadline {
                return Err(Error::new("timeout", "workflow job"));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if job.state != State::Succeeded {
            return Err(Error::new("job", format!("{:?}", job.diagnostic)));
        }
        let receipt_bytes =
            std::fs::read(&job.artifacts[1]).map_err(|e| Error::new("io", e.to_string()))?;
        let job_receipt: RenderReceipt = serde_json::from_slice(&receipt_bytes)?;
        if job_receipt.output_digest != cpu.receipt.output_digest {
            return Err(Error::new(
                "conformance",
                "async job differs from inspected preview",
            ));
        }
        let resumed = host.events("agent", cursor)?;
        if resumed
            .iter()
            .any(|e| cursor.is_some_and(|c| e.sequence <= c))
        {
            return Err(Error::new("conformance", "event resumption"));
        }
        let score = perception["inspection"]["mean_linear_rgb"][1]
            .as_f64()
            .expect("numeric mean");
        if score > best_green {
            best_green = score;
            chosen = name.into();
        }
        rows.push(json!({"variant":name,"revision":revision,"protected_digest":protected,"inspection":perception["inspection"],"cpu_receipt":cpu.receipt,"gpu":gpu,"job_id":job.id,"job_state":job.state,"resumed_events":resumed,"snapshot_json_bytes":snapshot_json_bytes,"output_admission_bytes":settings.width*settings.height*64}));
    }
    let final_cursor = host.events("agent", None)?.last().map(|e| e.sequence);
    drop(host);
    let mut store = NativeStore::open(root.join("project"))?;
    let mut document =
        recover(&store.load()?)?.ok_or_else(|| Error::new("storage", "missing project"))?;
    let revision = rows
        .iter()
        .find(|v| v["variant"] == chosen)
        .expect("chosen")["revision"]
        .as_str()
        .expect("revision");
    let request = session.selection_request(
        &fixtures::principal(),
        &chosen,
        revision,
        "m05:selection:0001",
        1024 * 1024,
    )?;
    write(root.join("selection-request.json"), canonical(&request)?)?;
    let receipt = durable_execute(&mut store, &mut document, &fixtures::principal(), &request)?;
    let mut recovered = recover(&store.load()?)?.expect("durably committed");
    if durable_execute(&mut store, &mut recovered, &fixtures::principal(), &request)? != receipt
        || recovered.snapshot().revision()? != revision
        || protected_digest(recovered.snapshot())? != protected
    {
        return Err(Error::new("conformance", "selection recovery or retry"));
    }
    write(root.join("selected-document.json"), canonical(&recovered)?)?;
    let reopened = JobHost::open(&root.join("project"), authored)?;
    if !reopened.events("agent", final_cursor)?.is_empty() {
        return Err(Error::new(
            "conformance",
            "unexpected events after reconnect",
        ));
    }
    drop(reopened);
    let report = json!({"status":"passed","profile":"gltf2-static-lambertian-v0","input_digest":digest(&bytes),"import":import,"variants":rows,"chosen":chosen,"selection_rule":"highest mean green channel; deterministic test objective, not an aesthetic judgement","receipt":receipt,"root_revision_before":base,"protected_digest":protected,"recovery":true,"lost_ack_retry":true,"job_reconnect":true,"seconds":started.elapsed().as_secs_f64(),"gpu_exercised":gpu_enabled});
    write(
        root.join("agent_workflow_report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "{}",
        json!({"status":"passed","chosen":chosen,"report":root.join("agent_workflow_report.json")})
    );
    Ok(())
}
