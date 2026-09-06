# Static Blender 2.93 acceptance

179 native and 154 Wasm tests; 137 CLI calls across seven cases; exact native/Wasm import revisions and CPU pixels; actual Metal/WebGPU and OPFS recovery. 1308 prior images/passes and 436 receipts remain byte identical, with all five shaders and existing fixtures unchanged.

The seven cases comprise four original pointer-width/byte-order variants, a tilted
camera, two instances sharing a mesh/material, and one pinned official 2.93 startup
file. This qualifies `blend-293-static-v1`, with explicit material/light conversion
policy and exact original-source extraction. It does not qualify arbitrary Blender
files or writing native edits into Blender format. M09 remains deferred; INPUT-06
and the broader M13 milestone remain partial.

The real input has 804804 source bytes and
1914238 canonical typed-asset bytes. Its source hash,
independent mesh/camera/light facts and analytic comparisons are recorded in
`blend-workflow/reference_comparison.json`. Source-containing real archives and the
upstream binary remain outside tracked evidence. Original CC0 synthetic source
archives and rendered artifacts are included. Runtime imports never fetch files or
invoke Blender/Python. Fixture generators and reference fetches are development tools.

`validation_report.json` lists 74 checks and their commands.
`blend-workflow/cross_platform.json` contains numerical comparisons;
`resources.json` records per-process import/render memory and source/archive sizes;
`dependency_delta.json` and `dependency_inventory.json` audit the direct libm edge,
resolved features and platform links. `source_integrity.json`, `tested_package.json`
and `final_implementation_manifest.json` identify the tested source and artifacts.
Linux and Windows were compile-checked; runtime qualification is macOS ARM64 and
Chrome Wasm/WebGPU. Process observations are neither benchmarks nor allocator caps.

Development evidence retains the first native workflow expectation failure, the
native/Wasm atan mismatch and other compile/test refinements. The interrupted run
was stopped to reject negative source emission before zero-strength multiplication.
The final run uses the corrected source. Native/Wasm equality was not relaxed.
Copied text logs have normalized line endings and trailing whitespace only;
`log_normalization.json` records raw and copied hashes, with raw artifacts retained.

## Reproduction

Use the pinned toolchain and offline Cargo dependencies. Start isolated Chrome CDP
on port 9224 with WebGPU enabled and serve this checkout's `web` directory on 8779.
Use `fixtures/blend-static/fetch-reference.py <outside-repo-directory>` to fetch the
pinned development input. Build the pre-extension `9f7b4bd` host in a separate
checkout for old-client checks. Then run:

```sh
RENDER_BLEND_REFERENCE_PATH=<reference-directory>/startup-293.blend RENDER_BLEND_BASELINE_HOST=<baseline-checkout>/target/debug/render-host python3 scripts/validate-blend-static.py
python3 scripts/collect-blend-static-evidence.py <new-run-directory> <previous-gpu-media-run>
```

The acceptance script requires both explicit paths. The collector requires the
retained prior GPU-media run to compare its rendered artifacts and independently
regenerates both original fixture families into temporary directories. It refuses
to replace an existing evidence directory. The original run paths are recorded in
the JSON reports; substitute local paths when reproducing on another machine.
