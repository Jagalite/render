//! Version293 static conversion. Independent authored semantics, no source execution.
use super::reader::{File, Record};
use crate::{cameras::Lens, document::*, geometry::*, *};
use glam::{DAffine3, DMat3, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "blend-293-static-v1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub scene: String,
    pub meters_per_unit: f64,
    pub allow_principled_approximation: bool,
    pub allow_point_light_approximation: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub profile: String,
    pub source_digest: String,
    pub source_asset: String,
    pub source_bytes: u64,
    pub source_asset_json_bytes: u64,
    pub version: u16,
    pub pointer_bits: u8,
    pub byte_order: String,
    pub policy: Policy,
    pub source_unit_scale: f64,
    pub entities: BTreeMap<String, Id>,
    pub materials: BTreeMap<String, Id>,
    pub selected_camera: Option<Id>,
    pub selected_light: Option<Id>,
    pub losses: Vec<String>,
}
pub struct Imported {
    pub commands: Vec<Command>,
    pub settings: render::Settings,
    pub report: Report,
}
fn bad(message: impl Into<String>) -> Error {
    Error::new("blend", message).at("import")
}
fn unsupported(feature: &str) -> Error {
    Error::new("unsupported_blend", format!("{PROFILE}: {feature}"))
        .at("import")
        .with_context("feature", feature)
}
fn check(cancel: &mut impl FnMut() -> bool) -> Result<()> {
    if cancel() {
        Err(Error::new(
            "cancelled",
            "blend import cancelled before publication",
        ))
    } else {
        Ok(())
    }
}
fn ptr(r: Record<'_, '_>, field: &str) -> Result<u64> {
    r.field(field)?.pointer(0)
}
fn int(r: Record<'_, '_>, field: &str) -> Result<i64> {
    r.field(field)?.integer(0)
}
fn float(r: Record<'_, '_>, field: &str) -> Result<f64> {
    r.field(field)?.f32(0).map(f64::from)
}
fn vec<const N: usize>(r: Record<'_, '_>, field: &str) -> Result<[f64; N]> {
    let v = r.field(field)?;
    let mut out = [0.; N];
    for (i, x) in out.iter_mut().enumerate() {
        *x = f64::from(v.f32(i)?);
    }
    Ok(out)
}
fn embedded<'f, 'a>(r: Record<'f, 'a>, field: &str, kind: &str) -> Result<Record<'f, 'a>> {
    r.field(field)?.record(kind, 0)
}
fn name(r: Record<'_, '_>) -> Result<String> {
    let id = embedded(r, "id", "ID")?;
    for f in ["lib", "override_library"] {
        if ptr(id, f)? != 0 {
            return Err(unsupported(f));
        }
    }
    let text = id.field("name")?.text()?;
    if text.len() < 3 || !text.as_bytes()[..2].iter().all(u8::is_ascii_uppercase) {
        return Err(bad("empty datablock name"));
    }
    Ok(text[2..].to_owned())
}
fn none(r: Record<'_, '_>, fields: &[&str]) -> Result<()> {
    for f in fields {
        if ptr(r, f)? != 0 {
            return Err(unsupported(f));
        }
    }
    Ok(())
}
fn list(
    file: &File<'_>,
    head: Record<'_, '_>,
    kind: &str,
    cancel: &mut impl FnMut() -> bool,
) -> Result<Vec<u64>> {
    let mut token = ptr(head, "first")?;
    let last = ptr(head, "last")?;
    let mut previous = 0;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    while token != 0 {
        check(cancel)?;
        if !seen.insert(token) {
            return Err(bad("cyclic source list"));
        }
        if out.len() == 256 {
            return Err(Error::new("budget", "source list exceeds 256 entries"));
        }
        let r = file.single(token, kind)?;
        if file.block(token)?.count != 1 || ptr(r, "prev")? != previous {
            return Err(bad("source list count or back-link mismatch"));
        }
        out.push(token);
        previous = token;
        token = ptr(r, "next")?;
    }
    if previous != last {
        return Err(bad("source list tail mismatch"));
    }
    Ok(out)
}
fn empty_lists(r: Record<'_, '_>, fields: &[&str]) -> Result<()> {
    for field in fields {
        let h = embedded(r, field, "ListBase")?;
        if ptr(h, "first")? != 0 || ptr(h, "last")? != 0 {
            return Err(unsupported(field));
        }
    }
    Ok(())
}
fn stable(document: Id, source: &str, kind: &str, label: &str) -> Result<Id> {
    let hash = digest(&canonical(&(document, source, kind, label))?);
    Ok(Id(
        u128::from_str_radix(&hash[7..39], 16).expect("digest hexadecimal")
    ))
}
fn count(r: Record<'_, '_>, field: &str, max: usize) -> Result<usize> {
    let n = int(r, field)?;
    if n < 0 || n as u64 > max as u64 {
        Err(Error::new(
            "budget",
            format!("source {field} count exceeds {max}"),
        ))
    } else {
        Ok(n as usize)
    }
}
fn matrix(r: Record<'_, '_>, field: &str) -> Result<DAffine3> {
    let a = vec::<16>(r, field)?;
    if a[3] != 0. || a[7] != 0. || a[11] != 0. || a[15] != 1. {
        return Err(bad("source parent inverse is not affine"));
    }
    Ok(DAffine3::from_mat3_translation(
        DMat3::from_cols_array(&[a[0], a[1], a[2], a[4], a[5], a[6], a[8], a[9], a[10]]),
        DVec3::new(a[12], a[13], a[14]),
    ))
}
fn transform(r: Record<'_, '_>, units: f64) -> Result<Transform> {
    if int(r, "partype")? != 0 || int(r, "transflag")? != 0 {
        return Err(unsupported("object parenting/instancing mode"));
    }
    // Delta channels remain a separate unsupported profile, never silently ignored.
    if vec::<3>(r, "dloc")? != [0.; 3]
        || vec::<3>(r, "drot")? != [0.; 3]
        || vec::<4>(r, "dquat")? != [1., 0., 0., 0.]
        || float(r, "drotAngle")? != 0.
        || vec::<3>(r, "dscale")? != [1.; 3]
    {
        return Err(unsupported("delta transforms"));
    }
    let angles = vec::<3>(r, "rot")?;
    let rotation = match int(r, "rotmode")? {
        // Source XYZ applies X then Y then Z to column vectors. The native
        // ordered product is Z,Y,X with angles in that same order.
        1 => TransformOp::RotationRadians {
            angles: [angles[2], angles[1], angles[0]],
            order: RotationOrder::Zyx,
        },
        0 => {
            let q = vec::<4>(r, "quat")?;
            let q = DQuat::from_xyzw(q[1], q[2], q[3], q[0]);
            if (q.length_squared() - 1.).abs() > 1e-6 {
                return Err(bad("nonunit source quaternion"));
            }
            TransformOp::Quaternion(q.normalize().to_array())
        }
        _ => {
            return Err(unsupported(
                "rotation order; first profile supports XYZ or quaternion",
            ));
        }
    };
    let mut parent_inverse = if ptr(r, "parent")? == 0 {
        DAffine3::IDENTITY
    } else {
        matrix(r, "parentinv")?
    };
    parent_inverse.translation *= units;
    let t = Transform {
        columns: [
            parent_inverse.matrix3.x_axis.to_array(),
            parent_inverse.matrix3.y_axis.to_array(),
            parent_inverse.matrix3.z_axis.to_array(),
            parent_inverse.translation.to_array(),
        ],
        operations: vec![
            TransformOp::TranslationMeters(vec::<3>(r, "loc")?.map(|x| x * units)),
            rotation,
            TransformOp::Scale(vec(r, "size")?),
        ],
    };
    t.inverse()?;
    Ok(t)
}
fn mesh(
    file: &File<'_>,
    token: u64,
    units: f64,
    cancel: &mut impl FnMut() -> bool,
) -> Result<Mesh> {
    let r = file.single(token, "Mesh")?;
    name(r)?;
    none(r, &["adt", "ipo", "key", "texcomesh", "dvert"])?;
    if int(r, "totface")? != 0 {
        return Err(unsupported("legacy tessellated faces"));
    }
    let nv = count(r, "totvert", 16384)?;
    let ne = count(r, "totedge", 32768)?;
    let nf = count(r, "totpoly", 8192)?;
    let nl = count(r, "totloop", 32768)?;
    if nv == 0 || nf == 0 || nl < 3 {
        return Err(unsupported("empty or loose-only mesh"));
    }
    for (field, count) in [("mvert", nv), ("medge", ne), ("mpoly", nf), ("mloop", nl)] {
        if file.block(ptr(r, field)?)?.count != count {
            return Err(bad("mesh array element count mismatch"));
        }
    }
    let mut positions = Vec::with_capacity(nv);
    for i in 0..nv {
        check(cancel)?;
        positions
            .push(vec::<3>(file.record(ptr(r, "mvert")?, "MVert", i)?, "co")?.map(|x| x * units));
    }
    let mut edges = Vec::with_capacity(ne);
    for i in 0..ne {
        check(cancel)?;
        let e = file.record(ptr(r, "medge")?, "MEdge", i)?;
        edges.push([
            count(e, "v1", nv - 1)? as u32,
            count(e, "v2", nv - 1)? as u32,
        ]);
    }
    let mut vertices = Vec::with_capacity(nl);
    let mut loop_edges = Vec::with_capacity(nl);
    for i in 0..nl {
        check(cancel)?;
        let l = file.record(ptr(r, "mloop")?, "MLoop", i)?;
        let v = count(l, "v", nv - 1)? as u32;
        let e = count(l, "e", ne.saturating_sub(1))?;
        if edges.get(e).is_none_or(|edge| !edge.contains(&v)) {
            return Err(bad("loop edge does not contain its vertex"));
        }
        vertices.push(v);
        loop_edges.push(e);
    }
    let mut faces = Vec::with_capacity(nf);
    let mut used = 0;
    for i in 0..nf {
        check(cancel)?;
        let p = file.record(ptr(r, "mpoly")?, "MPoly", i)?;
        let start = count(p, "loopstart", nl)?;
        let n = count(p, "totloop", nl)?;
        if start != used || n < 3 || start + n > nl {
            return Err(bad("polygon loops must form a disjoint complete partition"));
        }
        if int(p, "mat_nr")? != 0 {
            return Err(unsupported("multiple polygon material slots"));
        }
        if int(p, "flag")? & 1 != 0 {
            return Err(unsupported("smooth polygon normals"));
        }
        for j in start..start + n {
            let edge = edges[loop_edges[j]];
            let next = vertices[start + (j - start + 1) % n];
            if !edge.contains(&next) || next == vertices[j] {
                return Err(bad("loop edge does not join adjacent polygon vertices"));
            }
        }
        faces.push(vertices[start..start + n].to_vec());
        used += n;
    }
    if used != nl {
        return Err(bad("unowned polygon loops"));
    }
    let mut mesh = Mesh::from_polygons(Positions::F64(positions), &faces, &edges)?;
    // Preserve named corner UVs. Other custom layers are explicitly reported as
    // preserved source-only metadata by the import report; no shader reads them.
    let cd = embedded(r, "ldata", "CustomData")?;
    let layers = count(cd, "totlayer", 64)?;
    if layers > 0 && file.block(ptr(cd, "layers")?)?.count != layers {
        return Err(bad("custom layer count mismatch"));
    }
    for i in 0..layers {
        check(cancel)?;
        let layer = file.record(ptr(cd, "layers")?, "CustomDataLayer", i)?;
        let kind = int(layer, "type")?;
        if kind == 16 {
            let name = layer.field("name")?.text()?.to_owned();
            if name.is_empty() || mesh.attributes.contains_key(&name) {
                return Err(bad("empty or duplicate UV layer name"));
            }
            if file.block(ptr(layer, "data")?)?.count != nl {
                return Err(bad("UV array count mismatch"));
            }
            let mut uv = Vec::with_capacity(nl);
            for j in 0..nl {
                check(cancel)?;
                uv.push(
                    vec::<2>(file.record(ptr(layer, "data")?, "MLoopUV", j)?, "uv")?
                        .map(|x| x as f32),
                );
            }
            let id = stable(Id(0), "blend-corner-uv", "attribute", &name)?;
            mesh.attributes.insert(
                name,
                Attribute {
                    id,
                    domain: Domain::Corner,
                    semantic: "uv".into(),
                    transfer: Transfer::Linear,
                    values: AttributeValues::Vec2(uv),
                },
            );
        } else if kind != 26 {
            return Err(unsupported("non-UV render corner attributes"));
        }
    }
    mesh.validate()?;
    mesh.triangles()?;
    Ok(mesh)
}
fn sockets<'f, 'a>(
    file: &'f File<'a>,
    node: Record<'f, 'a>,
    direction: &str,
    cancel: &mut impl FnMut() -> bool,
) -> Result<BTreeMap<String, (u64, Record<'f, 'a>)>> {
    let mut out = BTreeMap::new();
    for token in list(
        file,
        embedded(node, direction, "ListBase")?,
        "bNodeSocket",
        cancel,
    )? {
        let r = file.single(token, "bNodeSocket")?;
        let name = r.field("identifier")?.text()?.to_owned();
        if out.insert(name, (token, r)).is_some() {
            return Err(bad("duplicate node socket identifier"));
        }
    }
    Ok(out)
}
fn shader<'f, 'a>(
    file: &'f File<'a>,
    tree: u64,
    output_kind: &str,
    shader_kind: &str,
    cancel: &mut impl FnMut() -> bool,
) -> Result<BTreeMap<String, (u64, Record<'f, 'a>)>> {
    let tree = file.single(tree, "bNodeTree")?;
    name(tree)?;
    none(tree, &["adt"])?;
    let nodes = list(file, embedded(tree, "nodes", "ListBase")?, "bNode", cancel)?;
    let links = list(
        file,
        embedded(tree, "links", "ListBase")?,
        "bNodeLink",
        cancel,
    )?;
    if nodes.len() != 2 || links.len() != 1 {
        return Err(unsupported(
            "node graph requires one constant shader and one output",
        ));
    }
    let mut output = None;
    let mut shader = None;
    for token in nodes {
        let r = file.single(token, "bNode")?;
        none(r, &["id", "storage"])?;
        empty_lists(r, &["internal_links"])?;
        if int(r, "flag")? & 512 != 0 {
            return Err(unsupported("muted shader node"));
        }
        match r.field("idname")?.text()? {
            kind if kind == output_kind => {
                if output.replace((token, r)).is_some() {
                    return Err(bad("duplicate shader output"));
                }
            }
            kind if kind == shader_kind => {
                if shader.replace((token, r)).is_some() {
                    return Err(bad("duplicate constant shader"));
                }
            }
            _ => return Err(unsupported("shader node type")),
        }
    }
    let (out_token, out) = output.ok_or_else(|| unsupported("missing output node"))?;
    let (shader_token, shader) = shader.ok_or_else(|| unsupported("missing constant shader"))?;
    if int(out, "custom1")? != 0 {
        return Err(unsupported("engine-specific shader output target"));
    }
    let inputs = sockets(file, shader, "inputs", cancel)?;
    for (_, r) in inputs.values() {
        if ptr(*r, "link")? != 0 {
            return Err(unsupported("linked constant shader input"));
        }
    }
    let outputs = sockets(file, shader, "outputs", cancel)?;
    let destinations = sockets(file, out, "inputs", cancel)?;
    let link = file.single(links[0], "bNodeLink")?;
    let output_name = if shader_kind == "ShaderNodeBackground" {
        "Background"
    } else {
        "BSDF"
    };
    let source = outputs
        .get(output_name)
        .ok_or_else(|| bad("shader output socket absent"))?;
    let target = destinations
        .get("Surface")
        .ok_or_else(|| bad("surface output input absent"))?;
    if ptr(link, "fromnode")? != shader_token
        || ptr(link, "tonode")? != out_token
        || ptr(link, "fromsock")? != source.0
        || ptr(link, "tosock")? != target.0
        || int(link, "flag")? != 2
        || ptr(target.1, "link")? != links[0]
    {
        return Err(bad("shader link identity or validity mismatch"));
    }
    for (name, (_, r)) in &destinations {
        if name != "Surface" && ptr(*r, "link")? != 0 {
            return Err(unsupported("volume or displacement shader"));
        }
    }
    Ok(inputs)
}
fn socket_float(
    file: &File<'_>,
    inputs: &BTreeMap<String, (u64, Record<'_, '_>)>,
    key: &str,
) -> Result<f32> {
    let r = inputs
        .get(key)
        .ok_or_else(|| bad(format!("missing {key} socket")))?
        .1;
    if int(r, "type")? != 0 {
        return Err(bad("float socket type mismatch"));
    }
    file.single(ptr(r, "default_value")?, "bNodeSocketValueFloat")?
        .field("value")?
        .f32(0)
}
fn socket_color(
    file: &File<'_>,
    inputs: &BTreeMap<String, (u64, Record<'_, '_>)>,
    key: &str,
) -> Result<[f32; 4]> {
    let r = inputs
        .get(key)
        .ok_or_else(|| bad(format!("missing {key} socket")))?
        .1;
    if int(r, "type")? != 2 {
        return Err(bad("color socket type mismatch"));
    }
    Ok(vec::<4>(
        file.single(ptr(r, "default_value")?, "bNodeSocketValueRGBA")?,
        "value",
    )?
    .map(|x| x as f32))
}
fn material(
    file: &File<'_>,
    token: u64,
    id: Id,
    policy: &Policy,
    cancel: &mut impl FnMut() -> bool,
) -> Result<Material> {
    if !policy.allow_principled_approximation {
        return Err(unsupported(
            "Principled conversion requires explicit approximation policy",
        ));
    }
    let r = file.single(token, "Material")?;
    name(r)?;
    none(r, &["adt", "ipo", "gp_style"])?;
    if int(r, "use_nodes")? != 1 {
        return Err(unsupported("material requires constant Principled nodes"));
    }
    if int(r, "blend_method")? != 0 || int(r, "blend_shadow")? != 1 || int(r, "blend_flag")? != 0 {
        return Err(unsupported("material blend/shadow/culling mode"));
    }
    let inputs = shader(
        file,
        ptr(r, "nodetree")?,
        "ShaderNodeOutputMaterial",
        "ShaderNodeBsdfPrincipled",
        cancel,
    )?;
    let expected = [
        "Base Color",
        "Subsurface",
        "Subsurface Radius",
        "Subsurface Color",
        "Metallic",
        "Specular",
        "Specular Tint",
        "Roughness",
        "Anisotropic",
        "Anisotropic Rotation",
        "Sheen",
        "Sheen Tint",
        "Clearcoat",
        "Clearcoat Roughness",
        "IOR",
        "Transmission",
        "Transmission Roughness",
        "Emission",
        "Emission Strength",
        "Alpha",
        "Normal",
        "Clearcoat Normal",
        "Tangent",
    ];
    if inputs.len() != expected.len() || expected.iter().any(|name| !inputs.contains_key(*name)) {
        return Err(unsupported("Principled socket set"));
    }
    for key in [
        "Subsurface",
        "Specular Tint",
        "Anisotropic",
        "Sheen",
        "Clearcoat",
        "Transmission",
    ] {
        if socket_float(file, &inputs, key)? != 0. {
            return Err(unsupported(key));
        }
    }
    if socket_float(file, &inputs, "Specular")? != 0.5
        || socket_float(file, &inputs, "Alpha")? != 1.
    {
        return Err(unsupported("nondefault specular or alpha"));
    }
    let color = socket_color(file, &inputs, "Base Color")?;
    let emission = socket_color(file, &inputs, "Emission")?;
    if emission[..3].iter().any(|x| *x < 0.) {
        return Err(bad("negative source material emission color"));
    }
    if color[3] != 1. {
        return Err(unsupported("base color alpha"));
    }
    let strength = socket_float(file, &inputs, "Emission Strength")?;
    if strength < 0. {
        return Err(bad("negative emission strength"));
    }
    let m = Material {
        id,
        base_color: [color[0], color[1], color[2]],
        emission: [
            emission[0] * strength,
            emission[1] * strength,
            emission[2] * strength,
        ],
        roughness: socket_float(file, &inputs, "Roughness")?,
        metallic: socket_float(file, &inputs, "Metallic")?,
        texture: None,
        pbr: Some(pbr::Surface {
            double_sided: true,
            ..Default::default()
        }),
    };
    m.validate()?;
    Ok(m)
}
fn lens(file: &File<'_>, token: u64, aspect: f64, units: f64) -> Result<Lens> {
    let r = file.single(token, "Camera")?;
    name(r)?;
    none(r, &["adt", "ipo"])?;
    if float(r, "shiftx")? != 0. || float(r, "shifty")? != 0. {
        return Err(unsupported("camera shift"));
    }
    let dof = embedded(r, "dof", "CameraDOFSettings")?;
    if int(dof, "flag")? & 1 != 0 {
        return Err(unsupported("camera depth of field"));
    }
    let near = float(r, "clipsta")? * units;
    let far = float(r, "clipend")? * units;
    let fit = int(r, "sensor_fit")?;
    let horizontal = match fit {
        0 => aspect >= 1.,
        1 => true,
        2 => false,
        _ => return Err(bad("camera sensor fit")),
    };
    let lens = match int(r, "type")? {
        0 => {
            let sensor = if fit == 2 {
                float(r, "sensor_y")?
            } else {
                float(r, "sensor_x")?
            };
            let focal = float(r, "lens")?;
            if sensor <= 0. || focal <= 0. {
                return Err(bad("camera sensor/focal length"));
            }
            let tan = sensor / (2. * focal) / if horizontal { aspect } else { 1. };
            Lens::Perspective {
                vertical_fov_radians: 2. * libm::atan(tan),
                aspect_ratio: Some(aspect),
                near,
                far: Some(far),
            }
        }
        1 => {
            let scale = float(r, "ortho_scale")? * units;
            let (xmag, ymag) = if horizontal {
                (scale / 2., scale / (2. * aspect))
            } else {
                (scale * aspect / 2., scale / 2.)
            };
            Lens::Orthographic {
                xmag,
                ymag,
                near,
                far,
            }
        }
        _ => return Err(unsupported("panoramic camera")),
    };
    lens.validate()?;
    Ok(lens)
}
fn members(
    file: &File<'_>,
    scene: Record<'_, '_>,
    cancel: &mut impl FnMut() -> bool,
) -> Result<BTreeSet<u64>> {
    let mut out = BTreeSet::new();
    let mut pending = vec![(ptr(scene, "master_collection")?, BTreeSet::new())];
    let mut visited = BTreeSet::new();
    while let Some((token, mut path)) = pending.pop() {
        check(cancel)?;
        if !path.insert(token) {
            return Err(bad("cyclic collection hierarchy"));
        }
        if !visited.insert(token) {
            continue;
        }
        if visited.len() > 64 {
            return Err(Error::new("budget", "at most 64 source collections"));
        }
        let c = file.single(token, "Collection")?;
        name(c)?;
        if int(c, "flag")? & !32 != 0 {
            return Err(unsupported("collection visibility or instancing flags"));
        }
        for child in list(
            file,
            embedded(c, "children", "ListBase")?,
            "CollectionChild",
            cancel,
        )? {
            pending.push((
                ptr(file.single(child, "CollectionChild")?, "collection")?,
                path.clone(),
            ));
        }
        for object in list(
            file,
            embedded(c, "gobject", "ListBase")?,
            "CollectionObject",
            cancel,
        )? {
            out.insert(ptr(file.single(object, "CollectionObject")?, "ob")?);
        }
        if out.len() > 64 {
            return Err(Error::new("budget", "at most 64 source objects"));
        }
    }
    // The first profile selects an unfiltered single view layer. UI runtime flags
    // do not replace authored collection visibility.
    let layers = list(
        file,
        embedded(scene, "view_layers", "ListBase")?,
        "ViewLayer",
        cancel,
    )?;
    if layers.len() != 1 {
        return Err(unsupported("multiple or missing view layers"));
    }
    let layer = file.single(layers[0], "ViewLayer")?;
    none(layer, &["mat_override"])?;
    if int(layer, "flag")? & 1 == 0 {
        return Err(unsupported("disabled render view layer"));
    }
    let mut pending = list(
        file,
        embedded(layer, "layer_collections", "ListBase")?,
        "LayerCollection",
        cancel,
    )?;
    let mut seen = BTreeSet::new();
    let mut layer_collections = BTreeSet::new();
    while let Some(token) = pending.pop() {
        check(cancel)?;
        if !seen.insert(token) || seen.len() > 64 {
            return Err(bad("cyclic or excessive layer collections"));
        }
        let l = file.single(token, "LayerCollection")?;
        if int(l, "flag")? != 0 {
            return Err(unsupported("view layer collection exclusions/holdouts"));
        }
        layer_collections.insert(ptr(l, "collection")?);
        pending.extend(list(
            file,
            embedded(l, "layer_collections", "ListBase")?,
            "LayerCollection",
            cancel,
        )?);
    }
    if layer_collections != visited {
        return Err(bad("view layer and scene collection membership differ"));
    }
    Ok(out)
}
pub fn import(
    bytes: &[u8],
    document: Id,
    policy: &Policy,
    mut settings: render::Settings,
    mut cancel: impl FnMut() -> bool,
) -> Result<Imported> {
    check(&mut cancel)?;
    settings.validate()?;
    if policy.scene.is_empty()
        || policy.scene.len() > 64
        || !policy.meters_per_unit.is_finite()
        || policy.meters_per_unit <= 0.
    {
        return Err(bad("invalid scene name or explicit metric scale"));
    }
    let file = File::parse(bytes, &mut cancel)?;
    if file.header.version != 293 {
        return Err(unsupported("semantic file version; expected293"));
    }
    let mut scenes = Vec::new();
    for b in &file.blocks {
        check(&mut cancel)?;
        if b.code == *b"SC\0\0" {
            let r = file.single(b.token, "Scene")?;
            if name(r)? == policy.scene {
                scenes.push(r);
            }
        }
    }
    if scenes.len() != 1 {
        return Err(bad("scene name must select exactly one Scene datablock"));
    }
    let scene = scenes[0];
    none(scene, &["adt", "set", "gpd", "rigidbody_world"])?;
    if int(scene, "use_nodes")? != 0 {
        return Err(unsupported("scene compositor"));
    }
    if ptr(scene, "ed")? != 0 {
        empty_lists(file.single(ptr(scene, "ed")?, "Editing")?, &["seqbase"])?;
    }
    let source_unit_scale = float(embedded(scene, "unit", "UnitSettings")?, "scale_length")?;
    if source_unit_scale <= 0. {
        return Err(bad("nonpositive scene unit scale"));
    }
    let source = digest(bytes);
    let source_asset = source::Asset::Blend {
        version: 293,
        bytes: bytes.to_vec(),
    };
    let mut report=Report{profile:PROFILE.into(),source_digest:source.clone(),source_asset:source_asset.content_id()?,source_bytes:bytes.len() as u64,source_asset_json_bytes:canonical(&source_asset)?.len() as u64,version:293,pointer_bits:(file.header.pointer_bytes*8) as u8,byte_order:format!("{:?}",file.header.endian).to_lowercase(),policy:policy.clone(),source_unit_scale,entities:BTreeMap::new(),materials:BTreeMap::new(),selected_camera:None,selected_light:None,losses:vec!["Exact original container retained as inert provenance; scripts, UI, custom properties and non-render mesh metadata are source-only. Exporting that source does not incorporate native edits.".into(),"Source render engine, sampling, display transform and compositor outputs are not reproduced; native caller sampling/output settings apply.".into(),"The explicit meters_per_unit policy controls all geometry, translations and clipping; source unit scale is reported separately.".into()]};
    let tokens = members(&file, scene, &mut cancel)?;
    let mut by_name = BTreeMap::new();
    let mut ids = BTreeMap::new();
    for token in &tokens {
        let r = file.single(*token, "Object")?;
        let label = name(r)?;
        let id = stable(document, &source, "object", &label)?;
        if by_name.insert(label.clone(), *token).is_some() {
            return Err(bad("duplicate object name"));
        }
        ids.insert(*token, id);
        report.entities.insert(label, id);
    }
    let render = embedded(scene, "r", "RenderData")?;
    let xasp = float(render, "xasp")?;
    let yasp = float(render, "yasp")?;
    let width = int(render, "xsch")?;
    let height = int(render, "ysch")?;
    if width <= 0 || height <= 0 || xasp <= 0. || yasp <= 0. {
        return Err(bad(
            "source render dimensions and pixel aspect must be positive",
        ));
    }
    let aspect = xasp * width as f64 / (yasp * height as f64);
    if !aspect.is_finite() || aspect <= 0. {
        return Err(bad("invalid source camera aspect"));
    }
    let mut commands = vec![Command::PutSource {
        asset: source_asset,
    }];
    let mut native = Snapshot::empty(document);
    let mut lights = Vec::new();
    let mut material_tokens = BTreeMap::new();
    let mut mesh_tokens = BTreeMap::<u64, String>::new();
    for (label, token) in by_name {
        check(&mut cancel)?;
        let r = file.single(token, "Object")?;
        none(
            r,
            &[
                "adt",
                "track",
                "proxy",
                "proxy_group",
                "proxy_from",
                "ipo",
                "action",
                "poselib",
                "pose",
                "gpd",
                "soft",
                "dup_group",
                "fluidsimSettings",
                "rigidbody_object",
                "rigidbody_constraint",
            ],
        )?;
        empty_lists(
            r,
            &[
                "modifiers",
                "greasepencil_modifiers",
                "shader_fx",
                "constraints",
                "constraintChannels",
                "effect",
                "nlastrips",
                "hooks",
                "particlesystem",
            ],
        )?;
        if int(r, "mode")? != 0 {
            return Err(unsupported("object edit/sculpt/pose mode"));
        }
        if int(r, "restrictflag")? != 0 {
            return Err(unsupported("object visibility flags"));
        }
        if ptr(r, "pd")? != 0 {
            let field = file.single(ptr(r, "pd")?, "PartDeflect")?;
            if int(field, "deflect")? != 0 || int(field, "forcefield")? != 0 {
                return Err(unsupported("active collision or force field"));
            }
        }
        let parent = match ptr(r, "parent")? {
            0 => None,
            p => Some(
                *ids.get(&p)
                    .ok_or_else(|| unsupported("parent outside selected scene"))?,
            ),
        };
        let mut entity = Entity {
            id: ids[&token],
            name: label,
            parent,
            mesh: None,
            material: None,
            transform: transform(r, policy.meters_per_unit)?,
        };
        match int(r, "type")? {
            0 => {
                if ptr(r, "data")? != 0 {
                    return Err(unsupported("empty object data"));
                }
            }
            1 => {
                let data = ptr(r, "data")?;
                let key = if let Some(key) = mesh_tokens.get(&data) {
                    key.clone()
                } else {
                    let m = mesh(&file, data, policy.meters_per_unit, &mut cancel)?;
                    let key = m.content_id()?;
                    native
                        .meshes
                        .insert(key.clone(), std::sync::Arc::new(m.clone()));
                    commands.push(Command::PutMesh { mesh: m });
                    mesh_tokens.insert(data, key.clone());
                    key
                };
                let source_mesh = file.single(data, "Mesh")?;
                if int(source_mesh, "totcol")? != 1 || int(r, "totcol")? != 1 {
                    return Err(unsupported("mesh requires one material slot"));
                }
                let block = file.block(ptr(r, "matbits")?)?;
                if block.code != *b"DATA" || block.structure != 0 || block.count != 1 {
                    return Err(bad("material slot flags must be a raw data block"));
                }
                let bits = block.data;
                if bits.len() != 4 || bits[0] > 1 {
                    return Err(bad("object material slot flags"));
                }
                let array = if bits[0] == 1 {
                    ptr(r, "mat")?
                } else {
                    ptr(source_mesh, "mat")?
                };
                let material_token = file.pointer_array(array, 1)?[0];
                let mr = file.single(material_token, "Material")?;
                let label = name(mr)?;
                let mid = stable(document, &source, "material", &label)?;
                if let Some(old) = material_tokens.insert(label.clone(), material_token)
                    && old != material_token
                {
                    return Err(bad("duplicate material datablock name"));
                }
                report.materials.insert(label, mid);
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    native.materials.entry(mid)
                {
                    let mat = material(&file, material_token, mid, policy, &mut cancel)?;
                    entry.insert(mat.clone());
                    commands.push(Command::PutMaterial {
                        material: Box::new(mat),
                    });
                }
                entity.mesh = Some(key);
                entity.material = Some(mid);
            }
            11 => {
                let lens = lens(&file, ptr(r, "data")?, aspect, policy.meters_per_unit)?;
                native.cameras.insert(entity.id, lens.clone());
                commands.push(Command::SetCamera {
                    entity: entity.id,
                    lens,
                });
            }
            10 => lights.push((entity.id, ptr(r, "data")?)),
            _ => return Err(unsupported("object geometry type")),
        }
        native.entities.insert(entity.clone())?;
        commands.push(Command::CreateEntity { entity });
        if commands.len() > 250 {
            return Err(Error::new(
                "budget",
                "blend conversion exceeds transaction command limit",
            ));
        }
    }
    native.version = 6;
    native.validate()?;
    let camera = ptr(scene, "camera")?;
    if camera != 0 {
        let id = *ids
            .get(&camera)
            .ok_or_else(|| bad("selected camera outside scene"))?;
        settings.camera = native.camera(id)?;
        report.selected_camera = Some(id);
    }
    if lights.len() > 1 {
        return Err(unsupported("more than one point light"));
    }
    settings.light.intensity = [0.; 3];
    for (id, token) in lights {
        check(&mut cancel)?;
        let l = file.single(token, "Lamp")?;
        name(l)?;
        none(l, &["adt", "ipo"])?;
        if int(l, "type")? != 0 || int(l, "use_nodes")? != 0 {
            return Err(unsupported("non-point or node-driven light"));
        }
        if !policy.allow_point_light_approximation {
            return Err(unsupported(
                "source light conversion requires explicit point approximation policy",
            ));
        }
        let energy = float(l, "energy")?;
        let radius = float(l, "soft")? * policy.meters_per_unit;
        if energy < 0. || radius < 0. {
            return Err(bad("negative source light power/radius"));
        }
        let color = [float(l, "r")?, float(l, "g")?, float(l, "b")?];
        if color.iter().any(|x| *x < 0.) {
            return Err(bad("negative source light color"));
        }
        settings.light = render::PointLight {
            position: native.world_transform(id)?.translation.to_array(),
            intensity: color.map(|v| (v * energy / (4. * std::f64::consts::PI)) as f32),
        };
        report.selected_light = Some(id);
        report.losses.push(format!("Light converted to one isotropic point using RGB times source power divided by4pi; source radius {radius}m and source engine shadow controls are not evaluated. Lighting is a static render-setting value, not bound to later entity edits."));
    }
    settings.environment = [0.; 3];
    if ptr(scene, "world")? != 0 {
        let w = file.single(ptr(scene, "world")?, "World")?;
        name(w)?;
        none(w, &["adt", "ipo"])?;
        if int(w, "use_nodes")? != 1 {
            return Err(unsupported("world requires constant Background nodes"));
        }
        let inputs = shader(
            &file,
            ptr(w, "nodetree")?,
            "ShaderNodeOutputWorld",
            "ShaderNodeBackground",
            &mut cancel,
        )?;
        if inputs.len() != 2 {
            return Err(unsupported("background inputs"));
        }
        let color = socket_color(&file, &inputs, "Color")?;
        if color[..3].iter().any(|x| *x < 0.) {
            return Err(bad("negative source environment color"));
        }
        let strength = socket_float(&file, &inputs, "Strength")?;
        if strength < 0. {
            return Err(bad("negative environment strength"));
        }
        settings.environment = [
            color[0] * strength,
            color[1] * strength,
            color[2] * strength,
        ];
    }
    if !report.materials.is_empty() {
        report.losses.push("Constant Principled base color, metallic, roughness and emission use the native single-scattering metallic/roughness BRDF; source distribution/subsurface implementation and exact renderer parity are not claimed.".into());
    }
    settings.validate()?;
    check(&mut cancel)?;
    commands.push(Command::SetRenderSettings {
        settings: settings.clone(),
    });
    Ok(Imported {
        commands,
        settings,
        report,
    })
}
