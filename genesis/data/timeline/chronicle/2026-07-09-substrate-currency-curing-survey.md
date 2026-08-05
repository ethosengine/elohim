---
id: "chronicle-substrate-currency-curing-survey"
kind: chronicle
status: noted
date: 2026-07-09
ceremony: substrate-currency
mode: curing-survey (operator-steered — deliverable is a sprint handoff, not a gospel rewrite)
surfaces_rewritten: []
coherence_verdict: N/A (no surface rewritten this cycle; four-lens output synthesized into a handoff)
next_topic_sampled: notary-authority live-GREEN + bulk-seed witness bootstrap
---

## What changed

No gospel surface was rewritten this cycle. The operator steered the ceremony into curing-survey
mode: the four lenses (librarian present-state, historian failure-shapes, cartographer gap-ranking,
Explore code-trace) deep-read the p2p-dataplane / DHT-trust / doorway-federation system to produce a
**sprint handoff for Fable** to re-orchestrate the cure. Deliverable:
`scratchpad/fable-curing-handoff-2026-07-09.md` (offered for durable placement).

Core finding: the resurfaced `elohim-host-landing` "duplicate EPRs / stale data" trigger is the tail
of a *resolved* incident — the duplicate-EPR class is retired (`f4d967f7d`; EprRouter is a path-keyed
HashMap, structurally incapable of duplicate rows) and the per-host amber-write minter is cured
(`9f9c4aec4`). The live disease is the gap between **code-LIT and live-GREEN on notary-authority
Phase C** (2/3 scenarios green; adam transport-blocked by F-T19 view-federation timeouts) plus two
un-witnessed ingest/back-fill paths (bulk-seed `ContentBulkCreated` ignored in `projector.rs`;
Automerge back-fill of pre-existing content deferred).

## Coherence-check sampling

Phase 4b fresh-context coherence-check was not run (no rewritten surface to prime a downstream agent
against). The four lenses instead cross-checked each other and converged: librarian's LIT/INERT matrix,
historian's failure-shapes, cartographer's ranking, and the code-trace all name Layer-2
notary-authority as the weakest link and bulk-seed witness as the ingest-layer twin.

## Wisdom worth carrying forward

Three memory-vs-code drifts surfaced that the next hygiene pass should close (curation candidates, not
done here): (1) `notary-authority.feature` prose says the overlay is "not yet wired" — false against
`7af352617`; (2) memory says iroh `DualGossipPublisher` is "never constructed" — it IS constructed as a
degenerate single-transport passthrough (`main.rs:2851`); the *dual fan-out branch* (`:2362`) is the
dead code; (3) the arc-factor lever is retracted for the adam-storm case (`c7b459c1f`) — memory
`project_per_node_memory_is_conductor_authority_arc` still frames it as the storm lever. Also: the two
most drifted gospel primers Fable will read for this sprint are `rust-architect.md` (50 findings) and
`doorway/doorway-service/CLAUDE.md` (23) — a gospel rewrite of those is the natural follow-on cycle.
