//! Point edits prepare ordinary transactions; no authored snapshot is mutated here.
use super::{Asset, BlockId, Budget, Chunk, Entry, types::metric, validation::check_cancel};
use crate::{
    Error, Id, Result, canonical,
    document::{self, Command, Document, Principal},
    geometry_query::ElementId,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointUpdate {
    pub point: ElementId,
    /// Absolute displacement relative to the immutable base, in local meters.
    pub delta_meters: [f32; 3],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub base_revision: String,
    pub idempotency_key: String,
    pub entity: Id,
    pub source_asset: String,
    pub points: Vec<PointUpdate>,
    pub budget: Budget,
    pub max_added_bytes: u64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringReport {
    pub source: String,
    pub output: String,
    pub requested_points: usize,
    pub changed_chunks: usize,
    pub authored_chunk_bytes: usize,
    pub global_snapshot_admission: bool,
}
pub struct Prepared {
    pub transaction: document::Request,
    pub report: AuthoringReport,
}
pub fn prepare(
    document: &Document,
    principal: &Principal,
    request: &Request,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Prepared> {
    check_cancel(&mut cancelled)?;
    if !principal.can_write || principal.id.is_empty() {
        return Err(Error::new(
            "permission",
            "authenticated sculpt write capability required",
        ));
    }
    if request.version != 0 || !(16..=128).contains(&request.idempotency_key.len()) {
        return Err(Error::new(
            "request",
            "invalid sculpt request version or idempotency key",
        ));
    }
    request.budget.validate()?;
    if request.points.is_empty() || request.points.len() as u64 > request.budget.max_point_updates {
        return Err(Error::new(
            "budget",
            "sculpt point update count exceeds profile",
        ));
    }
    let snapshot = document.snapshot();
    let source = snapshot
        .sculpt_assets
        .get(&request.source_asset)
        .ok_or_else(|| Error::new("reference", "sculpt source asset missing"))?;
    let mesh = snapshot
        .meshes
        .get(&source.base_mesh)
        .ok_or_else(|| Error::new("reference", "sculpt base mesh missing"))?;
    let known = mesh.point_ids.iter().copied().collect::<BTreeSet<_>>();
    let mut groups: BTreeMap<BlockId, BTreeMap<u8, [f32; 3]>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for update in &request.points {
        check_cancel(&mut cancelled)?;
        metric(update.delta_meters)?;
        if !seen.insert(update.point.0) || !known.contains(&update.point.0) {
            return Err(Error::new(
                "sculpt_point",
                "duplicate or unknown stable point ID",
            ));
        }
        groups
            .entry(BlockId(update.point.0 >> 6))
            .or_default()
            .insert(
                (update.point.0 & 63) as u8,
                update.delta_meters.map(|v| if v == 0. { 0. } else { v }),
            );
    }
    if groups.len() as u64 > request.budget.max_changed_chunks {
        return Err(Error::new(
            "budget",
            "sculpt touched block count exceeds budget",
        ));
    }
    let mut asset: Asset = source.as_ref().clone();
    let mut commands = Vec::new();
    let mut report = AuthoringReport {
        source: request.source_asset.clone(),
        requested_points: request.points.len(),
        global_snapshot_admission: true,
        ..Default::default()
    };
    for (block, updates) in groups {
        check_cancel(&mut cancelled)?;
        let old = source.blocks.get(&block);
        let mut points = if let Some(key) = old {
            snapshot
                .sculpt_chunks
                .get(key)
                .ok_or_else(|| Error::new("reference", "sculpt source chunk missing"))?
                .points
                .iter()
                .map(|p| (p.slot, p.delta_meters))
                .collect::<BTreeMap<_, _>>()
        } else {
            BTreeMap::new()
        };
        for (slot, delta) in updates {
            if delta == [0.; 3] {
                points.remove(&slot);
            } else {
                points.insert(slot, delta);
            }
        }
        if points.is_empty() {
            if asset.blocks.remove(&block).is_some() {
                report.changed_chunks += 1;
            }
        } else {
            let chunk = Chunk {
                version: 0,
                base_mesh: source.base_mesh.clone(),
                block,
                points: points
                    .into_iter()
                    .map(|(slot, delta_meters)| Entry { slot, delta_meters })
                    .collect(),
            };
            let key = chunk.content_id()?;
            if old != Some(&key) {
                report.changed_chunks += 1;
                report.authored_chunk_bytes += canonical(&chunk)?.len();
                if report.authored_chunk_bytes as u64 > request.budget.max_chunk_bytes {
                    return Err(Error::new(
                        "budget",
                        "sculpt authored chunk bytes exceed budget",
                    ));
                }
                asset.blocks.insert(block, key);
                commands.push(Command::PutSculptChunk { chunk });
            }
        }
    }
    report.output = asset.content_id()?;
    commands.push(Command::PutSculptAsset { asset });
    commands.push(Command::SetSculpt {
        entity: request.entity,
        source_mesh: source.base_mesh.clone(),
        source_asset: Some(request.source_asset.clone()),
        asset: Some(report.output.clone()),
    });
    let transaction = document::Request {
        version: 0,
        base_revision: request.base_revision.clone(),
        idempotency_key: request.idempotency_key.clone(),
        commands,
        max_added_bytes: request.max_added_bytes,
    };
    if document.retry(principal, &transaction)?.is_none() {
        // Ordinary preparation performs CAS, full cross-domain admission and byte limits.
        let candidate = document.prepare(principal, &transaction)?;
        check_cancel(&mut cancelled)?;
        let next = candidate.snapshot();
        // Cold reconstruction must fit the same budget so a saved checkpoint is
        // independently renderable without relying on a prior process cache.
        let mut cache = super::Cache::default();
        cache.evaluate(
            &report.output,
            next.sculpt_assets[&report.output].clone(),
            mesh.clone(),
            &next.sculpt_chunks,
            &request.budget,
            &mut cancelled,
        )?;
    }
    check_cancel(&mut cancelled)?;
    Ok(Prepared {
        transaction,
        report,
    })
}
