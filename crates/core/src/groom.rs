//! Guide-based strand generation with stable surface attachment. Generated tubes
//! use the named curve sweep profile; this is not a physical fiber BSDF.
use crate::{Error, Id, Result, canonical, curves::*, digest, document::Snapshot, geometry::*};
use glam::{DAffine3, DMat3, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Root {
    pub entity: Id,
    pub topology: String,
    pub corners: [u64; 3],
    pub barycentric: [f64; 3],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guide {
    pub curve: Curve,
    pub root: Root,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Child {
    pub id: Id,
    pub guide: Id,
    pub root: Root,
    pub length_scale: f64,
    pub radius_scale: f64,
    pub twist: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Groom {
    pub guides: Vec<Guide>,
    pub children: Vec<Child>,
    pub tessellation: Tessellation,
}
/// Topology identity excludes positions and attributes so deformation preserves
/// attachments. A topology edit requires explicit correspondence/rebinding.
pub fn topology(mesh: &Mesh) -> Result<String> {
    Ok(digest(&canonical(&(
        &mesh.edges,
        &mesh.face_offsets,
        &mesh.corners,
        &mesh.point_ids,
        &mesh.edge_ids,
        &mesh.face_ids,
        &mesh.corner_ids,
    ))?))
}
impl Root {
    fn frame(&self, s: &Snapshot) -> Result<DAffine3> {
        if self
            .barycentric
            .iter()
            .any(|x| !x.is_finite() || *x < 0. || *x > 1.)
            || (self.barycentric.iter().sum::<f64>() - 1.).abs() > 1e-10
        {
            return Err(Error::new("groom_root", "invalid barycentric root weights"));
        }
        let entity = s
            .entities
            .get(self.entity)
            .ok_or_else(|| Error::new("reference", "groom anchor entity missing"))?;
        let mesh = entity
            .mesh
            .as_ref()
            .and_then(|key| s.meshes.get(key))
            .ok_or_else(|| {
                Error::new(
                    "groom_root",
                    "root requires an explicit polygon mesh anchor",
                )
            })?;
        if topology(mesh)? != self.topology {
            return Err(Error::new(
                "stale_topology",
                "groom root topology changed; rebind with explicit correspondence",
            ));
        }
        let tri = mesh
            .triangles()?
            .into_iter()
            .find(|c| c.map(|i| mesh.corner_ids[i as usize]) == self.corners)
            .ok_or_else(|| {
                Error::new(
                    "groom_root",
                    "stable root corners no longer form the authored triangle",
                )
            })?;
        let p = tri.map(|i| mesh.positions.get(mesh.corners[i as usize].vertex as usize));
        let u = (p[1] - p[0]).normalize();
        let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
        let v = n.cross(u);
        let at =
            p[0] * self.barycentric[0] + p[1] * self.barycentric[1] + p[2] * self.barycentric[2];
        let frame = DAffine3::from_mat3_translation(DMat3::from_cols(u, v, n), at);
        if !frame.is_finite() {
            return Err(Error::new(
                "groom_root",
                "degenerate deformed root triangle",
            ));
        }
        Ok(frame)
    }
}
impl Groom {
    pub fn has_colors(&self) -> bool {
        self.guides.iter().any(|g| g.curve.has_colors())
    }
    pub fn validate(&self, s: &Snapshot) -> Result<()> {
        self.tessellation.validate()?;
        if self.guides.is_empty()
            || self.guides.len() > 256
            || self.guides.len() + self.children.len() > 2048
        {
            return Err(Error::new(
                "budget",
                "groom supports 1..256 guides and at most 2048 total strands",
            ));
        }
        let mut ids = BTreeSet::new();
        for g in &self.guides {
            if !ids.insert(g.curve.id) {
                return Err(Error::new("duplicate_id", "duplicate guide strand"));
            }
            g.curve.validate()?;
            if g.curve.closed || g.curve.controls[0].position != [0.; 3] {
                return Err(Error::new(
                    "groom",
                    "guide must be open and start at its root origin",
                ));
            }
            g.root.frame(s)?;
        }
        let guides = ids.clone();
        for c in &self.children {
            if !ids.insert(c.id) {
                return Err(Error::new("duplicate_id", "duplicate child strand"));
            }
            if !guides.contains(&c.guide) {
                return Err(Error::new("reference", "child guide missing"));
            }
            if !c.length_scale.is_finite()
                || !(0.01..=100.).contains(&c.length_scale)
                || !c.radius_scale.is_finite()
                || !(0.01..=100.).contains(&c.radius_scale)
                || !c.twist.is_finite()
                || c.twist.abs() > 1e6
            {
                return Err(Error::new("groom", "invalid child generation parameters"));
            }
            c.root.frame(s)?;
        }
        Ok(())
    }
    pub fn cache_key(&self, s: &Snapshot, entity: Id) -> Result<String> {
        let mut anchors = BTreeMap::new();
        for root in self
            .guides
            .iter()
            .map(|g| &g.root)
            .chain(self.children.iter().map(|c| &c.root))
        {
            let anchor = s
                .entities
                .get(root.entity)
                .ok_or_else(|| Error::new("reference", "groom anchor missing"))?;
            anchors.insert(
                root.entity,
                (
                    anchor.mesh.clone(),
                    s.world_transform(root.entity)?.to_cols_array(),
                ),
            );
        }
        Ok(digest(&canonical(&(
            self,
            anchors,
            s.world_transform(entity)?.to_cols_array(),
        ))?))
    }
    pub fn evaluate(
        &self,
        s: &Snapshot,
        entity: Id,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Evaluated> {
        if cancelled() {
            return Err(Error::new("cancelled", "groom evaluation cancelled"));
        }
        self.validate(s)?;
        let world = s.world_transform(entity)?;
        let inv = world.inverse();
        if !inv.is_finite() {
            return Err(Error::new("singular", "groom transform must be invertible"));
        }
        let mut strands: Vec<_> = self
            .guides
            .iter()
            .map(|g| (g.curve.clone(), &g.root))
            .collect();
        for child in &self.children {
            let mut curve = self
                .guides
                .iter()
                .find(|g| g.curve.id == child.guide)
                .expect("validated guide")
                .curve
                .clone();
            curve.id = child.id;
            let rotation = glam::DQuat::from_rotation_z(child.twist);
            for p in &mut curve.controls {
                p.position =
                    (rotation * DVec3::from_array(p.position) * child.length_scale).to_array();
                p.radius *= child.radius_scale;
            }
            strands.push((curve, &child.root));
        }
        let mut positions = vec![];
        let mut faces = vec![];
        let mut uv = vec![];
        let mut colors = self.has_colors().then(Vec::new);
        let mut ranges = BTreeMap::new();
        let mut samples = 0usize;
        for (curve, root) in strands {
            if cancelled() {
                return Err(Error::new("cancelled", "groom child evaluation cancelled"));
            }
            let id = curve.id;
            let derived = Asset {
                shape: Shape::Curves {
                    curves: vec![curve],
                },
                tessellation: self.tessellation.clone(),
            }
            .evaluate(&mut cancelled)?;
            samples = samples
                .checked_add(derived.receipt.samples)
                .ok_or_else(|| Error::new("budget", "groom sample overflow"))?;
            if positions.len() + derived.mesh.positions.len()
                > self.tessellation.max_vertices as usize
                || samples > self.tessellation.max_samples as usize
            {
                return Err(Error::new(
                    "budget",
                    "aggregate groom tessellation budget exceeded",
                ));
            }
            let transform = inv * s.world_transform(root.entity)? * root.frame(s)?;
            let start = positions.len() as u32;
            if let Some(colors) = &mut colors {
                if let Some(attr) = derived.mesh.color_attribute() {
                    let AttributeValues::Vec4(values) = &attr.values else {
                        unreachable!("curve point colors")
                    };
                    colors.extend(values);
                } else {
                    colors.resize(colors.len() + derived.mesh.positions.len(), [1.; 4]);
                }
            }
            for i in 0..derived.mesh.positions.len() {
                positions.push(
                    transform
                        .transform_point3(derived.mesh.positions.get(i))
                        .to_array(),
                );
            }
            for tri in derived.mesh.triangles()? {
                faces.push(
                    tri.map(|i| start + derived.mesh.corners[i as usize].vertex)
                        .to_vec(),
                );
            }
            // Each generated source face is triangular, retaining corner UV seams.
            let AttributeValues::Vec2(values) = &derived.mesh.attributes["curve_uv"].values else {
                unreachable!()
            };
            uv.extend(values);
            ranges.insert(id, [start, positions.len() as u32]);
        }
        let mut mesh = Mesh::from_polygons(Positions::F64(positions), &faces, &[])?;
        mesh.attributes.insert(
            "curve_uv".into(),
            Attribute {
                id: Id(1),
                domain: Domain::Corner,
                semantic: "uv".into(),
                transfer: Transfer::Linear,
                values: AttributeValues::Vec2(uv),
            },
        );
        if let Some(colors) = colors {
            mesh.attributes.insert(
                "curve_color".into(),
                Attribute {
                    id: Id(200),
                    domain: Domain::Point,
                    semantic: "color_rgba".into(),
                    transfer: Transfer::Linear,
                    values: AttributeValues::Vec4(colors),
                },
            );
        }
        mesh.validate()?;
        let mut receipt=Conversion{source_digest:self.cache_key(s,entity)?,policy_digest:digest(&canonical(&self.tessellation)?),curve_ranges:ranges,point_ranges:BTreeMap::new(),samples,vertices:mesh.positions.len(),triangles:faces.len(),derived_bytes:canonical(&mesh)?.len(),approximation:"guide copies generated at stable barycentric roots; local tangent frames follow anchor deformation; bounded polygon sweeps; no physical fiber BSDF".into()};
        if self.has_colors() {
            receipt.approximation.push_str("; polyline guide RGBA preserved through child copies and point-domain sweep transfer");
        }
        Ok(Evaluated { mesh, receipt })
    }
}
