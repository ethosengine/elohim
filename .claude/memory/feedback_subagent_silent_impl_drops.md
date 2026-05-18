---
name: feedback-subagent-silent-impl-drops
description: "Rust-architect subagents can silently drop From impls during refactor moves; per-crate `cargo build` won't catch impls that are only exercised by sibling crates in the workspace"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 4b9a03ba-e6e3-4770-9ee2-00e1721d108f
---

When dispatching a subagent to do a per-domain mechanical move of `impl From<X> for Y` blocks out of a monolith into sibling modules, the subagent can silently drop impls — especially toward the end of long sessions where its context is loaded. The dropped impls aren't compile errors in the modified crate when nothing in `lib.rs` directly references them; only `cargo build --workspace` exposes the breakage when downstream `api/*.rs` or sibling crates try to invoke the missing trait.

**Why:** Plan 3.A subagent dispatch (2026-05-18) moved 87 From impls across A.4–A.9. The shefa/qahal/infrastructure/epr commits were clean. A.6 (imagodei) dropped StageTrace + RecognitionDistributionResult impls. A.9 (inputs) dropped six `From<XxxInputView> for XxxDbInput` impls (CreateAgreement, CreateStewardAffinity, CreateNodeStewardship, CreateReaCommitment, UpdateReaCommitmentState, RecognitionTrigger). Each individual commit's `cargo build -p elohim-storage` was green — the impls live in a shared per-crate compilation unit so missing `From` impls don't error until a CALLER outside `lib.rs` invokes them. Workspace build (which compiles `elohim-storage`'s downstream `api/` modules and `cache_stream.rs`) catches it.

**How to apply:**
- Before commit-per-domain, count `^impl From<` in the source before and after, expect the delta to drop by exactly N (the impls intended for the move). If the count drops by more, something got dropped.
- Run `cargo build --workspace` (not `-p elohim-storage`) as the per-task gate when moving impls out of a hub crate, OR ensure the final task does so before declaring done.
- In subagent prompts, instruct: "After each move, run `grep -c '^impl From' /projects/.../views.rs` on the source AND `grep -c '^impl From' /projects/.../views_convert/<domain>.rs` on the destination; confirm `before_src - after_src == after_dst`."
- The fix is mechanical: `git show <pre-refactor-commit>:path/to/file.rs | grep -B1 -A30 "impl From<DroppedType>"` recovers the original body verbatim.

Related: [[feedback-signature-changes-grep-callers]] (same shape — workspace-level invariants escape crate-level builds).
