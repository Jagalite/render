# Sparse glTF fixture contract

These are original variants of this repository's original animated ribbon. The
checker PNG is authored directly from binary PNG fields. All files are CC0-1.0;
[data/provenance.json](data/provenance.json) records input and output hashes.

`generate.py <new-directory>` writes fresh files and never replaces a reviewed
fixture. Dense, zero-base sparse/u16 UV, and interleaved-base sparse/u8 UV variants
encode identical points, skin palettes/weights, POSITION morphs, clips, normals
and texture coordinates. Sparse index component widths rotate through byte,
short and int. The byte UV variant has aligned interleaved base data; the short UV
variant has sparse UV overlays. JSON/binary resource bundles and GLBs are provided.

Four points form a two-joint ribbon; the shoulder turns 90 degrees at one second
and returns at two seconds. Root translation and stepped POSITION morph weights
retain the original closed-form animation reference. A nearest-filtered 2×2 PNG
has red, green, blue and white quarters. UV coordinates include one third, exactly
representable by both normalized integer encodings after float conversion.

Tests compare native geometry and texture content, known deformed positions,
dense/sparse image and pass identity, GLB and explicit-resource import, durable
restoration, browser-local GPU rendering, cancellation and transactional failure.
Source identities are distinct across encodings and remain scoped stable IDs;
image comparison across variants therefore compares visibility rather than raw IDs.
Within each native/browser variant, object IDs must match exactly.

No fixture here is a golden render. Negative variants are created in test memory
or fresh artifact directories. Original animated and static PBR fixtures remain
byte-for-byte unchanged.
