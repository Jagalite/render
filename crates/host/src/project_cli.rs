//! One-shot typed project operations. Filesystem paths stay in the native adapter.
use render_core::{
    document::*,
    render::*,
    storage::{native::NativeStore, *},
    *,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    #[serde(default)]
    limits: Limits,
    operation: Operation,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Limits {
    #[serde(default = "input_limit")]
    max_input_bytes: u64,
    #[serde(default = "output_limit")]
    max_output_bytes: u64,
    #[serde(default = "wall_limit")]
    max_wall_ms: u64,
    #[serde(default)]
    cancel_file: Option<PathBuf>,
}
fn input_limit() -> u64 {
    64 * 1024 * 1024
}
fn output_limit() -> u64 {
    64 * 1024 * 1024
}
fn wall_limit() -> u64 {
    60000
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: input_limit(),
            max_output_bytes: output_limit(),
            max_wall_ms: wall_limit(),
            cancel_file: None,
        }
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    clip: Id,
    time: Time,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Backend {
    #[default]
    Cpu,
    Gpu,
}
#[derive(Deserialize)]
#[serde(tag = "format", rename_all = "snake_case", deny_unknown_fields)]
enum Source {
    Blend {
        path: PathBuf,
        policy: blend::Policy,
    },
    Hair {
        path: PathBuf,
        policy: hair_import::Policy,
    },
    Vol {
        path: PathBuf,
        emission: Option<PathBuf>,
        policy: volume_import::Policy,
    },
    Obj {
        path: PathBuf,
        entity: Id,
        name: String,
        material: Option<Id>,
    },
    Glb {
        path: PathBuf,
        policy: gltf_scene::PbrPolicy,
    },
    Gltf {
        path: PathBuf,
        buffers: Vec<PathBuf>,
        images: Vec<PathBuf>,
        policy: gltf_scene::PbrPolicy,
    },
}
#[derive(Deserialize)]
#[serde(tag = "format", rename_all = "snake_case", deny_unknown_fields)]
enum Export {
    Source {
        asset: String,
    },
    Document {},
    Obj {
        entity: Id,
        at: Option<Sample>,
    },
    Glb {
        at: Option<gltf_export::Sample>,
        policy: gltf_export::Policy,
    },
}
#[derive(Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Init {
        document_id: Id,
    },
    Restore {
        source: PathBuf,
    },
    Inspect {},
    QueryGeometry {
        request: geometry_query::Request,
    },
    Apply {
        request: document::Request,
    },
    AuthorPaint {
        request: Box<painting::Request>,
    },
    BakePaint {
        request: Box<painting::BakeRequest>,
    },
    AuthorUv {
        request: Box<uv::Request>,
    },
    Import {
        base_revision: String,
        idempotency_key: String,
        max_added_bytes: u64,
        source: Source,
        settings: Option<Box<Settings>>,
    },
    Evaluate {
        revision: String,
        at: Option<Sample>,
    },
    Render {
        revision: String,
        at: Option<Sample>,
        backend: Backend,
        output: PathBuf,
    },
    Frame {
        #[serde(default)]
        backend: Backend,
        request: sequence::FrameRequest,
        output: PathBuf,
    },
    Sequence {
        #[serde(default)]
        backend: Backend,
        request: sequence::SequenceRequest,
        output: PathBuf,
    },
    Products {
        revision: String,
        output: PathBuf,
    },
    Export {
        revision: String,
        output: PathBuf,
        content: Export,
    },
}
struct Control {
    limits: Limits,
    start: Instant,
    input_bytes: u64,
}
impl Control {
    fn cancelled(&self) -> bool {
        self.start.elapsed().as_millis() >= u128::from(self.limits.max_wall_ms)
            || self.limits.cancel_file.as_ref().is_some_and(|p| p.exists())
    }
    fn check(&self) -> Result<()> {
        if self.cancelled() {
            Err(Error::new(
                "cancelled",
                "project operation cancelled or wall budget exceeded",
            ))
        } else {
            Ok(())
        }
    }
    fn read(&mut self, path: &Path) -> Result<Vec<u8>> {
        self.read_limited(path, u64::MAX)
    }
    fn read_limited(&mut self, path: &Path, source_limit: u64) -> Result<Vec<u8>> {
        self.check()?;
        let remaining =
            source_limit.min(self.limits.max_input_bytes.saturating_sub(self.input_bytes));
        let mut bytes = vec![];
        File::open(path)
            .map_err(io)?
            .take(remaining.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(io)?;
        self.input_bytes += bytes.len() as u64;
        if bytes.len() as u64 > remaining {
            return Err(Error::new(
                "budget",
                "source or aggregate project input byte limit exceeded",
            ));
        }
        self.check()?;
        Ok(bytes)
    }
}
fn io(e: std::io::Error) -> Error {
    Error::new("io", e.to_string())
}
// Keep the ordinary durable transaction engine, with a cancellation check at
// its publication boundary after candidate preparation has finished.
struct CancellableStore<'a> {
    store: &'a mut NativeStore,
    control: &'a Control,
}
impl JournalStore for CancellableStore<'_> {
    fn load(&self) -> Result<Vec<Envelope>> {
        self.store.load()
    }
    fn publish(&mut self, envelope: &Envelope) -> Result<()> {
        self.control.check()?;
        self.store.publish(envelope)
    }
}
struct FinishFlag<'a>(&'a AtomicBool);
impl Drop for FinishFlag<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
fn principal() -> Principal {
    Principal {
        id: "local-project-cli".into(),
        can_write: true,
    }
}
fn load(root: &Path) -> Result<(NativeStore, Document)> {
    if !root.is_dir() {
        return Err(Error::new("not_found", "project directory does not exist"));
    }
    let store = NativeStore::open(root)?;
    let doc = recover(&store.load()?)?
        .ok_or_else(|| Error::new("not_found", "project has no committed document"))?;
    Ok((store, doc))
}
fn pinned(doc: &Document, revision: &str) -> Result<()> {
    if doc.snapshot().revision()? != revision {
        return Err(Error::new(
            "stale_revision",
            "project revision differs from request",
        ));
    }
    Ok(())
}
fn new_project(root: &Path, doc: Document, control: &Control) -> Result<Value> {
    doc.snapshot().validate()?;
    control.check()?;
    fs::create_dir(root).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            Error::new("output_exists", "project path already exists")
        } else {
            io(e)
        }
    })?;
    let mut store = NativeStore::open(root)?;
    CancellableStore {
        store: &mut store,
        control,
    }
    .publish(&Envelope::new(0, None, &doc)?)?;
    Ok(
        json!({"revision":doc.snapshot().revision()?,"document_id":doc.snapshot().document_id,"durable":true}),
    )
}
fn evaluated(
    doc: &Document,
    at: Option<&Sample>,
    control: &Control,
) -> Result<(Scene, Option<animation::Receipt>)> {
    let mut evaluator = Evaluator::default();
    if let Some(at) = at {
        let (s, r) =
            evaluator.evaluate_at(doc.snapshot(), at.clip, at.time, || control.cancelled())?;
        Ok((s, Some(r)))
    } else {
        Ok((
            evaluator.evaluate_with_cancel(doc.snapshot(), || control.cancelled())?,
            None,
        ))
    }
}

#[derive(Serialize)]
struct Artifact {
    path: String,
    bytes: u64,
    digest: String,
}
struct Output {
    root: PathBuf,
    revision: String,
    limit: u64,
    bytes: u64,
    bundles: Vec<Value>,
}
impl Output {
    fn new(root: PathBuf, revision: String, limit: u64) -> Result<Self> {
        fs::create_dir(&root).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Error::new("output_exists", "artifact directory already exists")
            } else {
                io(e)
            }
        })?;
        Ok(Self {
            root,
            revision,
            limit,
            bytes: 0,
            bundles: vec![],
        })
    }
    fn bundle(
        &mut self,
        name: &str,
        files: Vec<(&str, Vec<u8>)>,
        metadata: Value,
        control: &Control,
    ) -> Result<()> {
        control.check()?;
        let size = files.iter().map(|(_, b)| b.len() as u64).sum::<u64>();
        if size > self.limit.saturating_sub(self.bytes) {
            return Err(Error::new("budget", "artifact byte limit exceeded"));
        }
        let pending = self.root.join(format!(".{name}.pending"));
        fs::create_dir(&pending).map_err(io)?;
        let mut artifacts = vec![];
        for (file, bytes) in files {
            control.check()?;
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(pending.join(file))
                .map_err(io)?;
            f.write_all(&bytes).map_err(io)?;
            f.sync_all().map_err(io)?;
            artifacts.push(Artifact {
                path: format!("{name}/{file}"),
                bytes: bytes.len() as u64,
                digest: digest(&bytes),
            });
        }
        sync_directory(&pending)?;
        control.check()?;
        fs::rename(&pending, self.root.join(name)).map_err(io)?;
        sync_directory(&self.root)?;
        self.bytes += size;
        self.bundles
            .push(json!({"name":name,"metadata":metadata,"artifacts":artifacts}));
        Ok(())
    }
    fn image(
        &mut self,
        name: &str,
        image: &Image,
        metadata: Value,
        control: &Control,
    ) -> Result<()> {
        self.bundle(
            name,
            vec![
                ("image.ppm", image.ppm()),
                ("image.pfm", image.pfm()),
                ("receipt.json", canonical(&image.receipt)?),
                (
                    "passes.json",
                    canonical(&(
                        image.width,
                        image.height,
                        &image.depth,
                        &image.normals,
                        &image.objects,
                    ))?,
                ),
            ],
            metadata,
            control,
        )
    }
    fn finish(self, result: Result<()>) -> Result<Value> {
        let status = match &result {
            Ok(_) => "complete",
            Err(e) if e.code == "cancelled" => "cancelled",
            Err(_) => "failed",
        };
        let manifest = json!({"version":0,"status":status,"authored_revision":self.revision,"artifact_bytes":self.bytes,"bundles":self.bundles,"error":result.as_ref().err()});
        let bytes = canonical(&manifest)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join("manifest.pending"))
            .map_err(io)?;
        file.write_all(&bytes).map_err(io)?;
        file.sync_all().map_err(io)?;
        fs::rename(
            self.root.join("manifest.pending"),
            self.root.join("manifest.json"),
        )
        .map_err(io)?;
        sync_directory(&self.root)?;
        result.map_err(|e| {
            e.with_context(
                "output_manifest",
                self.root.join("manifest.json").display().to_string(),
            )
        })?;
        Ok(
            json!({"status":"complete","revision":self.revision,"manifest":self.root.join("manifest.json"),"artifact_bytes":self.bytes}),
        )
    }
}
fn sync_directory(path: &Path) -> Result<()> {
    // Windows cannot open directories through std::fs::File; rename remains the
    // visibility boundary there. Crash durability is validated on macOS only.
    #[cfg(unix)]
    File::open(path).map_err(io)?.sync_all().map_err(io)?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
fn gpu(scene: &Scene, settings: &Settings, control: &Control) -> Result<Image> {
    control.check()?;
    // Reject unsupported scene profiles before attempting native device creation.
    render_gpu::pack(scene, settings, 0)?;
    with_gpu(control, |renderer, cancelled| {
        pollster::block_on(renderer.render(scene, settings, 0, cancelled))
    })
}
fn with_gpu<T>(
    control: &Control,
    run: impl FnOnce(&mut render_gpu::Renderer, &AtomicBool) -> Result<T>,
) -> Result<T> {
    control.check()?;
    let cancelled = AtomicBool::new(false);
    let done = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !done.load(Ordering::Acquire) {
                if control.cancelled() {
                    cancelled.store(true, Ordering::Release);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let _finished = FinishFlag(&done);
        let mut renderer = pollster::block_on(render_gpu::Renderer::new())?;
        run(&mut renderer, &cancelled)
    })
}

pub fn run(root: &Path, input: &str) -> Result<Value> {
    let mut bytes = vec![];
    if input == "-" {
        std::io::stdin()
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(io)?;
    } else {
        File::open(input)
            .map_err(io)?
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(io)?;
    }
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(Error::new("budget", "request exceeds 16 MiB"));
    }
    let request: Request = serde_json::from_slice(&bytes)?;
    if request.version != 0 {
        return Err(Error::new(
            "operation_version",
            "project CLI supports version 0",
        ));
    }
    let l = &request.limits;
    if l.max_input_bytes == 0
        || l.max_input_bytes > 128 * 1024 * 1024
        || l.max_output_bytes == 0
        || l.max_output_bytes > 512 * 1024 * 1024
        || l.max_wall_ms == 0
        || l.max_wall_ms > 600000
    {
        return Err(Error::new("budget", "invalid project CLI limits"));
    }
    let mut control = Control {
        limits: request.limits,
        start: Instant::now(),
        input_bytes: bytes.len() as u64,
    };
    if control.input_bytes > control.limits.max_input_bytes {
        return Err(Error::new(
            "budget",
            "request exceeds aggregate input limit",
        ));
    }
    control.check()?;
    match request.operation {
        Operation::Init { document_id } => {
            new_project(root, Document::new(Snapshot::empty(document_id))?, &control)
        }
        Operation::Restore { source } => {
            let doc: Document = serde_json::from_slice(&control.read(&source)?)?;
            new_project(root, doc, &control)
        }
        operation => execute(root, operation, &mut control),
    }
}
fn execute(root: &Path, operation: Operation, control: &mut Control) -> Result<Value> {
    let (mut store, mut doc) = load(root)?;
    control.check()?;
    match operation {
        Operation::Init { .. } | Operation::Restore { .. } => {
            Err(Error::new("operation", "creation must use a fresh project"))
        }
        Operation::Inspect {} => Ok(
            json!({"revision":doc.snapshot().revision()?,"document_digest":digest(&canonical(&doc)?),"snapshot":doc.snapshot(),"operations":api::registry(),"durability":"native journal"}),
        ),
        Operation::QueryGeometry { request } => Ok(serde_json::to_value(
            geometry_query::Cache::default()
                .query(doc.snapshot(), &request, || control.cancelled())?,
        )?),
        Operation::Apply { request } => {
            control.check()?;
            let receipt = durable_execute(
                &mut CancellableStore {
                    store: &mut store,
                    control,
                },
                &mut doc,
                &principal(),
                &request,
            )?;
            Ok(json!({"receipt":receipt}))
        }
        Operation::AuthorPaint { request } => {
            let prepared = painting::prepare(&doc, &principal(), &request, || control.cancelled())?;
            control.check()?;
            let receipt = durable_execute(
                &mut CancellableStore {
                    store: &mut store,
                    control,
                },
                &mut doc,
                &principal(),
                &prepared.transaction,
            )?;
            Ok(json!({"receipt":receipt,"report":prepared.report}))
        }
        Operation::BakePaint { request } => {
            let prepared =
                painting::prepare_bake(&doc, &principal(), &request, || control.cancelled())?;
            control.check()?;
            let receipt = durable_execute(
                &mut CancellableStore {
                    store: &mut store,
                    control,
                },
                &mut doc,
                &principal(),
                &prepared.transaction,
            )?;
            Ok(json!({"receipt":receipt,"report":prepared.report}))
        }
        Operation::AuthorUv { request } => {
            let prepared = uv::prepare(&doc, &principal(), &request, || control.cancelled())?;
            control.check()?;
            let receipt = durable_execute(
                &mut CancellableStore {
                    store: &mut store,
                    control,
                },
                &mut doc,
                &principal(),
                &prepared.transaction,
            )?;
            Ok(json!({"receipt":receipt,"report":prepared.report,"uv_asset":prepared.uv_asset}))
        }
        Operation::Import {
            base_revision,
            idempotency_key,
            max_added_bytes,
            source,
            mut settings,
        } => {
            let (mut commands, report) = match source {
                Source::Blend { path, policy } => {
                    let bytes = control.read_limited(&path, blend::MAX_INPUT_BYTES as u64)?;
                    let initial = settings
                        .take()
                        .map(|s| *s)
                        .or_else(|| doc.snapshot().render_settings.clone())
                        .ok_or_else(|| {
                            Error::new(
                                "render_settings",
                                "blend import requires caller sampling/output settings",
                            )
                        })?;
                    let imported = blend::import(
                        &bytes,
                        doc.snapshot().document_id,
                        &policy,
                        initial,
                        || control.cancelled(),
                    )?;
                    (imported.commands, json!(imported.report))
                }
                Source::Hair { path, policy } => {
                    let bytes = control.read_limited(&path, hair_import::MAX_INPUT_BYTES as u64)?;
                    let imported =
                        hair_import::import(&bytes, doc.snapshot().document_id, &policy, || {
                            control.cancelled()
                        })?;
                    (imported.commands, json!(imported.report))
                }
                Source::Vol {
                    path,
                    emission,
                    policy,
                } => {
                    let bytes =
                        control.read_limited(&path, volume_import::MAX_INPUT_BYTES as u64)?;
                    let emission = emission
                        .as_ref()
                        .map(|p| {
                            control.read_limited(
                                p,
                                (volume_import::MAX_INPUT_BYTES - bytes.len()) as u64,
                            )
                        })
                        .transpose()?;
                    let imported = volume_import::import(
                        &bytes,
                        emission.as_deref(),
                        doc.snapshot().document_id,
                        &policy,
                        || control.cancelled(),
                    )?;
                    (imported.commands, json!(imported.report))
                }
                Source::Obj {
                    path,
                    entity,
                    name,
                    material,
                } => {
                    let bytes = control.read(&path)?;
                    let text = std::str::from_utf8(&bytes)
                        .map_err(|e| Error::new("encoding", e.to_string()))?;
                    let (mesh, loss) = interchange::import_obj(text)?;
                    let key = mesh.content_id()?;
                    (
                        vec![
                            Command::PutMesh { mesh },
                            Command::CreateEntity {
                                entity: Entity {
                                    id: entity,
                                    name,
                                    parent: None,
                                    mesh: Some(key),
                                    material,
                                    transform: Transform::default(),
                                },
                            },
                        ],
                        json!(loss),
                    )
                }
                Source::Glb { path, policy } => {
                    let imported = gltf_scene::import_pbr_glb(
                        &control.read(&path)?,
                        doc.snapshot().document_id,
                        &policy,
                    )?;
                    (imported.commands, json!(imported.report))
                }
                Source::Gltf {
                    path,
                    buffers,
                    images,
                    policy,
                } => {
                    if buffers.len() > 256 || images.len() > 256 {
                        return Err(Error::new("budget", "resource table exceeds 256 entries"));
                    }
                    let json = control.read(&path)?;
                    let buffers = buffers
                        .iter()
                        .map(|p| control.read(p))
                        .collect::<Result<Vec<_>>>()?;
                    let images = images
                        .iter()
                        .map(|p| control.read(p))
                        .collect::<Result<Vec<_>>>()?;
                    let imported = gltf_scene::import_pbr_scene(
                        &json,
                        &buffers,
                        &images,
                        doc.snapshot().document_id,
                        &policy,
                    )?;
                    (imported.commands, json!(imported.report))
                }
            };
            if let Some(settings) = settings {
                commands.push(Command::SetRenderSettings {
                    settings: *settings,
                });
            }
            control.check()?;
            let receipt = durable_execute(
                &mut CancellableStore {
                    store: &mut store,
                    control,
                },
                &mut doc,
                &principal(),
                &document::Request {
                    version: 0,
                    base_revision,
                    idempotency_key,
                    commands,
                    max_added_bytes,
                },
            )?;
            Ok(json!({"receipt":receipt,"import":report}))
        }
        Operation::Evaluate { revision, at } => {
            pinned(&doc, &revision)?;
            let (scene, receipt) = evaluated(&doc, at.as_ref(), control)?;
            Ok(
                json!({"authored_revision":revision,"evaluation_revision":scene.revision,"animation":receipt,"geometry_builds":scene.geometry_builds,"instances":scene.instances.iter().map(|i|json!({"id":i.id,"triangles":i.geometry.triangles.len(),"bounds":[i.bounds.min.to_array(),i.bounds.max.to_array()],"transform":i.transform.to_cols_array_2d()})).collect::<Vec<_>>(),"conversions":scene.conversions,"displacements":scene.displacements,"procedures":scene.procedures,"media_cells":scene.media.cell_count()}),
            )
        }
        Operation::Render {
            revision,
            at,
            backend,
            output,
        } => {
            pinned(&doc, &revision)?;
            let mut out = Output::new(output, revision, control.limits.max_output_bytes)?;
            let result = (|| {
                let (scene, receipt) = evaluated(&doc, at.as_ref(), control)?;
                let settings = doc.snapshot().render_settings.as_ref().ok_or_else(|| {
                    Error::new("render_settings", "project has no render settings")
                })?;
                let image = match backend {
                    Backend::Cpu => render(&scene, settings, || control.cancelled())?,
                    Backend::Gpu => gpu(&scene, settings, control)?,
                };
                out.image("image", &image, json!({"animation":receipt}), control)
            })();
            out.finish(result)
        }
        Operation::Frame {
            request,
            output,
            backend,
        } => {
            pinned(&doc, &request.revision)?;
            let mut out = Output::new(
                output,
                request.revision.clone(),
                control.limits.max_output_bytes,
            )?;
            let result = (|| {
                let frame = match backend {
                    Backend::Cpu => {
                        sequence::render_frame(doc.snapshot(), &request, || control.cancelled())?
                    }
                    Backend::Gpu => with_gpu(control, |gpu, cancelled| {
                        pollster::block_on(gpu.render_frame(doc.snapshot(), &request, cancelled))
                    })?,
                };
                out.image(
                    "frame",
                    &frame.image,
                    json!({"evaluation":frame.evaluation,"temporal_times":frame.temporal_times}),
                    control,
                )
            })();
            out.finish(result)
        }
        Operation::Sequence {
            request,
            output,
            backend,
        } => {
            pinned(&doc, &request.revision)?;
            let mut out = Output::new(
                output,
                request.revision.clone(),
                control.limits.max_output_bytes,
            )?;
            let emit = |i: usize, frame: &sequence::Frame| {
                out.image(
                    &format!("frame-{i:06}"),
                    &frame.image,
                    json!({"evaluation":frame.evaluation,"temporal_times":frame.temporal_times}),
                    control,
                )
            };
            let result = match backend {
                Backend::Cpu => sequence::render_sequence(doc.snapshot(), &request, emit, || {
                    control.cancelled()
                }),
                Backend::Gpu => with_gpu(control, |gpu, cancelled| {
                    pollster::block_on(gpu.render_sequence(
                        doc.snapshot(),
                        &request,
                        emit,
                        cancelled,
                    ))
                }),
            }
            .map(|_| ());
            out.finish(result)
        }
        Operation::Products { revision, output } => {
            pinned(&doc, &revision)?;
            let mut out = Output::new(output, revision.clone(), control.limits.max_output_bytes)?;
            let result = (|| {
                let products =
                    products::render_products(doc.snapshot(), &revision, || control.cancelled())?;
                for (i, view) in products.views.iter().enumerate() {
                    out.bundle(
                        &format!("view-{i:04}"),
                        vec![
                            ("image.ppm", view.raw.ppm()),
                            ("image.pfm", view.raw.pfm()),
                            ("receipt.json", canonical(&view.raw.receipt)?),
                            (
                                "passes.json",
                                canonical(&(
                                    view.raw.width,
                                    view.raw.height,
                                    &view.raw.depth,
                                    &view.raw.normals,
                                    &view.raw.objects,
                                ))?,
                            ),
                            ("display.json", canonical(&view.display)?),
                            ("albedo.json", canonical(&view.albedo)?),
                        ],
                        json!({"view":view.name}),
                        control,
                    )?;
                }
                for (i, (name, bake)) in products.bakes.iter().enumerate() {
                    out.bundle(
                        &format!("bake-{i:04}"),
                        vec![("bake.json", canonical(bake)?)],
                        json!({"bake":name}),
                        control,
                    )?;
                }
                Ok(())
            })();
            out.finish(result)
        }
        Operation::Export {
            revision,
            output,
            content,
        } => {
            pinned(&doc, &revision)?;
            let mut out = Output::new(output, revision, control.limits.max_output_bytes)?;
            let result = (|| match content {
                Export::Source { asset } => {
                    let exported = source::export(
                        doc.snapshot(),
                        &source::ExportRequest {
                            revision: doc.snapshot().revision()?,
                            asset,
                        },
                        || control.cancelled(),
                    )?;
                    let report = json!({"profile":exported.profile,"source_asset":exported.source_asset,"source_digest":exported.source_digest,"version":exported.version,"bytes":exported.bytes.len(),"native_edits_applied":false});
                    out.bundle(
                        "source",
                        vec![
                            ("original.blend", exported.bytes),
                            ("report.json", canonical(&report)?),
                        ],
                        report,
                        control,
                    )
                }
                Export::Document {} => out.bundle(
                    "document",
                    vec![("document.json", canonical(&doc)?)],
                    json!({"profile":"native-document-v0","losses":[]}),
                    control,
                ),
                Export::Glb { at, policy } => {
                    let exported = gltf_export::export(
                        doc.snapshot(),
                        &gltf_export::Request {
                            revision: doc.snapshot().revision()?,
                            at,
                            policy,
                        },
                        || control.cancelled(),
                    )?;
                    out.bundle(
                        "scene",
                        vec![
                            ("scene.glb", exported.glb),
                            ("report.json", canonical(&exported.report)?),
                        ],
                        serde_json::to_value(&exported.report)?,
                        control,
                    )
                }
                Export::Obj { entity, at } => {
                    let (scene, receipt) = evaluated(&doc, at.as_ref(), control)?;
                    let inst =
                        scene
                            .instances
                            .iter()
                            .find(|i| i.id == entity)
                            .ok_or_else(|| {
                                Error::new(
                                    "reference",
                                    "OBJ export requires visible evaluated surface geometry",
                                )
                            })?;
                    let mut positions = vec![];
                    let mut faces = vec![];
                    let mut uv = vec![];
                    for t in &inst.geometry.triangles {
                        control.check()?;
                        let start = positions.len() as u32;
                        positions.extend(
                            t.positions
                                .map(|p| inst.transform.transform_point3(p).to_array()),
                        );
                        faces.push(vec![start, start + 1, start + 2]);
                        uv.extend(t.uv.map(|p| p.to_array()));
                    }
                    let mut mesh = geometry::Mesh::from_polygons(
                        geometry::Positions::F64(positions),
                        &faces,
                        &[],
                    )?;
                    mesh.attributes.insert(
                        "uv".into(),
                        geometry::Attribute {
                            id: Id(1),
                            domain: geometry::Domain::Corner,
                            semantic: "uv".into(),
                            transfer: geometry::Transfer::Linear,
                            values: geometry::AttributeValues::Vec2(uv),
                        },
                    );
                    let (text, mut loss) = interchange::export_obj(&mesh)?;
                    loss.losses.push("evaluated world-space triangles with duplicated vertices; authored topology, instances, materials, animation, procedural data and non-surface media are not exported".into());
                    out.bundle("mesh",vec![("mesh.obj",text.into_bytes()),("loss.json",canonical(&loss)?)],json!({"entity":entity,"animation":receipt,"profile":"evaluated-world-triangles-obj-v0"}),control)
                }
            })();
            out.finish(result)
        }
    }
}

pub fn help() -> &'static str {
    "render-host project <project-directory> <request.json|->
One versioned JSON request per process. Relative paths use the working directory.
Request: {\"version\":0,\"operation\":{\"method\":\"inspect\"}}
Methods: init, restore, inspect, apply, import, evaluate, render, frame, sequence, products, export.
Import formats: OBJ, GLB, glTF, VOL3, bounded HAIR and Blender293 static profiles.
Source export returns exact original .blend bytes; native edits remain in the native archive.
Mutations use existing revision-checked document transactions. Reads/renders pin revision.
Optional limits: max_input_bytes, max_output_bytes, max_wall_ms, cancel_file.
Outputs require a fresh directory; manifest.json records complete/failed/cancelled status.
Full request fields and examples: docs/project_cli.md
"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_is_checked_after_preparation_but_not_inside_admitted_commit() {
        let root =
            std::env::temp_dir().join(format!("render-cli-publication-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let cancel = root.join("cancel");
        let control = Control {
            limits: Limits {
                cancel_file: Some(cancel.clone()),
                ..Default::default()
            },
            start: Instant::now(),
            input_bytes: 0,
        };
        let mut store = NativeStore::open(root.join("project")).unwrap();
        let mut doc = Document::new(Snapshot::empty(Id(1))).unwrap();
        let initial = Envelope::new(0, None, &doc).unwrap();
        store.publish(&initial).unwrap();
        let request = document::Request {
            version: 0,
            base_revision: doc.snapshot().revision().unwrap(),
            idempotency_key: "cli:publication:01".into(),
            max_added_bytes: 1024 * 1024,
            commands: vec![Command::CreateEntity {
                entity: Entity {
                    id: Id(2),
                    name: "ordinary transaction".into(),
                    parent: None,
                    mesh: None,
                    material: None,
                    transform: Transform::default(),
                },
            }],
        };
        let candidate = doc.prepare(&principal(), &request).unwrap();
        let mut next = doc.clone();
        next.commit(candidate).unwrap();
        fs::write(&cancel, b"cancel after candidate preparation").unwrap();
        let envelope = Envelope::new(1, Some(initial.digest), &next).unwrap();
        assert_eq!(
            CancellableStore {
                store: &mut store,
                control: &control
            }
            .publish(&envelope)
            .unwrap_err()
            .code,
            "cancelled"
        );
        assert_eq!(store.load().unwrap().len(), 1);
        assert_eq!(doc.snapshot().entities.iter().count(), 0);
        fs::remove_file(&cancel).unwrap();
        store.set_boundary_hook(move |boundary| {
            if boundary == storage::native::Fault::AfterWrite {
                fs::write(&cancel, b"cancel inside admitted publication").unwrap();
            }
        });
        let receipt = durable_execute(
            &mut CancellableStore {
                store: &mut store,
                control: &control,
            },
            &mut doc,
            &principal(),
            &request,
        )
        .unwrap();
        assert!(control.cancelled() && receipt.durable);
        assert_eq!(
            recover(&store.load().unwrap())
                .unwrap()
                .unwrap()
                .snapshot()
                .revision()
                .unwrap(),
            receipt.revision
        );
        assert_eq!(doc.snapshot().entities.iter().count(), 1);
    }
}
