# Development observations and retained costs

The candidate initially passed five native analytic UV tests, the existing three
displacement tests, generated-kernel validation and three explicit Metal cases.
Strict Clippy identified nested conditionals and a material command variant that
grew to 592 bytes after optional selectors were added. The conditionals were
simplified and the private Rust command payload boxed; public Serde JSON remains
unchanged. No lint was suppressed and no runtime dependency was added.

The first full run `run-20260906T064911Z` passed 120 native and 101 Wasm tests,
Metal comparisons and Clippy. During integration review the geometry-only exporter
loss-report omission was found. The reporting fix was added after those tests had
compiled; its initial formatter mistakenly required Display on stable Id and the
next Linux compilation caught it. This was a portable compile error, not a Linux
runtime issue. The short second run `run-20260906T065231Z` caught an incomplete
formatter correction requiring LowerHex on Id. The final correction formats the
inner u128. Both failed runs, exact errors and durations are retained under
`development-runs`; their partial test results are not final-source evidence.

The final frozen-source run `run-20260906T065252Z` passed all gates, including the
sixth test requiring explicit omitted-UV reporting. Source and package hashes tie
that result to the implementation. The final docs/capability/evidence update did
not change tested Rust or Cargo files. The fixture generator reproduces input
bytes independently; no golden image or existing fixture was changed.

Committed log copies trim terminal progress-line trailing spaces and redundant EOF
blank lines. Raw logs remain unchanged under artifacts. No diagnostics were removed.
