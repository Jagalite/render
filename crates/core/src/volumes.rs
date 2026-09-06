//! Sparse, piecewise-constant RGB media. Rust CPU/WASM reference transport.
//! Extinction and emission integrate analytically per cell interval; point-light
//! single scattering uses bounded midpoint quadrature, never a dense grid.
use crate::{
    Error, Id, Result, canonical, digest,
    render::{Bounds, Bvh, Ray},
};
use glam::{DAffine3, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub coordinate: [i32; 3],
    pub density: f64,
    pub emission: [f64; 3],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub origin: [f64; 3],
    pub voxel_size: [f64; 3],
    pub cells: Vec<Cell>,
    /// Coefficients per world meter at density one. Scaling geometry changes
    /// optical path length, not the coefficients.
    pub absorption: [f64; 3],
    pub scattering: [f64; 3],
    pub anisotropy: f64,
    pub max_step_meters: f64,
}
impl Asset {
    pub fn validate(&self) -> Result<()> {
        if self.cells.is_empty() || self.cells.len() > 16384 {
            return Err(Error::new(
                "budget",
                "sparse volume requires 1..16384 occupied cells",
            ));
        }
        if self.origin.iter().any(|x| !x.is_finite() || x.abs() > 1e9)
            || self
                .voxel_size
                .iter()
                .any(|x| !x.is_finite() || *x < 1e-6 || *x > 1e6)
            || self
                .absorption
                .iter()
                .chain(&self.scattering)
                .any(|x| !x.is_finite() || *x < 0. || *x > 1e6)
            || !self.anisotropy.is_finite()
            || self.anisotropy.abs() > 0.95
            || !self.max_step_meters.is_finite()
            || !(1e-5..=1e6).contains(&self.max_step_meters)
        {
            return Err(Error::new(
                "volume",
                "invalid metric grid, coefficients, anisotropy or quadrature policy",
            ));
        }
        let mut coords = BTreeSet::new();
        for c in &self.cells {
            if !coords.insert(c.coordinate) {
                return Err(Error::new(
                    "duplicate_id",
                    "duplicate sparse cell coordinate",
                ));
            }
            if c.coordinate.iter().any(|x| x.unsigned_abs() > 1000000)
                || !c.density.is_finite()
                || !(0.0..=1e6).contains(&c.density)
                || c.emission
                    .iter()
                    .any(|x| !x.is_finite() || *x < 0. || *x > 1e6)
            {
                return Err(Error::new("volume", "invalid occupied cell channel"));
            }
            if c.density == 0. && c.emission == [0.; 3] {
                return Err(Error::new("volume", "empty cells must remain absent"));
            }
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
}
#[derive(Clone, Debug)]
pub struct TransportCell {
    pub bounds: Bounds,
    pub inverse: DAffine3,
    pub extinction: DVec3,
    pub scattering: DVec3,
    pub emission: DVec3,
    g: f64,
    step: f64,
}
#[derive(Clone, Debug, Default)]
pub struct Media {
    cells: Vec<TransportCell>,
    bvh: Bvh,
}
#[derive(Clone, Debug)]
struct Segment {
    near: f64,
    far: f64,
    active: Vec<usize>,
    extinction: DVec3,
    emission: DVec3,
    step: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Transport {
    pub transmittance: DVec3,
    pub radiance: DVec3,
}
fn interval(b: Bounds, r: Ray, mut near: f64, mut far: f64) -> Option<(f64, f64)> {
    for axis in 0..3 {
        if r.direction[axis] == 0. {
            // Half-open voxel ownership prevents duplicate intervals on a face.
            if r.origin[axis] < b.min[axis] || r.origin[axis] >= b.max[axis] {
                return None;
            }
        } else {
            let a = (b.min[axis] - r.origin[axis]) / r.direction[axis];
            let z = (b.max[axis] - r.origin[axis]) / r.direction[axis];
            near = near.max(a.min(z));
            far = far.min(a.max(z));
            if near >= far {
                return None;
            }
        }
    }
    Some((near, far))
}
pub fn phase(cosine: f64, g: f64) -> f64 {
    (1. - g * g)
        / (4. * std::f64::consts::PI * (1. + g * g - 2. * g * cosine.clamp(-1., 1.)).powf(1.5))
}
fn decay(sigma: DVec3, length: f64) -> DVec3 {
    (-sigma * length).exp()
}
fn integral(sigma: DVec3, length: f64) -> DVec3 {
    DVec3::from_array(std::array::from_fn(|i| {
        if sigma[i] == 0. {
            length
        } else {
            -(-sigma[i] * length).exp_m1() / sigma[i]
        }
    }))
}
impl Media {
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    /// Read-only evaluated transport inputs; authoring and BVH remain private.
    pub fn transport_cells(&self) -> &[TransportCell] {
        &self.cells
    }
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
    pub fn build<'a>(
        assets: impl IntoIterator<Item = (Id, &'a Asset, DAffine3)>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Self> {
        let mut cells = Vec::new();
        let mut world_bounds = Vec::new();
        for (id, asset, world) in assets {
            asset.validate()?;
            if !world.is_finite() || world.matrix3.determinant().abs() < 1e-15 {
                return Err(Error::new("transform", "singular volume transform")
                    .with_context("entity", format!("{:032x}", id.0)));
            }
            if cells.len() + asset.cells.len() > 16384 {
                return Err(Error::new(
                    "budget",
                    "scene exceeds 16384 occupied volume cells",
                ));
            }
            let mut sorted: Vec<_> = asset.cells.iter().collect();
            sorted.sort_by_key(|c| c.coordinate);
            for c in sorted {
                if cancelled() {
                    return Err(Error::new(
                        "cancelled",
                        "sparse volume evaluation cancelled",
                    ));
                }
                let min = DVec3::from_array(asset.origin)
                    + DVec3::from_array(c.coordinate.map(f64::from))
                        * DVec3::from_array(asset.voxel_size);
                let bounds = Bounds {
                    min,
                    max: min + DVec3::from_array(asset.voxel_size),
                };
                let mut wb = Bounds {
                    min: DVec3::splat(f64::INFINITY),
                    max: DVec3::splat(f64::NEG_INFINITY),
                };
                for mask in 0..8 {
                    let p = world.transform_point3(DVec3::from_array(std::array::from_fn(|a| {
                        if mask & (1 << a) == 0 {
                            bounds.min[a]
                        } else {
                            bounds.max[a]
                        }
                    })));
                    wb.min = wb.min.min(p);
                    wb.max = wb.max.max(p);
                }
                world_bounds.push(wb);
                cells.push(TransportCell {
                    bounds,
                    inverse: world.inverse(),
                    extinction: (DVec3::from_array(asset.absorption)
                        + DVec3::from_array(asset.scattering))
                        * c.density,
                    scattering: DVec3::from_array(asset.scattering) * c.density,
                    emission: DVec3::from_array(c.emission),
                    g: asset.anisotropy,
                    step: asset.max_step_meters,
                });
            }
        }
        Ok(Self {
            cells,
            bvh: Bvh::build(&world_bounds),
        })
    }
    fn segments(
        &self,
        ray: Ray,
        near: f64,
        far: f64,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Vec<Segment>> {
        if !ray.origin.is_finite()
            || !ray.direction.is_finite()
            || (ray.direction.length_squared() - 1.).abs() > 1e-8
            || !near.is_finite()
            || near < 0.
            || far.is_nan()
            || far < near
        {
            return Err(Error::new(
                "ray",
                "media requires a finite unit world ray and ordered nonnegative bounds",
            ));
        }
        let mut events = Vec::new();
        for i in self.bvh.candidates(ray, near, far) {
            if cancelled() {
                return Err(Error::new("cancelled", "volume traversal cancelled"));
            }
            let c = &self.cells[i];
            let local = Ray {
                origin: c.inverse.transform_point3(ray.origin),
                direction: c.inverse.transform_vector3(ray.direction),
            };
            if let Some((a, b)) = interval(c.bounds, local, near, far) {
                events.push((a, true, i));
                events.push((b, false, i));
            }
        }
        events.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        let mut active = BTreeSet::new();
        let mut previous = near;
        let mut out = Vec::new();
        let mut entries = 0;
        for (at, enter, i) in events {
            if at > previous && !active.is_empty() {
                entries += active.len();
                if entries > 131072 {
                    return Err(Error::new(
                        "budget",
                        "overlapping medium interval budget exceeded",
                    ));
                }
                let mut extinction = DVec3::ZERO;
                let mut emission = DVec3::ZERO;
                let mut step = f64::INFINITY;
                for &j in &active {
                    let c: &TransportCell = &self.cells[j];
                    extinction += c.extinction;
                    emission += c.emission;
                    step = step.min(c.step);
                }
                out.push(Segment {
                    near: previous,
                    far: at,
                    active: active.iter().copied().collect(),
                    extinction,
                    emission,
                    step,
                });
            }
            if enter {
                active.insert(i);
            } else {
                active.remove(&i);
            }
            previous = at;
        }
        Ok(out)
    }
    pub fn transmittance(
        &self,
        ray: Ray,
        near: f64,
        far: f64,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<DVec3> {
        if cancelled() {
            return Err(Error::new("cancelled", "volume transmittance cancelled"));
        }
        let mut tau = DVec3::ZERO;
        for s in self.segments(ray, near, far, &mut cancelled)? {
            tau += s.extinction * (s.far - s.near);
        }
        Ok((-tau).exp())
    }
    /// Incoming light callback includes surface occlusion and medium attenuation
    /// from the sample to the point light. Its direction points toward the light.
    pub fn transport(
        &self,
        ray: Ray,
        near: f64,
        far: f64,
        mut incoming: impl FnMut(DVec3, &mut dyn FnMut() -> bool) -> Result<(DVec3, DVec3)>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Transport> {
        if cancelled() {
            return Err(Error::new("cancelled", "volume transport cancelled"));
        }
        let mut result = Transport {
            transmittance: DVec3::ONE,
            radiance: DVec3::ZERO,
        };
        let mut steps = 0u64;
        for s in self.segments(ray, near, far, &mut cancelled)? {
            let scattering = s
                .active
                .iter()
                .any(|&i| self.cells[i].scattering.max_element() > 0.);
            let n = if scattering {
                ((s.far - s.near) / s.step).ceil().max(1.) as u64
            } else {
                1
            };
            steps = steps.saturating_add(n);
            if steps > 65536 {
                return Err(Error::new(
                    "budget",
                    "volume quadrature exceeds 65536 steps per ray",
                ));
            }
            let ds = (s.far - s.near) / n as f64;
            for k in 0..n {
                if cancelled() {
                    return Err(Error::new("cancelled", "volume quadrature cancelled"));
                }
                let mut source = s.emission;
                if scattering {
                    let p = ray.origin + ray.direction * (s.near + (k as f64 + 0.5) * ds);
                    let (light, direction) = incoming(p, &mut cancelled)?;
                    for &i in &s.active {
                        let c = &self.cells[i];
                        source += c.scattering * light * phase(ray.direction.dot(direction), c.g);
                    }
                }
                result.radiance += result.transmittance * source * integral(s.extinction, ds);
                result.transmittance *= decay(s.extinction, ds);
            }
        }
        if !result.radiance.is_finite() || !result.transmittance.is_finite() {
            return Err(Error::new("numerics", "nonfinite volume transport"));
        }
        Ok(result)
    }
}
/// Asset receipt, kept separate from private BVH/cell layouts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub occupied_cells: usize,
    pub profile: String,
    pub assets: BTreeMap<Id, String>,
}
