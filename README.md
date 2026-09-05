# Rust-native 3D Platform

**Experimental foundation · September 2026 · API/storage/ABI v0**

An independent, fully Rust creative platform with agent-native operations, browser/native execution, rendering before rich interactive editing, and staged major-Blender-feature coverage. No Blender, Cycles, or other non-Rust computational runtime is used as a planned production shortcut.

## Start here

- [Build, run, API contracts and supported profiles](docs/implementation.md)
- [M00–M04 validation evidence and qualifications](evidence/m00-m04/README.md)
- [Complete architecture specification](docs/architecture.md)
- [Milestones and release gates](docs/milestones.md)
- [Representation decisions](docs/representation_decisions.md)
- [API, ABI and agent contracts](docs/api_and_agent_contracts.md)
- [Implementation-agent guidance](AGENTS.md)

## Machine-readable planning

- `planning/milestones.json`: 15 milestone definitions, dependencies, acceptance criteria and required evidence.
- `planning/feature_matrix.json`: capability status by feature family and compatibility axis.
- `planning/initial_backlog.json`: first 12 implementation tasks and dependencies.
- `schemas/material_edit_request.schema.json`: narrow illustrative request schema.
- `examples/`: positive/negative example requests with validation caveats.
- `planning/document_validation_report.json`: document-shape and planning-graph checks performed while preparing this package.

The Rust workspace implements document transactions, native durability and jobs, a CPU renderer, generated portable GPU kernels, scoped OBJ/glTF interchange, and browser-local execution. The supported profile is intentionally narrow: polygonal scenes, Lambertian surfaces, point/environment lighting, and an approximate raster preview. See the evidence for runtime hosts, measurements and exclusions. M05–M14 remain planned.

```sh
cargo test --workspace --locked
cargo run --release -p render-host -- demo artifacts/demo --gpu
sh scripts/build-web.sh
cargo run -p render-host -- serve web 8765
```

The browser build requires `wasm-bindgen-cli` 0.2.104. Open `http://127.0.0.1:8765/` for the Rust-owned browser conformance application. It writes an isolated test fixture into origin storage. `python3 scripts/validate-foundation.py` records repeatable offline build, test, ABI, dependency and benchmark results.

Markdown remains the canonical requirements source. The bundled illustrative request schema is a design example; the running v0 contract is documented in the implementation guide. `SHA256SUMS` covers the documentation bundle and is refreshed when those documents change; Cargo.lock and the evidence manifests identify implementation dependencies and measured artifacts.

The architecture supersedes the earlier Cycles/backend-bridge proposal. Blender is a development reference/test oracle only. Source references and their evidence boundaries appear in section 30 of the complete specification.
