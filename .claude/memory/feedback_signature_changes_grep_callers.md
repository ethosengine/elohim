---
name: Signature changes need crate-wide caller grep before spec-compliance
description: Subagent reviews catch in-file regressions but miss other-file callers; signature changes in Rust need grep across src/+tests/+benches/ before declaring done
type: feedback
originSessionId: 72a4534a-dd50-4984-be17-9d287ef54e6b
---
When a Rust function's signature changes, the implementer subagent (and the spec/code reviewers) typically verify the inline tests adapted, but they don't always grep for callers across the whole crate. **Integration tests in `tests/`, examples in `examples/`, and benches in `benches/` get missed** — the change passes inline review then fails at pre-push (or CI) when those external-but-still-in-crate callers don't match the new signature.

**Why this matters:** Subagent context windows are scoped to the file they're editing. They reason well within that scope but don't proactively scan adjacent directories. The pre-push hook *will* catch the failure, but only after a 30+ minute Rust build/test cycle — burning real time and operator attention.

**How to apply:** Before declaring a Rust signature-change task spec-compliant, the implementer prompt should explicitly include a grep across all caller locations:

```bash
grep -rn "<function_name>" elohim/elohim-storage/src elohim/elohim-storage/tests elohim/elohim-storage/benches elohim/elohim-storage/examples 2>/dev/null
```

For *every* call site found, verify it uses the new signature. Inline tests (in `src/.../foo.rs::tests`) are usually obvious; integration tests in `tests/<name>.rs` are easy to forget — but they're full crates that link against the public API and will break loudly.

Concrete trigger from M1 (2026-05-07): Tasks 4+5 changed `build_response_slice`'s arg list from 8 flat params to `(view_kind, ctx: SliceContext)`. Inline `#[cfg(test)]` tests in `view_federation.rs` were updated and reviewed. The integration test at `tests/p2p_command_view_federate.rs` had **three** call sites still using the 8-arg form. The pre-push gate caught it at minute ~60 of the cargo cycle. Cost: one full re-push attempt.

**Specifically for view_federation.rs / build_response_slice:** the SliceContext struct is `pub`, so callers in tests/ should `use elohim_storage::p2p::view_federation::SliceContext` and construct an instance literal at the call site. `connected_peers: &[]` is the right empty default for tests that don't simulate libp2p state.
