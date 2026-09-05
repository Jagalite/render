//! Storage envelopes use content identities; adapters own publication and durability.
use crate::{
    Error, Result, canonical, digest,
    document::{Document, Principal, Receipt, Request},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub format: u32,
    pub sequence: u64,
    pub previous: Option<String>,
    pub digest: String,
    pub payload: Vec<u8>,
}
impl Envelope {
    pub fn new(sequence: u64, previous: Option<String>, doc: &Document) -> Result<Self> {
        let payload = canonical(doc)?;
        Ok(Self {
            format: 0,
            sequence,
            previous,
            digest: digest(&payload),
            payload,
        })
    }
    pub fn decode(&self) -> Result<Document> {
        if self.format != 0 || digest(&self.payload) != self.digest {
            return Err(Error::new("integrity", "invalid storage envelope"));
        }
        let doc: Document = serde_json::from_slice(&self.payload)?;
        doc.snapshot().validate()?;
        Ok(doc)
    }
}
/// Adapters guarantee all-or-error publication to their advertised durability class.
pub trait JournalStore {
    fn load(&self) -> Result<Vec<Envelope>>;
    fn publish(&mut self, envelope: &Envelope) -> Result<()>;
}
/// Atomically persist a host-state transition (jobs/events) outside authored operations.
pub fn durable_update<S: JournalStore, T>(
    store: &mut S,
    doc: &mut Document,
    update: impl FnOnce(&mut Document) -> Result<T>,
) -> Result<T> {
    let records = store.load()?;
    if let Some(saved) = recover(&records)?
        && digest(&canonical(&saved)?) != digest(&canonical(doc)?)
    {
        return Err(Error::new("conflict", "storage state changed"));
    }
    let mut next = doc.clone();
    let value = update(&mut next)?;
    next.mark_durable();
    let envelope = Envelope::new(
        records.len() as u64,
        records.last().map(|e| e.digest.clone()),
        &next,
    )?;
    store.publish(&envelope)?;
    *doc = next;
    Ok(value)
}
pub fn recover(records: &[Envelope]) -> Result<Option<Document>> {
    let mut previous = None;
    let mut latest = None;
    for (sequence, e) in records.iter().enumerate() {
        if e.sequence != sequence as u64 || e.previous != previous {
            return Err(Error::new("journal", "noncontiguous journal chain"));
        }
        latest = Some(e.decode()?);
        previous = Some(e.digest.clone());
    }
    Ok(latest)
}
pub fn durable_execute<S: JournalStore>(
    store: &mut S,
    doc: &mut Document,
    p: &Principal,
    r: &Request,
) -> Result<Receipt> {
    let retained = doc.retry(p, r)?;
    let records = store.load()?;
    if let Some(saved) = recover(&records)?
        && digest(&canonical(&saved)?) != digest(&canonical(doc)?)
    {
        return Err(Error::new(
            "conflict",
            "storage root changed; reload before retry",
        ));
    }
    if !records.is_empty()
        && let Some(receipt) = retained.as_ref().filter(|r| r.durable)
    {
        return Ok(receipt.clone());
    }
    let mut next = doc.clone();
    let mut receipt = match retained {
        Some(receipt) => receipt,
        None => next.execute(p, r)?,
    };
    next.mark_durable();
    receipt.durable = true;
    let envelope = Envelope::new(
        records.len() as u64,
        records.last().map(|r| r.digest.clone()),
        &next,
    )?;
    store.publish(&envelope)?;
    *doc = next;
    Ok(receipt)
}

#[derive(Default)]
pub struct MemoryStore {
    pub records: Vec<Envelope>,
    pub quota: usize,
    pub fail_publication: bool,
}
impl JournalStore for MemoryStore {
    fn load(&self) -> Result<Vec<Envelope>> {
        Ok(self.records.clone())
    }
    fn publish(&mut self, e: &Envelope) -> Result<()> {
        if self.fail_publication {
            return Err(Error::new("storage", "injected publication failure"));
        }
        if e.sequence != self.records.len() as u64
            || e.previous != self.records.last().map(|r| r.digest.clone())
        {
            return Err(Error::new("conflict", "publication sequence changed"));
        }
        let bytes = self.records.iter().map(|e| e.payload.len()).sum::<usize>() + e.payload.len();
        if self.quota > 0 && bytes > self.quota {
            return Err(Error::new("quota", "storage quota exceeded"));
        }
        e.decode()?;
        self.records.push(e.clone());
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use super::*;
    use std::{
        fs::{self, File, OpenOptions},
        io::Write,
        path::{Path, PathBuf},
    };
    fn io(e: std::io::Error) -> Error {
        Error::new("storage", e.to_string()).at("storage")
    }
    #[derive(Serialize, Deserialize)]
    struct NativeRecord {
        format: u32,
        sequence: u64,
        previous: Option<String>,
        digest: String,
        document: serde_json::Value,
    }
    fn visit_meshes(
        value: &mut serde_json::Value,
        mut visit: impl FnMut(&mut serde_json::Value) -> Result<()>,
    ) -> Result<()> {
        fn snapshot(
            value: &mut serde_json::Value,
            visit: &mut impl FnMut(&mut serde_json::Value) -> Result<()>,
        ) -> Result<()> {
            let meshes = value
                .get_mut("meshes")
                .and_then(serde_json::Value::as_object_mut)
                .ok_or_else(|| Error::new("native_format", "missing mesh table"))?;
            for mesh in meshes.values_mut() {
                visit(mesh)?;
            }
            Ok(())
        }
        snapshot(&mut value["snapshot"], &mut visit)?;
        if let Some(jobs) = value["jobs"]["jobs"].as_object_mut() {
            for job in jobs.values_mut() {
                if job["render_input"].is_object() {
                    snapshot(&mut job["render_input"]["snapshot"], &mut visit)?;
                }
            }
        }
        if let Some(journal) = value["journal"].as_array_mut() {
            for entry in journal {
                if let Some(commands) = entry["commands"].as_array_mut() {
                    for command in commands {
                        if command["operation"] == "put_mesh" {
                            visit(&mut command["mesh"])?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Fault {
        None,
        AfterWrite,
        AfterFileSync,
        AfterRename,
        AfterDirectorySync,
    }
    pub struct NativeStore {
        root: PathBuf,
        pub fault: Fault,
        boundary_hook: Option<Box<dyn Fn(Fault) + Send + Sync>>,
    }
    impl NativeStore {
        pub fn open(path: impl AsRef<Path>) -> Result<Self> {
            fs::create_dir_all(path.as_ref()).map_err(io)?;
            Ok(Self {
                root: path.as_ref().to_owned(),
                fault: Fault::None,
                boundary_hook: None,
            })
        }
        fn fail(&self, at: Fault) -> Result<()> {
            if let Some(hook) = &self.boundary_hook {
                hook(at);
            }
            if self.fault == at {
                Err(Error::new("storage", "injected crash boundary"))
            } else {
                Ok(())
            }
        }
        /// Fault-injection seam for independently terminating the writer process.
        pub fn set_boundary_hook(&mut self, hook: impl Fn(Fault) + Send + Sync + 'static) {
            self.boundary_hook = Some(Box::new(hook));
        }
        fn chunk_path(&self, id: &str) -> Result<PathBuf> {
            let hash = id
                .strip_prefix("sha256:")
                .filter(|h| {
                    h.len() == 64
                        && h.bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
                .ok_or_else(|| Error::new("integrity", "invalid mesh chunk reference"))?;
            Ok(self.root.join("chunks").join(format!("{hash}.bin")))
        }
        fn compact(&self, e: &Envelope) -> Result<NativeRecord> {
            let mut document: serde_json::Value = serde_json::from_slice(&e.payload)?;
            fs::create_dir_all(self.root.join("chunks")).map_err(io)?;
            visit_meshes(&mut document, |value| {
                let mesh: crate::geometry::Mesh = serde_json::from_value(value.clone())?;
                let bytes = crate::mesh_codec::encode(&mesh)?;
                let id = digest(&bytes);
                let path = self.chunk_path(&id)?;
                if path.exists() {
                    if digest(&fs::read(&path).map_err(io)?) != id {
                        return Err(Error::new("integrity", "existing mesh chunk is corrupt"));
                    }
                } else {
                    let staging = path.with_extension("pending");
                    let mut file = File::create(&staging).map_err(io)?;
                    file.write_all(&bytes).map_err(io)?;
                    file.sync_all().map_err(io)?;
                    fs::rename(staging, path).map_err(io)?;
                }
                *value = serde_json::json!({"chunk":id});
                Ok(())
            })?;
            File::open(self.root.join("chunks"))
                .map_err(io)?
                .sync_all()
                .map_err(io)?;
            Ok(NativeRecord {
                format: 0,
                sequence: e.sequence,
                previous: e.previous.clone(),
                digest: e.digest.clone(),
                document,
            })
        }
        fn expand(&self, mut record: NativeRecord) -> Result<Envelope> {
            if record.format != 0 {
                return Err(Error::new("native_format", "unsupported package version"));
            }
            visit_meshes(&mut record.document, |value| {
                let id = value["chunk"]
                    .as_str()
                    .ok_or_else(|| Error::new("native_format", "missing chunk reference"))?;
                let bytes = fs::read(self.chunk_path(id)?).map_err(io)?;
                if digest(&bytes) != id {
                    return Err(Error::new("integrity", "mesh chunk checksum mismatch"));
                }
                *value = serde_json::to_value(crate::mesh_codec::decode(&bytes)?)?;
                Ok(())
            })?;
            let envelope = Envelope {
                format: 0,
                sequence: record.sequence,
                previous: record.previous,
                digest: record.digest,
                payload: canonical(&record.document)?,
            };
            envelope.decode()?;
            Ok(envelope)
        }
    }
    impl JournalStore for NativeStore {
        fn load(&self) -> Result<Vec<Envelope>> {
            let mut paths = Vec::new();
            for entry in fs::read_dir(&self.root).map_err(io)? {
                let path = entry.map_err(io)?.path();
                if path.extension().is_some_and(|s| s == "json") {
                    paths.push(path);
                }
            }
            paths.sort();
            let mut records = Vec::new();
            for path in paths {
                let bytes = fs::read(path).map_err(io)?;
                let record: NativeRecord = serde_json::from_slice(&bytes)?;
                records.push(self.expand(record)?);
            }
            recover(&records)?;
            Ok(records)
        }
        fn publish(&mut self, e: &Envelope) -> Result<()> {
            // OS advisory lock provides cross-process serialization; lock file remains reusable.
            let lock = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(self.root.join("writer.lock"))
                .map_err(io)?;
            lock.lock().map_err(io)?;
            let records = self.load()?;
            if e.sequence != records.len() as u64
                || e.previous != records.last().map(|e| e.digest.clone())
            {
                return Err(Error::new("conflict", "another writer published first"));
            }
            e.decode()?;
            let record = self.compact(e)?;
            let staging = self.root.join(format!("{:020}.pending", e.sequence));
            let final_path = self.root.join(format!("{:020}.json", e.sequence));
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&staging)
                .map_err(io)?;
            file.write_all(&canonical(&record)?).map_err(io)?;
            self.fail(Fault::AfterWrite)?;
            file.sync_all().map_err(io)?;
            self.fail(Fault::AfterFileSync)?;
            fs::rename(&staging, &final_path).map_err(io)?;
            self.fail(Fault::AfterRename)?;
            File::open(&self.root).map_err(io)?.sync_all().map_err(io)?;
            self.fail(Fault::AfterDirectorySync)?;
            Ok(())
        }
    }
}
