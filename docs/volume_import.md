# VOL3 density and emission import

The `vol3-cell-constant-v1` profile reads little-endian VOL version 3 float32 grids
through shared Rust code. Supply a one-channel density file and, optionally, an
aligned three-channel linear-sRGB emission file. Dimensions and file bounding boxes
must match exactly. The importer does not resolve paths or execute imported metadata;
filesystem access belongs only to the native adapter.

The [published VOL format](https://mitsuba.readthedocs.io/en/stable/src/generated/plugins_volumes.html#grid-based-volume-data-source-gridvolume)
contains dimensions, channel count, bounds and x-fast interleaved values. Filtering,
wrapping, spectral conversion, units and optical interpretation are scene choices,
not values encoded in the file. This profile requires explicit cell-constant sampling
with zero outside. It does not reproduce Mitsuba's default trilinear/clamped scene
interpretation. No Mitsuba, OpenVDB or other foreign computational runtime is used.

Every import supplies this policy; there are no implicit optical or unit defaults:

```json
{
  "bounds": "file",
  "reconstruction": "cell_constant_zero_outside",
  "meters_per_unit": 1,
  "density_scale": 1,
  "emission_scale": [1, 1, 1],
  "absorption": [0.5, 1, 2],
  "scattering": [0, 0, 0],
  "anisotropy": 0,
  "max_step_meters": 0.1
}
```

`bounds` is `file` or `unit_cube`. Coordinates and cell widths are multiplied by
`meters_per_unit`; coefficients remain per world meter. Extinction is density times
the sum of absorption and scattering. RGB emission is independently scaled radiance per world
meter, so zero-density emitting cells are valid. The native renderer integrates
cell extinction/emission analytically and uses bounded midpoint quadrature for
point-light single scattering. It does not add indirect in-scattering or simulation.

Aggregate binary input is at most 4 MiB. Native file reads enforce that cap before
allocating the full source. Decoding scans source bytes directly, retains at most
16,384 occupied cells and omits exact zero cells. An all-zero source creates an
empty entity without a fabricated volume asset. The existing native metric, optical,
coordinate and scene-wide cell limits still apply. Values must be finite and
nonnegative even when their scale is zero; scaled density/emission cannot exceed
1,000,000. Invalid headers, versions, encodings, channels, grid alignment, extents,
truncation and trailing bytes are explicit errors.

The native `project` import source is
`{"format":"vol","path":"density.vol","emission":"emission.vol","policy":{...}}`.
`emission` may be omitted or null. The ordinary operation includes a base revision,
idempotency key, transaction byte budget and optional render settings. It returns
`receipt` and typed `import` report. Paths are caller-supplied and relative to cwd.

The agent/browser operation is `import_vol`, with `bytes`, optional `emission_bytes`,
`policy`, `settings`, `base_revision` and `idempotency_key`. It returns `receipt` and
`report`, and uses the existing empty-project import gate. JSON control data also
has a 16 MiB cap; binary arrays count toward that wire limit. Native and browser
source values, policy, document ID and transaction fields produce the same revision
and source-derived IDs. The JSON Schema describes this DTO and both camera lenses;
cross-field and binary validity still require Rust checks.

Imports publish through ordinary `CreateEntity`, `PutVolume` and `SetVolume`
transactions. No snapshot schema or privileged mutation path is added. Cancellation
is checked during scanning, evaluation and before publication through the cancellable
core API; native CLI cancellation also gates durable publication. Browser dispatch
is synchronous and bounded, so this is not asynchronous browser import preemption.
Failed permission, revision, budget or storage admission leaves the root unchanged.
Explicit native archives and browser OPFS persistence retain exact sparse values.

The report includes both source digests, combined source identity, coordinate and
optical policy, dimensions, metric domain/cell widths, stable entity/asset IDs,
source/occupied cell counts and serialized native asset bytes. Raw VOL containers
and their source layout metadata are not stored as native authoring history. Retain
the report/source files when source provenance or later re-export is required.

Occupied media remains explicitly unsupported on Metal/WebGPU. Paging, trilinear
fields, other volume formats and GPU transport have separate acceptance gates.
Reproduce the original CC0 fixtures with
`python3 fixtures/volume-import/generate.py <new-directory>`. Run acceptance using
`python3 scripts/validate-volume-import.py`, the exact built browser package served
at localhost:8775 and isolated Chrome CDP at 9224.
