//! Authored lens semantics; projection is independent of renderer memory layout.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Lens {
    Perspective {
        vertical_fov_radians: f64,
        aspect_ratio: Option<f64>,
        near: f64,
        far: Option<f64>,
    },
    Orthographic {
        xmag: f64,
        ymag: f64,
        near: f64,
        far: f64,
    },
}
impl Lens {
    pub fn validate(&self) -> Result<()> {
        let valid = match *self {
            Self::Perspective {
                vertical_fov_radians,
                aspect_ratio,
                near,
                far,
            } => {
                vertical_fov_radians.is_finite()
                    && vertical_fov_radians > 0.
                    && vertical_fov_radians < std::f64::consts::PI
                    && aspect_ratio.is_none_or(|a| a.is_finite() && a > 0.)
                    && near.is_finite()
                    && near > 0.
                    && far.is_none_or(|f| f.is_finite() && f > near)
            }
            Self::Orthographic {
                xmag,
                ymag,
                near,
                far,
            } => {
                [xmag, ymag, near, far].iter().all(|n| n.is_finite())
                    && xmag > 0.
                    && ymag > 0.
                    && near >= 0.
                    && far > near
            }
        };
        if valid {
            Ok(())
        } else {
            Err(Error::new("camera", "invalid lens or clipping planes"))
        }
    }
}
