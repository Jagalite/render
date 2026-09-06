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
    Quaternion([f64; 4]),
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
                TransformOp::Quaternion(q) => {
                    let q = glam::DQuat::from_array(*q);
                    if !q.is_finite() || (q.length_squared() - 1.).abs() > 1e-8 {
                        return Err(Error::new(
                            "quaternion",
                            "authored quaternion must be unit length",
                        ));
                    }
                    DAffine3::from_quat(q)
                }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pbr: Option<crate::pbr::Surface>,
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
            pbr: None,
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
        if let Some(surface) = &self.pbr {
            surface.validate()?;
            if surface
                .advanced
                .as_ref()
                .is_some_and(|a| matches!(a.model, crate::scattering::Model::Dielectric { .. }))
                && (self.roughness != 0. || self.metallic != 0.)
            {
                return Err(Error::new(
                    "material",
                    "ideal dielectric requires roughness=0 and metallic=0",
                ));
            }
            if self.texture.is_some() {
                return Err(Error::new(
                    "material",
                    "legacy texture cannot be combined with PBR bindings",
                ));
            }
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub images: BTreeMap<String, Arc<crate::textures::ImageAsset>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cameras: BTreeMap<Id, crate::cameras::Lens>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub geometry_assets: BTreeMap<String, Arc<crate::curves::Asset>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub geometry_bindings: BTreeMap<Id, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub volume_assets: BTreeMap<String, Arc<crate::volumes::Asset>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub volume_bindings: BTreeMap<Id, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub grooms: BTreeMap<Id, crate::groom::Groom>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<crate::animation::State>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imaging: Option<crate::products::State>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub procedural_assets: BTreeMap<String, Arc<crate::procedural::Graph>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub procedural_bindings: BTreeMap<Id, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub modeling_receipts: BTreeMap<String, crate::modeling::Receipt>,
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
            images: BTreeMap::new(),
            cameras: BTreeMap::new(),
            geometry_assets: BTreeMap::new(),
            geometry_bindings: BTreeMap::new(),
            volume_assets: BTreeMap::new(),
            volume_bindings: BTreeMap::new(),
            grooms: BTreeMap::new(),
            animation: None,
            imaging: None,
            procedural_assets: BTreeMap::new(),
            procedural_bindings: BTreeMap::new(),
            modeling_receipts: BTreeMap::new(),
            render_settings: None,
        }
    }
    pub fn revision(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
    pub fn validate(&self) -> Result<()> {
        if self.version > 14
            || (self.version < 14
                && self.meshes.values().any(|m| {
                    m.attributes
                        .values()
                        .any(|a| matches!(a.values, crate::geometry::AttributeValues::Vec4(_)))
                }))
            || (self.version < 13 && self.animation.as_ref().is_some_and(|a| a.needs_v13()))
            || (self.version < 12
                && self.materials.values().any(|m| {
                    m.pbr.as_ref().is_some_and(|p| {
                        p.bindings()
                            .into_iter()
                            .flatten()
                            .any(|b| b.uv_attribute.is_some())
                    })
                }))
            || (self.version < 11 && self.animation.as_ref().is_some_and(|a| a.needs_v11()))
            || (self.version < 10
                && (!self.procedural_assets.is_empty()
                    || !self.procedural_bindings.is_empty()
                    || !self.modeling_receipts.is_empty()))
            || (self.version < 9
                && self
                    .materials
                    .values()
                    .any(|m| m.pbr.as_ref().is_some_and(|p| p.displacement.is_some())))
            || (self.version < 8
                && self
                    .materials
                    .values()
                    .any(|m| m.pbr.as_ref().is_some_and(|p| p.advanced.is_some())))
            || (self.version < 7 && self.imaging.is_some())
            || (self.version < 6
                && (self.animation.is_some()
                    || self.entities.iter().any(|e| {
                        e.transform
                            .operations
                            .iter()
                            .any(|op| matches!(op, TransformOp::Quaternion(_)))
                    })
                    || self
                        .layers
                        .iter()
                        .flat_map(|l| l.overrides.values())
                        .any(|o| {
                            o.transform.as_ref().is_some_and(|t| {
                                t.operations
                                    .iter()
                                    .any(|op| matches!(op, TransformOp::Quaternion(_)))
                            })
                        })))
            || (self.version < 5 && !self.grooms.is_empty())
            || (self.version < 4
                && (!self.volume_assets.is_empty() || !self.volume_bindings.is_empty()))
            || (self.version < 3
                && (!self.geometry_assets.is_empty() || !self.geometry_bindings.is_empty()))
            || (self.version < 2 && (self.version == 0) != self.render_settings.is_none())
            || (self.version < 2
                && (!self.images.is_empty()
                    || !self.cameras.is_empty()
                    || self.materials.values().any(|m| m.pbr.is_some())
                    || self
                        .render_settings
                        .as_ref()
                        .is_some_and(|s| s.camera.lens.is_some())))
        {
            return Err(Error::new("schema_version", "unsupported document version"));
        }
        if let Some(settings) = &self.render_settings {
            settings.validate()?;
        }
        if self.geometry_assets.len() > 256 {
            return Err(Error::new("budget", "at most 256 typed geometry assets"));
        }
        for (key, asset) in &self.geometry_assets {
            if asset.content_id()? != *key {
                return Err(Error::new(
                    "integrity",
                    "typed geometry content digest mismatch",
                ));
            }
        }
        for (entity, key) in &self.geometry_bindings {
            let e = self
                .entities
                .get(*entity)
                .ok_or_else(|| Error::new("reference", "geometry attachment entity missing"))?;
            if e.mesh.is_some() || !self.geometry_assets.contains_key(key) {
                return Err(Error::new(
                    "reference",
                    "typed geometry attachment is missing or conflicts with a mesh",
                ));
            }
        }
        if self.volume_assets.len() > 256 {
            return Err(Error::new("budget", "at most 256 sparse volume assets"));
        }
        for (key, asset) in &self.volume_assets {
            if asset.content_id()? != *key {
                return Err(Error::new("integrity", "volume digest mismatch"));
            }
        }
        for (id, key) in &self.volume_bindings {
            if self.entities.get(*id).is_none() || !self.volume_assets.contains_key(key) {
                return Err(Error::new(
                    "reference",
                    "missing volume attachment entity or asset",
                ));
            }
        }
        if self.grooms.len() > 64 {
            return Err(Error::new("budget", "at most 64 groom components"));
        }
        for (id, groom) in &self.grooms {
            let e = self
                .entities
                .get(*id)
                .ok_or_else(|| Error::new("reference", "groom entity missing"))?;
            if e.mesh.is_some() || self.geometry_bindings.contains_key(id) {
                return Err(Error::new(
                    "groom",
                    "groom geometry component conflicts with other surface geometry",
                ));
            }
            groom.validate(self)?;
        }
        if let Some(animation) = &self.animation {
            animation.validate(self)?;
        }
        if let Some(imaging) = &self.imaging {
            imaging.validate(self)?;
        }
        if self.procedural_assets.len() > 128 || self.modeling_receipts.len() > 256 {
            return Err(Error::new(
                "budget",
                "procedural asset/provenance table limit exceeded",
            ));
        }
        for (key, graph) in &self.procedural_assets {
            graph.validate(&self.meshes)?;
            if graph.content_id()? != *key {
                return Err(Error::new("integrity", "procedural graph digest mismatch"));
            }
        }
        for (id, key) in &self.procedural_bindings {
            let e = self
                .entities
                .get(*id)
                .ok_or_else(|| Error::new("reference", "procedural entity missing"))?;
            if !self.procedural_assets.contains_key(key)
                || e.mesh.is_some()
                || self.geometry_bindings.contains_key(id)
                || self.grooms.contains_key(id)
            {
                return Err(Error::new(
                    "reference",
                    "missing or conflicting procedural geometry component",
                ));
            }
        }
        for (key, receipt) in &self.modeling_receipts {
            if digest(&canonical(receipt)?) != *key {
                return Err(Error::new(
                    "integrity",
                    "modeling provenance digest mismatch",
                ));
            }
        }
        let mut pixels = 0u64;
        for (key, image) in &self.images {
            if image.content_id()? != *key {
                return Err(Error::new("integrity", "image content digest mismatch"));
            }
            pixels += u64::from(image.width) * u64::from(image.height);
        }
        if self.images.len() > 16 || pixels > 12 * 1024 * 1024 {
            return Err(Error::new(
                "budget",
                "image table exceeds 16 assets or 12 million pixels",
            ));
        }
        for (entity, lens) in &self.cameras {
            lens.validate()?;
            self.camera(*entity)?;
        }
        for (key, m) in &self.meshes {
            if &m.content_id()? != key {
                return Err(Error::new("integrity", "mesh digest mismatch"));
            }
        }
        for (id, m) in &self.materials {
            m.validate()?;
            if let Some(surface) = &m.pbr {
                for b in surface.bindings().into_iter().flatten() {
                    if !self.images.contains_key(&b.image) {
                        return Err(Error::new("reference", "PBR texture image missing"));
                    }
                }
            }
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
            if !self.procedural_bindings.contains_key(&e.id)
                && !self.grooms.contains_key(&e.id)
                && let (Some(mesh), Some(material)) = (e.mesh.as_ref(), e.material)
                && let Some(surface) = &self.materials[&material].pbr
            {
                surface.validate_uv_bindings(&self.meshes[mesh])?;
            }
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
                if !self.procedural_bindings.contains_key(id)
                    && !self.grooms.contains_key(id)
                    && let (Some(mesh), Some(material)) = (
                        self.entities.get(*id).and_then(|e| e.mesh.as_ref()),
                        o.material,
                    )
                    && let Some(surface) = &self.materials[&material].pbr
                {
                    surface.validate_uv_bindings(&self.meshes[mesh])?;
                }
            }
        }
        Ok(())
    }
    pub fn camera(&self, entity: Id) -> Result<crate::render::Camera> {
        let lens = self
            .cameras
            .get(&entity)
            .ok_or_else(|| Error::new("reference", "camera component missing"))?
            .clone();
        lens.validate()?;
        let world = self.world_transform(entity)?;
        let x = world.matrix3.x_axis;
        let y = world.matrix3.y_axis;
        let z = world.matrix3.z_axis;
        if [
            x.length_squared() - 1.,
            y.length_squared() - 1.,
            z.length_squared() - 1.,
            x.dot(y),
            x.dot(z),
            y.dot(z),
        ]
        .iter()
        .any(|v| v.abs() > 1e-6)
            || world.matrix3.determinant() < 0.
        {
            return Err(Error::new(
                "unsupported_camera",
                "camera profile requires a rigid orientation; scaled/sheared/reflected camera transforms are rejected",
            ));
        }
        let position = world.translation;
        let camera = crate::render::Camera {
            position: position.to_array(),
            target: (position - z).to_array(),
            up: y.to_array(),
            vertical_fov_radians: match lens {
                crate::cameras::Lens::Perspective {
                    vertical_fov_radians,
                    ..
                } => vertical_fov_radians,
                _ => 1.,
            },
            lens: Some(lens),
        };
        camera.basis()?;
        Ok(camera)
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
    pub fn composition(&self) -> Result<(Self, BTreeSet<Id>)> {
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
        out.layers.clear();
        out.validate()?;
        Ok((
            out,
            visibility
                .into_iter()
                .filter_map(|(id, hidden)| hidden.then_some(id))
                .collect(),
        ))
    }
    /// Legacy flattened surface view. Dependency-aware consumers use composition
    /// so hidden anchors remain available to grooms and later deformation.
    pub fn composed(&self) -> Result<Self> {
        let (mut out, hidden) = self.composition()?;
        for id in hidden {
            out.geometry_bindings.remove(&id);
            out.volume_bindings.remove(&id);
            out.grooms.remove(&id);
            out.procedural_bindings.remove(&id);
            out.entities
                .get_mut(id)
                .expect("validated layer target")
                .mesh = None;
        }
        out.validate()?;
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    CreateBox {
        entity: Id,
        min: [f64; 3],
        max: [f64; 3],
    },
    ModelMesh {
        entity: Id,
        source_mesh: String,
        #[serde(rename = "operator")]
        operation: crate::modeling::Operation,
        budget: crate::modeling::Budget,
    },
    SetProcedural {
        entity: Id,
        graph: Option<crate::procedural::Graph>,
    },
    BakeProcedural {
        entity: Id,
        source_graph: String,
    },
    SetImaging {
        imaging: Option<crate::products::State>,
    },
    MergeAnimation {
        animation: crate::animation::State,
    },
    SetAnimation {
        animation: Option<crate::animation::State>,
    },
    SetGroom {
        entity: Id,
        groom: Option<crate::groom::Groom>,
    },
    PutVolume {
        asset: crate::volumes::Asset,
    },
    SetVolume {
        entity: Id,
        asset: Option<String>,
    },
    PutGeometry {
        asset: crate::curves::Asset,
    },
    SetGeometry {
        entity: Id,
        asset: Option<String>,
    },
    UseCamera {
        entity: Id,
    },
    PutImage {
        image: crate::textures::ImageAsset,
    },
    SetCamera {
        entity: Id,
        lens: crate::cameras::Lens,
    },
    SetRenderSettings {
        settings: crate::render::Settings,
    },
    PutMesh {
        mesh: Mesh,
    },
    PutMaterial {
        material: Box<Material>,
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
    if matches!(c,Command::CreateEntity{entity} if entity.transform.operations.iter().any(|op|matches!(op,TransformOp::Quaternion(_))))
        || matches!(c,Command::SetTransform{transform,..} if transform.operations.iter().any(|op|matches!(op,TransformOp::Quaternion(_))))
        || matches!(c,Command::PutLayer{layer} if layer.overrides.values().any(|o|o.transform.as_ref().is_some_and(|t|t.operations.iter().any(|op|matches!(op,TransformOp::Quaternion(_))))))
    {
        s.version = s.version.max(6);
    }
    if matches!(c,Command::PutMaterial{material} if material.pbr.as_ref().is_some_and(|p|p.advanced.is_some()))
    {
        s.version = s.version.max(8);
    }
    if matches!(c,Command::PutMaterial{material} if material.pbr.as_ref().is_some_and(|p|p.displacement.is_some()))
    {
        s.version = s.version.max(9);
    }
    if matches!(c,Command::PutMaterial{material} if material.pbr.as_ref().is_some_and(|p|p.bindings().into_iter().flatten().any(|b|b.uv_attribute.is_some())))
    {
        s.version = s.version.max(12);
    }
    match c {
        Command::CreateBox {
            entity: id,
            min,
            max,
        } => {
            let e = entity(s, *id)?;
            if e.mesh.is_some() {
                return Err(Error::new(
                    "geometry",
                    "primitive target already has a mesh",
                ));
            }
            let mesh = crate::modeling::box_mesh(*min, *max)?;
            let key = mesh.content_id()?;
            s.meshes.insert(key.clone(), Arc::new(mesh));
            entity(s, *id)?.mesh = Some(key);
            s.version = s.version.max(10);
        }
        Command::ModelMesh {
            entity: id,
            source_mesh,
            operation,
            budget,
        } => {
            if entity(s, *id)?.mesh.as_ref() != Some(source_mesh) {
                return Err(Error::new(
                    "stale_selection",
                    "modeling source mesh changed",
                ));
            }
            let (mesh, receipt) = crate::modeling::apply(
                s.meshes
                    .get(source_mesh)
                    .ok_or_else(|| Error::new("reference", "modeling source asset missing"))?,
                operation,
                budget,
                || false,
            )?;
            let key = mesh.content_id()?;
            s.meshes.insert(key.clone(), Arc::new(mesh));
            entity(s, *id)?.mesh = Some(key);
            s.modeling_receipts
                .insert(digest(&canonical(&receipt)?), receipt);
            s.version = s.version.max(10);
        }
        Command::SetProcedural { entity: id, graph } => {
            entity(s, *id)?;
            if let Some(graph) = graph {
                graph.validate(&s.meshes)?;
                let key = graph.content_id()?;
                s.procedural_assets
                    .insert(key.clone(), Arc::new(graph.clone()));
                s.procedural_bindings.insert(*id, key);
            } else {
                s.procedural_bindings.remove(id);
            }
            s.version = s.version.max(10);
        }
        Command::BakeProcedural {
            entity: id,
            source_graph,
        } => {
            if s.procedural_bindings.get(id) != Some(source_graph) {
                return Err(Error::new(
                    "stale_selection",
                    "procedural source graph changed",
                ));
            }
            let result = s.procedural_assets[source_graph].evaluate(&s.meshes, || false)?;
            let key = result.mesh.content_id()?;
            s.meshes.insert(key.clone(), Arc::new(result.mesh));
            entity(s, *id)?.mesh = Some(key);
            s.procedural_bindings.remove(id);
            s.version = s.version.max(10);
        }
        Command::SetImaging { imaging } => {
            s.imaging = imaging.clone();
            s.version = s.version.max(7);
        }
        Command::SetAnimation { animation } => {
            s.animation = animation.clone();
            s.version = s
                .version
                .max(if animation.as_ref().is_some_and(|a| a.needs_v13()) {
                    13
                } else if animation.as_ref().is_some_and(|a| a.needs_v11()) {
                    11
                } else {
                    6
                });
        }
        Command::MergeAnimation { animation } => {
            s.animation
                .get_or_insert_with(Default::default)
                .merge(animation)?;
            s.version = s.version.max(if animation.needs_v13() {
                13
            } else if animation.needs_v11() {
                11
            } else {
                6
            });
        }
        Command::SetGroom { entity: id, groom } => {
            entity(s, *id)?;
            if let Some(g) = groom {
                s.grooms.insert(*id, g.clone());
            } else {
                s.grooms.remove(id);
            }
            s.version = s.version.max(5);
        }
        Command::PutVolume { asset } => {
            s.volume_assets
                .insert(asset.content_id()?, Arc::new(asset.clone()));
            s.version = s.version.max(4);
        }
        Command::SetVolume { entity: id, asset } => {
            entity(s, *id)?;
            if let Some(key) = asset {
                s.volume_bindings.insert(*id, key.clone());
            } else {
                s.volume_bindings.remove(id);
            }
            s.version = s.version.max(4);
        }
        Command::PutGeometry { asset } => {
            s.geometry_assets
                .insert(asset.content_id()?, Arc::new(asset.clone()));
            s.version = s.version.max(3);
        }
        Command::SetGeometry { entity: id, asset } => {
            entity(s, *id)?;
            if let Some(key) = asset {
                s.geometry_bindings.insert(*id, key.clone());
            } else {
                s.geometry_bindings.remove(id);
            }
            s.version = s.version.max(3);
        }
        Command::UseCamera { entity } => {
            let camera = s.camera(*entity)?;
            let settings = s.render_settings.as_mut().ok_or_else(|| {
                Error::new(
                    "render_settings",
                    "selecting a camera requires authored render settings",
                )
            })?;
            settings.camera = camera;
            settings.validate()?;
            s.version = s.version.max(2);
        }
        Command::PutImage { image } => {
            image.decode()?;
            s.images
                .insert(image.content_id()?, Arc::new(image.clone()));
            s.version = s.version.max(2);
        }
        Command::SetCamera { entity, lens } => {
            lens.validate()?;
            s.cameras.insert(*entity, lens.clone());
            s.version = s.version.max(2);
        }
        Command::SetRenderSettings { settings } => {
            settings.validate()?;
            s.render_settings = Some(settings.clone());
            s.version = s
                .version
                .max(if settings.camera.lens.is_some() { 2 } else { 1 });
        }
        Command::PutMesh { mesh } => {
            let id = mesh.content_id()?;
            if mesh
                .attributes
                .values()
                .any(|a| matches!(a.values, crate::geometry::AttributeValues::Vec4(_)))
            {
                s.version = s.version.max(14);
            }
            s.meshes.insert(id, Arc::new(mesh.clone()));
        }
        Command::PutMaterial { material } => {
            material.validate()?;
            if material.pbr.is_some() {
                s.version = s.version.max(2);
            }
            s.materials.insert(material.id, material.as_ref().clone());
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
