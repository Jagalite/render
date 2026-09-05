# ABI alpha contract

Version 0 is an experimental process-local interface. `include/render.h` is generated from `crates/ffi/src/lib.rs`; `scripts/generate-abi-header.py --check` checks correspondence. `abi_conformance.json` records an independent Python ctypes client.

`render_entry(version, table_size)` returns a pointer to an immutable function table only when both values match. The table creates and destroys engines, executes requests, queries response size, copies a response into caller-owned bytes, and releases responses. Engine and response handles include kind and generation checks. A released handle cannot address a newly allocated object. Responses use the same version-0 JSON operation engine as Rust clients.

Caller pointers must be valid for their lengths. The engine never retains request pointers and never lends internal object pointers. Length and capacity failures return integer status without writing beyond caller capacity. Result handles must be released. No callbacks occur while the registry lock is held. All calls serialize through that lock.

Inspect, operation registry, transaction, idempotent retry, unsupported version/size, small output capacity, released response and stale engine are exercised by the independent client. Native durability is implemented by the host storage adapter, not by this memory-only ABI.

The ABI trusts its in-process caller and is not a plugin sandbox. Unwinds are caught at the boundary; a poisoned registry requires process restart. Invalid foreign pointers, aborts and fatal process faults cannot be contained by catching Rust unwinds. No stable Rust layout, domain-wide API freeze, or ABI v1 compatibility is asserted. M05 owns stabilization.
