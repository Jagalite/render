//! glTF material/image/lens decoding used by the named static PBR adapter.
use crate::{
    cameras::Lens,
    gltf_scene::{array, at, index, number, offset},
    pbr::Surface,
    textures::*,
    *,
};
use serde_json::Value;
fn invalid(message: &str) -> Error {
    Error::new("gltf_scene", message).at("import")
}
fn unsupported(message: &str) -> Error {
    Error::new("unsupported_gltf", message).at("import")
}
pub fn images(root: &Value, buffers: &[Vec<u8>], external: &[Vec<u8>]) -> Result<Vec<ImageAsset>> {
    let images = array(root, "images")?;
    if images.len() > 16 || (!external.is_empty() && external.len() != images.len()) {
        return Err(Error::new(
            "budget",
            "image table or explicit resources outside profile",
        ));
    }
    let mut output = vec![];
    let mut pixels = 0u64;
    for (i, img) in images.iter().enumerate() {
        if !img.is_object() || img.get("mimeType").is_some_and(|v| !v.is_string()) {
            return Err(invalid("image object or MIME type"));
        }
        if img.get("extensions").is_some() {
            return Err(unsupported("image extension"));
        }
        let bytes = if let Some(uri) = img.get("uri") {
            if !uri.is_string() || img.get("bufferView").is_some() {
                return Err(invalid("image URI or bufferView"));
            }
            external
                .get(i)
                .ok_or_else(|| {
                    invalid("supply every external image by index; URIs are never fetched")
                })?
                .clone()
        } else {
            let view = at(root, "bufferViews", index(&img["bufferView"])?)?;
            if view.get("byteStride").is_some() {
                return Err(invalid("encoded image bufferView cannot be strided"));
            }
            let bin = buffers
                .get(index(&view["buffer"])?)
                .ok_or_else(|| invalid("image buffer"))?;
            let start = offset(view, "byteOffset")?;
            let len = index(&view["byteLength"])?;
            let end = start
                .checked_add(len)
                .ok_or_else(|| invalid("image offset overflow"))?;
            bin.get(start..end)
                .ok_or_else(|| invalid("image bufferView bounds"))?
                .to_vec()
        };
        let mime = match img.get("mimeType").and_then(Value::as_str) {
            Some("image/png") => Mime::Png,
            Some("image/jpeg") => Mime::Jpeg,
            None if img.get("uri").is_some() => {
                match image::guess_format(&bytes).map_err(|e| invalid(&e.to_string()))? {
                    image::ImageFormat::Png => Mime::Png,
                    image::ImageFormat::Jpeg => Mime::Jpeg,
                    _ => return Err(unsupported("image format")),
                }
            }
            _ => return Err(unsupported("image MIME type")),
        };
        let asset = ImageAsset::from_encoded(mime, bytes)?;
        pixels += u64::from(asset.width) * u64::from(asset.height);
        if pixels > 12 * 1024 * 1024 {
            return Err(Error::new(
                "budget",
                "decoded image aggregate exceeds twelve million pixels",
            ));
        }
        output.push(asset);
    }
    Ok(output)
}
fn wrap(value: Option<&Value>) -> Result<Wrap> {
    Ok(match value.map(index).transpose()?.unwrap_or(10497) {
        10497 => Wrap::Repeat,
        33071 => Wrap::Clamp,
        33648 => Wrap::Mirror,
        _ => return Err(invalid("texture wrap mode")),
    })
}
fn binding(
    root: &Value,
    ids: &[String],
    info: Option<&Value>,
    role: TextureRole,
) -> Result<Option<Binding>> {
    let Some(info) = info else { return Ok(None) };
    if info.get("extensions").is_some()
        || info.get("texCoord").map(index).transpose()?.unwrap_or(0) != 0
    {
        return Err(unsupported(
            "texture extension or UV set other than TEXCOORD_0",
        ));
    }
    let texture = at(root, "textures", index(&info["index"])?)?;
    if texture.get("extensions").is_some() {
        return Err(unsupported("texture extension"));
    }
    let image = ids
        .get(index(&texture["source"])?)
        .ok_or_else(|| invalid("texture image reference"))?
        .clone();
    let sampler = if let Some(i) = texture.get("sampler") {
        let s = at(root, "samplers", index(i)?)?;
        if !s.is_object() {
            return Err(invalid("sampler must be an object"));
        }
        if s.get("extensions").is_some() {
            return Err(unsupported("sampler extension"));
        }
        Sampler {
            wrap_s: wrap(s.get("wrapS"))?,
            wrap_t: wrap(s.get("wrapT"))?,
            mag: match s.get("magFilter").map(index).transpose()?.unwrap_or(9729) {
                9728 => Filter::Nearest,
                9729 => Filter::Linear,
                _ => return Err(invalid("magnification filter")),
            },
            min: match s.get("minFilter").map(index).transpose()?.unwrap_or(9987) {
                9728 => MinFilter::Nearest,
                9729 => MinFilter::Linear,
                9984 => MinFilter::NearestMipNearest,
                9985 => MinFilter::LinearMipNearest,
                9986 => MinFilter::NearestMipLinear,
                9987 => MinFilter::LinearMipLinear,
                _ => return Err(invalid("minification filter")),
            },
        }
    } else {
        Sampler::default()
    };
    Ok(Some(Binding {
        image,
        role,
        sampler,
    }))
}
pub fn surface(root: &Value, ids: &[String], material: &Value) -> Result<Surface> {
    if material.get("extensions").is_some()
        || material.get("alphaMode").is_some_and(|v| v != "OPAQUE")
    {
        return Err(unsupported("material extension or non-opaque alpha mode"));
    }
    let p = &material["pbrMetallicRoughness"];
    let surface = Surface {
        advanced: None,
        displacement: None,
        double_sided: material
            .get("doubleSided")
            .map(|v| {
                v.as_bool()
                    .ok_or_else(|| invalid("doubleSided must be boolean"))
            })
            .transpose()?
            .unwrap_or(false),
        base_color: binding(root, ids, p.get("baseColorTexture"), TextureRole::SrgbColor)?,
        metallic_roughness: binding(
            root,
            ids,
            p.get("metallicRoughnessTexture"),
            TextureRole::LinearData,
        )?,
        normal: binding(
            root,
            ids,
            material.get("normalTexture"),
            TextureRole::LinearData,
        )?,
        emission: binding(
            root,
            ids,
            material.get("emissiveTexture"),
            TextureRole::SrgbColor,
        )?,
        occlusion: binding(
            root,
            ids,
            material.get("occlusionTexture"),
            TextureRole::LinearData,
        )?,
        normal_scale: material["normalTexture"]
            .get("scale")
            .map(number)
            .transpose()?
            .unwrap_or(1.) as f32,
        occlusion_strength: material["occlusionTexture"]
            .get("strength")
            .map(number)
            .transpose()?
            .unwrap_or(1.) as f32,
    };
    surface.validate()?;
    Ok(surface)
}
pub fn lens(camera: &Value) -> Result<Lens> {
    if camera.get("extensions").is_some() {
        return Err(unsupported("camera extension"));
    }
    let lens = match camera["type"].as_str() {
        Some("perspective") => {
            let p = &camera["perspective"];
            Lens::Perspective {
                vertical_fov_radians: number(&p["yfov"])?,
                aspect_ratio: p.get("aspectRatio").map(number).transpose()?,
                near: number(&p["znear"])?,
                far: p.get("zfar").map(number).transpose()?,
            }
        }
        Some("orthographic") => {
            let p = &camera["orthographic"];
            Lens::Orthographic {
                xmag: number(&p["xmag"])?,
                ymag: number(&p["ymag"])?,
                near: number(&p["znear"])?,
                far: number(&p["zfar"])?,
            }
        }
        _ => return Err(invalid("camera type")),
    };
    lens.validate()?;
    Ok(lens)
}
