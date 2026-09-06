//! Bounded polygon authoring profiles, shared by direct commands and node graphs.
use crate::{Error, Result, canonical, digest, geometry::*, topology::EditMesh};
use glam::{DMat3, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
type Weights = Vec<(usize, f64)>;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub vertices: u32,
    pub faces: u32,
    pub bytes: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            vertices: 65536,
            faces: 65536,
            bytes: 64 * 1024 * 1024,
        }
    }
}
impl Budget {
    pub fn validate(&self) -> Result<()> {
        self.check(0, 0)
    }
    fn check(&self, points: usize, faces: usize) -> Result<()> {
        if self.vertices == 0
            || self.vertices > 524288
            || self.faces == 0
            || self.faces > 524288
            || self.bytes == 0
            || self.bytes > 512 * 1024 * 1024
            || points > self.vertices as usize
            || faces > self.faces as usize
        {
            return Err(Error::new(
                "budget",
                "modeling geometry growth budget exceeded",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Boolean {
    Union,
    Intersection,
    Difference,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Triangulate,
    ExtrudeFace {
        face: u64,
        distance_meters: f64,
    },
    InsetFace {
        face: u64,
        fraction: f64,
    },
    Bridge {
        first: Vec<u64>,
        second: Vec<u64>,
    },
    SplitEdge {
        edge: u64,
        fraction: f64,
    },
    Weld {
        points: Vec<u64>,
    },
    DissolveEdge {
        edge: u64,
    },
    Mirror {
        axis: u8,
        offset_meters: f64,
    },
    Array {
        count: u32,
        step: [f64; 3],
    },
    Subdivide,
    Solidify {
        thickness_meters: f64,
    },
    BevelBox {
        distance_meters: f64,
    },
    BooleanBox {
        operation: Boolean,
        min: [f64; 3],
        max: [f64; 3],
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainMap {
    #[serde(with = "crate::geometry::local_map")]
    pub sources: BTreeMap<u64, Vec<u64>>,
    pub created: Vec<u64>,
    pub deleted: Vec<u64>,
    pub merged: Vec<u64>,
    pub split: Vec<u64>,
    pub preserved: Vec<u64>,
    pub ambiguous: Vec<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Receipt {
    pub source: String,
    pub output: String,
    pub operation_digest: String,
    pub points: DomainMap,
    pub edges: DomainMap,
    pub faces: DomainMap,
    pub corners: DomainMap,
    pub output_bytes: usize,
    pub policy: String,
}
#[derive(Clone, Debug)]
struct Point {
    position: DVec3,
    source: Weights,
}
#[derive(Clone, Debug)]
struct Face {
    vertices: Vec<usize>,
    source: Weights,
    corners: Vec<Weights>,
    uv: Option<Vec<[f32; 2]>>,
}
#[derive(Clone, Debug)]
struct Draft {
    points: Vec<Point>,
    faces: Vec<Face>,
}
fn blend(parts: impl IntoIterator<Item = (Weights, f64)>) -> Weights {
    let mut out = BTreeMap::new();
    for (weights, scale) in parts {
        for (i, w) in weights {
            *out.entry(i).or_insert(0.) += w * scale;
        }
    }
    out.into_iter().filter(|(_, w)| w.abs() > 1e-15).collect()
}
impl Draft {
    fn from_mesh(m: &Mesh) -> Self {
        Self {
            points: (0..m.positions.len())
                .map(|i| Point {
                    position: m.positions.get(i),
                    source: vec![(i, 1.)],
                })
                .collect(),
            faces: m
                .face_offsets
                .windows(2)
                .enumerate()
                .map(|(f, w)| Face {
                    vertices: m.corners[w[0] as usize..w[1] as usize]
                        .iter()
                        .map(|c| c.vertex as usize)
                        .collect(),
                    source: vec![(f, 1.)],
                    corners: (w[0]..w[1]).map(|i| vec![(i as usize, 1.)]).collect(),
                    uv: None,
                })
                .collect(),
        }
    }
    fn normal(&self, f: usize) -> Result<DVec3> {
        let face = &self.faces[f];
        let p = self.points[face.vertices[0]].position;
        let mut n = DVec3::ZERO;
        for pair in face.vertices[1..].windows(2) {
            n += (self.points[pair[0]].position - p).cross(self.points[pair[1]].position - p);
        }
        if n.length_squared() < 1e-24 {
            return Err(Error::new("degenerate", "face normal undefined"));
        }
        Ok(n.normalize())
    }
    fn compact(&mut self) {
        let used: BTreeSet<_> = self
            .faces
            .iter()
            .flat_map(|f| f.vertices.iter().copied())
            .collect();
        let map: BTreeMap<_, _> = used
            .iter()
            .enumerate()
            .map(|(new, old)| (*old, new))
            .collect();
        self.points = used.iter().map(|i| self.points[*i].clone()).collect();
        for f in &mut self.faces {
            for i in &mut f.vertices {
                *i = map[i];
            }
        }
    }
}
fn assign(old: &[u64], weights: &[Weights]) -> Result<(Vec<u64>, DomainMap)> {
    let mut used = BTreeSet::new();
    let mut next = old
        .iter()
        .max()
        .copied()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| Error::new("identity", "local ID space exhausted"))?;
    let mut ids = vec![];
    let mut sources: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut created = vec![];
    let mut merged = vec![];
    let mut ambiguous = vec![];
    for w in weights {
        let existing = if w.len() == 1 && (w[0].1 - 1.).abs() < 1e-8 {
            Some(old[w[0].0])
        } else {
            None
        };
        let id = if let Some(id) = existing.filter(|id| !used.contains(id)) {
            id
        } else {
            let id = next;
            next = next
                .checked_add(1)
                .ok_or_else(|| Error::new("identity", "local ID space exhausted"))?;
            created.push(id);
            id
        };
        used.insert(id);
        ids.push(id);
        for (i, _) in w {
            sources.entry(old[*i]).or_default().push(id);
        }
        if w.len() > 1 {
            merged.push(id);
        }
        if w.is_empty() {
            ambiguous.push(id);
        }
    }
    let deleted = old
        .iter()
        .filter(|id| !sources.contains_key(id))
        .copied()
        .collect();
    let split = sources
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(id, _)| *id)
        .collect();
    let preserved = sources
        .iter()
        .filter(|(id, v)| v.as_slice() == [**id])
        .map(|(id, _)| *id)
        .collect();
    Ok((
        ids,
        DomainMap {
            sources,
            created,
            deleted,
            merged,
            split,
            preserved,
            ambiguous,
        },
    ))
}
fn transfer(
    values: &AttributeValues,
    policy: &Transfer,
    weights: &[Weights],
) -> Result<AttributeValues> {
    fn weighted<const N: usize>(
        input: impl Fn(usize) -> [f32; N],
        policy: &Transfer,
        weights: &[Weights],
    ) -> Result<Vec<[f32; N]>> {
        let mut out = vec![];
        for w in weights {
            if w.is_empty() {
                return Err(Error::new(
                    "attribute_transfer",
                    "new elements need explicit defaults for this attribute domain",
                ));
            }
            let mut value = [0.; N];
            if matches!(policy, Transfer::Nearest | Transfer::Categorical) {
                let index = w
                    .iter()
                    .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
                    .expect("weights")
                    .0;
                value = input(index);
                if *policy == Transfer::Categorical && w.iter().any(|(i, _)| input(*i) != value) {
                    return Err(Error::new(
                        "attribute_transfer",
                        "categorical source values disagree",
                    ));
                }
            } else {
                for (i, weight) in w {
                    let v = input(*i);
                    for c in 0..N {
                        value[c] += (f64::from(v[c]) * weight) as f32;
                    }
                }
                if *policy == Transfer::Normalize {
                    let length = value.iter().map(|x| x * x).sum::<f32>().sqrt();
                    if length <= 1e-12 {
                        return Err(Error::new(
                            "attribute_transfer",
                            "cannot normalize zero interpolated attribute",
                        ));
                    }
                    for x in &mut value {
                        *x /= length;
                    }
                }
            }
            out.push(value);
        }
        Ok(out)
    }
    Ok(match values {
        AttributeValues::Scalar(v) => AttributeValues::Scalar(
            weighted(|i| [v[i]], policy, weights)?
                .into_iter()
                .map(|v| v[0])
                .collect(),
        ),
        AttributeValues::Vec2(v) => AttributeValues::Vec2(weighted(|i| v[i], policy, weights)?),
        AttributeValues::Vec3(v) => AttributeValues::Vec3(weighted(|i| v[i], policy, weights)?),
        AttributeValues::Vec4(v) => AttributeValues::Vec4(weighted(|i| v[i], policy, weights)?),
        AttributeValues::Category(v) => {
            let mut out = vec![];
            for w in weights {
                let first = w
                    .first()
                    .ok_or_else(|| {
                        Error::new(
                            "attribute_transfer",
                            "new categorical element has no default",
                        )
                    })?
                    .0;
                let value = v[first];
                if w.iter().any(|(i, _)| v[*i] != value) {
                    return Err(Error::new(
                        "attribute_transfer",
                        "categorical source values disagree",
                    ));
                }
                out.push(value);
            }
            AttributeValues::Category(out)
        }
    })
}
fn finish(
    source: &Mesh,
    mut d: Draft,
    operation: &Operation,
    budget: &Budget,
) -> Result<(Mesh, Receipt)> {
    // Warped new quads are realized with the explicit 0-2 diagonal profile.
    // Original input polygons were validated as planar before modification.
    let mut realized = Vec::new();
    for face in d.faces {
        if face.vertices.len() == 4 {
            let p: [DVec3; 4] = std::array::from_fn(|i| d.points[face.vertices[i]].position);
            let normal = (p[1] - p[0]).cross(p[2] - p[0]);
            let scale = (p[3] - p[0]).length().max(1.);
            if normal.length_squared() > 1e-24
                && normal.normalize().dot(p[3] - p[0]).abs() > scale * 1e-10
            {
                for tri in [[0, 1, 2], [0, 2, 3]] {
                    realized.push(Face {
                        vertices: tri.map(|i| face.vertices[i]).to_vec(),
                        source: face.source.clone(),
                        corners: tri.iter().map(|i| face.corners[*i].clone()).collect(),
                        uv: face.uv.as_ref().map(|uv| tri.map(|i| uv[i]).to_vec()),
                    });
                }
                continue;
            }
        }
        realized.push(face);
    }
    d.faces = realized;
    d.compact();
    budget.check(d.points.len(), d.faces.len())?;
    let polygons: Vec<_> = d
        .faces
        .iter()
        .map(|f| f.vertices.iter().map(|i| *i as u32).collect())
        .collect();
    let mut output = Mesh::from_polygons(
        Positions::F64(d.points.iter().map(|p| p.position.to_array()).collect()),
        &polygons,
        &[],
    )?;
    let point_weights: Vec<_> = d.points.iter().map(|p| p.source.clone()).collect();
    let face_weights: Vec<_> = d.faces.iter().map(|f| f.source.clone()).collect();
    let corner_weights: Vec<_> = d
        .faces
        .iter()
        .flat_map(|f| f.corners.iter().cloned())
        .collect();
    let edge_weights: Vec<_> = output
        .edges
        .iter()
        .map(|edge| {
            let points: BTreeSet<_> = edge
                .iter()
                .flat_map(|i| d.points[*i as usize].source.iter().map(|(p, _)| *p))
                .collect();
            if points.len() != 2 {
                return vec![];
            }
            source
                .edges
                .iter()
                .position(|e| e.iter().all(|p| points.contains(&(*p as usize))))
                .map_or_else(Vec::new, |i| vec![(i, 1.)])
        })
        .collect();
    let (ids, points) = assign(&source.point_ids, &point_weights)?;
    output.point_ids = ids;
    let (ids, edges) = assign(&source.edge_ids, &edge_weights)?;
    output.edge_ids = ids;
    let (ids, faces) = assign(&source.face_ids, &face_weights)?;
    output.face_ids = ids;
    let (ids, corners) = assign(&source.corner_ids, &corner_weights)?;
    output.corner_ids = ids;
    for (name, attribute) in &source.attributes {
        if matches!(
            attribute.semantic.as_str(),
            "normal" | "tangent" | "tangent_sign"
        ) {
            continue;
        }
        let weights = match attribute.domain {
            Domain::Point => &point_weights,
            Domain::Edge => &edge_weights,
            Domain::Face => &face_weights,
            Domain::Corner => &corner_weights,
        };
        let mut out = attribute.clone();
        out.values = transfer(&attribute.values, &attribute.transfer, weights)?;
        if attribute.semantic == "uv" {
            let AttributeValues::Vec2(uv) = &mut out.values else {
                unreachable!()
            };
            let mut start = 0;
            for face in &d.faces {
                if let Some(values) = &face.uv {
                    uv[start..start + values.len()].copy_from_slice(values);
                }
                start += face.vertices.len();
            }
        }
        output.attributes.insert(name.clone(), out);
    }
    output.default_uv_attribute = source.default_uv_attribute;
    output.triangles()?;
    output = EditMesh::new(&output)?.commit()?.mesh;
    let bytes = canonical(&output)?.len();
    if bytes as u64 > budget.bytes {
        return Err(Error::new("budget", "modeling output exceeds byte budget"));
    }
    let receipt=Receipt{source:source.content_id()?,output:output.content_id()?,operation_digest:digest(&canonical(operation)?),points,edges,faces,corners,output_bytes:bytes,policy:"exclusive radial validation; global array/connectivity rebuild; explicit local correspondence; warped new quads realize diagonal 0-2; declared point/corner/face transfer; new edge data requires defaults; derived shading frames recomputed; no implicit topology-reference rebinding".into()};
    Ok((output, receipt))
}
fn finite(x: f64) -> Result<()> {
    if !x.is_finite() || x.abs() > 1e9 {
        Err(Error::new(
            "modeling",
            "parameter outside finite metric range",
        ))
    } else {
        Ok(())
    }
}
fn face_index(m: &Mesh, id: u64) -> Result<usize> {
    m.face_ids
        .iter()
        .position(|x| *x == id)
        .ok_or_else(|| Error::new("stale_selection", "selected face ID missing"))
}
fn edge_index(m: &Mesh, id: u64) -> Result<usize> {
    m.edge_ids
        .iter()
        .position(|x| *x == id)
        .ok_or_else(|| Error::new("stale_selection", "selected edge ID missing"))
}
fn manifold(m: &Mesh, closed: bool) -> Result<Vec<Vec<usize>>> {
    let mut fans = vec![vec![]; m.edges.len()];
    for (f, w) in m.face_offsets.windows(2).enumerate() {
        for c in &m.corners[w[0] as usize..w[1] as usize] {
            fans[c.edge as usize].push(f);
        }
    }
    if fans
        .iter()
        .any(|f| f.is_empty() || f.len() > 2 || (closed && f.len() != 2))
    {
        return Err(Error::new(
            "topology_profile",
            "operator requires the documented manifold surface profile",
        ));
    }
    Ok(fans)
}
pub fn apply(
    source: &Mesh,
    operation: &Operation,
    budget: &Budget,
    mut cancelled: impl FnMut() -> bool,
) -> Result<(Mesh, Receipt)> {
    if cancelled() {
        return Err(Error::new("cancelled", "modeling cancelled"));
    }
    source.validate()?;
    budget.check(source.positions.len(), source.faces())?;
    let used_edges: BTreeSet<_> = source.corners.iter().map(|c| c.edge as usize).collect();
    let used_points: BTreeSet<_> = source.corners.iter().map(|c| c.vertex as usize).collect();
    if used_edges.len() != source.edges.len() || used_points.len() != source.positions.len() {
        return Err(Error::new(
            "topology_profile",
            "surface operations reject loose elements",
        ));
    }
    if source.corners.len() > 262144 || source.face_offsets.windows(2).any(|w| w[1] - w[0] > 1024) {
        return Err(Error::new(
            "budget",
            "modeling corner/polygon work budget exceeded",
        ));
    }
    source.triangles()?;
    let estimate = match operation {
        Operation::Subdivide => (
            source.positions.len() + source.edges.len() + source.faces(),
            source.corners.len(),
        ),
        Operation::Solidify { .. } | Operation::Mirror { .. } => (
            source.positions.len() * 2,
            source.faces() * 2 + source.edges.len(),
        ),
        Operation::InsetFace { face, .. } | Operation::ExtrudeFace { face, .. } => {
            let f = face_index(source, *face)?;
            let n = (source.face_offsets[f + 1] - source.face_offsets[f]) as usize;
            (source.positions.len() + n, source.faces() + n)
        }
        Operation::SplitEdge { .. } => (source.positions.len() + 1, source.faces()),
        _ => (source.positions.len(), source.faces()),
    };
    budget.check(estimate.0, estimate.1)?;
    let mut d = Draft::from_mesh(source);
    match operation {
        Operation::Triangulate => {
            let triangles = source.triangles()?;
            let old = d.faces.clone();
            d.faces.clear();
            for tri in triangles {
                let f = source
                    .face_offsets
                    .windows(2)
                    .position(|w| w[0] <= tri[0] && tri[0] < w[1])
                    .expect("triangle owner");
                let start = source.face_offsets[f];
                d.faces.push(Face {
                    vertices: tri
                        .map(|c| source.corners[c as usize].vertex as usize)
                        .to_vec(),
                    source: old[f].source.clone(),
                    corners: tri
                        .iter()
                        .map(|c| old[f].corners[(c - start) as usize].clone())
                        .collect(),
                    uv: None,
                });
            }
        }
        Operation::ExtrudeFace {
            face,
            distance_meters,
        } => {
            finite(*distance_meters)?;
            if distance_meters.abs() < 1e-9 {
                return Err(Error::new(
                    "degenerate",
                    "extrusion distance must be nonzero",
                ));
            }
            let f = face_index(source, *face)?;
            let normal = d.normal(f)?;
            let old = d.faces[f].clone();
            budget.check(
                d.points.len() + old.vertices.len(),
                d.faces.len() + old.vertices.len(),
            )?;
            let mut top = vec![];
            for &i in &old.vertices {
                top.push(d.points.len());
                let mut p = d.points[i].clone();
                p.position += normal * distance_meters;
                d.points.push(p);
            }
            d.faces[f].vertices = top.clone();
            for i in 0..old.vertices.len() {
                let j = (i + 1) % old.vertices.len();
                d.faces.push(Face {
                    vertices: vec![old.vertices[i], old.vertices[j], top[j], top[i]],
                    source: old.source.clone(),
                    corners: vec![
                        old.corners[i].clone(),
                        old.corners[j].clone(),
                        old.corners[j].clone(),
                        old.corners[i].clone(),
                    ],
                    uv: Some(vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]),
                });
            }
        }
        Operation::InsetFace { face, fraction } => {
            if !fraction.is_finite() || !(0.0..1.).contains(fraction) || *fraction == 0. {
                return Err(Error::new("modeling", "inset fraction must be in (0,1)"));
            }
            let f = face_index(source, *face)?;
            let old = d.faces[f].clone();
            let n = d.normal(f)?;
            let center = old
                .vertices
                .iter()
                .map(|i| d.points[*i].position)
                .sum::<DVec3>()
                / old.vertices.len() as f64;
            for i in 0..old.vertices.len() {
                let a = d.points[old.vertices[i]].position;
                let b = d.points[old.vertices[(i + 1) % old.vertices.len()]].position;
                if (b - a).cross(center - a).dot(n) <= 1e-12 {
                    return Err(Error::new(
                        "topology_profile",
                        "inset requires a strictly convex planar face",
                    ));
                }
            }
            let centroid = blend(
                old.vertices
                    .iter()
                    .map(|i| (d.points[*i].source.clone(), 1. / old.vertices.len() as f64)),
            );
            let mut inner = vec![];
            for i in &old.vertices {
                let p = &d.points[*i];
                inner.push(d.points.len());
                d.points.push(Point {
                    position: p.position.lerp(center, *fraction),
                    source: blend([
                        (p.source.clone(), 1. - fraction),
                        (centroid.clone(), *fraction),
                    ]),
                });
            }
            let center_uv = blend(
                old.corners
                    .iter()
                    .map(|w| (w.clone(), 1. / old.corners.len() as f64)),
            );
            let inner_corners: Vec<_> = old
                .corners
                .iter()
                .map(|w| blend([(w.clone(), 1. - fraction), (center_uv.clone(), *fraction)]))
                .collect();
            d.faces[f].corners = inner_corners.clone();
            d.faces[f].vertices = inner.clone();
            for i in 0..old.vertices.len() {
                let j = (i + 1) % old.vertices.len();
                d.faces.push(Face {
                    vertices: vec![old.vertices[i], old.vertices[j], inner[j], inner[i]],
                    source: old.source.clone(),
                    corners: vec![
                        old.corners[i].clone(),
                        old.corners[j].clone(),
                        inner_corners[j].clone(),
                        inner_corners[i].clone(),
                    ],
                    uv: None,
                });
            }
        }
        Operation::SplitEdge { edge, fraction } => {
            if !fraction.is_finite() || *fraction <= 0. || *fraction >= 1. {
                return Err(Error::new(
                    "modeling",
                    "edge split fraction must be in (0,1)",
                ));
            }
            let e = edge_index(source, *edge)?;
            let [a, b] = source.edges[e].map(|i| i as usize);
            let new = d.points.len();
            d.points.push(Point {
                position: d.points[a].position.lerp(d.points[b].position, *fraction),
                source: blend([
                    (d.points[a].source.clone(), 1. - fraction),
                    (d.points[b].source.clone(), *fraction),
                ]),
            });
            for face in &mut d.faces {
                let old = face.clone();
                face.vertices.clear();
                face.corners.clear();
                for i in 0..old.vertices.len() {
                    let j = (i + 1) % old.vertices.len();
                    face.vertices.push(old.vertices[i]);
                    face.corners.push(old.corners[i].clone());
                    if [
                        old.vertices[i].min(old.vertices[j]),
                        old.vertices[i].max(old.vertices[j]),
                    ] == [a.min(b), a.max(b)]
                    {
                        let t = if old.vertices[i] == a {
                            *fraction
                        } else {
                            1. - fraction
                        };
                        face.vertices.push(new);
                        face.corners.push(blend([
                            (old.corners[i].clone(), 1. - t),
                            (old.corners[j].clone(), t),
                        ]));
                    }
                }
            }
        }
        Operation::Weld { points } => {
            if points.len() < 2 {
                return Err(Error::new("selection", "weld needs at least two points"));
            }
            let indices: Result<BTreeSet<_>> = points
                .iter()
                .map(|id| {
                    source
                        .point_ids
                        .iter()
                        .position(|p| p == id)
                        .ok_or_else(|| Error::new("stale_selection", "weld point missing"))
                })
                .collect();
            let indices = indices?;
            if indices.len() != points.len() {
                return Err(Error::new("duplicate_id", "duplicate weld point"));
            }
            let keep = *indices.first().expect("selected points");
            let position =
                indices.iter().map(|i| d.points[*i].position).sum::<DVec3>() / indices.len() as f64;
            let weights = blend(
                indices
                    .iter()
                    .map(|i| (d.points[*i].source.clone(), 1. / indices.len() as f64)),
            );
            d.points[keep] = Point {
                position,
                source: weights,
            };
            for face in &mut d.faces {
                for p in &mut face.vertices {
                    if indices.contains(p) {
                        *p = keep;
                    }
                }
                let old = face.clone();
                face.vertices.clear();
                face.corners.clear();
                for i in 0..old.vertices.len() {
                    if old.vertices[i]
                        != old.vertices[(i + old.vertices.len() - 1) % old.vertices.len()]
                    {
                        face.vertices.push(old.vertices[i]);
                        face.corners.push(old.corners[i].clone());
                    }
                }
                if face.vertices.len() != face.vertices.iter().collect::<BTreeSet<_>>().len() {
                    return Err(Error::new(
                        "degenerate",
                        "weld would create a self-touching face",
                    ));
                }
            }
            d.faces.retain(|f| f.vertices.len() >= 3);
        }
        Operation::DissolveEdge { edge } => {
            let e = edge_index(source, *edge)?;
            let fans = manifold(source, false)?;
            if fans[e].len() != 2 {
                return Err(Error::new(
                    "topology_profile",
                    "dissolve requires an interior two-face edge",
                ));
            }
            let (a, b) = (fans[e][0], fans[e][1]);
            if d.normal(a)?.dot(d.normal(b)?) < 1. - 1e-10 {
                return Err(Error::new(
                    "topology_profile",
                    "dissolve requires coplanar equally oriented faces",
                ));
            }
            let [u, v] = source.edges[e].map(|i| i as usize);
            let mut segments = BTreeMap::new();
            for f in [a, b] {
                let face = &d.faces[f];
                for i in 0..face.vertices.len() {
                    let j = (i + 1) % face.vertices.len();
                    let (x, y) = (face.vertices[i], face.vertices[j]);
                    if [x.min(y), x.max(y)] == [u.min(v), u.max(v)] {
                        continue;
                    }
                    if segments.insert(x, (y, face.corners[i].clone())).is_some() {
                        return Err(Error::new(
                            "topology_profile",
                            "dissolve boundary is not a simple loop",
                        ));
                    }
                }
            }
            let first = *segments.keys().next().expect("boundary");
            let mut at = first;
            let mut vertices = vec![];
            let mut corners = vec![];
            for _ in 0..segments.len() {
                let (next, c) = segments
                    .get(&at)
                    .ok_or_else(|| Error::new("topology_profile", "dissolve boundary broken"))?;
                vertices.push(at);
                corners.push(c.clone());
                at = *next;
            }
            if at != first {
                return Err(Error::new(
                    "topology_profile",
                    "dissolve boundary does not close",
                ));
            }
            let source = blend([
                (d.faces[a].source.clone(), 0.5),
                (d.faces[b].source.clone(), 0.5),
            ]);
            d.faces[a] = Face {
                vertices,
                source,
                corners,
                uv: None,
            };
            d.faces.remove(b);
        }
        Operation::Mirror {
            axis,
            offset_meters,
        } => {
            if *axis > 2 {
                return Err(Error::new("modeling", "mirror axis must be 0..2"));
            }
            finite(*offset_meters)?;
            budget.check(d.points.len() * 2, d.faces.len() * 2)?;
            let old = d.clone();
            let start = d.points.len();
            for mut p in old.points {
                p.position[*axis as usize] = 2. * offset_meters - p.position[*axis as usize];
                d.points.push(p);
            }
            for mut f in old.faces {
                f.vertices = f.vertices.iter().rev().map(|i| start + i).collect();
                f.corners.reverse();
                d.faces.push(f);
            }
        }
        Operation::Array { count, step } => {
            if *count == 0 || *count > 128 || step.iter().any(|x| !x.is_finite() || x.abs() > 1e9) {
                return Err(Error::new("budget", "array count/step exceeds profile"));
            }
            budget.check(
                d.points.len() * (*count as usize),
                d.faces.len() * (*count as usize),
            )?;
            let old = d.clone();
            for i in 1..*count {
                if cancelled() {
                    return Err(Error::new("cancelled", "array evaluation cancelled"));
                }
                let start = d.points.len();
                for p in &old.points {
                    let mut p = p.clone();
                    p.position += DVec3::from_array(*step) * f64::from(i);
                    d.points.push(p);
                }
                for f in &old.faces {
                    let mut f = f.clone();
                    for v in &mut f.vertices {
                        *v += start;
                    }
                    d.faces.push(f);
                }
            }
        }
        Operation::Bridge { first, second } => bridge(source, &mut d, first, second)?,
        Operation::Subdivide => d = subdivide(source, &d)?,
        Operation::Solidify { thickness_meters } => solidify(source, &mut d, *thickness_meters)?,
        Operation::BevelBox { distance_meters } => d = bevel_box(source, *distance_meters)?,
        Operation::BooleanBox {
            operation,
            min,
            max,
        } => d = boolean_box(source, *operation, *min, *max)?,
    }
    if cancelled() {
        return Err(Error::new("cancelled", "modeling candidate discarded"));
    }
    finish(source, d, operation, budget)
}

fn bridge(m: &Mesh, d: &mut Draft, first: &[u64], second: &[u64]) -> Result<()> {
    if first.len() < 3 || first.len() != second.len() || first.len() > 4096 {
        return Err(Error::new(
            "selection",
            "bridge requires equally sized boundary loops of 3..4096 points",
        ));
    }
    let fans = manifold(m, false)?;
    let mut seen = BTreeSet::new();
    let resolve = |id: &u64| {
        m.point_ids
            .iter()
            .position(|v| v == id)
            .ok_or_else(|| Error::new("stale_selection", "bridge point missing"))
    };
    let a: Vec<_> = first.iter().map(resolve).collect::<Result<_>>()?;
    let b: Vec<_> = second.iter().map(resolve).collect::<Result<_>>()?;
    for &i in a.iter().chain(&b) {
        if !seen.insert(i) {
            return Err(Error::new(
                "selection",
                "bridge loops must be disjoint and unique",
            ));
        }
    }
    let boundary = |x: usize, y: usize| -> Result<(usize, usize)> {
        let edge = m
            .edges
            .iter()
            .position(|e| e.map(|v| v as usize) == [x, y] || e.map(|v| v as usize) == [y, x])
            .ok_or_else(|| Error::new("selection", "bridge loop edge missing"))?;
        if fans[edge].len() != 1 {
            return Err(Error::new(
                "topology_profile",
                "bridge loops must follow boundary edges",
            ));
        }
        let face = fans[edge][0];
        let corner = (m.face_offsets[face]..m.face_offsets[face + 1])
            .find(|c| {
                m.corners[*c as usize].vertex as usize == x
                    && m.corners[*c as usize].edge as usize == edge
            })
            .ok_or_else(|| {
                Error::new(
                    "orientation",
                    "first loop follows existing face winding; second must oppose it",
                )
            })?;
        Ok((face, corner as usize))
    };
    for i in 0..a.len() {
        let j = (i + 1) % a.len();
        let (f, ca) = boundary(a[i], a[j])?;
        let (_, cb) = boundary(b[j], b[i])?;
        let f2 = (m.face_offsets[f]..m.face_offsets[f + 1])
            .find(|c| m.corners[*c as usize].vertex as usize == a[j])
            .expect("face corner") as usize;
        let bface = m
            .face_offsets
            .windows(2)
            .position(|w| w[0] as usize <= cb && cb < (w[1] as usize))
            .expect("face");
        let b2 = (m.face_offsets[bface]..m.face_offsets[bface + 1])
            .find(|c| m.corners[*c as usize].vertex as usize == b[i])
            .expect("face corner") as usize;
        d.faces.push(Face {
            vertices: vec![a[j], a[i], b[i], b[j]],
            source: vec![(f, 1.)],
            corners: vec![
                vec![(f2, 1.)],
                vec![(ca, 1.)],
                vec![(b2, 1.)],
                vec![(cb, 1.)],
            ],
            uv: Some(vec![[1., 0.], [0., 0.], [0., 1.], [1., 1.]]),
        });
    }
    Ok(())
}
fn subdivide(m: &Mesh, d: &Draft) -> Result<Draft> {
    let fans = manifold(m, false)?;
    if d.faces.iter().any(|f| f.vertices.len() != 4) {
        return Err(Error::new(
            "topology_profile",
            "Catmull-Clark v0 requires quad faces",
        ));
    }
    let mut points = d.points.clone();
    let mut face_points = vec![];
    for face in &d.faces {
        face_points.push(points.len());
        points.push(Point {
            position: face
                .vertices
                .iter()
                .map(|i| d.points[*i].position)
                .sum::<DVec3>()
                / 4.,
            source: blend(
                face.vertices
                    .iter()
                    .map(|i| (d.points[*i].source.clone(), 0.25)),
            ),
        });
    }
    let mut edge_points = vec![];
    for (e, vertices) in m.edges.iter().enumerate() {
        let [a, b] = vertices.map(|i| i as usize);
        edge_points.push(points.len());
        let (position, source) = if fans[e].len() == 1 {
            (
                (d.points[a].position + d.points[b].position) * 0.5,
                blend([
                    (d.points[a].source.clone(), 0.5),
                    (d.points[b].source.clone(), 0.5),
                ]),
            )
        } else {
            let (f, g) = (face_points[fans[e][0]], face_points[fans[e][1]]);
            (
                (d.points[a].position
                    + d.points[b].position
                    + points[f].position
                    + points[g].position)
                    * 0.25,
                blend([
                    (d.points[a].source.clone(), 0.25),
                    (d.points[b].source.clone(), 0.25),
                    (points[f].source.clone(), 0.25),
                    (points[g].source.clone(), 0.25),
                ]),
            )
        };
        points.push(Point { position, source });
    }
    let mut incident = vec![vec![]; d.points.len()];
    for (e, vertices) in m.edges.iter().enumerate() {
        for &v in vertices {
            incident[v as usize].push(e);
        }
    }
    for (i, original) in d.points.iter().enumerate() {
        let edges = &incident[i];
        let boundary: Vec<_> = edges
            .iter()
            .filter(|e| fans[**e].len() == 1)
            .map(|e| {
                m.edges[*e]
                    .iter()
                    .find(|v| **v != i as u32)
                    .copied()
                    .expect("other vertex") as usize
            })
            .collect();
        if boundary.len() == 2 {
            points[i].position = original.position * 0.75
                + (d.points[boundary[0]].position + d.points[boundary[1]].position) * 0.125;
            points[i].source = blend([
                (original.source.clone(), 0.75),
                (d.points[boundary[0]].source.clone(), 0.125),
                (d.points[boundary[1]].source.clone(), 0.125),
            ]);
        } else if boundary.is_empty() && edges.len() >= 3 {
            let adjacent: BTreeSet<_> = edges
                .iter()
                .flat_map(|e| fans[*e].iter().copied())
                .collect();
            if adjacent.len() != edges.len() {
                return Err(Error::new(
                    "topology_profile",
                    "subdivision vertex fan is not manifold",
                ));
            }
            let n = edges.len() as f64;
            let f = adjacent
                .iter()
                .map(|f| points[face_points[*f]].position)
                .sum::<DVec3>()
                / n;
            let r = edges
                .iter()
                .map(|e| {
                    let [a, b] = m.edges[*e].map(|i| i as usize);
                    (d.points[a].position + d.points[b].position) * 0.5
                })
                .sum::<DVec3>()
                / n;
            let fw = blend(
                adjacent
                    .iter()
                    .map(|f| (points[face_points[*f]].source.clone(), 1. / (n * n))),
            );
            let rw = blend(edges.iter().flat_map(|e| {
                m.edges[*e].map(|v| (d.points[v as usize].source.clone(), 1. / (n * n)))
            }));
            points[i].position = (f + 2. * r + (n - 3.) * original.position) / n;
            points[i].source = blend([(fw, 1.), (rw, 1.), (original.source.clone(), (n - 3.) / n)]);
        } else {
            return Err(Error::new(
                "topology_profile",
                "subdivision requires a single interior or two-edge boundary fan",
            ));
        }
    }
    let mut faces = vec![];
    for (f, face) in d.faces.iter().enumerate() {
        let centroid = blend(face.corners.iter().map(|w| (w.clone(), 0.25)));
        let start = m.face_offsets[f] as usize;
        for i in 0..4 {
            let next = (i + 1) % 4;
            let previous = (i + 3) % 4;
            let outgoing = m.corners[start + i].edge as usize;
            let incoming = m.corners[start + previous].edge as usize;
            faces.push(Face {
                vertices: vec![
                    face.vertices[i],
                    edge_points[outgoing],
                    face_points[f],
                    edge_points[incoming],
                ],
                source: face.source.clone(),
                corners: vec![
                    face.corners[i].clone(),
                    blend([
                        (face.corners[i].clone(), 0.5),
                        (face.corners[next].clone(), 0.5),
                    ]),
                    centroid.clone(),
                    blend([
                        (face.corners[previous].clone(), 0.5),
                        (face.corners[i].clone(), 0.5),
                    ]),
                ],
                uv: None,
            });
        }
    }
    Ok(Draft { points, faces })
}
fn solidify(m: &Mesh, d: &mut Draft, thickness: f64) -> Result<()> {
    finite(thickness)?;
    if thickness <= 1e-9 {
        return Err(Error::new(
            "modeling",
            "solidify thickness must be positive",
        ));
    }
    let fans = manifold(m, false)?;
    if fans.iter().all(|f| f.len() == 2) {
        return Err(Error::new(
            "topology_profile",
            "solidify v0 requires an open manifold surface",
        ));
    }
    let old = d.clone();
    let mut normals = vec![DVec3::ZERO; old.points.len()];
    for (f, face) in old.faces.iter().enumerate() {
        let n = old.normal(f)?;
        for &i in &face.vertices {
            normals[i] += n;
        }
    }
    let start = d.points.len();
    for (i, p) in old.points.iter().enumerate() {
        if normals[i].length_squared() < 1e-24 {
            return Err(Error::new("degenerate", "solidify normal undefined"));
        }
        let mut p = p.clone();
        p.position -= normals[i].normalize() * thickness;
        d.points.push(p);
    }
    for face in &old.faces {
        let mut back = face.clone();
        back.vertices = back.vertices.iter().rev().map(|i| start + i).collect();
        back.corners.reverse();
        d.faces.push(back);
    }
    for (e, fan) in fans.iter().enumerate() {
        if fan.len() != 1 {
            continue;
        }
        let f = fan[0];
        let face = &old.faces[f];
        let i = (0..face.vertices.len())
            .find(|i| m.corners[m.face_offsets[f] as usize + i].edge as usize == e)
            .expect("boundary corner");
        let j = (i + 1) % face.vertices.len();
        let (a, b) = (face.vertices[i], face.vertices[j]);
        d.faces.push(Face {
            vertices: vec![b, a, start + a, start + b],
            source: face.source.clone(),
            corners: vec![
                face.corners[j].clone(),
                face.corners[i].clone(),
                face.corners[i].clone(),
                face.corners[j].clone(),
            ],
            uv: Some(vec![[1., 0.], [0., 0.], [0., 1.], [1., 1.]]),
        });
    }
    Ok(())
}
pub fn box_mesh(min: [f64; 3], max: [f64; 3]) -> Result<Mesh> {
    let min = DVec3::from_array(min);
    let max = DVec3::from_array(max);
    if !min.is_finite()
        || !max.is_finite()
        || min.abs().max_element() > 1e9
        || max.abs().max_element() > 1e9
        || (max - min).min_element() <= 1e-9
    {
        return Err(Error::new(
            "primitive",
            "box bounds must be finite and strictly ordered",
        ));
    }
    let points = (0..8)
        .map(|i| std::array::from_fn(|a| if i & (1 << a) == 0 { min[a] } else { max[a] }))
        .collect();
    Mesh::from_polygons(
        Positions::F64(points),
        &[
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ]
        .map(|f| f.to_vec()),
        &[],
    )
}
fn axis_box(m: &Mesh) -> Result<(DVec3, DVec3)> {
    if m.positions.len() != 8 || m.faces() != 6 || !m.attributes.is_empty() {
        return Err(Error::new(
            "topology_profile",
            "box Boolean/bevel requires an un-attributed axis-aligned box",
        ));
    }
    manifold(m, true)?;
    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for i in 0..8 {
        min = min.min(m.positions.get(i));
        max = max.max(m.positions.get(i));
    }
    let reference = box_mesh(min.to_array(), max.to_array())?;
    for i in 0..8 {
        if !(0..8).any(|j| m.positions.get(i).distance(reference.positions.get(j)) < 1e-10) {
            return Err(Error::new(
                "topology_profile",
                "mesh is not an axis-aligned box",
            ));
        }
    }
    let draft = Draft::from_mesh(m);
    let center = (min + max) * 0.5;
    for f in 0..6 {
        if draft
            .normal(f)?
            .dot(draft.points[draft.faces[f].vertices[0]].position - center)
            <= 0.
        {
            return Err(Error::new(
                "topology_profile",
                "box faces must point outward",
            ));
        }
        if draft.faces[f].vertices.len() != 4 || draft.normal(f)?.abs().max_element() < 1. - 1e-10 {
            return Err(Error::new(
                "topology_profile",
                "box faces must be axis aligned quads",
            ));
        }
    }
    Ok((min, max))
}
fn nearest_source(m: &Mesh, p: DVec3) -> Weights {
    let i = (0..m.positions.len())
        .min_by(|a, b| {
            m.positions
                .get(*a)
                .distance_squared(p)
                .total_cmp(&m.positions.get(*b).distance_squared(p))
        })
        .expect("source vertices");
    vec![(i, 1.)]
}
fn bevel_box(m: &Mesh, distance: f64) -> Result<Draft> {
    let (min, max) = axis_box(m)?;
    finite(distance)?;
    let half = (max - min) * 0.5;
    let center = (max + min) * 0.5;
    if distance <= 1e-9 || distance >= half.min_element() {
        return Err(Error::new(
            "bevel",
            "bevel width must be positive and below half the shortest box side",
        ));
    }
    let mut planes = vec![];
    for axis in 0..3 {
        for sign in [-1., 1.] {
            let mut n = DVec3::ZERO;
            n[axis] = sign;
            planes.push((n, half[axis]));
        }
    }
    for a in 0..3 {
        for b in a + 1..3 {
            for sa in [-1., 1.] {
                for sb in [-1., 1.] {
                    let mut n = DVec3::ZERO;
                    n[a] = sa;
                    n[b] = sb;
                    planes.push((n, half[a] + half[b] - distance));
                }
            }
        }
    }
    for mask in 0..8 {
        let n = DVec3::from_array(std::array::from_fn(|a| {
            if mask & (1 << a) == 0 { -1. } else { 1. }
        }));
        planes.push((n, half.element_sum() - 2. * distance));
    }
    let mut vertices: Vec<DVec3> = vec![];
    for a in 0..planes.len() {
        for b in a + 1..planes.len() {
            for c in b + 1..planes.len() {
                let mat = DMat3::from_cols(planes[a].0, planes[b].0, planes[c].0).transpose();
                if mat.determinant().abs() < 1e-12 {
                    continue;
                }
                let p = mat.inverse() * DVec3::new(planes[a].1, planes[b].1, planes[c].1);
                if planes.iter().all(|(n, h)| n.dot(p) <= h + 1e-9)
                    && !vertices.iter().any(|v| v.distance(p) < 1e-9)
                {
                    vertices.push(p);
                }
            }
        }
    }
    let mut faces = vec![];
    for (normal, h) in planes {
        let mut indices: Vec<_> = vertices
            .iter()
            .enumerate()
            .filter(|(_, p)| (normal.dot(**p) - h).abs() < 1e-8)
            .map(|(i, _)| i)
            .collect();
        if indices.len() < 3 {
            continue;
        }
        let at = indices.iter().map(|i| vertices[*i]).sum::<DVec3>() / indices.len() as f64;
        let n = normal.normalize();
        let u = if n.z.abs() < 0.9 {
            DVec3::Z.cross(n).normalize()
        } else {
            DVec3::X.cross(n).normalize()
        };
        let v = n.cross(u);
        indices.sort_by(|a, b| {
            let a = vertices[*a] - at;
            let b = vertices[*b] - at;
            a.dot(v)
                .atan2(a.dot(u))
                .total_cmp(&b.dot(v).atan2(b.dot(u)))
        });
        faces.push(Face {
            corners: vec![vec![]; indices.len()],
            vertices: indices,
            source: vec![],
            uv: None,
        });
    }
    Ok(Draft {
        points: vertices
            .into_iter()
            .map(|p| Point {
                position: p + center,
                source: nearest_source(m, p + center),
            })
            .collect(),
        faces,
    })
}
fn boolean_box(m: &Mesh, operation: Boolean, bmin: [f64; 3], bmax: [f64; 3]) -> Result<Draft> {
    let (amin, amax) = axis_box(m)?;
    box_mesh(bmin, bmax)?;
    let bmin = DVec3::from_array(bmin);
    let bmax = DVec3::from_array(bmax);
    let coords: [Vec<f64>; 3] = std::array::from_fn(|a| {
        let mut values = vec![amin[a], amax[a], bmin[a], bmax[a]];
        values.sort_by(f64::total_cmp);
        values.dedup();
        values
    });
    let mut occupied = BTreeSet::new();
    for x in 0..coords[0].len() - 1 {
        for y in 0..coords[1].len() - 1 {
            for z in 0..coords[2].len() - 1 {
                let cell = [x, y, z];
                let p = DVec3::from_array(std::array::from_fn(|a| {
                    (coords[a][cell[a]] + coords[a][cell[a] + 1]) * 0.5
                }));
                let a = p.cmpgt(amin).all() && p.cmplt(amax).all();
                let b = p.cmpgt(bmin).all() && p.cmplt(bmax).all();
                if match operation {
                    Boolean::Union => a || b,
                    Boolean::Intersection => a && b,
                    Boolean::Difference => a && !b,
                } {
                    occupied.insert(cell);
                }
            }
        }
    }
    if occupied.is_empty() {
        return Err(Error::new("empty_geometry", "box Boolean result is empty"));
    }
    let mut points = vec![];
    let mut lookup = BTreeMap::new();
    let mut faces = vec![];
    for cell in &occupied {
        for axis in 0..3 {
            for upper in [false, true] {
                let mut neighbor = *cell;
                let valid = if upper {
                    neighbor[axis] += 1;
                    true
                } else if neighbor[axis] > 0 {
                    neighbor[axis] -= 1;
                    true
                } else {
                    false
                };
                if valid && occupied.contains(&neighbor) {
                    continue;
                }
                let a = (axis + 1) % 3;
                let b = (axis + 2) % 3;
                let mut face = vec![];
                let corners = if upper {
                    [[0, 0], [1, 0], [1, 1], [0, 1]]
                } else {
                    [[0, 0], [0, 1], [1, 1], [1, 0]]
                };
                for uv in corners {
                    let mut corner = *cell;
                    corner[axis] += usize::from(upper);
                    corner[a] += uv[0];
                    corner[b] += uv[1];
                    let index = *lookup.entry(corner).or_insert_with(|| {
                        let p = DVec3::from_array(std::array::from_fn(|a| coords[a][corner[a]]));
                        let i = points.len();
                        points.push(Point {
                            position: p,
                            source: (0..m.positions.len())
                                .find(|i| m.positions.get(*i).distance(p) < 1e-10)
                                .map_or_else(Vec::new, |i| vec![(i, 1.)]),
                        });
                        i
                    });
                    face.push(index);
                }
                faces.push(Face {
                    vertices: face,
                    source: vec![],
                    corners: vec![vec![]; 4],
                    uv: None,
                });
            }
        }
    }
    Ok(Draft { points, faces })
}
