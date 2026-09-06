# Development failures and costs

Early authoring builds failed before executing tests: one unmatched delimiter in
Rust kernel IR construction, missing borrowing/unwrapping of fixture transaction
requests, and an unwrapped `Arc<Pyramid>` in the secondary-texture test. These were
corrected as compile errors. No image reference, numerical threshold, renderer
profile exclusion or input validation was loosened to pass a test.

The focused analytic tests and explicit Metal test then passed. Full acceptance
logs and command durations are retained in `validation_report.json` and `logs/`.
Per-edit compile RSS and aggregate interactive development time were not measured;
workflow RSS/footprint measurements describe the recorded CLI processes only.
