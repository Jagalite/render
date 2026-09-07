# Runtime geometry chunk integration review

The renderer owns disposable storage; geometry-query adapts only its compact build
entry point and actual layout accounting. The GPU consumes the same indices and
iteration order. Neither the builder split algorithm nor ray traversal changed.
Document and operation schemas remain independent of this Rust container layout.

Five Rust fixtures test ordering, boundaries, source immutability, all cancellation
checkpoints, copy accounting and nested allocations. The replacement seam is not a
public document operation: source revision, budgets and semantic checks stay with
its eventual domain caller. Existing public query stale/invalid/cancel tests still
run and admit the same portable work budgets.

Old tests and sampler probes intentionally change disposable UVs or duplicate a
triangle for an overlap error. Their mutations now construct replacement arrays;
all arithmetic, expected failures, tolerances and image assertions remain unchanged.
These are complete-array diagnostic transformations, not locality demonstrations.
The acceptance script adds the previously omitted ignored multibounce GPU test and
native PBR sampler workflow because those probe setups changed here.

Initial BVH construction retains its flat allocation while moving nodes into chunks.
The node storage allowance is conservative given the actual builder's leaf minimum
of two items for split trees and maximum four; tiny fixed costs use the existing
4096-byte fixed charge. Native and Wasm retained query layout can change, while all
portable source/work/admission decisions must agree. No allocator or process cap
is inferred from layout accounting.

No full M10, refit, sculpt, remesh or editor completion claim. Resource evidence must
separate root table copies from local payload copies and admit nested payload costs.
Final qualification passed 99 checks, 232 native tests and 203 Wasm tests. The collector also compared 26 sampler probe artifacts with the qualified pre-change binary. See evidence/runtime-geometry-chunks for package hashes, observations and unchanged outputs.

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
