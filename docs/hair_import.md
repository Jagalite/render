# External HAIR strand import

The `hair-polyline-surface-v1` profile reads the published
[HAIR binary layout](https://www.cemyuksel.com/research/hairmodels/) using shared Rust.
It preserves root-to-tip polylines, positive varying thickness and source-derived
strand/control identities as native authored curves. It renders bounded polygon
sweeps with the existing Principled surface and coverage models. It does not infer
root attachments, spline smoothing, physical fiber scattering or animation.

The file supplies a fixed header, point positions and optional segment, thickness,
transparency and RGB arrays. Missing arrays use their corresponding header defaults.
Unused default fields are not applied or numerically validated. Information bytes
are inert source metadata; their contents contribute to the source digest and are
never interpreted as instructions. No C++ loader or foreign computation is used.

The importer requires an explicit policy:

```json
{
  "byte_order": "little_endian",
  "meters_per_unit": 1,
  "thickness": "diameter",
  "color_space": "linear_srgb",
  "transparency": "coverage_one_minus_transparency",
  "representation": "swept_polyline_surface",
  "roughness": 0.6,
  "tessellation": {
    "chord_error": 0.001,
    "radial_error": 0.01,
    "max_samples": 8192,
    "max_vertices": 65536,
    "max_depth": 18
  }
}
```

`byte_order` may also be `big_endian`, `thickness` may be `radius`, and `color_space`
may be `srgb`. Positions retain source XYZ orientation and scale to meters. Thickness
scales with the same unit factor, then halves when interpreted as diameter. There
is no inferred axis convention, color encoding or thickness meaning. sRGB input
converts through the existing Rust color function into native linear base color.

By default RGB and transparency must each be uniform within a strand; optional
arrays may vary between strands. Identical appearances share material/geometry groups.
The optional `point_attributes: "linear_rgba_f32"` policy selects the
[typed polyline RGBA profile](curve_colors.md), preserving within-strand variation
as native controls with an explicit f32 alpha rounding report. Values are not averaged or quantized into generated textures. Color and
transparency lie in [0,1], while positive thickness and finite positions must satisfy
native curve bounds. Zero-radius tips and adjacent coincident points are rejected.

Transparency means one minus surface coverage, using existing opaque or stochastic
BLEND semantics. It applies to each swept surface intersection, including entry and
exit, rather than describing absorption through a fiber. Materials are double-sided,
nonmetallic Principled surfaces with caller-selected roughness. Existing finite-depth,
alpha-traversal and CPU/Metal/WebGPU precision profiles still apply.

Input is bounded to 4 MiB, 2,048 strands, 16,384 controls and 128 appearance groups;
each strand requires 2..4,096 controls. The caller's native tessellation policy is
bounded to 32,768 samples and 65,536 vertices per asset. Actual preflight conversions
also enforce aggregate limits of 32,768 samples, 65,536 vertices and 131,072 triangles
across the imported asset groups. Preflight fails before publication if any curve
cannot meet its tolerances. There is no silent decimation or fabricated minimum radius.
These limits describe this input profile, not arbitrary source model compatibility.

The native project source is
`{"format":"hair","path":"strands.hair","policy":{...}}`.
The ordinary import operation supplies the base revision, idempotency key, mutation
budget and optional settings. It returns `receipt` and typed `import` report.
Native file reading enforces both source and aggregate request byte limits.

The agent/browser method is `import_hair`, with `bytes`, `policy`, `settings`,
`base_revision` and `idempotency_key`; it returns `receipt` and `report`. It retains
the empty-project import gate and 16 MiB JSON control limit. Existing `PutMaterial`,
`PutGeometry`, `CreateEntity` and `SetGeometry` transactions own all publication.
The default profile preserves its snapshot version; colored controls require v15.
The FFI dispatch shape remains unchanged. Browser dispatch is
synchronous; core/native cancellation checks parsing, sweep evaluation and admission
before atomic publication. Failed durable publication preserves the original root.

The report records source digest/flags/counts, interpretation policy, original strand
indices and point ranges, stable curve/entity/material/asset identities, authored
asset bytes and derived sample/vertex/triangle counts. `observed_derived_json_bytes`
is the platform-local serialized length of disposable f64 geometry; native/Wasm
math can change its final decimal spellings without changing authored identity or
rendered pixels. The workflow checks it against each actual evaluator receipt,
rather than requiring this observation to be identical across platforms. Control IDs
are retained in native assets in source point order. Native archives and browser
OPFS store the authored curves and materials exactly. Raw source containers and
information bytes remain caller-retained, not native source-container history.

Reproduce original CC0 fixtures with
`python3 fixtures/hair-import/generate.py <new-directory>`.
Run `python3 scripts/validate-hair-import.py` using the exact packaged browser at
localhost:8776 and isolated Chrome CDP at 9224. See the integration review and
acceptance evidence for the qualified corpus and platform/resource boundary.
