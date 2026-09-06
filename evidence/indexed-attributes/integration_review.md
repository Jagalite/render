# Integration review

This correction changes source admission, fixture metadata and validation evidence.
It does not alter authored snapshot schemas, evaluated geometry math, render kernels
or material sampling. Indexed names are canonical unsigned decimal suffixes;
checking the preceding index bounds work by the number of provided attributes,
not by the largest declared index. Missing zero, gaps and overflow suffixes fail
with the shared glTF diagnostic. Supported-family admission remains separate.

Animation input min/max are required scalar arrays, converted to the accessor's
float32 domain and compared with decoded endpoints. Existing finite/increasing
rational time checks remain in force. Array/type/endpoint errors fail before any
transaction commits. Both public native and browser import paths share these checks.

The corrected fixtures preserve all binary values and independent analytical
oracles. Additional unused UV aliases exercise the actual eight-set boundary;
export-loss assertions now account for all seven omitted sets. A missing last UV
set remains a distinct material-reference error; an interior gap is a source-format
error. The CLI malformed fixture was updated accordingly and its atomicity gate
remains unchanged. Source-hash identity changes are expected and audited, while
render colors/depth/normals and pixel correspondence are constrained exactly.

Independent source validation is deliberately outside the product dependency graph.
The runtime Rust audit remains unchanged; the development oracle's provenance is
separate and reproducible. No reference-image baseline was rewritten, and the
previous source-validity overclaim is amended in both historical evidence READMEs.
No unresolved blocker remains for this bounded conformance correction.
