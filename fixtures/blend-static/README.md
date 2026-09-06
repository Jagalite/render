# Blender static profile fixtures

`generate.py` authors original CC0 analytic containers with a deliberately minimal
SDNA field set. They cover a parented quad, four corners with named UVs, a camera,
constant Principled/background nodes, one point light and an inactive force-field
record. Four pointer-width/byte-order variants describe the same evaluated scene.
A tilted camera case exercises nonzero rotations around all three axes. A second
case has two spatial instances sharing one mesh and material.
These synthetic containers test the Rust contract; they do not establish that
Blender itself accepts this deliberately reduced field set. Layout JSON is test
mutation metadata, not a maintained runtime schema. Regenerate into a temporary
directory and byte-compare before accepting a change.

Real-file development qualification uses the official Blender 2.93 startup file at
commit `cb886aba06d562ee629f2ee64f3692d008c68a35`, SHA256
`339bc1c5cdc7fb0f3a9250f66d9616b4511e42bf08feb31ccf01344b5db655ea`.
It is 804804 bytes and uses 64-bit little-endian relocation identifiers. The upstream
repository's COPYING points to its GPL license. This file is separate from our CC0
fixtures; neither its bytes nor source-containing archives are redistributed here.
`fetch-reference.py` retrieves pinned data and the referenced license as optional
development inputs. Product import/render code never performs that fetch or needs
a Blender process. Reference outputs must record their exact source hash.

Primary format/semantic references:
- https://github.com/blender/blender/blob/main/doc/blender_file_format/mystery_of_the_blend.html
- https://docs.blender.org/manual/de/2.93/render/shader_nodes/shader/principled.html
- https://docs.blender.org/manual/ja/2.93/render/lights/light_object.html
- https://docs.blender.org/manual/fr/2.93/render/cameras.html

The selected material conversion is an explicit approximation to the existing
single-scattering native BRDF. A point-light conversion uses isotropic power/4pi;
source finite radius and engine shadow controls are reported losses. This is not a
Cycles/Eevee image-equivalence corpus. Original source bytes remain inert provenance
while converted entities/materials use native editable semantics.
