# Independent CLI fixture

Original synthetic metric box and ground plane, with a translation clip and UV
normal bake. `create.json` and `animate.json` are ordinary document command arrays.
`floor.obj` is an independently authored polygon/UV input. The independent client
supplies current revisions and durable retry keys, selects the authored box mesh
for a bevel, and drives only `render-host project` requests. No fixture-specific
engine commands or direct Rust calls are used by that client.

Run `python3 scripts/project-cli-workflow.py artifacts/cli-<new-name> [--gpu]` after
building `render-host`. The output records every input/response plus a recovered
project, evaluated times, render products, five shutter frames and explicit exports.
The fixture is intentionally small; it does not claim large-scene scalability or
compatibility beyond the existing named profiles.
