//! Native mesh chunk v0: explicit counts, little-endian numeric payloads, typed attributes.
use crate::{Error, Result, canonical, geometry::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Serialize, Deserialize)]
struct Header {
    precision: u8,
    points: u32,
    edges: u32,
    offsets: u32,
    corners: u32,
    faces: u32,
    attributes: BTreeMap<String, Attribute>,
}
pub fn encode(mesh: &Mesh) -> Result<Vec<u8>> {
    mesh.validate()?;
    let header = Header {
        precision: if matches!(mesh.positions, Positions::F64(_)) {
            64
        } else {
            32
        },
        points: mesh.positions.len() as u32,
        edges: mesh.edges.len() as u32,
        offsets: mesh.face_offsets.len() as u32,
        corners: mesh.corners.len() as u32,
        faces: mesh.faces() as u32,
        attributes: mesh.attributes.clone(),
    };
    let meta = canonical(&header)?;
    let mut out = b"R3DMESH0".to_vec();
    out.extend((meta.len() as u32).to_le_bytes());
    out.extend(meta);
    match &mesh.positions {
        Positions::F32(p) => {
            for &v in p.iter().flatten() {
                out.extend(v.to_le_bytes());
            }
        }
        Positions::F64(p) => {
            for &v in p.iter().flatten() {
                out.extend(v.to_le_bytes());
            }
        }
    }
    for &v in mesh.edges.iter().flatten() {
        out.extend(v.to_le_bytes());
    }
    for &v in &mesh.face_offsets {
        out.extend(v.to_le_bytes());
    }
    for c in &mesh.corners {
        out.extend(c.vertex.to_le_bytes());
        out.extend(c.edge.to_le_bytes());
    }
    for ids in [
        &mesh.point_ids,
        &mesh.edge_ids,
        &mesh.face_ids,
        &mesh.corner_ids,
    ] {
        for &v in ids {
            out.extend(v.to_le_bytes());
        }
    }
    Ok(out)
}
struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Cursor<'_> {
    fn read<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| Error::new("mesh_chunk", "byte range overflow"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| Error::new("mesh_chunk", "truncated numeric payload"))?;
        self.offset = end;
        Ok(bytes.try_into().expect("checked array"))
    }
}
pub fn decode(bytes: &[u8]) -> Result<Mesh> {
    if bytes.len() > 128 * 1024 * 1024 || bytes.get(..8) != Some(b"R3DMESH0") {
        return Err(Error::new(
            "mesh_chunk",
            "invalid signature or excessive chunk size",
        ));
    }
    let mut c = Cursor { bytes, offset: 8 };
    let len = u32::from_le_bytes(c.read()?) as usize;
    let end = c
        .offset
        .checked_add(len)
        .ok_or_else(|| Error::new("mesh_chunk", "metadata size overflow"))?;
    let header: Header = serde_json::from_slice(
        bytes
            .get(c.offset..end)
            .ok_or_else(|| Error::new("mesh_chunk", "truncated metadata"))?,
    )?;
    c.offset = end;
    if ![32, 64].contains(&header.precision) || header.faces.checked_add(1) != Some(header.offsets)
    {
        return Err(Error::new("mesh_chunk", "invalid precision/counts"));
    }
    let expected = u64::from(header.points) * 3 * u64::from(header.precision / 8)
        + u64::from(header.edges) * 8
        + u64::from(header.offsets) * 4
        + u64::from(header.corners) * 8
        + (u64::from(header.points)
            + u64::from(header.edges)
            + u64::from(header.faces)
            + u64::from(header.corners))
            * 8;
    if expected != (bytes.len() - end) as u64 {
        return Err(Error::new(
            "mesh_chunk",
            "numeric payload length does not match counts",
        ));
    }
    let positions = if header.precision == 32 {
        let mut p = Vec::new();
        for _ in 0..header.points {
            p.push([
                f32::from_le_bytes(c.read()?),
                f32::from_le_bytes(c.read()?),
                f32::from_le_bytes(c.read()?),
            ]);
        }
        Positions::F32(p)
    } else {
        let mut p = Vec::new();
        for _ in 0..header.points {
            p.push([
                f64::from_le_bytes(c.read()?),
                f64::from_le_bytes(c.read()?),
                f64::from_le_bytes(c.read()?),
            ]);
        }
        Positions::F64(p)
    };
    let mut edges = vec![];
    for _ in 0..header.edges {
        edges.push([u32::from_le_bytes(c.read()?), u32::from_le_bytes(c.read()?)]);
    }
    let mut face_offsets = vec![];
    for _ in 0..header.offsets {
        face_offsets.push(u32::from_le_bytes(c.read()?));
    }
    let mut corners = vec![];
    for _ in 0..header.corners {
        corners.push(Corner {
            vertex: u32::from_le_bytes(c.read()?),
            edge: u32::from_le_bytes(c.read()?),
        });
    }
    let mut identities = Vec::new();
    for count in [header.points, header.edges, header.faces, header.corners] {
        let mut ids = vec![];
        for _ in 0..count {
            ids.push(u64::from_le_bytes(c.read()?));
        }
        identities.push(ids);
    }
    let mut ids = identities.into_iter();
    let mesh = Mesh {
        positions,
        edges,
        face_offsets,
        corners,
        point_ids: ids.next().expect("four domains"),
        edge_ids: ids.next().expect("four domains"),
        face_ids: ids.next().expect("four domains"),
        corner_ids: ids.next().expect("four domains"),
        attributes: header.attributes,
    };
    mesh.validate()?;
    Ok(mesh)
}
