---
index: false
name: feedback-subagent-disjointness-read-write
title: Subagent disjointness = read-set ∩ write-set
description: "Parallel subagents are disjoint only if neither's read-set intersects the other's write-set; a porter reading source another task deletes is NOT disjoint."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 32ed30bb-9c4a-4a71-9026-524a934f5f9e
---

When parallelizing implementer subagents, two tasks are disjoint **only if neither's READ-set intersects the other's WRITE-set** — NOT merely "they write different crates."

2026-06-26: I parallelized a Rust task that *ported* the Angular omnibar (read-set = `app/elohim-app/.../protocol-omni/*`) with one that *deleted* that omnibar (write-set = the same files). Different crates (elohim-render vs elohim-app), so I judged them disjoint — wrongly. The deleter's commit landed first; the porter, reading the now-deleted source, fell back to the wrong (surviving) sibling component and ported it. Cost: a full re-port from the git blob of the deleted file.

**Why:** the subagent-driven "don't run implementers in parallel" rule's stated rationale is *conflicts*, which reads as "same files" — and the trap is a cross-crate **read/write overlap**: a port / migrate / reference / "match the existing X" task is coupled to whatever task MUTATES its source, even in a different crate.

**How to apply:** before dispatching subagents in parallel, list each task's read-set (files it inspects, ports, or matches against) AND write-set (files it edits/creates/deletes); parallelize the pair only if `read(A) ∩ write(B) = ∅` and `read(B) ∩ write(A) = ∅`. If a task's job is to mirror/delete/reference source another task touches, serialize them (and have the second read the git blob if the source is already gone). Related: [[feedback_concurrent_sessions_shared_worktree]], [[feedback_subagent_liveness_clock_skew]].
