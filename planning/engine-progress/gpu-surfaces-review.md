# Conductor/coat integration review

Review scope: existing typed model semantics → immutable evaluated scene → private
GPU packing → Rust-generated shader → static/shutter/sequence publication.

- Native authoring fields remain eta/k or weight/IOR/roughness. No public data map,
  snapshot revision, ABI or source extension admission is added. Transactions still
  validate parameter ranges and stale revisions before publication.
- GPU records compile conductor F0 with the same f64 expression, then perform
  reflection in f32. Two vec4 records add32 bytes per extended instance and remain
  inside existing geometry-addressing and GPU allocation budgets. Packed geometry
  cache identity includes material records; no stale mutable snapshot is reused.
- Conductor sampling and PDF use GGX exclusively. Coat uses probability weight/2,
  CPU counter dimension2 and the same two-component PDF. Primary versus secondary
  sampling, alpha visibility and AO call paths are retained.
- Existing opaque and Principled-alpha shader hashes are pinned and pass after
  refactoring the IR call builders. The third pipeline has its own layout and error
  scopes and is discarded on renderer destruction. Actual Metal tests alternate
  opaque→alpha→conductor→coat→alpha→opaque.
- Shared scene/frame/sequence entry points select the new pipeline using packed
  profile2. Alpha failure markers and final readback publication are unchanged;
  both new models run the nominal/temporal/partial-sequence failure fixture.
- Two old unsupported-material tests used coat. They now use the remaining
  dielectric exclusion; the CLI fixture also sets its required roughness/metallic0.
  New positive conductor/coat workflows retain meaningful GPU admission coverage.
  Failed acceptance logs record the obsolete assertion and invalid initial replacement.
- Precision restrictions remain explicit: compiled F0, coat parameters, sampled
  branches and grazing transport can differ from CPU f64. This is a named optical
  approximation with numerical/analytic checks, not arbitrary layered-material parity.

The completed acceptance report and original artifacts accompany this review.
Full M09 editing and dielectric/media GPU support remain separate gates.
