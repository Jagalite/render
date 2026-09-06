# Original named UV fixture

CC0-1.0. All geometry, UV values and images were authored for this repository.
`generate.py` uses Python standard-library encoding to write glTF, GLB and embedded
PNG data; it never runs a renderer or updates a reference image. Reproduce into a
new directory, then compare its files with `data/provenance.json`:

```
python3 fixtures/named-uv/generate.py /tmp/render-named-uv-fixture-check
```

One two-triangle quad spans [-1,1] in XY. In normalized coordinates (x,y), UV0 is
(x,y), UV1 is (y,x), UV2 is (1-x,y), and UV7 is constant (1/3,2/3). UV1 uses
normalized u16; UV2 uses normalized u8 with a four-byte vertex stride. Images are
2x2, nearest-filtered and clamped. Base color uses UV0, emission and normal use
UV1, occlusion uses UV2, and metallic/roughness uses UV7.

At world point (-0.5,0.5,0), expected coordinates are (.25,.75), (.75,.25),
(.75,.75), and (1/3,2/3), respectively. Independently applying the sRGB transfer
function to the byte arrays in the generator produces the scalar color/emission
oracle. Normal UV1 has tangent +Y and negative handedness: the encoded tangent
normal's Y component maps to world +X. Tests additionally scale the selected set
to force its own mip footprint, select it for height displacement, bake its
emission, and test spatial alpha coverage.

The fixture is an analytic input, not a golden rendering. Core tests constrain
samples and transfer numerically; CPU/Metal and browser acceptance compare renders,
persistence, invalid/stale/cancelled operations and resource limits.

Source-conformance correction: UV3 through UV6 explicitly alias the existing
constant UV7 accessor, so all eight set indices are consecutive. Binary geometry,
image and animation payloads are unchanged. The morph source additionally declares
its unchanged [0,1] animation-input bounds. See
[the source validation contract](../../docs/gltf_source_conformance.md).
