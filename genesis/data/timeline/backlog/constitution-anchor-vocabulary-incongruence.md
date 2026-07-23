---
id: "backlog-constitution-anchor-vocabulary-incongruence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "constitution.md anchors on 'blockchain' while Stance I.2 says 'not a blockchain' — the gospel-tier strand that skipped the reconciliation"
slug: "constitution-anchor-vocabulary-incongruence"
written: "2026-07-23"
author: "claude (ontology arc — OWL 2 graduation evaluation)"
status: "envisioned"
priority: "medium"
relatedNodeIds: []
tags: [constitution, vocabulary-drift, gospel-tier, anchoring, ratification, deferred-by-design]
shift_objective: |
  DO NOT PICK THIS UP AS A VOCABULARY FIX. Operator judgment (2026-07-23): the resolution
  should FALL OUT of getting the human-flourishing system primitives and EPR atoms right —
  the layering/writing/ratification/evolution model that makes the constitution load-bearing
  and authoritative. Fixing the word "blockchain" before that model exists would settle the
  vocabulary at the wrong layer and pin semantics we cannot yet observe.
  Pick up when: the EPR-atom + primitives work has produced an actual anchoring mechanism
  (how a constitutional layer is written, ratified, versioned, and verified), at which point
  the correct vocabulary is READ OFF the mechanism rather than chosen.
---

# The incongruence

Two gospel-tier documents disagree about the substrate the constitution anchors on.

`genesis/docs/content/elohim-protocol/values-forward.md:103` — **Stance I.2, "Not a blockchain, and not a token."**

> The substrate is agent-authored source chains, DHT-notarized, with economics expressed in REA / ValueFlows — not a global blockchain, and not a token economy.
>
> *How we reached it.* A global chain re-centralizes what it claims to distribute: one canonical ledger, one consensus everyone must join, one asset whose price becomes the system's true objective function.

`genesis/docs/content/elohim-protocol/constitution.md` — fourteen references the other way:

| Line | Text |
|---|---|
| `:59` | "Verifiable (blockchain-anchored, auditable)" |
| `:75` | "The blockchain constitution **co-locates our treasure with our values**." |
| `:138` | "Are cryptographically verified against blockchain-anchored versions" |
| `:178`, `:292`, `:358`, `:457`, `:554` | `# Hash: [blockchain-anchored]` in five constitutional-layer schema blocks |
| `:651` | "Verify each layer's hash against blockchain anchor" |
| `:693` | "Fetching anchor from blockchain" |
| `:725` | "Constitution verified against blockchain and peer consensus." |
| `:918`, `:922` | "Running Elohim agents with blockchain-verified values" / "The blockchain persistence is available." |

The manifesto carries it too (`manifesto.md:240`, `:278`, `:418` — "multi-layered blockchain constitution", "encoded in blockchain smart contracts at the global layer").

**A third document already reconciled and the constitution was not updated.** `governance-layers-architecture.md:140` now reads:

> Core definitions anchored at the graduated-immutability global layer as substrate-level HARD-BLOCKs (**not amendable policy, and not a blockchain smart contract**) — the deepest commitments are the hardest to change, and the existential floors are mechanical.

So this is the familiar shape: one concept, corrected in some homes, stale in the home carrying the most authority. The constitution is the stale strand, not values-forward.

## Why this is parked rather than fixed

Surfaced during the OWL 2 graduation evaluation (`genesis/research/owl2-graduation-floor-ceiling-ontology-2026-07-23.md` §8) while auditing the vision corpus for what it demands of an ontology. It is *not* an ontology-arc blocker and must not be bundled into the reach reconciliation.

Operator judgment, recorded verbatim in intent (2026-07-23): how the constitution is *layered, written, ratified, and evolves* — and thereby becomes load-bearing and authoritative to the whole — should **fall out of** getting the human-flourishing system primitives and the EPR atoms right. The contextual boundaries on human values that this machinery aggregates will be "under relentless attack and tension forevermore," so the anchoring mechanism has to be earned by the primitives beneath it, not chosen by editing a noun.

The engineering reason this is the right call: a rename now would pin semantics we cannot currently observe. `constitution.md:681-690` already specifies a `ConstitutionalAnchor` record — layer, community_id, version, content_hash, previous_version, amendment_rationale, ratification_proof, timestamp. That is a *versioned, hash-chained, rationale-bearing, ratification-proofed artifact* whose actual substrate is an open question, not a settled one. Whatever the EPR-atom work produces as the real anchoring primitive (DHT notarization, an attestation class, a Mishpat `Precedent`, something not yet named) will tell us the correct word. Read the vocabulary off the mechanism.

## What "done" looks like when it is picked up

1. The anchoring mechanism exists in code — a constitutional layer can actually be written, ratified, versioned, and verified against something.
2. `ConstitutionalAnchor` is reconciled with whatever that mechanism is (or is retired in favor of it).
3. The vocabulary in `constitution.md` and `manifesto.md` is regenerated from the mechanism's real name, with the same treatment `governance-layers-architecture.md` already received.
4. `values-forward.md` Stance I.2 is checked for whether it still states the truth, and strengthened if the mechanism gives it more to say.

## Watch-out

Do not "fix" this by find-and-replacing `blockchain` → `DHT`. `constitution.md:69-75` builds a genuine argument on the *properties* it wants (persistence for what must be etched, co-location of treasure with values); the argument survives the substrate change, but the prose must be rewritten to make the property claim rather than the technology claim. A mechanical rename would leave five schema blocks asserting `# Hash: [dht-anchored]` about a mechanism that may not hash that way.
