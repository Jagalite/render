//! Sparse authored f32 displacement over an immutable triangle mesh.
mod types;
pub use types::{
    Asset, BLOCK_LEN, BlockId, Budget, Chunk, Entry, MAX_ASSETS, MAX_BINDINGS, MAX_CHUNKS, PROFILE,
};

mod validation;
pub use validation::ChunkMap;

pub(crate) use validation::validate_snapshot;

mod evaluation;
pub use evaluation::{Cache, EvaluationReport};

mod authoring;
pub use authoring::{AuthoringReport, PointUpdate, Prepared, Request, prepare};
