---
id: "backlog-governance-native-dna-upgrade-path"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "In-network governance upgrade path — the network enforces its own DNA revert/upgrade lineage from inside (replaces operator-side reinstall flags after the Wave-3 re-genesis)"
slug: "governance-native-dna-upgrade-path"
written: "2026-08-04"
author: "holochain-iroh convergence campaign (held item; operator-named 2026-08-04)"
status: "backlog"
priority: "vision-deferred"
tags: [governance, dna-migration, lineage, mishpat, qahal, p2p-design-gate, vision]
cites:
  - genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
  - genesis/research/serialization-canonicality-cross-pollination-2026-08-11.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
---

# In-network governance upgrade path (vision-deferred; revisit after the convergence campaign)

Operator intent (2026-08-04): ideally the network itself enforces governance-ratified
revert/upgrade paths from inside — DNA migration as a community decision the peers
execute, not an operator-side `ALLOW_DNA_REINSTALL` flag. The convergence campaign's
Wave-3 re-genesis is authorized precisely because we are still firmly in dev proving
primitives; it should be the LAST out-of-band reset — this capability is what replaces it.

## Why the timing is newly right

Upstream laid the substrate rails in 0.6.2/0.7.0: `InitProperties` (DB-stored,
init-readable, cleared after successful init — conductor-opaque migration payload)
and `MigrationTarget` on `CloseChain`/`OpenChain` (`Dna(DnaHash)` | `Agent(AgentPubKey)`)
— i.e., source chains can now natively close toward a successor DNA. A governance
layer (mishpat bounds + qahal consent, earned-reach ratification per
`project_earned_reach_governance_pr_ceremony_vision`) deciding WHEN a peer executes
that close/open pair is the elohim-native design space.

## Constraints for the future design pass

- MUST go through the p2p-design-gate skill (entry-type classification, CID identity, coordinator/signal surfaces) before any route or table exists.
- This is a peer-native DHT/REA capability — k8s/deployments.json gaps are not protocol gaps (`feedback_k8s_is_not_the_architecture`); the design lands in the brit/rakia + DNA home.
- Ratification = deliberation by the operator+agents community completing at acceptance, never a solo stamp (`feedback_ratification_is_us_not_operator_solo`); and the human-in-the-loop is a floor-not-terminal-authority guard applies to any "operator override" escape hatch (`feedback_human_loop_not_terminal_authority`).
- Coordinator-only changes already have a no-lineage-cost hot-swap path (`update_coordinators`); this item is about INTEGRITY-zome / DNA-hash moves — the class that today forces re-key + re-genesis.
- **The wire-format layer carries the same skew and must be designed with this, not after it.** A peer that cannot decode a newer peer's *payload* and a peer stuck on a stale *DNA hash* are the same failure — schema skew across a diverse peer population — differing only in which layer declares the schema. Any change to EPR canonical bytes changes every CID and every signature, so a format migration IS a lineage migration and needs this item's close/open ceremony. See [arch-dataplane-borrows](epr:arch-dataplane-borrows-backlog) row 10 (schema-as-content-addressed-EPR; `Envelope.schema_ref` is the unused hook) and the survey it came from, [serialization-canonicality](epr:serialization-canonicality-cross-pollination-2026-08-11) — which names the two production precedents for binding bytes to a schema identity: Avro's schema fingerprint (ship the schema id with the data) and SSZ's fork-versioned schemas (all clients agree in advance). Holochain's DNA hash is our third instance of the same pattern.

## DoD (for the eventual design pass, not now)

A design doc through p2p-design-gate + a2o governance scenario(s) covering: proposal → deliberation → ratified migration commitment → per-peer `CloseChain`/`OpenChain` execution window → laggard/refusal handling (refusal-not-outvoting) → revert path symmetric with upgrade.
