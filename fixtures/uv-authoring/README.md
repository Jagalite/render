# UV authoring fixture contract

Original CC0-1.0 inputs create a one-meter cube and an eight-by-eight linear checker
through ordinary commands. Unwrap cuts all twelve stable edge IDs into six disk
charts; packing requests a 64-by-64 atlas, one-pixel padding and twelve pixels per
meter, preserving any pins. The two-pixel padding variant is an explicit no-fit negative. These are inputs, not generated golden images.

Run `python3 fixtures/uv-authoring/generate.py /tmp/uv-inputs` and compare every byte
with `data`. The workflow must unwrap and pack through the CLI and shared browser
API, preserve source meshes and named-set intent, reject stale/cancelled/over-budget
requests, recover the archive and reproduce rendered output on CPU/Metal/WebGPU.
Independent analytic coordinates, chart areas and constraints live in core tests.
