# Sparse displacement fixture (candidate)

`python3 fixtures/sculpt-displacements/generate.py OUTPUT` reproduces `grid.json`.
The original CC0-1.0 fixture contains an 8 by 8 triangular planar grid with stable
point IDs near u64::MAX and two neighboring absolute local-meter displacements.
The grid has 81 points and 128 triangles. The unchanged mesh remains the authored
base; point IDs in operation inputs are decimal strings. Rust consumes mesh JSON
without a JavaScript number round trip.

Qualification must author through the ordinary CLI and browser agent operation,
check invalid/stale inputs, render CPU/Metal/WebGPU, restore archive/OPFS state and
clear the edits back to the empty asset. No image goldens are generated here.
