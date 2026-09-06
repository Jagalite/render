# Sparse medium analytic corpus

Original Render project work, dedicated under CC0-1.0. No external scene or runtime.
`generate.py` calculates front-to-back RGB Beer integrals with Python `math.expm1`
and writes 17 cases. The Rust CPU and generated GPU implementations are independent
consumers; the fixture does not use either implementation to generate expected values.

Cases cover absorption/emission, vacuum, small and large optical depth, series
boundary, ordered overlaps, sparse holes, metric and negative scale, shear, camera
near/far clipping, camera inside a cell, a missed cell and zero-density emission.
The inside case explicitly includes the established 1e-5m camera ray minimum.

Reproduction must write to a temporary directory and compare bytes. Never overwrite
accepted data to make a failing implementation pass. Actual Metal ray tests also
exercise half-open faces and tiny offsets on both sides of a shared face. Existing
VOL3 source fixtures remain unchanged and serve as a separate boundary regression.
