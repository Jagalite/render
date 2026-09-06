# Original analytic alpha layers

`generate.py <new-directory>` creates three original CC0 glTF/GLB fixtures without
rendering or regenerating reference images. Every fixture contains a green-emissive
front quad at z=1 and a red-emissive rear quad at z=0. The directly encoded 4x1 RGBA
PNG has white RGB and alpha bytes 0,85,170,255. Nearest sampling and an orthographic
camera produce four exact-width stripes. A 32x16 image with 64 samples and depth1,
no light/environment, should have R=1-coverage, G=coverage and B=0.

MASK uses the default cutoff 0.5: the left half is red, the right half green.
BLEND uses factor0.5: expected green stripe means are 0,1/6,1/3,1/2, with a declared
absolute sampling tolerance0.03. OPAQUE is entirely green despite textured alpha.
The deterministic Rust tests additionally cover cutoff boundaries including >1,
factor endpoints/defaults, two stacked layers (green mean0.75), backface culling,
double-sided normals, near/far clipping, orthographic depths2/3, invalid inputs,
transaction atomicity, cancellation and save/reload.

`data/provenance.json` records exact generated-byte hashes and authorship.
Rendered acceptance images are observations, not automatically approved goldens.
