//! Canonical sparse displacement payloads; no runtime indices or cached geometry.
use crate::{Error, Result, canonical, digest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROFILE: &str = "sparse-displacement-v1";
pub const BLOCK_LEN: usize = 64;
pub const MAX_CHUNKS: usize = 1024;
pub const MAX_ASSETS: usize = 64;
pub const MAX_BINDINGS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockId(pub u64);
impl Serialize for BlockId {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for BlockId {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        let parsed = value.parse::<u64>().map_err(serde::de::Error::custom)?;
        if parsed.to_string() != value || parsed > (u64::MAX >> 6) {
            return Err(serde::de::Error::custom(
                "displacement block requires canonical decimal 0..288230376151711743",
            ));
        }
        Ok(Self(parsed))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub slot: u8,
    pub delta_meters: [f32; 3],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chunk {
    pub version: u32,
    pub base_mesh: String,
    pub block: BlockId,
    pub points: Vec<Entry>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub version: u32,
    pub base_mesh: String,
    pub blocks: BTreeMap<BlockId, String>,
}
pub(crate) fn hash(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn metric(delta: [f32; 3]) -> Result<()> {
    if delta.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
        return Err(Error::new(
            "sculpt_metric",
            "displacement must be finite and at most 1e6 local meters",
        ));
    }
    Ok(())
}
impl Chunk {
    pub fn validate(&self) -> Result<()> {
        if self.version != 0
            || !hash(&self.base_mesh)
            || self.block.0 > (u64::MAX >> 6)
            || self.points.is_empty()
            || self.points.len() > BLOCK_LEN
        {
            return Err(Error::new(
                "sculpt_chunk",
                "invalid version, mesh, block or sparse point count",
            ));
        }
        let mut previous = None;
        for point in &self.points {
            metric(point.delta_meters)?;
            if point.slot >= 64
                || previous.is_some_and(|slot| slot >= point.slot)
                || (self.block.0 == 0 && point.slot == 0)
                || point.delta_meters == [0.; 3]
            {
                return Err(Error::new(
                    "sculpt_chunk",
                    "entries require increasing valid positive point identities and nonzero displacement",
                ));
            }
            previous = Some(point.slot);
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
    pub fn delta(&self, slot: u8) -> [f32; 3] {
        self.points
            .binary_search_by_key(&slot, |p| p.slot)
            .map(|i| self.points[i].delta_meters)
            .unwrap_or([0.; 3])
    }
}
impl Asset {
    pub fn validate(&self) -> Result<()> {
        if self.version != 0
            || !hash(&self.base_mesh)
            || self.blocks.len() > MAX_CHUNKS
            || self
                .blocks
                .iter()
                .any(|(block, key)| block.0 > (u64::MAX >> 6) || !hash(key))
        {
            return Err(Error::new(
                "sculpt_asset",
                "invalid version, source mesh or sparse blocks",
            ));
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub max_point_updates: u64,
    pub max_changed_chunks: u64,
    pub max_chunk_bytes: u64,
    pub max_basis_bytes: u64,
    pub max_triangle_updates: u64,
    pub max_refit_nodes: u64,
    pub max_copy_bytes: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            max_point_updates: 4096,
            max_changed_chunks: 64,
            max_chunk_bytes: 1 << 20,
            max_basis_bytes: 64 << 20,
            max_triangle_updates: 131072,
            max_refit_nodes: 262144,
            max_copy_bytes: 64 << 20,
        }
    }
}
impl Budget {
    pub fn validate(&self) -> Result<()> {
        let cap = Self::default();
        if [
            (self.max_point_updates, cap.max_point_updates),
            (self.max_changed_chunks, cap.max_changed_chunks),
            (self.max_chunk_bytes, cap.max_chunk_bytes),
            (self.max_basis_bytes, cap.max_basis_bytes),
            (self.max_triangle_updates, cap.max_triangle_updates),
            (self.max_refit_nodes, cap.max_refit_nodes),
            (self.max_copy_bytes, cap.max_copy_bytes),
        ]
        .iter()
        .any(|(n, max)| *n == 0 || n > max)
        {
            return Err(Error::new(
                "budget",
                "sculpt budgets must be positive within the sparse-displacement-v1 caps",
            ));
        }
        Ok(())
    }
}
