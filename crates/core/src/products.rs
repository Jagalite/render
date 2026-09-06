//! Rust color transforms, guided spatial denoising, render views and UV baking.
//! Products preserve scene-linear render passes and carry separate display data.
use crate::{Error, Id, Result, canonical, digest, document::*, render::*};
use glam::{DMat3, DVec3, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpace {
    LinearSrgb,
    Srgb,
    LinearDisplayP3,
    DisplayP3,
}
fn xyz_matrix(space: ColorSpace) -> DMat3 {
    // Primary chromaticities and D65 from W3C CSS Color 4; matrix derived here.
    let xy = match space {
        ColorSpace::LinearSrgb | ColorSpace::Srgb => [[0.64, 0.33], [0.3, 0.6], [0.15, 0.06]],
        _ => [[0.68, 0.32], [0.265, 0.69], [0.15, 0.06]],
    };
    let columns = xy.map(|[x, y]| DVec3::new(x / y, 1., (1. - x - y) / y));
    let m = DMat3::from_cols(columns[0], columns[1], columns[2]);
    let white = DVec3::new(0.3127 / 0.329, 1., (1. - 0.3127 - 0.329) / 0.329);
    m * DMat3::from_diagonal(m.inverse() * white)
}
fn decode(x: f64) -> f64 {
    if x.abs() <= 0.04045 {
        x / 12.92
    } else {
        x.signum() * ((x.abs() + 0.055) / 1.055).powf(2.4)
    }
}
fn encode(x: f64) -> f64 {
    if x.abs() <= 0.0031308 {
        12.92 * x
    } else {
        x.signum() * (1.055 * x.abs().powf(1. / 2.4) - 0.055)
    }
}
pub fn convert(rgb: [f64; 3], from: ColorSpace, to: ColorSpace) -> Result<[f64; 3]> {
    if rgb.iter().any(|x| !x.is_finite() || x.abs() > 1e12) {
        return Err(Error::new("color", "invalid color sample"));
    }
    let linear = if matches!(from, ColorSpace::Srgb | ColorSpace::DisplayP3) {
        rgb.map(decode)
    } else {
        rgb
    };
    let out = (xyz_matrix(to).inverse() * xyz_matrix(from) * DVec3::from_array(linear)).to_array();
    Ok(if matches!(to, ColorSpace::Srgb | ColorSpace::DisplayP3) {
        out.map(encode)
    } else {
        out
    })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bilateral {
    pub radius: u32,
    pub spatial_sigma: f64,
    pub luminance_sigma: f64,
    pub normal_sigma: f64,
    pub relative_depth_sigma: f64,
}
impl Bilateral {
    pub fn validate(&self) -> Result<()> {
        if self.radius == 0
            || self.radius > 4
            || [
                self.spatial_sigma,
                self.luminance_sigma,
                self.normal_sigma,
                self.relative_depth_sigma,
            ]
            .iter()
            .any(|x| !x.is_finite() || *x <= 0. || *x > 1e6)
        {
            return Err(Error::new(
                "denoiser",
                "invalid Rust bilateral-v0 filter policy",
            ));
        }
        Ok(())
    }
    pub fn filter(
        &self,
        image: &Image,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Vec<[f32; 3]>> {
        self.validate()?;
        validate_image(image)?;
        let mut output = vec![[0.; 3]; image.linear.len()];
        let lum = |p: [f32; 3]| {
            f64::from(p[0]) * 0.2126 + f64::from(p[1]) * 0.7152 + f64::from(p[2]) * 0.0722
        };
        for y in 0..image.height {
            if cancelled() {
                return Err(Error::new("cancelled", "denoising cancelled"));
            }
            for x in 0..image.width {
                let i = (y * image.width + x) as usize;
                let mut sum = DVec3::ZERO;
                let mut weights = 0.;
                for dy in -(self.radius as i32)..=self.radius as i32 {
                    for dx in -(self.radius as i32)..=self.radius as i32 {
                        let (nx, ny) = (i64::from(x) + i64::from(dx), i64::from(y) + i64::from(dy));
                        if nx < 0
                            || ny < 0
                            || nx >= i64::from(image.width)
                            || ny >= i64::from(image.height)
                        {
                            continue;
                        }
                        let j = (ny * i64::from(image.width) + nx) as usize;
                        if image.objects[i] != image.objects[j] {
                            continue;
                        }
                        let dl =
                            (lum(image.linear[i]) - lum(image.linear[j])) / self.luminance_sigma;
                        let dn = (Vec3::from_array(image.normals[i])
                            - Vec3::from_array(image.normals[j]))
                        .length() as f64
                            / self.normal_sigma;
                        let dd = f64::from(image.depth[i] - image.depth[j])
                            / (self.relative_depth_sigma
                                * f64::from(image.depth[i].abs()).max(1e-6));
                        let ds = f64::from(dx * dx + dy * dy)
                            / (self.spatial_sigma * self.spatial_sigma);
                        let weight = (-0.5 * (ds + dl * dl + dn * dn + dd * dd)).exp();
                        sum += DVec3::from_array(image.linear[j].map(f64::from)) * weight;
                        weights += weight;
                    }
                }
                output[i] = (sum / weights).as_vec3().to_array();
            }
        }
        Ok(output)
    }
}
fn validate_image(image: &Image) -> Result<()> {
    let count = u64::from(image.width) * u64::from(image.height);
    if count == 0
        || count > 16 * 1024 * 1024
        || image.linear.len() as u64 != count
        || image.normals.len() as u64 != count
        || image.depth.len() as u64 != count
        || image.objects.len() as u64 != count
        || image
            .linear
            .iter()
            .flatten()
            .chain(image.normals.iter().flatten())
            .chain(image.depth.iter())
            .any(|x| !x.is_finite())
    {
        return Err(Error::new("image", "malformed render pass layout"));
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    None,
    ReinhardLuminance,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gamut {
    Preserve,
    Clip,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pipeline {
    pub destination: ColorSpace,
    pub exposure_stops: f64,
    pub tone: Tone,
    pub gamut: Gamut,
    pub denoiser: Option<Bilateral>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Display {
    pub rgb: Vec<[f32; 3]>,
    pub destination: ColorSpace,
    pub source_digest: String,
    pub policy_digest: String,
    pub output_digest: String,
    pub approximation: String,
}
impl Pipeline {
    pub fn validate(&self) -> Result<()> {
        if !self.exposure_stops.is_finite() || self.exposure_stops.abs() > 32. {
            return Err(Error::new("color", "exposure must be within +/-32 stops"));
        }
        if let Some(d) = &self.denoiser {
            d.validate()?;
        }
        Ok(())
    }
    pub fn process(&self, image: &Image, mut cancelled: impl FnMut() -> bool) -> Result<Display> {
        self.validate()?;
        validate_image(image)?;
        if image.receipt.color_space != "linear-sRGB" {
            return Err(Error::new(
                "color",
                "display pipeline requires tagged linear-sRGB input",
            ));
        }
        let filtered = if let Some(d) = &self.denoiser {
            d.filter(image, &mut cancelled)?
        } else {
            image.linear.clone()
        };
        let mut rgb = Vec::with_capacity(filtered.len());
        for p in filtered {
            if cancelled() {
                return Err(Error::new("cancelled", "color conversion cancelled"));
            }
            let mut p = DVec3::from_array(p.map(f64::from)) * 2f64.powf(self.exposure_stops);
            if self.tone == Tone::ReinhardLuminance {
                let y = p.dot(DVec3::new(0.2126, 0.7152, 0.0722)).max(0.);
                p /= 1. + y;
            }
            let mut p = convert(p.to_array(), ColorSpace::LinearSrgb, self.destination)?;
            if self.gamut == Gamut::Clip {
                p = p.map(|x| x.clamp(0., 1.));
            }
            rgb.push(p.map(|x| x as f32));
        }
        let output_digest = digest(
            &rgb.iter()
                .flatten()
                .flat_map(|x| x.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        Ok(Display{rgb,destination:self.destination,source_digest:digest(&canonical(&image.linear)?),policy_digest:digest(&canonical(self)?),output_digest,approximation:"Rust D65 matrix/TRC profiles; explicit exposure, luminance Reinhard and gamut policy; optional object/normal/depth-guided bilateral-v0 spatial bias; no arbitrary ICC/LUT or temporal denoising".into()})
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub name: String,
    pub include: Option<BTreeSet<Id>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BakePass {
    Albedo,
    Emission,
    WorldNormal,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bake {
    pub name: String,
    pub entity: Id,
    pub width: u32,
    pub height: u32,
    pub pass: BakePass,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub pipeline: Pipeline,
    pub views: Vec<View>,
    pub bakes: Vec<Bake>,
}
impl State {
    pub fn validate(&self, s: &Snapshot) -> Result<()> {
        self.pipeline.validate()?;
        if self.views.is_empty() || self.views.len() > 16 || self.bakes.len() > 16 {
            return Err(Error::new(
                "budget",
                "imaging needs 1..16 views and at most 16 bakes",
            ));
        }
        let mut names = BTreeSet::new();
        for v in &self.views {
            if v.name.is_empty() || v.name.len() > 128 || !names.insert(&v.name) {
                return Err(Error::new("view", "invalid or duplicate view name"));
            }
            if let Some(ids) = &v.include {
                for id in ids {
                    if s.entities.get(*id).is_none() {
                        return Err(Error::new("reference", "render view entity missing"));
                    }
                }
            }
        }
        for b in &self.bakes {
            if b.name.is_empty()
                || b.name.len() > 128
                || !names.insert(&b.name)
                || b.width == 0
                || b.height == 0
                || b.width > 2048
                || b.height > 2048
            {
                return Err(Error::new("bake", "invalid name or bake dimensions"));
            }
            if s.entities.get(b.entity).is_none() {
                return Err(Error::new("reference", "bake entity missing"));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Baked {
    pub width: u32,
    pub height: u32,
    pub pass: BakePass,
    pub rgb: Vec<[f32; 3]>,
    pub covered: Vec<bool>,
    pub revision: String,
    pub request_digest: String,
    pub output_digest: String,
    pub approximation: String,
}
pub fn bake(
    scene: &Scene,
    request: &Bake,
    revision: &str,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Baked> {
    scene.validate_uv_bindings()?;
    if scene.revision != revision {
        return Err(Error::new("stale_revision", "bake scene revision changed"));
    }
    if request.width == 0 || request.height == 0 || request.width > 2048 || request.height > 2048 {
        return Err(Error::new(
            "budget",
            "bake resolution limited to 2048 squared",
        ));
    }
    let inst = scene
        .instances
        .iter()
        .find(|i| i.id == request.entity)
        .ok_or_else(|| Error::new("reference", "bake requires visible surface geometry"))?;
    if inst.geometry.triangles.len() > 65536 {
        return Err(Error::new("budget", "bake triangle budget exceeded"));
    }
    let count = (request.width * request.height) as usize;
    let mut rgb = vec![[0.; 3]; count];
    let mut covered = vec![false; count];
    let mut work = 0u64;
    for tri in &inst.geometry.triangles {
        if cancelled() {
            return Err(Error::new("cancelled", "bake cancelled"));
        }
        if tri
            .uv
            .iter()
            .any(|p| p.min_element() < 0. || p.max_element() > 1.)
        {
            return Err(Error::new(
                "bake_uv",
                "baking requires an explicit UV atlas in [0,1]",
            ));
        }
        let [a, b, c] = tri.uv;
        let edge = b - a;
        let other = c - a;
        let det = edge.perp_dot(other);
        if det.abs() < 1e-12 {
            return Err(Error::new("bake_uv", "degenerate UV triangle"));
        }
        let lo = a.min(b).min(c);
        let hi = a.max(b).max(c);
        let xmin = (lo.x * request.width as f32).floor() as u32;
        let xmax = ((hi.x * request.width as f32).ceil() as u32).min(request.width);
        let ymin = (lo.y * request.height as f32).floor() as u32;
        let ymax = ((hi.y * request.height as f32).ceil() as u32).min(request.height);
        work += u64::from(xmax - xmin) * u64::from(ymax - ymin);
        if work > 64 * 1024 * 1024 {
            return Err(Error::new("budget", "bake UV raster work exceeded"));
        }
        for y in ymin..ymax {
            if cancelled() {
                return Err(Error::new("cancelled", "bake raster cancelled"));
            }
            for x in xmin..xmax {
                let uv = Vec2::new(
                    (x as f32 + 0.5) / request.width as f32,
                    (y as f32 + 0.5) / request.height as f32,
                );
                let d = uv - a;
                let u = d.perp_dot(other) / det;
                let v = edge.perp_dot(d) / det;
                let w = 1. - u - v;
                if u < -1e-7 || v < -1e-7 || w < -1e-7 {
                    continue;
                }
                let index = (y * request.width + x) as usize;
                // Coincident coverage only on triangle edges is assigned to the first
                // triangle. Interior overlaps are an explicit atlas error.
                if covered[index] {
                    if u > 1e-6 && v > 1e-6 && w > 1e-6 {
                        return Err(Error::new("bake_overlap", "UV interiors overlap"));
                    }
                    continue;
                }
                let m = &inst.material;
                let value = match request.pass {
                    BakePass::WorldNormal => {
                        let n = (tri.positions[1] - tri.positions[0])
                            .cross(tri.positions[2] - tri.positions[0])
                            .normalize();
                        (inst.inverse.matrix3.transpose() * n).normalize().as_vec3()
                            * inst.transform.matrix3.determinant().signum() as f32
                    }
                    BakePass::Albedo | BakePass::Emission => {
                        let mut value = if request.pass == BakePass::Albedo {
                            albedo(m, uv)
                        } else {
                            Vec3::from_array(m.emission)
                        };
                        let binding = m.pbr.as_ref().and_then(|p| {
                            if request.pass == BakePass::Albedo {
                                p.base_color.as_ref()
                            } else {
                                p.emission.as_ref()
                            }
                        });
                        if let Some(binding) = binding {
                            let (sample_uv, dx, dy) = if let Some(id) = binding.uv_attribute {
                                let slot = inst
                                    .geometry
                                    .uv_attributes
                                    .iter()
                                    .position(|v| *v == id)
                                    .ok_or_else(|| {
                                        Error::new(
                                            "reference",
                                            "bake material UV attribute missing",
                                        )
                                    })?;
                                let coords = tri.uv_sets[slot];
                                let e = coords[1] - coords[0];
                                let f = coords[2] - coords[0];
                                (
                                    coords[0] * w + coords[1] * u + coords[2] * v,
                                    (e * other.y - f * edge.y) / (det * request.width as f32),
                                    (f * edge.x - e * other.x) / (det * request.height as f32),
                                )
                            } else {
                                (
                                    uv,
                                    Vec2::new(1. / request.width as f32, 0.),
                                    Vec2::new(0., 1. / request.height as f32),
                                )
                            };
                            value *= scene.images[&(binding.image.clone(), binding.role)]
                                .sample(&binding.sampler, sample_uv, dx, dy)
                                .truncate();
                        }
                        value
                    }
                };
                rgb[index] = value.to_array();
                covered[index] = true;
            }
        }
    }
    if !covered.iter().any(|x| *x) {
        return Err(Error::new("bake_uv", "UV atlas covers no pixel centers"));
    }
    let output_digest = digest(&canonical(&(&rgb, &covered))?);
    Ok(Baked{width:request.width,height:request.height,pass:request.pass,rgb,covered,revision:revision.into(),request_digest:digest(&canonical(request)?),output_digest,approximation:"UV pixel-center raster; first-owner shared edges; uncovered mask; reject interior overlap; albedo/emission mip footprints and geometric world normals; no dilation or lightmap transport".into()})
}
#[derive(Clone, Debug)]
pub struct ViewOutput {
    pub name: String,
    pub raw: Image,
    pub display: Display,
    pub albedo: Vec<[f32; 3]>,
}
#[derive(Clone, Debug)]
pub struct Products {
    pub views: Vec<ViewOutput>,
    pub bakes: BTreeMap<String, Baked>,
}
pub fn render_products(
    snapshot: &Snapshot,
    revision: &str,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Products> {
    if snapshot.revision()? != revision {
        return Err(Error::new(
            "stale_revision",
            "render product revision changed",
        ));
    }
    let state = snapshot
        .imaging
        .as_ref()
        .ok_or_else(|| Error::new("imaging", "authored imaging policy missing"))?;
    state.validate(snapshot)?;
    let settings = snapshot.render_settings.as_ref().ok_or_else(|| {
        Error::new(
            "render_settings",
            "render products require authored settings",
        )
    })?;
    let bytes =
        state.views.len() as u64 * u64::from(settings.width) * u64::from(settings.height) * 128
            + state
                .bakes
                .iter()
                .map(|b| u64::from(b.width) * u64::from(b.height) * 16)
                .sum::<u64>();
    if bytes > settings.max_bytes {
        return Err(Error::new(
            "budget",
            "render product retention exceeds authored byte budget",
        ));
    }
    let mut views = vec![];
    let mut evaluator = Evaluator::default();
    for view in &state.views {
        if cancelled() {
            return Err(Error::new("cancelled", "render views cancelled"));
        }
        let mut s = snapshot.clone();
        if let Some(include) = &view.include {
            s.layers.push(Layer {
                name: format!("derived view {}", view.name),
                overrides: s
                    .entities
                    .iter()
                    .filter(|e| !include.contains(&e.id))
                    .map(|e| {
                        (
                            e.id,
                            Override {
                                transform: None,
                                material: None,
                                hidden: true,
                            },
                        )
                    })
                    .collect(),
            });
        }
        let scene = evaluator.evaluate_with_cancel(&s, &mut cancelled)?;
        let raw = render(&scene, settings, &mut cancelled)?;
        let display = state.pipeline.process(&raw, &mut cancelled)?;
        let mut albedos = vec![[0.; 3]; raw.linear.len()];
        for y in 0..settings.height {
            if cancelled() {
                return Err(Error::new("cancelled", "albedo pass cancelled"));
            }
            for x in 0..settings.width {
                let pixel = y * settings.width + x;
                let px = f64::from(x) + random(pixel, 0, 0, settings.seed);
                let py = f64::from(y) + random(pixel, 0, 1, settings.seed);
                let ray = settings
                    .camera
                    .ray(px, py, settings.width, settings.height)?;
                let (near, far) = settings.camera.clip(ray);
                if let Some(hit) = scene.intersect(ray, near, far) {
                    albedos[pixel as usize] =
                        if scene.instances[hit.instance].material.pbr.is_some() {
                            shading(
                                &scene,
                                &hit,
                                [
                                    settings.camera.ray(
                                        px + 1.,
                                        py,
                                        settings.width,
                                        settings.height,
                                    )?,
                                    settings.camera.ray(
                                        px,
                                        py + 1.,
                                        settings.width,
                                        settings.height,
                                    )?,
                                ],
                            )
                            .color
                            .to_array()
                        } else {
                            albedo(&scene.instances[hit.instance].material, hit.uv).to_array()
                        };
                }
            }
        }
        views.push(ViewOutput {
            name: view.name.clone(),
            raw,
            display,
            albedo: albedos,
        });
    }
    let scene = evaluator.evaluate_with_cancel(snapshot, &mut cancelled)?;
    let mut bakes = BTreeMap::new();
    for request in &state.bakes {
        bakes.insert(
            request.name.clone(),
            bake(&scene, request, revision, &mut cancelled)?,
        );
    }
    Ok(Products { views, bakes })
}
