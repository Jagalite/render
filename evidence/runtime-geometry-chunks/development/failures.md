# Development observations

native1.log: after requested Cargo clean, the candidate target symlink pointed to
an absent shared cache. Explicitly recreating that cache directory restored builds.
native2.log: the first compile identified an existing BVH consumer of Vec::first;
the immutable container now supplies a read-only first accessor. No semantics or
tests were relaxed. native3.log passed four storage and ten spatial tests;
native4.log passed five storage tests including actual nested payload observations.
The deleted temporary Khronos validator was restored at pinned version 2.0.0-dev.3.10;
all three tool file hashes match prior qualification. The Blender startup reference
is taken byte-for-byte from the previous qualified workflow's original source export.

clippy1.log and clippy2.log found old disposable-geometry probes using mutable
iteration or push. Their exact UV scaling/triangle duplication is now constructed
as a new immutable array. Assertions and numeric tolerances are unchanged. The
acceptance script explicitly runs the affected ignored multibounce GPU test and
native PBR sampler workflow in addition to its existing named UV and bake tests.

acceptance1.log: all five tests passed on native and Wasm, but the resource collector
correctly rejected missing Wasm observations: std::println does not emit through
that runner. The test now uses wasm_bindgen_test::console_log on Wasm while retaining
println on native. A fresh complete qualification is required for the updated test.
