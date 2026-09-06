# Development observations and failed checks

Initial compilation of the new GPU test failed because animation::Receipt does
not implement PartialEq. The test now compares canonical receipt bytes rather
than changing the public receipt type.

The first negative time test expected `time`, while checked rational overflow
correctly reports `overflow`. After correcting that assertion, the first complete
acceptance attempt failed because the test then tried to derive a revision from
an intentionally invalid snapshot. Snapshot::revision validates settings and
correctly rejected it. The test now retains the original valid revision and
asserts that malformed dimensions reject without overflow. No implementation gate
was weakened. The [failed run](failed-preliminary-run/validation_report.json) and
its [test output](failed-preliminary-run/native_tests.txt) are retained.

The final suite passed. During its earlier phases, the browser client gained an
additional stale-sequence callback check before that client was executed. The
browser report records the expected one acknowledged frame followed by conflict.
No Rust runtime or Rust test source changed during the successful run. Documentation,
capability status, evidence and the final source manifest were completed afterward.

The temporal cache review found retention proportional to shutter sample count in
the existing CPU evaluator reuse. Disposable sample evaluators now bound retained
poses on CPU/GPU. All 84 prior CLI image/pass/receipt artifacts are byte-identical;
no rendering speed claim is made. Device fence and geometry rebuild overhead remain
possible optimization targets requiring separate measurements.

Committed test-log copies remove redundant blank lines at EOF for Git whitespace
checks. Diagnostics are unchanged; byte-original logs remain in the named artifact run.
