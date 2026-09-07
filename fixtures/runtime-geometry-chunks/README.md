# Runtime geometry chunk fixture

Original project-authored arithmetic fixtures, CC0. Reproduce with
`cargo test -p render-core --test runtime_geometry_chunks -- --nocapture` and the
same test on wasm32-unknown-unknown with wasm-bindgen-test-runner. No external data
or computational dependency. Qualification passed; see `evidence/runtime-geometry-chunks/README.md`.

The resource fixture generates 4,096 half-unit right triangles on a 64 by 64 grid,
with two UV sets per triangle and the existing BVH builder. Changing one triangle
must copy one 64-triangle chunk and the 64-entry root table, preserve all old values,
and share the 63 unaffected chunks. It measures nested UV allocations separately.
A no-op replacement of the final BVH node measures one partial node chunk and its
nested leaf-item copies; it deliberately does not represent a geometric refit.

Other tests cover zero, one, 63/64/65, 127/128/129 and 4,097 elements; forward/reverse
iteration; a counted Clone implementation; invalid indices and every cancellation
checkpoint; and the independently analytic diffuse-plane render after geometry clone.
The full existing engine corpus supplies authored CLI and actual GPU/OPFS evidence.
