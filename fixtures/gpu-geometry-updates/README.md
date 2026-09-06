# GPU geometry update fixture contract

Original CC0-1.0 grid inputs contain 289 stable points and 512 triangles. A local
variant moves only the central point by 0.125 meters along local Z, preserving all
topology IDs and arrays. A dense variant raises every point by 0.5 meters, testing
fragmented-range fallback without changing topology. A resize variant adds one disconnected triangle outside
the main grid, preserving all earlier element IDs. The generator creates inputs,
not image references. Run `python3 fixtures/gpu-geometry-updates/generate.py <dir>`
and compare the resulting `grid-local-edit.json` with `data` byte for byte.

Use ordinary document transactions and immutable evaluated scenes. Measure a single
Renderer across unchanged input, a local edit, undo, topology resize, material and
camera changes. Compare every persistent-renderer result against a fresh renderer,
including color and auxiliary passes. Require a visible geometric change, exact
fresh/reused GPU output and bounded CPU/GPU differences. Test pre/post submission
cancellation, invalid/stale requests, device recreation and shader variant switches.

Retain actual upload/reallocation counts, byte ranges or byte counts, packed host
bytes and retained shadow bytes. A full host pack remains global work. Report dense
or fragmented fallback and geometry-sharing changes explicitly; this fixture alone
does not qualify localized sculpting or assert a timing improvement. Preserve all
previous render/fixture/shader/dependency artifacts. This bounded workflow is qualified in `evidence/gpu-geometry-updates`.
