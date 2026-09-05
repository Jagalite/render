# M05 integration decisions

These decisions extend the accepted M00–M04 foundation. The full creative-suite
architecture and Rust computational boundary are unchanged.

## ADR-008 — Narrow ABI v1

Accept the six-function C-compatible table after independent Python ctypes and
compiled C11 lifecycle/buffer tests and the Python agent workflow. Retain v0
negotiation and independent operation versions. The stable boundary is the
entry point, fixed-width table, opaque handles, ownership and failure contract.
It does not freeze private Rust types or document serialization. The local ABI
is synchronous, serialized and memory-only. Durable execution and asynchronous
jobs remain host adapters. Revisit before adding callbacks, borrowed buffers,
new supported target ABIs or remote tenants.

## ADR-009 and ADR-012 — Restricted variants through shared transactions

A session owns at most three ephemeral branches. The branch endpoint admits
only typed lighting and existing material color/emission edits. A digest covers
all remaining authored properties, including hierarchy, bindings, textures,
camera, settings and geometry. Rendering pins a revision and records perception
passes. Selection produces a normal resolved transaction and conservatively
rechecks the original root. Native CLI, ABI and browser use this same Rust core.
Transient branches are intentionally not a serialized workspace contract.

The native job host now accepts validated pinned variant inputs and retains
idempotent submissions plus structured pass artifacts. Its worker never makes
a variant the authored root. External transactions and job metadata continue
to share journal publication and conflict checks. Revisit for resumable browser
jobs, larger workspaces, general property capabilities or collaboration merges.

## ADR-005 — Persist authored rendering intent and full storage identity

Snapshot version 1 adds typed render settings through a transaction command.
Version 0 documents remain readable with their previous hashes. Older runtimes
reject version 1, preventing silent loss of lighting on reopen. Native journal
envelopes keep their existing version and durability protocol.

Browser saves compare the full document digest under a Web Lock. Comparing
only the authored revision could lose a retained no-op transaction receipt.
Native agent retries likewise validate the requested store even when a retained
receipt makes the in-memory document unchanged; a new empty target is published.
Tests cover these two metadata-sensitive cases. Browser close is its published
storage class, not a native fsync claim.

## ADR-010 — Scoped glTF scene adapter

Add a distinct Rust scene importer beside the old mesh-only codec. The bounded
profile decodes a selected hierarchy, indexed/static triangle primitives and
shared mesh content; it retains normal/UV attributes and material bindings.
External resource bytes are supplied explicitly; no URI or imported text is
executed. IDs are derived once from namespace/source identity and recorded in
an import mapping, rather than borrowed runtime indices.

Khronos Box is unmodified, attributed to Cesium under CC-BY-4.0 and hash-pinned.
Lambertian conversion is opt-in and reported. Unsupported PBR, textures,
animations, cameras, skins, extensions and other geometry modes fail explicitly.
This accepts an M05 product fixture profile, not complete INPUT-01. No runtime
dependency was added. Revisit against the larger static-scene and PBR corpus.

## Failed checks and integration review

The first browser cancellation check failed: an immediate cancel arrived before
the Rust async body registered the preview. Admission now runs before returning
the Promise, and the same immediate-cancel assertion passes. No test tolerance
was relaxed. Review also found same-revision storage metadata conflicts, retries
to a different store, and newline control messages whose size was checked only
after allocation. Full document identity, explicit target validation and a
bounded stream reader address those cases; regressions exercise each boundary.

An early native workflow run took 11.57 seconds, with 33,325,056 bytes maximum
RSS and 16,107,136 bytes peak memory footprint. It preceded final document-version
and persistence fixes; current measurements are in `validation_report.json`.
Iteration-wide engineering time and machine cost were not measured. Strict
Clippy also caught a large request enum and a test initializer; both were fixed.
Existing M00–M04 reference artifacts were not regenerated or edited.
