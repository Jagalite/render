# Sculpt displacement hand-off — 2026-09-07

## Checkpoint and scope

The sparse sculpt candidate was checkpointed at `6502349`, based on `7bd2e25`.
The user subsequently requested one branch. All development branches were contained
in that checkpoint; the consolidated working branch is now **`main`**.
Qualification remains unfinished, and the implementation pause remains in effect.
Publishing and consolidating this checkpoint does not complete M10.

Use the primary checkout `/Volumes/seed2/Projects/render` on `main` for future work.
Remote: `git@github.com:Jagalite/render.git`, default branch `main`.
Older worktree directories are retained as detached checkouts for local evidence;
they are not active development branches. The original candidate artifacts remain at
`/Volumes/seed2/Projects/render/artifacts/worktrees/sculpt-displacements`.

Read [architecture](../../docs/architecture.md),
[the candidate contract](sculpt-displacements-contract.md),
[the integration review](sculpt-displacements-review.md),
[the profile](../../docs/sculpt_displacements.md), and ADR-041 in
[decision records](../../docs/decision_records.md) before continuing.

## Implemented

- Snapshot19 typed, content-addressed sparse displacement chunks, assets and entity
  bindings. Stable point/block IDs use decimal strings in the new operation schema.
- Ordinary PutSculptChunk/PutSculptAsset/compare-replace SetSculpt transactions;
  high-level `author_sculpt` through CLI and agent durable publication.
- Immutable base topology, triangle/node chunk replacement, fixed BVH partition and
  affected-leaf/ancestor refits. Cold reconstruction and warm refits agree exactly.
- A bounded sculpt-only session CPU cache, preserved on failure/cancellation.
  Root-render dispatch now forwards its cancellation callback. Browser agent CPU
  dispatch shares this core path; one-shot CLI and GPU previews reconstruct cold.
- Explicit source/resource/numerical limits and rejection of conflicting geometry,
  shading-frame, groom and material-displacement consumers. UV/color corner data is
  retained. Empty surfaces and degenerate packed-f32 triangles are rejected.
- Eleven native/Wasm tests, independent wire schemas, a reproducible high-ID fixture,
  native/Metal and browser WebGPU/OPFS workflows, and portable resource counters.

## Verified evidence

The final-source acceptance run completed successfully:
`artifacts/sculpt-displacements/run-20260907T041410Z`.

**106 checks, 243 native tests, 214 Wasm tests passed.** These include Clippy,
formatting, native/browser workflows, real Metal/WebGPU, OPFS recovery, and Linux/
Windows compile checks. Linux and Windows runtime behavior was not qualified.
The sculpt CLI workflow passed 28 calls. Native/Wasm CPU pixels and passes match
exactly; Metal/WebGPU maximum color error was `8.940696716308594e-08`, within the
existing `2e-5` tolerance. Portable native/Wasm resource counters match exactly.

Compact reports are committed in [sculpt-displacements-checkpoint](sculpt-displacements-checkpoint/):
validation report, tested package identities, source integrity, resource counters and
cross-platform comparison. At hand-off, the recorded Rust/Cargo input hashes were
checked against the current files. The validation report's log paths are relative to
the **local acceptance run**, not this compact checkpoint directory.

Full logs, rendered artifacts, development failures, binaries and local build caches
remain under ignored `artifacts/`; pushing this commit does not transfer them. A fresh
checkout must reproduce qualification or obtain the retained run artifacts. No final
`evidence/sculpt-displacements` bundle has been collected, and prior-output/shader/
dependency preservation has not yet been attested by the new collector.

Development failures are retained locally: an undersampled 16x16 image assertion
(fixed by retaining the assertion at 64x64), a mutable-session caller compile error,
and a Clippy let-and-return diagnostic. The final suite includes the subsequent
source-admission and root-render cancellation fixes.

## Resume in this order

1. Measure global snapshot validation, canonical serialization and SHA256 separately
   from local copy/refit counts. A small native development probe was planned but
   **has not been added**. Consume the initial/final archives from the acceptance
   run; retain raw timings, source/binary/input hashes and process resource evidence.
   Do not claim whole-engine speed or memory improvement from local counters.
   Snapshot revision calculation includes validation, serialization and hashing;
   label measurement boundaries accurately. Qualify any added probe separately;
   implementation changes require appropriate acceptance reruns.
2. Run `scripts/collect-sculpt-displacements-evidence.py` with the run directory and
   `evidence/sculpt-displacements` as its arguments. Set
   `RENDER_SCULPT_PREVIOUS_RUN` to the predecessor run below. The collector and
   `scripts/compare-sculpt-probe.py` are prepared but **have not been executed**.
   They must verify all 2,004 prior render/pass/receipt artifacts, prior fixtures,
   query/chunk contracts, unchanged shaders/dependencies, and 26 PBR probe artifacts
   against the preserved `7bd2e25` binary. Investigate failures; do not rebaseline.
3. Finish the evidence README/manifests, preserve qualified package copies, and
   reconcile final source/package identities. Update ADR-041, the integration review,
   profile documentation, feature matrix and M10 subgate only after evidence passes.
4. Review the final diff and commit qualification updates directly on `main`.
   Keep one development branch unless the user requests a different workflow.
   The consolidated checkpoint remains an experimental candidate, not a release.

The predecessor's repeated PBR process-footprint increase remains unexplained.
Keep that observation visible. Global admission, canonical hashing, complete root
reference tables, instance bounds and GPU host packing retain global work.
For the 2,048-triangle resource case, one changed point affects six triangles and
35 BVH nodes, copying 128 triangle and 512 node payloads; the bounds scan still
visits 6,144 vertices. Actual layouts exclude allocator/Arc headers and private map
allocation layout; basis/current layouts can share chunks.

Brushes, masks, face sets, symmetry, multiresolution, remesh, retopology, broader
painting/baking and the deferred M09 editor remain open. Future engine work should
continue through bounded headless interfaces; this candidate does not complete M10.

## Reproduction and local resources

Run `python3 scripts/validate-sculpt-displacements.py` from the checkout being
qualified. Existing absolute artifact paths below refer to the original candidate
checkout; preserve or explicitly configure those baseline resources when using main.
It configures offline builds, two Cargo jobs, no incremental compilation and no
native dev/test debug data. It requires the following baseline inputs and local
Chrome/server setup. The local helper
`artifacts/sculpt-displacements/development/run-acceptance.py` records their exact
values; equivalent environment settings are:

```sh
RENDER_BLEND_REFERENCE_PATH=/Volumes/seed2/Projects/render/artifacts/worktrees/spatial-queries/artifacts/spatial-queries/run-20260907T001728Z/blend-workflow/startup-293-source/source/original.blend
RENDER_BLEND_BASELINE_HOST=/Volumes/seed2/Projects/render/artifacts/worktrees/blend-static-profile/artifacts/blend-static/baseline/render-host
RENDER_UV_BASELINE_HOST=/Volumes/seed2/Projects/render/artifacts/worktrees/uv-authoring/artifacts/uv-authoring/baseline/render-host
RENDER_PAINT_BASELINE_HOST=/Volumes/seed2/Projects/render/artifacts/worktrees/tiled-painting/artifacts/tiled-painting/baseline/render-host
RENDER_SPATIAL_BASELINE_HOST=/Volumes/seed2/Projects/render/artifacts/worktrees/spatial-queries/artifacts/spatial-queries/baseline/render-host
RENDER_SCULPT_PREVIOUS_RUN=/Volumes/seed2/Projects/render/artifacts/worktrees/runtime-geometry-chunks/artifacts/runtime-geometry-chunks/run-20260907T023350Z
```

Export these variables when invoking the relevant scripts. Baseline sculpt probe:
`artifacts/sculpt-displacements/baseline/render-host`, SHA256
`a419368056efe7694dccca8f13e92e9f796eaadc1fba2c4ec875a497bf9ece49`.
The independent glTF validator was restored under
`/private/tmp/render-gltf-validator-20260906/node_modules/gltf-validator`
(version `2.0.0-dev.3.10`); recheck local tool prerequisites before rerunning.

At pause, Chrome CDP used port 9224 and the candidate web server used port 8785,
serving this checkout's `web/`. Port 8784 serves the older storage checkout.
These services may be gone by the next session: verify rather than assume they are
running. The shared Cargo target is
`/Volumes/seed2/Projects/render/artifacts/build-cache-20260906`.
No build/test job remained active when the user paused.

Never edit Rust, Cargo files or embedded fixtures while Cargo is running. Keep
source/package provenance exact, honor the fully Rust runtime boundary and ordinary
transaction authority, and do not suppress diagnostics or regenerate old goldens.
