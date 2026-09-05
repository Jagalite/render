use crate::{Error, Id, Result, canonical, digest, geometry::Mesh};
use glam::{DAffine3, DMat3, DVec3};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const CHUNK: usize = 64;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entity {
    pub id: Id,
    pub name: String,
    pub parent: Option<Id>,
    pub mesh: Option<String>,
    pub material: Option<Id>,
    pub transform: Transform,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transform {
    /// Column-major affine matrix, meters, f64; shear and negative scale are preserved.
    pub columns: [[f64; 3]; 4],
    #[serde(default)]
    pub operations: Vec<TransformOp>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum TransformOp {
    TranslationMeters([f64; 3]),
    Scale([f64; 3]),
    RotationRadians {
        angles: [f64; 3],
        order: RotationOrder,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RotationOrder {
    Xyz,
    Xzy,
    Yxz,
    Yzx,
    Zxy,
    Zyx,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            columns: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [0., 0., 0.]],
            operations: vec![],
        }
    }
}
impl Transform {
    pub fn translation(x: f64, y: f64, z: f64) -> Self {
        let mut t = Self::default();
        t.operations.push(TransformOp::TranslationMeters([x, y, z]));
        t
    }
    pub fn affine(&self) -> Result<DAffine3> {
        if self.columns.iter().flatten().any(|n| !n.is_finite()) {
            return Err(Error::new("transform", "nonfinite affine transform"));
        }
        let mut affine = DAffine3::from_mat3_translation(
            DMat3::from_cols_array_2d(&[self.columns[0], self.columns[1], self.columns[2]]),
            DVec3::from_array(self.columns[3]),
        );
        for op in &self.operations {
            let next = match op {
                TransformOp::TranslationMeters(v) => {
                    DAffine3::from_translation(DVec3::from_array(*v))
                }
                TransformOp::Scale(v) => DAffine3::from_scale(DVec3::from_array(*v)),
                TransformOp::RotationRadians { angles, order } => {
                    let order = match order {
                        RotationOrder::Xyz => glam::EulerRot::XYZ,
                        RotationOrder::Xzy => glam::EulerRot::XZY,
                        RotationOrder::Yxz => glam::EulerRot::YXZ,
                        RotationOrder::Yzx => glam::EulerRot::YZX,
                        RotationOrder::Zxy => glam::EulerRot::ZXY,
                        RotationOrder::Zyx => glam::EulerRot::ZYX,
                    };
                    DAffine3::from_quat(glam::DQuat::from_euler(
                        order, angles[0], angles[1], angles[2],
                    ))
                }
            };
            if !next.is_finite() {
                return Err(Error::new(
                    "transform",
                    "nonfinite authored transform channel",
                ));
            }
            affine *= next;
        }
        if !affine.is_finite() {
            return Err(Error::new("transform", "transform channel overflow"));
        }
        Ok(affine)
    }
    pub fn inverse(&self) -> Result<DAffine3> {
        let a = self.affine()?;
        if a.matrix3.determinant() == 0. {
            return Err(Error::new(
                "singular",
                "operation requires an invertible transform",
            ));
        }
        let inv = a.inverse();
        if !inv.is_finite() {
            return Err(Error::new("singular", "inverse exceeds numerical range"));
        }
        Ok(inv)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub linear_rgb: Vec<[f32; 3]>,
}
impl Texture {
    pub fn validate(&self) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || u64::from(self.width) * u64::from(self.height) != self.linear_rgb.len() as u64
            || self
                .linear_rgb
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err(Error::new(
                "texture",
                "invalid scene-linear texture dimensions or samples",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Material {
    pub id: Id,
    pub base_color: [f32; 3],
    pub emission: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    pub texture: Option<Texture>,
}
impl Material {
    pub fn diffuse(id: Id, color: [f32; 3]) -> Self {
        Self {
            id,
            base_color: color,
            emission: [0.; 3],
            roughness: 1.,
            metallic: 0.,
            texture: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self
            .base_color
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.).contains(v))
            || self.emission.iter().any(|v| !v.is_finite() || *v < 0.)
            || ![self.roughness, self.metallic]
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.).contains(v))
        {
            return Err(Error::new(
                "material",
                "material parameters violate finite range",
            ));
        }
        if let Some(t) = &self.texture {
            t.validate()?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "Vec<Entity>", into = "Vec<Entity>")]
pub struct EntityTable {
    chunks: Vec<Arc<Vec<Entity>>>,
    index: BTreeMap<Id, (usize, usize)>,
}
impl From<EntityTable> for Vec<Entity> {
    fn from(t: EntityTable) -> Self {
        let mut entities = t.iter().cloned().collect::<Vec<_>>();
        entities.sort_by_key(|e| e.id);
        entities
    }
}
impl TryFrom<Vec<Entity>> for EntityTable {
    type Error = Error;
    fn try_from(entities: Vec<Entity>) -> Result<Self> {
        let mut table = Self::default();
        for e in entities {
            table.insert(e)?;
        }
        Ok(table)
    }
}
impl EntityTable {
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.chunks.iter().flat_map(|c| c.iter())
    }
    pub fn get(&self, id: Id) -> Option<&Entity> {
        let &(c, i) = self.index.get(&id)?;
        self.chunks.get(c)?.get(i)
    }
    pub fn get_mut(&mut self, id: Id) -> Option<&mut Entity> {
        let &(c, i) = self.index.get(&id)?;
        Some(&mut Arc::make_mut(&mut self.chunks[c])[i])
    }
    pub fn insert(&mut self, e: Entity) -> Result<()> {
        if self.get(e.id).is_some() {
            return Err(Error::new("duplicate_id", "entity already exists"));
        }
        if self.chunks.last().is_none_or(|c| c.len() == CHUNK) {
            self.chunks.push(Arc::new(Vec::new()));
        }
        let c = self.chunks.len() - 1;
        let i = self.chunks[c].len();
        self.index.insert(e.id, (c, i));
        Arc::make_mut(&mut self.chunks[c]).push(e);
        Ok(())
    }
    pub fn shared_chunks(&self, other: &Self) -> usize {
        self.chunks
            .iter()
            .zip(&other.chunks)
            .filter(|(a, b)| Arc::ptr_eq(a, b))
            .count()
    }
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub name: String,
    pub overrides: BTreeMap<Id, Override>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Override {
    pub transform: Option<Transform>,
    pub material: Option<Id>,
    pub hidden: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub version: u32,
    pub document_id: Id,
    pub entities: EntityTable,
    pub meshes: BTreeMap<String, Arc<Mesh>>,
    pub materials: BTreeMap<Id, Material>,
    pub layers: Vec<Layer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_settings: Option<crate::render::Settings>,
}
impl Snapshot {
    pub fn empty(document_id: Id) -> Self {
        Self {
            version: 0,
            document_id,
            entities: EntityTable::default(),
            meshes: BTreeMap::new(),
            materials: BTreeMap::new(),
            layers: vec![],
            render_settings: None,
        }
    }
    pub fn revision(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
    pub fn validate(&self) -> Result<()> {
        if self.version > 1 || (self.version == 0) != self.render_settings.is_none() {
            return Err(Error::new("schema_version", "unsupported document version"));
        }
        if let Some(settings) = &self.render_settings {
            settings.validate()?;
        }
        for (key, m) in &self.meshes {
            if &m.content_id()? != key {
                return Err(Error::new("integrity", "mesh digest mismatch"));
            }
        }
        for (id, m) in &self.materials {
            m.validate()?;
            if id != &m.id {
                return Err(Error::new("identity", "material table identity mismatch"));
            }
        }
        let mut ids = BTreeSet::new();
        for e in self.entities.iter() {
            if !ids.insert(e.id) {
                return Err(Error::new("duplicate_id", "duplicate entity"));
            }
            e.transform.affine()?;
            if e.mesh
                .as_ref()
                .is_some_and(|id| !self.meshes.contains_key(id))
                || e.material
                    .is_some_and(|id| !self.materials.contains_key(&id))
            {
                return Err(Error::new("reference", "missing geometry or material"));
            }
            self.world_transform(e.id)?;
        }
        let mut names = BTreeSet::new();
        for layer in &self.layers {
            if !names.insert(&layer.name) {
                return Err(Error::new("layer", "duplicate layer name"));
            }
            for (id, o) in &layer.overrides {
                if self.entities.get(*id).is_none()
                    || o.material.is_some_and(|m| !self.materials.contains_key(&m))
                {
                    return Err(Error::new("reference", "invalid layer target"));
                }
                if let Some(t) = &o.transform {
                    t.affine()?;
                }
            }
        }
        Ok(())
    }
    pub fn world_transform(&self, id: Id) -> Result<DAffine3> {
        let mut next = Some(id);
        let mut seen = BTreeSet::new();
        let mut out = DAffine3::IDENTITY;
        while let Some(id) = next {
            if !seen.insert(id) || seen.len() > 256 {
                return Err(Error::new("cycle", "parent cycle or depth >256"));
            }
            let e = self
                .entities
                .get(id)
                .ok_or_else(|| Error::new("reference", "parent or entity missing"))?;
            out = e.transform.affine()? * out;
            next = e.parent;
        }
        if !out.is_finite() {
            return Err(Error::new("transform", "composed transform overflow"));
        }
        Ok(out)
    }
    pub fn composed(&self) -> Result<Self> {
        let mut out = self.clone();
        let mut visibility = BTreeMap::new();
        for layer in &self.layers {
            for (id, o) in &layer.overrides {
                let e = out
                    .entities
                    .get_mut(*id)
                    .ok_or_else(|| Error::new("reference", "layer target"))?;
                if let Some(t) = &o.transform {
                    e.transform = t.clone();
                }
                if let Some(m) = o.material {
                    e.material = Some(m);
                }
                visibility.insert(*id, o.hidden);
            }
        }
        for (id, hidden) in visibility {
            if hidden {
                out.entities
                    .get_mut(id)
                    .expect("validated layer target")
                    .mesh = None;
            }
        }
        out.layers.clear();
        out.validate()?;
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    SetRenderSettings {
        settings: crate::render::Settings,
    },
    PutMesh {
        mesh: Mesh,
    },
    PutMaterial {
        material: Material,
    },
    CreateEntity {
        entity: Entity,
    },
    Rename {
        entity: Id,
        name: String,
    },
    Reparent {
        entity: Id,
        parent: Option<Id>,
    },
    SetTransform {
        entity: Id,
        transform: Transform,
    },
    SetMaterial {
        entity: Id,
        material: Id,
    },
    SetMaterialScalar {
        material: Id,
        parameter: ScalarParameter,
        value: f32,
    },
    PutLayer {
        layer: Layer,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScalarParameter {
    Roughness,
    Metallic,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub base_revision: String,
    pub idempotency_key: String,
    pub commands: Vec<Command>,
    pub max_added_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub document_id: Id,
    pub revision: String,
    pub base_revision: String,
    pub delta_digest: String,
    pub command_count: usize,
    pub durable: bool,
}
#[derive(Debug, Clone)]
pub struct Candidate {
    snapshot: Snapshot,
    receipt: Receipt,
    scope: String,
    payload: String,
    commands: Vec<Command>,
}
impl Candidate {
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Accepted {
    pub payload: String,
    pub receipt: Receipt,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub receipt: Receipt,
    pub commands: Vec<Command>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    snapshot: Snapshot,
    accepted: BTreeMap<String, Accepted>,
    journal: Vec<JournalEntry>,
    #[serde(default)]
    jobs: crate::jobs::Jobs,
}
#[derive(Clone, Debug)]
pub struct Principal {
    pub id: String,
    pub can_write: bool,
}
impl Document {
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }
    pub fn jobs(&self) -> &crate::jobs::Jobs {
        &self.jobs
    }
    pub fn jobs_mut(&mut self) -> &mut crate::jobs::Jobs {
        &mut self.jobs
    }
    pub fn new(snapshot: Snapshot) -> Result<Self> {
        snapshot.validate()?;
        Ok(Self {
            snapshot,
            accepted: BTreeMap::new(),
            journal: vec![],
            jobs: crate::jobs::Jobs::default(),
        })
    }
    fn scope(&self, p: &Principal, r: &Request) -> Result<String> {
        if !p.can_write || p.id.is_empty() {
            return Err(Error::new(
                "permission",
                "authenticated write capability required",
            ));
        }
        if r.version != 0
            || r.idempotency_key.len() < 16
            || r.idempotency_key.len() > 128
            || r.commands.is_empty()
            || r.commands.len() > 256
        {
            return Err(Error::new(
                "request",
                "unsupported version or invalid request bounds",
            ));
        }
        Ok(String::from_utf8(canonical(&(
            p.id.as_str(),
            self.snapshot.document_id,
            r.idempotency_key.as_str(),
        ))?)
        .expect("JSON UTF8"))
    }
    pub fn retry(&self, p: &Principal, r: &Request) -> Result<Option<Receipt>> {
        let scope = self.scope(p, r)?;
        if let Some(a) = self.accepted.get(&scope) {
            if a.payload != digest(&canonical(r)?) {
                return Err(Error::new(
                    "idempotency_mismatch",
                    "key already used with a different request",
                ));
            }
            return Ok(Some(a.receipt.clone()));
        }
        Ok(None)
    }
    pub fn prepare(&self, p: &Principal, r: &Request) -> Result<Candidate> {
        let scope = self.scope(p, r)?;
        if self.retry(p, r)?.is_some() {
            return Err(Error::new("already_committed", "use the retained receipt"));
        }
        if self.accepted.len() >= 10000 {
            return Err(Error::new(
                "admission",
                "document retains at most 10000 idempotency receipts; export a checkpoint to a new document before continuing",
            ));
        }
        let base = self.snapshot.revision()?;
        if base != r.base_revision {
            return Err(
                Error::new("stale_revision", "request base is no longer current")
                    .with_context("requested_revision", r.base_revision.clone())
                    .with_context("current_revision", base),
            );
        }
        let mut snapshot = self.snapshot.clone();
        for command in &r.commands {
            apply(&mut snapshot, command)?;
        }
        snapshot.validate()?;
        let before = canonical(&self.snapshot)?.len();
        let after = canonical(&snapshot)?.len();
        if after > 128 * 1024 * 1024 {
            return Err(Error::new(
                "budget",
                "document exceeds the 128 MiB canonical-state profile",
            ));
        }
        if after.saturating_sub(before) as u64 > r.max_added_bytes {
            return Err(Error::new("budget", "candidate exceeds added-byte budget"));
        }
        let receipt = Receipt {
            document_id: snapshot.document_id,
            revision: snapshot.revision()?,
            base_revision: base,
            delta_digest: digest(&canonical(&r.commands)?),
            command_count: r.commands.len(),
            durable: false,
        };
        Ok(Candidate {
            snapshot,
            receipt,
            scope,
            payload: digest(&canonical(r)?),
            commands: r.commands.clone(),
        })
    }
    pub fn commit(&mut self, candidate: Candidate) -> Result<Receipt> {
        if let Some(a) = self.accepted.get(&candidate.scope) {
            if a.payload == candidate.payload {
                return Ok(a.receipt.clone());
            }
            return Err(Error::new(
                "idempotency_mismatch",
                "concurrent key payload conflict",
            ));
        }
        if self.accepted.len() >= 10000 {
            return Err(Error::new(
                "admission",
                "document idempotency retention limit reached before commit",
            ));
        }
        if self.snapshot.revision()? != candidate.receipt.base_revision {
            return Err(Error::new(
                "conflict",
                "candidate base changed before commit",
            ));
        }
        if candidate.snapshot.revision()? != candidate.receipt.revision {
            return Err(Error::new(
                "integrity",
                "candidate changed after validation",
            ));
        }
        self.snapshot = candidate.snapshot;
        self.journal.push(JournalEntry {
            receipt: candidate.receipt.clone(),
            commands: candidate.commands,
        });
        self.accepted.insert(
            candidate.scope,
            Accepted {
                payload: candidate.payload,
                receipt: candidate.receipt.clone(),
            },
        );
        Ok(candidate.receipt)
    }
    pub fn execute(&mut self, p: &Principal, r: &Request) -> Result<Receipt> {
        if let Some(receipt) = self.retry(p, r)? {
            return Ok(receipt);
        }
        let c = self.prepare(p, r)?;
        self.commit(c)
    }
    pub fn execute_wire(&mut self, p: &Principal, json: &[u8]) -> Result<Vec<u8>> {
        if json.len() > 16 * 1024 * 1024 {
            return Err(Error::new("budget", "control message too large"));
        }
        let r: Request = serde_json::from_slice(json)?;
        canonical(&self.execute(p, &r)?)
    }
    pub fn mark_durable(&mut self) {
        for a in self.accepted.values_mut() {
            a.receipt.durable = true;
        }
        for r in &mut self.journal {
            r.receipt.durable = true;
        }
    }
}
fn apply(s: &mut Snapshot, c: &Command) -> Result<()> {
    fn entity(s: &mut Snapshot, id: Id) -> Result<&mut Entity> {
        s.entities
            .get_mut(id)
            .ok_or_else(|| Error::new("not_found", "entity does not exist"))
    }
    match c {
        Command::SetRenderSettings { settings } => {
            settings.validate()?;
            s.render_settings = Some(settings.clone());
            s.version = 1;
        }
        Command::PutMesh { mesh } => {
            let id = mesh.content_id()?;
            s.meshes.insert(id, Arc::new(mesh.clone()));
        }
        Command::PutMaterial { material } => {
            material.validate()?;
            s.materials.insert(material.id, material.clone());
        }
        Command::CreateEntity { entity: e } => s.entities.insert(e.clone())?,
        Command::Rename { entity: id, name } => entity(s, *id)?.name = name.clone(),
        Command::Reparent { entity: id, parent } => entity(s, *id)?.parent = *parent,
        Command::SetTransform {
            entity: id,
            transform,
        } => entity(s, *id)?.transform = transform.clone(),
        Command::SetMaterial {
            entity: id,
            material,
        } => entity(s, *id)?.material = Some(*material),
        Command::SetMaterialScalar {
            material,
            parameter,
            value,
        } => {
            let m = s
                .materials
                .get_mut(material)
                .ok_or_else(|| Error::new("not_found", "material does not exist"))?;
            match parameter {
                ScalarParameter::Roughness => m.roughness = *value,
                ScalarParameter::Metallic => m.metallic = *value,
            }
        }
        Command::PutLayer { layer } => {
            if let Some(existing) = s.layers.iter_mut().find(|l| l.name == layer.name) {
                *existing = layer.clone();
            } else {
                s.layers.push(layer.clone());
            }
        }
    }
    Ok(())
}
