---
id: "backlog-dependency-graph-cited-contributor-economy"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The dependency graph as a cited contributor economy — epr-meta over every dependency, unclaimed presences, modelled valueflows"
slug: "dependency-graph-cited-contributor-economy"
written: "2026-10-02"
author: "operator (Matthew Dowell), captured by Claude during the gradient-view session"
status: "envisioned"
priority: "medium"
jobs: [elohim, rakia, brit]
relatedNodeIds:
  - "backlog-citation-apparatus-claim-grain-typed-edges-reach-gate"
  - "backlog-brit-act-level-attribution-identity-chain"
tags: [citation, attribution, contributor-presence, valueflows, epr-rea, rakia, brit, cargo, mishpat, outreach, unit-item, unit-history, phase-growth, design-input]
---

## The operator's thought

If the project takes its own citation discipline seriously, it applies it to its dependency
graph. The operator's direction (2026-10-02): cargo becomes a module in rakia/brit; every
dependency gets an `.epr-meta`; the graph is used to model epr-rea, valueflows and aggregates at
the scale of open source itself, with a Mishpat economy over the contributor presences. Outreach
to the developer community then says: this is what we think you are owed and how it breaks down;
consider authoring your own `.epr-meta`, improve on the cited claims, reconcile with the network,
and host a peer to take part in the negotiation. "We don't need permission to model that."

The operator's framing of the purpose: restoration and reconciliation for developers whose work
was taken into the commons. If the protocol succeeds it works like a union that brings its own
institutions, so that valuations are published **with** the people named, not against them,
whether or not they are yet at the table.

## What exists, read on 2026-10-02

- 34 tracked `Cargo.lock` files naming about 1,200 distinct Rust packages (the project's own
  crates included), nearly all sourced from crates.io. A `pnpm-lock.yaml` covers the Node side.
- A lockfile entry already carries name, version, source and a checksum of the exact bytes. It is
  a bibliography entry with a fingerprint.
- Dependencies are already a planned presence cohort, as a hand-picked list and not as a graph.
  `genesis/docs/content/elohim-protocol/history/2026-06-21-contributor-presence-whoswho-grounding.md`
  names "Libraries / projects (cohort 1 — clean provenance, seed first)", and
  `genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md`
  defines a `prior-art` standing for "code/ideas we actually compose" (Khan/Perseus, Holochain,
  libp2p, iroh, Automerge, hREA, gitoxide, Eclipse Che). This entry extends that cohort from
  about ten projects to the whole resolved graph.

## Canon this must be reconciled with

Found by a delegated reader on 2026-10-02; each passage below was then re-checked against its file.

- `manifesto.md` line 661: the protocol "deliberately records the facts and defers the valuation".
- `succession.md` line 606 refuses "Any published default allocation."
- The bootstrap spec gates outbound claim-invites on a confirmable fact, and defers opt-out.
- The grounding doc already names individual authors behind cohort 1, which sits in tension with
  observation 2 below.

"This is what we think you are owed" presses on the first two. Observation 3 (the number is a
claim, contestable, method cited) is the reconciling move, and may not be enough; the first slice
publishes no numbers for that reason.

Missing, as with document cites: what of ours relies on which dependency and how heavily, a typed
edge, and the reverse lookup.

## Design observations (unverified; brainstorm input)

1. **Read the graph from `cargo metadata`.** The module consumes cargo's answer and adds meaning;
   it does not rebuild the resolver.
2. **Presences stay unclaimed until the person claims them.** Identity is claimed, never inferred.
   The presence is addressed to the crate or project and held in stewardship. No presence is
   minted for a named person from commit emails.
3. **The number is a claim.** Any weighting (depth, call sites, criticality) is a judgement. It is
   published with its method cited and a way to contest it. "Owed" here is modelled value, and the
   first sentence a maintainer reads says so.
4. **Outreach leads with the citation.** "Here is exactly how we depend on your work" is something
   a maintainer can verify. The valuation follows as an invitation to correct it.
5. **A load specimen that needs no mesh.** A real power-law graph with real version churn is a
   more honest test of epr-rea aggregation than a synthetic one.

## Tension to settle before publishing

Claude's caution in session: modelling public data needs no permission; publishing valuations
beside named people is the step that warrants their invitation. The operator's answer: the
valuation is published with them and on their behalf, and the institution exists to give them
standing at a table they were never offered. Observation 2 is what makes both hold: the valuation
attaches to the work and its unclaimed presence, and becomes personal only when claimed.

## Smallest first slice

One workspace; generated `.epr-meta` per dependency; unclaimed presences; no numbers.

## Open questions

- Where does the module live: rakia (package meaning over the store) or brit (acts and
  attribution), and what does each own?
- What is the unit of contribution below the crate: release, commit, maintainer act?
- How does an upstream author's own `.epr-meta` supersede the generated one?

No design is chosen. This entry graduates through `/brainstorm` and the `p2p-design-gate`.
