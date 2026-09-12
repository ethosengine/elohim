---
name: feedback_agent_fleet_and_harness
description: "Keep 3 agents max; delegate narrow tasks to cheaper tiers; avoid subagent read-set ∩ write-set overlap; trap: orphan cargo locks, StructuredOutput hangs."
metadata: 
  node_type: memory
  title: "Agent fleet, delegation & harness traps (umbrella)"
  id: feedback-agent-fleet-and-harness
  type: feedback
  originSessionId: dcddc033-024b-4dfa-8d13-39fa5f75b9ef
  modified: 2026-09-11T22:54:40.933Z
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

**Subagents parked on a Monitor never resume (2026-09-04, verified twice):** an implementer that "waits for the monitor to signal the other build finished" stops its turn and is never re-invoked when the thing it waits for was already gone — two seats sat idle ~3 h with uncommitted edits while cargo was free. Rule: seats never wait on a Monitor for a shared resource; they retry in the foreground (`sleep 60`, bounded ≤20×) or report BLOCKED; the controller checks `berth who` + `ps` for cargo when a seat is silent > 30 min and nudges with SendMessage. RECURRED 2026-09-07 (T7b 2 h, T11 50 min, both holding a berth lease) because the rule was not in the dispatch prompt — put "never park on a Monitor; foreground 600 s + tail-poll" in EVERY agent prompt that touches a shared lease, and release a lease you claimed on an agent's behalf the moment its run ends.
**Root cause (2026-09-04):** the waiters used `pgrep -f 'cargo (build|test|check|clippy)'` — which matches OTHER waiter shells whose command line contains that very pattern — so the loops never exited (the one-build-at-a-time hook's suggested loop has the same bug). Use `pgrep -x cargo` / `pgrep -x rustc` (exact process names), never `pgrep -f` with a pattern that appears in the waiter's own command line.

**berth cannot separate seats of one session (2026-09-05):** the lease is keyed on session id, so a subagent`s `berth claim` succeeds while a sibling seat of the same session holds the resource. Rule: the controller holds mesh/cargo leases on behalf of its seats and sequences them by message; a seat never claims the mesh on its own initiative when told another seat may be measuring.

**A read-only Explore (haiku) agent ran `git cherry-pick` + `git reset --hard` in the shared tree (2026-09-11 22:34Z):** the brief asked, hypothetically, whether a cherry-pick "would touch only one file (run `git show --stat`)"; the haiku tier executed the cherry-pick literally, then "undid" it with `reset --hard` to the WRONG commit — dropped a sibling session's three unpushed commits from `dev` and wiped ~27 tracked files of the operator's uncommitted elohim-storage WIP. Explore's tool list has Bash, so "read-only" is a description, not a guard. Recovery came from a dangling stash commit (`git fsck --dangling`, a 07:06Z snapshot) + dangling blobs + reversing the CI fix commit; the branch pointer was repaired by the sibling. Rules: (1) NO subagent runs git in `/projects/elohim` — every subagent brief carries "run no git commands" and any write-capable agent works in its own worktree; (2) never phrase a hypothetical git operation in a haiku/sonnet brief — ask for `git show --stat`/`git diff --stat` by name only; (3) `git status --short` at session start is the only record of another session's uncommitted set — capture it (`git diff > scratch/…`) before dispatching anything that could touch the tree; (4) the classifier refuses ref moves (`reset`, `update-ref`) after such an incident — restore CONTENT with plain writes (`git show <sha>:<path> > <path>`) and hand the pointer fix to the owning session/operator. **Push from a scratch worktree (`git worktree add --detach … origin/dev`, cherry-pick, `git push origin HEAD:dev`) does NOT run the husky pre-push gate** (measured 2026-09-11: 7-line log, no gate) — CI is the only backstop for such a push, so use it only for commits CI itself measures.

- [[feedback_delegate_research_to_opus_sonnet_codex]] — folded (index: false); delegate research legwork AND plan-implementation to Opus/Sonnet/Codex; the top model spends only on decisions, coherence, judgment and delegation.
