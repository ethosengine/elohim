---
id: "backlog-feedback-generation-publishes-on-lifetime-visits"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A new feedback generation can publish before it has swept its subscriptions: the publication check reads each member's lifetime last-visited stamp, not coverage in the generation being built"
slug: "feedback-generation-publishes-on-lifetime-visits"
written: "2026-10-12"
author: "claude-opus-5-5 (sprint 2026-10-12-idle-call-floor-sprint; found by the Codex review leg, verified against HEAD)"
status: "open"
priority: "medium"
severity: medium
nodes: [alpha, household]
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [accountable-correction, feedback-projector, discovery, correctness, generation, plane-projection]
cites:
  - genesis/docs/superpowers/sprints/2026-10-12-idle-call-floor-sprint.md
  - genesis/data/timeline/backlog/feedback-discovery-sweep-is-o-n-in-history.md
---

**What.** `FeedbackProjector::tick` publishes the generation it is building when no subscription has a null
`last_visited_at`, no application member of that generation is pending, and the tick left no group pending
(`elohim/elohim-storage/src/services/feedback_projector.rs`, the block ending in `publish_generation`).
`last_visited_at` is a lifetime stamp on the subscription row. It survives a policy change, and it is written
before a member's enumeration runs, so a failed visit sets it too.

**The sequence.** A peer holds 16 subscriptions, all visited at some point under generation G. The pinned policy
changes; `resolve_generation` creates a new building generation G+1 with no applications. The first sweep visits
8 members (`MAX_MEMBERS_PER_SWEEP`). If those 8 leave nothing pending, the check finds zero unvisited
subscriptions and zero pending members and publishes G+1, which supersedes G without the other 8 members' acts
having been replayed into it.

**Status of the evidence.** Read from code at `b74697a70` and at its parent; the same condition is present before
the 2026-10-12 sprint (this is not introduced by the durable settled record, which is keyed to the generation and
seeds nothing in a new one). No run has shown it; no policy change has been exercised on a peer with more
subscriptions than one sweep visits.

**Cure candidates.** Coverage must be a fact about the generation being built: a per-generation visited marker
(the `feedback_subscription_settled` table added on 2026-10-12 already carries `generation_id` and could carry a
"swept in this generation" row), or a publication check that requires every member to have completed an
enumeration since the generation was created. A visit that failed enumeration must not count.

**Done when.** A test changes the policy on a peer with more than `MAX_MEMBERS_PER_SWEEP` previously visited
subscriptions, half of them carrying accepted corrections, and the new generation publishes only after every
member has been enumerated in it.
