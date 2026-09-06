# VOL3 density/emission import contract

Base646ed8b. Apply architecture8.2/15, M07 and INPUT-05; docs/m07_m08.md and
volumes.rs define the existing sparse-media semantics. Preserve fully Rust runtime,
immutable transactions, browser-local behavior, stable IDs and M09 deferral.

Read the published binary format only, without embedding Mitsuba/OpenVDB or invoking
a conversion process. Primary reference: Mitsuba3 volume documentation, retrieved
2026-09-06: https://mitsuba.readthedocs.io/en/stable/src/generated/plugins_volumes.html
The VOL3 file encodes little-endian float grid values, dimensions and a bounding box;
Mitsuba scene choices for filtering, wrapping, units and optical meaning are separate.

Admit one scalar density file and optional aligned three-channel linear-sRGB emission
file. Require explicit cell-constant reconstruction with zero outside, file-box versus
unit-cube bounds, meters per source unit, density/emission scale, world-meter optical
coefficients, phase anisotropy and quadrature step. Do not imply Mitsuba trilinear,
clamped or spectral scene equivalence. Reject other versions/encodings/channel layouts,
mismatched grids, nonfinite/negative values, bad extents, truncation and trailing data.

Bound aggregate input to4MiB, scan without allocating a dense decoded grid, omit exact
zero cells and stop before exceeding the existing16,384 occupied-cell profile. All-zero
data is an empty volume entity, not an invalid fabricated nonzero cell. Return typed
source/coordinate/interpretation, source digest, stable entity/asset and resource reports.
Persist native sparse values through existing PutVolume/CreateEntity/SetVolume commands;
raw VOL containers and source layout metadata are not native authoring history.

Expose the same core parser through native project import and a typed agent/browser
import operation. Reuse transactional import admission, retry, permissions and durable
publication. Preserve existing source/import responses and native snapshot schemas.
Extend the documented JSON Schema subset where the new source is represented.

Acceptance: original reproducible CC0 density/emission grids; independent header/data
checks; analytic homogeneous attenuation and emission, heterogeneous/cell-axis/unit
mapping, sparse/empty behavior, malformed/negative/over-budget/cancel/stale failures,
transaction retry/recovery, native/Wasm and actual Chrome CPU/OPFS workflows. GPU
volume transport remains explicitly unsupported. Record resource/dependency/source
provenance and unchanged prior images, receipts and four generated shaders. Only then
update capability status. External paging, trilinear fields, other formats and GPU
transport remain independent gates; volume import does not implement fluid simulation.
