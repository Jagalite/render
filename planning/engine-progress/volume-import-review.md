# VOL3 import integration review

Review scope: core binary adapter, existing sparse-media semantics, transactional
agent/browser and native file adapters, schema subset, persistence and fixture gates.

- Source bytes are parsed with explicit little-endian reads and checked u64 products;
  no host alignment, Rust memory layout, URI resolution or executable metadata leaks
  into the contract. Header lengths and channels are checked before indexed reads.
- File density and RGB emission share exact dimensions/bounds. X-fast indexing maps
  to stable integer coordinates; native cells are sorted before content identity.
  Explicit metric and optical policies prevent silent source scene interpretation.
- Raw values are validated before scaling, including zero-scale channels. Native
  asset validation remains authoritative for metric/optical limits. Empty data gets
  no invalid synthetic asset. Input and occupied-cell caps bound memory/work.
- Native reads now have a source-local cap in addition to the existing aggregate
  request cap. General file reads preserve their prior aggregate behavior.
- Shared import admission accepts a typed serializable report without a glTF-only
  private helper. Existing glTF command/report shapes are preserved. Evaluation and
  cancellation complete before atomic publication; durable failures keep session
  state unchanged. Root mutation classification includes the new operation.
- Snapshot schemas, FFI ABI shape, dependency graph and four kernel generators are
  unchanged. The public agent DTO and operation registry gain an explicit method;
  reviewed JSON Schema gains VOL policy and existing camera lens shapes.
- Native binary imports and browser byte-array imports have separate adapter/wire
  budgets, documented explicitly. Browser synchronous dispatch is not advertised as
  asynchronously preemptible. Native archives retain values, not raw VOL containers.
- Numerical oracles cover axis mapping, heterogeneous optical length, metric policy,
  Beer/emission integration, empty/sparse behavior and independent density/emission
  scaling. Negative gates include malformed data, limits, cancellation, stale roots,
  permissions and failed durable publication. Full packaged acceptance is recorded
  separately; this review alone is not a capability-completion claim.
