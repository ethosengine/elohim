---
id: subject-routing-locus-graph-design
status: Draft
class: process-meta
process_subdomain: doc-lifecycle
artifact_kind: spec
written: 2026-06-11
derived_from:
  - genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md
  - genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md
cites:
  - subject-routed-decomposition-design | the class-axis cascade this graph's ROUTING projection extends — class → {write_location, decomposition_flow}; this design adds the citation + scope projections over the same loci | sha256:0d910143a8498b64 | path: genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md
  - semantic-computable-links-design | the cite-envelope + reverse-index propagation this design GENERALIZES from doc↔doc cites to the derived_from / cascade / code-subject edges | sha256:1460bc102580ab0d | path: genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md
  - managed-surface-edit-discipline-design | the edit-time surface registry (in_cite_graph) that answers node membership — the single scope source this graph's nodes draw on | sha256:e5afb16c974b109b | path: genesis/docs/superpowers/specs/2026-06-05-managed-surface-edit-discipline-design.md
  - .claude/subject-routing.yaml
  - genesis/data/timeline/backlog/subject-routing-locus-census.md
---

# The subject-routing locus graph — one structure, three projections

## Thesis

The subject-routing **loci** form a recursive graph that is *simultaneously*:

1. the **decomposition-routing table** (locus → class → homes — `subject-routing.yaml` cascade),
2. the **citation / drift-propagation graph** (gospel/doc → cited targets, fingerprinted — `cites:` envelopes), and
3. the **memory-operation scoping unit** (locus → the surfaces below it — the ceremony/stasis scope).

These are not three systems. They are **three projections of one node set** (the loci), differing only in
edge type. Naming this unlocks three capabilities that have been latent: (a) drift that flows *both ways*
across every edge type, (b) parent-agnostic composition resolved at runtime, and (c) focused memory work
(ceremony or stasis) routed to whichever locus has the weakest canon. This spec is the synthesis; the
implementation status (§6) marks what is wired versus deferred.

## 1. Node + edge taxonomy

**Nodes** are loci — sub-trees with a gospel (`CLAUDE.md`), durable subject docs, and code. The repo root is
`locus: root` (the parent constitution). The 2026-06-11 census (`backlog-subject-routing-locus-census`)
enumerated 19 true in-tree loci; `app/lamad` is the landed reference template.

**Edges** (today only the first is a live drift-edge):

| Edge | Direction | Carrier | Drift-aware today? |
|---|---|---|---|
| **cite** | doc/gospel → target | `cites:` envelope w/ body-fingerprint | ✅ yes (`cite-propagate`) |
| **derived_from** | residue → source spec | `derived_from:` frontmatter | ⚠ lineage breadcrumb only |
| **cascade** | child locus → parent | `.claude/subject-routing.yaml` walk-up | ⚠ composes, not drift-tracked |
| **code-subject** | code site → subject home | `// subject: <slug>` breadcrumb | ❌ convention nascent (lamad seeds it) |

The cardinal move: **promote `derived_from`, `cascade`, and `code-subject` to first-class drift edges** so
they propagate like `cite` does.

## 2. Bidirectional drift (the "both ways" requirement)

A `cite` already does both directions:

- **Forward**: the citing doc stores the target's content-body `sha256` at seal time (`cite-gen --seal`).
- **Reverse**: `cite-propagate` builds a reverse index and fans a target's verdict (`held`/`stale`/`dead`)
  *out* to every citing edge, stamping `status:` inline. A body change → fingerprint mismatch → `STALE` on
  every dependent. *This is the "a change in one place marks drift in derived nodes" mechanism — it exists,
  but only for doc↔doc cite edges.*

The gap is the other three edge types. Extend the same reverse-index fan-out so a **subject-home change** marks:

- citing gospels `STALE` — **works today** (e.g. `app/lamad/CLAUDE.md` → `lamad-domain-gospel`);
- **decomposed residue** that was `derived_from` the changed spec → re-verify queue;
- **sub-manifests** whose pinned assumptions moved (cascade edge);
- **code sites** carrying `// subject: <slug>` → the implementer's re-verification list.

"Both ways" is not bidirectional *detection* (A→B and B→A as separate links). It is: an edge authored in one
direction (dependent → source) is **traversed in reverse** at propagation time (source-change → flag-dependents).
One authored edge, two read directions. The reverse index is the shared primitive; §6 lists the per-edge work.

## 2b. The layered-drift hierarchy (the depth axis)

The edges of §1 carry a **depth direction**, which gives the graph a layer ordering:

```
substrate       elohim-protocol-specification, sdk schemas/wire types     ← the "W3C / HTML spec"
   ↑ assumes
domain          lamad · qahal · imagodei · shefa · avodah · elohim · mishpat   (app-manifest design domains)
   ↑ assumes
implementation  app pillars (render the domain) · crates (steward-node → an orchestration epic)
```

A `cites:`/`derived_from` edge from layer N to layer N−1 is a declaration: *"I assume the layer below; I do
not redesign it."* A domain designer composes ON the substrate the way a web developer builds on HTML
without authoring the W3C spec. Drift flows **up the layers**: when the substrate changes (HTML6 ships),
every domain that cited `elohim-protocol-specification` goes STALE — "re-verify the assumptions your work
rested on" — and that staleness cascades to the implementations citing those domains. The graph tells a
domain designer the ground moved *without their having to watch the substrate themselves*.

Edges are **vertical** (to the layer below — `avodah → elohim-protocol-specification`) or **horizontal** (to
a sibling at the same layer it composes — `avodah → shefa-domain-gospel`, since avodah work creates shefa
value). Both are real dependencies; both drift-track. The depth ordering is what lets a per-locus roll-up
(§4) weight a substrate-level change as higher-blast-radius than a leaf change.

The disposition test for a no-gospel candidate is therefore **"which layer?"**: a true **design domain** (own
app-manifest — avodah/elohim/mishpat) earns a gospel that cites the substrate below; an **implementation
crate** (steward-node, under the compute-hub-storage orchestration epic) cites the epic *above* it, not a
domain of its own; a **marginal** candidate (infrastructure) is held until ownership is decided.

### 2b.1 The domain layer is a LENS layer (operator articulation, 2026-06-11)

The domains are not partitions of the primitive set — there is **ONE shared core** (EPR content atoms + REA
events + governance/reach), and each design domain is a **cohesive lens that reinterprets that core**:

| Lens | Reinterprets the core as… | Archetype |
|---|---|---|
| lamad | learning · mastery · attribution over the nodes | LMS |
| shefa | authoring + value flow + exchange — "how do I author the node," the R, stewarded inventories, the "stuff" axis | CMS × Mint/Analytics × Amazon/real-estate |
| avodah | **process** — *the process as canon*; coordination, flows, risk pools | Cybersyn control room |
| qahal | the social graph — community space of stewards | the network |
| mishpat | consensus, election hygiene, **limits on the rest** | the constitution |
| imagodei | identity ground every lens references | — |
| elohim | the cross-cutting coordination vocabulary itself | substrate-adjacent |

A single `EconomicEvent` *is* value flow to shefa, process telemetry to avodah, attributable contribution to
lamad. Two consequences the avodah pilot proved load-bearing:

- **Lens overlap is the model working, not duplication.** Mutual risk-pooling exists in avodah (process view:
  `CoveragePolicy`, `MemberRiskProfile`) AND shefa (value view: mutual-credit, premium gates) — same core
  primitive, two cohesive readings. A census/triage that treats this as "misfiled" is applying a
  partition model to a lens model.
- **The meta-vs-substrate hazard is universal, per-lens.** Every lens risks confusing its reinterpretation
  with the core it senses (shefa: the CMS owning vs projecting the EPRs; avodah: the control room owning vs
  reading the events). Each lens gospel carries its own form of the hazard rail.

Ceremony refinement from the avodah pilot: lens agents produce confident **category/notarization claims**;
require a substrate citation (manifest/schema/zome wiring) for any such claim — the pilot's only fictions
were uncited architectural assertions ("Category C", invented memory slugs), caught by verification.

## 3. Parent-agnostic composition (the cascade)

A child locus must work **standalone** (a submodule checked out alone is a complete repo) — it must NOT
hardcode a parent. Composition is therefore resolved by the **walker at runtime**, not by the child's content:

- **`.git` is the authoritative boundary** (fixed 2026-06-11). `find_repo_root` previously stopped at the
  first `subject-routing.yaml`, so the monorepo's first sub-manifest *shadowed* root instead of composing on
  it. Now a sub-tree manifest is a cascade member; `.git` bounds the walk. Consequence: in-tree loci
  (`app/lamad`) compose up to root automatically; **submodules bound at their own `.git`, parent-agnostic by
  construction.** Regression-tested in `_lib/__tests__/subject_routing_test.py`.
- **Runtime claimed-parents walk-up** (proposed). For a submodule to *optionally* compose with a parent
  without knowing it: the child declares a `claimed_parents:` marker (a parent **identity**, never a path);
  the walker, running in the embedding context, walks up into the parent **only when** a claimed-parent
  matches where the child is actually embedded. Parent stays base, child nearest-wins. Standalone checkout:
  no parent present, marker inert.
- **Deep-merge deferred** (blocks functional remaps). `_merge_into` does `base.classes[cls] = spec` — a full
  replace, so a *partial* class override in a sub-manifest clobbers root's `write_location`/`discard`/etc.
  Until per-key nearest-wins lands, sub-manifests stay **declarative** (`default_class` + locus identity),
  never overriding `classes`. The lamad sub-manifest documents this constraint inline.

## 4. The locus as universal memory scope

Every locus aggregates the **deterministic drift signals** of everything below it: cite `STALE` counts,
`claude-md` drift accumulators, un-decomposed island docs, un-sealed cite debt, `claimed-unverified` claims,
and `placement-audit --ledger` items under the subtree. Rolled up per locus, this is a **canon-health score**.

That score is a **routing signal**, not just a report. Two consequences:

- **Scoped execution.** The locus is the scope unit for *both* memory operations:
  - `memory-ceremony --locus <path>` — the four-lens deep-read (historian/librarian/cartographer/storyteller)
    operates on one locus's surfaces → *deep, well-written canon* instead of a shallow global pass.
  - `memory-stasis-loop --locus <path>` — drives that locus's whole discipline (compaction · dumps ·
    decompose · MAP/path · cites) to stasis against its own scoreboard.
  - **Global is just `locus: root`** — the existing global ceremony/stasis is the degenerate case.
- **Routing.** The global pass becomes a **router over per-locus roll-ups**: it reads every locus's
  canon-health and answers *where* focused work should run — deepest-drift → ceremony, highest maintenance
  pressure → stasis. The SessionStart `MEMORY BUDGET` headline (and `placement-audit`) gain per-locus lines:
  `ceremony due at <locus>` / `stasis pressure at <locus>`.

This is the same signal-driven-ceremony pattern already used globally (drift-score ≥ threshold → ceremony),
now **fractal**: measured, routed, and executed at locus granularity.

## 5. Why this is one graph, restated

- The **routing** projection answers *where does a new artifact's residue land* (class → homes).
- The **citation** projection answers *what does this doc depend on, and is it still true* (fingerprint drift).
- The **scope** projection answers *where should focused memory work run, and how deep* (canon-health roll-up).

All three index the same loci. A change to a subject home is, in one stroke: a routing fact (its decomposition
homes), a citation event (dependents go stale), and a scope signal (its locus's canon-health drops, possibly
triggering a focused ceremony). Modeling them separately is why drift has been invisible across edge types.

## 6. Implementation status

**Wired (this session):**
- cite envelopes + body-fingerprints + reverse-index propagation (pre-existing).
- cascade compose-fix: `find_repo_root` honors `.git`; 3 regression asserts.
- lamad reference locus: `lamad-domain-gospel` id-anchor; `app/lamad/CLAUDE.md` cite-rail + **code-citation
  discipline** section (`// subject:` breadcrumbs + generated-provenance-as-citation); first cascade
  sub-manifest (`app/lamad/.claude/subject-routing.yaml`).
- census backlog (`backlog-subject-routing-locus-census`) — ~20 loci queued.

**Deferred (backlog — ordered by leverage):**
1. **Per-locus drift roll-up** + `placement-audit` per-locus headline (unlocks §4 routing).
2. `memory-ceremony --locus` and `memory-stasis-loop --locus` scope flags (§4 execution).
3. `_merge_into` deep-merge (unblocks functional per-locus class remaps, §3).
4. Promote `derived_from` / `cascade` / `code-subject` to drift edges in `cite-propagate` (§2).
5. `// subject: <slug>` code-breadcrumb scanner (joins code to the graph).
6. Runtime claimed-parents walk-up resolver + root submodule-awareness stanza (§3).

## 7. Dogfood — the design surfaced its own failure cases

Three live instances of "a source changed and nothing flagged the dependents" appeared while writing this:

- The constitution's header **claims** a `subject_routing:` block in a `CLAUDE.md` cascades; the resolver only
  reads `.claude/subject-routing.yaml`. A doc↔code drift no edge tracked.
- The `find_repo_root` shadow bug: latent until the first sub-manifest — the cascade was never exercised.
- The deprecation-sentinel **false-fired** on the word "Deprecated" inside an *archive* doc — an un-routed
  island in the live scan path. Routing it to `history/` (per the census) removes the false signal at its root.

Each is exactly the failure §2 closes. The graph that routes our artifacts should also be the graph that tells
us when our assumptions about them have moved.
