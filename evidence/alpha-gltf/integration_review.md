# Import, material, camera and persistence review

- Source alphaMode/factor/cutoff feed existing typed Principled opacity; RGB and
  alpha retain distinct transfer functions. OPAQUE default fields/receipts remain
  unchanged, as checked against prior workflows.
- Finite cutoff >=0 extends the admitted value domain without changing field
  layout. Values >1 preserve the source threshold and are fully transparent.
  Older engines can reject those values; no silent clamp is used.
- Camera continuation retains the original segment, handles exhausted intervals
  before media, preserves the initial near distance and records ray-origin depth.
  Secondary rays remain independent of camera clipping.
- Imported commands use ordinary validation/transactions. Failure, retry, stale
  requests, cancellation and save/recovery preserve root authority. Browser empty-
  import admission is explicitly distinguished from native stale-write checking.
- GPU alpha is rejected, including failed output manifests with zero bundles.
  CPU success does not update the GPU scattering capability.
- No dependency/ABI field changes, reference-image regeneration, external resource
  fetching or imported metadata execution. M09 stays deferred.

Reviewed through code inspection, independently specified analytic tests and real
native/browser workflows. No unresolved issue is claimed closed by compilation alone.
