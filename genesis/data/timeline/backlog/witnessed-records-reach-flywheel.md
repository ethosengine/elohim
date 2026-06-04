---
title: "Witnessed records + reach flywheel — drain queue, witness credits, steward-finding, claim incentives"
created: 2026-06-04
domain: "design"
tags: [reach, witness, presence, stewardship, collective, incentives, shefa, qahal]
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
---

# Witnessed records + the reach flywheel

Full mechanics of the umbrella's §4 reach doctrine (intent-declared / observed-earned
/ witness-without-authority), operator-framed 2026-06-04:

1. **Drain queue**: records created with intended reach > earned batch privately on
   the creator's device (agent-scoped outbox) and drain as validation stories
   accumulate, elevating toward earned/warranted stasis. Effective reach =
   min(intended, earned).
2. **Witness credit**: creating a record on behalf of another (collective, dwelling
   entry, contributor presence — the Google-Maps-community-entry shape) earns a
   stewardship contribution credit (REA event), strengthened by an **authority
   disclaimer** ("I created this; I claim no authority over it") — the
   ContributorPresence STEWARDED state generalized from people to any record.
   Witnessing ≠ squatting because of the disclaimer.
3. **Steward-finding flywheel**: a witnessed collective is inert until it finds its
   stewards. Each steward connection elevates earned reach → the record gossips at
   that reach → blobs shard onto that reach-level of the quilted substrate
   (dwelling→collective→commons donut tiers) → individual hosting burden offloads to
   quilt and commons → stewarding cheapens → next steward arrives. Burden
   distribution IS the reward loop. (The household is the flywheel's first turn,
   degenerate by design: stewards instant, reach self-supplied — see formation spec.)
4. **Claim incentives**: value accumulated through an unclaimed collective makes
   claiming worth it; transferred standing should OFFSET the sharding cost stewards
   bore pre-claim. Substrate slot exists: ContributorPresence
   `claim_recognition_transferred_value/unit` reserved fields; the transfer-executor
   EconomicEvent is a named gap (resilience epic Part V).
5. **Incentive-stability analysis** (operator-flagged): how the incentives drive
   stability needs explicit modeling before commons-scale rollout — griefing
   (witness-spam for credit), claim-sniping, reach-inflation, disclaimer abuse.

This mechanic is also the **activation engine for dwelling-presence + the global
directory** (see `dwelling-first-class-entity.md`).
