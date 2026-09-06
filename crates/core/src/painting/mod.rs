//! Bounded editable linear paint tiles. Publication uses ordinary document transactions.
mod authoring;
pub use authoring::{BakeRequest, Prepared, PreparedBake, Request, prepare, prepare_bake};
mod bake;
mod tiles;
pub use bake::{BakeBudget, BakeReport, Baked, bake};
mod brush;
use crate::{Error, Result};
pub use brush::{Brush, Budget, Mode, Painted, Sample, Stroke, StrokeInput, StrokeReport, paint};
pub use tiles::{Asset, Coordinate, Layer, Tile, TileMap, TileRef};
pub const PROFILE: &str = "paint-tiles-v1";
pub const TILE_SIDE: u32 = 32;
pub const TILE_PIXELS: usize = 1024;
pub const MAX_TILES: usize = 512;
pub const MAX_ASSETS: usize = 64;
pub(crate) fn mul16(a: u16, b: u16) -> u16 {
    ((u32::from(a) * u32::from(b) + 32767) / 65535) as u16
}
pub(crate) fn check_cancel(cancelled: &mut impl FnMut() -> bool) -> Result<()> {
    if cancelled() {
        Err(Error::new("cancelled", "painting cancelled"))
    } else {
        Ok(())
    }
}
pub(crate) fn valid_hash(hash: &str) -> bool {
    hash.len() == 71
        && hash.starts_with("sha256:")
        && hash[7..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
