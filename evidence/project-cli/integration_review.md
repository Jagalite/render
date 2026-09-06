# CLI integration review

No blocking findings remain within `project-cli-v0` after the final native suite
and independent CLI workflow. This is a local implementation review, not a
security certification or a change to reference-image approval policy.

- **Authority:** apply and file imports call `durable_execute` with the host-owned
  local principal. They do not directly change published snapshots. New native
  archives use a fresh journal directory and validated initial envelope. Ordinary
  API and CLI requests produce equal accepted delta receipts.
- **Cancellation:** a JournalStore adapter checks cancellation after candidate
  preparation and immediately before native publication. A controlled test proves
  cancellation then leaves the journal unchanged; cancellation during an admitted
  commit still yields a durable completed transaction. Rendering uses the existing
  evaluator/transport callbacks; the GPU watcher owns a drop guard so unwinding
  cannot leave the watcher waiting forever.
- **Output publication:** image/product bundles use fresh pending directories,
  synced files and rename. The final manifest records complete/failed/cancelled
  state and references only published bundles. Partial sequence tests verify
  surviving artifact bytes and unchanged authoring. Admitted publication and
  native driver initialization are explicitly non-interruptible boundaries.
- **Paths:** all import paths are caller-provided. glTF metadata does not resolve
  files or URLs. Authored view/bake labels stay metadata; host-generated numeric
  names determine output paths. Existing outputs and projects reject replacement.
- **Request validation:** explicit version, strict host fields, bounded control and
  aggregate imported bytes, output byte limits and wall/cancellation controls.
  Empty struct variants ensure unknown fields are rejected for no-argument methods.
  Native journal recovery and engine allocation profiles remain separately scoped.
- **Revision semantics:** read/render/export operations check the current recovered
  revision and then retain that immutable snapshot. Later concurrent edits cannot
  change the in-flight input. Apply/import retry behavior remains the engine's
  converted-command identity contract; no hidden import side ledger was invented.
- **Backend scope:** native GPU pack/profile validation runs before device creation.
  The depth-two rejection is preserved, while depth-one CPU/Metal comparison uses
  an explicit settings transaction on a restored project. CPU frame/sequence and
  imaging paths retain existing named policies.
- **Portability:** only the native host adapter and tests changed. Shared engine,
  GPU kernel, browser and ABI sources, plus Cargo manifests/lockfile, remain
  unchanged. Windows directory durability is explicitly compile-only; macOS
  artifact publication is the exercised native workflow.

The code path contains no fixture-generator calls. The new external fixture files
and client are test inputs to ordinary operations; they do not substitute for
missing engine implementations. M09's interactive editor, broader file adapters,
advanced GPU transport and later modeling/simulation profiles retain their gates.
