---
title: "History/ADR: Attestation Consolidation (Phase 2a substrate dedup)"
id: attestation-consolidation-phase2a-dedup
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [attestation, dht-entry-types, consolidation, governance-action, shamir, projection]
# DISTILLS a 7-stage cross-DNA sprint that collapsed 18+ attestation-shaped DHT entry types
# into one reused Content entry. Landed 2026-05-15 (34fcf1070) by evidence: green migrations
# + live validators + manifest declarations. Raw impl plan retires to git.
distills:
  - genesis/docs/superpowers/plans/2026-05-11-attestation-consolidation-implementation-plan.md
canonical:
  - ../architecture/2026-05-11-attestation-consolidation-design.md   # the still-live design canon
memory_anchors:
  - project_collapse_bureaucracy_into_protocol
  - project_three_layer_truth_model
  - feedback_serde_json_value_breaks_zome_boundary
  - project_socially_derived_security
---

# Attestation Consolidation (Phase 2a substrate dedup) — LANDED 2026-05-15 (`34fcf1070`)

> **One-sentence lesson:** Never grep-and-delete a DHT entry type by name — confirm caller count first.
> The "vestigial" removal claim was WRONG for two entry types that had live callers, and a mid-sprint
> verify-before-remove re-org saved the merge.

**What was built.** A 7-stage (A–G) cross-DNA sprint that collapsed 18+ attestation-shaped DHT entry
types across four DNAs (imagodei, lamad, mishpat, infrastructure) into a single reused `Content` entry
discriminated by `content_type: "attestation:<subtype>"`, with the subtype vocabulary declared in pillar
manifests rather than minted as bespoke entry types. M-of-N social-threshold patterns decomposed into a
parent `governance-action:<kind>` Content entry plus child vote-attestations (with an operational tally
projection); Shamir share material decoupled off the DHT onto a libp2p request-response transport. Net
effect: ~−20 DHT entry types reclaimed; 22 legacy per-type projection tables → 2 unified + 1 derived
tally; 25+ legacy attestation routes → 8 unified routes; a discriminator-chain validator
(`attestation_validator.rs`, floors F1/F5/F7/F8 live) gates issuance.

**What superseded the plan body / why we turned.** Truth migrated into living surfaces and the plan
checkboxes became the only thing left: the migrations exist (`2026-05-12-100000_attestations`,
`_100100_governance_actions`, `_100200_governance_action_tally`,
`_100300_drop_legacy_attestation_tables`), all four pillar manifests carry `attestation:` declarations,
the Stage-G Shamir transport landed (`p2p/shamir_transport.rs` + `recovery/share_assembler.rs`), and the
EPR foundation post-attestation audit re-verified the merge with 0 orphaned tasks and 0 stale-route
hits. A verified consolidation is best remembered as green migrations + live validators + manifest
declarations. The plan also explicitly SUPERSEDED the Tiered-Quilt Wave-0 "Attestation dedupe"
direction.

**Watch-out for future planners.**
1. Mid-sprint reality-drift forced a "verify-before-remove" re-org of Stage C: the original "vestigial"
   removal claim was WRONG for `CustodianCommitment` (14 live callers in shard replication) and
   `ContentSuccession` (live callers in versioning) — never grep-and-delete an entry type by name;
   confirm caller count first.
2. B.9 (imagodei) was a full-replacement bridge but B.10 (mishpat + infrastructure) was ADDITIVE (writes
   both locally and via cross-DNA call) — bridge-conversion strategy was not uniform across DNAs.
3. The unified `attestations`/`governance_actions` tables are Category-A projections (source of truth =
   Holochain DHT, `dht_anchor_hash NOT NULL`); `governance_action_tally` is Category-C operational (no
   anchor, rebuildable by replaying the attestation signal stream grouped by parent). Don't treat the
   tally as notarized.
4. A pre-launch HARD cutover with no back-compat shim was the chosen path — acceptable only because it
   preceded launch.
5. Recovery's remaining migration onto this substrate is owned by a SEPARATE live plan
   (`2026-05-15-recovery-m4-completion-shamir-optional-plan.md`); retiring this body orphans nothing.

## Bidirectional links

- **This record → canonical:** [attestation-consolidation design](../architecture/2026-05-11-attestation-consolidation-design.md) (the still-live design canon; the watch-out plants near its 7-stage migration-plan section).
- **Distilled-from (raw impl plan in git history):** attestation-consolidation-implementation-plan (linked in frontmatter).
