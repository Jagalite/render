//! Bounded evaluated PBR GLB export. No filesystem, URI resolution or mutation.
use crate::{animation, document::Snapshot, render::*, textures::*, *};
use glam::{DAffine3, DMat4};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub clip: Id,
    pub time: Time,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub allow_approximations: bool,
    pub max_bytes: u64,
    pub max_vertices: u32,
    pub max_position_error_meters: f64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            allow_approximations: false,
            max_bytes: gltf_scene::MAX_INPUT as u64,
            max_vertices: 10000,
            max_position_error_meters: 1e-5,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub revision: String,
    pub at: Option<Sample>,
    pub policy: Policy,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub profile: String,
    pub authored_revision: String,
    pub evaluated_revision: String,
    pub animation: Option<animation::Receipt>,
    pub source_entities: BTreeMap<Id, u32>,
    pub meshes: usize,
    pub materials: usize,
    pub images: usize,
    pub exported_vertices: usize,
    pub max_position_error_meters: f64,
    pub bytes: usize,
    pub output_digest: String,
    pub losses: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exported {
    pub glb: Vec<u8>,
    pub report: Report,
}
fn unsupported(s: &str) -> Error {
    Error::new("unsupported_gltf", s).at("export")
}
fn budget(s: &str) -> Error {
    Error::new("budget", s).at("export")
}
fn check(cancelled: &mut impl FnMut() -> bool) -> Result<()> {
    if cancelled() {
        Err(Error::new("cancelled", "GLB export cancelled"))
    } else {
        Ok(())
    }
}
struct Builder {
    limit: usize,
    binary: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
    images: Vec<Value>,
    image_ids: BTreeMap<String, usize>,
    image_pixels: u64,
    samplers: Vec<Value>,
    textures: Vec<Value>,
    texture_ids: BTreeMap<Vec<u8>, usize>,
}
impl Builder {
    fn view(&mut self, bytes: &[u8], vertex: bool) -> Result<usize> {
        let padding = (4 - self.binary.len() % 4) % 4;
        if self
            .binary
            .len()
            .checked_add(padding)
            .and_then(|n| n.checked_add(bytes.len()))
            .is_none_or(|n| n > self.limit)
        {
            return Err(budget("GLB binary budget"));
        }
        self.binary.resize(self.binary.len() + padding, 0);
        let mut view = json!({"buffer":0,"byteOffset":self.binary.len(),"byteLength":bytes.len()});
        if vertex {
            view["target"] = json!(34962);
        }
        self.views.push(view);
        self.binary.extend_from_slice(bytes);
        Ok(self.views.len() - 1)
    }
    fn accessor<const N: usize>(
        &mut self,
        values: &[[f32; N]],
        kind: &str,
        position: bool,
    ) -> Result<usize> {
        if values.is_empty() || values.iter().flatten().any(|v| !v.is_finite()) {
            return Err(unsupported("empty or nonfinite vertex array"));
        }
        let bytes: Vec<u8> = values
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let view = self.view(&bytes, true)?;
        let mut a =
            json!({"bufferView":view,"componentType":5126,"type":kind,"count":values.len()});
        if position {
            let mut min = [f32::INFINITY; N];
            let mut max = [f32::NEG_INFINITY; N];
            for row in values {
                for i in 0..N {
                    min[i] = min[i].min(row[i]);
                    max[i] = max[i].max(row[i]);
                }
            }
            a["min"] = json!(min.as_slice());
            a["max"] = json!(max.as_slice());
        }
        self.accessors.push(a);
        Ok(self.accessors.len() - 1)
    }
    fn binding(&mut self, b: &Binding, g: &Geometry, snapshot: &Snapshot) -> Result<Value> {
        let set = match b.uv_attribute {
            Some(id) => g
                .uv_attributes
                .iter()
                .position(|v| *v == id)
                .ok_or_else(|| Error::new("reference", "export UV selector missing"))?,
            None => 0,
        };
        let key = canonical(&(&b.image, &b.sampler))?;
        let texture = if let Some(i) = self.texture_ids.get(&key) {
            *i
        } else {
            if self.textures.len() >= 64 {
                return Err(budget("GLB texture table exceeds 64"));
            }
            let image = if let Some(i) = self.image_ids.get(&b.image) {
                *i
            } else {
                if self.images.len() >= 16 {
                    return Err(budget("GLB image table exceeds 16"));
                }
                let asset = snapshot
                    .images
                    .get(&b.image)
                    .ok_or_else(|| Error::new("reference", "encoded export image missing"))?;
                self.image_pixels += u64::from(asset.width) * u64::from(asset.height);
                if self.image_pixels > 12 * 1024 * 1024 {
                    return Err(budget(
                        "GLB decoded image aggregate exceeds twelve million pixels",
                    ));
                }
                let view = self.view(&asset.encoded, false)?;
                let i = self.images.len();
                self.images.push(json!({"bufferView":view,"mimeType":match asset.mime{Mime::Png=>"image/png",Mime::Jpeg=>"image/jpeg"}}));
                self.image_ids.insert(b.image.clone(), i);
                i
            };
            let wrap = |v| match v {
                Wrap::Repeat => 10497,
                Wrap::Clamp => 33071,
                Wrap::Mirror => 33648,
            };
            self.samplers.push(json!({"wrapS":wrap(b.sampler.wrap_s),"wrapT":wrap(b.sampler.wrap_t),"magFilter":match b.sampler.mag{Filter::Nearest=>9728,Filter::Linear=>9729},"minFilter":match b.sampler.min{MinFilter::Nearest=>9728,MinFilter::Linear=>9729,MinFilter::NearestMipNearest=>9984,MinFilter::LinearMipNearest=>9985,MinFilter::NearestMipLinear=>9986,MinFilter::LinearMipLinear=>9987}}));
            let i = self.textures.len();
            self.textures
                .push(json!({"source":image,"sampler":self.samplers.len()-1}));
            self.texture_ids.insert(key, i);
            i
        };
        Ok(json!({"index":texture,"texCoord":set}))
    }
    fn material(&mut self, inst: &Instance, snapshot: &Snapshot) -> Result<Value> {
        let m = &inst.material;
        m.validate()?;
        let p = m
            .pbr
            .as_ref()
            .ok_or_else(|| unsupported("evaluated GLB export requires PBR materials"))?;
        if m.texture.is_some() || m.emission.iter().any(|x| *x > 1.) {
            return Err(unsupported(
                "legacy textures or HDR emission need a separate glTF export profile",
            ));
        }
        let opacity = if let Some(a) = &p.advanced {
            if a.model != scattering::Model::Principled {
                return Err(unsupported("extended BSDF has no core glTF representation"));
            }
            a.opacity.clone()
        } else {
            scattering::Opacity::Opaque
        };
        let (alpha, mode, cutoff) = match opacity {
            scattering::Opacity::Opaque => (1., "OPAQUE", None),
            scattering::Opacity::Blend { factor } => (factor, "BLEND", None),
            scattering::Opacity::Mask { factor, cutoff } => (factor, "MASK", Some(cutoff)),
        };
        let mut out = json!({"name":format!("material-{:032x}",m.id.0),"doubleSided":p.double_sided,"emissiveFactor":m.emission,"alphaMode":mode,"pbrMetallicRoughness":{"baseColorFactor":[f64::from(m.base_color[0]),f64::from(m.base_color[1]),f64::from(m.base_color[2]),alpha],"metallicFactor":m.metallic,"roughnessFactor":m.roughness}});
        if let Some(c) = cutoff {
            out["alphaCutoff"] = json!(c);
        }
        for (binding, key, base) in [
            (&p.base_color, "baseColorTexture", true),
            (&p.metallic_roughness, "metallicRoughnessTexture", true),
            (&p.normal, "normalTexture", false),
            (&p.emission, "emissiveTexture", false),
            (&p.occlusion, "occlusionTexture", false),
        ] {
            if let Some(b) = binding {
                let mut info = self.binding(b, &inst.geometry, snapshot)?;
                if key == "normalTexture" {
                    info["scale"] = json!(p.normal_scale);
                }
                if key == "occlusionTexture" {
                    info["strength"] = json!(p.occlusion_strength);
                }
                if base {
                    out["pbrMetallicRoughness"][key] = info;
                } else {
                    out[key] = info;
                }
            }
        }
        Ok(out)
    }
    fn geometry(&mut self, g: &Geometry, cancelled: &mut impl FnMut() -> bool) -> Result<Value> {
        let first = g
            .triangles
            .first()
            .ok_or_else(|| unsupported("empty exported geometry"))?;
        let mut positions = vec![];
        let mut normals = vec![];
        let mut tangents = vec![];
        let mut colors = vec![];
        let mut uv = vec![vec![]; g.uv_attributes.len().max(1)];
        for t in &g.triangles {
            check(cancelled)?;
            let p = t.positions.map(|v| v.as_vec3());
            if p.iter().any(|v| !v.is_finite())
                || (p[1] - p[0]).cross(p[2] - p[0]).length_squared() == 0.
            {
                return Err(Error::new(
                    "precision",
                    "f32 exported triangle collapses or overflows",
                ));
            }
            positions.extend(p.map(|v| v.to_array()));
            if t.normals.is_some() != first.normals.is_some()
                || t.tangents.is_some() != first.tangents.is_some()
            {
                return Err(unsupported("inconsistent triangle frame arrays"));
            }
            if let Some(n) = t.normals {
                if n.iter()
                    .any(|v| !v.is_finite() || (v.length_squared() - 1.).abs() > 1e-6)
                {
                    return Err(unsupported("glTF export requires unit authored normals"));
                }
                normals.extend(n.map(|v| v.as_vec3().to_array()));
            }
            if let Some(t) = t.tangents {
                if first.normals.is_none()
                    || t.iter().any(|v| {
                        !v.is_finite()
                            || (v.truncate().length_squared() - 1.).abs() > 1e-6
                            || v.w.abs() != 1.
                            || v.w != t[0].w
                    })
                {
                    return Err(unsupported(
                        "glTF tangents require unit directions, normals and one handedness per triangle",
                    ));
                }
                tangents.extend(t.map(|v| v.as_vec4().to_array()));
            }
            if let Some(c) = t.colors {
                colors.extend(c.map(|v| v.to_array()));
            }
            if g.uv_attributes.is_empty() {
                uv[0].extend(t.uv.map(|v| v.to_array()));
            } else {
                if t.uv != t.uv_sets[0] {
                    return Err(unsupported("default UV does not match first named set"));
                }
                for (output, values) in uv.iter_mut().zip(&t.uv_sets) {
                    output.extend(values.map(|v| v.to_array()));
                }
            }
        }
        let mut attrs = json!({"POSITION":self.accessor(&positions,"VEC3",true)?});
        if !normals.is_empty() {
            attrs["NORMAL"] = json!(self.accessor(&normals, "VEC3", false)?);
        }
        if !tangents.is_empty() {
            attrs["TANGENT"] = json!(self.accessor(&tangents, "VEC4", false)?);
        }
        if !colors.is_empty() {
            attrs["COLOR_0"] = json!(self.accessor(&colors, "VEC4", false)?);
        }
        for (i, values) in uv.iter().enumerate() {
            attrs[format!("TEXCOORD_{i}")] = json!(self.accessor(values, "VEC2", false)?);
        }
        Ok(attrs)
    }
}
fn transform(inst: &Instance, tolerance: f64) -> Result<([f32; 16], f64)> {
    let t = inst.transform;
    let axes = [t.matrix3.x_axis, t.matrix3.y_axis, t.matrix3.z_axis];
    if !t.is_finite() || axes.iter().any(|v| v.length_squared() < 1e-24) {
        return Err(unsupported("singular or nonfinite export transform"));
    }
    for i in 0..3 {
        for j in i + 1..3 {
            if axes[i].normalize().dot(axes[j].normalize()).abs() > 1e-8 {
                return Err(unsupported(
                    "sheared world transform is not a core glTF TRS matrix",
                ));
            }
        }
    }
    let matrix = DMat4::from(t).to_cols_array().map(|v| v as f32);
    let rounded = DAffine3::from_mat4(DMat4::from_cols_array(&matrix.map(f64::from)));
    if !rounded.is_finite() || rounded.matrix3.determinant() == 0. {
        return Err(Error::new(
            "precision",
            "export transform exceeds f32 range",
        ));
    }
    let mut max: f64 = 0.;
    for tri in &inst.geometry.triangles {
        for p in tri.positions {
            let difference = t
                .transform_point3(p)
                .distance(rounded.transform_point3(p.as_vec3().as_dvec3()));
            if !difference.is_finite() {
                return Err(Error::new(
                    "precision",
                    "nonfinite exported world coordinate",
                ));
            }
            max = max.max(difference);
        }
    }
    if max > tolerance {
        return Err(Error::new(
            "precision",
            "GLB world-position quantization exceeds requested tolerance",
        )
        .with_context("observed_meters", max.to_string()));
    }
    Ok((matrix, max))
}

pub fn export(
    snapshot: &Snapshot,
    request: &Request,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Exported> {
    check(&mut cancelled)?;
    let revision = snapshot.revision()?;
    if revision != request.revision {
        return Err(Error::new(
            "stale_revision",
            "GLB export revision does not match authored snapshot",
        ));
    }
    let policy = &request.policy;
    if !policy.allow_approximations {
        return Err(unsupported(
            "explicit consent required for evaluated topology, f32 and authoring-data losses",
        ));
    }
    if policy.max_bytes == 0
        || policy.max_bytes > gltf_scene::MAX_INPUT as u64
        || policy.max_vertices == 0
        || policy.max_vertices > 10000
        || !policy.max_position_error_meters.is_finite()
        || policy.max_position_error_meters <= 0.
        || policy.max_position_error_meters > 1.
    {
        return Err(budget("invalid GLB export policy limits"));
    }
    let mut evaluator = Evaluator::default();
    let (scene, animation) = if let Some(at) = &request.at {
        let (s, r) = evaluator.evaluate_at(snapshot, at.clip, at.time, &mut cancelled)?;
        (s, Some(r))
    } else {
        (
            evaluator.evaluate_with_cancel(snapshot, &mut cancelled)?,
            None,
        )
    };
    scene.validate_geometry_bindings()?;
    if !scene.media.is_empty() {
        return Err(unsupported("sparse media has no core glTF representation"));
    }
    if scene.instances.is_empty() || scene.instances.len() > 64 {
        return Err(budget(
            "GLB export requires 1..64 visible surface instances",
        ));
    }
    let mut b = Builder {
        limit: policy.max_bytes as usize,
        binary: vec![],
        views: vec![],
        accessors: vec![],
        images: vec![],
        image_ids: BTreeMap::new(),
        image_pixels: 0,
        samplers: vec![],
        textures: vec![],
        texture_ids: BTreeMap::new(),
    };
    let mut meshes = vec![];
    let mut materials = vec![];
    let mut nodes = vec![];
    let mut geometry_ids: BTreeMap<String, Value> = BTreeMap::new();
    let mut mesh_ids = BTreeMap::new();
    let mut material_ids = BTreeMap::new();
    let mut entities = BTreeMap::new();
    let mut vertices = 0usize;
    let mut position_error: f64 = 0.;
    // Private entity chunk order can change on archive recovery. File tables use
    // durable identity order so equivalent snapshots yield identical GLB bytes.
    let mut instances = scene.instances.iter().collect::<Vec<_>>();
    instances.sort_by_key(|i| i.id);
    for inst in instances {
        check(&mut cancelled)?;
        let (matrix, error) = transform(inst, policy.max_position_error_meters)?;
        position_error = position_error.max(error);
        let material_key = canonical(&(&inst.material, &inst.geometry.uv_attributes))?;
        let material = if let Some(i) = material_ids.get(&material_key) {
            *i
        } else {
            if materials.len() >= 64 {
                return Err(budget("GLB material table exceeds 64"));
            }
            let i = materials.len();
            materials.push(b.material(inst, snapshot)?);
            material_ids.insert(material_key, i);
            i
        };
        let mesh_key = (inst.geometry_id.clone(), material);
        let mesh = if let Some(i) = mesh_ids.get(&mesh_key) {
            *i
        } else {
            vertices = vertices
                .checked_add(inst.geometry.triangles.len() * 3)
                .ok_or_else(|| budget("vertex count overflow"))?;
            if vertices > policy.max_vertices as usize {
                return Err(budget("GLB exported vertex budget"));
            }
            let attrs = if let Some(a) = geometry_ids.get(&inst.geometry_id) {
                a.clone()
            } else {
                let a = b.geometry(&inst.geometry, &mut cancelled)?;
                geometry_ids.insert(inst.geometry_id.clone(), a.clone());
                a
            };
            let i = meshes.len();
            meshes.push(json!({"primitives":[{"attributes":attrs,"material":material,"mode":4}]}));
            mesh_ids.insert(mesh_key, i);
            i
        };
        let name = snapshot.entities.get(inst.id).map(|e| e.name.as_str());
        if name.is_some_and(|n| n.len() > 1024) {
            return Err(budget("GLB node label exceeds 1024 UTF-8 bytes"));
        }
        let name = name
            .map(str::to_owned)
            .unwrap_or_else(|| format!("entity-{:032x}", inst.id.0));
        entities.insert(inst.id, nodes.len() as u32);
        nodes.push(json!({"name":name,"mesh":mesh,"matrix":matrix}));
    }
    let mesh_count = meshes.len();
    let material_count = materials.len();
    let image_count = b.images.len();
    let mut root = json!({"asset":{"version":"2.0","generator":"render Rust evaluated PBR export v1"},"scene":0,"scenes":[{"nodes":(0..nodes.len()).collect::<Vec<_>>()}],"nodes":nodes,"meshes":meshes,"materials":materials,"buffers":[{"byteLength":b.binary.len()}],"bufferViews":b.views,"accessors":b.accessors});
    if !b.images.is_empty() {
        root["images"] = json!(b.images);
        root["textures"] = json!(b.textures);
        root["samplers"] = json!(b.samplers);
    }
    check(&mut cancelled)?;
    let mut json = canonical(&root)?;
    json.resize(json.len().next_multiple_of(4), b' ');
    b.binary.resize(b.binary.len().next_multiple_of(4), 0);
    let total = 28 + json.len() + b.binary.len();
    if total > policy.max_bytes as usize {
        return Err(budget("complete GLB exceeds output budget"));
    }
    let mut glb = Vec::with_capacity(total);
    for n in [0x46546c67, 2, total as u32, json.len() as u32, 0x4e4f534a] {
        glb.extend(n.to_le_bytes());
    }
    glb.extend(json);
    for n in [b.binary.len() as u32, 0x004e4942] {
        glb.extend(n.to_le_bytes());
    }
    glb.extend(b.binary);
    check(&mut cancelled)?;
    let report=Report{profile:"gltf2-evaluated-pbr-export-v1".into(),authored_revision:revision,evaluated_revision:scene.revision,animation,source_entities:entities,meshes:mesh_count,materials:material_count,images:image_count,exported_vertices:vertices,max_position_error_meters:position_error,bytes:total,output_digest:digest(&glb),losses:vec![
        "Evaluated local triangles with duplicated corner vertices and f32 attributes/transforms; authored polygon/loose topology, stable IDs and custom attributes are not preserved.".into(),
        "World transforms replace authored parenting; clip, rig, controller, procedural, curve and displacement intent becomes the selected evaluated surface. Native archives retain authoring data.".into(),
        "Render settings, cameras, lights, environment, hidden and non-renderable authoring objects are not exported. Reapply an explicit render recipe when comparing engine images.".into(),
        "Core glTF metallic/roughness semantics retain the engine's bounded GGX, normal-frame fallback, texture filtering and CPU alpha qualifications; no arbitrary external-renderer pixel fidelity claim.".into()]};
    Ok(Exported { glb, report })
}
