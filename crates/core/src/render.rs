use crate::{
    Error, Id, Result, canonical, digest,
    document::{Material, Snapshot},
};
use glam::{DAffine3, DVec3, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: DVec3,
    pub direction: DVec3,
}
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: DVec3,
    pub max: DVec3,
}
impl Bounds {
    fn empty() -> Self {
        Self {
            min: DVec3::splat(f64::INFINITY),
            max: DVec3::splat(f64::NEG_INFINITY),
        }
    }
    fn point(&mut self, p: DVec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    fn union(&mut self, b: Self) {
        self.point(b.min);
        self.point(b.max);
    }
    pub fn hit(&self, ray: Ray, mut near: f64, mut far: f64) -> bool {
        for axis in 0..3 {
            let d = ray.direction[axis];
            if d == 0. {
                if ray.origin[axis] < self.min[axis] || ray.origin[axis] > self.max[axis] {
                    return false;
                }
            } else {
                let a = (self.min[axis] - ray.origin[axis]) / d;
                let b = (self.max[axis] - ray.origin[axis]) / d;
                near = near.max(a.min(b));
                far = far.min(a.max(b));
                if far < near {
                    return false;
                }
            }
        }
        true
    }
}
#[derive(Clone, Debug)]
pub struct Triangle {
    pub positions: [DVec3; 3],
    pub uv: [Vec2; 3],
    pub normals: Option<[DVec3; 3]>,
    pub tangents: Option<[glam::DVec4; 3]>,
}
impl Triangle {
    pub fn bounds(&self) -> Bounds {
        let mut b = Bounds::empty();
        for p in self.positions {
            b.point(p);
        }
        b
    }
    pub fn intersect(&self, r: Ray, tmin: f64, tmax: f64) -> Option<(f64, f64, f64)> {
        let e1 = self.positions[1] - self.positions[0];
        let e2 = self.positions[2] - self.positions[0];
        let p = r.direction.cross(e2);
        let det = e1.dot(p);
        let scale = e1.length() * e2.length() * r.direction.length();
        if det.abs() <= scale * 1e-12 {
            return None;
        }
        let inv = 1. / det;
        let s = r.origin - self.positions[0];
        let u = s.dot(p) * inv;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = s.cross(e1);
        let v = r.direction.dot(q) * inv;
        if v < 0. || u + v > 1. {
            return None;
        }
        let t = e2.dot(q) * inv;
        if t < tmin || t > tmax {
            return None;
        }
        Some((t, u, v))
    }
}
#[derive(Clone, Debug)]
pub struct Node {
    pub bounds: Bounds,
    pub children: Option<[usize; 2]>,
    pub items: Vec<usize>,
}
#[derive(Clone, Debug, Default)]
pub struct Bvh {
    pub nodes: Vec<Node>,
}
impl Bvh {
    pub fn build(bounds: &[Bounds]) -> Self {
        let mut b = Self::default();
        if !bounds.is_empty() {
            b.build_node((0..bounds.len()).collect(), bounds);
        }
        b
    }
    fn build_node(&mut self, mut ids: Vec<usize>, bounds: &[Bounds]) -> usize {
        let mut box_ = Bounds::empty();
        for &i in &ids {
            box_.union(bounds[i]);
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            bounds: box_,
            children: None,
            items: vec![],
        });
        if ids.len() <= 4 {
            self.nodes[index].items = ids;
        } else {
            let span = box_.max - box_.min;
            let axis = if span.x >= span.y && span.x >= span.z {
                0
            } else if span.y >= span.z {
                1
            } else {
                2
            };
            ids.sort_by(|&a, &b| {
                (bounds[a].min[axis] + bounds[a].max[axis])
                    .total_cmp(&(bounds[b].min[axis] + bounds[b].max[axis]))
            });
            let right = ids.split_off(ids.len() / 2);
            let a = self.build_node(ids, bounds);
            let b = self.build_node(right, bounds);
            self.nodes[index].children = Some([a, b]);
        }
        index
    }
    pub fn candidates(&self, ray: Ray, tmin: f64, tmax: f64) -> Vec<usize> {
        if self.nodes.is_empty() {
            return vec![];
        }
        let mut stack = vec![0];
        let mut out = vec![];
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i];
            if !n.bounds.hit(ray, tmin, tmax) {
                continue;
            }
            if let Some([a, b]) = n.children {
                stack.push(b);
                stack.push(a);
            } else {
                out.extend(&n.items);
            }
        }
        out
    }
}
#[derive(Clone, Debug)]
pub struct Geometry {
    pub triangles: Vec<Triangle>,
    pub bvh: Bvh,
}
#[derive(Clone, Debug)]
pub struct Instance {
    pub id: Id,
    pub geometry: Arc<Geometry>,
    pub geometry_id: String,
    pub transform: DAffine3,
    pub inverse: DAffine3,
    pub material: Material,
    pub bounds: Bounds,
}
#[derive(Clone, Debug)]
pub struct Scene {
    pub revision: String,
    pub instances: Vec<Instance>,
    pub bvh: Bvh,
    pub geometry_builds: usize,
    pub conversions: Vec<crate::curves::Conversion>,
    pub media: crate::volumes::Media,
    pub displacements: Vec<crate::displacement::Receipt>,
    pub procedures: Vec<crate::procedural::Receipt>,
    pub images: BTreeMap<(String, crate::textures::TextureRole), Arc<crate::textures::Pyramid>>,
}
#[derive(Default)]
pub struct Evaluator {
    procedures: BTreeMap<String, (Arc<crate::geometry::Mesh>, crate::procedural::Receipt)>,
    displaced: BTreeMap<String, (Arc<crate::geometry::Mesh>, crate::displacement::Receipt)>,
    cache: BTreeMap<String, Arc<Geometry>>,
    derived: BTreeMap<String, (Arc<crate::geometry::Mesh>, crate::curves::Conversion)>,
}
impl Evaluator {
    pub fn evaluate(&mut self, snapshot: &Snapshot) -> Result<Scene> {
        self.evaluate_with_cancel(snapshot, || false)
    }
    pub fn evaluate_with_cancel(
        &mut self,
        snapshot: &Snapshot,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Scene> {
        if cancelled() {
            return Err(Error::new("cancelled", "geometry evaluation cancelled"));
        }
        snapshot.validate()?;
        let revision = snapshot.revision()?;
        let (s, hidden) = snapshot.composition()?;
        let mut images = BTreeMap::new();
        let mut decoded = BTreeMap::new();
        for material in s.materials.values() {
            if let Some(surface) = &material.pbr {
                for binding in surface.bindings().into_iter().flatten() {
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        images.entry((binding.image.clone(), binding.role))
                    {
                        if let std::collections::btree_map::Entry::Vacant(entry) =
                            decoded.entry(binding.image.clone())
                        {
                            entry.insert(s.images[&binding.image].decode()?);
                        }
                        entry.insert(Arc::new(crate::textures::Pyramid::new(
                            &decoded[&binding.image],
                            binding.role,
                        )));
                    }
                }
            }
        }
        let mut instances = vec![];
        let mut geometry_builds = 0;
        let mut conversions = BTreeMap::new();
        let mut displacements = BTreeMap::new();
        let mut procedures = BTreeMap::new();
        for e in s.entities.iter() {
            if hidden.contains(&e.id) {
                continue;
            }
            if cancelled() {
                return Err(Error::new("cancelled", "geometry evaluation cancelled"));
            }
            let groom = s.grooms.get(&e.id);
            let procedure = s.procedural_bindings.get(&e.id);
            let key = if let Some(key) = procedure {
                key.clone()
            } else if let Some(groom) = groom {
                groom.cache_key(&s, e.id)?
            } else if let Some(key) = e.mesh.as_ref().or_else(|| s.geometry_bindings.get(&e.id)) {
                key.clone()
            } else {
                continue;
            };
            let key = &key;
            let mesh = if procedure.is_some() {
                if !self.procedures.contains_key(key) {
                    let result = s.procedural_assets[key].evaluate(&s.meshes, &mut cancelled)?;
                    self.procedures
                        .insert(key.clone(), (Arc::new(result.mesh), result.receipt));
                }
                let (mesh, receipt) = &self.procedures[key];
                procedures.insert(key.clone(), receipt.clone());
                mesh.clone()
            } else if let Some(mesh) = s.meshes.get(key) {
                mesh.clone()
            } else {
                if !self.derived.contains_key(key) {
                    let evaluated = if let Some(groom) = groom {
                        groom.evaluate(&s, e.id, &mut cancelled)?
                    } else {
                        s.geometry_assets[key].evaluate(&mut cancelled)?
                    };
                    self.derived
                        .insert(key.clone(), (Arc::new(evaluated.mesh), evaluated.receipt));
                }
                let (mesh, receipt) = &self.derived[key];
                conversions.insert(key.clone(), receipt.clone());
                mesh.clone()
            };
            let displacement = e
                .material
                .and_then(|id| s.materials.get(&id))
                .and_then(|m| m.pbr.as_ref())
                .and_then(|p| p.displacement.as_ref());
            let displaced_key = displacement
                .map(|d| canonical(&(key, d)).map(|v| digest(&v)))
                .transpose()?;
            let (key, mesh) = if let (Some(d), Some(dkey)) = (displacement, displaced_key.as_ref())
            {
                if !self.displaced.contains_key(dkey) {
                    let (mesh, receipt) = d.evaluate(&mesh, &images, &mut cancelled)?;
                    self.displaced
                        .insert(dkey.clone(), (Arc::new(mesh), receipt));
                }
                let (mesh, receipt) = &self.displaced[dkey];
                displacements.insert(dkey.clone(), receipt.clone());
                (dkey, mesh.clone())
            } else {
                (key, mesh)
            };
            let geometry = if let Some(g) = self.cache.get(key) {
                g.clone()
            } else {
                let triangles = mesh
                    .triangles()?
                    .iter()
                    .map(|c| {
                        fn values(
                            mesh: &crate::geometry::Mesh,
                            semantic: &str,
                            c: &[u32; 3],
                        ) -> Result<Option<[DVec3; 3]>> {
                            use crate::geometry::*;
                            let Some(attribute) =
                                mesh.attributes.values().find(|a| a.semantic == semantic)
                            else {
                                return Ok(None);
                            };
                            let AttributeValues::Vec3(values) = &attribute.values else {
                                return Err(Error::new(
                                    "attribute",
                                    "shading vector requires vec3",
                                ));
                            };
                            let mut out = [DVec3::ZERO; 3];
                            for (j, &corner) in c.iter().enumerate() {
                                let index = match attribute.domain {
                                    Domain::Point => mesh.corners[corner as usize].vertex as usize,
                                    Domain::Corner => corner as usize,
                                    _ => {
                                        return Err(Error::new(
                                            "attribute",
                                            "shading vector requires point/corner domain",
                                        ));
                                    }
                                };
                                out[j] = Vec3::from_array(values[index]).as_dvec3();
                                if out[j].length_squared() < 1e-20 {
                                    return Err(Error::new("attribute", "zero shading vector"));
                                }
                            }
                            Ok(Some(out))
                        }
                        let normals = values(&mesh, "normal", c)?;
                        let tangents = if let Some(t) = values(&mesh, "tangent", c)? {
                            let sign = mesh
                                .attributes
                                .values()
                                .find(|a| a.semantic == "tangent_sign")
                                .ok_or_else(|| Error::new("attribute", "tangent sign missing"))?;
                            let crate::geometry::AttributeValues::Scalar(values) = &sign.values
                            else {
                                return Err(Error::new(
                                    "attribute",
                                    "tangent sign requires scalar",
                                ));
                            };
                            let mut output = [glam::DVec4::ZERO; 3];
                            for (j, &corner) in c.iter().enumerate() {
                                let index = match sign.domain {
                                    crate::geometry::Domain::Point => {
                                        mesh.corners[corner as usize].vertex as usize
                                    }
                                    crate::geometry::Domain::Corner => corner as usize,
                                    _ => {
                                        return Err(Error::new("attribute", "tangent sign domain"));
                                    }
                                };
                                if values[index].abs() != 1. {
                                    return Err(Error::new(
                                        "attribute",
                                        "tangent sign must be -1 or 1",
                                    ));
                                }
                                output[j] = t[j].extend(f64::from(values[index]));
                            }
                            Some(output)
                        } else {
                            None
                        };
                        Ok(Triangle {
                            positions: c.map(|i| {
                                mesh.positions.get(mesh.corners[i as usize].vertex as usize)
                            }),
                            uv: c.map(|i| mesh.uv(i as usize)),
                            normals,
                            tangents,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let bvh = Bvh::build(&triangles.iter().map(Triangle::bounds).collect::<Vec<_>>());
                let g = Arc::new(Geometry { triangles, bvh });
                self.cache.insert(key.clone(), g.clone());
                geometry_builds += 1;
                g
            };
            if geometry.triangles.is_empty() {
                continue;
            }
            let transform = s.world_transform(e.id)?;
            if transform.matrix3.determinant() == 0. {
                return Err(Error::new(
                    "singular",
                    "renderable instance has singular transform",
                ));
            }
            let inverse = transform.inverse();
            if !inverse.is_finite() {
                return Err(Error::new("precision", "instance inverse overflow"));
            }
            let mut bounds = Bounds::empty();
            for t in &geometry.triangles {
                for p in t.positions {
                    bounds.point(transform.transform_point3(p));
                }
            }
            let material = e
                .material
                .and_then(|id| s.materials.get(&id))
                .cloned()
                .unwrap_or_else(|| Material::diffuse(Id(0), [0.8; 3]));
            if material.pbr.is_none() && (material.metallic != 0. || material.roughness != 1.) {
                return Err(Error::new(
                    "unsupported_material",
                    "Lambertian profile requires metallic=0 and roughness=1; other values require a later scattering profile",
                ));
            }
            instances.push(Instance {
                id: e.id,
                geometry,
                geometry_id: key.clone(),
                transform,
                inverse,
                material,
                bounds,
            });
        }
        let bvh = Bvh::build(&instances.iter().map(|i| i.bounds).collect::<Vec<_>>());
        let mut media_assets = Vec::new();
        for (id, key) in &s.volume_bindings {
            if hidden.contains(id) {
                continue;
            }
            media_assets.push((*id, s.volume_assets[key].as_ref(), s.world_transform(*id)?));
        }
        let media = crate::volumes::Media::build(media_assets, &mut cancelled)?;
        Ok(Scene {
            revision,
            instances,
            bvh,
            geometry_builds,
            conversions: conversions.into_values().collect(),
            media,
            displacements: displacements.into_values().collect(),
            procedures: procedures.into_values().collect(),
            images,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Hit {
    pub front_face: bool,
    pub distance: f64,
    pub position: DVec3,
    pub normal: DVec3,
    pub geometric_normal: DVec3,
    pub tangent: DVec3,
    pub tangent_sign: f64,
    pub uv: Vec2,
    pub instance: usize,
    pub triangle: usize,
}
impl Scene {
    pub fn intersect(&self, ray: Ray, tmin: f64, tmax: f64) -> Option<Hit> {
        let mut nearest = tmax;
        let mut hit = None;
        for i in self.bvh.candidates(ray, tmin, tmax) {
            let inst = &self.instances[i];
            let local = Ray {
                origin: inst.inverse.transform_point3(ray.origin),
                direction: inst.inverse.transform_vector3(ray.direction),
            };
            for ti in inst.geometry.bvh.candidates(local, tmin, nearest) {
                let tri = &inst.geometry.triangles[ti];
                if let Some((t, u, v)) = tri.intersect(local, tmin, nearest) {
                    let local_normal = (tri.positions[1] - tri.positions[0])
                        .cross(tri.positions[2] - tri.positions[0])
                        .normalize();
                    let is_pbr = inst.material.pbr.is_some();
                    let mut geometric_normal =
                        (inst.inverse.matrix3.transpose() * local_normal).normalize();
                    if !is_pbr {
                        geometric_normal *= inst.transform.matrix3.determinant().signum();
                    }
                    let back = geometric_normal.dot(ray.direction) > 0.;
                    if back
                        && inst.material.pbr.as_ref().is_some_and(|p| {
                            !p.double_sided
                                && !p.advanced.as_ref().is_some_and(|a| {
                                    matches!(a.model, crate::scattering::Model::Dielectric { .. })
                                })
                        })
                    {
                        continue;
                    }
                    if back {
                        geometric_normal = -geometric_normal;
                    }
                    let local_shading = if is_pbr {
                        tri.normals
                            .map(|n| {
                                let interpolated = n[0] * (1. - u - v) + n[1] * u + n[2] * v;
                                if interpolated.length_squared() > 1e-20 {
                                    interpolated.normalize()
                                } else {
                                    local_normal
                                }
                            })
                            .unwrap_or(local_normal)
                    } else {
                        local_normal
                    };
                    let mut normal = if is_pbr {
                        (inst.inverse.matrix3.transpose() * local_shading).normalize()
                    } else {
                        geometric_normal
                    };
                    if normal.dot(geometric_normal) < 0. {
                        normal = -normal;
                    }
                    let (local_tangent, sign) = if let Some(t) = tri.tangents {
                        let t = t[0] * (1. - u - v) + t[1] * u + t[2] * v;
                        (t.truncate(), if t.w < 0. { -1. } else { 1. })
                    } else {
                        let a = tri.uv[1] - tri.uv[0];
                        let b = tri.uv[2] - tri.uv[0];
                        let determinant = f64::from(a.x * b.y - a.y * b.x);
                        if determinant.abs() > 1e-12 {
                            let e1 = tri.positions[1] - tri.positions[0];
                            let e2 = tri.positions[2] - tri.positions[0];
                            let t = (e1 * f64::from(b.y) - e2 * f64::from(a.y)) / determinant;
                            let bt = (e2 * f64::from(a.x) - e1 * f64::from(b.x)) / determinant;
                            (t, local_shading.cross(t).dot(bt).signum())
                        } else {
                            let helper = if local_shading.z.abs() < 0.999 {
                                DVec3::Z
                            } else {
                                DVec3::X
                            };
                            (helper.cross(local_shading).normalize(), 1.)
                        }
                    };
                    let mut tangent = inst.transform.transform_vector3(local_tangent);
                    tangent -= normal * normal.dot(tangent);
                    if tangent.length_squared() < 1e-20 {
                        tangent = if normal.z.abs() < 0.999 {
                            DVec3::Z
                        } else {
                            DVec3::X
                        }
                        .cross(normal);
                    }
                    tangent = tangent.normalize();
                    let mut tangent_sign = sign * inst.transform.matrix3.determinant().signum();
                    if is_pbr && back {
                        tangent = -tangent;
                        tangent_sign = -tangent_sign;
                    }
                    nearest = t;
                    hit = Some(Hit {
                        front_face: !back,
                        distance: t,
                        position: ray.origin + t * ray.direction,
                        normal,
                        geometric_normal,
                        tangent,
                        tangent_sign,
                        uv: tri.uv[0] * (1. - u - v) as f32
                            + tri.uv[1] * u as f32
                            + tri.uv[2] * v as f32,
                        instance: i,
                        triangle: ti,
                    });
                }
            }
        }
        hit
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub vertical_fov_radians: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lens: Option<crate::cameras::Lens>,
}
impl Camera {
    pub fn fov(&self) -> f64 {
        match self.lens {
            Some(crate::cameras::Lens::Perspective {
                vertical_fov_radians,
                ..
            }) => vertical_fov_radians,
            _ => self.vertical_fov_radians,
        }
    }
    pub fn clip(&self, ray: Ray) -> (f64, f64) {
        let Some(lens) = &self.lens else {
            return (1e-5, f64::INFINITY);
        };
        let cosine = ray
            .direction
            .dot((DVec3::from_array(self.target) - DVec3::from_array(self.position)).normalize());
        match *lens {
            crate::cameras::Lens::Perspective { near, far, .. } => {
                (near / cosine, far.unwrap_or(f64::INFINITY) / cosine)
            }
            crate::cameras::Lens::Orthographic { near, far, .. } => (near.max(1e-5), far),
        }
    }
    pub fn basis(&self) -> Result<(DVec3, DVec3, DVec3)> {
        if let Some(lens) = &self.lens {
            lens.validate()?;
        }
        let p = DVec3::from_array(self.position);
        let d = DVec3::from_array(self.target) - p;
        let up = DVec3::from_array(self.up);
        if !p.is_finite()
            || !d.is_finite()
            || !up.is_finite()
            || d.length_squared() == 0.
            || d.cross(up).length_squared() == 0.
            || !self.vertical_fov_radians.is_finite()
            || self.vertical_fov_radians <= 0.
            || self.vertical_fov_radians >= std::f64::consts::PI
        {
            return Err(Error::new(
                "camera",
                "invalid camera basis or field of view",
            ));
        }
        let forward = d.normalize();
        let right = forward.cross(up).normalize();
        Ok((forward, right, right.cross(forward)))
    }
    pub fn ray(&self, x: f64, y: f64, width: u32, height: u32) -> Result<Ray> {
        let (f, r, u) = self.basis()?;
        if let Some(crate::cameras::Lens::Orthographic { xmag, ymag, .. }) = self.lens {
            return Ok(Ray {
                origin: DVec3::from_array(self.position)
                    + r * ((2. * x / width as f64 - 1.) * xmag)
                    + u * ((1. - 2. * y / height as f64) * ymag),
                direction: f,
            });
        }
        let aspect = match self.lens {
            Some(crate::cameras::Lens::Perspective {
                aspect_ratio: Some(a),
                ..
            }) => a,
            _ => width as f64 / height as f64,
        };
        let half = (self.fov() / 2.).tan();
        let sx = (2. * x / width as f64 - 1.) * aspect * half;
        let sy = (1. - 2. * y / height as f64) * half;
        Ok(Ray {
            origin: DVec3::from_array(self.position),
            direction: (f + r * sx + u * sy).normalize(),
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PointLight {
    pub position: [f64; 3],
    pub intensity: [f32; 3],
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub seed: u32,
    pub max_depth: u32,
    pub max_bytes: u64,
    pub environment: [f32; 3],
    pub light: PointLight,
    pub camera: Camera,
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        self.camera.basis()?;
        let pixels = u64::from(self.width) * u64::from(self.height);
        if pixels == 0
            || self.width > 16384
            || self.height > 16384
            || self.samples == 0
            || self.samples > 65536
            || self.max_depth == 0
            || self.max_depth > 16
        {
            return Err(Error::new(
                "render_settings",
                "dimensions/samples/depth outside profile",
            ));
        }
        if pixels * 64 > self.max_bytes {
            return Err(Error::new(
                "budget",
                "render output exceeds memory admission",
            ));
        }
        if self
            .environment
            .iter()
            .chain(&self.light.intensity)
            .any(|v| !v.is_finite() || *v < 0.)
            || self.light.position.iter().any(|v| !v.is_finite())
        {
            return Err(Error::new("light", "nonfinite or negative light"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderReceipt {
    pub revision: String,
    pub settings_digest: String,
    pub backend: String,
    pub samples: u32,
    pub seed: u32,
    pub color_space: String,
    pub approximation: String,
    pub width: u32,
    pub height: u32,
    pub output_digest: String,
}
#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub linear: Vec<[f32; 3]>,
    pub depth: Vec<f32>,
    pub normals: Vec<[f32; 3]>,
    pub objects: Vec<Option<Id>>,
    pub receipt: RenderReceipt,
}
pub fn random(pixel: u32, sample: u32, dimension: u32, seed: u32) -> f64 {
    let mut x = pixel.wrapping_mul(747796405)
        ^ sample.wrapping_mul(2891336453)
        ^ dimension.wrapping_mul(277803737)
        ^ seed;
    x = ((x >> ((x >> 28) + 4)) ^ x).wrapping_mul(277803737);
    x = (x >> 22) ^ x;
    (f64::from(x >> 8) + 0.5) / 16777216.
}
pub fn cosine_direction(normal: DVec3, u: f64, v: f64) -> DVec3 {
    let r = u.sqrt();
    let phi = std::f64::consts::TAU * v;
    let helper = if normal.z.abs() < 0.999 {
        DVec3::Z
    } else {
        DVec3::X
    };
    let tangent = helper.cross(normal).normalize();
    let bitangent = normal.cross(tangent);
    (tangent * (r * phi.cos()) + bitangent * (r * phi.sin()) + normal * (1. - u).sqrt()).normalize()
}
pub fn diffuse_pdf(cosine: f64) -> f64 {
    cosine.max(0.) / std::f64::consts::PI
}
pub fn albedo(material: &Material, uv: Vec2) -> Vec3 {
    let base = Vec3::from_array(material.base_color);
    if let Some(t) = &material.texture {
        let u = uv.x.rem_euclid(1.);
        let v = uv.y.rem_euclid(1.);
        let x = (u * t.width as f32) as usize;
        let y = (v * t.height as f32) as usize;
        base * Vec3::from_array(
            t.linear_rgb
                [y.min(t.height as usize - 1) * t.width as usize + x.min(t.width as usize - 1)],
        )
    } else {
        base
    }
}
pub fn projected_uv(triangle: &Triangle, ray: Ray) -> Option<Vec2> {
    let e1 = triangle.positions[1] - triangle.positions[0];
    let e2 = triangle.positions[2] - triangle.positions[0];
    let p = ray.direction.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-16 {
        return None;
    }
    let delta = ray.origin - triangle.positions[0];
    let u = delta.dot(p) / det;
    let v = ray.direction.dot(delta.cross(e1)) / det;
    let uv = triangle.uv[0] * (1. - u - v) as f32
        + triangle.uv[1] * u as f32
        + triangle.uv[2] * v as f32;
    if uv.is_finite() { Some(uv) } else { None }
}
#[derive(Debug)]
pub struct Shading {
    pub color: Vec3,
    pub emission: Vec3,
    pub metallic: f32,
    pub roughness: f32,
    pub occlusion: f32,
    pub normal: DVec3,
}
pub fn shading(scene: &Scene, hit: &Hit, differentials: [Ray; 2]) -> Shading {
    let instance = &scene.instances[hit.instance];
    let material = &instance.material;
    let surface = material.pbr.as_ref().expect("PBR material");
    let tri = &instance.geometry.triangles[hit.triangle];
    let d = differentials.map(|ray| {
        projected_uv(
            tri,
            Ray {
                origin: instance.inverse.transform_point3(ray.origin),
                direction: instance.inverse.transform_vector3(ray.direction),
            },
        )
        .map_or(Vec2::ZERO, |uv| uv - hit.uv)
    });
    let texture = |binding: Option<&crate::textures::Binding>| {
        binding.map_or(glam::Vec4::ONE, |b| {
            scene.images[&(b.image.clone(), b.role)].sample(&b.sampler, hit.uv, d[0], d[1])
        })
    };
    let color =
        Vec3::from_array(material.base_color) * texture(surface.base_color.as_ref()).truncate();
    let orm = texture(surface.metallic_roughness.as_ref());
    let emission =
        Vec3::from_array(material.emission) * texture(surface.emission.as_ref()).truncate();
    let occlusion = 1. + surface.occlusion_strength * (texture(surface.occlusion.as_ref()).x - 1.);
    let mut normal = hit.normal;
    if surface.normal.is_some() {
        let mut local = texture(surface.normal.as_ref()).truncate() * 2. - Vec3::ONE;
        local.x *= surface.normal_scale;
        local.y *= surface.normal_scale;
        let mapped = hit.tangent * f64::from(local.x)
            + hit.normal.cross(hit.tangent) * (hit.tangent_sign * f64::from(local.y))
            + hit.normal * f64::from(local.z);
        if mapped.length_squared() > 1e-20 {
            normal = mapped.normalize();
        }
    }
    Shading {
        color,
        emission,
        metallic: material.metallic * orm.z,
        roughness: material.roughness * orm.y,
        occlusion,
        normal,
    }
}
fn shade_pbr(
    scene: &Scene,
    hit: &Hit,
    surface: &Shading,
    settings: &Settings,
    ray: Ray,
    indices: [u32; 2],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Vec3> {
    let n = surface.normal;
    let view = -ray.direction;
    let mut radiance = surface.emission;
    let offset = hit.position + hit.geometric_normal * 1e-5;
    let delta = DVec3::from_array(settings.light.position) - hit.position;
    let distance = delta.length();
    if distance > 1e-5 {
        let l = delta / distance;
        if hit.geometric_normal.dot(l) > 0.
            && n.dot(l) > 0.
            && scene
                .intersect(
                    Ray {
                        origin: offset,
                        direction: l,
                    },
                    1e-5,
                    distance - 2e-5,
                )
                .is_none()
        {
            radiance += (crate::pbr::brdf(
                surface.color,
                surface.metallic,
                surface.roughness,
                n,
                view,
                l,
            ) * (n.dot(l) / (distance * distance)))
                .as_vec3()
                * Vec3::from_array(settings.light.intensity)
                * scene
                    .media
                    .transmittance(
                        Ray {
                            origin: offset,
                            direction: l,
                        },
                        1e-5,
                        distance - 2e-5,
                        &mut *cancelled,
                    )?
                    .as_vec3();
        }
    }
    let [pixel, sample] = indices;
    let l = crate::pbr::sample(
        n,
        view,
        surface.roughness,
        random(pixel, sample, 4, settings.seed),
        random(pixel, sample, 2, settings.seed),
        random(pixel, sample, 3, settings.seed),
    );
    let pdf = crate::pbr::pdf(n, view, l, surface.roughness);
    if pdf > 0.
        && hit.geometric_normal.dot(l) > 0.
        && scene
            .intersect(
                Ray {
                    origin: offset,
                    direction: l,
                },
                1e-5,
                f64::INFINITY,
            )
            .is_none()
    {
        radiance += (crate::pbr::brdf(
            surface.color,
            surface.metallic,
            surface.roughness,
            n,
            view,
            l,
        ) * (n.dot(l) / pdf))
            .as_vec3()
            * Vec3::from_array(settings.environment)
            * surface.occlusion
            * scene
                .media
                .transmittance(
                    Ray {
                        origin: offset,
                        direction: l,
                    },
                    1e-5,
                    f64::INFINITY,
                    &mut *cancelled,
                )?
                .as_vec3();
    }
    Ok(radiance)
}
pub fn render(scene: &Scene, s: &Settings, mut cancelled: impl FnMut() -> bool) -> Result<Image> {
    if scene.instances.iter().any(|i| {
        i.material
            .pbr
            .as_ref()
            .is_some_and(|p| p.advanced.is_some())
    }) {
        return crate::scattering::render(scene, s, cancelled);
    }
    s.validate()?;
    let pbr_profile = scene.instances.iter().any(|i| i.material.pbr.is_some());
    if pbr_profile && s.max_depth != 1 {
        return Err(Error::new(
            "unsupported_profile",
            "PBR v0 supports one bounce",
        ));
    }
    let count = (s.width * s.height) as usize;
    let mut linear = vec![[0.; 3]; count];
    let mut depth = vec![0.; count];
    let mut normals = vec![[0.; 3]; count];
    let mut objects = vec![None; count];
    for y in 0..s.height {
        if cancelled() {
            return Err(Error::new("cancelled", "render cancelled at row boundary"));
        }
        for x in 0..s.width {
            let pixel = y * s.width + x;
            let mut sum = Vec3::ZERO;
            for sample in 0..s.samples {
                if sample.is_multiple_of(16) && cancelled() {
                    return Err(Error::new(
                        "cancelled",
                        "render cancelled at sample boundary",
                    ));
                }
                let mut ray = s.camera.ray(
                    x as f64 + random(pixel, sample, 0, s.seed),
                    y as f64 + random(pixel, sample, 1, s.seed),
                    s.width,
                    s.height,
                )?;
                let mut throughput = Vec3::ONE;
                for bounce in 0..s.max_depth {
                    let (near, far) = if bounce == 0 {
                        s.camera.clip(ray)
                    } else {
                        (1e-5, f64::INFINITY)
                    };
                    let intersection = scene.intersect(ray, near, far);
                    if !scene.media.is_empty() {
                        let volume = scene.media.transport(
                            ray,
                            near,
                            intersection.as_ref().map_or(far, |h| h.distance),
                            |p, cancel| {
                                let delta = DVec3::from_array(s.light.position) - p;
                                let distance = delta.length();
                                if distance < 1e-5 {
                                    return Ok((DVec3::ZERO, DVec3::Z));
                                }
                                let direction = delta / distance;
                                let shadow = Ray {
                                    origin: p,
                                    direction,
                                };
                                let light = if scene.intersect(shadow, 1e-5, distance).is_some() {
                                    DVec3::ZERO
                                } else {
                                    DVec3::from_array(s.light.intensity.map(f64::from))
                                        * scene.media.transmittance(shadow, 0., distance, cancel)?
                                        / (distance * distance)
                                };
                                Ok((light, direction))
                            },
                            &mut cancelled,
                        )?;
                        sum += throughput * volume.radiance.as_vec3();
                        throughput *= volume.transmittance.as_vec3();
                    }
                    let Some(hit) = intersection else {
                        sum += throughput * Vec3::from_array(s.environment);
                        break;
                    };
                    if scene.instances[hit.instance].material.pbr.is_some() {
                        let px = x as f64 + random(pixel, sample, 0, s.seed);
                        let py = y as f64 + random(pixel, sample, 1, s.seed);
                        let surface = shading(
                            scene,
                            &hit,
                            [
                                s.camera.ray(px + 1., py, s.width, s.height)?,
                                s.camera.ray(px, py + 1., s.width, s.height)?,
                            ],
                        );
                        if sample == 0 {
                            depth[pixel as usize] = hit.distance as f32;
                            normals[pixel as usize] = surface.normal.as_vec3().to_array();
                            objects[pixel as usize] = Some(scene.instances[hit.instance].id);
                        }
                        sum += throughput
                            * shade_pbr(
                                scene,
                                &hit,
                                &surface,
                                s,
                                ray,
                                [pixel, sample],
                                &mut cancelled,
                            )?;
                        break;
                    }
                    if bounce == 0 && sample == 0 {
                        depth[pixel as usize] = hit.distance as f32;
                        normals[pixel as usize] = hit.normal.as_vec3().to_array();
                        objects[pixel as usize] = Some(scene.instances[hit.instance].id);
                    }
                    let m = &scene.instances[hit.instance].material;
                    let color = albedo(m, hit.uv);
                    sum += throughput * Vec3::from_array(m.emission);
                    let to_light = DVec3::from_array(s.light.position) - hit.position;
                    let distance = to_light.length();
                    let light_dir = to_light / distance;
                    let cosine = hit.normal.dot(light_dir).max(0.);
                    if cosine > 0.
                        && distance > 1e-5
                        && scene
                            .intersect(
                                Ray {
                                    origin: hit.position + hit.normal * 1e-5,
                                    direction: light_dir,
                                },
                                1e-5,
                                distance - 2e-5,
                            )
                            .is_none()
                    {
                        sum += throughput
                            * color
                            * Vec3::from_array(s.light.intensity)
                            * (cosine / (std::f64::consts::PI * distance * distance)) as f32
                            * scene
                                .media
                                .transmittance(
                                    Ray {
                                        origin: hit.position + hit.normal * 1e-5,
                                        direction: light_dir,
                                    },
                                    1e-5,
                                    distance - 2e-5,
                                    &mut cancelled,
                                )?
                                .as_vec3();
                    }
                    let direction = cosine_direction(
                        hit.normal,
                        random(pixel, sample, 2 + bounce * 2, s.seed),
                        random(pixel, sample, 3 + bounce * 2, s.seed),
                    );
                    ray = Ray {
                        origin: hit.position + hit.normal * 1e-5,
                        direction,
                    };
                    throughput *= color;
                    if bounce + 1 == s.max_depth
                        && scene.intersect(ray, 1e-5, f64::INFINITY).is_none()
                    {
                        sum += throughput
                            * Vec3::from_array(s.environment)
                            * scene
                                .media
                                .transmittance(ray, 1e-5, f64::INFINITY, &mut cancelled)?
                                .as_vec3();
                    }
                }
            }
            linear[pixel as usize] = (sum / s.samples as f32).to_array();
        }
    }
    if linear
        .iter()
        .flatten()
        .chain(normals.iter().flatten())
        .chain(depth.iter())
        .any(|v| !v.is_finite())
    {
        return Err(Error::new(
            "numerics",
            "CPU render produced nonfinite output",
        ));
    }
    let mut receipt = RenderReceipt {
        revision: scene.revision.clone(),
        settings_digest: digest(&canonical(s)?),
        backend: if pbr_profile {
            "cpu-f64-pbr-v0"
        } else {
            "cpu-f64-diffuse-v0"
        }
        .into(),
        samples: s.samples,
        seed: s.seed,
        color_space: "linear-sRGB".into(),
        approximation: if pbr_profile {
            "single-scattering GGX/Schlick/Smith; roughness >=0.05; one bounce; diffuse/GGX mixture PDF; mipmapped textures; geometric visibility with mapped shading normals".into()
        } else {
            format!(
                "finite depth {}; two-sided Lambertian; point light; nearest repeat textures; no MIS needed for discrete point light",
                s.max_depth
            )
        },
        width: s.width,
        height: s.height,
        output_digest: digest(
            &linear
                .iter()
                .flatten()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ),
    };
    if !scene.displacements.is_empty() {
        receipt.approximation.push_str(
            "; bounded uniform geometric displacement; see displacement conversion receipts",
        );
    }
    if !scene.media.is_empty() {
        receipt.backend = "cpu-f64-sparse-media-v0".into();
        receipt.approximation.push_str("; sparse constant RGB cells: exact Beer extinction/emission, bounded midpoint point-light single scattering; shadow/environment attenuation; no indirect in-scattering");
    }
    if !scene.conversions.is_empty() {
        receipt.approximation.push_str(
            "; typed curves/points use bounded polygon sweeps; see evaluation conversion receipts",
        );
    }
    Ok(Image {
        width: s.width,
        height: s.height,
        linear,
        depth,
        normals,
        objects,
        receipt,
    })
}
pub fn linear_to_srgb(x: f32) -> f32 {
    if x <= 0.0031308 {
        12.92 * x
    } else {
        1.055 * x.powf(1. / 2.4) - 0.055
    }
}
impl Image {
    pub fn ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        for p in &self.linear {
            for v in p {
                out.push((linear_to_srgb(*v).clamp(0., 1.) * 255. + 0.5) as u8);
            }
        }
        out
    }
    pub fn pfm(&self) -> Vec<u8> {
        let mut out = format!("PF\n{} {}\n-1.0\n", self.width, self.height).into_bytes();
        for row in self.linear.chunks(self.width as usize).rev() {
            for pixel in row {
                for v in pixel {
                    out.extend(v.to_le_bytes());
                }
            }
        }
        out
    }
}
