# HAIR input integration review

Review covers binary layout, existing native curve/material semantics, import
admission, derived budgets, source identity and native/browser persistence.

- Header and complete payload lengths are checked before indexed reads. Counts
  are capped before allocations, optional array offsets derive from bounded counts,
  and explicit byte order avoids host layout/alignment or format inference.
- Segment totals must exactly match points; each native curve validates position,
  radius, noncoincidence and stable control identity. Unused defaults are metadata,
  while every applied color/coverage value is validated before interpretation.
- Thickness and source XYZ are preserved under explicit metric policy. Uniform
  appearances group deterministically; within-strand color/transparency variation
  fails explicitly. No source data is silently averaged, baked or executed.
- Imported state remains native curves and materials. Disposable sweeps preflight
  actual aggregate resource counts before commands reach ordinary transactions.
  Native curve evaluation supplies cancellation and geometric tolerance failures.
- Existing generic typed import helper handles command/report admission. The new
  method is a durable root mutation; permission, retry, stale and publication failure
  semantics match existing APIs. Native reads reuse the source-local byte cap.
- No snapshot version, maintained shader, generated kernel or Cargo change is
  needed. The public agent DTO, registry and structural schema gain one method;
  core/native cancellation remains distinct from synchronous browser dispatch.
- Explicit Principled polygon-surface coverage is not a fiber BSDF. Source input
  arrays do not invent anchors or animated grooms. Positive-radius/finite-count
  source limits and per-surface alpha traversal remain part of the named profile.
- Independent manually-authored curves and header/data inspection complement image
  parity. Acceptance includes both byte orders, optional arrays, material/color and
  unit interpretation, aggregate sweep limits, invalid/cancel/stale/permission and
  durable recovery, plus prior artifact/shader identity. This review alone does not
  establish completion; packaged acceptance evidence records the actual results.

The initial cross-platform report equality check incorrectly treated disposable f64
JSON byte length as source identity. Actual evaluator receipts confirm this is a
platform-local resource observation; all authored fields and CPU pixels match. The
DTO now names observed_derived_json_bytes explicitly, and independent client paths
check its equality to each platform's actual conversion receipts. Exact semantic
report/revision and pixel checks remain. This qualification is not a geometry fix.
