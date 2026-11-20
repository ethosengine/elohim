---
name: project_radicle_lessons_reconciled_into_roadmap
title: "Radicle lessons reconciled into the roadmap"
description: "2026-11-05: Radicle/Heartwood survey sorted by reach into plan §7; §7.3 is the artifact-store sprint handoff; bites when planning Track E or F2."
metadata:
  node_type: memory
  title: Radicle lessons reconciled into the roadmap
  type: project
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-05T14:59:23.162Z
---

**What exists (2026-11-05).** `genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md`
§7 is the reconciliation of the Radicle/Heartwood survey
(`genesis/research/radicle-heartwood-cross-pollination-2026-11-05.md`) into the roadmap, and §7.3 is
the **handoff for the artifact-store sprint** (plan row 5 = Track E slices 2–4 + join F2, with an
elohim-storage hardening lane H1–H3 beside it). §7.4 lists four steward rulings the sprint needs
(the BLAKE3 iroh store; k/d per reach rung; hardening-lane ordering; the yank/revoke matrix).
Grounding reports: `genesis/local-dev/radicle-survey/{ground-epr-brit,codex-ground-rakia,codex-ground-custody}.md`.

**The ruling that sorts everything:** survey lessons land by reach, per D6. At `self`/`intimate`
custody is a local `custody-blob` pledge with zero DHT heads and the only gate is I1 (a hit needs
CID-verified bytes); the non-publisher, failure-domain, freshness-counted custody gate belongs to the
**release channel head** (F5), never to Track E's local put. Track E must not grow a replica count.

**Why:** the survey's own "What this changes" list read spec §10 only partly; applying its headline
lesson to the local tier would have re-fused the custody and bytes planes the 2026-11-01 arc critics
had just separated.

**How to apply:**
- Where §7 and the survey differ, §7 governs. Do not re-derive the sort.
- Two live defects in elohim-storage are now rows: holder-identity binding (`gossip_dispatch.rs:410`
  applies a sender-declared `peer_id`; dispatch carries only `propagation_source`, so the fix threads
  gossipsub's signed `message.source`; custody counts read those rows) and `BlobStore` non-atomic
  inline writes / unverified inline reads. Neither blocks Track E (the tier ALPN admits only the
  person's own devices; the eprfs tier store already writes temp+fsync+rename).
- Sprint precondition: brit `build find` is dark in this repo until brit `9776a9ad95` is pinned.
- One byte store per device, served by the tier daemon (§10.12); `BlobStoreTier` backs it where
  elohim-storage runs. Never a second copy.
- **Rulings R1–R4 (2026-11-05) are recorded in plan §7.4** after a four-leg Astra (gpt-6-astra)
  challenge that refuted every draft; the session model ruled. Headlines: BLAKE3 is a transport
  hash, never an identity (CIDv1 with a canonical emission profile per class and epoch); one
  authoritative payload store per device; Intimate reach needs no custody quorum, Trusted and above
  admit elections under a versioned custody admission policy naming tolerated failures; H2 gates
  E4, H1 gates every custody use of advertised holdings; quarantine/refuse and consent withdrawal
  override channel defaults and heads move only through the election. Record:
  `genesis/local-dev/radicle-survey/rulings-final.md`.
Related: [[project_rakia_as_artifact_store_custody_registry]], [[project_cross_pollination_surveys]],
[[feedback_master_peer_compute_before_hardware]].
