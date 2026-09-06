use crate::{Error, Id, Result, canonical, digest};
use glam::{DVec3, Vec2, Vec4};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "precision", content = "values")]
pub enum Positions {
    F32(Vec<[f32; 3]>),
    F64(Vec<[f64; 3]>),
}
impl Positions {
    pub fn len(&self) -> usize {
        match self {
            Self::F32(p) => p.len(),
            Self::F64(p) => p.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, i: usize) -> DVec3 {
        match self {
            Self::F32(p) => DVec3::from_array(p[i].map(f64::from)),
            Self::F64(p) => DVec3::from_array(p[i]),
        }
    }
    pub fn set(&mut self, i: usize, p: DVec3) -> Result<()> {
        if i >= self.len() || !p.is_finite() {
            return Err(Error::new("position", "invalid vertex index or position"));
        }
        match self {
            Self::F32(v) => {
                let f = p.as_vec3();
                if !f.is_finite() {
                    return Err(Error::new("precision", "position exceeds f32"));
                }
                v[i] = f.to_array();
            }
            Self::F64(v) => v[i] = p.to_array(),
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Domain {
    Point,
    Edge,
    Face,
    Corner,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "values")]
pub enum AttributeValues {
    Scalar(Vec<f32>),
    Vec2(Vec<[f32; 2]>),
    Vec3(Vec<[f32; 3]>),
    Vec4(Vec<[f32; 4]>),
    Category(Vec<u32>),
}
impl AttributeValues {
    pub fn len(&self) -> usize {
        match self {
            Self::Scalar(v) => v.len(),
            Self::Vec2(v) => v.len(),
            Self::Vec3(v) => v.len(),
            Self::Vec4(v) => v.len(),
            Self::Category(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn finite(&self) -> bool {
        match self {
            Self::Scalar(v) => v.iter().all(|v| v.is_finite()),
            Self::Vec2(v) => v.iter().flatten().all(|v| v.is_finite()),
            Self::Vec3(v) => v.iter().flatten().all(|v| v.is_finite()),
            Self::Vec4(v) => v.iter().flatten().all(|v| v.is_finite()),
            Self::Category(_) => true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Transfer {
    Linear,
    Nearest,
    Normalize,
    Categorical,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attribute {
    pub id: Id,
    pub domain: Domain,
    pub semantic: String,
    pub transfer: Transfer,
    pub values: AttributeValues,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Corner {
    pub vertex: u32,
    pub edge: u32,
}

/// One ring per polygon. Explicit edge records preserve loose and nonmanifold data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mesh {
    pub positions: Positions,
    pub edges: Vec<[u32; 2]>,
    pub face_offsets: Vec<u32>,
    pub corners: Vec<Corner>,
    pub point_ids: Vec<u64>,
    pub edge_ids: Vec<u64>,
    pub face_ids: Vec<u64>,
    pub corner_ids: Vec<u64>,
    pub attributes: BTreeMap<String, Attribute>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_uv_attribute: Option<Id>,
}
impl Mesh {
    pub fn from_polygons(
        positions: Positions,
        faces: &[Vec<u32>],
        loose: &[[u32; 2]],
    ) -> Result<Self> {
        let mut mesh = Self {
            positions,
            edges: Vec::new(),
            face_offsets: vec![0],
            corners: Vec::new(),
            point_ids: vec![],
            edge_ids: vec![],
            face_ids: vec![],
            corner_ids: vec![],
            attributes: BTreeMap::new(),
            default_uv_attribute: None,
        };
        let mut lookup = BTreeMap::new();
        for face in faces {
            if face.len() < 3 {
                return Err(Error::new("topology", "face has fewer than three corners"));
            }
            for i in 0..face.len() {
                let (a, b) = (face[i], face[(i + 1) % face.len()]);
                let key = (a.min(b), a.max(b));
                let edge = *lookup.entry(key).or_insert_with(|| {
                    mesh.edges.push([a, b]);
                    (mesh.edges.len() - 1) as u32
                });
                mesh.corners.push(Corner { vertex: a, edge });
            }
            mesh.face_offsets.push(mesh.corners.len() as u32);
        }
        for &[a, b] in loose {
            if let std::collections::btree_map::Entry::Vacant(e) =
                lookup.entry((a.min(b), a.max(b)))
            {
                e.insert(mesh.edges.len() as u32);
                mesh.edges.push([a, b]);
            }
        }
        mesh.point_ids = (1..=mesh.positions.len() as u64).collect();
        mesh.edge_ids = (1..=mesh.edges.len() as u64).collect();
        mesh.face_ids = (1..=faces.len() as u64).collect();
        mesh.corner_ids = (1..=mesh.corners.len() as u64).collect();
        mesh.validate()?;
        Ok(mesh)
    }
    pub fn faces(&self) -> usize {
        self.face_offsets.len().saturating_sub(1)
    }
    pub fn validate(&self) -> Result<()> {
        let bad = |m| Error::new("topology", m);
        if self.positions.len() > u32::MAX as usize
            || self.corners.len() > u32::MAX as usize
            || self.edges.len() > u32::MAX as usize
        {
            return Err(bad("geometry exceeds index profile"));
        }
        if (0..self.positions.len()).any(|i| !self.positions.get(i).is_finite()) {
            return Err(bad("nonfinite position"));
        }
        if self.face_offsets.first() != Some(&0)
            || self.face_offsets.last().copied() != Some(self.corners.len() as u32)
        {
            return Err(bad("invalid face offsets"));
        }
        for edge in &self.edges {
            if edge[0] == edge[1] || edge.iter().any(|&v| v as usize >= self.positions.len()) {
                return Err(bad("invalid edge"));
            }
        }
        for offsets in self.face_offsets.windows(2) {
            let (start, end) = (offsets[0] as usize, offsets[1] as usize);
            if end < start || end > self.corners.len() || end - start < 3 {
                return Err(bad("invalid polygon range"));
            }
            let mut seen = BTreeSet::new();
            for i in start..end {
                let c = self.corners[i];
                let next = self.corners[if i + 1 == end { start } else { i + 1 }].vertex;
                if !seen.insert(c.vertex) || c.vertex as usize >= self.positions.len() {
                    return Err(bad("invalid or repeated polygon vertex"));
                }
                let edge = self
                    .edges
                    .get(c.edge as usize)
                    .ok_or_else(|| bad("invalid corner edge"))?;
                if !((edge[0] == c.vertex && edge[1] == next)
                    || (edge[1] == c.vertex && edge[0] == next))
                {
                    return Err(bad("corner edge does not join adjacent corners"));
                }
            }
        }
        for (ids, count) in [
            (&self.point_ids, self.positions.len()),
            (&self.edge_ids, self.edges.len()),
            (&self.face_ids, self.faces()),
            (&self.corner_ids, self.corners.len()),
        ] {
            if ids.len() != count
                || ids.contains(&0)
                || ids.iter().collect::<BTreeSet<_>>().len() != count
            {
                return Err(bad("element identity mismatch"));
            }
        }
        if self
            .attributes
            .values()
            .filter(|a| a.semantic == "color_rgba")
            .count()
            > 1
        {
            return Err(Error::new(
                "attribute",
                "one RGBA color attribute per mesh is supported",
            ));
        }
        let mut attr_ids = BTreeSet::new();
        for attr in self.attributes.values() {
            let count = match attr.domain {
                Domain::Point => self.positions.len(),
                Domain::Edge => self.edges.len(),
                Domain::Face => self.faces(),
                Domain::Corner => self.corners.len(),
            };
            if !attr_ids.insert(attr.id) || attr.values.len() != count || !attr.values.finite() {
                return Err(Error::new(
                    "attribute",
                    "invalid identity, length or finite-value contract",
                ));
            }
            if attr.semantic == "color_rgba" {
                if !matches!(attr.domain, Domain::Point | Domain::Corner) {
                    return Err(Error::new(
                        "attribute",
                        "RGBA colors require point or corner domain",
                    ));
                }
                let AttributeValues::Vec4(values) = &attr.values else {
                    return Err(Error::new("attribute", "RGBA colors require vec4 values"));
                };
                if values.iter().flatten().any(|v| !(0.0..=1.0).contains(v)) {
                    return Err(Error::new(
                        "attribute",
                        "RGBA color components must lie in [0,1]",
                    ));
                }
            }
            if attr.semantic == "uv"
                && (attr.domain != Domain::Corner
                    || !matches!(attr.values, AttributeValues::Vec2(_)))
            {
                return Err(Error::new(
                    "attribute",
                    "UV data must be corner-domain vec2",
                ));
            }
        }
        if let Some(id) = self.default_uv_attribute {
            self.uv_values(id)?;
        }
        Ok(())
    }
    pub fn color_attribute(&self) -> Option<&Attribute> {
        self.attributes
            .values()
            .find(|a| a.semantic == "color_rgba")
    }
    pub fn color_rgba(&self, corner: usize) -> Result<Vec4> {
        let c = self
            .corners
            .get(corner)
            .ok_or_else(|| Error::new("attribute", "color corner index out of range"))?;
        let Some(a) = self.color_attribute() else {
            return Ok(Vec4::ONE);
        };
        let AttributeValues::Vec4(values) = &a.values else {
            return Err(Error::new("attribute", "RGBA colors require vec4"));
        };
        let index = match a.domain {
            Domain::Point => c.vertex as usize,
            Domain::Corner => corner,
            _ => return Err(Error::new("attribute", "color domain")),
        };
        values
            .get(index)
            .map(|v| Vec4::from_array(*v))
            .ok_or_else(|| Error::new("attribute", "color value missing"))
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
    pub fn default_uv_id(&self) -> Option<Id> {
        self.default_uv_attribute.or_else(|| {
            self.attributes
                .values()
                .find(|a| a.semantic == "uv")
                .map(|a| a.id)
        })
    }
    pub fn uv(&self, corner: usize) -> Vec2 {
        self.default_uv_id()
            .and_then(|id| self.uv_values(id).ok())
            .map(|values| Vec2::from_array(values[corner]))
            .unwrap_or(Vec2::ZERO)
    }
    pub fn uv_values(&self, id: Id) -> Result<&[[f32; 2]]> {
        self.attributes
            .values()
            .find_map(|a| match (&a.values, a.domain, a.semantic.as_str()) {
                (AttributeValues::Vec2(values), Domain::Corner, "uv") if a.id == id => {
                    Some(values.as_slice())
                }
                _ => None,
            })
            .ok_or_else(|| Error::new("reference", "selected corner UV attribute is missing"))
    }
    /// Ear clipping of planar simple polygons. Degenerate/nonplanar polygons fail explicitly.
    pub fn triangles(&self) -> Result<Vec<[u32; 3]>> {
        self.validate()?;
        let mut out = Vec::new();
        for pair in self.face_offsets.windows(2) {
            let mut ring: Vec<u32> = (pair[0]..pair[1]).collect();
            let p = |c: u32| self.positions.get(self.corners[c as usize].vertex as usize);
            let origin = p(ring[0]);
            let mut n = DVec3::ZERO;
            for i in 1..ring.len() - 1 {
                n += (p(ring[i]) - origin).cross(p(ring[i + 1]) - origin);
            }
            let scale = ring
                .iter()
                .map(|&c| (p(c) - origin).length())
                .fold(0f64, f64::max);
            if scale == 0. || n.length() <= scale * scale * 1e-12 {
                return Err(Error::new("degenerate", "polygon has zero area"));
            }
            n = n.normalize();
            if ring
                .iter()
                .any(|&c| (p(c) - origin).dot(n).abs() > scale * 1e-8)
            {
                return Err(Error::new(
                    "nonplanar",
                    "triangulation requires a planar polygon",
                ));
            }
            while ring.len() > 3 {
                let mut ear = None;
                for i in 0..ring.len() {
                    let (a, b, c) = (
                        ring[(i + ring.len() - 1) % ring.len()],
                        ring[i],
                        ring[(i + 1) % ring.len()],
                    );
                    let (pa, pb, pc) = (p(a), p(b), p(c));
                    if (pb - pa).cross(pc - pb).dot(n) <= scale * scale * 1e-12 {
                        continue;
                    }
                    let inside = ring.iter().any(|&q| {
                        q != a
                            && q != b
                            && q != c
                            && [(pa, pb), (pb, pc), (pc, pa)].iter().all(|&(u, v)| {
                                (v - u).cross(p(q) - u).dot(n) >= -scale * scale * 1e-12
                            })
                    });
                    if !inside {
                        ear = Some((i, [a, b, c]));
                        break;
                    }
                }
                let (i, t) = ear.ok_or_else(|| {
                    Error::new(
                        "triangulation",
                        "self-intersection or unsupported degenerate polygon",
                    )
                })?;
                out.push(t);
                ring.remove(i);
            }
            out.push([ring[0], ring[1], ring[2]]);
        }
        Ok(out)
    }
}

/// Canonical decimal local IDs in JSON maps. Explicit parsing also works through
/// serde's tagged-enum buffer, which does not coerce string keys into integers.
pub mod local_map {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    pub fn serialize<T: Serialize, S: serde::Serializer>(
        map: &BTreeMap<u64, T>,
        s: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        map.iter()
            .map(|(id, value)| (id.to_string(), value))
            .collect::<BTreeMap<_, _>>()
            .serialize(s)
    }
    pub fn deserialize<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
        d: D,
    ) -> std::result::Result<BTreeMap<u64, T>, D::Error> {
        let input = BTreeMap::<String, T>::deserialize(d)?;
        input
            .into_iter()
            .map(|(id, value)| {
                let n = id.parse::<u64>().map_err(serde::de::Error::custom)?;
                if n.to_string() != id {
                    return Err(serde::de::Error::custom(
                        "local ID must be canonical decimal",
                    ));
                }
                Ok((n, value))
            })
            .collect()
    }
}
