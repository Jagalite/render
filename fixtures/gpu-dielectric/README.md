# Original ideal-interface fixtures

`generate.py <new-directory>` uses only Python's standard library to generate CC0
quad geometry, smooth frames, material recipes and sampled analytic expectations.
Three embedded GLBs have matching JSON/BIN sources: a single interface, a parallel
entry/exit slab and a single interface with a tilted authored shading normal.
A bounded emissive target gives a known transmitted or reflected signal. Native
ideal dielectric is authored after import; no glTF transmission extension is claimed.

The generated16 analytic recipes cover IOR1/1.5/3, tint, eta² cancellation, Snell's
law with a narrow target, internal total reflection, grazing incidence, X/Z mirrors,
near/far clipping and MASK/BLEND. They use1x1 pixels,16 samples,seed42, zero point
and environment lighting, and depths2/3. Python independently evaluates the documented
counter samples and optical factors; no renderer result or golden image is an input.

`cases.json` adds six broader single/slab/frame, named-UV MASK/BLEND and morph-shutter
recipes. The CLI applies ordinary material/transform transactions, retries, renders,
exports and restores archives, and tests invalid/stale/cancel/budget requests.
The browser runs the same archived scenes and analytic checks through Rust/WebGPU
and OPFS, including animated frames and an ordered two-frame sequence.
