//! Opt-in authored directions under additive morphs and affine blend skinning.
use super::*;
use glam::DMat3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameBinding {
    pub topology: String,
    pub normal: Id,
    pub tangent: Option<Id>,
    pub tangent_sign: Option<Id>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionOffsets {
    pub attribute: Id,
    #[serde(with = "crate::geometry::local_map")]
    pub offsets: BTreeMap<u64, [f64; 3]>,
}
fn attribute<'a>(mesh: &'a Mesh, id: Id, semantic: &str) -> Result<&'a Attribute> {
    mesh.attributes
        .values()
        .find(|a| a.id == id && a.semantic == semantic)
        .ok_or_else(|| {
            Error::new(
                "reference",
                format!("deformation {semantic} attribute missing"),
            )
        })
}
fn ids(mesh: &Mesh, domain: Domain) -> Result<&[u64]> {
    match domain {
        Domain::Point => Ok(&mesh.point_ids),
        Domain::Corner => Ok(&mesh.corner_ids),
        _ => Err(Error::new(
            "deformation_frame",
            "directions require point or corner attributes",
        )),
    }
}
impl FrameBinding {
    pub fn validate(&self, mesh: &Mesh) -> Result<()> {
        if self.topology != crate::groom::topology(mesh)? {
            return Err(Error::new(
                "stale_topology",
                "deformation frame topology changed",
            ));
        }
        for semantic in ["normal", "tangent", "tangent_sign"] {
            if mesh
                .attributes
                .values()
                .filter(|a| a.semantic == semantic)
                .count()
                > 1
            {
                return Err(Error::new(
                    "deformation_frame",
                    "ambiguous shading attributes require explicit resolution",
                ));
            }
        }
        for (id, semantic) in [(Some(self.normal), "normal"), (self.tangent, "tangent")] {
            if let Some(id) = id {
                let a = attribute(mesh, id, semantic)?;
                ids(mesh, a.domain)?;
                let AttributeValues::Vec3(v) = &a.values else {
                    return Err(Error::new(
                        "deformation_frame",
                        "directions require vec3 attributes",
                    ));
                };
                if v.iter().any(|v| {
                    !DVec3::from_array(v.map(f64::from)).is_finite()
                        || DVec3::from_array(v.map(f64::from)).length_squared() < 1e-24
                }) {
                    return Err(Error::new(
                        "deformation_frame",
                        "base direction is zero or non-finite",
                    ));
                }
            }
        }
        if mesh.attributes.values().any(|a| a.semantic == "tangent") != self.tangent.is_some()
            || mesh
                .attributes
                .values()
                .any(|a| a.semantic == "tangent_sign")
                != self.tangent_sign.is_some()
        {
            return Err(Error::new(
                "deformation_frame",
                "all authored tangent components must be bound",
            ));
        }
        if self.tangent.is_some() != self.tangent_sign.is_some() {
            return Err(Error::new(
                "deformation_frame",
                "tangent and handedness must be bound together",
            ));
        }
        if let Some(id) = self.tangent_sign {
            let sign = attribute(mesh, id, "tangent_sign")?;
            let tangent = attribute(mesh, self.tangent.expect("checked"), "tangent")?;
            let AttributeValues::Scalar(v) = &sign.values else {
                return Err(Error::new(
                    "deformation_frame",
                    "handedness requires scalar attributes",
                ));
            };
            if sign.domain != tangent.domain || v.iter().any(|v| !matches!(*v, -1. | 1.)) {
                return Err(Error::new(
                    "deformation_frame",
                    "handedness domain or sign is invalid",
                ));
            }
        }
        Ok(())
    }
}
impl DirectionOffsets {
    pub fn validate(&self, mesh: &Mesh, semantic: &str) -> Result<()> {
        let a = attribute(mesh, self.attribute, semantic)?;
        if !matches!(a.values, AttributeValues::Vec3(_)) {
            return Err(Error::new(
                "deformation_frame",
                "morphed direction requires vec3",
            ));
        }
        let valid = ids(mesh, a.domain)?
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if self.offsets.is_empty()
            || self.offsets.iter().any(|(id, v)| {
                !valid.contains(id) || v.iter().any(|x| !x.is_finite() || x.abs() > 1e6)
            })
        {
            return Err(Error::new("morph", "invalid direction offsets"));
        }
        Ok(())
    }
}
struct Vectors {
    id: Id,
    domain: Domain,
    values: Vec<DVec3>,
}
impl Vectors {
    fn read(mesh: &Mesh, id: Id, semantic: &str) -> Result<Self> {
        let a = attribute(mesh, id, semantic)?;
        let AttributeValues::Vec3(v) = &a.values else {
            return Err(Error::new("deformation_frame", "vec3 direction required"));
        };
        Ok(Self {
            id,
            domain: a.domain,
            values: v
                .iter()
                .map(|v| DVec3::from_array(v.map(f64::from)))
                .collect(),
        })
    }
    fn morph(
        &mut self,
        offsets: &DirectionOffsets,
        weight: f64,
        mesh: &Mesh,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<()> {
        if offsets.attribute != self.id {
            return Err(Error::new(
                "reference",
                "morph direction does not match frame binding",
            ));
        }
        for (v, id) in self.values.iter_mut().zip(ids(mesh, self.domain)?) {
            if cancelled() {
                return Err(Error::new("cancelled", "direction morph cancelled"));
            }
            if let Some(delta) = offsets.offsets.get(id) {
                *v += DVec3::from_array(*delta) * weight;
            }
        }
        Ok(())
    }
    fn point(&self, mesh: &Mesh, i: usize) -> usize {
        if self.domain == Domain::Point {
            i
        } else {
            mesh.corners[i].vertex as usize
        }
    }
    fn write(self, mesh: &mut Mesh) -> Result<()> {
        let mut values = Vec::with_capacity(self.values.len());
        for v in self.values {
            if !v.is_finite() || v.length_squared() < 1e-24 {
                return Err(Error::new(
                    "deformation_frame",
                    "deformed direction is zero or non-finite",
                ));
            }
            values.push(v.normalize().as_vec3().to_array());
        }
        mesh.attributes
            .values_mut()
            .find(|a| a.id == self.id)
            .expect("validated attribute")
            .values = AttributeValues::Vec3(values);
        Ok(())
    }
}
pub(crate) struct PreparedFrames {
    binding: FrameBinding,
    normal: Vectors,
    tangent: Option<Vectors>,
    signs: Option<Vec<f32>>,
}
impl PreparedFrames {
    pub(crate) fn new(binding: &FrameBinding, mesh: &Mesh) -> Result<Self> {
        binding.validate(mesh)?;
        let signs = binding
            .tangent_sign
            .map(|id| {
                let AttributeValues::Scalar(v) = &attribute(mesh, id, "tangent_sign")?.values
                else {
                    unreachable!("validated sign");
                };
                Ok::<_, Error>(v.clone())
            })
            .transpose()?;
        Ok(Self {
            binding: binding.clone(),
            normal: Vectors::read(mesh, binding.normal, "normal")?,
            tangent: binding
                .tangent
                .map(|id| Vectors::read(mesh, id, "tangent"))
                .transpose()?,
            signs,
        })
    }
    pub(crate) fn morph(
        &mut self,
        morph: &Morph,
        weight: f64,
        mesh: &Mesh,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<()> {
        if let Some(offsets) = &morph.normal_offsets {
            self.normal.morph(offsets, weight, mesh, cancelled)?;
        }
        if let Some(offsets) = &morph.tangent_offsets {
            self.tangent
                .as_mut()
                .ok_or_else(|| Error::new("reference", "morph tangent has no frame binding"))?
                .morph(offsets, weight, mesh, cancelled)?;
        }
        Ok(())
    }
    pub(crate) fn skin(
        &mut self,
        matrices: &[DMat3],
        mesh: &Mesh,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<()> {
        for i in 0..self.normal.values.len() {
            if cancelled() {
                return Err(Error::new("cancelled", "direction skinning cancelled"));
            }
            let m = matrices[self.normal.point(mesh, i)];
            if !m.is_finite() || m.determinant().abs() < 1e-12 {
                return Err(Error::new(
                    "singular_pose",
                    "blended direction transform is singular",
                ));
            }
            self.normal.values[i] = m.inverse().transpose() * self.normal.values[i];
        }
        if let Some(t) = &mut self.tangent {
            for i in 0..t.values.len() {
                if cancelled() {
                    return Err(Error::new("cancelled", "tangent skinning cancelled"));
                }
                let m = matrices[t.point(mesh, i)];
                if !m.is_finite() || m.determinant().abs() < 1e-12 {
                    return Err(Error::new(
                        "singular_pose",
                        "blended tangent transform is singular",
                    ));
                }
                t.values[i] = m * t.values[i];
                self.signs.as_mut().expect("validated signs")[i] *= m.determinant().signum() as f32;
            }
        }
        Ok(())
    }
    pub(crate) fn write(self, mesh: &mut Mesh) -> Result<()> {
        self.normal.write(mesh)?;
        if let Some(t) = self.tangent {
            t.write(mesh)?;
        }
        if let Some(id) = self.binding.tangent_sign {
            mesh.attributes
                .values_mut()
                .find(|a| a.id == id)
                .expect("validated sign")
                .values = AttributeValues::Scalar(self.signs.expect("validated signs"));
        }
        Ok(())
    }
}
