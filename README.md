# Rust-native 3D Platform — architecture and milestone package

**Version 0.1 · September 5, 2026 · Planning only**

An independent, fully Rust creative platform with agent-native operations, browser/native execution, rendering before rich interactive editing, and staged major-Blender-feature coverage. No Blender, Cycles, or other non-Rust computational runtime is used as a planned production shortcut.

## Start here

- [Complete architecture specification](docs/architecture.md)
- [Milestones and release gates](docs/milestones.md)
- [Representation decisions](docs/representation_decisions.md)
- [API, ABI and agent contracts](docs/api_and_agent_contracts.md)
- [Implementation-agent guidance](AGENTS.md)

## Machine-readable planning

- `planning/milestones.json`: 15 milestone definitions, dependencies, acceptance criteria and required evidence.
- `planning/feature_matrix.json`: planned coverage by feature family and compatibility axis.
- `planning/initial_backlog.json`: first 12 implementation tasks and dependencies.
- `schemas/material_edit_request.schema.json`: narrow illustrative request schema.
- `examples/`: positive/negative example requests with validation caveats.
- `planning/document_validation_report.json`: document-shape and planning-graph checks performed while preparing this package.

The editable Word version is provided as a separate companion artifact; Markdown is the canonical repository source. No software implementation, renderer test, hardware benchmark, production dependency audit, or achieved milestone is claimed by this package.

The architecture supersedes the earlier Cycles/backend-bridge proposal. Blender is a development reference/test oracle only. Source references and their evidence boundaries appear in section 30 of the complete specification.
