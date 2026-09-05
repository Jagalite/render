//! Trusted in-process ABI v1 (v0 negotiation retained). The only unsafe operations copy caller-owned
//! byte slices; scene mutation remains in the safe transactional core.
use render_core::{document::*, *};
use serde::Deserialize;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Mutex, OnceLock},
};
enum Entry {
    Engine(Box<agent::Session>),
    Buffer(Vec<u8>),
}
struct Slot {
    generation: u32,
    entry: Option<Entry>,
}
#[derive(Default)]
struct Registry {
    slots: Vec<Slot>,
}
impl Registry {
    fn insert(&mut self, entry: Entry) -> u64 {
        let live = self.slots.iter().filter(|s| s.entry.is_some()).count();
        let used: usize = self
            .slots
            .iter()
            .filter_map(|s| match &s.entry {
                Some(Entry::Buffer(b)) => Some(b.len()),
                _ => None,
            })
            .sum();
        let added = match &entry {
            Entry::Buffer(b) => b.len(),
            _ => 0,
        };
        if live >= 128 || used.saturating_add(added) > 32 * 1024 * 1024 {
            return 0;
        }
        if let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.entry.is_none() && s.generation < u32::MAX)
        {
            slot.entry = Some(entry);
            return (u64::from(slot.generation) << 32) | (index as u64 + 1);
        }
        if self.slots.len() >= u32::MAX as usize {
            return 0;
        }
        let handle = self.slots.len() as u64 + 1;
        self.slots.push(Slot {
            generation: 0,
            entry: Some(entry),
        });
        handle
    }
    fn slot(&mut self, h: u64) -> Result<&mut Slot> {
        let index = (h as u32)
            .checked_sub(1)
            .ok_or_else(|| Error::new("stale_handle", "zero handle"))? as usize;
        self.slots
            .get_mut(index)
            .filter(|s| s.generation == (h >> 32) as u32 && s.entry.is_some())
            .ok_or_else(|| Error::new("stale_handle", "invalid generation or released handle"))
    }
    fn remove(&mut self, h: u64, buffer: bool) -> Result<()> {
        let slot = self.slot(h)?;
        if matches!(slot.entry, Some(Entry::Buffer(_))) != buffer {
            return Err(Error::new("handle_type", "wrong handle kind"));
        }
        slot.entry = None;
        slot.generation = slot.generation.saturating_add(1);
        Ok(())
    }
}
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}
fn boundary<T: Copy>(failure: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(failure)
}
#[derive(Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
enum ApiRequest {
    Inspect,
    Execute { request: Request },
    Registry,
    Agent { request: agent::Request },
}
extern "C" fn create(high: u64, low: u64) -> u64 {
    boundary(0, || {
        let Ok(doc) = Document::new(Snapshot::empty(Id(
            (u128::from(high) << 64) | u128::from(low)
        ))) else {
            return 0;
        };
        let Ok(mut registry) = registry().lock() else {
            return 0;
        };
        registry.insert(Entry::Engine(Box::new(agent::Session::new(doc))))
    })
}
extern "C" fn destroy(handle: u64) -> i32 {
    boundary(-1, || {
        registry()
            .lock()
            .ok()
            .and_then(|mut r| r.remove(handle, false).ok())
            .map_or(-1, |_| 0)
    })
}
/// # Safety
/// `input` must reference `length` initialized readable bytes for the duration of
/// this call. The caller must not mutate those bytes concurrently. Length <=16 MiB.
unsafe extern "C" fn request(handle: u64, input: *const u8, length: u64) -> u64 {
    boundary(0, || {
        if input.is_null() || length == 0 || length > 16 * 1024 * 1024 {
            return 0;
        }
        // SAFETY: caller promises a readable region; bounds above fit usize/isize on supported hosts.
        let bytes = unsafe { std::slice::from_raw_parts(input, length as usize) };
        let Ok(mut registry) = registry().lock() else {
            return 0;
        };
        let result = (|| -> Result<serde_json::Value> {
            let slot = registry.slot(handle)?;
            let Some(Entry::Engine(doc)) = &mut slot.entry else {
                return Err(Error::new("handle_type", "engine handle required"));
            };
            let query: ApiRequest = serde_json::from_slice(bytes)?;
            match query {
                ApiRequest::Inspect => Ok(
                    serde_json::json!({"revision":doc.document().snapshot().revision()?,"snapshot":doc.document().snapshot(),"durability":"memory-only local ABI"}),
                ),
                ApiRequest::Execute { request } => Ok(serde_json::to_value(doc.execute_root(
                    &Principal {
                        id: "trusted-native-client".into(),
                        can_write: true,
                    },
                    &request,
                )?)?),
                ApiRequest::Agent { request } => doc.dispatch(
                    &Principal {
                        id: "trusted-native-client".into(),
                        can_write: true,
                    },
                    request,
                ),
                ApiRequest::Registry => Ok(serde_json::to_value(render_core::api::registry())?),
            }
        })();
        match canonical(&result) {
            Ok(bytes) => registry.insert(Entry::Buffer(bytes)),
            Err(_) => 0,
        }
    })
}
extern "C" fn response_size(handle: u64) -> u64 {
    boundary(0, || {
        let Ok(mut registry) = registry().lock() else {
            return 0;
        };
        let Ok(slot) = registry.slot(handle) else {
            return 0;
        };
        match &slot.entry {
            Some(Entry::Buffer(bytes)) => bytes.len() as u64,
            _ => 0,
        }
    })
}
/// # Safety
/// `output` must reference `capacity` writable bytes, exclusively borrowed for this
/// call. No engine or response memory is loaned to the caller.
unsafe extern "C" fn response_read(handle: u64, output: *mut u8, capacity: u64) -> i32 {
    boundary(-1, || {
        let Ok(mut registry) = registry().lock() else {
            return -1;
        };
        let Ok(slot) = registry.slot(handle) else {
            return -1;
        };
        let Some(Entry::Buffer(bytes)) = &slot.entry else {
            return -1;
        };
        if output.is_null() || capacity < bytes.len() as u64 {
            return -2;
        }
        // SAFETY: caller owns output; source remains pinned under the registry lock.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
        }
        0
    })
}
extern "C" fn release(handle: u64) -> i32 {
    boundary(-1, || {
        registry()
            .lock()
            .ok()
            .and_then(|mut r| r.remove(handle, true).ok())
            .map_or(-1, |_| 0)
    })
}
#[repr(C)]
pub struct ApiTable {
    pub struct_size: u32,
    pub abi_version: u32,
    pub create: extern "C" fn(u64, u64) -> u64,
    pub destroy: extern "C" fn(u64) -> i32,
    pub request: unsafe extern "C" fn(u64, *const u8, u64) -> u64,
    pub response_size: extern "C" fn(u64) -> u64,
    pub response_read: unsafe extern "C" fn(u64, *mut u8, u64) -> i32,
    pub release: extern "C" fn(u64) -> i32,
}
static API_V0: ApiTable = ApiTable {
    struct_size: std::mem::size_of::<ApiTable>() as u32,
    abi_version: 0,
    create,
    destroy,
    request,
    response_size,
    response_read,
    release,
};
static API_V1: ApiTable = ApiTable {
    struct_size: std::mem::size_of::<ApiTable>() as u32,
    abi_version: 1,
    create,
    destroy,
    request,
    response_size,
    response_read,
    release,
};
/// Returns a process-lifetime immutable function table or null for incompatible ABI/size.
#[unsafe(no_mangle)]
pub extern "C" fn render_entry(version: u32, minimum_size: u32) -> *const ApiTable {
    let table = match version {
        0 => &API_V0,
        1 => &API_V1,
        _ => return std::ptr::null(),
    };
    if minimum_size > table.struct_size {
        std::ptr::null()
    } else {
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_and_stale_buffers() {
        let engine = create(1, 2);
        assert_ne!(engine, 0);
        let input = br#"{"method":"inspect"}"#;
        // SAFETY: input/output buffers stay live for their explicit lengths.
        let buffer = unsafe { request(engine, input.as_ptr(), input.len() as u64) };
        let mut bytes = vec![0; response_size(buffer) as usize];
        assert_eq!(
            unsafe { response_read(buffer, bytes.as_mut_ptr(), bytes.len() as u64) },
            0
        );
        assert!(
            serde_json::from_slice::<serde_json::Value>(&bytes)
                .unwrap()
                .get("Ok")
                .is_some()
        );
        assert_eq!(release(buffer), 0);
        assert_eq!(response_size(buffer), 0);
        assert_eq!(destroy(engine), 0);
        assert_eq!(destroy(engine), -1);
        assert!(render_entry(2, 0).is_null());
        assert!(!render_entry(1, 0).is_null());
    }
}
