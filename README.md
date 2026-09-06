# Rust-native 3D Platform

**Experimental agent-rendering alpha · September 2026 · narrow ABI v1**

An independent, fully Rust creative platform with agent-native operations, browser/native execution, rendering before rich interactive editing, and staged major-Blender-feature coverage. No Blender, Cycles, or other non-Rust computational runtime is used as a planned production shortcut.

## Start here

- [Agent workflow, ABI v1 and supported input profile](docs/agent_alpha.md)
- [M05 validation evidence](evidence/m05/README.md)
- [Animated glTF/GLB profile](docs/animated_gltf.md) and [validation evidence](evidence/animated-gltf/README.md)
- [Multi-bounce opaque PBR transport](docs/multibounce_pbr.md) and [validation evidence](evidence/multibounce-pbr/README.md)
- [Static PBR profile](docs/static_pbr.md) and [validation evidence](evidence/static-pbr/README.md)
- [Build, run, foundation contracts and supported profiles](docs/implementation.md)
- [Planned input, material, animation and Blender support](docs/input_support.md)
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
- `schemas/agent_request.schema.json`: implemented agent operations v0 wire contract.
- `examples/`: positive/negative example requests with validation caveats.
- `planning/document_validation_report.json`: document-shape and planning-graph checks performed while preparing this package.

The Rust workspace completes the experimental M00–M08 milestone profiles: transactional documents and durable jobs; static and animated glTF/GLB import; modeling and typed procedural graphs; curves, points, groomed hair and sparse media; advanced CPU scattering, displacement, bakes and Rust color/denoising; and exact-time rigs, skinning, morphs and shutter sequences. Metal/WebGPU support remains profile-specific. See [M07/M08 semantics](docs/m07_m08.md), [the evidence](evidence/m06-m08/README.md) and [input-format limits](docs/input_support.md). M09 remains deferred while engine and interoperability work continues.

```sh
cargo test --workspace --locked
cargo run --release -p render-host -- demo artifacts/demo --gpu
sh scripts/build-web.sh
cargo run -p render-host -- serve web 8765
```

The browser build requires `wasm-bindgen-cli` 0.2.104. Open `http://127.0.0.1:8765/` for the Rust-owned browser conformance application. It writes an isolated test fixture into origin storage. `python3 scripts/validate-foundation.py` records repeatable offline build, test, ABI, dependency and benchmark results.

Markdown remains the canonical requirements source. The bundled illustrative request schema is a design example; the running v0 contract is documented in the implementation guide. `SHA256SUMS` covers the documentation bundle and is refreshed when those documents change; Cargo.lock and the evidence manifests identify implementation dependencies and measured artifacts.

The architecture supersedes the earlier Cycles/backend-bridge proposal. Blender is a development reference/test oracle only. Source references and their evidence boundaries appear in section 30 of the complete specification.


The [general project CLI](docs/project_cli.md) supports typed requests for ordinary
authoring, explicit file imports, exact-time evaluation, scene/frame/sequence
rendering, imaging and exports. Run `target/debug/render-host project --help`.
[CLI validation evidence](evidence/project-cli/README.md) includes a complete
independent command-line workflow and cancellation/recovery tests.

[GPU shutter frames and streaming sequences](docs/gpu_shutter.md) are validated
on Metal/WebGPU through the CLI and browser APIs; see [acceptance evidence](evidence/gpu-shutter/README.md).

[Sparse glTF and normalized UV0 inputs](docs/gltf_accessors.md) now convert through
the shared native/browser importer into existing typed geometry and animation.

[glTF MASK/BLEND alpha](docs/gltf_alpha.md) is validated through native/browser
Rust CPU import, rendering and recovery; [evidence](evidence/alpha-gltf/README.md)
records exact passes and the corrected camera clipping/depth cases.
