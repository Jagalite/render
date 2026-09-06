# Retained failures and costs

These are development results, not successful completion evidence. The final
validation run in the parent directory is the completion record.

- `faceted-glass-furnace.txt`: a strict unit-radiance assertion on an IOR 1.5 faceted
  sphere failed at 16 path events and 128 samples (pixel range 0.96875–1.0; test
  runtime 0.12 seconds). Internal reflections can retain paths beyond the named
  finite-depth policy, so exact infinite-depth furnace output was an invalid oracle
  for this fixture. The strict check was rebuilt as a normal-incidence parallel
  dielectric slab: eta factors cancel, there is no TIR and the remaining reflection
  tail is below 0.04^15. Its original 1e-5 output tolerance remains. Analytic Snell,
  Fresnel, TIR, coat/conductor reciprocity, PDF and furnace checks remain independent.
  The renderer's finite-depth bias was not hidden or relabeled as unbiased.
- `review-native-tests.txt`: the same sphere assertion stopped the review run after
  the modeling, animation, displacement and imaging tests passed. The final run
  includes the corrected analytic fixture and the mixed media/coat integration case.
- `early-integration.txt`: an earlier run failed the newly added loose-element
  displacement regression while implementation/test changes were being compiled.
  Final validation was restarted from stable Rust sources; the rejection, root
  cancellation and all inherited gates passed.

Earlier implementation investigations retained their artifact directories locally:
`artifacts/m07/*-01` and `artifacts/m08/character-01`. The first volume resource probe
failed macOS `sysctl kern.clockrate` access while the native workflow itself passed;
the final escalated `/usr/bin/time -l` run records valid RSS and footprint. Initial
color-vector scratch expectations were replaced with independently tabulated W3C
rational XYZ matrices before acceptance. No historical reference image was changed.
