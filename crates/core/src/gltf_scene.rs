//! Static glTF 2.0 scene profile. No URI resolution or executable metadata.
use crate::{document::*, geometry::*, *};
use glam::{DAffine3, DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_INPUT: usize = 4 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Explicit consent to converting glTF's dielectric BRDF to Lambertian.
    pub allow_lambertian: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub profile: String,
    pub source_digest: String,
    pub losses: Vec<String>,
    pub source_nodes: BTreeMap<usize, Id>,
    pub source_materials: BTreeMap<usize, Id>,
    pub instances: usize,
    pub unique_meshes: usize,
}
pub struct Imported {
    pub commands: Vec<Command>,
    pub report: Report,
}
fn bad(message: &str) -> Error {
    Error::new("gltf_scene", message).at("import")
}
fn unsupported(message: &str) -> Error {
    Error::new("unsupported_gltf", message).at("import")
}
pub(super) fn index(v: &Value) -> Result<usize> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| bad("expected unsigned index"))
}
pub(super) fn number(v: &Value) -> Result<f64> {
    v.as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| bad("expected finite number"))
}
pub(super) fn vector<const N: usize>(v: Option<&Value>, default: [f64; N]) -> Result<[f64; N]> {
    let Some(v) = v else { return Ok(default) };
    let a = v
        .as_array()
        .filter(|a| a.len() == N)
        .ok_or_else(|| bad("vector dimension"))?;
    let mut out = default;
    for (dst, src) in out.iter_mut().zip(a) {
        *dst = number(src)?;
    }
    Ok(out)
}
pub(super) fn array<'a>(v: &'a Value, key: &str) -> Result<&'a [Value]> {
    match v.get(key) {
        None => Ok(&[]),
        Some(x) => x
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| bad("expected array")),
    }
}
pub(super) fn at<'a>(v: &'a Value, key: &str, i: usize) -> Result<&'a Value> {
    array(v, key)?
        .get(i)
        .ok_or_else(|| bad("index out of range"))
}
pub(super) fn offset(v: &Value, key: &str) -> Result<usize> {
    v.get(key).map(index).unwrap_or(Ok(0))
}
fn identity(document: Id, source: &str, kind: &str, index: usize) -> Result<Id> {
    // Persisted source-to-ID mapping; these are not runtime slot handles.
    let hash = digest(&canonical(&(document, source, kind, index))?);
    Ok(Id(
        u128::from_str_radix(&hash[7..39], 16).expect("hex digest")
    ))
}
fn u32le(bytes: &[u8], offset: usize) -> Result<u32> {
    let a = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| bad("truncated GLB word"))?;
    Ok(u32::from_le_bytes(a.try_into().expect("four bytes")))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PbrPolicy {
    pub allow_approximations: bool,
}
pub fn import_pbr_glb(bytes: &[u8], document: Id, policy: &PbrPolicy) -> Result<Imported> {
    if !policy.allow_approximations {
        return Err(unsupported(
            "PBR v0 requires explicit consent to roughness regularization and fallback tangent policy",
        ));
    }
    import_glb_mode(
        bytes,
        document,
        &Policy {
            allow_lambertian: false,
        },
        true,
    )
}
pub fn import_glb(bytes: &[u8], document: Id, policy: &Policy) -> Result<Imported> {
    import_glb_mode(bytes, document, policy, false)
}
fn import_glb_mode(bytes: &[u8], document: Id, policy: &Policy, pbr: bool) -> Result<Imported> {
    if bytes.len() > MAX_INPUT {
        return Err(Error::new("budget", "GLB exceeds 4 MiB profile"));
    }
    if u32le(bytes, 0)? != 0x46546c67
        || u32le(bytes, 4)? != 2
        || u32le(bytes, 8)? as usize != bytes.len()
    {
        return Err(bad("invalid GLB magic, version or declared length"));
    }
    let mut cursor = 12;
    let mut chunks = vec![];
    while cursor < bytes.len() {
        let length = u32le(bytes, cursor)? as usize;
        let kind = u32le(bytes, cursor + 4)?;
        cursor += 8;
        if !length.is_multiple_of(4) {
            return Err(bad("unaligned GLB chunk"));
        }
        let end = cursor
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| bad("truncated GLB chunk"))?;
        chunks.push((kind, &bytes[cursor..end]));
        cursor = end;
    }
    if chunks.len() != 2 || chunks[0].0 != 0x4e4f534a || chunks[1].0 != 0x004e4942 {
        return Err(unsupported(
            "profile requires JSON followed by one BIN chunk",
        ));
    }
    let root: Value = serde_json::from_slice(chunks[0].1)?;
    let buffer = at(&root, "buffers", 0)?;
    if buffer.get("uri").is_some() {
        return Err(bad("GLB embedded buffer cannot declare URI"));
    }
    let length = index(&buffer["byteLength"])?;
    if chunks[1].1.len() < length
        || chunks[1].1.len() - length > 3
        || chunks[1].1[length..].iter().any(|b| *b != 0)
    {
        return Err(bad("invalid BIN length or padding"));
    }
    import_scene_mode(
        chunks[0].1,
        &[chunks[1].1[..length].to_vec()],
        &[],
        document,
        policy,
        pbr,
    )
}

struct Accessor<'a> {
    bytes: &'a [u8],
    start: usize,
    stride: usize,
    count: usize,
    component: usize,
    width: usize,
}
impl Accessor<'_> {
    fn component(&self, element: usize, axis: usize) -> &[u8] {
        let start = self.start + element * self.stride + axis * self.width;
        &self.bytes[start..start + self.width]
    }
    fn floats<const N: usize>(&self) -> Result<Vec<[f32; N]>> {
        if self.component != 5126 {
            return Err(unsupported("attributes require float32"));
        }
        (0..self.count)
            .map(|i| {
                let v = std::array::from_fn(|axis| {
                    f32::from_le_bytes(self.component(i, axis).try_into().expect("validated width"))
                });
                if v.iter().any(|x| !x.is_finite()) {
                    Err(bad("nonfinite attribute"))
                } else {
                    Ok(v)
                }
            })
            .collect()
    }
}
fn accessor<'a>(
    root: &Value,
    buffers: &'a [Vec<u8>],
    id: usize,
    shape: &str,
    axes: usize,
) -> Result<Accessor<'a>> {
    let a = at(root, "accessors", id)?;
    if a.get("sparse").is_some() || a.get("normalized").is_some_and(|v| v != false) {
        return Err(unsupported("sparse/normalized accessor"));
    }
    if a["type"] != shape {
        return Err(bad("accessor shape mismatch"));
    }
    let component = index(&a["componentType"])?;
    let width = match component {
        5121 => 1,
        5123 => 2,
        5125 | 5126 => 4,
        _ => return Err(unsupported("component type")),
    };
    let count = index(&a["count"])?;
    if count == 0 || count > 196608 {
        return Err(Error::new("budget", "accessor count outside profile"));
    }
    let view = at(root, "bufferViews", index(&a["bufferView"])?)?;
    if shape == "SCALAR" && view.get("byteStride").is_some() {
        return Err(bad("index bufferView cannot declare byteStride"));
    }
    let bytes = buffers
        .get(index(&view["buffer"])?)
        .ok_or_else(|| bad("missing buffer"))?;
    let view_start = offset(view, "byteOffset")?;
    let view_len = index(&view["byteLength"])?;
    let start = offset(a, "byteOffset")?;
    let element = width * axes;
    let stride = view
        .get("byteStride")
        .map(index)
        .transpose()?
        .unwrap_or(element);
    if stride < element
        || !stride.is_multiple_of(width)
        || (view.get("byteStride").is_some()
            && (!(4..=252).contains(&stride) || !stride.is_multiple_of(4)))
    {
        return Err(bad("invalid accessor stride"));
    }
    let absolute = view_start
        .checked_add(start)
        .ok_or_else(|| bad("offset overflow"))?;
    let end = (count - 1)
        .checked_mul(stride)
        .and_then(|n| n.checked_add(element))
        .and_then(|n| n.checked_add(start))
        .ok_or_else(|| bad("accessor overflow"))?;
    if !absolute.is_multiple_of(width)
        || end > view_len
        || view_start
            .checked_add(view_len)
            .is_none_or(|n| n > bytes.len())
    {
        return Err(bad("accessor exceeds view/buffer or is unaligned"));
    }
    Ok(Accessor {
        bytes,
        start: absolute,
        stride,
        count,
        component,
        width,
    })
}
fn transform(node: &Value) -> Result<Transform> {
    let a = if node.get("matrix").is_some() {
        if ["translation", "rotation", "scale"]
            .iter()
            .any(|k| node.get(k).is_some())
        {
            return Err(bad("matrix and TRS are mutually exclusive"));
        }
        let m = vector(node.get("matrix"), [0.; 16])?;
        if [m[3], m[7], m[11], m[15]] != [0., 0., 0., 1.] {
            return Err(bad("non-affine matrix"));
        }
        DAffine3::from_mat4(DMat4::from_cols_array(&m))
    } else {
        let t = DVec3::from_array(vector(node.get("translation"), [0.; 3])?);
        let s = DVec3::from_array(vector(node.get("scale"), [1.; 3])?);
        let r = DQuat::from_array(vector(node.get("rotation"), [0., 0., 0., 1.])?);
        if (r.length_squared() - 1.).abs() > 1e-5 {
            return Err(bad("rotation quaternion must be normalized"));
        }
        DAffine3::from_scale_rotation_translation(s, r.normalize(), t)
    };
    let t = Transform {
        columns: a.to_cols_array_2d(),
        operations: vec![],
    };
    t.affine()?;
    Ok(t)
}

pub fn import_scene(
    json: &[u8],
    buffers: &[Vec<u8>],
    document: Id,
    policy: &Policy,
) -> Result<Imported> {
    import_scene_mode(json, buffers, &[], document, policy, false)
}
pub fn import_pbr_scene(
    json: &[u8],
    buffers: &[Vec<u8>],
    images: &[Vec<u8>],
    document: Id,
    policy: &PbrPolicy,
) -> Result<Imported> {
    if !policy.allow_approximations {
        return Err(unsupported(
            "PBR v0 requires explicit approximation consent",
        ));
    }
    import_scene_mode(
        json,
        buffers,
        images,
        document,
        &Policy {
            allow_lambertian: false,
        },
        true,
    )
}
fn import_scene_mode(
    json: &[u8],
    buffers: &[Vec<u8>],
    external_images: &[Vec<u8>],
    document: Id,
    policy: &Policy,
    pbr_mode: bool,
) -> Result<Imported> {
    let total = buffers
        .iter()
        .chain(external_images.iter())
        .try_fold(json.len(), |n, b| n.checked_add(b.len()))
        .ok_or_else(|| Error::new("budget", "input overflow"))?;
    if total > MAX_INPUT {
        return Err(Error::new("budget", "scene exceeds 4 MiB profile"));
    }
    let root: Value = serde_json::from_slice(json)?;
    if root["asset"]["version"] != "2.0"
        || root["asset"].get("minVersion").is_some_and(|v| v != "2.0")
    {
        return Err(unsupported("glTF version"));
    }
    for key in [
        "animations",
        "skins",
        "cameras",
        "textures",
        "images",
        "extensionsRequired",
        "extensionsUsed",
    ] {
        if !(array(&root, key)?.is_empty()
            || pbr_mode && ["cameras", "textures", "images"].contains(&key))
        {
            return Err(unsupported(&format!(
                "{key} outside static diffuse scene profile"
            )));
        }
    }
    if root.get("extensions").is_some() {
        return Err(unsupported("root extensions"));
    }
    let declared = array(&root, "buffers")?;
    if buffers.len() != declared.len() || buffers.is_empty() {
        return Err(bad("provide every declared buffer explicitly"));
    }
    for (buffer, data) in declared.iter().zip(buffers) {
        if index(&buffer["byteLength"])? != data.len() {
            return Err(bad("buffer length mismatch"));
        }
    }
    let nodes = array(&root, "nodes")?;
    if nodes.is_empty()
        || nodes.len() > 64
        || array(&root, "meshes")?.len() > 64
        || array(&root, "materials")?.len() > 64
    {
        return Err(Error::new("budget", "scene table limit is 64"));
    }
    let source = if pbr_mode {
        digest(&canonical(&(&root, buffers, external_images))?)
    } else {
        digest(&canonical(&(&root, buffers))?)
    };
    let mut report = Report {
        profile: if pbr_mode {
            "gltf2-static-pbr-v0"
        } else {
            "gltf2-static-lambertian-v0"
        }
        .into(),
        source_digest: source.clone(),
        losses: vec![],
        source_nodes: BTreeMap::new(),
        source_materials: BTreeMap::new(),
        instances: 0,
        unique_meshes: 0,
    };
    let mut commands = vec![];
    if !pbr_mode && !policy.allow_lambertian {
        return Err(unsupported(
            "explicit allow_lambertian consent required: glTF dielectric specular is not implemented",
        ));
    }
    if pbr_mode {
        report.losses.push("Single-scattering GGX, roughness regularized to >=0.05; missing tangents use a per-triangle UV basis, not MikkTSpace seam smoothing. OPAQUE alpha is ignored as specified by glTF.".into());
    } else {
        report.losses.push("Opt-in Lambertian conversion omits glTF dielectric specular; renderer is two-sided and uses geometric normals. No PBR fidelity claim.".into());
    }
    report.losses.push("Names are retained; extras and generator metadata are not authored components. Only the selected scene is imported.".into());
    let image_assets = if pbr_mode {
        crate::gltf_materials::images(&root, buffers, external_images)?
    } else {
        vec![]
    };
    let image_ids = image_assets
        .iter()
        .map(crate::textures::ImageAsset::content_id)
        .collect::<Result<Vec<_>>>()?;
    for image in image_assets {
        commands.push(Command::PutImage { image });
    }
    let mut materials = vec![];
    for (i, m) in array(&root, "materials")?.iter().enumerate() {
        if pbr_mode
            && (!m.is_object()
                || m.get("pbrMetallicRoughness")
                    .is_some_and(|v| !v.is_object()))
        {
            return Err(bad("material and PBR properties must be objects"));
        }
        if !pbr_mode
            && (m.get("extensions").is_some()
                || m.get("normalTexture").is_some()
                || m.get("occlusionTexture").is_some()
                || m.get("emissiveTexture").is_some()
                || m.get("alphaMode").is_some_and(|v| v != "OPAQUE"))
        {
            return Err(unsupported("material texture, extension or alpha mode"));
        }
        let pbr = &m["pbrMetallicRoughness"];
        if !pbr_mode
            && (pbr.get("baseColorTexture").is_some()
                || pbr.get("metallicRoughnessTexture").is_some())
        {
            return Err(unsupported("textured material"));
        }
        let metallic = pbr
            .get("metallicFactor")
            .map(number)
            .transpose()?
            .unwrap_or(1.);
        let roughness = pbr
            .get("roughnessFactor")
            .map(number)
            .transpose()?
            .unwrap_or(1.);
        if !pbr_mode && (metallic != 0. || roughness != 1.) {
            return Err(unsupported(
                "material requires metallic=0 and roughness=1; no silent scalar conversion",
            ));
        }
        let color = vector(pbr.get("baseColorFactor"), [1.; 4])?;
        if !pbr_mode && color[3] != 1. {
            return Err(unsupported("base color alpha"));
        }
        let id = identity(document, &source, "material", i)?;
        let mut material =
            Material::diffuse(id, [color[0] as f32, color[1] as f32, color[2] as f32]);
        material.emission = vector(m.get("emissiveFactor"), [0.; 3])?.map(|n| n as f32);
        if pbr_mode {
            if material.emission.iter().any(|v| *v > 1.) {
                return Err(bad("glTF emissive factor exceeds one"));
            }
            if color.iter().any(|v| !(0.0..=1.).contains(v)) {
                return Err(bad("base color factor range"));
            }
            material.metallic = metallic as f32;
            material.roughness = roughness as f32;
            material.pbr = Some(crate::gltf_materials::surface(&root, &image_ids, m)?);
        }
        material.validate()?;
        commands.push(Command::PutMaterial { material });
        materials.push(id);
        report.source_materials.insert(i, id);
    }
    let default_material = if pbr_mode {
        let id = identity(document, &source, "default-material", 0)?;
        let mut material = Material::diffuse(id, [1.; 3]);
        material.metallic = 1.;
        material.pbr = Some(crate::pbr::Surface::default());
        commands.push(Command::PutMaterial { material });
        Some(id)
    } else {
        None
    };
    let mut meshes = vec![];
    let mut elements = 0usize;
    let mut primitive_count = 0usize;
    let mut hashes = BTreeSet::new();
    for source_mesh in array(&root, "meshes")? {
        if source_mesh.get("weights").is_some() || source_mesh.get("extensions").is_some() {
            return Err(unsupported("mesh weights/extensions"));
        }
        let mut parts = vec![];
        for p in array(source_mesh, "primitives")? {
            primitive_count += 1;
            if primitive_count > 64 {
                return Err(Error::new("budget", "64 primitives per import"));
            }
            if p.get("mode").map(index).transpose()?.unwrap_or(4) != 4
                || p.get("targets").is_some()
                || p.get("extensions").is_some()
            {
                return Err(unsupported("only static TRIANGLES primitives"));
            }
            let attributes = p["attributes"]
                .as_object()
                .ok_or_else(|| bad("primitive attributes missing"))?;
            if attributes.keys().any(|k| {
                !(["POSITION", "NORMAL", "TEXCOORD_0"].contains(&k.as_str())
                    || (pbr_mode && k == "TANGENT"))
            }) {
                return Err(unsupported(
                    "vertex attribute outside POSITION/NORMAL/TEXCOORD_0",
                ));
            }
            let position_accessor = accessor(
                &root,
                buffers,
                index(&p["attributes"]["POSITION"])?,
                "VEC3",
                3,
            )?;
            elements += position_accessor.count;
            if elements > 20000 {
                return Err(Error::new(
                    "budget",
                    "20000 total points and corners per import",
                ));
            }
            let positions = position_accessor.floats::<3>()?;
            let indices: Vec<u32> = if let Some(id) = p.get("indices") {
                let a = accessor(&root, buffers, index(id)?, "SCALAR", 1)?;
                elements += a.count;
                if elements > 20000 {
                    return Err(Error::new(
                        "budget",
                        "20000 total points and corners per import",
                    ));
                }
                if a.component == 5126 {
                    return Err(bad("indices must be unsigned integers"));
                }
                (0..a.count)
                    .map(|i| match a.width {
                        1 => a.component(i, 0)[0] as u32,
                        2 => {
                            u16::from_le_bytes(a.component(i, 0).try_into().expect("width")) as u32
                        }
                        _ => u32::from_le_bytes(a.component(i, 0).try_into().expect("width")),
                    })
                    .collect()
            } else {
                elements += positions.len();
                if elements > 20000 {
                    return Err(Error::new(
                        "budget",
                        "20000 total points and corners per import",
                    ));
                }
                (0..positions.len() as u32).collect()
            };
            if !indices.len().is_multiple_of(3)
                || indices.iter().any(|i| *i as usize >= positions.len())
            {
                return Err(bad("invalid triangle indices"));
            }
            let mut mesh = Mesh::from_polygons(
                Positions::F32(positions),
                &indices
                    .chunks_exact(3)
                    .map(|s| s.to_vec())
                    .collect::<Vec<_>>(),
                &[],
            )?;
            for (key, axes) in [("NORMAL", 3), ("TEXCOORD_0", 2)] {
                if let Some(id) = attributes.get(key) {
                    let a = accessor(
                        &root,
                        buffers,
                        index(id)?,
                        if axes == 3 { "VEC3" } else { "VEC2" },
                        axes,
                    )?;
                    if a.count != mesh.positions.len() {
                        return Err(bad("attribute vertex count mismatch"));
                    }
                    let (semantic, domain, transfer, values) = if axes == 3 {
                        (
                            "normal",
                            Domain::Point,
                            Transfer::Normalize,
                            AttributeValues::Vec3(a.floats::<3>()?),
                        )
                    } else {
                        let uv = a.floats::<2>()?;
                        (
                            "uv",
                            Domain::Corner,
                            Transfer::Linear,
                            AttributeValues::Vec2(
                                indices.iter().map(|i| uv[*i as usize]).collect(),
                            ),
                        )
                    };
                    mesh.attributes.insert(
                        semantic.into(),
                        Attribute {
                            id: Id(if axes == 3 { 1 } else { 2 }),
                            semantic: semantic.into(),
                            domain,
                            transfer,
                            values,
                        },
                    );
                }
            }
            if pbr_mode {
                if let Some(tangent) = attributes.get("TANGENT") {
                    if !attributes.contains_key("NORMAL") {
                        return Err(bad("TANGENT requires NORMAL"));
                    }
                    let a = accessor(&root, buffers, index(tangent)?, "VEC4", 4)?;
                    if a.count != mesh.positions.len() {
                        return Err(bad("tangent count mismatch"));
                    }
                    let values = a.floats::<4>()?;
                    if values.iter().any(|t| {
                        t[3].abs() != 1.
                            || ((t[0] * t[0] + t[1] * t[1] + t[2] * t[2]) - 1.).abs() > 0.01
                    }) {
                        return Err(bad("invalid tangent vector/sign"));
                    }
                    mesh.attributes.insert(
                        "tangent".into(),
                        Attribute {
                            id: Id(3),
                            domain: Domain::Point,
                            semantic: "tangent".into(),
                            transfer: Transfer::Normalize,
                            values: AttributeValues::Vec3(
                                values.iter().map(|t| [t[0], t[1], t[2]]).collect(),
                            ),
                        },
                    );
                    mesh.attributes.insert(
                        "tangent_sign".into(),
                        Attribute {
                            id: Id(4),
                            domain: Domain::Point,
                            semantic: "tangent_sign".into(),
                            transfer: Transfer::Nearest,
                            values: AttributeValues::Scalar(values.iter().map(|t| t[3]).collect()),
                        },
                    );
                }
                if let Some(Attribute {
                    values: AttributeValues::Vec3(normals),
                    ..
                }) = mesh.attributes.get("normal")
                    && normals
                        .iter()
                        .any(|n| (glam::Vec3::from_array(*n).length_squared() - 1.).abs() > 0.01)
                {
                    return Err(bad("normals must be unit vectors"));
                }
                let material = p
                    .get("material")
                    .map(index)
                    .transpose()?
                    .and_then(|i| array(&root, "materials").ok()?.get(i));
                if let Some(m) = material {
                    let surface = crate::gltf_materials::surface(&root, &image_ids, m)?;
                    if surface.bindings().iter().any(Option::is_some)
                        && !attributes.contains_key("TEXCOORD_0")
                    {
                        return Err(bad("textured primitive requires TEXCOORD_0"));
                    }
                }
            }
            let hash = mesh.content_id()?;
            if hashes.insert(hash.clone()) {
                commands.push(Command::PutMesh { mesh });
            }
            let material = if let Some(i) = p.get("material") {
                *materials
                    .get(index(i)?)
                    .ok_or_else(|| bad("material index"))?
            } else {
                default_material
                    .ok_or_else(|| unsupported("explicit supported material required"))?
            };
            parts.push((hash, material));
        }
        if parts.is_empty() {
            return Err(bad("mesh has no primitives"));
        }
        meshes.push(parts);
    }
    let scene = at(
        &root,
        "scenes",
        root.get("scene").map(index).transpose()?.unwrap_or(0),
    )?;
    let mut queue: Vec<(usize, Option<Id>)> = array(scene, "nodes")?
        .iter()
        .map(|v| Ok((index(v)?, None)))
        .collect::<Result<_>>()?;
    let mut visited = BTreeSet::new();
    while let Some((i, parent)) = queue.pop() {
        if !visited.insert(i) {
            return Err(bad("cycle, duplicate root or multiple parents"));
        }
        let node = nodes.get(i).ok_or_else(|| bad("node index"))?;
        if !node.is_object() {
            return Err(bad("node must be an object"));
        }
        if ["skin", "weights", "camera", "extensions"]
            .iter()
            .any(|k| node.get(k).is_some() && !(pbr_mode && *k == "camera"))
        {
            return Err(unsupported("node skin/weights/camera/extensions"));
        }
        let id = identity(document, &source, "node", i)?;
        report.source_nodes.insert(i, id);
        if pbr_mode && let Some(camera) = node.get("camera") {
            commands.push(Command::SetCamera {
                entity: id,
                lens: crate::gltf_materials::lens(at(&root, "cameras", index(camera)?)?)?,
            });
        }
        commands.push(Command::CreateEntity {
            entity: Entity {
                id,
                name: node
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("glTF node")
                    .into(),
                parent,
                mesh: None,
                material: None,
                transform: transform(node)?,
            },
        });
        if let Some(m) = node.get("mesh") {
            for (part, (hash, material)) in meshes
                .get(index(m)?)
                .ok_or_else(|| bad("mesh index"))?
                .iter()
                .enumerate()
            {
                let child = identity(document, &source, &format!("node:{i}:primitive"), part)?;
                commands.push(Command::CreateEntity {
                    entity: Entity {
                        id: child,
                        name: format!("primitive {part}"),
                        parent: Some(id),
                        mesh: Some(hash.clone()),
                        material: Some(*material),
                        transform: Transform::default(),
                    },
                });
                report.instances += 1;
            }
        }
        for child in array(node, "children")? {
            queue.push((index(child)?, Some(id)));
        }
        if commands.len() > 256 {
            return Err(Error::new(
                "budget",
                "import transaction exceeds 256 commands",
            ));
        }
    }
    if report.instances == 0 {
        return Err(bad("selected scene has no mesh instances"));
    }
    report.unique_meshes = hashes.len();
    Ok(Imported { commands, report })
}
