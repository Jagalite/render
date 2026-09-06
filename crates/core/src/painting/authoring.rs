use super::*;
use crate::{
    Id,
    document::{self, Command, Document, Principal},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub base_revision: String,
    pub idempotency_key: String,
    pub canvas: Id,
    pub source_asset: String,
    pub stroke: StrokeInput,
    pub budget: Budget,
    pub max_added_bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeRequest {
    pub version: u32,
    pub base_revision: String,
    pub idempotency_key: String,
    pub canvas: Id,
    pub source_asset: String,
    pub budget: BakeBudget,
    pub max_added_bytes: u64,
}
pub struct Prepared {
    pub transaction: document::Request,
    pub report: StrokeReport,
}
pub struct PreparedBake {
    pub transaction: document::Request,
    pub report: BakeReport,
}
fn admission(principal: &Principal, version: u32, key: &str) -> Result<()> {
    if !principal.can_write || principal.id.is_empty() {
        return Err(Error::new(
            "permission",
            "authenticated paint write capability required",
        ));
    }
    if version != 0 || !(16..=128).contains(&key.len()) {
        return Err(Error::new(
            "request",
            "invalid painting version or idempotency key",
        ));
    }
    Ok(())
}
fn selection(
    document: &Document,
    principal: &Principal,
    transaction: &document::Request,
    canvas: Id,
    source: &str,
) -> Result<()> {
    if document.retry(principal, transaction)?.is_none() {
        if document.snapshot().revision()? != transaction.base_revision {
            return Err(Error::new("stale_revision", "painting revision is stale"));
        }
        if document
            .snapshot()
            .paint_canvases
            .get(&canvas)
            .map(String::as_str)
            != Some(source)
        {
            return Err(Error::new(
                "stale_selection",
                "painting canvas source changed",
            ));
        }
    }
    Ok(())
}
pub fn prepare(
    document: &Document,
    principal: &Principal,
    request: &Request,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Prepared> {
    check_cancel(&mut cancelled)?;
    admission(principal, request.version, &request.idempotency_key)?;
    let snapshot = document.snapshot();
    let source = snapshot
        .paint_assets
        .get(&request.source_asset)
        .ok_or_else(|| Error::new("reference", "paint source asset missing"))?;
    let painted = paint(
        source,
        &snapshot.paint_tiles,
        &request.stroke,
        &request.budget,
        &mut cancelled,
    )?;
    let mut commands = painted
        .tiles
        .values()
        .map(|tile| Command::PutPaintTile {
            tile: tile.as_ref().clone(),
        })
        .collect::<Vec<_>>();
    commands.push(Command::PutPaintAsset {
        asset: painted.asset,
    });
    commands.push(Command::SetPaintCanvas {
        canvas: request.canvas,
        source_asset: Some(request.source_asset.clone()),
        asset: Some(painted.report.output.clone()),
    });
    let transaction = document::Request {
        version: 0,
        base_revision: request.base_revision.clone(),
        idempotency_key: request.idempotency_key.clone(),
        commands,
        max_added_bytes: request.max_added_bytes,
    };
    selection(
        document,
        principal,
        &transaction,
        request.canvas,
        &request.source_asset,
    )?;
    check_cancel(&mut cancelled)?;
    Ok(Prepared {
        transaction,
        report: painted.report,
    })
}
pub fn prepare_bake(
    document: &Document,
    principal: &Principal,
    request: &BakeRequest,
    mut cancelled: impl FnMut() -> bool,
) -> Result<PreparedBake> {
    check_cancel(&mut cancelled)?;
    admission(principal, request.version, &request.idempotency_key)?;
    let snapshot = document.snapshot();
    let source = snapshot
        .paint_assets
        .get(&request.source_asset)
        .ok_or_else(|| Error::new("reference", "paint bake source asset missing"))?;
    let baked = bake(
        source,
        &snapshot.paint_tiles,
        &request.budget,
        &mut cancelled,
    )?;
    let transaction = document::Request {
        version: 0,
        base_revision: request.base_revision.clone(),
        idempotency_key: request.idempotency_key.clone(),
        max_added_bytes: request.max_added_bytes,
        commands: vec![
            // Bind retry identity to the selected canvas and authored source even
            // when another canvas composites to exactly the same image bytes.
            Command::SetPaintCanvas {
                canvas: request.canvas,
                source_asset: Some(request.source_asset.clone()),
                asset: Some(request.source_asset.clone()),
            },
            Command::PutImage { image: baked.image },
        ],
    };
    selection(
        document,
        principal,
        &transaction,
        request.canvas,
        &request.source_asset,
    )?;
    check_cancel(&mut cancelled)?;
    Ok(PreparedBake {
        transaction,
        report: baked.report,
    })
}
