---
name: Pace multi-agent sprints — don't run cargo tests concurrently across agents
description: When multiple agents work the codebase simultaneously, serialize cargo test runs to protect PVC / disk. Coordinate file scope between agents. Pre/post hooks help manage target dirs but agents can still starve each other.
type: feedback
originSessionId: 60007cbf-4a59-4bce-9be7-6e57d1568cf6
---
When orchestrating multi-agent sprints, do NOT launch multiple agents that will run `cargo test` (or any cargo build) concurrently. Even with the shared cargo-target-pool, parallel compilation locks + disk pressure can starve agents and crash builds. The PVC is a shared resource.

**Why:** The user added pre/post hooks specifically to help manage the cargo target pool, but they don't solve the concurrency problem — they just prevent any one worktree from accumulating disk debt. Two agents both running `cargo test` on overlapping crates can still:
- Fight over compilation locks in the shared target dir
- Saturate disk I/O on the PVC
- Trigger out-of-disk errors mid-build that cascade to neighbouring agents

**How to apply:**
- **One cargo-test-running agent at a time.** If another agent is doing storage/Rust work, my agents do non-cargo work (a2o narratives, docs, frontend changes, plan authoring).
- **Memory pressure is real, not just disk.** Cargo's linker phase + multiple sccache/rustc workers can OOM a shared machine. The constraint is PVC AND RAM. When in doubt, serialize cargo runs.
- **Coordinate scope explicitly** when multi-agent: agree which file set each agent owns. Diagnostics that mention unfamiliar files (e.g., `stack.rs`, `subsidiarity.rs`, `escalation.rs` from a tiered-storage sprint while I'm in Phase 4 EPR scope) signal a parallel agent in another domain — don't fix their diagnostics, don't touch their files.
- **The operator may be running parallel sprints** without my knowledge — multiple agentic threads under one operator umbrella. Specifically observed: tiered-storage sprint co-existing with EPR delivery sprint, 2026-05-11. If diagnostic notifications mention files outside my current scope, assume a parallel sprint owns them.
- **Pause cleanly** when told to pace: stop spawning new agents, let the in-flight one finish, then synthesize. Don't try to be productive by reading lots of files (that uses context) or by spinning up more parallel work.
- **Non-conflicting parallel work that's still safe**: pure-doc edits (memory, plans), single-file refinements in directories the other sprint isn't touching. But even those can hit the editor's review attention budget — prefer stillness over filler work.
