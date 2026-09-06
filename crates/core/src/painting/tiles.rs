use super::{TILE_PIXELS, TILE_SIDE, valid_hash};
use crate::{Error, Id, Result, canonical, digest, textures::TextureRole};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

// Explicit little-endian wire values, independent of Rust layout and host endianness.
fn encode(words: impl Iterator<Item = u16>) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut s = String::new();
    for word in words {
        for byte in word.to_le_bytes() {
            s.push(HEX[(byte >> 4) as usize] as char);
            s.push(HEX[(byte & 15) as usize] as char);
        }
    }
    s
}
fn decode(s: &str, words: usize) -> std::result::Result<Vec<u16>, &'static str> {
    if s.len() != words * 4
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("paint tile requires exact-length lowercase little-endian hexadecimal");
    }
    fn nibble(b: u8) -> u8 {
        if b <= b'9' { b - b'0' } else { b - b'a' + 10 }
    }
    Ok(s.as_bytes()
        .chunks_exact(4)
        .map(|b| {
            u16::from_le_bytes([
                nibble(b[0]) * 16 + nibble(b[1]),
                nibble(b[2]) * 16 + nibble(b[3]),
            ])
        })
        .collect())
}
mod rgba {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        v: &[[u16; 4]],
        s: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&encode(v.iter().flat_map(|p| p.iter().copied())))
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> std::result::Result<Vec<[u16; 4]>, D::Error> {
        let s = String::deserialize(d)?;
        let words = decode(&s, TILE_PIXELS * 4).map_err(serde::de::Error::custom)?;
        Ok(words
            .chunks_exact(4)
            .map(|p| [p[0], p[1], p[2], p[3]])
            .collect())
    }
}
mod mask {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        v: &[u16],
        s: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&encode(v.iter().copied()))
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> std::result::Result<Vec<u16>, D::Error> {
        decode(&String::deserialize(d)?, TILE_PIXELS).map_err(serde::de::Error::custom)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Tile {
    Color {
        version: u32,
        #[serde(with = "rgba")]
        rgba_le: Vec<[u16; 4]>,
    },
    Mask {
        version: u32,
        #[serde(with = "mask")]
        coverage_le: Vec<u16>,
    },
}
impl Tile {
    pub fn transparent() -> Self {
        Self::Color {
            version: 0,
            rgba_le: vec![[0; 4]; TILE_PIXELS],
        }
    }
    pub fn full_mask() -> Self {
        Self::Mask {
            version: 0,
            coverage_le: vec![65535; TILE_PIXELS],
        }
    }
    pub fn validate(&self) -> Result<()> {
        let (version, len, valid) = match self {
            Self::Color { version, rgba_le } => (
                *version,
                rgba_le.len(),
                rgba_le.iter().all(|p| p[..3].iter().all(|&v| v <= p[3])),
            ),
            Self::Mask {
                version,
                coverage_le,
            } => (*version, coverage_le.len(), true),
        };
        if version != 0 {
            return Err(Error::new(
                "schema_version",
                "unsupported paint tile version",
            ));
        }
        if len != TILE_PIXELS || !valid {
            return Err(Error::new(
                "paint_tile",
                "tile length or premultiplied channels invalid",
            ));
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
    pub fn decoded_bytes(&self) -> u64 {
        match self {
            Self::Color { .. } => 8192,
            Self::Mask { .. } => 2048,
        }
    }
    pub fn is_default(&self) -> bool {
        match self {
            Self::Color { rgba_le, .. } => rgba_le.iter().all(|p| *p == [0; 4]),
            Self::Mask { coverage_le, .. } => coverage_le.iter().all(|v| *v == 65535),
        }
    }
    fn validate_placement(
        &self,
        coordinate: Coordinate,
        width: u32,
        height: u32,
        mask: bool,
    ) -> Result<()> {
        self.validate()?;
        if matches!(self, Self::Mask { .. }) != mask {
            return Err(Error::new(
                "paint_tile",
                "tile kind does not match layer channel",
            ));
        }
        for i in 0..TILE_PIXELS {
            let x = coordinate.x * TILE_SIDE + i as u32 % TILE_SIDE;
            let y = coordinate.y * TILE_SIDE + i as u32 / TILE_SIDE;
            if x >= width || y >= height {
                let default = match self {
                    Self::Color { rgba_le, .. } => rgba_le[i] == [0; 4],
                    Self::Mask { coverage_le, .. } => coverage_le[i] == 65535,
                };
                if !default {
                    return Err(Error::new(
                        "paint_tile",
                        "out-of-canvas tile padding must equal channel default",
                    ));
                }
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coordinate {
    pub x: u32,
    pub y: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileRef {
    pub coordinate: Coordinate,
    pub tile: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<TileRef>", into = "Vec<TileRef>")]
pub struct TileMap(pub BTreeMap<Coordinate, String>);
impl TryFrom<Vec<TileRef>> for TileMap {
    type Error = &'static str;
    fn try_from(refs: Vec<TileRef>) -> std::result::Result<Self, Self::Error> {
        if refs.len() > 256 {
            return Err("layer tile references exceed profile");
        }
        let mut map = BTreeMap::new();
        for r in refs {
            if map.insert(r.coordinate, r.tile).is_some() {
                return Err("duplicate paint tile coordinate");
            }
        }
        Ok(Self(map))
    }
}
impl From<TileMap> for Vec<TileRef> {
    fn from(value: TileMap) -> Self {
        value
            .0
            .into_iter()
            .map(|(coordinate, tile)| TileRef { coordinate, tile })
            .collect()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub id: Id,
    pub name: String,
    pub visible: bool,
    pub opacity: u16,
    pub color: TileMap,
    pub mask: TileMap,
}
impl Layer {
    pub fn empty(id: Id, name: String) -> Self {
        Self {
            id,
            name,
            visible: true,
            opacity: 65535,
            color: TileMap::default(),
            mask: TileMap::default(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub role: TextureRole,
    /// Bottom to top. Reordering explicitly changes compositing.
    pub layers: Vec<Layer>,
    pub strokes: Vec<super::Stroke>,
}
impl Asset {
    pub fn validate(&self) -> Result<()> {
        if self.version != 0 {
            return Err(Error::new(
                "schema_version",
                "unsupported paint asset version",
            ));
        }
        if self.width == 0
            || self.height == 0
            || self.width > 2048
            || self.height > 2048
            || self.layers.is_empty()
            || self.layers.len() > 8
        {
            return Err(Error::new(
                "budget",
                "paint dimensions or layer count exceed profile",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut counts = [0usize; 2];
        for layer in &self.layers {
            if !ids.insert(layer.id) || layer.name.is_empty() || layer.name.len() > 128 {
                return Err(Error::new(
                    "paint_layer",
                    "duplicate layer ID or invalid name",
                ));
            }
            for (kind, tiles) in [&layer.color, &layer.mask].into_iter().enumerate() {
                counts[kind] += tiles.0.len();
                for (c, hash) in &tiles.0 {
                    if c.x >= self.width.div_ceil(TILE_SIDE)
                        || c.y >= self.height.div_ceil(TILE_SIDE)
                        || !valid_hash(hash)
                    {
                        return Err(Error::new(
                            "paint_tile",
                            "tile coordinate or content hash invalid",
                        ));
                    }
                }
            }
        }
        if self.strokes.len() > 16 {
            return Err(Error::new(
                "budget",
                "canvas retains at most 16 stroke recipes",
            ));
        }
        let mut stroke_ids = BTreeSet::new();
        for stroke in &self.strokes {
            stroke.validate()?;
            if !ids.contains(&stroke.layer) || !stroke_ids.insert(stroke.id) {
                return Err(Error::new(
                    "paint_stroke",
                    "missing layer or duplicate stroke ID",
                ));
            }
        }
        if counts.into_iter().any(|n| n > 256) {
            return Err(Error::new(
                "budget",
                "canvas exceeds 256 references per tile kind",
            ));
        }
        Ok(())
    }
    pub fn validate_tiles(&self, tiles: &BTreeMap<String, Arc<Tile>>) -> Result<()> {
        self.validate()?;
        for layer in &self.layers {
            for (mask, refs) in [(false, &layer.color), (true, &layer.mask)] {
                for (&c, hash) in &refs.0 {
                    let tile = tiles
                        .get(hash)
                        .ok_or_else(|| Error::new("reference", "paint tile missing"))?;
                    if tile.content_id()? != *hash {
                        return Err(Error::new("integrity", "paint tile hash mismatch"));
                    }
                    tile.validate_placement(c, self.width, self.height, mask)?;
                }
            }
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
}
