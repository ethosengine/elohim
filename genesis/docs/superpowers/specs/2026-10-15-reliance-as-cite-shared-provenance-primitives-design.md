---
title: Reliance as cite — the first implementation of the shared provenance primitives, for code, the co-op grocer and the community-garden network
id: reliance-as-cite-shared-provenance-primitives-design
status: Draft
class: architecture
date: 2026-10-15
context-tier: disclosed
steward: human:matthew
author: "Claude, drafted at the operator's direction (operator design session, 2026-10-15)"
tags: [covenant, provenance, cite, rea, epr-rea, bridges, presence, commons, vsm, dependencies]
graduation-trigger: habit cites-own-dependencies turns green on the §8 verification, with the gate run cited in its DELTA
cites:
  - "elohim-protocol-manifesto | manifesto | sha256:66c8e3b40b858b92 | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "glossary | glossary | sha256:b7c64d8426c97ef6 | path: genesis/docs/content/elohim-protocol/glossary.md"
  - "succession | succession | sha256:663c98c4b4e5af87 | path: genesis/docs/content/elohim-protocol/succession.md"
  - "commons-pool-as-collective-party-design | A commons pool is a collective that is party to its members' commitments | sha256:0f5f91ec5681b780 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
  - "epr-rea-valueflow-fabric | EPR-REA ValueFlow Fabric | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md"
  - "private-thought-governed-fruit | private-thought-governed-fruit | sha256:5b6f5cdb858277e4 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - COVENANT.md
  - "sealed-contract-edges-governor-frontier | Sealed Contract Edges | sha256:ace1788fa44a293f | path: genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md"
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - .claude/skills/p2p-design-gate/SKILL.md
  - .claude/skills/elohim-commons-design/SKILL.md
  - elohim/epr-rea/src/model.rs
  - elohim/epr-rea/src/external.rs
  - elohim/epr/src/witness.rs
  - bridges/jenkins/src/translate.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
  - elohim/elohim-storage/src/recursion.rs
  - .epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json
  - elohim/brit/FUNDING.json
---

# Reliance as `cite`

This spec proposes how the Elohim Protocol records what its own software relies on, using records
we expect a co-op grocer could use for its suppliers and a community-garden network for its seed
lines; the operator's rulings on the open questions in §9 settle it. Each dependency becomes a
`cite` input on the build's process, naming the package by a content address and its makers
through a presence held in trust. There is no code-specific schema: the design fills seven missing
nodes in primitives that already exist. A new crate, `bridges/cargo`, observes cargo and writes the
records. No DNA integrity zome changes and no submodule pin moves.

Terms used throughout:

- **EPR** is the protocol's content-addressed record, an atom whose address is the hash of its
  bytes. **epr-rea** (`elohim/epr-rea`) expresses REA (Resource–Event–Agent accounting: agents
  performing events on resources) over those records. **epr-meta** is the governance package
  (`.epr-meta`) a directory carries.
- A **CID** (content identifier) is a self-describing wrapper around a hash of the bytes.
  `bafkrei…` is the raw-bytes, sha256 form, so the checksum crates.io already publishes is the hash
  inside one.
- The **DHT** (distributed hash table) is where peers hold and validate notarized entries; it is a
  notary, not a database. A **DNA** is an application's rule set on Holochain; its content hash
  defines a distinct network. A **zome** is a module of a DNA: the **integrity** zome holds entry
  types and validation, and changing it moves the DNA hash (a reinstall, new agent keys, and peers
  split onto separate networks); the **coordinator** zome holds functions and can be swapped
  without moving it. A change that leaves integrity alone is **DNA-hash-NEUTRAL**.
- A **presence** holds recognition for a contributor who is not yet on the network, until they
  claim it. In this spec, **standing** is the `standing:` field on a presence record: its recorded
  relationship to us, such as `prior-art` or `relied-on`. (Elsewhere in the corpus the word also
  names a person's relational track record.)
- **Reach** is how far a contribution travels beyond its author's own circle; it is earned. A
  **reach tier** is a named step on that scale; the vocabulary is in declared drift (§9,
  question 10).
- An **elohim** is one of the protocol's AI agents, running on a participant's own hardware and
  acting for that person or community. A **holon** is a space that holds its own activity ledger:
  a person, a household, a collective, a region.
- The **head plane** is the recurring cost every notarized item adds: a sweep row, a re-provided
  network record and a conductor round-trip, on every peer, for as long as the item lives. A
  **composite root** is one lasting record that points at many rebuildable ones, so only it pays
  head-plane cost. It requires the records it points at to be rebuildable from their source; here
  the source is the lockfile.
- A **release**, in this design, is a commit on `dev` that changes a lockfile.
- The **graduation evaluator** (`elohim/elohim-storage/src/graduation/`) has two paths. **Path 1**
  turns observations into an attestation gated by a diversity threshold. **Path 2** turns a set of
  observations into one summary `EconomicEvent` carrying `observation_refs`.
- A **rung** is a step on a pool's ladder of consequences. Rung one is a notice and a recorded
  choice, not a refusal.
- **VSM** is Stafford Beer's Viable System Model, in which each viable system contains smaller ones
  and sits inside larger ones; System 2 coordinates, System 3 runs operations, System 3\* audits
  them, and an algedonic signal is an alarm that bypasses the ordinary channels.
- The **SDO/RWA test** asks what the dominant few (social dominance orientation) and the many
  disposed to follow them (right-wing authoritarianism, in Robert Altemeyer's sense) could see,
  join and compel through a store. Its six hard boundaries are in
  `genesis/docs/architecture/private-thought-governed-fruit.md` §4.
- **rakia** is the protocol's build-graph engine. **brit** is the protocol's git-compatible tool, a
  fork of gitoxide, that seals source snapshots.

## 1. Why

Manifesto Part IV-B, "A Covenant From Below: Citing What Was Taken"
(`genesis/docs/content/elohim-protocol/manifesto.md:692`), makes this commitment for software:

> Every program rests on hundreds of libraries written by people it never names. The protocol
> commits to citing its own dependencies the way it asks authors to cite sources: each one
> recorded with how the work relies on it, each with a presence held in trust until its makers
> claim it. The record states the facts of reliance and defers the valuation.

The covenant asks to be "found keeping the terms when the people named come to look." On
2026-10-15 the operator named this the social contract of code: the covenant as it applies to
software, where a licence's notice was the old contract's one enforced term. He made it urgent
because people have started looking: DTPR (Digital Trust for Places and Routines, an
AI-transparency labelling project); Alex and the Noosphere (a reasoning-quality graph project);
WeAll (a civic-protocol project); and Dave Plummer (a retired Microsoft engineer who runs Dave's
Garage), to whom a letter has been written.

The operator's governing rule: **this is the first implementation of the EPR, EPR-REA, epr-meta
and VSM ontology**, of how value graduates, flows peer to peer and aggregates into commons. What
we build for code is what we build for the co-op grocer and for a network of community gardens. So
the design reuses and refines the deepest primitives. It replaces an earlier draft that put a
bibliography schema in rakia.

## 2. One ontology, three domains

**Rival goods move by `transfer`; non-rival knowledge, designs and code are `cite`d.** A crate of
tomatoes delivered to the grocer is no longer at the farm. A recipe the grocer draws on, or a
library a program builds on, stays where it was. Today `ReaVerb` has `Cite` (live for notes only)
and no `transfer`; the DNA's `REA_ACTIONS` has `transfer`. Node 3 below gives both from one source,
because the grocer and the garden need `transfer`.

How to read the table: the grocer and garden columns stand on their own and need no code to follow.
The last column is implementation state, for engineers.

| What the record does | Code (now) | Co-op grocer | Garden network | Where it lives · state |
|---|---|---|---|---|
| Holds a seat for a maker not yet present | a presence for each upstream project | the farm or packer named on the delivery note | the grower named with a seed library accession | `ContributorPresence` (imagodei `lib.rs:501`) and `Inflow.presence` · implemented |
| Names the thing by what it is | crate@version; the lockfile checksum is the sha256 inside the CID | the case or lot code on the box, and the PLU code at the till | the accession number and variety name | epr-rea: "anything content-addressed is a resource"; the plan maps a produce lot to `tracking_identifier` · implemented |
| Describes the work and its recipe | the build (`edge-pipeline@1`, gate recipes) | receiving against the delivery note, shelving, selling under the PLU | a grow-out: sowing at the stated isolation distance, growing, saving seed | `ProcessSpec` / `Process` · implemented |
| Records drawing on knowledge without taking it | `cite` each dependency | a deli recipe card naming the cookbook or supplier method it follows | parent and selection notes naming the lines and growers a selection drew on | `ReaVerb::Cite` · live for notes only |
| Records goods changing hands | — | each supplier invoice line, matched to the delivery note signed at the door | a seed packet checked out, and seed returned after the grow-out | DNA `REA_ACTIONS` (`:224`) only; not in `ReaVerb` |
| Labels the parties where the thing arrives | the offer's `disclosure.inflows` | country-of-origin labelling on the shelf | the seed packet's label: variety, accession number, grow-out year | epr-rea `Inflow` · implemented (Jenkins offer) |
| Notes a link someone else already checks | `DepEdge` with `Governor::Compiler` (cargo enforces it) | an organic certifier's certificate number on the invoice | an organic seed certificate, or a variety released under the Open Source Seed Initiative pledge (the seed world's own licence, which travels with the seed) | implemented, path identity only |
| Records whole-work ancestry | a fork or vendored patch is `derived_from` its upstream (plus `modify` for a patch) | a store-brand item whose spec sheet names the supplier product it is made from | a selection whose notes name its parent variety | SDK relationship `derived_from` · exists; four lists drift |
| Describes a pinned outside artifact | a crate's manifest | a supplier's spec sheet for one product | the variety record: name, isolation distance, grow-out history | the AI model manifest `.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json` (bytes CID, license, source, procedure) · used once |
| Turns many observations into one lasting record | a summary per lockfile over its cite observations | the month's reconciliation of invoices against delivery notes | the grow-out year's summary across accessions | graduation evaluator, Path 2 `summary_event` with `observation_refs` · built, tests only |
| Gathers members' records into a shared view | a reverse index of who relies on a package | which member stores stock each farm's lots, read by lot code | which gardens are growing out each accession this year | `CoverageRollup` (`recursion.rs:192`); rollup-atom spec (care-aggregation §5.3) · one level, unchained |
| The commons itself | `commons-pool:dependency-index` | a co-op federation's shared supplier records | the seed library's collection | `Collective` + `Membership(MemberKind::Collective)` + the 2026-10-04 pool spec · collective-as-party missing |
| Judges what it was worth | deferred | deferred | deferred | `author-lens` (mishpat) · implemented; version-DAG spec |
| Watches whether the practice holds (VSM) | a habit as the System 3\* audit; the rung-one notice as Systems 2 and 3; an algedonic sensor | a monthly check that every invoice line has a delivery note | a check that every packet out has a return or a note | `epr flow` measures, notes, habits · dev plane implemented |

**The grocer and garden columns are a hypothesis, not a validated claim.** The identifiers and
practices in them are ordinary ones anyone in those trades can check; none of them exists in any
Elohim code. Whether the same records actually describe a co-op's supply or a seed library's lines
has to be tested with practitioners, in a design session this spec defers.

The code column's shapes live in epr-rea: `FlowEvent` (`model.rs:377`; `ReaVerb::Cite` at
`epr/src/witness.rs:310`), `ProcessSpec` (`model.rs:64`), `DepEdge` (`model.rs:450-500`, where
every governor except `CiteSeal` "is a citation of a STRONGER system already enforcing the edge")
and `Inflow` (`external.rs:311`, an unclaimed presence that is "Never a payee"). The precedent is
`bridges/jenkins/src/translate.rs`, which models the edge pipeline as a valueflow that is observed,
not in control. It mints its `Process` with `inputs: Vec::new()` (`:405`); that empty list is the
missing node.

## 3. Where each external system enters

**cargo is observed and not in control.** A `bridges/cargo` crate translates cargo's resolve graph
into epr-rea, as `bridges/jenkins` translates Jenkins: the bridge seam, where a crate translates an
external protocol. It reads `cargo metadata --format-version 1 --locked` and emits, into the epr
flow store, one offer whose `disclosure.inflows` name unclaimed presences, one `Process` with a
`Cite` input per resolved package, and a `DepEdge{Governor::Compiler}` per direct edge. Because it
lives in the monorepo it needs no attested pin move, so the fleet does not roll. This placement
needs the operator's confirmation (§9, question 1).

**rakia** owns the process side: build manifest to `ProcessSpec`, and the input closure. **brit**
seals and attributes: its snapshot carries the reliance records, and the composition spec's
`InterfaceRef{kind: external}` is the later home for imports. Both change only where their own
concern is touched, in later waves.

As illustration only, the grocer would enter through a supplier-invoice or point-of-sale bridge
and the garden network through a seed-library bridge, writing the same records into the same
fabric. Neither is designed here.

## 4. Missing nodes

**1. Inputs on a process.** Chain: recipe to process, between `ProcessSpec` and the `Process`
outputs. Missing node: input events on the Process. Assertion: a Process lists a `FlowEvent{Cite}`
for each resource it rests on. Probe: a bridge-minted Process has non-empty inputs whose CIDs
resolve. Current state: `translate.rs:405` mints `inputs: Vec::new()`.

**2. Identity for an outside package.** Chain: lockfile entry to resource. Missing node: external
resource identity. Assertion: a package version is addressed by the CID that wraps the lockfile's
own checksum: `bafkrei…` (raw codec) over sha256 for crates.io; a sha2-512 multihash for npm
`integrity`. Probe: recompute from the `.crate` bytes and get the same CID. Home: `epr::cid`
(`raw_cid_from_sha256_hex`, `raw_cid_from_sri`, `compute_raw_cid`), not epr-rea: `epr` is the
crate that already mints CIDs, and epr-rea, elohim-storage and the bridges all depend on it.
epr-rea takes its addressing from `epr` alone, so a second home would mean a second address
format. Its tests pin the same raw-codec vector that
`eprfs-core::BlobCid` and brit pin. Sha1 integrity strings are refused, not wrapped. `DepEdge`
names an outside package through an additive `to_resource: Option<Cid>`, with `to` keeping a
readable coordinate such as `crates.io:serde@1.0.228`. Before this, `DepEdge.to` was a path
only; "slug identity for code artifacts is an open question" (`model.rs:480`).

**3. One action vocabulary.** Chain: action vocabularies, between the epr-rea verbs, the DNA's
actions and the SDK. Missing node: one generated action enum. Assertion: `cite`, `use`, `consume`,
`produce`, `transfer`, `transfer-custody`, `modify` and the rest come from one
`elohim/sdk/schemas/v1/enums/rea-action.schema.json`, codegenned to Rust and TypeScript.

As built, the enum has 27 values. Its core tier is exactly the DNA's 25 `REA_ACTIONS`; the other
two are `affirm` and `dismiss`, the review verbs `ReaVerb` carries, so every `ReaVerb` maps onto a
generated word (`ReaVerb::as_action`). The parity rule has two schema keys, because the usual
`_dna` key would generate constants into the integrity zome and move the DNA hash. `_codegen`
generates the list into TypeScript and into the `epr` crate only. `_dnaMirror` makes
`pnpm run schema:check-dna` read the hand-written `REA_ACTIONS` from the integrity source as text
and hold it equal to the core tier; changing the DNA list stays a deliberate DNA move, made in the
same change as the tier. One honest limit: `REA_ACTIONS` is declared but no integrity rule
validates an action against it, so membership is a shared name, not an enforced gate.

Two follow-ons are named and not built. `ReaVerb` is not widened: adding `Transfer` for rival goods
touches the exhaustive matches in `witness.rs` and `epr-rea/src/epistemic.rs` and needs kebab-case
serde for multi-word verbs. And the enum still lacks action strings used outside the DNA list:
`respond-to-challenge`, `custody-spool`, `hosting-agreement`, `sunsets-lineage` and
`correction-action` in elohim-storage and doorway, and the mishpat commitment actions
`delegates-compute` and `replicates-dwelling`.

Probe: the codegen verify gate and `schema:check-dna`. Before this, `ReaVerb` had 6, DNA
`REA_ACTIONS` had 25 and the SDK had no enum. The fabric spec,
`genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md`, tracks this
three-way drift in §2.6 and §8.

**4. A manifest for any pinned outside artifact.** Chain: pinned artifact. Missing node: a general
external-artifact manifest, generalized from the model manifest: bytes or source CID, `license` as
an SPDX expression, the CID of the license or NOTICE text, `source`, upstream repository, and
declared funding channels as facts. Probe: the model manifest validates against it unchanged.
Current state: the shape exists once, in the model manifest.

**5. Ancestry of forks and vendored code.** Chain: ancestry, between a gitlink and the upstream it
came from. Missing node: `derived_from` declarations for brit (from gitoxide), sophia (from
Perseus), holochain-conductor (from holochain) and the vendored quinn (from quinn at `434c3586`),
each in the parent repo's `.epr-meta` beside its gitlink, so no pin moves. Assertion: each declares
the existing SDK `derived_from`. Probe: each declaration parses, its ancestor resolves to a held
presence and a pinned upstream commit, and the fork's carried licence text has the same CID as the
ancestor's. Current state: the relationship exists; the declarations do not.

**6. From observation to lasting record.** Chain: observation to durable record. Missing node: a
caller for the graduation evaluator's Path 2: one summary per tracked `Cargo.lock`, carrying
`observation_refs` to its cite events. The repository tracks 35 lockfiles, and each summary is a
versioned head for its lockfile: when the lockfile changes (a release), a new summary supersedes the
old one. Which summary applies is declared by the lockfile it summarizes, not resolved by
recency (the p2p-design-gate rule for versioned heads). Assertion: cite
events stay in the operational flow store (class C, rebuilt from lockfiles) and only the summaries
are candidate heads, the composite-root shape: 35 heads in total, not 1,211, and the count does not
grow with releases. In the past year `dev` had 285 lockfile-changing commits across all lockfiles
(34 of them for `elohim/Cargo.lock`), so expect about 285 head moves a year. Probe: each lockfile's
current summary lists `observation_refs` equal to the cite set recomputed from that lockfile.
Current state: the evaluator is built and exercised by tests only.

**7. From member records to a commons.** Chain: member records to commons. Missing nodes: a
`CoverageDomain` or rollup atom for reliance keyed by upstream presence; a collective as a party;
`witness_quorum`; federation of aggregates. This node is deferred to sequence step 8, the pool
spec, and carries no acceptance criterion here. Current state: `CoverageRollup` rolls up one
level, unchained.

## 5. P2P design gate

### Entity: cite events, their Process, and the per-lockfile summary

- **Classification**: Ephemeral (C) for each cite event and its Process. The summary is a candidate
  head: written class C in step 7, as the evaluator's summary event with `observation_refs` in the
  epr flow store; its graduation to a notarized head follows the recipe-declared graduation policy
  and is deferred past this spec.
- **Justification**: a cite event can be deleted and rebuilt from its lockfile. What would be a lie
  if changed silently is a lockfile's statement of what it rests on, which the summary carries.
- **Head-Plane Cost Budget**: cite events, none. Summaries, once graduated: 35 versioned heads, one
  per lockfile, at seed and at one year; the count does not grow with releases. About 285 head
  moves a year (285 lockfile-changing commits on `dev` in the past year, 34 of them for
  `elohim/Cargo.lock`). Bundling shape: composite root (`observation_refs` point at the cite
  events).
- **Network Stakes**: unresolved — the plan declares no stages or floor-protected costs.
- **Content Address Strategy**: Content-Derived (CID).
- **Address Justification**: a cite names a package version by the CID wrapping the lockfile's
  checksum. The lockfile is the declared head; nothing resolves by recency.
- **Transport Affinity**: n-a; the record names a package's bytes and does not carry them.
- **Source of Truth**: operational, the epr flow store, rebuilt from lockfiles; the summary too,
  until it graduates.
- **Integrity Zome + DNA-hash class**: none touched — DNA-hash-NEUTRAL. On graduation the summary
  is the existing elohim-DNA `EconomicEvent` with action `cite`, through Path 2 (§9, question 4,
  settled); integrity only checks that the action is non-empty, so no hash moves. `FlowEvent`'s
  doc comment (`model.rs:373-374`) says the same: an observation "crystallizes to the DHT
  `EconomicEvent` per recipe-declared graduation policy".
- **Coordinator Zome**: none in this spec.
- **Projections**: no SQLite table with `dht_anchor_hash`. Automerge sync at reach tier:
  `unresolved — reach vocabulary in declared drift`.
- **HTTP Route**: none.
- **Anti-Pattern Check**: the design addresses packages by the CID that wraps the checksum, not by
  the bare checksum. The three-address-formats risk is live: `DepEdge.from`/`to` are repo paths
  while the cite names a CID; node 2 is where they meet.
- **SDO/RWA Test**: (Drafted with the spec; for the operator's review.) The worst holder of the
  reverse index sees, across members, who relies on a public package. That is a cross-holon join,
  bounded by boundary 2 ("activity ledgers are held by the holon they describe; no cross-holon join
  without each holon's own crossing act") only if each member's publication is its own act. The pool
  is public reach, and the 2026-10-04 pool spec builds public-purpose pools only. Nothing is
  compelled: each record comes from the member's own lockfile, and members may remove their own
  records.

### Entity: `DepEdge` (one per direct edge)

- **Classification**: Ephemeral (C) for the edges `bridges/cargo` mints. They carry a `Compiler`
  governor, which means "no seal: cite the unit", and they are computed from cargo's resolve, so
  they are rebuilt, never graduated. A hand-sealed `DepEdge` keeps the B2 class its doc comment
  states (`elohim/epr-rea/src/model.rs:477-481`), and even then gap #11 graduates one crystallized
  *sealed edge-set* into the existing Attestation at push, never single edges ("B2: granular seals
  stay sidecar, the crystallized proof notarizes";
  `genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md`
  :216 and :266). The bridge is the "later accelerator" that spec's §7 names (:251), mining
  cargo's dependency graph into edges; it is not a consumer of gap #11.
- **Justification**: the edge restates what cargo already enforces; deleting it loses nothing the
  lockfile cannot rebuild.
- **Head-Plane Cost Budget**: none.
- **Network Stakes**: unresolved.
- **Content Address Strategy**: Slug, plus a CID for an outside upstream.
- **Address Justification**: `from` and `to` are repo-relative paths, the v1 floor; "slug identity
  for code artifacts is an open question" (`model.rs:480`). An edge to an outside package keeps a
  readable coordinate in `to` (`crates.io:serde@1.0.228`) and names the bytes in the additive
  `to_resource`, the CID node 2 wraps from the lockfile checksum.
- **Transport Affinity**: n-a.
- **Source of Truth**: the `.eprfs/status/` sidecar, rebuilt from the lockfile.
- **Integrity Zome + DNA-hash class**: none touched — DNA-hash-NEUTRAL.
- **Coordinator Zome**, **Projections**, **HTTP Route**: none in this plan.
- **Anti-Pattern Check**: as for cite events, the path-versus-CID split is live.
- **SDO/RWA Test**: (Drafted with the spec; for the operator's review.) An edge between two public
  packages reveals what the lockfile already reveals.

### Entity: upstream presence held in trust (`Inflow.presence`, promoted to `ContributorPresence`)

- **Classification**: as `Inflow.presence` (a `presence:<slug>` string in the offer's disclosure)
  it lives in the flow store with the offer. The bridge names an unclaimed presence for every
  upstream, so every dependency has its held seat as a record from day one. Promoted, it is
  Notarized (A), reusing the existing imagodei `ContributorPresence` type; only that promotion waits.
- **Justification**: a held seat that could be silently moved or erased breaks the covenant; a type
  already exists, with the lifecycle unclaimed, stewarded, claimed.
- **Head-Plane Cost Budget**: at seed, the four ancestors plus the existing prior-art cohort (the
  eight files in `genesis/data/presences/` with `standing: prior-art`). Beyond the seed cohort,
  promotion is deferred to the pool spec (step 8); its trigger is §9, question 5. Promotion
  approaching one per upstream project would pass ~500 and need a bundling shape or the operator's
  sign-off.
- **Network Stakes**: unresolved.
- **Content Address Strategy**: Slug.
- **Address Justification**: `Inflow.presence` is `presence:<slug>`; `ContributorPresence` carries
  `id: String` and `external_identifiers_json`. The gate's "external system integration" case
  applies: the upstream project's repository URL is the canonical external identifier, carried in
  `external_identifiers_json`, and the slug is its local alias. One presence per upstream project,
  `presenceType: organization`; by design, no presence is minted for a person from commit emails.
- **Transport Affinity**: n-a.
- **Source of Truth**: the flow store until promotion; Holochain DHT once promoted.
- **Integrity Zome + DNA-hash class**: `imagodei_integrity` — DNA-hash-NEUTRAL (existing type;
  `standing: relied-on` goes into `presences.schema.json`, not integrity).
- **Coordinator Zome**: `imagodei::create_contributor_presence`
  (`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2020`) `-> PresenceOutput`.
- **Projections**: SQLite `contributor_presences` (initial migration;
  `elohim/elohim-storage/src/db/contributor_presences.rs`), `dht_anchor_hash`: yes. Automerge sync
  at reach tier: `unresolved — reach vocabulary in declared drift`.
- **HTTP Route**: none.
- **Anti-Pattern Check**: cross-namespace identity matching is refused by the commit-email rule. No
  sovereignty framing.
- **SDO/RWA Test**: (Drafted with the spec; for the operator's review.) Person presences minted
  from commit emails would let the worst holder join authorship across projects by email, against
  boundary 5 ("correlation across identity namespaces is a consent act"). Organization presences
  hold only what each project already publishes. Claiming is the makers' own act.

### Entity: derivation declaration (`derived_from`)

- **Classification**: unresolved. The plan puts each declaration in the parent repo's `.epr-meta`
  beside the gitlink without classifying it. The gate's rule (an authored edge is notarized, a
  computed one is not) points to Linked (A2); operator to confirm.
- **Justification**: the downstream project authors the ancestry claim.
- **Head-Plane Cost Budget**: four declarations at seed.
- **Network Stakes**: unresolved.
- **Content Address Strategy** and **Address Justification**: Composite: the ancestor is named by
  its held presence, its repository URL, and the pinned upstream commit or release the fork
  departed from; the licence text it carries is named by CID.
- **Transport Affinity**: n-a.
- **Source of Truth**: the version-controlled `.epr-meta` file; none of the gate's three options
  applies yet.
- **Integrity Zome + DNA-hash class**: none — DNA-hash-NEUTRAL; no pin moves.
- **Coordinator Zome**, **Projections**, **HTTP Route**: none.
- **Anti-Pattern Check**: none caught; reuses the SDK relationship instead of adding a fifth list.
- **SDO/RWA Test**: (Drafted with the spec; for the operator's review.) A declaration states what
  each fork's history already makes public, and names projects, not people.

### Design constraints discovered

- Bridge-minted `DepEdge`s and cite events come from the same lockfile and share its class: both
  are C and neither graduates. Only the per-lockfile summary is a candidate head (§9, question 3,
  settled).
- The generated action list is checked against `REA_ACTIONS`, not edited into integrity, because
  editing integrity moves the DNA hash.
- The composite root holds only while cite events stay off the head plane, and the head count stays
  at 35 only while each summary supersedes its predecessor rather than adding a head per release.

## 6. Commons-design reading

The pool's **facility** is rival: hosting members' published reliance records, the compute that
rolls up the reverse index, and the work of keeping it fresh. Its **artifacts** are non-rival: the
cite events, the reverse index and the held presences, which a maker improves by reading them.

```
Pool: dependency-index  reach: public (contextual reach pending)  parts: facility=hosting+rollup+freshness labor  artifacts=cite events, reverse index, presences
Present (declaration + probe): none yet
Declared, unwired:  Q6 (Membership exists; no join rule chosen)  Q7 (holdings = reliance records + index)  Q11 (reach ladder)
Absent:             Q1 Q2 Q3 Q4 Q5 Q8 Q9 Q10
Failure-predicting absences: Q1 Q2 Q3 Q4 — the spec must have the charter choose them
Labor misplaced on the elohim: none in this design — reading lockfiles, matching presences and drafting records sit with each member's elohim; recognition, valuation, admission and settlement sit with persons
Plurality: lineages=2 available (Claude, Codex)  closure-across=UNDECLARED
Rights on artifacts held by members: access contribution removal(own records, trace kept) management exclusion(fabricated records, by ladder)
Alienation: held by no one
```

Q1 to Q11 are the eleven declarations the pool's charter must choose, from Elinor Ostrom's design
principles as `.claude/skills/elohim-commons-design/SKILL.md` words them. The four whose absence
most predicts failure are Q1 congruence (rules fit this pool's people and resource), Q2
proportionality (draws proportional to contributions), Q3 monitoring the resource's condition, and
Q4 graduated consequences that start with a notice. The rest: Q5 monitoring users and their draws;
Q6 user boundaries, how one joins and leaves; Q7 resource boundaries, what the pool holds; Q8
collective choice, how the rules change; Q9 the arena for resolving conflicts; Q10 recognition by
the parent holon; Q11 nesting and the member ceiling.

Two consequences for the build. **The local gate's default is rung one, a notice rather than a
refusal**: a stale reliance record produces a notice and an offer to regenerate, and proceeding
records the choice, as the warn-only T2-receipt leg does. Strict mode (`RELIANCE=strict`) is
opt-in. **There is no pool-elohim**: the sensor is an algedonic measure, `reliance-record-fresh@1`,
read by our own habit.

## 7. Sequence

1. **Docs lead:** this spec; a blind-reader pass; the backlog item
   `dependency-graph-cited-contributor-economy` moves to `designed`, linking here.
2. **`COVENANT.md`** at the repository root: written, and passed a cold read as an upstream
   maintainer (READY). It sets out its terms, each with a "Today" part (what holds now, and what
   checks it) and a "Promised" part; a "Where we stand today" section; and a closing "A covenant,
   not a treaty".
3. **Shared primitives:** node 3 (action enum, codegen, verify gate), node 2 (external CID identity
   in `epr::cid`, reached from epr-rea through `DepEdge.to_resource`), node 4 (external-artifact manifest schema, validated against the
   model manifest).
4. **`bridges/cargo`** as in §3. Also fill node 1 for the Jenkins edge Process, which gains its
   source and image inputs.
5. **Node 5:** the derivation declarations. Promote only the four ancestors and the existing
   prior-art cohort to seed presences (`standing: relied-on` added to `presences.schema.json`).
6. **Habit and gate:** `genesis/orchestrator/.epr-meta/cites-own-dependencies.habit.md`, born
   `unwired` with a `retire-when`; a gate project in `genesis/orchestrator/build-manifest.json`
   (modelled on `rakia-codegen`) running the bridge in verify mode, notice first, `RELIANCE=strict`
   opt-in; re-project with `habits-project.py`.
7. **Node 6:** the Path 2 summary per lockfile, written to the epr flow store as class C and
   superseded on each release. Graduation to a notarized head is deferred past this spec.
8. **Pool spec:** `commons-pool:dependency-index` on the 2026-10-04 pool spec, with the charter's
   eleven declarations to choose, node 7, presence promotion beyond the seed cohort, and §9's
   questions recorded for ruling.
9. **npm** through a `bridges/pnpm` crate over the same nodes.

Already decided by the operator on 2026-10-15: Rust first, then npm; `COVENANT.md` with the
licenses as they are today; write the pool spec and build nothing in the pool ring yet.

## 8. Verification

- **Covenant:** every "Today" claim in `COVENANT.md` can be checked in the repository, and every
  term without a running check is marked "Promised".
- **Node 3:** `pnpm run schema:codegen:ts` and the Rust codegen verify pass; existing `ReaVerb`
  users and the DNA constant resolve from one source, the generated list checked against
  `REA_ACTIONS` with integrity untouched.
- **Node 2:** a fixture `.crate` file, hashed with sha256, gives the same `bafkrei…` as the CID
  wrapped from its `Cargo.lock` checksum.
- **Node 4:** the existing model manifest validates against the new schema with no edits.
- **Bridge** (`cargo test` in `bridges/cargo`): a fixture workspace yields a Process with one `Cite`
  input per resolved package, deterministic across two runs; each input names an unclaimed
  presence and carries its declared SPDX licence (the licence-text CID needs the registry source
  on disk and follows); one run over `elohim/` reports its scale, with `EXIT=$?` echoed on its
  own line. 1,211 is the number of distinct crates across all 35 lockfiles, not one workspace's
  scale: the largest lockfile, elohim-storage's, has 1,066 packages, and at `origin/dev`
  245d7aec8 `elohim/Cargo.lock` has 468, of which the `elohim/` workspace resolves 457 and cites
  438 (the rest are its members).
- **Gate:** bump one crate in `Cargo.lock` without regenerating; `just gate` prints the rung-one
  notice naming the workspace and passes. With `RELIANCE=strict` it refuses.
- **Habit:** `habits-project.py --check` is clean; `habits-status.py` shows
  `cites-own-dependencies` unwired, then green, with the gate run cited in its DELTA.
- **Spec:** a blind-reader pass, and a cold read of the §2 table by someone thinking about a
  grocer: does every row hold without code words?

## 9. Open questions for the operator

1. **Where the reader lives.** The plan moves the dependency reader from rakia and brit upstream
   into `bridges/cargo` in the monorepo, so no attested pin moves. Confirm.
2. **Settlement and bridges.** Offer terms say settlement is "a separate governance act", and node
   4 records funding channels as facts only.
   1. Should a settlement bridge pay through channels makers have declared?
   2. If so, who rules on it, and when?
   3. brit's `FUNDING.json` is inherited from gitoxide and declares a Drips channel set up by
      Sebastian Thiel. Does a channel our fork inherited count as its maker's declaration to us?
3. **Do bridge-minted `DepEdge`s graduate? Settled: no.** They are computed from cargo's resolve and
   stay class C. Gap #11 graduates one crystallized sealed edge-set, never single edges (the
   sealed-edges spec, :216 and :266); see §5. Settled in the working memorandum
   `genesis/research/2026-10-15-covenant-commons-working-mou.md`, §F (F1).
4. **The summary's notarized entry type. Settled:** the existing elohim-DNA `EconomicEvent` with
   action `cite`, through Path 2; no new entry type. Building it means bridging three mismatches
   between Path 2 and this caller: Path 2 summarizes a time window, where ours is a versioned head
   per lockfile; Path 2 reads storage's `observations` table, where the cite events live in the
   flow store; and `total_quantity` is an f64. Supersession is chosen by the declaring lockfile,
   never by the newest link in the `triggered_by` chain. Same memorandum, §F (F2).
5. **What promotes a held presence.** Every upstream has an unclaimed presence in the flow store
   from day one. Beyond the seed cohort, what triggers promotion to a notarized
   `ContributorPresence`? Deferred to the pool spec.
6. **A pool treasury.** The 2026-10-04 pool spec allows a separate treasury resource for a pool's
   care. `succession.md` §7.1 says "a protocol with a treasury" has a chokepoint, and the corpus
   refuses a published default allocation (stewardship backlog rows 19 and 22). May
   `dependency-index` hold a treasury, and if money flows, how without a default allocation?
7. **Who stewards a held presence.** `ContributorPresence.steward_id` is one optional string. An
   elohim in that seat conflicts with the rule that recognition does not belong to the elohim; the
   commons reading puts recognition, valuation, admission and settlement with persons. Backlog row
   3 adds that a flat `steward_id` cannot express a steward as a path.
8. **Backlog rows 32–38** (`commons-holonic-stewardship-backlog.md`). Which must
   `dependency-index` settle before it is built?
   - Row 32: which reach tiers may carry a binding decision, and what committed outcome record
     closes one.
   - Row 33: whether every contest fixes its rules before participants are known and its
     randomness after the pool is frozen.
   - Row 34: whether a draw is narrowed by the member's contribution-to-draw ratio over a window
     the charter declares.
   - Row 35: how members amend the charter, closing with an outcome two peers compute alike.
   - Row 36: which graduated-consequence ladder the members agree to, notice first, and which
     arena hears disputes.
   - Row 37: which pool-condition measure members can read, and the member ceiling or parent pool.
   - Row 38: how many distinct elohim lineages members' seats need, and what cross-lineage
     agreement a binding outcome needs.

   The orchestrating agent recommends that rows touching Q1–Q4, the failure-predicting
   declarations, block the build and the rest follow. By the Ostrom principles each row names, that
   is row 34 (proportionality, Q2), row 36 (graduated consequences, Q4) and row 37 (monitoring the
   resource, Q3); rows 32, 33, 35 and 38 would follow. Confirm or override.
9. **Four drifting relationship vocabularies:** the zome constants, the lamad manifest's
   relationships, the lamad graph edges, and the SDK relationship enum. This plan uses the SDK's
   `derived_from` as it stands. Which list owns the relationship, and when are they unified?
10. **Reach vocabulary drift.** Every reach-tier field above is unresolved, and the pool's reach
    reads "public (contextual reach pending)". The gate forbids canonizing a tier name until the
    drift resolves.

## Sources

- `/projects/.claude-config/plans/zippy-conjuring-gem.md` — the approved plan, technical source of
  truth
- `COVENANT.md`
- `genesis/docs/content/elohim-protocol/manifesto.md` (Part IV-B, line 692)
- `genesis/docs/content/elohim-protocol/glossary.md`
- `genesis/docs/content/elohim-protocol/succession.md` (§7.1)
- `genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md`
- `genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md` (§2.6, §8)
- `genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md`
  (gap #11: :216, :266; the later accelerator: §7, :251)
- `genesis/research/2026-10-15-covenant-commons-working-mou.md` (§F, settling §9 questions 3 and 4)
- `genesis/docs/architecture/private-thought-governed-fruit.md` (§4)
- `genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md` (rows 3, 19, 22, 32–38)
- `.claude/skills/p2p-design-gate/SKILL.md`
- `.claude/skills/elohim-commons-design/SKILL.md`
- `elohim/epr-rea/src/model.rs`, `elohim/epr-rea/src/external.rs`, `elohim/epr/src/witness.rs`
- `elohim/elohim-storage/src/graduation/`
- `bridges/jenkins/src/translate.rs`
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`,
  `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`,
  `elohim/holochain/dna/imagodei/dna.yaml`
- `elohim/elohim-storage/src/recursion.rs`, `elohim/elohim-storage/src/db/contributor_presences.rs`
- `.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json`
- `elohim/brit/FUNDING.json`
