//! CPU/WASM material extensions: ideal dielectric, conductor GGX and a
//! single-interface clearcoat approximation with explicit opacity semantics.
use crate::{Error, Result, canonical, digest, document::Material, render::*};
use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Model {
    Principled,
    Dielectric {
        ior: f64,
    },
    Conductor {
        eta: [f64; 3],
        k: [f64; 3],
    },
    Coated {
        weight: f64,
        ior: f64,
        roughness: f64,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Opacity {
    Opaque,
    Blend { factor: f64 },
    Mask { factor: f64, cutoff: f64 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    pub model: Model,
    pub opacity: Opacity,
}
impl Surface {
    pub fn validate(&self) -> Result<()> {
        match &self.model {
            Model::Principled => {}
            Model::Dielectric { ior } => {
                if !ior.is_finite() || !(1.0..=3.).contains(ior) {
                    return Err(Error::new("material", "dielectric IOR must be in [1,3]"));
                }
            }
            Model::Conductor { eta, k } => {
                if eta
                    .iter()
                    .chain(k)
                    .any(|x| !x.is_finite() || *x < 0. || *x > 100.)
                    || eta.iter().zip(k).any(|(e, k)| *e == 0. && *k == 0.)
                {
                    return Err(Error::new(
                        "material",
                        "invalid conductor optical constants",
                    ));
                }
            }
            Model::Coated {
                weight,
                ior,
                roughness,
            } => {
                if !weight.is_finite()
                    || !(0.0..=1.).contains(weight)
                    || !ior.is_finite()
                    || !(1.0..=3.).contains(ior)
                    || !roughness.is_finite()
                    || !(0.05..=1.).contains(roughness)
                {
                    return Err(Error::new("material", "invalid single-interface coat"));
                }
            }
        }
        let (factor, cutoff) = match self.opacity {
            Opacity::Opaque => (1., 0.),
            Opacity::Blend { factor } => (factor, 0.),
            Opacity::Mask { factor, cutoff } => (factor, cutoff),
        };
        if !factor.is_finite()
            || !(0.0..=1.).contains(&factor)
            || !cutoff.is_finite()
            || cutoff < 0.
        {
            return Err(Error::new("material", "invalid alpha coverage"));
        }
        Ok(())
    }
}
pub fn dielectric_fresnel(cosine: f64, eta_i: f64, eta_t: f64) -> f64 {
    if eta_i == eta_t {
        return 0.;
    }
    let c = cosine.abs().clamp(0., 1.);
    let sin_t2 = (eta_i / eta_t).powi(2) * (1. - c * c);
    if sin_t2 >= 1. {
        return 1.;
    }
    let ct = (1. - sin_t2).sqrt();
    let rs = (eta_i * c - eta_t * ct) / (eta_i * c + eta_t * ct);
    let rp = (eta_t * c - eta_i * ct) / (eta_t * c + eta_i * ct);
    0.5 * (rs * rs + rp * rp)
}
pub fn refract(incident: DVec3, normal: DVec3, eta: f64) -> Option<DVec3> {
    let c = (-incident).dot(normal);
    let sin2 = eta * eta * (1. - c * c).max(0.);
    if sin2 >= 1. {
        None
    } else {
        Some((eta * incident + (eta * c - (1. - sin2).sqrt()) * normal).normalize())
    }
}
fn ggx(f: DVec3, roughness: f64, n: DVec3, v: DVec3, l: DVec3) -> DVec3 {
    let nv = n.dot(v);
    let nl = n.dot(l);
    if nv <= 0. || nl <= 0. || (v + l).length_squared() < 1e-20 {
        return DVec3::ZERO;
    }
    let h = (v + l).normalize();
    let a2 = roughness.max(0.05).powi(4);
    let visibility =
        1. / ((nl + (a2 + (1. - a2) * nl * nl).sqrt()) * (nv + (a2 + (1. - a2) * nv * nv).sqrt()));
    f * crate::pbr::distribution(n.dot(h), roughness) * visibility
}
fn ggx_pdf(n: DVec3, v: DVec3, l: DVec3, roughness: f64) -> f64 {
    if n.dot(v) <= 0. || n.dot(l) <= 0. || (v + l).length_squared() < 1e-20 {
        return 0.;
    }
    let h = (v + l).normalize();
    crate::pbr::distribution(n.dot(h), roughness) * n.dot(h).max(0.) / (4. * v.dot(h).abs())
}
pub fn brdf(model: &Model, s: &Shading, n: DVec3, v: DVec3, l: DVec3) -> DVec3 {
    let h = if (v + l).length_squared() > 1e-20 {
        (v + l).normalize()
    } else {
        return DVec3::ZERO;
    };
    match model {
        Model::Dielectric { .. } => DVec3::ZERO,
        Model::Principled => crate::pbr::brdf(s.color, s.metallic, s.roughness, n, v, l),
        Model::Conductor { eta, k } => {
            let e = DVec3::from_array(*eta);
            let k = DVec3::from_array(*k);
            let f0 = ((e - DVec3::ONE) * (e - DVec3::ONE) + k * k)
                / ((e + DVec3::ONE) * (e + DVec3::ONE) + k * k);
            ggx(
                crate::pbr::fresnel(f0, v.dot(h)),
                f64::from(s.roughness),
                n,
                v,
                l,
            )
        }
        Model::Coated {
            weight,
            ior,
            roughness,
        } => {
            let fv = dielectric_fresnel(n.dot(v), 1., *ior);
            let fl = dielectric_fresnel(n.dot(l), 1., *ior);
            let fh = dielectric_fresnel(v.dot(h), 1., *ior);
            crate::pbr::brdf(s.color, s.metallic, s.roughness, n, v, l)
                * (1. - weight * fv)
                * (1. - weight * fl)
                + ggx(DVec3::splat(weight * fh), *roughness, n, v, l)
        }
    }
}
pub fn pdf(model: &Model, s: &Shading, n: DVec3, v: DVec3, l: DVec3) -> f64 {
    match model {
        Model::Dielectric { .. } => 0.,
        Model::Principled => crate::pbr::pdf(n, v, l, s.roughness),
        Model::Conductor { .. } => ggx_pdf(n, v, l, f64::from(s.roughness)),
        Model::Coated {
            weight, roughness, ..
        } => {
            let q = weight * 0.5;
            (1. - q) * crate::pbr::pdf(n, v, l, s.roughness) + q * ggx_pdf(n, v, l, *roughness)
        }
    }
}
pub fn sample_direction(model: &Model, s: &Shading, n: DVec3, v: DVec3, r: [f64; 4]) -> DVec3 {
    match model {
        Model::Conductor { .. } => crate::pbr::sample(n, v, s.roughness, 1., r[2], r[3]),
        Model::Coated {
            weight, roughness, ..
        } if r[0] < weight * 0.5 => crate::pbr::sample(n, v, *roughness as f32, 1., r[2], r[3]),
        _ => crate::pbr::sample(n, v, s.roughness, r[1], r[2], r[3]),
    }
}
fn opacity(scene: &Scene, hit: &Hit) -> f64 {
    let pbr = scene.instances[hit.instance].material.pbr.as_ref();
    let Some(surface) = pbr.and_then(|p| p.advanced.as_ref()) else {
        return 1.;
    };
    let alpha = pbr.and_then(|p| p.base_color.as_ref()).map_or(1., |b| {
        f64::from(
            scene.images[&(b.image.clone(), b.role)]
                .sample(
                    &b.sampler,
                    hit.uv_for(scene, b.uv_attribute),
                    glam::Vec2::ZERO,
                    glam::Vec2::ZERO,
                )
                .w,
        )
    });
    let alpha = alpha * f64::from(hit.color_rgba(scene).w);
    match surface.opacity {
        Opacity::Opaque => 1.,
        Opacity::Blend { factor } => alpha * factor,
        Opacity::Mask { factor, cutoff } => {
            if alpha * factor >= cutoff {
                1.
            } else {
                0.
            }
        }
    }
}
fn visibility(
    scene: &Scene,
    ray: Ray,
    far: f64,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<DVec3> {
    let mut transmission = 1.;
    let mut near = 1e-5;
    for _ in 0..64 {
        if cancelled() {
            return Err(Error::new("cancelled", "alpha shadow traversal cancelled"));
        }
        let Some(hit) = scene.intersect(ray, near, far) else {
            return Ok(scene.media.transmittance(ray, 0., far, &mut *cancelled)? * transmission);
        };
        transmission *= 1. - opacity(scene, &hit);
        if transmission <= 0. {
            return Ok(DVec3::ZERO);
        }
        near = hit.distance + 1e-5;
    }
    Err(Error::new("budget", "alpha shadow exceeded 64 surfaces"))
}
fn surface(
    scene: &Scene,
    hit: &Hit,
    settings: &Settings,
    px: f64,
    py: f64,
    primary: bool,
) -> Result<Shading> {
    let m = &scene.instances[hit.instance].material;
    if m.pbr.is_some() {
        if !primary {
            // This profile does not propagate ray differentials. Camera rays
            // describe primary hits only, including discarded-alpha continuation.
            return Ok(shading_uv(scene, hit, [glam::Vec2::ZERO; 2], None));
        }
        Ok(shading(
            scene,
            hit,
            [
                settings
                    .camera
                    .ray(px + 1., py, settings.width, settings.height)?,
                settings
                    .camera
                    .ray(px, py + 1., settings.width, settings.height)?,
            ],
        ))
    } else {
        Ok(Shading {
            color: albedo(m, hit.uv),
            emission: Vec3::from_array(m.emission),
            metallic: 0.,
            roughness: 1.,
            occlusion: 1.,
            normal: hit.normal,
        })
    }
}
fn model(material: &Material) -> Model {
    material
        .pbr
        .as_ref()
        .and_then(|p| p.advanced.as_ref())
        .map(|p| p.model.clone())
        .unwrap_or(Model::Principled)
}
/// Finite-depth reference integrator. Glass direct-light caustics are not sampled;
/// continuation rays can reach emission/environment. Alpha is stochastic coverage.
pub fn render(scene: &Scene, s: &Settings, mut cancelled: impl FnMut() -> bool) -> Result<Image> {
    scene.validate_geometry_bindings()?;
    s.validate()?;
    let count = (s.width * s.height) as usize;
    let mut linear = vec![[0.; 3]; count];
    let mut depth = vec![0.; count];
    let mut normals = vec![[0.; 3]; count];
    let mut objects = vec![None; count];
    for y in 0..s.height {
        if cancelled() {
            return Err(Error::new(
                "cancelled",
                "extended material render cancelled",
            ));
        }
        for x in 0..s.width {
            let pixel = y * s.width + x;
            let mut total = DVec3::ZERO;
            for sample in 0..s.samples {
                if cancelled() {
                    return Err(Error::new(
                        "cancelled",
                        "extended material sample cancelled",
                    ));
                }
                let px = f64::from(x) + random(pixel, sample, 0, s.seed);
                let py = f64::from(y) + random(pixel, sample, 1, s.seed);
                let mut ray = s.camera.ray(px, py, s.width, s.height)?;
                let primary_origin = ray.origin;
                let primary_clip = s.camera.clip(ray);
                let mut throughput = DVec3::ONE;
                let mut bounce = 0;
                let mut transparent = 0;
                while bounce < s.max_depth {
                    let (near, far) = if bounce == 0 {
                        // Alpha continuation still belongs to the original camera
                        // segment. Secondary scattering rays have their own extent.
                        let travelled = (ray.origin - primary_origin).dot(ray.direction);
                        (
                            if transparent == 0 {
                                primary_clip.0
                            } else {
                                (primary_clip.0 - travelled).max(1e-5)
                            },
                            primary_clip.1 - travelled,
                        )
                    } else {
                        (1e-5, f64::INFINITY)
                    };
                    // A discarded hit on the far plane may advance beyond the
                    // remaining segment. There is no more surface/media to visit.
                    if far < near {
                        total += throughput * DVec3::from_array(s.environment.map(f64::from));
                        break;
                    }
                    let hit = scene.intersect(ray, near, far);
                    let limit = hit.as_ref().map_or(far, |h| h.distance);
                    if !scene.media.is_empty() {
                        let volume = scene.media.transport(
                            ray,
                            near,
                            limit,
                            |p, mut cancel| {
                                let d = DVec3::from_array(s.light.position) - p;
                                let distance = d.length();
                                if distance < 1e-5 {
                                    return Ok((DVec3::ZERO, DVec3::Z));
                                }
                                let direction = d / distance;
                                let light = DVec3::from_array(s.light.intensity.map(f64::from))
                                    * visibility(
                                        scene,
                                        Ray {
                                            origin: p,
                                            direction,
                                        },
                                        distance,
                                        &mut cancel,
                                    )?
                                    / (distance * distance);
                                Ok((light, direction))
                            },
                            &mut cancelled,
                        )?;
                        total += throughput * volume.radiance;
                        throughput *= volume.transmittance;
                    }
                    let Some(hit) = hit else {
                        total += throughput * DVec3::from_array(s.environment.map(f64::from));
                        break;
                    };
                    let alpha = opacity(scene, &hit);
                    if random(pixel, sample, 1024 + bounce * 128 + transparent, s.seed) >= alpha {
                        transparent += 1;
                        if transparent > 64 {
                            return Err(Error::new(
                                "budget",
                                "alpha continuation exceeded 64 surfaces",
                            ));
                        }
                        ray.origin = hit.position + ray.direction * 1e-5;
                        continue;
                    }
                    let shading = surface(scene, &hit, s, px, py, bounce == 0)?;
                    let material = &scene.instances[hit.instance].material;
                    let model = model(material);
                    let n = shading.normal;
                    let view = -ray.direction;
                    if bounce == 0 && sample == 0 {
                        depth[pixel as usize] = hit.position.distance(primary_origin) as f32;
                        normals[pixel as usize] = n.as_vec3().to_array();
                        objects[pixel as usize] = Some(scene.instances[hit.instance].id);
                    }
                    total += throughput * shading.emission.as_dvec3();
                    let mut direction;
                    if let Model::Dielectric { ior } = model {
                        let eta = if hit.front_face { 1. / ior } else { ior };
                        let f = dielectric_fresnel(
                            view.dot(hit.geometric_normal),
                            if hit.front_face { 1. } else { ior },
                            if hit.front_face { ior } else { 1. },
                        );
                        if random(pixel, sample, 6 + bounce * 16, s.seed) < f {
                            direction = ray.direction
                                + 2. * view.dot(hit.geometric_normal) * hit.geometric_normal;
                        } else if let Some(refracted) =
                            refract(ray.direction, hit.geometric_normal, eta)
                        {
                            direction = refracted;
                            throughput *= shading.color.as_dvec3() * eta * eta;
                        } else {
                            direction = ray.direction
                                + 2. * view.dot(hit.geometric_normal) * hit.geometric_normal;
                        }
                    } else {
                        let delta = DVec3::from_array(s.light.position) - hit.position;
                        let distance = delta.length();
                        if distance > 1e-5 {
                            let l = delta / distance;
                            if n.dot(l) > 0. && hit.geometric_normal.dot(l) > 0. {
                                let visible = visibility(
                                    scene,
                                    Ray {
                                        origin: hit.position + hit.geometric_normal * 1e-5,
                                        direction: l,
                                    },
                                    (distance - 2e-5).max(1e-5),
                                    &mut cancelled,
                                )?;
                                let f = if material.pbr.is_none() {
                                    shading.color.as_dvec3() / std::f64::consts::PI
                                } else {
                                    brdf(&model, &shading, n, view, l)
                                };
                                total += throughput
                                    * f
                                    * n.dot(l)
                                    * visible
                                    * DVec3::from_array(s.light.intensity.map(f64::from))
                                    / (distance * distance);
                            }
                        }
                        let r = std::array::from_fn(|i| {
                            random(pixel, sample, 2 + bounce * 16 + i as u32, s.seed)
                        });
                        direction = if material.pbr.is_none() {
                            cosine_direction(n, r[2], r[3])
                        } else {
                            sample_direction(&model, &shading, n, view, r)
                        };
                        if direction.dot(hit.geometric_normal) <= 0. {
                            break;
                        }
                        let density = if material.pbr.is_none() {
                            n.dot(direction) / std::f64::consts::PI
                        } else {
                            pdf(&model, &shading, n, view, direction)
                        };
                        if density <= 0. {
                            break;
                        }
                        let f = if material.pbr.is_none() {
                            shading.color.as_dvec3() / std::f64::consts::PI
                        } else {
                            brdf(&model, &shading, n, view, direction)
                        };
                        throughput *= f * n.dot(direction) / density;
                    }
                    // Occlusion affects indirect transport, after direct light and emission.
                    throughput *= f64::from(shading.occlusion);
                    direction = direction.normalize();
                    ray = Ray {
                        origin: hit.position
                            + hit.geometric_normal
                                * (if direction.dot(hit.geometric_normal) > 0. {
                                    1e-5
                                } else {
                                    -1e-5
                                }),
                        direction,
                    };
                    bounce += 1;
                    transparent = 0;
                    if !throughput.is_finite() {
                        return Err(Error::new("numerics", "extended BSDF throughput overflow"));
                    }
                    if bounce == s.max_depth {
                        let escaped = visibility(scene, ray, f64::INFINITY, &mut cancelled)?;
                        total +=
                            throughput * escaped * DVec3::from_array(s.environment.map(f64::from));
                    }
                }
            }
            let p = (total / f64::from(s.samples)).as_vec3();
            if !p.is_finite() || p.min_element() < 0. {
                return Err(Error::new("numerics", "extended material output invalid"));
            }
            linear[pixel as usize] = p.to_array();
        }
    }
    let output_digest = digest(
        &linear
            .iter()
            .flatten()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    Ok(Image {
        width: s.width,
        height: s.height,
        linear,
        depth,
        normals,
        objects,
        receipt: RenderReceipt {
            revision: scene.revision.clone(),
            settings_digest: digest(&canonical(s)?),
            backend: "cpu-f64-extended-bsdf-v0".into(),
            samples: s.samples,
            seed: s.seed,
            color_space: "linear-sRGB".into(),
            approximation: format!(
                "finite depth {}; ideal dielectric radiance transport; conductor Schlick from eta/k; single-interface GGX clearcoat without inter-layer multiple scattering; stochastic alpha with base-level coverage textures; no sampled glass point-light caustics; geometric normals for transmission; sparse single scattering when present",
                s.max_depth
            ),
            width: s.width,
            height: s.height,
            output_digest,
        },
    })
}
