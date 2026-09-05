# Draft transaction examples

`material_edit.prepare.json` is a schema-valid **illustrative** material-edit preparation request. The IDs/digest are synthetic; no running service or existing project is implied.

`material_edit.invalid.json` deliberately puts roughness outside [0,1] and must fail the bundled schema. Both examples were checked with a JSON Schema Draft 2020-12 validator during document preparation.

Schema validation does not validate authentication, entity existence, the chosen material model, revision consistency, budgets, idempotency, candidate state or atomic commit behavior. Those are future engine tests. The request cannot assign its own authenticated actor or grant its own capabilities.

A `prepare` request creates a candidate, not a durable authored-document commit. The full prepare/inspect/evaluate/commit receipt protocol is still to be implemented and versioned. This is not a complete wire specification.
