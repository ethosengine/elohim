---
name: feedback_agent_fleet_and_harness
title: Agent fleet, delegation & harness traps (umbrella)
description: "Keep 3 agents max; delegate narrow tasks to cheaper tiers; pin reader agents to an explicit model ID; subagent disjointness = read-set ∩ write-set; orphan cargo locks, StructuredOutput hangs, skew."
metadata:
  node_type: memory
  type: feedback
---

# Agent fleet, delegation & harness traps (umbrella)

Folds the multi-agent fleet-sizing, delegation and workflow-harness trap cluster. Members:

- [[feedback_three_agent_fleet_ceiling]] — Keep exactly 3 agents active at all times — more risks crashing the dev workspace; orchestrator watches returns, accelerates, and makes design calls.
- [[feedback_delegate_narrow_tasks_to_cheaper_tiers]] — Operator directive 2026-07-02: top-tier agent fleets burn the session limit — delegate narrow, crisply-defined tasks to opus/sonnet; keep the top tier for orchestration and judgment.
- [[feedback_pin_reader_agents_to_older_opus]] — Opus 5 handles complexity well but writes less accessibly — pin blind-reader to a full model ID, not the floating `opus` alias.
- [[feedback-subagent-disjointness-read-write]] — Parallel subagents are disjoint only if neither's read-set intersects the other's write-set; a porter reading source another task deletes is NOT disjoint.
- [[feedback_codex_side_delegation_queue]] — Well-specified disjoint tasks belong in genesis/data/timeline/backlog (not session lists) so ANY agent — Claude, Codex, Gemini — can claim them; offer during CI waits
- [[feedback_workflow_long_cargo_orphan_lock]] — Bash timeout orphans cargo still holding .cargo-lock; let it finish (work lands on disk), keep one profile per gate phase, run_in_background for >10min cargo.
- [[feedback_workflow_structuredoutput_hang]] — schema'd workflow agents retry empty {} StructuredOutput forever (48→481 calls, no completion notify) and hang the run — go schemaless prose + stall-watcher
- [[feedback_subagent_liveness_clock_skew]] — Container clocks skew hours apart — never infer agent death from transcript mtime vs date; check writer-relative freshness and TaskStop live racers first.
- [[feedback_overnight_permission_stalls]] — An idle overnight session may be blocked on a permission prompt (auth paths), not done; check the transcript tail and never race a blocked session.

**Shared-index race (2026-09-05):** two agents committing in ONE worktree raced on the git
index — the Rust implementer's `git add` landed between the Codex lane's staging audit and its
`git commit`, so Codex's commit a5fe238fc swept four Rust files in under the wrong message.
Neither agent misbehaved; `git add` + `git commit` is not atomic across agents. Rule for
parallel committers in one tree: commit by PATHSPEC (`git commit -m … -- <paths>`), which
commits only the named paths regardless of what else is staged; or give each agent its own
worktree. Put the rule in every implementer dispatch that shares a tree.

**Subagents parked on a Monitor never resume (2026-09-04, verified twice):** an implementer that "waits for the monitor to signal the other build finished" stops its turn and is never re-invoked when the thing it waits for was already gone — two seats sat idle ~3 h with uncommitted edits while cargo was free. Rule: seats never wait on a Monitor for a shared resource; they retry in the foreground (`sleep 60`, bounded ≤20×) or report BLOCKED; the controller checks `berth who` + `ps` for cargo when a seat is silent > 30 min and nudges with SendMessage.
**Root cause (2026-09-04):** the waiters used `pgrep -f 'cargo (build|test|check|clippy)'` — which matches OTHER waiter shells whose command line contains that very pattern — so the loops never exited (the one-build-at-a-time hook's suggested loop has the same bug). Use `pgrep -x cargo` / `pgrep -x rustc` (exact process names), never `pgrep -f` with a pattern that appears in the waiter's own command line.

**berth cannot separate seats of one session (2026-09-05):** the lease is keyed on session id, so a subagent`s `berth claim` succeeds while a sibling seat of the same session holds the resource. Rule: the controller holds mesh/cargo leases on behalf of its seats and sequences them by message; a seat never claims the mesh on its own initiative when told another seat may be measuring.
