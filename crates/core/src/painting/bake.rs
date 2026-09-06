use super::{Asset, Coordinate, TILE_SIDE, Tile, check_cancel, mul16};
use crate::{
    Error, Result,
    textures::{ImageAsset, Mime, TextureRole},
};
use image::ImageEncoder;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeBudget {
    pub max_pixel_work: u64,
    pub max_encoded_bytes: u64,
}
impl Default for BakeBudget {
    fn default() -> Self {
        Self {
            max_pixel_work: 16_777_216,
            max_encoded_bytes: 4 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeReport {
    pub source: String,
    pub image: String,
    pub role: TextureRole,
    pub pixel_work: u64,
    pub rgba8_bytes: u64,
    pub encoded_bytes: u64,
    pub max_premultiplied_quantization_error: f64,
}
pub struct Baked {
    pub image: ImageAsset,
    pub report: BakeReport,
}
fn to_srgb(x: f64) -> f64 {
    if x <= 0.0031308 {
        x * 12.92
    } else {
        1.055 * libm::pow(x, 1. / 2.4) - 0.055
    }
}
fn from_srgb(x: f64) -> f64 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        libm::pow((x + 0.055) / 1.055, 2.4)
    }
}
fn byte(x: f64) -> u8 {
    (x.clamp(0., 1.) * 255. + 0.5).floor() as u8
}
pub fn bake(
    source: &Asset,
    tiles: &BTreeMap<String, Arc<Tile>>,
    budget: &BakeBudget,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Baked> {
    check_cancel(&mut cancelled)?;
    source.validate_tiles(tiles)?;
    if budget.max_pixel_work == 0
        || budget.max_pixel_work > 16_777_216
        || budget.max_encoded_bytes == 0
        || budget.max_encoded_bytes > 4 * 1024 * 1024
    {
        return Err(Error::new("budget", "paint bake budget exceeds profile"));
    }
    let work = u64::from(source.width) * u64::from(source.height) * source.layers.len() as u64;
    if work > budget.max_pixel_work {
        return Err(Error::new("budget", "paint bake pixel work exceeded"));
    }
    check_cancel(&mut cancelled)?;
    let mut rgba = Vec::with_capacity(source.width as usize * source.height as usize * 4);
    let mut error = 0.0_f64;
    for y in 0..source.height {
        check_cancel(&mut cancelled)?;
        for x in 0..source.width {
            let c = Coordinate {
                x: x / TILE_SIDE,
                y: y / TILE_SIDE,
            };
            let index = ((y % TILE_SIDE) * TILE_SIDE + x % TILE_SIDE) as usize;
            let mut composite = [0u16; 4];
            for layer in &source.layers {
                if !layer.visible {
                    continue;
                }
                let Some(hash) = layer.color.0.get(&c) else {
                    continue;
                };
                let Tile::Color { rgba_le, .. } = tiles[hash].as_ref() else {
                    unreachable!("validated color")
                };
                let coverage = if let Some(hash) = layer.mask.0.get(&c) {
                    let Tile::Mask { coverage_le, .. } = tiles[hash].as_ref() else {
                        unreachable!("validated mask")
                    };
                    coverage_le[index]
                } else {
                    65535
                };
                let factor = mul16(coverage, layer.opacity);
                let src = rgba_le[index].map(|v| mul16(v, factor));
                for (d, s) in composite.iter_mut().zip(src) {
                    *d = s + mul16(*d, 65535 - src[3]);
                }
            }
            let alpha = byte(f64::from(composite[3]) / 65535.);
            let mut encoded = [0, 0, 0, alpha];
            if alpha != 0 {
                for k in 0..3 {
                    let straight = f64::from(composite[k]) / f64::from(composite[3]);
                    encoded[k] = byte(if source.role == TextureRole::SrgbColor {
                        to_srgb(straight)
                    } else {
                        straight
                    });
                }
            }
            for k in 0..4 {
                let value = f64::from(encoded[k]) / 255.;
                let reconstructed = if k == 3 {
                    value
                } else {
                    (if source.role == TextureRole::SrgbColor {
                        from_srgb(value)
                    } else {
                        value
                    }) * f64::from(alpha)
                        / 255.
                };
                error = error.max((reconstructed - f64::from(composite[k]) / 65535.).abs());
            }
            rgba.extend_from_slice(&encoded);
        }
    }
    check_cancel(&mut cancelled)?;
    let mut encoded = Vec::new();
    image::codecs::png::PngEncoder::new(&mut encoded)
        .write_image(
            &rgba,
            source.width,
            source.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| Error::new("image", e.to_string()))?;
    if encoded.len() as u64 > budget.max_encoded_bytes {
        return Err(Error::new(
            "budget",
            "paint PNG encoded byte budget exceeded",
        ));
    }
    check_cancel(&mut cancelled)?;
    let image = ImageAsset::from_encoded(Mime::Png, encoded)?;
    let report = BakeReport {
        source: source.content_id()?,
        image: image.content_id()?,
        role: source.role,
        pixel_work: work,
        rgba8_bytes: rgba.len() as u64,
        encoded_bytes: image.encoded.len() as u64,
        max_premultiplied_quantization_error: error,
    };
    check_cancel(&mut cancelled)?;
    Ok(Baked { image, report })
}
