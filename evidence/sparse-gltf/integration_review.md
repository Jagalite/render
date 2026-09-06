# Sparse accessor integration review

The decoder boundary was reviewed with the Khronos glTF2 specification and sparse
index/value schemas. Dense data stays borrowed; missing-base storage is bounded
and reused; sparse source views are range/alignment checked before copying. All
indices are validated as strictly increasing and in range. No unchecked source
index becomes a durable ID. Conversion continues through existing typed commands.

Public request/snapshot/report types and dependencies are unchanged. Additional
normalization is restricted to unsigned UV0 roles; existing f64 skin weights retain
their normalization path. Component-aligned but non-four-byte vertex offsets now
reject explicitly. Historical valid fixture bytes remain unchanged.

Dense, sparse-zero-u16 and sparse-base-u8 fixtures passed exact native mesh/image
content and CPU pixel/depth/normal comparisons, with a separate closed-form skin,
POSITION morph and absolute animation reference. Negative tests verify atomic
session state on malformed ranges, indices, extensions, flags and budgets. A zero
morph accessor with no base data and no sparse replacements is also tested.

Full acceptance passed. Renderer, shader, GPU lifecycle and public mutation
authority code are unchanged from 5416cfa. Actual native/browser imported GPU
renders and restore workflows passed before capability updates.

Final acceptance passed, including the corrected independent 43-request native and
real Chrome WebGPU/OPFS clients. No unresolved issue remains in this bounded input
profile. 80 historical images/pass/receipt artifacts retain exact bytes. The review
was performed by the implementation agent; no independent human/reference-image
review is claimed. Extra UV sets, alpha input, quantization and normal/tangent
morphs remain separate declared capabilities.
