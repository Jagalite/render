# Immutable runtime geometry acceptance

Evaluated triangle and BVH node storage now shares immutable 64-item chunks. This
bounded prerequisite passed on baseline c5e7233; M10 sculpting and refits remain open.
No document version, durable schema, operation, renderer arithmetic or shader changed.

Qualification passed 99 checks, 232 native tests and
203 Wasm tests. Five storage tests cover boundary sizes, forward/reverse
iteration, actual Clone counts, old-root immutability, invalid slots, every cancellation
checkpoint, nested allocations and an independent diffuse-plane render. All existing
CLI, packaged Chrome WebGPU/OPFS and Metal workflows pass. Linux/Windows are compile
checks; this does not claim their runtime qualification.

In the 4,096-triangle/2,047-node fixture, one triangle replacement clones 64 triangles,
one payload chunk and 64 root references. The 63 unaffected triangle chunks remain
shared. Nested UV clones add 3,072 bytes. A no-op final-node replacement clones 63 node
payloads and 32 root references; its leaf-item clones add 1024
bytes native and 512 bytes Wasm. It is not a refit test.

| Container layout excluding nested allocations | Native bytes | Wasm bytes |
|---|---:|---:|
| Triangles | 1508376 | 1507852 |
| BVH nodes | 197048 | 147652 |

These layout counts exclude Self, Arc headers and allocator overhead. Initial conversion
retains the flat builder allocation while moving into chunks. Root table replacement
remains global, as do changed-geometry evaluation, snapshot admission and GPU host
packing. Whole-process/time observations in chunk_probe_comparison.json are sequential,
uncontrolled observations including GPU initialization; no speedup claim follows.

The preservation collector checked 2004 prior render/pass/receipt
artifacts and 216 prior fixture files byte for byte. Query results
and portable work costs match the old binary, with layout changes separately recorded.
The adapted PBR sampler probe additionally matches 26 old-binary image/pass/receipt
and analytic report files. Exact old/new tested packages and source manifests are
recorded; dependencies, enabled features, native links and all five shaders are unchanged.

Development logs preserve the missing post-clean cache directory, missing first accessor,
old mutable diagnostic setups, and the failed Wasm observation collection before use of
the runner's console macro. No assertion or image tolerance was relaxed. The pinned
Khronos validator was restored after temporary files disappeared and its three file
hashes matched prior qualification; Blender reference bytes came from the previous
qualified exact-source export. Neither tool is a runtime dependency.

Reproduce with scripts/validate-runtime-geometry-chunks.py using localhost:8784 and
Chrome CDP 9224 plus the prior Blend/UV/paint/spatial baseline environment variables.
Collect with scripts/collect-runtime-geometry-chunks-evidence.py and
RENDER_CHUNKS_PREVIOUS_RUN set to the qualified spatial-query run. Preserve the c5e7233
baseline binary/provenance under artifacts/runtime-geometry-chunks/baseline for the
additional sampler comparison. Source schemas and authored transaction authority stay
unchanged; the storage replacement seam only accepts private runtime slots.

## Repeated process resource observations

The initial sampler run had higher candidate resident memory, so four additional
alternating whole-process runs were made with the same preserved binaries. Every
repeat matched all 26 reference artifacts. Across three samples per binary:

| Binary | Resident bytes, min to max | Footprint bytes, min to max | Workflow seconds, min to max |
|---|---|---|---|
| baseline | 915079168 to 1227456512 | 1264357696 to 1681904576 | 44.310 to 52.886 |
| candidate | 1007091712 to 1482899456 | 1683559424 to 1799148800 | 44.582 to 49.863 |

The resident-memory ordering reversed in the first repeat. Resident-memory and
timing ranges overlap; the candidate footprint range stayed above the baseline
range in these samples. The footprint difference is not isolated to a cause. These
observations do not establish statistical equivalence or a memory/throughput gain.
The CPU/Metal process includes global image loading, GPU initialization and sampler
work; these are not sculpt-update latency measurements. Full resource logs, repeat
workflow reports and artifact hashes are retained. The measured local payload-copy
counts remain the qualified benefit of this storage prerequisite.
