---
title: "EPR-Meta Kinship, Lineage & Authority-Anchored Reconciliation"
id: epr-meta-kinship-lineage-reconciliation
tier: spec
status: Draft
created: 2026-07-12
maintainers: Matthew Dowell + Claude Fable 5
class: process-meta
process_subdomain: doc-lifecycle
topic: [epr-meta, eprfs, brit, lineage, kinship, provenance, content-addressing, cid, reconciliation, canonical-head, reach, stewardship]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR lineage-edges-and-remote-verdict-shipped
refines:
  - genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md
cites:
  - cite-fingerprint-cid-convergence | the convergence this design REFINES — one body digest, two renderings (full CID / sha256:hex16); supplies the full-CID-only-at-global-scale rule kinship matching inherits | sha256:0a657c9c1b0c43e7 | path: genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md
  - epr-meta-native-capability-dogfood-and-graph | names the elohim behavioral-ceiling judge + EprRef accountability anchor this design reuses as the reconciliation ceiling and authority anchor | sha256:99f0bf58985ff85b | path: genesis/docs/superpowers/specs/2026-07-10-epr-meta-native-capability-dogfood-and-graph-design.md
  - stewardship-over-sovereignty | the canon grounding authority in community-backstopped standing, not key possession — why the reconciliation anchor is socially-trusted, never self-sovereign apex | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md
---

# EPR-Meta Kinship, Lineage & Authority-Anchored Reconciliation

*Export/import provenance, DNA-like recognition, and judgment-mediated canonical heads.*

## 1. Problem

Exported and imported `.epr-meta` and eprfs package graphs need four things the current
content-addressing gives them no home for:

- **(a) Where an edge came from.** An exported edge must carry *where it was exported from* —
  provenance travels with the graph, not just the bytes.
- **(b) Well-identified holes, not breakage.** An import references objects that are not visible at
  the import site. Those references must survive as *well-identified holes* (known identity, known
  provenance, resolvable later) rather than reading as corruption or a dead link.
- **(c) Recognition-in-the-wild.** Encountering an artifact somewhere and being able to say *"this is
  my parent / my sibling / my child"* — kinship recognition without a central registry.
- **(d) Reconciliation without last-writer-wins.** Folding encountered subgraphs into a global model
  such that heads move by *judgment over history*, never by whoever wrote last.

## 2. Foundational constraint (stated honestly)

Cryptographic fingerprints are **deliberately kinship-blind.** The avalanche property means the CIDs
of two closely-related contents are themselves *unrelated* — a one-byte edit produces a digest with
no computable relationship to the original. Therefore **"filling in the missing links of a CID"**
(inferring lost intermediate contents from the hashes alone) is *not a solvable problem*, and this
design does not require it. DNA-ness — the ability to recognize kin — comes from **lineage carried
INSIDE the hashed bytes** plus an **optional similarity layer**, *never* from the hashes themselves.

This stands directly on the 2026-07-12 convergence: one canonical body digest, two renderings — the
full CID (`bafkrei…`, machine-facing) and the `sha256:hex16` short form (human-facing). One hard rule
is inherited from that convergence and governs everything below:

> **GLOBAL-scale kinship matching uses full CIDs only.** `hex16` (64 bits) is a *local display*
> short-form; at global population its birthday-collision probability makes it unsafe as a match key.
> Recognition across the wider network keys on the full CID; the short form is for human eyes.

## 3. The four mechanisms

### 3a. Lineage edges — the family registry (exact kinship)

Every atom and package carries its **parent CIDs** (`parents: [cid]` / `derivedFrom`) *inside its
canonical dag-cbor bytes*, so ancestry is part of the hashed identity itself — tamper-evident by
construction (you cannot restate your parentage without changing your own CID).

Kinship is then **ancestry-set intersection**, exact and offline-checkable:

- shared ancestor → **sibling**;
- you appear in its `parents` → it is your **child**;
- it appears in your `parents` → it is your **parent**.

This extends seams that already exist rather than inventing a new one:

- the **eprfs compose-graph edges** (`source` / `derived` / `composedBy`);
- **brit's `EprMeta` composition snapshots** (the git-domain analog);
- the **versioned-entity declared-HEAD DAG** principle — versions form a DAG (fork / revert / merge),
  and *which version applies is a DECLARED dependency, never recency.*

### 3b. Export envelope — provenance + declared boundary

An export ships more than the subgraph bytes. It carries:

- the **root CID** of the exported subgraph;
- **`exportedFrom`** — the source root's `EprRef` + the snapshot CID + a timestamp (the provenance
  stanza: *where this came from*);
- an **explicit boundary set** — the CIDs *referenced but NOT included*, i.e. the **declared holes**.

Precedent for the shape: a git **shallow-clone boundary**, or an **IPLD selector + CAR** file. brit's
composition snapshot is already ~90% of this artifact; it gains the provenance stanza and the boundary
list to become an export envelope.

### 3c. The `remote` verdict

Extend the cite/edge verdict vocabulary — today `ok` / `held` / `stale` / `dead` — with a fifth
verdict: **`remote`**. It means *identified-but-not-local*: the object's identity and provenance are
known (from the export envelope's boundary set), and it is *resolvable when the substrate is
reachable*. `remote` is what stops an import from misreading a well-identified hole (problem **(b)**)
as a `DEAD-CITE`. Dead is *"the slug resolves nowhere"*; remote is *"I know exactly what this is and
where it lives; I just cannot reach it right now."*

### 3d. Chunk-genome similarity — the actual DNA test (fuzzy, optional / research-flavored)

For kin whose relationship was **never recorded** (no lineage edge exists — an unrecorded fork, a
cousin that split before either carried parent CIDs): apply **content-defined chunking** to the body,
CID each chunk, and treat **the set of chunk-CIDs as the genome.** Overlap between two genomes
(Jaccard / MinHash) scores relatedness *with zero shared paperwork* — it detects cousins and
unrecorded forks that lineage edges alone would miss.

Composition of the two kinship tests:

- **similarity FLAGS** candidate kin cheaply (fuzzy, no lineage needed);
- **lineage CONFIRMS** and *locates the common ancestor* (exact);
- **judgment** (§4) decides *what to do* about the recognized relationship.

This layer is optional and research-flavored — the exact-lineage floor (3a–3c) is the shippable core.

## 4. Authority-anchored reconciliation (the keystone)

*Operator's refinement, 2026-07-12.*

Kinship answers **how claims RELATE**. It must **never, by itself, confer the right to declare the
merged head.** Relatedness is not authority.

**Authority derives from a claim-lineage that chains back to a socially-trusted anchor CID active in
the network** — an EPR with community-backstopped standing (earned reach, per the reach-earning
machinery), explicitly **NOT** key possession and **NOT** self-sovereign assertion. The anchor's trust
is *community-grounded* (cite `stewardship-over-sovereignty`; the identity-sovereignty ontology guard
applies — the commons backstops the individual, self-custody is never the apex tier).

A claim-chain that **terminates at such an anchor** carries the authority to *propose*: merges of
histories, canonical-head establishment, or head rewrites. The decision itself is a **judgment over
the history** — reviewed by the **elohim** (the behavioral-ceiling judge the 2026-07-10 dogfood spec
names; Mishpat-shaped: restorative, reviewable, revisable) — which reconciles all claims *up* into the
head state.

Properties:

- **(i) Heads move; history is never destroyed.** A "global CID rewrite" is a **new head
  DECLARATION carrying judgment provenance** — the old DAG remains intact. This is git's
  refs-vs-objects distinction: a ref moves, objects are immutable and stay.
- **(ii) Reconciliation is a lodged proposal, never an automatic merge.** Encountering kin writes a
  **fingerprint-deduped kinship finding** (the existing *flag → agent → canon* ledger pattern); the
  graft into the head is a **governance act**, not a side effect of encounter.
- **(iii) Floor / ceiling.** *Offline*, the carried lineage + export envelope **ARE** the trust
  snapshot — you reason from what travelled with the graph. *Connected*, the anchor's `EprRef`
  resolves to deep-validated standing and the judgment can **execute.**

This is the **same shape as the substrate's live rule that canonical channels alone move declared
heads** — generalized from content heads to *graph reconciliation.* Relatedness proposes; anchored
authority + judgment disposes.

## 5. P2P design-gate output

- **Lineage edges** — **A2 derived.** Fields live in the canonical bytes / DHT links on *existing*
  entries; content-derived addressing; **no new entry type.**
- **Export envelope** — content-derived artifact (has its own CID); **local until shared**; notarized
  via *existing* content anchors when it is.
- **`remote` verdict** — **C operational**, *computed never stored* (a verdict over local state +
  envelope, recomputed on read).
- **Kinship finding / reconciliation proposal** — **C locally** (the findings ledger), graduating to
  **B2 / governance-action** cross-peer via *existing* attestation + Mishpat types; **no new DHT
  entry type.**
- **Chunk-similarity index** — **C**, reconstructable from the bodies (a derived cache).
- **Anti-patterns caught:** full-CID-only at global scale (never `hex16` as a global match key);
  **authority-from-lineage must be community-anchored** — never a self-sovereign apex, per the
  identity-ontology guard.

## 6. Sequencing / out of scope

**Small next atoms (shippable core):**

- parent-CID fields (`parents` / `derivedFrom`) in the canonical envelope;
- the export-envelope provenance + boundary stanza (extend brit's composition snapshot);
- the **`remote`** verdict in `cite_graph.py` and brit's `verdict.rs`.

**Research-flavored / optional:** the chunk-genome similarity layer.

**Out of scope here:**

- the **elohim judgment runtime** itself (the named ceiling — the behavioral judge that reviews and
  executes reconciliation);
- **DHT notarization of exports** (envelopes are local-until-shared);
- any **hash-interpolation** — "filling in missing CIDs" — which §2 declares *unsolvable and
  unneeded.*
