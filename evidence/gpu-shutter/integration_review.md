GPU shutter integration review checklist and preliminary results
- Static parameters default weight=1, no accumulation, no pass preservation; existing kernel algorithm unchanged before final stores.
- Persistent output initialized by nominal dispatch; temporal color cannot reuse a prior frame's buffer.
- Device dispatches own disjoint pixels; host waits for completion before next dispatch.
- Primary passes explicitly preserved; final IDs decoded against nominal scene.
- Errors/cancellation discard frame; no publication before complete result; CLI existing bundle/manifest boundary reused.
- Browser synchronous registration avoids lost immediate cancellation, post-await revision check prevents stale frame return.
- Browser sequence awaits callback and tracks delivery attempts separately from acknowledgement; no whole-sequence frame retention.
- Authored snapshot remains immutable; animated preview never calls static record_render.
- Disposable evaluator limits temporal geometry retention; potential additional rebuild cost is documented without speedup claim.
- Core request validation happens before allocations and checks all sequence time arithmetic before emission.
- Full acceptance running; preliminary real Metal numerical/sequence/cancellation test passed.
- Preliminary test mistakes: Receipt has no PartialEq (compare canonical bytes); overflow error is 'overflow'; invalid snapshot cannot supply a revision (keep original valid revision).

Final review: the complete acceptance suite passed with no unresolved correctness
finding in this bounded profile. 84 historical artifacts retain exact bytes.
Core/kernel/GPU/CLI/browser boundaries were reviewed by the implementation agent;
no claim of independent human or reference-image review is made. The original
fixture and reference outputs were not edited. Resource observations and negative
checks are linked from README.md; unsupported material/media and platform runtime
scope remain explicit.
