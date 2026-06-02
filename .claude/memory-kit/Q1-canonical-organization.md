# Q1 — How the Canonical Surface Organizes into Developer Documentation

Generated: 2026-06-02 · READ-ONLY analysis · scratch synthesis doc
Inputs: surveys A1 (canonical), A2 (spine), A3 (code↔doc), B1 (work surface), B2 (vision) + filesystem verification.

---

## TL;DR

The canonical surface is **strong material with a fragmented index**. It already has the right
bones — an architecture spec set with a declared graph contract (`realizes / informed-by / informs /
defers`), seed epics as siblings, application archetypes as proof galleries, and SDK domain
vocabularies. What it lacks is **one organizing artifact that lets a human developer walk
manifesto → epic → architecture seed → pillar → code without manually reconstructing the graph.**

The natural organization is **two-axis: subject-domain (concern) for architecture, pillar for code.**
These two axes already exist independently; nothing connects them. The single highest-leverage
artifact is a **top-level `genesis/docs/content/elohim-protocol/architecture/MAP.md`** (or
`DEVELOPER-PATHS.md`) that renders both axes as one navigable table, plus closing two pillar-guide
gaps (lamad, avodah). The household-living-core seed (B2 gospel) should be the **default reading
entry**, not the chronological-first or breadth-first one.

---

## (1) THE SUBJECT-DOMAIN MAP

The canonical surface partitions cleanly along **two boundary types**, and the survey confirms only
one of them is operative for the architecture seeds:

- **Pillar boundary** (lamad/shefa/qahal/…) — the boundary for *code*, NOT for architecture seeds.
  Architecture seeds are cross-pillar by design (e.g., records-lifecycle touches all six).
- **Layer boundary** (DHT notary / libp2p data-ops / diesel / doorway projection) — strictly
  respected; this is the primary dividing line *within* a subject.
- **Concern boundary** (EPR envelope vs records-lifecycle vs transport) — the **operative division
  for architecture seeds.** This is the subject-domain axis.

### The 10 subject domains (17 seeds), with crisp boundaries

| # | Subject domain | Seeds | Where it ENDS (boundary to next) |
|---|----------------|-------|----------------------------------|
| **D1** | **EPR Envelope & Graph Substrate** (foundation, no pillar owns it) | core-graph-substrate, epr-integrator-compatibility-contract, epr-phase-2c-libp2p-federation | Ends at *what an atom is + how it signs/travels*. Where a payload acquires lifecycle state → D3. Where a payload is evidence → D2. |
| **D2** | **Evidence Primitives** (attestation ↔ observation split) | experience-story-epr, attestation-consolidation, observation-event-layer, dna-signal-as-epr-envelope | Ends at *the typed evidence carriers and the DHT/libp2p seam*. Attestations stay DHT-durable; observations are libp2p-ephemeral. Where they acquire Active→Closed status → D3. |
| **D3** | **Records Lifecycle & State Transitions** (master spec) | records-lifecycle-design (+ applications/) | Ends at *the substantive-record state machine* (Active/Subordinate/Shelved/Closed). Where data *retention/forgetting* is the subject → D4. |
| **D4** | **Memory Lifecycle & Consolidation** | memory-lifecycle-design | Ends at *the comet shape* (submerge/surface, consolidation, network-scale merge). Sibling to D3; seam = "substantive records vs data-retention." Reconciliation noted incomplete. |
| **D5** | **Data Plane & Transport** | iroh-libp2p-complementarity, tiered-quilt-stewardship | iroh-libp2p = *transport choice/diversity*; tiered-quilt = *governance/lifecycle of content custody* atop either transport. Where bytes are governed-as-economy → emits into D9. |
| **D6** | **Runtime Topology & Composition** | elohim-hub-boundaries | Ends at *the Hub trait + three-crate split* (hub trait ↔ node instance ↔ storage participant). Where collectives coordinate across hubs → D7. |
| **D7** | **Collective Coordination & Governance** | multi-collective-collaboration-epr | Ends at *recursive Qahal + dual-integrity consensus + friction-gradient*. Chain-layer mechanics named but unspec'd (gap). |
| **D8** | **Web2 Projection & Doorway** | doorway-access-tier-patterns, doorway-ssr-runtime | access-tiers = *reach-gating + cache-scoping*; ssr-runtime = *compute capability*. Non-overlapping. The Track-4 projection boundary; peer-native stays clean behind it. |
| **D9** | **Economic Coordination & REA Interop** | wave3-valueflows-hrea-interop | Ends at *hREA/VF-GraphQL ↔ EPR-REA translation* (bridges/valueflows). Lives in elohim-storage (protocol-shaped), NOT doorway. |
| **D10** | **Testing Infrastructure** (non-architectural floor) | sweettest-integration-layer | Ends at *the DNA integration-test tier*. Exercises D1 substrate; not a substrate concern itself. |

**Cross-cutting (no domain owns):** D1's envelope + IoC contract are foundational to all of D2–D9.

### How subject-domain maps onto pillar (the second axis)

The pillar coupling matrix (A1) shows architecture seeds fan across pillars; the *code* lives by
pillar. The legible cross-walk:

| Pillar | Owns code in | Shaped by subject domains |
|--------|--------------|---------------------------|
| **elohim** (core) | substrate primitives, runtime, data-ops | D1, D2, D3, D5, D6, D9 |
| **lamad** (learning) | content vocabulary, archival | D2, D3, D9 (+ seed epic: value_scanner, social_medium) |
| **shefa** (economy) | REA events, tier economics | D5, D9 (+ economic_coordination) |
| **imagodei** (identity) | custody, key-revocation, agent memory | D2, D4 (+ resilience) |
| **qahal** (community) | collective primitives, membership | D7 (+ governance) |
| **mishpat** (governance/judgment) | scenario hygiene, demotion/dissolution | D3, D7 |
| **doorway** (web2) | Track-4 projection, SSR | D8 |
| **avodah** (process) | reference impl of work-as-participation | D1 (demonstrator only) |

This is the table that does **not** exist anywhere as a single artifact. Both axes are real and
correct; the gap is purely that no document renders them together.

---

## (2) THE NEW-DEVELOPER SPINE

### The intended spine (and it IS declared — in the architecture INDEX)

`architecture/INDEX.md` already states the spine explicitly:

> "the architecture navigable in both directions: epic ↔ architecture spec ↔ sprint spec ↔ code."

So the spine is **manifesto → seed epic → architecture seed → (pillar) → code**, knit by the four
frontmatter edges. The bones exist. What's missing is a **human walk-path** over those bones.

### What exists at each tier (verified)

| Tier | Artifact | State |
|------|----------|-------|
| **T1 Vision** | `/README.md`, `manifesto.md` | Excellent. Names six pillars + three dimensions. Links to architecture (10×) but only weakly to seed epics by name (living_memory 1×, resilience 2×). |
| **T1.5 Seed epics** | `value_scanner/`, `living_memory/`, `autonomous_entity/`, `economic_coordination/`, `governance/`, `social_medium/`, `public_observer/`, `resilience/` | Present, gospel-dense (B2). But no index ties them to the architecture seeds that realize them — you must read `realizes:` edges seed-by-seed. |
| **T2 Architecture** | `architecture/INDEX.md` + 17 seeds + applications/ (8) + horizons/ (4) | Excellent reference library. INDEX is concern-organized, transparent about its own scope, but offers **no bridge to pillar-first reading**. |
| **T2.5 Domain vocabulary** | `elohim/sdk/domains/<pillar>/manifest.json` + per-domain CLAUDE.md (9 domains incl. infrastructure, mishpat) | Present but un-signposted from T2; reader isn't told when to consult which. |
| **T3 Pillar guides** | app pillar `claude.md`: elohim ✓, imagodei ✓ (218-line template), qahal ✓, shefa ✓; **lamad ✗** (separate SPA at `app/lamad/` w/ own CLAUDE.md), **avodah ✗** (dir exists, no guide) | Better than A2 reported (4/6 app pillars have guides), but 2 gaps + lamad indirection remain. |
| **T4 Tests/scenarios** | `genesis/a2o/features/<pillar>/` | Present; story-first mapping is brief in CLAUDE.md. |

### Verdict: the spine **exists as a graph but not as a path**

A new developer can *traverse* the spine (the edges are real and bidirectional in intent) but cannot
*follow* it — there is no "start here, then here" document. Today the implicit path is:

1. README → manifesto (vision: clear)
2. CLAUDE.md (Rust↔TS boundary, deployment contexts: dense, assumes context)
3. architecture/INDEX.md (substrate primitives: concern-organized)
4. …then drop into code with **no pillar-to-architecture cross-walk and no per-pillar reading order.**

imagodei is the one pillar where the path completes (its guide closes the loop). Everywhere else the
developer reconstructs the graph by hand. **The spine needs an index, not a rewrite.**

---

## (3) THE GAPS

### A. Subjects with NO seed (canonical holes)
1. **Chain-layer consensus mechanics** — named in multi-collective ("consensus weight =
   care-aggregate") but no spec. Infrastructure-tier; OPEN. Blocks qahal-grade authority design.
2. **Bridge governance-acceptance gate** — records-lifecycle calls it "Gap 9" (normative for all
   bridges) but no seed defines what a bridge's governance gate looks like. Blocks any new
   `bridges/<vendor>`. OPEN. (A3 corroborates: bridge pattern partially documented, reads as one-off.)
3. **Governance multi-factor merge check** — named in memory-lifecycle §3.3 as load-bearing
   (content-reach / authoritative-governance / valueflows-to-stewards / resiliency) but deferred to
   sibling specs that don't exist. OPEN, known dependency.
4. **Elohim-agent subsystem** (A3) — code exists (elohim-agent crate/service/sdk/specialists/mcp) but
   **no `tier: architecture` seed**. An architect cannot answer "what is an elohim-agent's role in the
   three-layer truth model" from the canonical surface. This is the largest code-with-no-doc hole.
5. **Pillar-service interface spec** (A3) — six pillar modules exist in `app/elohim-app/src/app/` but
   no canonical seed defines "what makes a pillar service" (interface, composition rules, truth-layer
   mapping). Described only through application archetypes.

### B. Seeds that STRADDLE (clarity gaps, not duplicates — burndown cleared real dupes)
1. **records-lifecycle ↔ memory-lifecycle** — both say "lifecycle," distinct subjects (substantive
   records vs data retention). Frontmatter admits "sibling vocabulary; reconciliation incomplete."
   Boundary exists; reconciliation is a known todo.
2. **doorway-access-tier-patterns ↔ doorway-ssr-runtime** — both doorway; seam is clear (reach-gating
   vs compute) but conceptually tight; likely to co-evolve. Flag for a shared doorway sub-index.
3. **Cold-archive terminus** — tiered-quilt names the shelved terminus; records-lifecycle defers
   "organization-dissolution." Neither closes "what happens at the terminus." Cross-reference
   incomplete. OPEN.

### C. Code with NO doc (legibility delta — A3 "moderate")
- **Elohim-agent subsystem** (see A.4) — undocumented at architecture tier.
- **Bridge pattern** — partially documented, no reusable template (see A.2).
- **App pillar services** — undocumented architecture (see A.5).
- **Infrastructure crates** (bitswap, compute, cache, token, render) — most lack README/CLAUDE.md;
  unclear what is load-bearing vs operational.
- **brit crate** (git-as-covenant) — rich internal design docs, NOT surfaced in canonical architecture.
- **Code-anchor links are one-directional**: specs cite code paths, but code files do not cite back
  via `// architecture:` headers. The graph is traversable spec→code, not code→spec.

### D. Pillar-guide gaps (T3)
- **lamad** — no app-pillar guide; bounces reader to separate SPA CLAUDE.md → sdk domain vocabulary.
  The SPA decomposition is itself unexplained (standalone app vs view layer?).
- **avodah** — pillar dir has models/services/components/routes but no guide; reader infers from code.

---

## (4) RECOMMENDATION — the organizing artifact

### Primary: a top-level Architecture MAP that renders BOTH axes as one navigable table

Create **`genesis/docs/content/elohim-protocol/architecture/MAP.md`** (sibling to INDEX.md). INDEX.md
already does "what is each spec + the relationship graph"; MAP.md does the thing nothing does today:
**the subject-domain × pillar cross-walk + the per-pillar reading order.** Keep it to ~2 pages. It
contains exactly three sections:

**Section 1 — Subject-Domain Map.** The D1–D10 table from (1) above, each domain stating its boundary
("where this ends and the next begins"). This gives a designer the concern lattice at a glance.

**Section 2 — The Spine, as a walk.** Six short stanzas, one per pillar, each:
> *To work on `<pillar>`: read (1) seed epic `<dir>/epic.md`, (2) architecture seeds `<D#…>`,
> (3) pillar guide `<path>`, (4) start at code `<path>`, (5) scenarios `a2o/features/<pillar>/`.*

This is the "DEVELOPER-PATHS" content A2 asked for, folded into MAP rather than a separate root file —
keeping it adjacent to the INDEX it complements. **Default the reader to the household-living-core
seed cluster** (per B2 gospel): records-lifecycle Part B + value_scanner + memory-lifecycle +
thin edge-elohim. The map's opening line should say "if you read one path first, read the household
seed — it is the living core, not one of equals."

**Section 3 — The Gap Ledger.** A short, honest table of the (3) gaps above (no-seed subjects,
straddles, code-with-no-doc), each tagged OPEN/known-todo, pointing at the matching
`.claude/memory-kit/gap-items/` entry. This makes the map self-aware about what it does NOT yet cover —
matching the existing repo culture (INDEX.md is already transparent about its own scope).

### Secondary (cheap, high-leverage, can land alongside MAP)
1. **Two pillar guides** — write `lamad` and `avodah` app-pillar `claude.md` to the imagodei template
   (models / services / components / guards / "architecture seeds that shape me" citations). Closes T3.
2. **Backlinks** — add a one-line "shaped by: D#" pointer from each pillar guide to its architecture
   seeds, and the reverse "informs pillar: X" already in seed frontmatter. Makes the spine
   bidirectional in practice, not just in intent.
3. **Manifesto → seed-epic links** — add the eight seed-epic names as explicit links in the manifesto
   so T1→T1.5 is a click, not a directory-scan.

### Why MAP.md and not a root DEVELOPER-PATHS.md
The architecture INDEX *already declares* the "epic ↔ spec ↔ code" spine and the frontmatter graph
contract. Putting the walk-path next to it (rather than at repo root) keeps one canonical navigation
home, avoids a second competing index, and lets the two files split cleanly: **INDEX = the graph
(what each node is + edges); MAP = the path (how a human walks it, by pillar, seed-first).** Two
files, one directory, zero duplication.

### What NOT to do
- Do **not** reorganize the architecture seeds by pillar. They are correctly concern-organized; the
  burndown confirmed no real duplicates. The fix is a cross-walk, not a re-filing.
- Do **not** write new architecture seeds to "fill gaps" in this pass. The gaps (chain-layer,
  bridge-governance, elohim-agent, pillar-service) are real but are *implementation-spec* work
  tracked in gap-items — list them in the Gap Ledger, don't author them here.

---

## One-line answer

The canonical surface is two clean axes — **10 concern-domains (D1–D10) for architecture, 6 pillars
for code** — knit by a declared but un-walked spine; the one artifact that makes it legible is a
**`architecture/MAP.md`** rendering both axes plus a seed-first, household-led per-pillar reading path
and an honest gap ledger, complemented by closing the lamad + avodah pillar-guide holes.
