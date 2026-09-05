//! Explicit static geometry profiles. Unsupported behavior produces errors or loss reports.
use crate::{Error, Id, Result, geometry::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LossReport {
    pub profile: String,
    pub losses: Vec<String>,
}
pub fn export_obj(mesh: &Mesh) -> Result<(String, LossReport)> {
    mesh.validate()?;
    let mut text = String::from("# render OBJ static polygon profile v0\n");
    for i in 0..mesh.positions.len() {
        let p = mesh.positions.get(i);
        text += &format!("v {} {} {}\n", p.x, p.y, p.z);
    }
    for c in 0..mesh.corners.len() {
        let uv = mesh.uv(c);
        text += &format!("vt {} {}\n", uv.x, uv.y);
    }
    for offsets in mesh.face_offsets.windows(2) {
        text.push('f');
        for c in offsets[0]..offsets[1] {
            text += &format!(" {}/{}", mesh.corners[c as usize].vertex + 1, c + 1);
        }
        text.push('\n');
    }
    let mut used = vec![false; mesh.edges.len()];
    for c in &mesh.corners {
        used[c.edge as usize] = true;
    }
    for (i, e) in mesh.edges.iter().enumerate() {
        if !used[i] {
            text += &format!("l {} {}\n", e[0] + 1, e[1] + 1);
        }
    }
    Ok((text,LossReport{profile:"OBJ geometry+corner UV v0".into(),losses:vec!["persistent element IDs and non-UV attributes are not representable in this profile".into()]}))
}
fn index(text: &str, count: usize) -> Result<u32> {
    let i: i64 = text
        .parse()
        .map_err(|_| Error::new("obj", "invalid index"))?;
    let resolved = if i < 0 { count as i64 + i } else { i - 1 };
    if resolved < 0 || resolved >= count as i64 {
        return Err(Error::new("obj", "index out of range"));
    }
    Ok(resolved as u32)
}
pub fn import_obj(text: &str) -> Result<(Mesh, LossReport)> {
    if text.len() > 64 * 1024 * 1024 {
        return Err(Error::new("budget", "OBJ exceeds input profile"));
    }
    let mut positions = vec![];
    let mut uv = vec![];
    let mut corner_uv = vec![];
    let mut faces = vec![];
    let mut loose = vec![];
    let mut losses = vec![];
    for line in text.lines() {
        let parts: Vec<_> = line
            .split('#')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            "v" => {
                if parts.len() != 4 {
                    return Err(Error::new("obj", "only xyz positions are supported"));
                }
                let mut p = [0.; 3];
                for i in 0..3 {
                    p[i] = parts[i + 1]
                        .parse::<f64>()
                        .map_err(|_| Error::new("obj", "invalid position"))?;
                }
                positions.push(p);
            }
            "vt" => {
                if parts.len() != 3 {
                    return Err(Error::new("obj", "only two-coordinate UVs supported"));
                }
                uv.push([
                    parts[1]
                        .parse::<f32>()
                        .map_err(|_| Error::new("obj", "UV"))?,
                    parts[2]
                        .parse::<f32>()
                        .map_err(|_| Error::new("obj", "UV"))?,
                ]);
            }
            "f" => {
                let mut face = vec![];
                for part in &parts[1..] {
                    let refs: Vec<_> = part.split('/').collect();
                    face.push(index(refs[0], positions.len())?);
                    let t = if refs.len() > 1 && !refs[1].is_empty() {
                        uv[index(refs[1], uv.len())? as usize]
                    } else {
                        [0.; 2]
                    };
                    corner_uv.push(t);
                    if refs.len() > 2 && !refs[2].is_empty() {
                        losses.push("imported normals regenerated from geometric faces".into());
                    }
                }
                faces.push(face);
            }
            "l" => {
                let ids = parts[1..]
                    .iter()
                    .map(|p| index(p, positions.len()))
                    .collect::<Result<Vec<_>>>()?;
                for pair in ids.windows(2) {
                    loose.push([pair[0], pair[1]]);
                }
            }
            "o" | "g" | "s" | "usemtl" | "mtllib" | "vn" => losses.push(format!(
                "{} metadata not retained; external assets are not fetched",
                parts[0]
            )),
            other => {
                return Err(Error::new(
                    "unsupported_obj",
                    format!("unsupported record {other}"),
                ));
            }
        }
    }
    let mut mesh = Mesh::from_polygons(Positions::F64(positions), &faces, &loose)?;
    if !uv.is_empty() {
        mesh.attributes.insert(
            "UVMap".into(),
            Attribute {
                id: Id(20),
                domain: Domain::Corner,
                semantic: "uv".into(),
                transfer: Transfer::Linear,
                values: AttributeValues::Vec2(corner_uv),
            },
        );
    }
    mesh.validate()?;
    losses.sort();
    losses.dedup();
    Ok((
        mesh,
        LossReport {
            profile: "OBJ geometry+corner UV v0".into(),
            losses,
        },
    ))
}

/// glTF 2.0 JSON + external binary pair. Adapter never resolves arbitrary URLs itself.
pub fn export_gltf(mesh: &Mesh) -> Result<(Vec<u8>, Vec<u8>, LossReport)> {
    let triangles = mesh.triangles()?;
    let mut positions = Vec::new();
    let mut uv = Vec::new();
    for triangle in &triangles {
        for &c in triangle {
            let p = mesh
                .positions
                .get(mesh.corners[c as usize].vertex as usize)
                .as_vec3();
            if !p.is_finite() {
                return Err(Error::new("precision", "glTF f32 position overflow"));
            }
            for v in p.to_array() {
                positions.extend(v.to_le_bytes());
            }
            for v in mesh.uv(c as usize).to_array() {
                uv.extend(v.to_le_bytes());
            }
        }
    }
    let n = triangles.len() * 3;
    let offset = positions.len();
    let mut bin = positions;
    bin.extend(uv);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for bytes in bin[..offset].chunks_exact(12) {
        for k in 0..3 {
            let f = f32::from_le_bytes(bytes[k * 4..k * 4 + 4].try_into().expect("chunk"));
            min[k] = min[k].min(f);
            max[k] = max[k].max(f);
        }
    }
    if n == 0 {
        return Err(Error::new("unsupported_gltf", "empty triangle export"));
    }
    let json = serde_json::json!({"asset":{"version":"2.0","generator":"render Rust static profile v0"},"buffers":[{"uri":"mesh.bin","byteLength":bin.len()}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":offset},{"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset}],"accessors":[{"bufferView":0,"componentType":5126,"count":n,"type":"VEC3","min":min,"max":max},{"bufferView":1,"componentType":5126,"count":n,"type":"VEC2"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"mode":4}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    Ok((crate::canonical(&json)?,bin,LossReport{profile:"glTF 2.0 static triangle positions+UV v0".into(),losses:vec!["polygon rings triangulated; loose geometry omitted; element IDs and non-UV attributes omitted".into()]}))
}
pub fn import_gltf(json: &[u8], bin: &[u8]) -> Result<(Mesh, LossReport)> {
    use serde_json::Value;
    if json.len() > 16 * 1024 * 1024 || bin.len() > 64 * 1024 * 1024 {
        return Err(Error::new("budget", "glTF input exceeds profile"));
    }
    let j: Value = serde_json::from_slice(json)?;
    let fail = || {
        Error::new(
            "unsupported_gltf",
            "requires glTF 2.0 single static triangle primitive with contiguous f32 positions and UVs",
        )
    };
    if json.len() > 16 * 1024 * 1024 || bin.len() > 64 * 1024 * 1024 {
        return Err(Error::new("budget", "glTF input exceeds profile"));
    }
    if j["asset"]["version"] != "2.0"
        || j["buffers"].as_array().is_none_or(|a| a.len() != 1)
        || j["buffers"][0]["byteLength"].as_u64() != Some(bin.len() as u64)
        || j.get("animations").is_some()
        || j.get("skins").is_some()
        || j.get("extensionsRequired").is_some()
        || j["meshes"].as_array().is_none_or(|a| a.len() != 1)
        || j["meshes"][0]["primitives"]
            .as_array()
            .is_none_or(|a| a.len() != 1)
    {
        return Err(fail());
    }
    let p = &j["meshes"][0]["primitives"][0];
    if p["mode"].as_u64().unwrap_or(4) != 4 || p.get("targets").is_some() {
        return Err(fail());
    }
    fn floats(j: &Value, bin: &[u8], a: u64, width: usize) -> Result<Vec<Vec<f32>>> {
        let fail = || Error::new("gltf_accessor", "unsupported or out-of-bounds accessor");
        let a = &j["accessors"][usize::try_from(a).map_err(|_| fail())?];
        if a["componentType"] != 5126
            || a["type"] != if width == 3 { "VEC3" } else { "VEC2" }
            || a.get("sparse").is_some()
            || a["normalized"].as_bool().unwrap_or(false)
        {
            return Err(fail());
        }
        let v = &j["bufferViews"]
            [usize::try_from(a["bufferView"].as_u64().ok_or_else(fail)?).map_err(|_| fail())?];
        if v["buffer"].as_u64() != Some(0) {
            return Err(fail());
        }
        let count = usize::try_from(a["count"].as_u64().ok_or_else(fail)?).map_err(|_| fail())?;
        let stride = usize::try_from(v["byteStride"].as_u64().unwrap_or((width * 4) as u64))
            .map_err(|_| fail())?;
        let base = usize::try_from(v["byteOffset"].as_u64().unwrap_or(0)).map_err(|_| fail())?;
        let offset = usize::try_from(a["byteOffset"].as_u64().unwrap_or(0)).map_err(|_| fail())?;
        let length =
            usize::try_from(v["byteLength"].as_u64().ok_or_else(fail)?).map_err(|_| fail())?;
        if count == 0
            || stride < width * 4
            || !stride.is_multiple_of(4)
            || base.checked_add(length).is_none_or(|n| n > bin.len())
            || count
                .checked_sub(1)
                .and_then(|n| n.checked_mul(stride))
                .and_then(|n| {
                    offset
                        .checked_add(width * 4)
                        .and_then(|offset| n.checked_add(offset))
                })
                .is_none_or(|n| n > length)
        {
            return Err(fail());
        }
        let mut out = vec![];
        for i in 0..count {
            let start = base + offset + i * stride;
            let values = (0..width)
                .map(|k| {
                    f32::from_le_bytes(
                        bin[start + k * 4..start + k * 4 + 4]
                            .try_into()
                            .expect("checked"),
                    )
                })
                .collect::<Vec<_>>();
            if values.iter().any(|v| !v.is_finite()) {
                return Err(fail());
            }
            out.push(values);
        }
        Ok(out)
    }
    if p.get("indices").is_some() {
        return Err(Error::new(
            "unsupported_gltf",
            "indexed primitives are outside initial adapter profile",
        ));
    }
    let pos = floats(
        &j,
        bin,
        p["attributes"]["POSITION"].as_u64().ok_or_else(fail)?,
        3,
    )?;
    if pos.len() % 3 != 0 {
        return Err(fail());
    }
    let positions = pos.iter().map(|v| [v[0], v[1], v[2]]).collect();
    let faces = (0..pos.len() as u32)
        .collect::<Vec<_>>()
        .chunks_exact(3)
        .map(|v| v.to_vec())
        .collect::<Vec<_>>();
    let mut mesh = Mesh::from_polygons(Positions::F32(positions), &faces, &[])?;
    if let Some(a) = p["attributes"]["TEXCOORD_0"].as_u64() {
        let uv = floats(&j, bin, a, 2)?;
        mesh.attributes = BTreeMap::from([(
            "UVMap".into(),
            Attribute {
                id: Id(20),
                domain: Domain::Corner,
                semantic: "uv".into(),
                transfer: Transfer::Linear,
                values: AttributeValues::Vec2(uv.iter().map(|v| [v[0], v[1]]).collect()),
            },
        )]);
    }
    mesh.validate()?;
    Ok((mesh,LossReport{profile:"glTF 2.0 static triangle positions+UV v0".into(),losses:vec!["mesh-only import; scene placement, materials and other attributes are not imported".into()]}))
}
