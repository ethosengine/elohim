---
id: "backlog-lamad-mastery-vision-remainder"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Lamad mastery — vision remainder gap ledger (designed in BLOOM-MASTERY-DESIGN, never built)"
slug: "lamad-mastery-vision-remainder"
written: "2026-06-11"
author: "lamad island recompose (mastery-canon authorship agent, evidence-verified)"
status: "envisioned"
priority: "low"
tags: [lamad, mastery, bloom, vision-remainder, island-recompose]
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-11-bloom-mastery-progression-design.md
---

# Mastery vision remainder — gap ledger

Companion to the canonical seed (`2026-06-11-bloom-mastery-progression-design.md` §8). Each item below was
designed in the retired `app/lamad/docs/BLOOM-MASTERY-DESIGN.md` (git history) but has no implementation
beyond, at most, type definitions. Line refs cite the retired source at its final commit.


Each bullet: one designed-but-unbuilt mechanic, with source line ref (BLOOM-MASTERY-DESIGN.md unless noted) and the disposition evidence. Backlog candidates.

- **ContentLifecycleService** (source :624-648) — service never built; only docs mention it (grep hits: `app/lamad/docs/BACKLOG.md`, the source design). Lifecycle TYPES landed (`app/lamad/src/app/models/content-lifecycle.model.ts`, exported `models/index.ts:157`) but have zero non-index consumers.
- **ContentNode.lifecycle + contentVersion fields** (source :533-555) — `grep lifecycle\|contentVersion` on `app/lamad/src/app/models/content-node.model.ts` returns nothing; the lifecycle model has no anchor on content.
- **Right-to-be-forgotten lifecycle transitions** (deprecate/archive/forget + staleAction + lifecycleGovernance) (source :141-196, :412-531) — no transition code, no governance hook; D2's `attestation:content-succession` (`elohim/sdk/domains/lamad/manifest/attestations.json`) covers only the supersedence slice.
- **Graph-evolution freshness factor** (`contentVersionAtMastery` vs `currentContentVersion`) (source :101-121, :131-134) — API stores `contentVersionAtMastery` (`elohim/elohim-storage/src/api/mastery.rs` InitializeMasteryRequest) but no current-version source exists to diff against; freshness is time-only.
- **Activity-relative freshness** (graph-adjacent engagement keeps mastery warm) (source :136-139) — server resets freshness 1.0 only on direct engagement with that node (`elohim/elohim-storage/src/db/content_mastery.rs:343`); no related-node propagation.
- **Per-level decay rates server-side** (source :123-129) — client `DECAY_RATES` per level live (`app/lamad/src/app/models/content-mastery.model.ts:309-318`); server uses flat `FRESHNESS_DECAY_PER_DAY = 0.05` (`db/content_mastery.rs:97`). Reconcile which is canonical.
- **ExpertiseDiscoveryService** (findExperts/findReviewers/findMentors/leaderboards/rising) (source :838-962) — never built; model types exist (`app/lamad/src/app/models/expertise-discovery.model.ts`) but are NOT exported from `models/index.ts`; already a tracked backlog item (`app/lamad/docs/BACKLOG.md` §Expertise Discovery System).
- **ExpertiseVisibility privacy controls** (discoverability/mentorship/leaderboard opt-in) (source :964-985) — no enforcement surface anywhere.
- **Phase 5 active-participation features** (analyze: comments/connections; evaluate: peer review; create: contributions) (source :757-761, :79-88) — no comment/peer-review/derivative components in `app/lamad/src/app/components/` (ls grep empty); manifest declares the contracts (`peer-review-completed`, `contribution-created` in `elohim/sdk/domains/lamad/manifest/signals.json`) but no lamad UI/flows emit them.
- **MasteryQuizComponent + FreshnessAlertComponent** (source :728-729) — neither exists in `app/lamad/src/app/components/`.
- **Privilege suspension on freshness decay** (`suspendedPrivileges`, `suspendedReason: 'freshness_decay'`) (source :326-329, :392-398) — model fields live (`content-mastery.model.ts:160-166`) but `grep suspendedPrivileges\|suspendedReason` in `content-mastery.service.ts` returns nothing; never set.
- **AffinityTrackingService.getEffectiveEngagement** (affinity x mastery composite) (source :651-668) — `grep getEffectiveEngagement` across app/ returns nothing.
- **attestation:mastery auto-minting policy** ("minted from private progress when policy fires" — `manifest/attestations.json` description; D2 §11.2) — generic `issue_attestation` coordinator exists (`elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs`) but contains no mastery trigger (`grep mastery` empty there and in `attestation_validator.rs`).
- **Mastery tier mapping** — `mastery-metadata.schema.json` tier enum `[familiar, proficient, expert, teaching]` has no declared mapping to the 8-level Bloom `MasteryLevel`; minting cannot be specified until this is reconciled (also flagged §7b of the architecture seed).
- **Source design's open questions still open** (source :999-1009) — per-content-type gate levels; domain-variant decay curves; cross-platform mastery credit; assessment anti-gaming rigor.
