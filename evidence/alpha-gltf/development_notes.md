# Recorded development outcomes

The initial candidate passed 113 native tests but integration review found two
boundary regressions in its camera fix: an exhausted primary interval reached
media with reversed bounds, and the initial perspective near distance was clamped
to the continuation epsilon. The owned validator was stopped during Wasm building.
Its partial report/logs are in `initial-candidate/`; no complete-Wasm claim is made.
The focused reproduction passed five tests and failed those two new cases. Their
compiled-source hashes and failure log are retained. Both pass in the final seven-
test alpha suite and full native/Wasm run.

The first final browser client expected `stale_revision` from a fresh import into
a populated BrowserAgent. Its existing empty-target admission returns `import_target`
first. The final client verifies that exact error plus separate stale render and
branch mutation rejection, without changing Rust or rebuilding the package. The
failed check and successful follow-up are explicitly linked in validation_report.
The native CLI independently checks stale import mutations for every alpha mode.

The source was frozen at candidate commit `35f92f2` while an isolated named-UV branch
was developed. No UV code entered this alpha build or its tested packages. Final
Rust/Cargo hashes match the run's source manifest. Full browser pixels and passes
match the native CLI exactly. The resume/finalization script is retained here.
