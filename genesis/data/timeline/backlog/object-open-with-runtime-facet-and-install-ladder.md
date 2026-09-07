---
id: "backlog-object-open-with-runtime-facet-and-install-ladder"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Selecting an object on the p2p dataplane needs a RUNTIME facet beside the custody facet — 'open with' a peer-native epr-app, then a one-click install ladder (browser → this device → my hub) where installing replicates the executable's store, runs it under an ark envelope, and 'offer to peers' is a separate REA commitment; Shefa's per-device view shows what is stored/hosted with whom"
slug: "object-open-with-runtime-facet-and-install-ladder"
written: "2026-09-08"
author: "operator (sleep-on-it thought, 2026-09-08) via orchestrator@fable-5.1"
status: "envisioned"
priority: "medium"
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:operator-runtime-surface"
  - "habit:runtime-upgrade-propagation"
  - "habit:runtime-death-witnessed"
  - "backlog-clone-content-escapes-via-shared-projection"
  - "backlog-rung5-p2p-propagation-of-tonights-coordinators"
tags: [resilience-card, open-with, epr-app, install, hub-steward, delegated-compute, rung-5, shefa, story-seam, p2p-design-gate]
shift_objective: |
  Give the dataplane object chip a second facet (runtime: which epr-apps can act on this object and
  where each runs) beside the custody facet it has today, and a one-click install ladder
  (browser → this device → my hub) that composes the four installer acts without equating them.
  Evidence of done: an object on the household mesh opens with an epr-app served by a peer, the
  install-to-device path replicates the executable as an elected artifact under an ark envelope, the
  hub path records an "offer runtime to peers" commitment through the existing compute-grant flow, and
  Shefa's per-device view lists the object's custody rows, its hosting commitments, and the governance
  holder of each grant. Mesh receipt under genesis/a2o/reports/, delta on operator-runtime-surface.
---

**The thought (operator, 2026-09-08).** The resiliency badge on the browser chrome reads only the
persistent plane: where the bytes are and how many custodians hold them. When a person selects an
object on the p2p dataplane they also need the **runtime** dimension, Google-Drive "open with" but
peer-native: open this spreadsheet with the peer-native spreadsheet epr-app (think IronCalc served
over p2p web across the collective's EPR dimensions). Then a further toggle: **install** that app.
One click invites you in, not only to host the tooling locally for offline use but, if the device is
capable, to replicate the persistent store of the executable server itself and become available to
offer that runtime to peers. Two clicks away: **install to your hub** instead, if you have one. That
attracts people to become hub stewards and gives them an always-on runtime for themselves and
others. Shefa gives the on-device / which-device view of what (thing/story) is actually stored and
hosted (compute valueflows) with whom (governance), so the elohim can help a person manage the whole
of their social compute surface.

**Why this composes rather than adds.**

- *Two facets, one chip.* A spreadsheet with five custodians and zero reachable runtimes is not
  usable; one whose only runtime is another person's hub is usable but dependent. Custody and
  runtime each carry their own reach and staleness (see `project_head_reach_freshness_semantics`:
  reach ≠ head ≠ replication). The resilience card is already data-starved
  (`project_resilience_card_data_plumbing`); this names the columns it lacks.
- *The install ladder is the four installer acts made one click.* The reimplementation plan
  §10.3 keeps four acts distinct: obtain the package, enable its runtime capabilities, join a
  community, provision or join a witness context. Browser = obtain + enable, cached. This device =
  obtain the executable as a content-addressed artifact, replicate its store, run it under an ark
  envelope so its death is witnessed (`runtime-death-witnessed`). Hub = the same, always-on, plus
  the separate act of offering the runtime to peers. The silent merge of these acts is the failure
  §10.3 names; one click must compose them, never equate them.
- *"Offer to peers" is a commitment, not a flag.* Delegated compute already mints grants as
  Class-A Commitment/EconomicEvent (`elohim/elohim-storage/src/api/compute_grants.rs`) with an REA
  reciprocity fold (`services/rea_observed_compute.rs`). A hub steward offering IronCalc is one more
  grant kind over the same flow. Hosting shows up as visible contribution in that fold, which is what
  makes hub stewardship attractive.
- *Install-to-hub dogfoods rung 5.* The epr-app executable is an elected artifact head the storage
  plane replicates and `services/release_adoption/` already judges. Installing is the hub adopting an
  app release by election; the invitation link is a candidate the hub applies. No new distribution
  channel, a UI over an existing election.
- *Shefa's device view is the same three columns.* Per-device custody rows, hosting/compute
  commitments from the reciprocity fold, and the governance holder of each grant.

**Surface principle (operator, 2026-09-08 follow-up).** Simple on the surface, distinct one layer
down. The person sees one click ("open with", "install", "install to my hub"); the four installer acts,
the custody/runtime facets, the grant, and the election are composed by the system beneath that
click through carefully drawn seams. The distinctions live in the seams and the receipts, never in the
prompts: a person is asked only for the choices that need them (§10.4 "quiet by default, intelligible
on demand"), and every composed act stays inspectable one layer deeper (Shefa's device view, the
resilience chip's facets) so the seamlessness never hides authority.

**Guards before any route.**

1. `p2p-design-gate` on every record: the host offer is Class A; local install state is private or
   ephemeral; the object being opened must name its witness context, so the D6 cell-qualifier
   contract (`clone-content-escapes-via-shared-projection`) is a precondition.
2. A hosted (doorway) session keeps its hosted trust limits: an invitation must never imply local
   confidential inference or custody on a device the person does not have (§10.4).
3. Uninstalling a view removes the interface and its active capabilities, never shared commitments,
   another person's evidence, or promised custody (§10.4).

**Chain / between / missing node.** chain: object selected on the dataplane → runtime offered by a
peer → object opened in a peer-served epr-app. Between "peer executes a stage and attests" (sprint
2026-09-08 T10, first instance of the edge for a task) and "peer serves an interactive epr-app over
the collective's EPR dimensions": missing node = *a runtime offer is a discoverable, reach-scoped
commitment a selecting client can resolve into an "open with" entry* (assertion: given an object CID
and the selector's reach, the client lists the peers whose offers cover that epr-app and the object's
witness context; probe: a2o scenario on the household mesh with jessica offering and matthew
selecting). Current state: envisioned; no offer kind for interactive runtimes, no runtime facet on the
chip, no per-device Shefa view.

**Placement.** Not in sprint 2026-09-08; it must not displace batch 3. Next slice after T10 lands:
write the spec with T10's receipt as evidence, then the badge facet and the browser rung of the
ladder on the household mesh, the hub rung once the Adam worker is enabled (operator item 3).
