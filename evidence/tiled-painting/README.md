# Tiled painting acceptance

The bounded `paint-tiles-v1` engine profile passed on UV baseline be20d1f. It edits
immutable color/mask tiles and explicit layers, continues checkpointed strokes,
undoes/redoes retained canvases and bakes ordinary PNG images for named UV material
binding. M10 sculpting, remeshing, retopology, broader painting/baking and M09 remain
separate; this is not an interactive editor completion claim.

The final run passed 85 acceptance checks,
213 native tests and 188 Wasm tests.
A supplementary review strengthened GPU depth/normal checks against the same tested
artifacts; `paint_pass_review.json` retains the comparator hashes and result.
The 48-request CLI workflow unwraps/packs a box, paints across
tile boundaries with pressure, adds a layer and mask, erases, checkpoints, recovers,
bakes, binds the image, renders and archives/reopens. Actual prior binaries reject
the new commands and snapshot18 checkpoints without creating state.

The exact packaged browser Wasm reproduces tile/image hashes, full snapshots,
numerical reports, revisions and CPU pixels/passes. OPFS recovery preserves edits.
CPU/Metal RMSE is 5.54225842208e-06; browser CPU/WebGPU RMSE is
5.54131434528e-06; Metal/WebGPU maximum channel error is
1.04308128357e-07. Painted output changes the original
checker render by up to 0.330267943 linear channel units.

The corpus retains 1880 prior render/pass/receipt
files and 205 prior fixture files byte for byte. All
five shaders and Cargo/resolved dependency/native-link profiles are unchanged.
The new painting input file reproduces identically. Runtime
evidence is macOS ARM64 and Chrome; Linux and Windows receive compile checks.

`resources.json` records per-operation work, touched tile copies, output bytes and
process observations. These are not allocator caps or comparative benchmarks;
observations were collected on a shared host. Dense bake and journal amplification
remain distinct from sparse tile edits. `development/` preserves failures and the
interrupted acceptance run for a duplicate endpoint dab found during review, plus
the corrected browser request-capture harness failure and hash-verified acceptance
continuation. The
new analytic regression requires nine dabs over eight spacings, including decimal
endpoints. Bake retries pin canvas/source identity even when PNG bytes match.
The new portable sRGB8 table preserves all 256 qualified native conversion values;
its Decimal64 analytic error bound and generator provenance are retained in
`srgb8_numeric_source.json`. This resolves 23 one-bit platform power differences.
Shared Rust f64 log2 rounded to f32 also preserves all 1,041 observed native mip
footprints, removing 144
platform logarithm differences; see `mip_lod_numeric_source.json`.
No render goldens were regenerated.

Reproduce using `scripts/validate-tiled-painting.py`, then
`scripts/collect-tiled-painting-evidence.py <run> <fresh-evidence-directory>`. Supply
`RENDER_BLEND_REFERENCE_PATH`, `RENDER_BLEND_BASELINE_HOST`,
`RENDER_UV_BASELINE_HOST`, `RENDER_PAINT_BASELINE_HOST` and
`RENDER_PAINT_PREVIOUS_RUN` from the retained workflow/baseline records. Serve the
exact package on localhost:8781 with Chrome CDP on 9224.
