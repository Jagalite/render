# Development and review observations

Initial core/native importer checks passed, followed by the full native/Wasm and
portability suite. The first independent CLI workflow completed dense import,
CPU/Metal rendering and restore before failing its stale-operation check. It used
a 15-character idempotency key, below the document contract's 16-character minimum.
The engine correctly returned `request` before checking revisions.

The initial diagnosis incorrectly focused on the literal stale revision. The first
retry switched to the valid pre-import revision but retained the too-short key and
failed again. Inspection of Document::scope identified the actual key-length rule.
The final client uses a valid-length key and the pre-import revision, and gets the
expected `stale_revision`. A browser malformed-import check also now supplies a
valid-length key. No engine validator or assertion was weakened.

The [first attempt](failed-workflow/workflow_report.json),
[first retry](failed-workflow-retry/workflow_report.json), exact failing requests,
and logs are retained. The final native/browser workflows pass. All Rust runtime,
Rust tests and fixture hashes matched the inputs to the completed broad suite,
so only the corrected client workflows were rerun. The validation report records
follow-up commands, costs, resolved failures and the superseded diagnosis history.
The [follow-up helper](resume_validation.py) is retained as execution evidence;
the normal reproduction command runs a fresh full suite with corrected clients.

Core review additionally tightened relative component alignment and four-byte base
vertex alignment. Checked arithmetic and bounds precede source reads, and sparse
index order/range precedes replacement copying. Missing-base compact storage is
reused. Renderer/shader/state authority paths were unchanged; 80 historical render
artifacts are byte-identical. New baseline reproductions of existing CPU alpha
camera issues are parked for the next alpha-input increment, outside this opaque
sparse-input profile.
