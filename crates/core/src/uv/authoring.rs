//! Versioned UV assets and preparation of ordinary, revision-bound commands.
use super::*;
use crate::document::{Command, Document, Principal};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub version: u32,
    pub mesh: String,
    pub sets: BTreeMap<Id, Layout>,
}
impl Asset {
    pub fn validate(&self, mesh: &Mesh) -> Result<()> {
        if self.version != 0 {
            return Err(Error::new("schema_version", "UV asset version unsupported"));
        }
        if self.sets.is_empty() || self.sets.len() > 8 {
            return Err(Error::new("budget", "UV asset requires 1..8 named sets"));
        }
        if self.mesh != mesh.content_id()? {
            return Err(Error::new("reference", "UV asset mesh mismatch"));
        }
        let mut names = BTreeSet::new();
        for (id, set) in &self.sets {
            if *id != set.attribute_id || set.mesh != self.mesh || !names.insert(&set.attribute) {
                return Err(bad("UV set identity, name or mesh mismatch"));
            }
            set.validate(mesh)?;
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        Ok(digest(&canonical(self)?))
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Unwrap { settings: Unwrap },
    Pack { attribute: Id, settings: Pack },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub base_revision: String,
    pub idempotency_key: String,
    pub entity: Id,
    pub source_mesh: String,
    pub source_uv_asset: Option<String>,
    pub operation: Operation,
    pub budget: Budget,
    pub max_added_bytes: u64,
}
pub struct Prepared {
    pub transaction: crate::document::Request,
    pub report: Report,
    pub uv_asset: String,
}
pub fn prepare(
    document: &Document,
    principal: &Principal,
    request: &Request,
    mut cancel: impl FnMut() -> bool,
) -> Result<Prepared> {
    check(&mut cancel)?;
    if !principal.can_write || principal.id.is_empty() {
        return Err(Error::new(
            "permission",
            "authenticated UV write capability required",
        ));
    }
    if request.version != 0 || !(16..=128).contains(&request.idempotency_key.len()) {
        return Err(Error::new(
            "request",
            "invalid UV request version or idempotency key",
        ));
    }
    let snapshot = document.snapshot();
    let source = snapshot
        .meshes
        .get(&request.source_mesh)
        .ok_or_else(|| Error::new("reference", "UV source mesh missing"))?;
    let prior = request
        .source_uv_asset
        .as_ref()
        .map(|key| {
            snapshot
                .uv_assets
                .get(key)
                .ok_or_else(|| Error::new("reference", "UV source asset missing"))
        })
        .transpose()?;
    if let Some(prior) = prior {
        prior.validate(source)?;
    }
    let mut computed = match &request.operation {
        Operation::Unwrap { settings } => unwrap(source, settings, &request.budget, &mut cancel)?,
        Operation::Pack {
            attribute,
            settings,
        } => {
            let set = prior
                .and_then(|p| p.sets.get(attribute))
                .ok_or_else(|| Error::new("reference", "UV packing source set missing"))?;
            pack(source, set, settings, &request.budget, &mut cancel)?
        }
    };
    let mut asset = prior.map(|a| a.as_ref().clone()).unwrap_or(Asset {
        version: 0,
        mesh: request.source_mesh.clone(),
        sets: BTreeMap::new(),
    });
    asset.mesh = computed.report.output_mesh.clone();
    for set in asset.sets.values_mut() {
        set.mesh = asset.mesh.clone();
    }
    asset
        .sets
        .insert(computed.layout.attribute_id, computed.layout);
    asset.validate(&computed.mesh)?;
    computed.report.output_bytes =
        (canonical(&computed.mesh)?.len() + canonical(&asset)?.len()) as u64;
    if computed.report.output_bytes > request.budget.max_output_bytes {
        return Err(Error::new(
            "budget",
            "UV mesh and complete asset exceed output budget",
        ));
    }
    let key = asset.content_id()?;
    let transaction = crate::document::Request {
        version: 0,
        base_revision: request.base_revision.clone(),
        idempotency_key: request.idempotency_key.clone(),
        max_added_bytes: request.max_added_bytes,
        commands: vec![
            Command::PutMesh {
                mesh: computed.mesh,
            },
            Command::PutUvAsset { asset },
            Command::SetUvAsset {
                entity: request.entity,
                source_mesh: request.source_mesh.clone(),
                source_asset: request.source_uv_asset.clone(),
                asset: Some(key.clone()),
            },
        ],
    };
    if document.retry(principal, &transaction)?.is_none() {
        if snapshot.revision()? != request.base_revision {
            return Err(Error::new("stale_revision", "UV request revision is stale"));
        }
        let entity = snapshot
            .entities
            .get(request.entity)
            .ok_or_else(|| Error::new("reference", "UV target entity missing"))?;
        if entity.mesh.as_ref() != Some(&request.source_mesh)
            || snapshot.uv_bindings.get(&request.entity) != request.source_uv_asset.as_ref()
        {
            return Err(Error::new(
                "stale_selection",
                "UV mesh or constraint asset changed",
            ));
        }
        if snapshot.procedural_bindings.contains_key(&request.entity) {
            return Err(Error::new(
                "uv_binding",
                "apply or clear procedural geometry before UV authoring",
            ));
        }
    }
    check(&mut cancel)?;
    Ok(Prepared {
        transaction,
        report: computed.report,
        uv_asset: key,
    })
}
