---
kind: chronicle
status: noted
date: 2026-05-18
ceremony: substrate-currency
surfaces_rewritten:
  - .claude/agents/rust-architect.md
  - doorway/doorway-service/CLAUDE.md
coherence_verdict: YELLOW
next_topic_sampled: recovery-class signal_kind extension + ReconcileController handlers + storage-stewardship-summary HTTP route
---

## What changed

`rust-architect.md` rewritten from 655 → 737 lines: cartographer's coverage-gap on the last six weeks of substrate work was the dominant driver (REA ledger services, topology↔REA bridge via `custody-blob`/`project-blob`/`serve-blob`, ReconcileController + RecoveryFlowProjector + ElohimContentSignal dispatcher, views_convert/ refactor, elohim-views crate, elohim-hub, CustodianCommitment + ContributorPresence anchors); historian surfaced 10 missing canonical-discipline citations including ts-rs cross-crate, subagent-silent-impl-drops, k8s-as-dev-substrate, reach-enum drift, signal_kind extensibility, and living-doc honesty-matrix maintenance; librarian fixed real path drift (request_offer_service.rs → exchange_service.rs, WriteBuffer crate-origin to elohim-cache-core), corrected the `[[project_libp2p_transport]]` slug, acknowledged lamad-v1 as scaffold, and rewrote forbidden-phrasing leaks at L55, L68, L506, and L654.

`doorway/doorway-service/CLAUDE.md` rewritten from 11 → 67 lines: storyteller-led targeted rewrite (not expansion), naming the identity sentence at top, territory-boundary against parent CLAUDE, six recent-landing key-file rows (pkarr resolver, SSR dispatch, render module, cache write-on-fetch annotation, iroh-dispatch-boundary annotation, elohim-views consumer note), inline citations at the four historian-recommended junctures, and the L37 k8s-substrate framing reframed per `feedback_k8s_is_dev_substrate_not_protocol`.

## Coherence-check sampling

Sampled next-topic: recovery-class `signal_kind` extension wiring + matching ReconcileController handlers + the `GET /storage-stewardship/summary` doorway-manifest route — the highest-leverage roadmap items from the just-landed resilience epic. Fresh-context Explore returned **YELLOW** with six findings. Four were mechanical and closed inline on `rust-architect.md`: added the `SIGNAL_KINDS` whitelist file path (twice), added a reach-drift "active prerequisite" note for routes that filter by reach buckets, and added the Vouch (Light Up the Graph) primitive as the canonical end-to-end worked example for new signal_kind additions. Three were larger-than-mechanical and backlogged to `feedback_substrate_currency_ceremony_2026_05_18_yellow_backlog`: a manifest-route worked example (overlaps with the parent `doorway/CLAUDE.md` rewrite the cartographer already named for next cycle), a query-composition aggregation pattern for routes like the stewardship summary, and a ground-truth check on the resilience-epic Part VII claim that ReconcileController has wired handlers (the controller's own top-of-file comment says "no-op stubs"). None of the backlog items blocks the recovery-class sprint.

## Wisdom worth carrying forward

When a chapter lands a load-bearing matrix of LIVE/DESIGNED/GAP claims (the resilience epic's Part IX), the matrix's accuracy decays measurably across one ceremony cycle: Phase 4b found that the chapter's own top-of-file claim about ReconcileController handlers may already be ahead of what code says. `feedback_living_doc_honesty_matrix_maintenance` is the right discipline; running it against the chapter that authored it is the right closing of the loop. Future ceremonies should treat *the chapter that introduced the matrix* as a standing audit target until the matrix's first round of substrate-work landings have come through and migrated the rows downward.
