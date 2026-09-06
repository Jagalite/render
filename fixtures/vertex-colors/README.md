# Original linear vertex-color oracle

CC0-1.0. `generate.py NEW_DIRECTORY` produces five source encodings of two quads
sharing one source material. All positions, UVs, color arrays and the 1x1 PNG are
original; provenance includes SHA-256 identities. No renderer output is an input.

The left quad has local u=(x+1)/2, v=(y+1)/2 and linear RGB=(u,v,1-u).
RGBA variants have alpha=u; VEC3 RGB has implicit alpha=1. The right quad has no
color attribute and must default to white without cloning the source material.
Float, normalized u8/u16 and zero-base sparse RGBA must produce identical pixels
in the OPAQUE profile. The base factor is (.6,.4,.2), the sRGB texture bytes are
(128,192,255), factor alpha is .75, and texture alpha is 128/255. Emission
(.1,.2,.3) is independent of vertex colors. Colors are linear, not sRGB decoded.

Tests modify in-memory source variants for out-of-range float clamping, malformed
accessors and MASK/BLEND coverage. These deliberately modified sources are not
claimed as independently valid fixture files. Native authored point/corner colors,
seams, displacement, modeling, baking, transactions and snapshot-v14 persistence
have separate analytic tests. The bounded base GPU profile supports OPAQUE only.
