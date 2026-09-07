use super::*;
use crate::{Error, Result, geometry::Mesh};
use glam::DVec3;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub type ChunkMap = BTreeMap<String, Arc<Chunk>>;

pub(crate) fn check_cancel(cancelled: &mut impl FnMut() -> bool) -> Result<()> {
    if cancelled() {
        Err(Error::new("cancelled", "sculpt operation cancelled"))
    } else {
        Ok(())
    }
}
pub(crate) fn position_ok(position: DVec3) -> Result<()> {
    if !position.is_finite() || position.abs().max_element() > 1e9 {
        return Err(Error::new(
            "sculpt_metric",
            "sculpt positions must be finite within 1e9 local meters",
        ));
    }
    Ok(())
}
pub(crate) fn triangle_ok(points: [DVec3; 3]) -> Result<()> {
    let cross = (points[1] - points[0]).cross(points[2] - points[0]);
    let scale = (points[1] - points[0])
        .length()
        .max((points[2] - points[0]).length());
    let f = points.map(|p| p.as_vec3());
    let cross32 = (f[1] - f[0]).cross(f[2] - f[0]);
    if cross.length() <= scale * scale * 1e-12
        || !cross.length_squared().is_finite()
        || cross.length_squared() < f64::MIN_POSITIVE
        || !cross32.length_squared().is_finite()
        || cross32.length_squared() < f32::MIN_POSITIVE
    {
        return Err(Error::new(
            "sculpt_degenerate",
            "triangle must have resolvable finite area in both f64 and packed f32 frames",
        ));
    }
    let normal = cross.normalize();
    if points
        .iter()
        .any(|p| (*p - points[0]).dot(normal).abs() > scale * 1e-8)
    {
        return Err(Error::new(
            "sculpt_degenerate",
            "triangle normal is not numerically resolvable",
        ));
    }
    Ok(())
}
impl Asset {
    pub(crate) fn delta(&self, point: u64, chunks: &ChunkMap) -> Result<[f32; 3]> {
        let block = BlockId(point >> 6);
        let Some(key) = self.blocks.get(&block) else {
            return Ok([0.; 3]);
        };
        let chunk = chunks
            .get(key)
            .ok_or_else(|| Error::new("reference", "sculpt displacement chunk missing"))?;
        if chunk.block != block || chunk.base_mesh != self.base_mesh {
            return Err(Error::new(
                "reference",
                "sculpt chunk block/source mismatch",
            ));
        }
        Ok(chunk.delta((point & 63) as u8))
    }
    pub(crate) fn position(&self, base: &Mesh, chunks: &ChunkMap, index: usize) -> Result<DVec3> {
        let point = *base
            .point_ids
            .get(index)
            .ok_or_else(|| Error::new("reference", "sculpt point index outside base mesh"))?;
        let value = base.positions.get(index)
            + DVec3::from_array(self.delta(point, chunks)?.map(f64::from));
        position_ok(value)?;
        Ok(DVec3::from_array(
            value.to_array().map(|v| if v == 0. { 0. } else { v }),
        ))
    }
    /// The document has already verified mesh/chunk shape and content hashes.
    /// This checks all cross-references and displaced surfaces without a dense
    /// position array or polygon triangulation conversion. Admission is global.
    pub(crate) fn validate_refs(&self, base: &Mesh, chunks: &ChunkMap) -> Result<()> {
        self.validate()?;
        if base.face_ids.is_empty()
            || base.positions.len() > 65_536
            || base.face_ids.len() > 131_072
            || base.corners.len() > 393_216
            || base.face_offsets.windows(2).any(|w| w[1] - w[0] != 3)
        {
            return Err(Error::new(
                "sculpt_profile",
                "sculpt base requires at most 65536 points and 1..131072 triangular faces",
            ));
        }
        if base
            .attributes
            .values()
            .any(|a| matches!(a.semantic.as_str(), "normal" | "tangent" | "tangent_sign"))
        {
            return Err(Error::new(
                "sculpt_profile",
                "initial sculpt profile requires flat normals; authored normals/tangents need explicit removal",
            ));
        }
        let ids = base.point_ids.iter().copied().collect::<BTreeSet<_>>();
        for (block, key) in &self.blocks {
            let chunk = chunks
                .get(key)
                .ok_or_else(|| Error::new("reference", "sculpt chunk missing"))?;
            if chunk.base_mesh != self.base_mesh || chunk.block != *block {
                return Err(Error::new(
                    "reference",
                    "sculpt chunk belongs to another block or mesh",
                ));
            }
            for point in &chunk.points {
                let id = (block.0 << 6) | u64::from(point.slot);
                if !ids.contains(&id) {
                    return Err(Error::new(
                        "reference",
                        "sculpt displacement names unknown base point",
                    ));
                }
            }
        }
        for i in 0..base.positions.len() {
            self.position(base, chunks, i)?;
        }
        for face in base.face_offsets.windows(2) {
            let start = face[0] as usize;
            triangle_ok(std::array::from_fn(|i| {
                base.positions.get(base.corners[start + i].vertex as usize)
            }))?;
            let points = [
                self.position(base, chunks, base.corners[start].vertex as usize)?,
                self.position(base, chunks, base.corners[start + 1].vertex as usize)?,
                self.position(base, chunks, base.corners[start + 2].vertex as usize)?,
            ];
            triangle_ok(points)?;
        }
        Ok(())
    }
}

pub(crate) fn validate_snapshot(snapshot: &crate::document::Snapshot) -> Result<()> {
    if snapshot.sculpt_chunks.len() > MAX_CHUNKS
        || snapshot.sculpt_assets.len() > MAX_ASSETS
        || snapshot.sculpt_bindings.len() > MAX_BINDINGS
    {
        return Err(Error::new(
            "budget",
            "sculpt retained chunk, asset or binding cap",
        ));
    }
    // Bound all retained source families before allocating stable-ID lookup sets,
    // including chunks not yet referenced by an asset.
    let sources = snapshot
        .sculpt_chunks
        .values()
        .map(|c| &c.base_mesh)
        .chain(snapshot.sculpt_assets.values().map(|a| &a.base_mesh))
        .collect::<BTreeSet<_>>();
    let mut total_source_bytes = 0usize;
    for source in sources {
        let base = snapshot
            .meshes
            .get(source)
            .ok_or_else(|| Error::new("reference", "sculpt source mesh missing"))?;
        if base.positions.len() > 65_536
            || base.face_ids.len() > 131_072
            || base.corners.len() > 393_216
        {
            return Err(Error::new("budget", "sculpt retained source element cap"));
        }
        let bytes = crate::canonical(base)?.len();
        total_source_bytes = total_source_bytes
            .checked_add(bytes)
            .ok_or_else(|| Error::new("budget", "sculpt source size overflow"))?;
        if bytes > 8 * 1024 * 1024 || total_source_bytes > 16 * 1024 * 1024 {
            return Err(Error::new(
                "budget",
                "sculpt canonical source exceeds 8 MiB individual or 16 MiB combined",
            ));
        }
    }
    let mut point_ids = BTreeMap::new();
    for (key, chunk) in &snapshot.sculpt_chunks {
        if chunk.content_id()? != *key {
            return Err(Error::new("integrity", "sculpt chunk digest mismatch"));
        }
        let base = snapshot
            .meshes
            .get(&chunk.base_mesh)
            .ok_or_else(|| Error::new("reference", "sculpt chunk base mesh missing"))?;
        if base.positions.len() > 65_536 {
            return Err(Error::new("budget", "sculpt base point cap"));
        }
        let ids = point_ids
            .entry(&chunk.base_mesh)
            .or_insert_with(|| base.point_ids.iter().copied().collect::<BTreeSet<_>>());
        for entry in &chunk.points {
            if !ids.contains(&((chunk.block.0 << 6) | u64::from(entry.slot))) {
                return Err(Error::new(
                    "reference",
                    "retained sculpt chunk names unknown point",
                ));
            }
        }
    }
    for (key, asset) in &snapshot.sculpt_assets {
        if asset.content_id()? != *key {
            return Err(Error::new("integrity", "sculpt asset digest mismatch"));
        }
        let base = snapshot
            .meshes
            .get(&asset.base_mesh)
            .ok_or_else(|| Error::new("reference", "sculpt asset base mesh missing"))?;
        asset.validate_refs(base, &snapshot.sculpt_chunks)?;
    }
    for (id, key) in &snapshot.sculpt_bindings {
        let asset = snapshot
            .sculpt_assets
            .get(key)
            .ok_or_else(|| Error::new("reference", "sculpt bound asset missing"))?;
        let entity = snapshot
            .entities
            .get(*id)
            .ok_or_else(|| Error::new("reference", "sculpt bound entity missing"))?;
        if entity.mesh.as_ref() != Some(&asset.base_mesh) {
            return Err(Error::new(
                "stale_selection",
                "sculpt entity base mesh changed",
            ));
        }
        if snapshot.procedural_bindings.contains_key(id)
            || snapshot.geometry_bindings.contains_key(id)
            || snapshot.grooms.contains_key(id)
            || snapshot.animation.as_ref().is_some_and(|a| {
                a.skins.contains_key(id)
                    || a.morphs.contains_key(id)
                    || a.shading_frames.contains_key(id)
            })
        {
            return Err(Error::new(
                "sculpt_profile",
                "sculpt conflicts with another entity geometry/deformation binding",
            ));
        }
        for material in std::iter::once(entity.material).chain(
            snapshot
                .layers
                .iter()
                .filter_map(|layer| layer.overrides.get(id).map(|o| o.material)),
        ) {
            if material
                .and_then(|m| snapshot.materials.get(&m))
                .and_then(|m| m.pbr.as_ref())
                .is_some_and(|p| p.displacement.is_some())
            {
                return Err(Error::new(
                    "sculpt_profile",
                    "sculpt and material displacement cannot be combined in this profile",
                ));
            }
        }
        if snapshot.grooms.values().any(|g| {
            g.guides.iter().any(|v| v.root.entity == *id)
                || g.children.iter().any(|v| v.root.entity == *id)
        }) {
            return Err(Error::new(
                "sculpt_profile",
                "groom roots on sculpted entities require an explicit displaced-anchor profile",
            ));
        }
    }
    Ok(())
}
