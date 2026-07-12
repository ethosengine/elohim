---
id: "backlog-commons-pool-as-economic-authority-wallets-in-bridge-layer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Commons pool as the economic authority unit — individual wallets live in the bridge layer, not the substrate core (re-sorts the identity-binding blocker)"
slug: "commons-pool-as-economic-authority-wallets-in-bridge-layer"
written: "2026-07-12"
author: "operator vision note (2026-07-12), answering the cold-review identity-binding blocker"
status: "open"
priority: "high"
area: "architecture/economic-attribution"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "memory:feedback-identity-sovereignty-ontology-guard"
  - "memory:project_earned_reach_governance_pr_ceremony_vision"
  - "memory:project_rea_compute_commitment_primitive"
cites:
  - opus-handoff-roadmap-2026-07-12 | Opus Handoff Roadmap | path: genesis/docs/reviews/2026-07-12-opus-handoff-roadmap.md
  - cold-outside-substrate-review-2026-07-12 | Cold Outside Substrate Review | path: genesis/docs/reviews/2026-07-12-cold-outside-substrate-review.md
tags: [economic-attribution, commons, identity, bridges, rea, ai-authorship, governance, roadmap-amendment]
---

# Commons pool as the economic authority unit

## The operator's reframe (2026-07-12, verbatim intent)

The cold review named "cross-signed individual identity binding" (who-served
bound to who-authored) as the blocker gating all economic attribution — you
can't pay creators when the binding is forgeable. The operator challenges the
ASSUMPTION under that blocker: **"who authored" is a tricky unit when AI is
ubiquitous. Commons pools might need to be the authority; individual
'wallets' need to stay in the bridge layer.**

## Why this re-sorts the blocker rather than dodging it

The false-binding attack (claim another agent's served-bytes credit) only
bites if the SUBSTRATE holds credit at an individual identity. Move the
authority/attribution unit to the **commons pool** and the attack loses its
target. The requirement drops from "unforgeable cryptographic binding of
transport-id ↔ agent-key" (the specced-but-BLOCKED resolver, unsigned
`STAGE1_SIGNATURE_SENTINEL` bindings) to "**socially-attested contribution
WITHIN a pool**" — which the earned-reach machinery already does. The hard
crypto problem is not solved; it is RELOCATED to a smaller, better-defined
surface.

Three ways it composes with what already exists:
1. **It is the identity-sovereignty guard applied to economics.** An
   individual wallet as root economic identity is the forbidden self-sovereign
   apex; community governance backstops it. Commons-pool-as-authority is that
   ontology one layer down.
2. **Wallets belong on the `bridges/` seam.** A wallet is an external
   (web2/crypto) protocol — it translates in/out at the bridge layer like
   every other external protocol, never in the canonical EPR-REA core.
3. **It is the right answer to ubiquitous-AI authorship.** If a commons pool
   is author-of-record, "an AI agent composed this from the commons" naturally
   credits the commons that trained/enabled it — closing the value-flows-back-
   to-commons loop instead of extracting to whoever ran the model. Individual
   authorship fractures under AI; commons authorship does not.

## Where the hard binding genuinely SURVIVES (the honest residue)

1. **The cash-out edge.** When value leaves a pool for an individual wallet,
   THAT transition needs the strong binding — but it is now a narrow,
   auditable boundary in the bridge layer (the KYC/attribution frontier), not
   a whole-substrate property. Confining the unforgeable-identity requirement
   to the one place it is load-bearing is a feature.
2. **Intra-pool distribution + pool governance.** Sybil resistance is still
   needed so one actor can't spin 10k identities to dominate a pool's
   governance or drain its distribution — but this is now a GOVERNANCE problem
   (councils, earned reach, attestation) rather than a TRANSPORT-CRYPTOGRAPHY
   problem. It is the problem the architecture is built to solve, not the one
   (cross-signed libp2p↔agent-key binding) it is stuck on.

## Roadmap consequence (amends opus-handoff-roadmap Tier 2 item 4)

Item 4 ("cross-signed identity binding gates ALL economic attribution") is
RE-SCOPED, not deleted: most of it moves from "blocking substrate primitive"
to "bridge-layer cash-out edge + pool governance," and only a thin slice stays
as hard crypto at the withdrawal boundary. The design session (p2p-design-gate
for any new entities): what IS a commons pool as a first-class REA/DHT entity
(is it a Mishpat commitment-holder? an Agreement party? a new entry?), how
contribution attests INTO it (earned-reach attestations as the input), how
value settles OUT of it (rollup → bridge → wallet), and what governance gates
pool membership (the Sybil-resistance-as-governance answer). Sequence with the
scale-envelope REA read-path economics and the earned-reach PR-ceremony vision.

## Deliverable

A brainstorm → design doc (p2p-design-gate) folded into the seam-map atlas +
the earned-reach governance canon, defining the commons pool as the economic
authority unit and the wallet-as-bridge boundary. Not code first — this is a
vision-level placement decision that reshapes the whole economic-attribution
layer, and it is the operator's to make. Captured here as made.
