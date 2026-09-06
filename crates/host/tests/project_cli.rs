use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn id(n: u128) -> String {
    format!("{n:032x}")
}
fn time(n: i64, d: u64) -> Value {
    json!({"numerator":n,"denominator":d})
}
struct Test {
    root: PathBuf,
    project: PathBuf,
}
impl Test {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "render-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self {
            project: root.join("project"),
            root,
        }
    }
    fn wire(&self, project: &Path, request: Value) -> std::process::Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_render-host"))
            .current_dir(repo())
            .args(["project"])
            .arg(project)
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&request).unwrap())
            .unwrap();
        child.wait_with_output().unwrap()
    }
    fn call(&self, operation: Value) -> Value {
        self.success(json!({"version":0,"operation":operation}))
    }
    fn success(&self, request: Value) -> Value {
        let out = self.wire(&self.project, request);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn fail(&self, request: Value, code: &str) -> Value {
        let out = self.wire(&self.project, request);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        let error: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(error["code"], code, "{error}");
        error
    }
    fn init(&self) {
        self.call(json!({"method":"init","document_id":id(99)}));
    }
    fn inspect(&self) -> Value {
        self.call(json!({"method":"inspect"}))
    }
    fn apply_request(&self, key: &str, commands: Value) -> Value {
        json!({"version":0,"base_revision":self.inspect()["revision"],"idempotency_key":key,"max_added_bytes":8*1024*1024,"commands":commands})
    }
    fn apply(&self, key: &str, commands: Value) -> Value {
        self.call(json!({"method":"apply","request":self.apply_request(key,commands)}))
    }
    fn author(&self) {
        self.init();
        self.apply(
            "cli:create:000001",
            read(repo().join("fixtures/project-cli/create.json")),
        );
        let mesh = self.inspect()["snapshot"]["entities"][0]["mesh"].clone();
        self.apply("cli:model:000001",json!([{"operation":"model_mesh","entity":id(1),"source_mesh":mesh,"operator":{"kind":"bevel_box","distance_meters":0.08},"budget":{"vertices":65536,"faces":65536,"bytes":8*1024*1024}}]));
        self.call(json!({"method":"import","base_revision":self.inspect()["revision"],"idempotency_key":"cli:import:00001","max_added_bytes":8*1024*1024,"source":{"format":"obj","path":"fixtures/project-cli/floor.obj","entity":id(4),"name":"floor","material":id(3)}}));
        self.apply(
            "cli:animate:0001",
            read(repo().join("fixtures/project-cli/animate.json")),
        );
    }
}
impl Drop for Test {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn shutter() -> Value {
    json!({"open":time(-1,32),"close":time(1,32),"samples":2})
}

#[test]
fn general_cli_authors_models_animates_renders_and_recovers_without_demo_commands() {
    let t = Test::new();
    t.author();
    let before = t.inspect();
    let revision = &before["revision"];
    let mut digests = vec![];
    for numerator in [0, 1, 2, 1] {
        let evaluation=t.call(json!({"method":"evaluate","revision":revision,"at":{"clip":id(10),"time":time(numerator,2)}}));
        let instance = evaluation["instances"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == id(1))
            .unwrap();
        let expected = -0.6 + 0.6 * numerator as f64;
        assert!((instance["transform"][3][0].as_f64().unwrap() - expected).abs() < 1e-12);
        digests.push(evaluation["evaluation_revision"].clone());
    }
    assert_eq!(digests[1], digests[3]);
    assert_ne!(digests[0], digests[2]);
    let static_out = t.root.join("static");
    t.call(json!({"method":"render","revision":revision,"backend":"cpu","output":static_out}));
    assert_eq!(read(static_out.join("manifest.json"))["status"], "complete");
    let frame = t.root.join("frame");
    let frame_request =
        json!({"revision":revision,"clip":id(10),"time":time(1,2),"shutter":shutter()});
    t.call(json!({"method":"frame","request":frame_request,"output":frame}));
    let sequence = t.root.join("sequence");
    t.call(json!({"method":"sequence","request":{"revision":revision,"clip":id(10),"times":[time(0,1),time(1,2),time(1,1)],"shutter":shutter()},"output":sequence}));
    let manifest = read(sequence.join("manifest.json"));
    assert_eq!(manifest["bundles"].as_array().unwrap().len(), 3);
    assert_eq!(
        read(sequence.join("frame-000001/receipt.json"))["output_digest"],
        read(frame.join("frame/receipt.json"))["output_digest"]
    );
    let products = t.root.join("products");
    t.call(json!({"method":"products","revision":revision,"output":products}));
    assert_eq!(
        read(products.join("manifest.json"))["bundles"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let b = read(products.join("bake-0000/bake.json"));
    assert!(b["covered"].as_array().unwrap().iter().all(|x| x == true));
    let exported = t.root.join("export");
    t.call(json!({"method":"export","revision":revision,"output":exported,"content":{"format":"document"}}));
    let restored = t.root.join("restored");
    let out=t.wire(&restored,json!({"version":0,"operation":{"method":"restore","source":exported.join("document/document.json")}}));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = t.wire(
        &restored,
        json!({"version":0,"operation":{"method":"inspect"}}),
    );
    assert!(out.status.success());
    let reopened: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(reopened["document_digest"], before["document_digest"]);
    let obj = t.root.join("obj");
    t.call(json!({"method":"export","revision":revision,"output":obj,"content":{"format":"obj","entity":id(1),"at":{"clip":id(10),"time":time(1,1)}}}));
    assert!(
        fs::read_to_string(obj.join("mesh/mesh.obj"))
            .unwrap()
            .contains("\nf ")
    );
    assert!(
        !read(obj.join("mesh/loss.json"))["losses"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(t.inspect()["document_digest"], before["document_digest"]);
}

#[test]
fn ordinary_api_and_project_cli_produce_equivalent_deltas_and_retry_receipts() {
    let t = Test::new();
    t.init();
    let peer = t.root.join("peer");
    assert!(
        t.wire(
            &peer,
            json!({"version":0,"operation":{"method":"init","document_id":id(99)}})
        )
        .status
        .success()
    );
    let request = t.apply_request(
        "cli:equivalent:01",
        read(repo().join("fixtures/project-cli/create.json")),
    );
    let cli = t.call(json!({"method":"apply","request":request}));
    let mut api = Command::new(env!("CARGO_BIN_EXE_render-host"))
        .current_dir(repo())
        .arg("api")
        .arg(&peer)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = serde_json::to_vec(&request).unwrap();
    line.push(b'\n');
    api.stdin.take().unwrap().write_all(&line).unwrap();
    let out = api.wait_with_output().unwrap();
    assert!(out.status.success());
    let other: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(other["Ok"], cli["receipt"]);
    let digest = t.inspect()["document_digest"].clone();
    let retry = t.call(json!({"method":"apply","request":request}));
    assert_eq!(retry, cli);
    assert_eq!(t.inspect()["document_digest"], digest);
    let mut changed = request;
    changed["commands"][0]["material"]["base_color"] = json!([1, 0, 0]);
    t.fail(
        json!({"version":0,"operation":{"method":"apply","request":changed}}),
        "idempotency_mismatch",
    );
}

#[test]
fn stale_invalid_cancelled_and_existing_paths_never_replace_authored_state() {
    let t = Test::new();
    t.fail(
        json!({"version":0,"operation":{"method":"inspect"}}),
        "not_found",
    );
    assert!(!t.project.exists());
    t.author();
    let before = t.inspect();
    t.fail(
        json!({"version":1,"operation":{"method":"inspect"}}),
        "operation_version",
    );
    let missing = t.root.join("missing-output");
    t.fail(json!({"version":0,"operation":{"method":"render","revision":"stale","backend":"cpu","output":missing}}),"stale_revision");
    assert!(!missing.exists());
    t.fail(
        json!({"version":0,"operation":{"method":"init","document_id":id(88)}}),
        "output_exists",
    );
    let cancel = t.root.join("cancel");
    fs::write(&cancel, []).unwrap();
    for op in [
        json!({"method":"evaluate","revision":before["revision"]}),
        json!({"method":"apply","request":t.apply_request("cli:cancel:00001",json!([{"operation":"rename","entity":id(1),"name":"cancelled"}]))}),
    ] {
        t.fail(
            json!({"version":0,"limits":{"cancel_file":cancel},"operation":op}),
            "cancelled",
        );
    }
    let bad = t.root.join("bad.json");
    fs::write(&bad, b"not a native document").unwrap();
    let restore = t.root.join("bad-restore");
    assert!(
        !t.wire(
            &restore,
            json!({"version":0,"operation":{"method":"restore","source":bad}})
        )
        .status
        .success()
    );
    assert!(!restore.exists());
    let out = t.root.join("budget");
    t.fail(json!({"version":0,"limits":{"max_output_bytes":32},"operation":{"method":"render","revision":before["revision"],"backend":"cpu","output":out}}),"budget");
    assert_eq!(read(out.join("manifest.json"))["status"], "failed");
    assert!(
        read(out.join("manifest.json"))["bundles"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let saved = fs::read(out.join("manifest.json")).unwrap();
    t.fail(json!({"version":0,"operation":{"method":"render","revision":before["revision"],"backend":"cpu","output":out}}),"output_exists");
    assert_eq!(fs::read(out.join("manifest.json")).unwrap(), saved);
    let malformed = json!({"version":0,"operation":{"method":"inspect","extra":true}});
    assert!(!t.wire(&t.project, malformed).status.success());
    assert_eq!(before["document_digest"], t.inspect()["document_digest"]);
}

#[test]
fn import_files_are_explicit_bounded_transactional_and_retryable() {
    let t = Test::new();
    t.init();
    let before = t.inspect();
    let request = json!({"version":0,"operation":{"method":"import","base_revision":before["revision"],"idempotency_key":"cli:glb:00000001","max_added_bytes":8*1024*1024,"source":{"format":"glb","path":"fixtures/static-pbr/BoxTextured/BoxTextured.glb","policy":{"allow_approximations":true}}}});
    let imported = t.success(request.clone());
    assert!(imported["import"]["instances"].as_u64().unwrap() > 0);
    assert_eq!(t.success(request), imported);
    let current = t.inspect();
    let base = json!({"method":"import","base_revision":current["revision"],"idempotency_key":"cli:bad:00000001","max_added_bytes":8*1024*1024,"source":{"format":"gltf","path":"fixtures/static-pbr/BoxTextured/BoxTextured.gltf","buffers":[],"images":[],"policy":{"allow_approximations":true}}});
    assert!(
        !t.wire(&t.project, json!({"version":0,"operation":base}))
            .status
            .success()
    );
    t.fail(json!({"version":0,"limits":{"max_input_bytes":1024},"operation":{"method":"import","base_revision":current["revision"],"idempotency_key":"cli:budget:00001","max_added_bytes":8*1024*1024,"source":{"format":"glb","path":"fixtures/static-pbr/BoxTextured/BoxTextured.glb","policy":{"allow_approximations":true}}}}),"budget");
    assert_eq!(t.inspect()["document_digest"], current["document_digest"]);
    let fresh = Test::new();
    fresh.init();
    let imported=fresh.call(json!({"method":"import","base_revision":fresh.inspect()["revision"],"idempotency_key":"cli:gltf:0000001","max_added_bytes":8*1024*1024,"source":{"format":"gltf","path":"fixtures/static-pbr/BoxTextured/BoxTextured.gltf","buffers":["fixtures/static-pbr/BoxTextured/BoxTextured0.bin"],"images":["fixtures/static-pbr/BoxTextured/CesiumLogoFlat.png"],"policy":{"allow_approximations":true}}}));
    assert!(imported["import"]["instances"].as_u64().unwrap() > 0);
}

#[test]
fn sequence_cancellation_preserves_completed_bundles_and_source_revision() {
    let t = Test::new();
    t.author();
    let mut settings = t.inspect()["snapshot"]["render_settings"].clone();
    settings["width"] = json!(96);
    settings["height"] = json!(64);
    settings["samples"] = json!(16);
    t.apply(
        "cli:long:0000001",
        json!([{"operation":"set_render_settings","settings":settings}]),
    );
    let before = t.inspect();
    let output = t.root.join("partial");
    let cancel = t.root.join("stop");
    let request = json!({"version":0,"limits":{"cancel_file":cancel,"max_wall_ms":10000},"operation":{"method":"sequence","request":{"revision":before["revision"],"clip":id(10),"times":(0..100).map(|n|time(n,100)).collect::<Vec<_>>(),"shutter":shutter()},"output":output}});
    let mut child = Command::new(env!("CARGO_BIN_EXE_render-host"))
        .current_dir(repo())
        .arg("project")
        .arg(&t.project)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    let start = Instant::now();
    while !output.join("frame-000000").exists() {
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "first frame did not publish"
        );
        assert!(
            child.try_wait().unwrap().is_none(),
            "sequence exited before first frame"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    fs::write(&cancel, b"cancel").unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stderr).unwrap()["code"],
        "cancelled"
    );
    let manifest = read(output.join("manifest.json"));
    assert_eq!(manifest["status"], "cancelled");
    let bundles = manifest["bundles"].as_array().unwrap();
    assert!(!bundles.is_empty() && bundles.len() < 100);
    for bundle in bundles {
        for a in bundle["artifacts"].as_array().unwrap() {
            assert_eq!(
                fs::metadata(output.join(a["path"].as_str().unwrap()))
                    .unwrap()
                    .len(),
                a["bytes"].as_u64().unwrap()
            );
        }
    }
    assert_eq!(t.inspect()["document_digest"], before["document_digest"]);
}

#[test]
fn gpu_profile_preflight_deadline_and_authored_names_have_safe_output_semantics() {
    let t = Test::new();
    t.author();
    let mut imaging = t.inspect()["snapshot"]["imaging"].clone();
    imaging["views"][0]["name"] = json!("../escape");
    t.apply(
        "cli:paths:000001",
        json!([{"operation":"set_imaging","imaging":imaging}]),
    );
    let products = t.root.join("safe-products");
    t.call(json!({"method":"products","revision":t.inspect()["revision"],"output":products}));
    assert!(products.join("view-0000/display.json").exists());
    assert!(!t.root.join("escape").exists());
    let out=t.wire(&t.project,json!({"version":0,"operation":{"method":"export","revision":t.inspect()["revision"],"output":t.root.join("unknown"),"content":{"format":"document","ignored":true}}}));
    assert!(!out.status.success());
    assert!(!t.root.join("unknown").exists());
    let mut material = t.inspect()["snapshot"]["materials"][id(2)].clone();
    // Keep this preflight test on a valid, still-unsupported GPU material.
    material["roughness"] = json!(0);
    material["metallic"] = json!(0);
    material["pbr"] = json!({"advanced":{"model":{"kind":"dielectric","ior":1.5},"opacity":{"kind":"opaque"}},"double_sided":false,"base_color":null,"metallic_roughness":null,"normal":null,"emission":null,"occlusion":null,"normal_scale":1,"occlusion_strength":1});
    t.apply(
        "cli:surface:0001",
        json!([{"operation":"put_material","material":material}]),
    );
    let state = t.inspect();
    let gpu = t.root.join("unsupported");
    t.fail(json!({"version":0,"operation":{"method":"render","revision":state["revision"],"backend":"gpu","output":gpu}}),"unsupported_profile");
    assert_eq!(read(gpu.join("manifest.json"))["status"], "failed");
    let mut settings = state["snapshot"]["render_settings"].clone();
    settings["width"] = json!(128);
    settings["height"] = json!(96);
    settings["samples"] = json!(1024);
    t.apply(
        "cli:deadline:001",
        json!([{"operation":"set_render_settings","settings":settings}]),
    );
    let before = t.inspect();
    t.fail(json!({"version":0,"limits":{"max_wall_ms":25},"operation":{"method":"render","revision":before["revision"],"backend":"cpu","output":t.root.join("timed")}}),"cancelled");
    assert_eq!(t.inspect()["document_digest"], before["document_digest"]);
}

#[test]
fn animated_glb_import_is_retryable_cancellable_and_preserves_existing_animation() {
    let t = Test::new();
    t.author();
    let mut settings = t.inspect()["snapshot"]["render_settings"].clone();
    settings["max_depth"] = json!(1);
    t.apply(
        "cli:animated-settings:01",
        json!([{"operation":"set_render_settings","settings":settings}]),
    );
    let before = t.inspect();
    let operation = json!({"method":"import","base_revision":before["revision"],"idempotency_key":"cli:animated:0001","max_added_bytes":8*1024*1024,"source":{"format":"glb","path":"fixtures/animated-gltf/character.glb","policy":{"allow_approximations":true}}});
    let cancelled = t.root.join("cancel-import");
    fs::write(&cancelled, "cancel").unwrap();
    t.fail(
        json!({"version":0,"limits":{"cancel_file":cancelled},"operation":operation}),
        "cancelled",
    );
    assert_eq!(t.inspect()["document_digest"], before["document_digest"]);
    let result = t.call(operation.clone());
    assert_eq!(result["import"]["profile"], "gltf2-animated-pbr-v0");
    assert_eq!(t.call(operation), result);
    let after = t.inspect();
    assert_eq!(
        after["snapshot"]["animation"]["clips"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    let clip = &result["import"]["source_clips"]["0"];
    t.call(json!({"method":"evaluate","revision":after["revision"],"at":{"clip":clip,"time":time(1,2)}}));
    t.call(json!({"method":"render","revision":after["revision"],"backend":"cpu","at":{"clip":clip,"time":time(1,2)},"output":t.root.join("animated-image")}));
    t.fail(json!({"version":0,"operation":{"method":"evaluate","revision":before["revision"],"at":{"clip":clip,"time":time(1,2)}}}),"stale_revision");
    assert_eq!(t.inspect()["document_digest"], after["document_digest"]);
}
