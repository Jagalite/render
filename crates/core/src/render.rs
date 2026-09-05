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
}
#[derive(Default)]
pub struct Evaluator {
    cache: BTreeMap<String, Arc<Geometry>>,
}
impl Evaluator {
    pub fn evaluate(&mut self, snapshot: &Snapshot) -> Result<Scene> {
        snapshot.validate()?;
        let revision = snapshot.revision()?;
        let s = snapshot.composed()?;
        let mut instances = vec![];
        let mut geometry_builds = 0;
        for e in s.entities.iter() {
            let Some(key) = &e.mesh else {
                continue;
            };
            let mesh = &s.meshes[key];
            let geometry = if let Some(g) = self.cache.get(key) {
                g.clone()
            } else {
                let triangles = mesh
                    .triangles()?
                    .iter()
                    .map(|c| Triangle {
                        positions: c
                            .map(|i| mesh.positions.get(mesh.corners[i as usize].vertex as usize)),
                        uv: c.map(|i| mesh.uv(i as usize)),
                    })
                    .collect::<Vec<_>>();
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
            if material.metallic != 0. || material.roughness != 1. {
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
        Ok(Scene {
            revision,
            instances,
            bvh,
            geometry_builds,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Hit {
    pub distance: f64,
    pub position: DVec3,
    pub normal: DVec3,
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
                    nearest = t;
                    let n = (tri.positions[1] - tri.positions[0])
                        .cross(tri.positions[2] - tri.positions[0]);
                    let mut normal = (inst.inverse.matrix3.transpose() * n).normalize()
                        * inst.transform.matrix3.determinant().signum();
                    if normal.dot(ray.direction) > 0. {
                        normal = -normal;
                    }
                    hit = Some(Hit {
                        distance: t,
                        position: ray.origin + t * ray.direction,
                        normal,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub vertical_fov_radians: f64,
}
impl Camera {
    pub fn basis(&self) -> Result<(DVec3, DVec3, DVec3)> {
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
        let half = (self.vertical_fov_radians / 2.).tan();
        let sx = (2. * x / width as f64 - 1.) * width as f64 / height as f64 * half;
        let sy = (1. - 2. * y / height as f64) * half;
        Ok(Ray {
            origin: DVec3::from_array(self.position),
            direction: (f + r * sx + u * sy).normalize(),
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointLight {
    pub position: [f64; 3],
    pub intensity: [f32; 3],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub fn render(scene: &Scene, s: &Settings, mut cancelled: impl FnMut() -> bool) -> Result<Image> {
    s.validate()?;
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
                    let Some(hit) = scene.intersect(ray, 1e-5, f64::INFINITY) else {
                        sum += throughput * Vec3::from_array(s.environment);
                        break;
                    };
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
                            * (cosine / (std::f64::consts::PI * distance * distance)) as f32;
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
                        sum += throughput * Vec3::from_array(s.environment);
                    }
                }
            }
            linear[pixel as usize] = (sum / s.samples as f32).to_array();
        }
    }
    let receipt = RenderReceipt {
        revision: scene.revision.clone(),
        settings_digest: digest(&canonical(s)?),
        backend: "cpu-f64-diffuse-v0".into(),
        samples: s.samples,
        seed: s.seed,
        color_space: "linear-sRGB".into(),
        approximation: format!(
            "finite depth {}; two-sided Lambertian; point light; nearest repeat textures; no MIS needed for discrete point light",
            s.max_depth
        ),
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
