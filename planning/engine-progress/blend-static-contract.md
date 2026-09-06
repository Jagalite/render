# Native .blend static profile contract

Implementation based on 9f7b4bd, with final qualification in evidence/blend-static.
Apply architecture 21/M13 and INPUT-06. This contract defines the bounded profile.
No Blender process, Python runtime or conversion service implements application work.

First implement bounded Rust container/SDNA inspection with explicit pointer width,
endianness, version, checked data/count limits and cancellation. Serialized pointers
are opaque relocation identifiers. Use exact block identities, never dereference
addresses or infer allocation ownership from serialized byte lengths. The pinned
real 293 startup file contains overlapping serialized ranges for distinct opaque
identifiers; a global address-range rejection would reject valid data. Ambiguous
exact identifiers among addressable blocks, malformed layouts and missing referenced
blocks must fail. Advisory REND/TEST metadata is preserved but is not addressable;
the real fixture demonstrates a temporary REND token aliasing GLOB.

Semantic conversion requires version 293 and the qualified reference facts.
Select scene membership through collections, retain source object parent transforms,
polygon/corner geometry and supported material/camera/light semantics. Never use
cached object matrices or derived meshes as an undocumented substitute for authored
evaluation. Diagnose active unsupported modifiers, drivers, constraints, animation,
linked libraries, geometry and node semantics before publication. Unreachable UI or
orphan data can remain preserved source bytes; it is not evaluated support.

Design a typed, immutable source-container asset and ordinary transactional insertion
before crossing the snapshot/API boundary. A tentative snapshot 16 extension must
omit empty defaults and retain old hashes, negotiate old-client versions, and keep
raw source provenance distinct from editable native semantics. No public schema may
serialize private SDNA indexes or Rust layouts. Publish the exact source/version
limits only after measuring canonical JSON and memory amplification.

Use original CC0 synthetic container fixtures for 32/64-bit LE/BE, malformed headers,
DNA/count/layout errors, pointer failures, cancellation and numerical decoding. Pin
actual licensed source bytes and compare independent development facts before any
real-file compatibility claim. The initial inspected official 293 startup file is
804804 bytes from cb886aba06d562ee629f2ee64f3692d008c68a35; its source/license evidence
is in /private/tmp/render-blend-research-20260906. Do not relabel it CC0. Resolve
redistribution provenance before including it as a tracked fixture.

Completion requires native/Wasm semantic parity, CLI and browser import/save/reopen/
render, exact source recovery, permissions, stale/cancel/budget/storage failure
atomicity, numerical/reference comparisons, resource/provenance/dependency evidence,
previous artifact identity and a cross-domain integration review. A working binary
reader alone does not complete INPUT-06 or this gate. M09 remains deferred.

Development details qualified from the real source: REND is non-addressable advisory
metadata; raw material-slot flags occupy a four-byte padded block; a non-null
PartDeflect can be inactive when both deflect and forcefield are zero. Validate
those fields and reject active values. Preserve the exact padding/source bytes.
Synthetic fixtures exercise the same inactive record and padded slot layout.
