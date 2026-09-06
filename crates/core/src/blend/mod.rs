//! Native bounded Blender container and selected static scene semantics.
pub(crate) mod reader;
mod scene;
pub use scene::{Imported, PROFILE, Policy, Report, import};
pub const MAX_INPUT_BYTES: usize = reader::MAX_BYTES;
