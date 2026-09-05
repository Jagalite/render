//! Rust-owned baseline image interchange. PPM is display sRGB; PFM output is linear.
use crate::{Error, Result, document::Texture};
pub fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
pub fn read_ppm_srgb(bytes: &[u8]) -> Result<Texture> {
    let mut offset = 0;
    fn token<'a>(bytes: &'a [u8], offset: &mut usize) -> Result<&'a [u8]> {
        loop {
            while bytes.get(*offset).is_some_and(u8::is_ascii_whitespace) {
                *offset += 1;
            }
            if bytes.get(*offset) == Some(&b'#') {
                while bytes.get(*offset).is_some_and(|b| *b != b'\n') {
                    *offset += 1;
                }
            } else {
                break;
            }
        }
        let start = *offset;
        while bytes
            .get(*offset)
            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'#')
        {
            *offset += 1;
        }
        if *offset == start {
            return Err(Error::new("ppm", "missing header token"));
        }
        Ok(&bytes[start..*offset])
    }
    if token(bytes, &mut offset)? != b"P6" {
        return Err(Error::new("ppm", "only binary P6 RGB is supported"));
    }
    fn number(bytes: &[u8]) -> Result<u32> {
        std::str::from_utf8(bytes)
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| Error::new("ppm", "invalid integer header"))
    }
    let width = number(token(bytes, &mut offset)?)?;
    let height = number(token(bytes, &mut offset)?)?;
    let max = number(token(bytes, &mut offset)?)?;
    if width == 0
        || height == 0
        || max != 255
        || u64::from(width) * u64::from(height) > 16 * 1024 * 1024
    {
        return Err(Error::new(
            "ppm",
            "invalid dimensions or unsupported max value",
        ));
    }
    let delimiter = *bytes
        .get(offset)
        .ok_or_else(|| Error::new("ppm", "missing data delimiter"))?;
    if !delimiter.is_ascii_whitespace() {
        return Err(Error::new("ppm", "missing whitespace delimiter"));
    }
    offset += 1;
    if delimiter == b'\r' && bytes.get(offset) == Some(&b'\n') {
        offset += 1;
    }
    if bytes.len() - offset != (u64::from(width) * u64::from(height) * 3) as usize {
        return Err(Error::new("ppm", "pixel payload length mismatch"));
    }
    let linear_rgb = bytes[offset..]
        .chunks_exact(3)
        .map(|p| {
            [
                srgb_to_linear(p[0] as f32 / 255.),
                srgb_to_linear(p[1] as f32 / 255.),
                srgb_to_linear(p[2] as f32 / 255.),
            ]
        })
        .collect();
    let texture = Texture {
        width,
        height,
        linear_rgb,
    };
    texture.validate()?;
    Ok(texture)
}
