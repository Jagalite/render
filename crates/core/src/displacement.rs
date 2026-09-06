//! Bounded, uniform UV displacement. Authored topology remains immutable.
use crate::{Error, Id, Result, canonical, digest, geometry::*, textures::*};
use glam::{DVec3, Vec2};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Height {
    Constant {
        meters: f64,
    },
    Wave {
        amplitude_meters: f64,
        frequency: [f64; 2],
        phase_radians: f64,
    },
    Image {
        binding: Binding,
        scale_meters: f64,
        bias_meters: f64,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Displacement {
    pub height: Height,
    pub subdivisions: u8,
    pub max_vertices: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub source_digest: String,
    pub policy_digest: String,
    pub subdivisions: u8,
    pub vertices: usize,
    pub triangles: usize,
    pub derived_bytes: usize,
    pub approximation: String,
}
impl Displacement {
    pub fn binding(&self) -> Option<&Binding> {
        if let Height::Image { binding, .. } = &self.height {
            Some(binding)
        } else {
            None
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.subdivisions > 6 || self.max_vertices < 3 || self.max_vertices > 524288 {
            return Err(Error::new(
                "budget",
                "displacement supports 0..6 subdivisions and 3..524288 vertices",
            ));
        }
        let values = match &self.height {
            Height::Constant { meters } => vec![*meters],
            Height::Wave {
                amplitude_meters,
                frequency,
                phase_radians,
            } => vec![
                *amplitude_meters,
                frequency[0],
                frequency[1],
                *phase_radians,
            ],
            Height::Image {
                binding,
                scale_meters,
                bias_meters,
            } => {
                if binding.role != TextureRole::LinearData {
                    return Err(Error::new("texture_role", "height requires linear data"));
                }
                vec![*scale_meters, *bias_meters]
            }
        };
        if values.iter().any(|x| !x.is_finite() || x.abs() > 1e6) {
            return Err(Error::new(
                "displacement",
                "height parameters exceed finite metric range",
            ));
        }
        Ok(())
    }
    fn height(
        &self,
        uv: Vec2,
        images: &BTreeMap<(String, TextureRole), Arc<Pyramid>>,
    ) -> Result<f64> {
        Ok(match &self.height {
            Height::Constant { meters } => *meters,
            Height::Wave {
                amplitude_meters,
                frequency,
                phase_radians,
            } => {
                amplitude_meters
                    * (std::f64::consts::TAU
                        * (f64::from(uv.x) * frequency[0] + f64::from(uv.y) * frequency[1])
                        + phase_radians)
                        .sin()
            }
            Height::Image {
                binding,
                scale_meters,
                bias_meters,
            } => {
                let image = images
                    .get(&(binding.image.clone(), binding.role))
                    .ok_or_else(|| Error::new("reference", "displacement image missing"))?;
                f64::from(image.sample(&binding.sampler, uv, Vec2::ZERO, Vec2::ZERO).x)
                    * scale_meters
                    + bias_meters
            }
        })
    }
    pub fn evaluate(
        &self,
        mesh: &Mesh,
        images: &BTreeMap<(String, TextureRole), Arc<Pyramid>>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<(Mesh, Receipt)> {
        self.validate()?;
        if cancelled() {
            return Err(Error::new("cancelled", "displacement cancelled"));
        }
        mesh.validate()?;
        let points: std::collections::BTreeSet<_> = mesh.corners.iter().map(|c| c.vertex).collect();
        let edges: std::collections::BTreeSet<_> = mesh.corners.iter().map(|c| c.edge).collect();
        if points.len() != mesh.positions.len() || edges.len() != mesh.edges.len() {
            return Err(Error::new(
                "topology_profile",
                "surface displacement rejects loose elements",
            ));
        }
        if mesh.attributes.values().any(|a| {
            !matches!(
                a.semantic.as_str(),
                "uv" | "normal" | "tangent" | "tangent_sign"
            )
        }) {
            return Err(Error::new(
                "attribute_transfer",
                "displacement v0 supports UV and recomputed geometric normals; other attributes need an explicit transfer profile",
            ));
        }
        let triangles = mesh.triangles()?;
        let n = 1u32 << self.subdivisions;
        let per = u64::from(n + 1) * u64::from(n + 2) / 2;
        if triangles.len() as u64 * per > u64::from(self.max_vertices) {
            return Err(Error::new(
                "budget",
                "displacement amplification exceeds vertex limit",
            ));
        }
        let mut normals = vec![DVec3::ZERO; mesh.positions.len()];
        for t in &triangles {
            let v = t.map(|c| mesh.corners[c as usize].vertex as usize);
            let p = v.map(|i| mesh.positions.get(i));
            let normal = (p[1] - p[0]).cross(p[2] - p[0]);
            for i in v {
                normals[i] += normal;
            }
        }
        for normal in &mut normals {
            if normal.length_squared() > 1e-24 {
                *normal = normal.normalize();
            }
        }
        let mut positions = vec![];
        let mut vertex_uv = vec![];
        let mut faces = vec![];
        let mut seams: BTreeMap<Vec<(u64, u32)>, DVec3> = BTreeMap::new();
        for tri in triangles {
            if cancelled() {
                return Err(Error::new("cancelled", "displacement cancelled"));
            }
            let indices = tri.map(|c| mesh.corners[c as usize].vertex as usize);
            let p = indices.map(|i| mesh.positions.get(i));
            let uv = tri.map(|c| mesh.uv(c as usize));
            let normal = indices.map(|i| normals[i]);
            let mut grid = BTreeMap::new();
            for i in 0..=n {
                for j in 0..=n - i {
                    if cancelled() {
                        return Err(Error::new("cancelled", "displacement dicing cancelled"));
                    }
                    let weights = [n - i - j, i, j];
                    let w = weights.map(|v| f64::from(v) / f64::from(n));
                    let uv = uv[0] * w[0] as f32 + uv[1] * w[1] as f32 + uv[2] * w[2] as f32;
                    let normal = normal[0] * w[0] + normal[1] * w[1] + normal[2] * w[2];
                    if normal.length_squared() < 1e-24 {
                        return Err(Error::new(
                            "displacement",
                            "interpolated displacement normal is undefined",
                        ));
                    }
                    let position = p[0] * w[0]
                        + p[1] * w[1]
                        + p[2] * w[2]
                        + normal.normalize() * self.height(uv, images)?;
                    if weights.contains(&0) {
                        let mut edge: Vec<_> = indices
                            .iter()
                            .zip(weights)
                            .filter(|(_, w)| *w != 0)
                            .map(|(v, w)| (mesh.point_ids[*v], w))
                            .collect();
                        edge.sort();
                        if let Some(previous) = seams.insert(edge, position)
                            && previous.distance(position) > 1e-8 * position.length().max(1.)
                        {
                            return Err(Error::new(
                                "displacement_seam",
                                "UV height discontinuity would crack a shared mesh edge",
                            ));
                        }
                    }
                    grid.insert((i, j), positions.len() as u32);
                    positions.push(position.to_array());
                    vertex_uv.push(uv.to_array());
                }
            }
            for i in 0..n {
                for j in 0..n - i {
                    faces.push(vec![grid[&(i, j)], grid[&(i + 1, j)], grid[&(i, j + 1)]]);
                    if i + j + 1 < n {
                        faces.push(vec![
                            grid[&(i + 1, j)],
                            grid[&(i + 1, j + 1)],
                            grid[&(i, j + 1)],
                        ]);
                    }
                }
            }
        }
        let mut derived = Mesh::from_polygons(Positions::F64(positions), &faces, &[])?;
        let corners = derived
            .corners
            .iter()
            .map(|c| vertex_uv[c.vertex as usize])
            .collect();
        derived.attributes.insert(
            "uv".into(),
            Attribute {
                id: Id(1),
                domain: Domain::Corner,
                semantic: "uv".into(),
                transfer: Transfer::Linear,
                values: AttributeValues::Vec2(corners),
            },
        );
        derived.validate()?;
        let receipt=Receipt{source_digest:mesh.content_id()?,policy_digest:digest(&canonical(self)?),subdivisions:self.subdivisions,vertices:derived.positions.len(),triangles:faces.len(),derived_bytes:canonical(&derived)?.len(),approximation:"uniform 4-way triangle dicing; area-weighted source vertex normal displacement; object-meter height; base-level image sampling; crack detection at shared topology edges; geometric output normals; derived correspondence only".into()};
        Ok((derived, receipt))
    }
}
