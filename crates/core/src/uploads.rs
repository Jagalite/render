//! Authenticated staging: a checksum-verified upload cannot mutate a document by itself.
use crate::{Error, Id, Result, digest, geometry::Mesh};
use std::collections::BTreeMap;
struct Upload {
    principal: String,
    expected_digest: String,
    length: usize,
    bytes: Vec<u8>,
}
pub struct Uploads {
    entries: BTreeMap<Id, Upload>,
    budget: usize,
}
impl Uploads {
    pub fn new(budget: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            budget,
        }
    }
    pub fn begin(
        &mut self,
        principal: &str,
        id: Id,
        length: usize,
        expected_digest: String,
    ) -> Result<()> {
        if principal.is_empty() || length == 0 {
            return Err(Error::new(
                "upload",
                "authenticated nonempty upload required",
            ));
        }
        if self.entries.contains_key(&id) {
            return Err(Error::new("duplicate_id", "upload exists"));
        }
        let reserved = self
            .entries
            .values()
            .try_fold(0usize, |n, u| n.checked_add(u.length))
            .ok_or_else(|| Error::new("budget", "upload size overflow"))?;
        if reserved.checked_add(length).is_none_or(|n| n > self.budget) {
            return Err(Error::new(
                "budget",
                "staged upload exceeds reserved byte budget",
            ));
        }
        self.entries.insert(
            id,
            Upload {
                principal: principal.into(),
                expected_digest,
                length,
                bytes: Vec::new(),
            },
        );
        Ok(())
    }
    pub fn append(&mut self, principal: &str, id: Id, offset: usize, bytes: &[u8]) -> Result<()> {
        let u = self
            .entries
            .get_mut(&id)
            .ok_or_else(|| Error::new("not_found", "upload"))?;
        if u.principal != principal {
            return Err(Error::new(
                "permission",
                "upload belongs to another principal",
            ));
        }
        if offset != u.bytes.len() || offset.checked_add(bytes.len()).is_none_or(|n| n > u.length) {
            return Err(Error::new(
                "upload_range",
                "expected contiguous in-bounds upload chunk",
            ));
        }
        u.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub fn finish_mesh(&mut self, principal: &str, id: Id) -> Result<Mesh> {
        let u = self
            .entries
            .get(&id)
            .ok_or_else(|| Error::new("not_found", "upload"))?;
        if u.principal != principal {
            return Err(Error::new("permission", "upload owner required"));
        }
        if u.bytes.len() != u.length || digest(&u.bytes) != u.expected_digest {
            return Err(Error::new(
                "integrity",
                "upload is incomplete or checksum does not match",
            ));
        }
        let mesh: Mesh = serde_json::from_slice(&u.bytes)?;
        mesh.validate()?;
        self.entries.remove(&id);
        Ok(mesh)
    }
    pub fn cancel(&mut self, principal: &str, id: Id) -> Result<()> {
        if self
            .entries
            .get(&id)
            .is_none_or(|u| u.principal != principal)
        {
            return Err(Error::new("permission", "upload owner required"));
        }
        self.entries.remove(&id);
        Ok(())
    }
}
