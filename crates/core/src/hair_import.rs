//! External HAIR polylines with explicit units, byte order and surface interpretation.
use crate::{
    Error, Id, Result, canonical, curves, digest,
    document::{Command, Entity, Material, Transform},
    pbr, scattering,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
pub const PROFILE: &str = "hair-polyline-surface-v1";
pub const RGBA_PROFILE: &str = "hair-polyline-rgba-v1";
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByteOrder {
    LittleEndian,
    BigEndian,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Thickness {
    Radius,
    Diameter,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpace {
    LinearSrgb,
    Srgb,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transparency {
    CoverageOneMinusTransparency,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    SweptPolylineSurface,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointAttributes {
    LinearRgbaF32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointAttributeConversion {
    pub control_count: u32,
    pub max_alpha_absolute_error: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub byte_order: ByteOrder,
    pub meters_per_unit: f64,
    pub thickness: Thickness,
    pub color_space: ColorSpace,
    pub transparency: Transparency,
    pub representation: Representation,
    pub roughness: f32,
    pub tessellation: curves::Tessellation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_attributes: Option<PointAttributes>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strand {
    pub source_index: u32,
    pub source_point_offset: u32,
    pub point_count: u32,
    pub curve: Id,
    pub entity: Id,
    pub material: Id,
    pub geometry_asset: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub profile: String,
    pub source_digest: String,
    pub policy: Policy,
    pub input_bytes: u64,
    pub source_strands: u32,
    pub source_points: u32,
    pub source_flags: u32,
    pub material_groups: u32,
    pub geometry_asset_json_bytes: u64,
    pub derived_samples: u64,
    pub derived_vertices: u64,
    pub derived_triangles: u64,
    /// Platform-local serialized length of disposable f64 geometry; not source identity.
    pub observed_derived_json_bytes: u64,
    pub strands: Vec<Strand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_attribute_conversion: Option<PointAttributeConversion>,
}
pub struct Imported {
    pub commands: Vec<Command>,
    pub report: Report,
}
fn error(code: &str, message: &str) -> Error {
    Error::new(code, message).at("import")
}
fn check(cancel: &mut impl FnMut() -> bool) -> Result<()> {
    if cancel() {
        Err(error(
            "cancelled",
            "HAIR import cancelled before publication",
        ))
    } else {
        Ok(())
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    order: ByteOrder,
}
impl Reader<'_> {
    fn u32(&self, at: usize) -> u32 {
        let b = self.bytes[at..at + 4].try_into().unwrap();
        match self.order {
            ByteOrder::LittleEndian => u32::from_le_bytes(b),
            ByteOrder::BigEndian => u32::from_be_bytes(b),
        }
    }
    fn u16(&self, at: usize) -> u16 {
        let b = self.bytes[at..at + 2].try_into().unwrap();
        match self.order {
            ByteOrder::LittleEndian => u16::from_le_bytes(b),
            ByteOrder::BigEndian => u16::from_be_bytes(b),
        }
    }
    fn float(&self, at: usize) -> f32 {
        f32::from_bits(self.u32(at))
    }
    fn unit(&self, at: usize) -> Result<f32> {
        let v = self.float(at);
        if !v.is_finite() || !(0.0..=1.).contains(&v) {
            Err(error(
                "hair",
                "color/transparency outside finite unit range",
            ))
        } else {
            Ok(if v == 0. { 0. } else { v })
        }
    }
}
struct InputStrand {
    index: u32,
    offset: u32,
    curve: curves::Curve,
}
struct Group {
    color: [f32; 3],
    opacity: scattering::Opacity,
    strands: Vec<InputStrand>,
}
pub fn import(
    bytes: &[u8],
    document_id: Id,
    policy: &Policy,
    mut cancel: impl FnMut() -> bool,
) -> Result<Imported> {
    check(&mut cancel)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(error("budget", "HAIR input exceeds 4 MiB"));
    }
    if bytes.len() < 128 || &bytes[..4] != b"HAIR" {
        return Err(error("hair", "missing or truncated HAIR header"));
    }
    policy.tessellation.validate().map_err(|e| e.at("import"))?;
    if policy.tessellation.max_samples > 32768 || policy.tessellation.max_vertices > 65536 {
        return Err(error(
            "budget",
            "HAIR sweep policy exceeds 32768 samples or 65536 vertices",
        ));
    }
    if !policy.meters_per_unit.is_finite()
        || policy.meters_per_unit <= 0.
        || !policy.roughness.is_finite()
        || !(0.0..=1.).contains(&policy.roughness)
    {
        return Err(error(
            "hair_policy",
            "invalid metric scale or Principled roughness",
        ));
    }
    let r = Reader {
        bytes,
        order: policy.byte_order,
    };
    let strands = r.u32(4);
    let points = r.u32(8);
    let flags = r.u32(12);
    if flags & 2 == 0 || flags & !31 != 0 {
        return Err(error(
            "unsupported_hair",
            "HAIR requires points and no unknown flags",
        ));
    }
    if strands == 0 || strands > 2048 || points == 0 || points > 16384 {
        return Err(error(
            "budget",
            "HAIR profile requires 1..2048 strands and 1..16384 total controls",
        ));
    }
    let ns = strands as usize;
    let np = points as usize;
    let segment_bytes = if flags & 1 != 0 { 2 * ns } else { 0 };
    let point_at = 128 + segment_bytes;
    let thickness_at = point_at + 12 * np;
    let transparency_at = thickness_at + if flags & 4 != 0 { 4 * np } else { 0 };
    let color_at = transparency_at + if flags & 8 != 0 { 4 * np } else { 0 };
    let end = color_at + if flags & 16 != 0 { 12 * np } else { 0 };
    if end != bytes.len() {
        return Err(error("hair", "truncated payload or trailing HAIR bytes"));
    }
    let mut counts = Vec::with_capacity(ns);
    let mut total = 0_u64;
    for i in 0..ns {
        let segments = if flags & 1 != 0 {
            u32::from(r.u16(128 + 2 * i))
        } else {
            r.u32(16)
        };
        if !(1..=4095).contains(&segments) {
            return Err(error("hair", "each strand requires 1..4095 segments"));
        }
        total += u64::from(segments) + 1;
        counts.push(segments + 1);
    }
    if total != u64::from(points) {
        return Err(error(
            "hair",
            "strand segment totals disagree with point count",
        ));
    }
    let point_attributes = policy.point_attributes.is_some();
    let profile = if point_attributes {
        RGBA_PROFILE
    } else {
        PROFILE
    };
    let mut max_alpha_absolute_error = 0_f64;
    let source_digest = digest(bytes);
    let identity = digest(&canonical(&(document_id, profile, &source_digest, policy))?);
    let id = |kind: &str, index: u32| -> Result<Id> {
        let d = digest(&canonical(&(&identity, kind, index))?);
        Ok(Id(u128::from_str_radix(&d[7..39], 16).expect("SHA256 hex")))
    };
    let mut groups: BTreeMap<([u32; 3], u32), Group> = BTreeMap::new();
    let mut offset = 0_usize;
    for (strand, count) in counts.iter().enumerate() {
        check(&mut cancel)?;
        let mut controls = Vec::with_capacity(*count as usize);
        let mut appearance = None;
        for j in 0..*count as usize {
            let i = offset + j;
            if i.is_multiple_of(256) {
                check(&mut cancel)?;
            }
            let position = std::array::from_fn(|c| {
                f64::from(r.float(point_at + 12 * i + 4 * c)) * policy.meters_per_unit
            });
            let thickness = f64::from(r.float(if flags & 4 != 0 {
                thickness_at + 4 * i
            } else {
                20
            }));
            let radius = thickness
                * policy.meters_per_unit
                * if policy.thickness == Thickness::Diameter {
                    0.5
                } else {
                    1.
                };
            let transparency = r.unit(if flags & 8 != 0 {
                transparency_at + 4 * i
            } else {
                24
            })?;
            let mut color = [0.; 3];
            for (c, out) in color.iter_mut().enumerate() {
                *out = r.unit(if flags & 16 != 0 {
                    color_at + 12 * i + 4 * c
                } else {
                    28 + 4 * c
                })?;
            }
            let current = (color, transparency);
            if !point_attributes && appearance.is_some_and(|a| a != current) {
                return Err(error(
                    "unsupported_hair",
                    "within-strand varying color/transparency requires a curve-attribute profile",
                )
                .with_context("strand", strand.to_string()));
            }
            appearance = Some(current);
            let rgba = if point_attributes {
                let rgb = match policy.color_space {
                    ColorSpace::LinearSrgb => color,
                    ColorSpace::Srgb => color.map(crate::imaging::srgb_to_linear),
                };
                let exact_alpha = 1. - f64::from(transparency);
                let alpha = exact_alpha as f32;
                max_alpha_absolute_error =
                    max_alpha_absolute_error.max((exact_alpha - f64::from(alpha)).abs());
                Some([rgb[0], rgb[1], rgb[2], alpha])
            } else {
                None
            };
            controls.push(curves::Control {
                id: id("point", i as u32)?,
                position,
                radius,
                tilt: 0.,
                color: rgba,
            });
        }
        let curve = curves::Curve {
            id: id("strand", strand as u32)?,
            basis: curves::Basis::Polyline,
            controls,
            closed: false,
        };
        curve
            .validate()
            .map_err(|e| e.at("import").with_context("strand", strand.to_string()))?;
        let (color, transparency) = appearance.expect("nonempty strand");
        let (color, opacity, key) = if point_attributes {
            let blend = curve
                .controls
                .iter()
                .any(|c| c.color.is_some_and(|v| v[3] < 1.));
            (
                [1.; 3],
                if blend {
                    scattering::Opacity::Blend { factor: 1. }
                } else {
                    scattering::Opacity::Opaque
                },
                ([1_f32.to_bits(); 3], u32::from(blend)),
            )
        } else {
            (
                color,
                if transparency == 0. {
                    scattering::Opacity::Opaque
                } else {
                    scattering::Opacity::Blend {
                        factor: 1. - f64::from(transparency),
                    }
                },
                (color.map(f32::to_bits), transparency.to_bits()),
            )
        };
        if !groups.contains_key(&key) && groups.len() == 128 {
            return Err(error("budget", "HAIR exceeds 128 material groups"));
        }
        groups
            .entry(key)
            .or_insert_with(|| Group {
                color,
                opacity,
                strands: vec![],
            })
            .strands
            .push(InputStrand {
                index: strand as u32,
                offset: offset as u32,
                curve,
            });
        offset += *count as usize;
    }
    let mut report = Report {
        profile: profile.into(),
        source_digest,
        policy: policy.clone(),
        input_bytes: bytes.len() as u64,
        source_strands: strands,
        source_points: points,
        source_flags: flags,
        material_groups: groups.len() as u32,
        geometry_asset_json_bytes: 0,
        derived_samples: 0,
        derived_vertices: 0,
        derived_triangles: 0,
        observed_derived_json_bytes: 0,
        strands: vec![],
        point_attribute_conversion: point_attributes.then_some(PointAttributeConversion {
            control_count: points,
            max_alpha_absolute_error,
        }),
    };
    let mut commands = vec![];
    for (index, group) in groups.into_values().enumerate() {
        check(&mut cancel)?;
        let entity = id("entity", index as u32)?;
        let material_id = id("material", index as u32)?;
        let asset = curves::Asset {
            shape: curves::Shape::Curves {
                curves: group.strands.iter().map(|s| s.curve.clone()).collect(),
            },
            tessellation: policy.tessellation.clone(),
        };
        let derived = asset.evaluate(&mut cancel)?;
        let conversion = derived.receipt;
        report.derived_samples += conversion.samples as u64;
        report.derived_vertices += conversion.vertices as u64;
        report.derived_triangles += conversion.triangles as u64;
        report.observed_derived_json_bytes += conversion.derived_bytes as u64;
        if report.derived_samples > 32768
            || report.derived_vertices > 65536
            || report.derived_triangles > 131072
        {
            return Err(error(
                "budget",
                "HAIR aggregate sweep samples/vertices/triangles exceed import profile",
            ));
        }
        report.geometry_asset_json_bytes += canonical(&asset)?.len() as u64;
        let key = asset.content_id()?;
        let color = match policy.color_space {
            ColorSpace::LinearSrgb => group.color,
            ColorSpace::Srgb => group.color.map(crate::imaging::srgb_to_linear),
        };
        let opacity = group.opacity;
        let material = Material {
            id: material_id,
            base_color: color,
            emission: [0.; 3],
            roughness: policy.roughness,
            metallic: 0.,
            texture: None,
            pbr: Some(pbr::Surface {
                double_sided: true,
                // Use canonical opaque PBR for the new profile so evaluated
                // GLB roundtrips retain the same sampling profile. Preserve
                // the original importer profile byte-for-byte.
                advanced: if point_attributes && matches!(opacity, scattering::Opacity::Opaque) {
                    None
                } else {
                    Some(scattering::Surface {
                        model: scattering::Model::Principled,
                        opacity,
                    })
                },
                ..Default::default()
            }),
        };
        material.validate()?;
        for s in &group.strands {
            report.strands.push(Strand {
                source_index: s.index,
                source_point_offset: s.offset,
                point_count: s.curve.controls.len() as u32,
                curve: s.curve.id,
                entity,
                material: material_id,
                geometry_asset: key.clone(),
            });
        }
        commands.push(Command::PutMaterial {
            material: Box::new(material),
        });
        commands.push(Command::PutGeometry { asset });
        commands.push(Command::CreateEntity {
            entity: Entity {
                id: entity,
                name: format!("Imported HAIR group {index}"),
                parent: None,
                mesh: None,
                material: Some(material_id),
                transform: Transform::default(),
            },
        });
        commands.push(Command::SetGeometry {
            entity,
            asset: Some(key),
        });
    }
    report.strands.sort_by_key(|s| s.source_index);
    check(&mut cancel)?;
    Ok(Imported { commands, report })
}
