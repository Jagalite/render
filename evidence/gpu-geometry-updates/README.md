# GPU geometry update acceptance

The bounded `gpu-buffer-updates-v1` renderer prerequisite passed on painting baseline
24de830. It reuses exact buffers, patches changed aligned records, rewrites dense or
fragmented same-size inputs in place and reallocates changed sizes. M10 sculpting
and M09 remain open; full host evaluation/packing is still global.

The final run passed 89 acceptance checks,
217 native tests and 188 Wasm tests.
The 12-case actual GPU workflow authors mesh bindings through ordinary
procedural/document transactions and compares every result with a fresh renderer.
It covers local edits, retained-snapshot undo, dense edits, resize, camera/material
changes, cancellation, invalid/stale inputs and device recreation.

A one-point edit uploads 528 bytes in 33 queue writes out of
102352 packed bytes, without reallocating. Repeating the same input writes
zero geometry bytes. A broad edit triggers a 102352-byte rewrite in the
same allocation. Native and browser report identical decisions and byte counts;
native/Wasm CPU pixels are exact, and fresh/reused GPU pixels, passes and receipts
are exact within each backend. Cross-backend GPU comparison uses existing numerical
tolerances; see `gpu-update-workflow/cross_platform.json`.

`gpu_update_resources.json` measures the precompiled conformance process separately
from Cargo: 3.50593 seconds and 49266688 peak resident bytes on this
shared macOS host. This includes fixture transactions, CPU renders, multiple GPU
adapters and serialization. It is not a comparative benchmark or allocator cap.
`representation_costs.json` distinguishes packed host records, serialization, the
retained shadow and transferred GPU bytes; driver/staging and other forms are extra.

All 1896 prior render/pass/receipt files and
210 prior fixture files remain byte identical. All five shaders and
resolved native/browser packages, features and native links are unchanged. Cargo
adds only the web Wasm target's direct edge to already-pinned serde 1.0.228 with
existing derive/rc features. No new computational FFI or package is added.
The new grid input reproduces byte for byte, including its independent one-point,
dense and topology-resize variants. The measured resource replay reproduces the
main workflow's documents and render artifacts exactly.

Runtime evidence is macOS ARM64 Metal and Chrome WebGPU; Linux/Windows receive
compile checks. The exact served Wasm hash and actual native GPU test binary hash
are retained. `development/` preserves the initial missing-serde browser compile
failure and the first test output relocation caused by Cargo's package working
directory. These were corrected without weakening gates or regenerating goldens.

Reproduce with `scripts/validate-gpu-geometry-updates.py`, serving the candidate
package on localhost:8782 with Chrome CDP on 9224. Supply the retained Blender
reference and old Blend/UV/painting baseline binaries through the environment names
used by the validator. Collect with `scripts/collect-gpu-geometry-updates-evidence.py
<run> <fresh-directory>` and set `RENDER_GPU_UPDATES_PREVIOUS_RUN` to the qualified
painting run. Existing rendering receipts and snapshot schemas are unchanged.
