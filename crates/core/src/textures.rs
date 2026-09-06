//! Typed encoded assets and Rust-owned color conversion/filtering.
use crate::{Error, Result, canonical, digest};
use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 2048;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mime {
    Png,
    Jpeg,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageAsset {
    pub mime: Mime,
    pub width: u32,
    pub height: u32,
    pub encoded: Vec<u8>,
}
impl ImageAsset {
    pub fn from_encoded(mime: Mime, encoded: Vec<u8>) -> Result<Self> {
        let mut asset = Self {
            mime,
            width: 1,
            height: 1,
            encoded,
        };
        let decoded = asset.decode_inner()?;
        asset.width = decoded.width();
        asset.height = decoded.height();
        Ok(asset)
    }
    fn decode_inner(&self) -> Result<image::RgbaImage> {
        if self.encoded.is_empty() || self.encoded.len() > MAX_IMAGE_BYTES {
            return Err(Error::new("budget", "encoded image exceeds 4 MiB"));
        }
        let expected = match self.mime {
            Mime::Png => image::ImageFormat::Png,
            Mime::Jpeg => image::ImageFormat::Jpeg,
        };
        if image::guess_format(&self.encoded).map_err(|e| Error::new("image", e.to_string()))?
            != expected
        {
            return Err(Error::new("image", "image MIME/signature mismatch"));
        }
        let mut reader = image::ImageReader::with_format(Cursor::new(&self.encoded), expected);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_DIMENSION);
        limits.max_image_height = Some(MAX_DIMENSION);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader
            .decode()
            .map_err(|e| Error::new("image", e.to_string()))?;
        if !matches!(
            decoded.color(),
            image::ColorType::L8
                | image::ColorType::La8
                | image::ColorType::Rgb8
                | image::ColorType::Rgba8
        ) {
            return Err(Error::new(
                "unsupported_image",
                "profile requires 8-bit PNG/JPEG samples",
            ));
        }
        Ok(decoded.into_rgba8())
    }
    pub fn decode(&self) -> Result<image::RgbaImage> {
        let image = self.decode_inner()?;
        if image.width() != self.width || image.height() != self.height {
            return Err(Error::new(
                "image",
                "declared image dimensions disagree with encoded data",
            ));
        }
        Ok(image)
    }
    pub fn validate(&self) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || self.width > MAX_DIMENSION
            || self.height > MAX_DIMENSION
            || self.encoded.is_empty()
            || self.encoded.len() > MAX_IMAGE_BYTES
        {
            return Err(Error::new(
                "budget",
                "image dimensions or bytes outside profile",
            ));
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureRole {
    SrgbColor,
    LinearData,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wrap {
    Repeat,
    Clamp,
    Mirror,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    Nearest,
    Linear,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinFilter {
    Nearest,
    Linear,
    NearestMipNearest,
    LinearMipNearest,
    NearestMipLinear,
    LinearMipLinear,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sampler {
    pub wrap_s: Wrap,
    pub wrap_t: Wrap,
    pub mag: Filter,
    pub min: MinFilter,
}
impl Default for Sampler {
    fn default() -> Self {
        Self {
            wrap_s: Wrap::Repeat,
            wrap_t: Wrap::Repeat,
            mag: Filter::Linear,
            min: MinFilter::LinearMipLinear,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub image: String,
    pub role: TextureRole,
    pub sampler: Sampler,
}
#[derive(Debug, Clone)]
pub struct Level {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<[f32; 4]>,
}
#[derive(Debug, Clone)]
pub struct Pyramid {
    pub levels: Vec<Level>,
}
impl Pyramid {
    pub fn new(image: &image::RgbaImage, role: TextureRole) -> Self {
        let rgba = image
            .pixels()
            .map(|pixel| {
                let mut p = pixel.0.map(|v| f32::from(v) / 255.);
                if role == TextureRole::SrgbColor {
                    for c in &mut p[..3] {
                        *c = crate::imaging::srgb_to_linear(*c);
                    }
                }
                p
            })
            .collect();
        let mut levels = vec![Level {
            width: image.width(),
            height: image.height(),
            rgba,
        }];
        while levels.last().expect("base level").width > 1
            || levels.last().expect("base level").height > 1
        {
            let prev = levels.last().expect("base level");
            let width = (prev.width / 2).max(1);
            let height = (prev.height / 2).max(1);
            let mut rgba = vec![];
            for y in 0..height {
                for x in 0..width {
                    // Area partition also includes the last row/column of NPOT images.
                    let (x0, x1) = (x * prev.width / width, (x + 1) * prev.width / width);
                    let (y0, y1) = (y * prev.height / height, (y + 1) * prev.height / height);
                    let mut sum = Vec4::ZERO;
                    for iy in y0..y1 {
                        for ix in x0..x1 {
                            sum += Vec4::from_array(prev.rgba[(iy * prev.width + ix) as usize]);
                        }
                    }
                    rgba.push((sum / ((x1 - x0) * (y1 - y0)) as f32).to_array());
                }
            }
            levels.push(Level {
                width,
                height,
                rgba,
            });
        }
        Self { levels }
    }
    pub fn sample(&self, sampler: &Sampler, uv: Vec2, dx: Vec2, dy: Vec2) -> Vec4 {
        let base = &self.levels[0];
        let dims = Vec2::new(base.width as f32, base.height as f32);
        let lod = (dx * dims)
            .length()
            .max((dy * dims).length())
            .max(1e-8)
            .log2();
        if lod <= 0. {
            return self.at_level(0, sampler, uv, sampler.mag);
        }
        let (filter, mip, blend) = match sampler.min {
            MinFilter::Nearest => (Filter::Nearest, false, false),
            MinFilter::Linear => (Filter::Linear, false, false),
            MinFilter::NearestMipNearest => (Filter::Nearest, true, false),
            MinFilter::LinearMipNearest => (Filter::Linear, true, false),
            MinFilter::NearestMipLinear => (Filter::Nearest, true, true),
            MinFilter::LinearMipLinear => (Filter::Linear, true, true),
        };
        let lod = if mip {
            lod.clamp(0., (self.levels.len() - 1) as f32)
        } else {
            0.
        };
        if !blend {
            return self.at_level((lod + 0.5).floor() as usize, sampler, uv, filter);
        }
        let first = lod.floor() as usize;
        self.at_level(first, sampler, uv, filter).lerp(
            self.at_level((first + 1).min(self.levels.len() - 1), sampler, uv, filter),
            lod.fract(),
        )
    }
    fn at_level(&self, level: usize, sampler: &Sampler, uv: Vec2, filter: Filter) -> Vec4 {
        let image = &self.levels[level];
        fn address(n: i64, size: u32, wrap: Wrap) -> usize {
            let size = i64::from(size);
            (match wrap {
                Wrap::Repeat => n.rem_euclid(size),
                Wrap::Clamp => n.clamp(0, size - 1),
                Wrap::Mirror => {
                    let n = n.rem_euclid(2 * size);
                    if n < size { n } else { 2 * size - 1 - n }
                }
            }) as usize
        }
        let pixel = |x, y| {
            Vec4::from_array(
                image.rgba[address(y, image.height, sampler.wrap_t) * image.width as usize
                    + address(x, image.width, sampler.wrap_s)],
            )
        };
        // Reduce first: finite but huge authored UVs must not overflow integer addressing.
        let coordinate = |v: f32, wrap| match wrap {
            Wrap::Repeat => v.rem_euclid(1.),
            Wrap::Clamp => v.clamp(0., 1.),
            Wrap::Mirror => v.rem_euclid(2.),
        };
        let x = coordinate(uv.x, sampler.wrap_s) * image.width as f32;
        let y = coordinate(uv.y, sampler.wrap_t) * image.height as f32;
        if filter == Filter::Nearest {
            return pixel(x.floor() as i64, y.floor() as i64);
        }
        let x = x - 0.5;
        let y = y - 0.5;
        let ix = x.floor() as i64;
        let iy = y.floor() as i64;
        pixel(ix, iy).lerp(pixel(ix + 1, iy), x - x.floor()).lerp(
            pixel(ix, iy + 1).lerp(pixel(ix + 1, iy + 1), x - x.floor()),
            y - y.floor(),
        )
    }
}
