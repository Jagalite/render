//! Restricted agent variants over the public transaction engine. Branches and
//! previews are ephemeral; only a selected transaction mutates the root document.
use crate::{
    document::{self, *},
    gltf_scene,
    render::*,
    *,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialEdit {
    pub material: Id,
    pub base_color: [f32; 3],
    pub emission: [f32; 3],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edits {
    pub materials: Vec<MaterialEdit>,
    pub light: PointLight,
    pub environment: [f32; 3],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Inspect,
    ImportGlb {
        bytes: Vec<u8>,
        policy: gltf_scene::Policy,
        settings: Settings,
        base_revision: String,
        idempotency_key: String,
    },
    ImportScene {
        json: Value,
        buffers: Vec<Vec<u8>>,
        policy: gltf_scene::Policy,
        settings: Settings,
        base_revision: String,
        idempotency_key: String,
    },
    ImportPbrGlb {
        bytes: Vec<u8>,
        policy: gltf_scene::PbrPolicy,
        settings: Settings,
        base_revision: String,
        idempotency_key: String,
    },
    ImportPbrScene {
        json: Value,
        buffers: Vec<Vec<u8>>,
        images: Vec<Vec<u8>>,
        policy: gltf_scene::PbrPolicy,
        settings: Settings,
        base_revision: String,
        idempotency_key: String,
    },
    SelectCamera {
        entity: Id,
        base_revision: String,
        idempotency_key: String,
    },
    Branch {
        branch: String,
        base_revision: String,
    },
    InspectBranch {
        branch: String,
    },
    Modify {
        branch: String,
        base_revision: String,
        idempotency_key: String,
        edits: Edits,
        max_added_bytes: u64,
    },
    RenderRootCpu {
        revision: String,
    },
    PreviewProducts {
        branch: String,
        revision: String,
    },
    PreviewAt {
        branch: String,
        request: crate::sequence::FrameRequest,
    },
    Preview {
        branch: String,
        revision: String,
    },
    PrepareCommit {
        branch: String,
        revision: String,
        idempotency_key: String,
        max_added_bytes: u64,
    },
    Commit {
        branch: String,
        revision: String,
        idempotency_key: String,
        max_added_bytes: u64,
    },
    Discard {
        branch: String,
    },
    Export,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub operation: Operation,
}
#[derive(Clone)]
struct Branch {
    owner: String,
    base: String,
    protected: String,
    document: Document,
    rendered: Option<RenderReceipt>,
}
#[derive(Clone)]
pub struct Session {
    document: Document,
    branches: BTreeMap<String, Branch>,
}
/// Includes geometry, hierarchy, instances, material assignments, layers, camera,
/// textures and BRDF profile. Only color/emission and lighting are removed.
pub fn protected_digest(snapshot: &Snapshot) -> Result<String> {
    let mut s = snapshot.clone();
    for m in s.materials.values_mut() {
        m.base_color = [0.; 3];
        m.emission = [0.; 3];
    }
    if let Some(settings) = &mut s.render_settings {
        settings.light = PointLight {
            position: [0.; 3],
            intensity: [0.; 3],
        };
        settings.environment = [0.; 3];
    }
    Ok(digest(&canonical(&s)?))
}
fn profile(snapshot: &Snapshot) -> Result<&Settings> {
    let settings = snapshot
        .render_settings
        .as_ref()
        .ok_or_else(|| Error::new("render_settings", "project has no authored render settings"))?;
    settings.validate()?;
    if settings.width > 128
        || settings.height > 128
        || settings.samples > 64
        || snapshot.entities.iter().count() > 192
        || canonical(snapshot)?.len() > 8 * 1024 * 1024
    {
        return Err(Error::new(
            "budget",
            "agent preview profile: 128x128, 64 samples, depth 1..16, 192 entities, 8 MiB snapshot",
        ));
    }
    Ok(settings)
}
fn write_permission(principal: &Principal) -> Result<()> {
    if !principal.can_write || principal.id.is_empty() {
        return Err(Error::new(
            "permission",
            "host-granted local write capability required",
        ));
    }
    Ok(())
}
impl Session {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            branches: BTreeMap::new(),
        }
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    /// Administrative root transactions use the same authorization and conflict
    /// checks. There is deliberately no administrative mutation of a branch.
    pub fn execute_root(
        &mut self,
        principal: &Principal,
        request: &document::Request,
    ) -> Result<Receipt> {
        self.document.execute(principal, request)
    }
    fn branch(&self, principal: &Principal, name: &str) -> Result<&Branch> {
        let branch = self
            .branches
            .get(name)
            .ok_or_else(|| Error::new("not_found", "variant not found"))?;
        if branch.owner != principal.id {
            return Err(Error::new(
                "permission",
                "variant belongs to another principal",
            ));
        }
        Ok(branch)
    }
    pub fn render_input(
        &self,
        principal: &Principal,
        name: &str,
        revision: &str,
    ) -> Result<(Snapshot, Settings)> {
        let b = self.branch(principal, name)?;
        if b.document.snapshot().revision()? != revision {
            return Err(Error::new("conflict", "stale variant revision"));
        }
        if protected_digest(b.document.snapshot())? != b.protected {
            return Err(Error::new("protected", "protected properties changed"));
        }
        Ok((
            b.document.snapshot().clone(),
            profile(b.document.snapshot())?.clone(),
        ))
    }
    /// Host-only render completion, never an operation accepting client receipts.
    pub fn record_render(
        &mut self,
        principal: &Principal,
        name: &str,
        image: &Image,
    ) -> Result<Value> {
        let (snapshot, settings) = self.render_input(principal, name, &image.receipt.revision)?;
        let pixels = (settings.width * settings.height) as usize;
        if image.receipt.settings_digest != digest(&canonical(&settings)?)
            || image.linear.len() != pixels
            || image.depth.len() != pixels
            || image.normals.len() != pixels
            || image.objects.len() != pixels
            || image
                .linear
                .iter()
                .flatten()
                .any(|n| !n.is_finite() || *n < 0.)
        {
            return Err(Error::new(
                "render_result",
                "renderer result does not match pinned input",
            ));
        }
        let visible = image.objects.iter().filter(|v| v.is_some()).count();
        let mean: [f64; 3] = std::array::from_fn(|c| {
            image.linear.iter().map(|v| f64::from(v[c])).sum::<f64>() / pixels as f64
        });
        self.branches
            .get_mut(name)
            .expect("validated branch")
            .rendered = Some(image.receipt.clone());
        Ok(
            json!({"receipt":image.receipt,"protected_digest":protected_digest(&snapshot)?,"inspection":{"visible_pixels":visible,"mean_linear_rgb":mean,"pixel_count":pixels},"passes":{"linear_rgb":image.linear,"depth_meters":image.depth,"normals_world":image.normals,"object_ids":image.objects},"charged_output_bytes":pixels * 64}),
        )
    }
    pub fn preview(
        &mut self,
        principal: &Principal,
        name: &str,
        revision: &str,
        cancel: impl FnMut() -> bool,
    ) -> Result<Value> {
        let (snapshot, settings) = self.render_input(principal, name, revision)?;
        let scene = Evaluator::default().evaluate(&snapshot)?;
        let image = render(&scene, &settings, cancel)?;
        self.record_render(principal, name, &image)
    }
    pub fn render_root_cpu(
        &self,
        revision: &str,
        mut cancel: impl FnMut() -> bool,
    ) -> Result<Value> {
        let snapshot = self.document.snapshot();
        if snapshot.revision()? != revision {
            return Err(Error::new("stale_revision", "root render revision changed"));
        }
        let settings = snapshot
            .render_settings
            .as_ref()
            .ok_or_else(|| Error::new("render_settings", "authored settings missing"))?;
        if settings.width > 256
            || settings.height > 256
            || settings.samples > 64
            || snapshot.entities.iter().count() > 192
            || canonical(snapshot)?.len() > 8 * 1024 * 1024
        {
            return Err(Error::new(
                "budget",
                "extended root CPU profile: 256 squared, 64 samples, 192 entities, 8 MiB snapshot",
            ));
        }
        let scene = Evaluator::default().evaluate_with_cancel(snapshot, &mut cancel)?;
        let image = render(&scene, settings, cancel)?;
        Ok(
            json!({"receipt":image.receipt,"conversions":scene.conversions,"displacements":scene.displacements,"inspection":{"visible_pixels":image.objects.iter().filter(|id|id.is_some()).count()},"passes":{"linear_rgb":image.linear,"depth_meters":image.depth,"normals_world":image.normals,"object_ids":image.objects}}),
        )
    }
    pub fn preview_at(
        &self,
        principal: &Principal,
        name: &str,
        request: &crate::sequence::FrameRequest,
        cancel: impl FnMut() -> bool,
    ) -> Result<Value> {
        let (snapshot, _) = self.render_input(principal, name, &request.revision)?;
        let frame = crate::sequence::render_frame(&snapshot, request, cancel)?;
        let image = &frame.image;
        let visible = image.objects.iter().filter(|id| id.is_some()).count();
        Ok(
            json!({"receipt":image.receipt,"evaluation":frame.evaluation,"temporal_times":frame.temporal_times,"inspection":{"visible_pixels":visible},"passes":{"linear_rgb":image.linear,"depth_meters":image.depth,"normals_world":image.normals,"object_ids":image.objects}}),
        )
    }
    /// A commit request is reusable with durable_execute; lighting and materials
    /// publish together. The root revision is rechecked by the document engine.
    pub fn selection_request(
        &self,
        principal: &Principal,
        name: &str,
        revision: &str,
        key: &str,
        max_added_bytes: u64,
    ) -> Result<document::Request> {
        write_permission(principal)?;
        let b = self.branch(principal, name)?;
        let (s, settings) = self.render_input(principal, name, revision)?;
        let settings_digest = digest(&canonical(&settings)?);
        if b.rendered
            .as_ref()
            .is_none_or(|r| r.revision != revision || r.settings_digest != settings_digest)
        {
            return Err(Error::new(
                "preview_required",
                "render and inspect this exact variant before selection",
            ));
        }
        let mut commands = vec![];
        for material in s.materials.values() {
            commands.push(Command::PutMaterial {
                material: material.clone(),
            });
        }
        commands.push(Command::SetRenderSettings { settings });
        Ok(document::Request {
            version: 0,
            base_revision: b.base.clone(),
            idempotency_key: key.into(),
            commands,
            max_added_bytes,
        })
    }
    fn import(
        &mut self,
        principal: &Principal,
        imported: gltf_scene::Imported,
        settings: Settings,
        base_revision: String,
        idempotency_key: String,
    ) -> Result<Value> {
        let mut commands = imported.commands;
        commands.push(Command::SetRenderSettings { settings });
        let request = document::Request {
            version: 0,
            base_revision,
            idempotency_key,
            commands,
            max_added_bytes: 8 * 1024 * 1024,
        };
        if let Some(receipt) = self.document.retry(principal, &request)? {
            return Ok(json!({"receipt":receipt,"report":imported.report}));
        }
        if self.document.snapshot().entities.iter().next().is_some()
            || !self.document.snapshot().materials.is_empty()
            || !self.document.snapshot().meshes.is_empty()
        {
            return Err(Error::new(
                "import_target",
                "scene import requires an empty project",
            ));
        }
        let candidate = self.document.prepare(principal, &request)?;
        profile(candidate.snapshot())?;
        Evaluator::default().evaluate(candidate.snapshot())?;
        let receipt = self.document.commit(candidate)?;
        Ok(json!({"receipt":receipt,"report":imported.report}))
    }
    pub fn dispatch(&mut self, principal: &Principal, request: Request) -> Result<Value> {
        if request.version != 0 {
            return Err(Error::new(
                "operation_version",
                "agent operations currently support version 0",
            ));
        }
        if canonical(&request)?.len() > 16 * 1024 * 1024 {
            return Err(Error::new("budget", "agent control limit 16 MiB"));
        }
        use Operation::*;
        match request.operation {
            Inspect => Ok(
                json!({"revision":self.document.snapshot().revision()?,"document_digest":digest(&canonical(&self.document)?),"snapshot":self.document.snapshot(),"protected_digest":protected_digest(self.document.snapshot())?,"branches":self.branches.iter().filter(|(_,b)| b.owner == principal.id).map(|(n,_)| n).collect::<Vec<_>>(),"profile":"agent-rendering-v0","durability":"memory; explicit host save/export required"}),
            ),
            ImportGlb {
                bytes,
                policy,
                settings,
                base_revision,
                idempotency_key,
            } => {
                write_permission(principal)?;
                let imported =
                    gltf_scene::import_glb(&bytes, self.document.snapshot().document_id, &policy)?;
                self.import(
                    principal,
                    imported,
                    settings,
                    base_revision,
                    idempotency_key,
                )
            }
            ImportScene {
                json,
                buffers,
                policy,
                settings,
                base_revision,
                idempotency_key,
            } => {
                write_permission(principal)?;
                let imported = gltf_scene::import_scene(
                    &serde_json::to_vec(&json)?,
                    &buffers,
                    self.document.snapshot().document_id,
                    &policy,
                )?;
                self.import(
                    principal,
                    imported,
                    settings,
                    base_revision,
                    idempotency_key,
                )
            }
            ImportPbrGlb {
                bytes,
                policy,
                settings,
                base_revision,
                idempotency_key,
            } => {
                write_permission(principal)?;
                let imported = gltf_scene::import_pbr_glb(
                    &bytes,
                    self.document.snapshot().document_id,
                    &policy,
                )?;
                self.import(
                    principal,
                    imported,
                    settings,
                    base_revision,
                    idempotency_key,
                )
            }
            ImportPbrScene {
                json,
                buffers,
                images,
                policy,
                settings,
                base_revision,
                idempotency_key,
            } => {
                write_permission(principal)?;
                let imported = gltf_scene::import_pbr_scene(
                    &serde_json::to_vec(&json)?,
                    &buffers,
                    &images,
                    self.document.snapshot().document_id,
                    &policy,
                )?;
                self.import(
                    principal,
                    imported,
                    settings,
                    base_revision,
                    idempotency_key,
                )
            }
            SelectCamera {
                entity,
                base_revision,
                idempotency_key,
            } => {
                write_permission(principal)?;
                let request = document::Request {
                    version: 0,
                    base_revision,
                    idempotency_key,
                    commands: vec![Command::UseCamera { entity }],
                    max_added_bytes: 8 * 1024 * 1024,
                };
                Ok(serde_json::to_value(
                    self.document.execute(principal, &request)?,
                )?)
            }
            Branch {
                branch,
                base_revision,
            } => {
                write_permission(principal)?;
                if branch.is_empty() || branch.len() > 64 {
                    return Err(Error::new(
                        "request",
                        "branch name must contain 1..64 bytes",
                    ));
                }
                if let Some(b) = self.branches.get(&branch) {
                    if b.owner != principal.id || b.base != base_revision {
                        return Err(Error::new("idempotency_mismatch", "branch ID already used"));
                    }
                    return Ok(
                        json!({"branch":branch,"base_revision":b.base,"revision":b.document.snapshot().revision()?}),
                    );
                }
                if self.branches.len() >= 3 {
                    return Err(Error::new("budget", "three live variants per session"));
                }
                if self.document.snapshot().revision()? != base_revision {
                    return Err(Error::new("conflict", "stale root revision"));
                }
                profile(self.document.snapshot())?;
                let protected = protected_digest(self.document.snapshot())?;
                self.branches.insert(
                    branch.clone(),
                    self::Branch {
                        owner: principal.id.clone(),
                        base: base_revision.clone(),
                        protected,
                        document: Document::new(self.document.snapshot().clone())?,
                        rendered: None,
                    },
                );
                Ok(json!({"branch":branch,"base_revision":base_revision,"revision":base_revision}))
            }
            InspectBranch { branch } => {
                let b = self.branch(principal, &branch)?;
                Ok(
                    json!({"branch":branch,"base_revision":b.base,"revision":b.document.snapshot().revision()?,"protected_digest":protected_digest(b.document.snapshot())?,"snapshot":b.document.snapshot(),"render":b.rendered}),
                )
            }
            Modify {
                branch,
                base_revision,
                idempotency_key,
                edits,
                max_added_bytes,
            } => {
                write_permission(principal)?;
                let b = self.branch(principal, &branch)?;
                if edits.materials.len() > 64 {
                    return Err(Error::new("budget", "material edit count"));
                }
                let mut commands = vec![];
                let mut seen = std::collections::BTreeSet::new();
                for edit in edits.materials {
                    if !seen.insert(edit.material) {
                        return Err(Error::new("request", "duplicate material edit"));
                    }
                    let mut material = b
                        .document
                        .snapshot()
                        .materials
                        .get(&edit.material)
                        .ok_or_else(|| {
                            Error::new("permission", "material outside variant's permitted set")
                        })?
                        .clone();
                    material.base_color = edit.base_color;
                    material.emission = edit.emission;
                    commands.push(Command::PutMaterial { material });
                }
                let mut settings = profile(b.document.snapshot())?.clone();
                settings.light = edits.light;
                settings.environment = edits.environment;
                commands.push(Command::SetRenderSettings { settings });
                let request = document::Request {
                    version: 0,
                    base_revision,
                    idempotency_key,
                    commands,
                    max_added_bytes,
                };
                if let Some(receipt) = b.document.retry(principal, &request)? {
                    return Ok(serde_json::to_value(receipt)?);
                }
                let candidate = b.document.prepare(principal, &request)?;
                if protected_digest(candidate.snapshot())? != b.protected {
                    return Err(Error::new(
                        "protected",
                        "variant edits alter protected properties",
                    ));
                }
                let b = self.branches.get_mut(&branch).expect("validated branch");
                let receipt = b.document.commit(candidate)?;
                b.rendered = None;
                Ok(serde_json::to_value(receipt)?)
            }
            RenderRootCpu { revision } => self.render_root_cpu(&revision, || false),
            PreviewProducts { branch, revision } => {
                let (snapshot, _) = self.render_input(principal, &branch, &revision)?;
                let products = crate::products::render_products(&snapshot, &revision, || false)?;
                Ok(
                    json!({"revision":revision,"views":products.views.iter().map(|v|json!({"name":v.name,"receipt":v.raw.receipt,"display":v.display,"albedo":v.albedo,"object_ids":v.raw.objects})).collect::<Vec<_>>(),"bakes":products.bakes}),
                )
            }
            PreviewAt { branch, request } => {
                self.preview_at(principal, &branch, &request, || false)
            }
            Preview { branch, revision } => self.preview(principal, &branch, &revision, || false),
            PrepareCommit {
                branch,
                revision,
                idempotency_key,
                max_added_bytes,
            } => Ok(serde_json::to_value(self.selection_request(
                principal,
                &branch,
                &revision,
                &idempotency_key,
                max_added_bytes,
            )?)?),
            Commit {
                branch,
                revision,
                idempotency_key,
                max_added_bytes,
            } => {
                let request = self.selection_request(
                    principal,
                    &branch,
                    &revision,
                    &idempotency_key,
                    max_added_bytes,
                )?;
                Ok(serde_json::to_value(
                    self.document.execute(principal, &request)?,
                )?)
            }
            Discard { branch } => {
                write_permission(principal)?;
                self.branch(principal, &branch)?;
                self.branches.remove(&branch);
                Ok(json!({"discarded":branch}))
            }
            Export => Ok(
                json!({"document":self.document,"revision":self.document.snapshot().revision()?,"ephemeral_variants_exported":false}),
            ),
        }
    }
    /// Publish root changes with the same journal transaction used by other
    /// clients. Failed persistence leaves both root and transient branches intact.
    pub fn dispatch_durable<S: crate::storage::JournalStore>(
        &mut self,
        store: &mut S,
        principal: &Principal,
        request: Request,
    ) -> Result<Value> {
        let root_mutation = matches!(
            &request.operation,
            Operation::ImportGlb { .. }
                | Operation::ImportScene { .. }
                | Operation::ImportPbrGlb { .. }
                | Operation::ImportPbrScene { .. }
                | Operation::SelectCamera { .. }
                | Operation::Commit { .. }
        );
        let records = if root_mutation {
            Some(store.load()?)
        } else {
            None
        };
        if let Some(records) = &records
            && let Some(saved) = crate::storage::recover(records)?
            && canonical(&saved)? != canonical(self.document())?
        {
            return Err(Error::new(
                "conflict",
                "storage state changed; reload before retry",
            ));
        }
        let empty_target = records.as_ref().is_some_and(Vec::is_empty);
        let mut next = self.clone();
        let mut result = next.dispatch(principal, request)?;
        if empty_target || canonical(self.document())? != canonical(next.document())? {
            let mut current = self.document.clone();
            crate::storage::durable_update(store, &mut current, |doc| {
                *doc = next.document.clone();
                Ok(())
            })?;
            next.document = current;
            if let Some(durable) = result.get_mut("durable") {
                *durable = Value::Bool(true);
            }
            if let Some(durable) = result.get_mut("receipt").and_then(|r| r.get_mut("durable")) {
                *durable = Value::Bool(true);
            }
        }
        *self = next;
        Ok(result)
    }
    pub fn dispatch_wire(&mut self, principal: &Principal, bytes: &[u8]) -> Result<Vec<u8>> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::new("budget", "agent control limit 16 MiB"));
        }
        canonical(&self.dispatch(principal, serde_json::from_slice(bytes)?)?)
    }
}

/// Stable names/versions and effects are reviewed independently of Session layout.
pub fn registry() -> Vec<crate::api::Operation> {
    [
        ("agent.render_root_cpu",false,"bounded CPU rendering for extended materials at a pinned root revision","evaluation and render boundaries"),
        ("agent.preview_products",false,"render authored views, color/denoising and typed UV bakes at a pinned revision","evaluation and render boundaries"),
        ("agent.preview_at",false,"render a pinned clip frame with exact-time shutter samples; keeps rest state immutable","evaluation and render boundaries"),
        (
            "agent.import_pbr_glb",
            true,
            "opaque static PBR GLB with bounded PNG/JPEG; explicit approximation consent",
            "atomic synchronous import",
        ),
        (
            "agent.import_pbr_scene",
            true,
            "opaque static PBR glTF with caller-supplied buffers and images; no URI resolution",
            "atomic synchronous import",
        ),
        (
            "agent.select_camera",
            true,
            "transactionally select an authored camera entity for root rendering",
            "before publication",
        ),
        (
            "agent.inspect",
            false,
            "read root revision, protected digest and authored snapshot",
            "immediate",
        ),
        (
            "agent.import_glb",
            true,
            "bounded GLB import into empty root; explicit Lambertian consent",
            "atomic synchronous import",
        ),
        (
            "agent.import_scene",
            true,
            "bounded glTF JSON plus explicit buffers; no URI resolution",
            "atomic synchronous import",
        ),
        (
            "agent.branch",
            false,
            "create an owner-scoped ephemeral variant at an exact root revision",
            "immediate",
        ),
        (
            "agent.inspect_branch",
            false,
            "read owner-scoped variant and render receipt",
            "immediate",
        ),
        (
            "agent.modify",
            true,
            "only existing material color/emission and point/environment lighting",
            "before publication",
        ),
        (
            "agent.preview",
            false,
            "bounded synchronous CPU render plus structured passes; root unchanged",
            "Rust closure; ABI synchronous",
        ),
        (
            "agent.prepare_commit",
            false,
            "return a retryable root transaction after rendered-variant verification",
            "immediate",
        ),
        (
            "agent.commit",
            true,
            "publish selected lighting/material transaction at exact root revision",
            "before publication",
        ),
        (
            "agent.discard",
            false,
            "release owner-scoped transient branch",
            "immediate",
        ),
        (
            "agent.export",
            false,
            "export committed document; transient branches omitted",
            "immediate",
        ),
    ]
    .into_iter()
    .map(
        |(name, mutation, effects, cancellation)| crate::api::Operation {
            name,
            version: 0,
            mutation,
            effects,
            cancellation,
        },
    )
    .collect()
}
