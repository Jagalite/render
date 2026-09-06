//! Opaque single-scattering metallic/roughness model. No shader-language source.
use crate::{
    Error, Result,
    textures::{Binding, TextureRole},
};
use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced: Option<crate::scattering::Surface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub displacement: Option<crate::displacement::Displacement>,
    pub double_sided: bool,
    pub base_color: Option<Binding>,
    pub metallic_roughness: Option<Binding>,
    pub normal: Option<Binding>,
    pub emission: Option<Binding>,
    pub occlusion: Option<Binding>,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
}
impl Default for Surface {
    fn default() -> Self {
        Self {
            advanced: None,
            displacement: None,
            double_sided: false,
            base_color: None,
            metallic_roughness: None,
            normal: None,
            emission: None,
            occlusion: None,
            normal_scale: 1.,
            occlusion_strength: 1.,
        }
    }
}
impl Surface {
    pub fn bindings(&self) -> [Option<&Binding>; 6] {
        [
            self.base_color.as_ref(),
            self.metallic_roughness.as_ref(),
            self.normal.as_ref(),
            self.emission.as_ref(),
            self.occlusion.as_ref(),
            self.displacement.as_ref().and_then(|d| d.binding()),
        ]
    }
    pub fn validate(&self) -> Result<()> {
        if let Some(d) = &self.displacement {
            d.validate()?;
        }
        if let Some(advanced) = &self.advanced {
            advanced.validate()?;
        }
        if !self.normal_scale.is_finite()
            || self.normal_scale < 0.
            || !self.occlusion_strength.is_finite()
            || !(0.0..=1.).contains(&self.occlusion_strength)
        {
            return Err(Error::new(
                "material",
                "normal scale or occlusion strength outside range",
            ));
        }
        for (slot, b) in self.bindings().into_iter().enumerate() {
            if let Some(b) = b {
                let role = if slot == 0 || slot == 3 {
                    TextureRole::SrgbColor
                } else {
                    TextureRole::LinearData
                };
                if b.role != role {
                    return Err(Error::new(
                        "texture_role",
                        "color/data texture role mismatch",
                    ));
                }
            }
        }
        Ok(())
    }
}
pub const MIN_ROUGHNESS: f64 = 0.05;
pub fn fresnel(f0: DVec3, cosine: f64) -> DVec3 {
    f0 + (DVec3::ONE - f0) * (1. - cosine.clamp(0., 1.)).powi(5)
}
pub fn distribution(noh: f64, roughness: f64) -> f64 {
    let alpha = roughness.max(MIN_ROUGHNESS).powi(2);
    let a2 = alpha * alpha;
    let d = noh * noh * (a2 - 1.) + 1.;
    if noh <= 0. {
        0.
    } else {
        a2 / (std::f64::consts::PI * d * d)
    }
}
pub fn brdf(base: Vec3, metallic: f32, roughness: f32, n: DVec3, v: DVec3, l: DVec3) -> DVec3 {
    let nv = n.dot(v);
    let nl = n.dot(l);
    if nv <= 0. || nl <= 0. || (v + l).length_squared() < 1e-20 {
        return DVec3::ZERO;
    }
    let h = (v + l).normalize();
    let nh = n.dot(h).max(0.);
    let vh = v.dot(h).max(0.);
    let color = base.as_dvec3();
    let metal = f64::from(metallic);
    let f0 = DVec3::splat(0.04).lerp(color, metal);
    let f = fresnel(f0, vh);
    let a2 = f64::from(roughness).max(MIN_ROUGHNESS).powi(4);
    let visibility =
        1. / ((nl + (a2 + (1. - a2) * nl * nl).sqrt()) * (nv + (a2 + (1. - a2) * nv * nv).sqrt()));
    (DVec3::ONE - f) * color * ((1. - metal) / std::f64::consts::PI)
        + f * distribution(nh, f64::from(roughness)) * visibility
}
pub fn pdf(n: DVec3, v: DVec3, l: DVec3, roughness: f32) -> f64 {
    if n.dot(l) <= 0. || n.dot(v) <= 0. || (v + l).length_squared() < 1e-20 {
        return 0.;
    }
    let h = (v + l).normalize();
    let vh = v.dot(h);
    if vh <= 0. {
        return 0.;
    }
    0.5 * n.dot(l) / std::f64::consts::PI
        + 0.5 * distribution(n.dot(h), f64::from(roughness)) * n.dot(h).max(0.) / (4. * vh)
}
pub fn sample(n: DVec3, v: DVec3, roughness: f32, selector: f64, u: f64, w: f64) -> DVec3 {
    if selector < 0.5 {
        return crate::render::cosine_direction(n, u, w);
    }
    let alpha = f64::from(roughness).max(MIN_ROUGHNESS).powi(2);
    let cos = ((1. - u) / (1. + (alpha * alpha - 1.) * u)).sqrt();
    let sin = (1. - cos * cos).max(0.).sqrt();
    let phi = std::f64::consts::TAU * w;
    let helper = if n.z.abs() < 0.999 {
        DVec3::Z
    } else {
        DVec3::X
    };
    let t = helper.cross(n).normalize();
    let b = n.cross(t);
    let h = (t * (sin * phi.cos()) + b * (sin * phi.sin()) + n * cos).normalize();
    (2. * v.dot(h) * h - v).normalize()
}
