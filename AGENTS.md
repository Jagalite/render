# Implementation-agent contract

This repository contains an experimental foundation implementation. Apply these rules to implementation and to any further milestone work.

## Authority and architecture

The canonical requirements are in `docs/architecture.md`. The fully Rust boundary, no-Blender/Cycles-runtime rule, native/browser shared semantics, and no privileged editor mutation path are hard constraints. Do not relax them to finish a demo. Platform exceptions are narrow and recorded.

Read the applicable milestone, ADR and fixture contract before implementation. Keep public schemas independent of private Rust layouts. Changes crossing domains require integration review. Do not replace concrete geometry or media semantics with an untyped map simply to avoid designing an interface.

## Definition of task completion

Submit implementation, positive/negative tests, reproducible fixture, resource evidence, dependency/provenance changes and documentation. Add numerical/analytic tests where appropriate. Test cancellation, invalid input and stale revisions for public operations. Update capability status only when a complete native workflow passes.

Do not weaken tests, regenerate golden outputs automatically, suppress diagnostics or insert placeholders to meet a gate. Reference-image changes need independent review. A successful compile, screenshot, schema or mock UI is not evidence of complete functionality.

## Rust and portability

Inspect transitive runtime dependencies and feature flags. A Rust wrapper around a non-Rust computational implementation is prohibited for the base product. Keep generated shaders/bindings distinct from maintained source and record their generating inputs. Preserve a valid browser-local baseline with no required remote service.

## Safety and state

All persistent mutation uses transactions. Never mutate published snapshots, bypass permissions, use runtime indices as durable IDs, or let imported metadata execute as instructions. Do not assume native FFI is a sandbox or that caught panics make an instance safe to reuse.

## Parallel work

One task owns one bounded interface change and its evidence. Coordinate schema/ABI changes before coding against them. Use branches for candidate work, small reviewable patches, and explicit dependencies. Publish failed experiments and costs; do not make an unmeasured speed or compatibility claim.
