//! Exact inert source containers. These are provenance, not evaluated scene data.
use crate::{Error, Result, canonical, digest};
use serde::{Deserialize, Serialize};
pub const MAX_ASSETS: usize = 4;
pub const MAX_TOTAL_BYTES: usize = 4 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case", deny_unknown_fields)]
pub enum Asset {
    Blend { version: u16, bytes: Vec<u8> },
}
impl Asset {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Blend { version, bytes } => {
                let file = crate::blend::reader::File::parse(bytes, || false)?;
                if *version != file.header.version {
                    return Err(Error::new(
                        "integrity",
                        "source format version does not match bytes",
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Blend { bytes, .. } => bytes,
        }
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportRequest {
    pub revision: String,
    pub asset: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exported {
    pub profile: String,
    pub source_asset: String,
    pub source_digest: String,
    pub version: u16,
    pub bytes: Vec<u8>,
}
pub fn export(
    snapshot: &crate::document::Snapshot,
    request: &ExportRequest,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Exported> {
    if cancelled() {
        return Err(Error::new("cancelled", "source export cancelled"));
    }
    if snapshot.revision()? != request.revision {
        return Err(Error::new(
            "stale_revision",
            "source export revision is stale",
        ));
    }
    let asset = snapshot
        .source_assets
        .get(&request.asset)
        .ok_or_else(|| Error::new("reference", "source asset missing"))?;
    let Asset::Blend { version, bytes } = asset.as_ref();
    let out = Exported {
        profile: "exact-original-blend-source-v1".into(),
        source_asset: request.asset.clone(),
        source_digest: digest(bytes),
        version: *version,
        bytes: bytes.clone(),
    };
    if cancelled() {
        return Err(Error::new(
            "cancelled",
            "source export cancelled before delivery",
        ));
    }
    Ok(out)
}
