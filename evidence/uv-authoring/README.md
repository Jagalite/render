# UV authoring acceptance

Bounded native Rust disk-chart unwrap, deterministic atlas packing and typed named
constraint assets passed on the c9b0188 baseline. This qualifies an M10 engine
subgate; painting, sculpting, retopology and the M09 interactive workflow remain open.

The final run passed 78 acceptance checks,
197 native tests and 172 Wasm tests.
The 28-request CLI workflow creates a checker box, unwraps
six charts, rejects a no-fit atlas, packs a valid atlas, adds another set, retries,
rejects stale/cancelled/over-budget writes, renders, archives and reopens. Actual
old-client binaries reject the new journal commands and snapshot17 checkpoint.

Chrome's exact packaged Wasm produces identical mesh/constraint hashes, numeric
reports, revisions and CPU pixels/passes. OPFS recovery preserves authoring state.
CPU/Metal RMSE is 4.58207438434e-09; browser CPU/WebGPU RMSE is
5.99162040346e-09; Metal/WebGPU maximum channel error is
2.98023223877e-08. Authored coordinates change checker
sampling by up to 0.308874581 linear channel units.

The corpus retains 1864 earlier render/pass/receipt
files byte for byte, 198 prior fixture files, all five
generated shaders and unchanged Cargo/resolved dependency/native-link profiles.
5 new input files reproduce identically. This is
macOS ARM64 and Chrome runtime evidence; Linux and Windows receive compile checks.

`resources.json` records observed process memory/time, complete authored output
bytes, solver work and atlas occupancy. These are observations, not allocator caps
or performance benchmarks. `development/` retains unsuccessful builds and the
fixture/old-client review corrections. No golden render references were regenerated.

Reproduce with `scripts/validate-uv-authoring.py`, then
`scripts/collect-uv-authoring-evidence.py <run> <fresh-evidence-directory>`. Supply
`RENDER_BLEND_REFERENCE_PATH`, `RENDER_BLEND_BASELINE_HOST`,
`RENDER_UV_BASELINE_HOST` and `RENDER_UV_PREVIOUS_RUN` as recorded in the workflow
and regression artifacts. The browser lane uses a local server on port 8780 and
Chrome CDP on port 9224. The original inputs and limits are documented in
`fixtures/uv-authoring/README.md` and `docs/uv_authoring.md`.
