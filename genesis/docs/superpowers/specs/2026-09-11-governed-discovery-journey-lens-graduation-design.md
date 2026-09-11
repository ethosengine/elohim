---
title: Governed discovery — one journey, reader lenses, and the graduation from local recall to native search
id: governed-discovery-journey-lens-graduation-design
status: proposed
class: devflow
serves: recall-reaches-authority
date: 2026-09-11
requires_env: household-nodes
context-tier: disclosed
steward: agent:orchestrator@claude-fable-5-1
graduation-trigger: station 0-3 landed and the standing reader green across a rolling window; then the L2 rung graduates the recipe to a household peer
cites:
  - "bounded-recall-mastery-sprint | the sprint whose five rounds and three fresh readers proved the journey this design generalizes | sha256:d4c6f66d1685d124 | path: genesis/docs/superpowers/plans/2026-09-11-bounded-recall-mastery-sprint.md"
  - "unified-memory-loop-design | the collective-memory owner split and the graduation-as-reach-rehearsal rule this ladder extends | sha256:07e941a325cc49c2 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md"
  - "private-thought-governed-fruit | the SDO/RWA boundaries every entity row answers — journeys are held by the holon, only outcomes cross | sha256:5b6f5cdb858277e4 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "cradle-to-grave-capability-gradient | the life-stage gradient a human lens preset must hold for — mediated agency, never an autonomy apex | sha256:1a5b2f7e6433230f | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md"
  - "elohim-seam-map-concern-routing | places the lens at the client/SDK seam and the recipe at the SDK seam; providers are bridges | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "ceremony-efficacy | the five fresh-start questions the question bank instantiates | sha256:23eb372f4ba66135 | path: genesis/docs/analysis/2026-09-09-memory-ceremony-efficacy.md"
---

# Governed discovery

One pattern, four interfaces, one graduation ladder. The pattern is the bounded-recall journey
the 2026-09-11 sprint proved on one question; this design makes it composable so that the
three follow-ons — the standing reader (a measured quarter), the bootstrapping head (converge
three surfaces into one), and the reader lens (a preset per reader in its zone of proximal
development) — are the same interfaces applied three times, and so that the same interfaces
carry unchanged when the reader is a human discovering meaning in a community on the native
network.

## The shape (read this first)

```
                ┌──────────────── Recipe (algorithm, EPR artifact, CID) ────────────────┐
  need/scope ─▶ │ scope → discover → filter → group → select → read → judge              │ ─▶ View
                │   budgets (mishpat)      measures (middot)      providers (declared)  │
                └────────────────────────────────────────────────────────────────────────┘
                        ▲                                   │
                 Lens (per reader)                     Sample (per journey)
                 stated ⊕ revealed ⊕ recipe            folds: screens, bytes, mistakes,
                 defaults → preset, revealed           reached — keyed by reader,
                 to the reader, contestable            question, recipe CID, lens CID
```

Four interfaces. Everything below is an instance of one of them.

| Interface | One sentence | Owner today | Graduates to |
|---|---|---|---|
| **Recipe** | The algorithm as a content-addressed artifact: stages, budgets, measures, providers, change authority. | `.epr-meta/elohim/algorithms/recall-contract.json` (v9) | an EPR content artifact contributed to the collective and published at a reach |
| **Lens** | The requisite-variety adapter: how dense, how many choices, how much scaffold — per reader, honesty constant. | none (every reader gets the recipe defaults) | a negotiated preset held by the reader, reviewed and revealed |
| **Sample** | One journey measured: reader, question, recipe, lens, screens, metered and unmetered bytes, mistaken assertions, reached. | folds on the plan, keyed `reader=` | a private journey record whose outcome is attested for the standing measure |
| **Projection** | A rendering of a view at a lens: the first screen, the SessionStart headline, the run-plane line, a doorway page. | three bespoke surfaces | one head, N projections |

## Interfaces that guide implementation

Rust signatures in the epr-cli crate (`elohim/eprfs/epr-cli/src/flow/memory/`), written so a
seat can implement against them without the prose. Names are proposals; the shapes are the
contract.

```rust
/// The algorithm. Loaded from an EPRFS artifact; its raw CID is the method pinned on every receipt.
pub struct Recipe { pub id: String, pub version: u32, pub method: Cid,
    pub stages: Vec<Stage>, pub limits: Budget, pub measures: Vec<MeasureRef>,
    pub source_roots: Vec<PathBuf>, pub providers: Vec<ProviderDecl>,
    pub defaults: RecipeDefaults, pub change_authority: String }

/// Discovery over one scope, one need, one budget, rendered for one lens.
pub trait Discover {
    fn discover(&self, scope: &Scope, need: &Terms, budget: &Budget, lens: &Lens) -> Window;
}
pub struct Window { pub candidates: Vec<Candidate>, pub omissions: Vec<Omission>,
    pub frontier: Vec<Frontier>, pub usage: Usage, pub selection_rule: String }
pub struct Candidate { pub source: SourceRef, pub matched: Vec<MatchKind>,
    pub located: Option<Located>, pub receipt: ReceiptKey, pub provider: ProviderId }
pub enum MatchKind { Title, Description, Body, Habit, Provider(ProviderId) }
pub struct Located { pub heading: String, pub lines: (usize, usize), pub hits: Vec<(Term, u32)> }

/// A provider is any candidate source the recipe declares: local lexical traversal (today),
/// MemPalace (optional, opaque ranking declared opaque), ContentGraphResolver (native), a peer.
pub trait Provider {
    fn id(&self) -> ProviderId;
    fn candidates(&self, query: &Terms, budget: &Budget) -> ProviderResult;
}
pub struct ProviderResult { pub ranked: Vec<Candidate>, pub ranking_known: bool,
    pub version: Option<String>, pub usage: Usage }

/// Who is reading. Agents come from the actor sidecar; humans from imagodei.
pub enum ReaderRef { Agent { role: String, model: String, session: String },
                     Human { agent_cid: String, tier: CapabilityTier } }

/// The lens: density and scaffold, never honesty.
pub struct Lens { pub level: LensLevel,            // minimal | simple | standard | detail | debug | trace
    pub choice_count: u8, pub density_bytes: usize, pub stimulus: Stimulus, pub textuality: Textuality,
    pub scaffold: Scaffold }                      // locate-and-hand-one-command | rank-and-let-choose
pub struct LensPreset { pub lens: Lens, pub reader: ReaderRef, pub recipe: Cid,
    pub provenance: LensProvenance, pub reviewed_at: Option<Timestamp>, pub cid: Cid }
pub struct LensProvenance { pub stated: Vec<Evidence>, pub revealed: Vec<Evidence>, pub defaults: Cid }
pub trait ResolveLens {
    fn resolve(&self, reader: &ReaderRef, recipe: &Recipe, requested: Option<LensLevel>) -> LensPreset;
}
/// The honesty floor is NOT a lens field. Every projection prints: recipe CID, lens preset CID,
/// selection rule, omissions, receipts. A lens may collapse them to one line; it may not drop them.

pub trait Render { fn render(&self, view: &View, lens: &Lens) -> String; }

/// One measured journey.
pub struct Sample { pub reader: ReaderRef, pub question: Cid, pub recipe: Cid, pub lens: Cid,
    pub screens_to_shape: u32, pub operations: u32, pub metered_bytes: u64, pub unmetered_bytes: u64,
    pub reached: bool, pub mistaken_assertions: Option<u32> /* set only by a second seat */ }
pub trait Sampler {
    fn run(&self, question: &Question, reader: &ReaderRef, recipe: &Recipe) -> Sample;
    fn judge(&self, sample: &Sample, seat: &ReaderRef) -> Sample;     // fills mistaken_assertions
    fn fold(&self, sample: &Sample) -> Vec<ObservationCid>;            // recall-journey middot
}
```

Rules the interfaces encode:

1. **Every view carries its method.** Recipe CID and lens preset CID are printed on every
   projection at every lens. Two readers can compare journeys by CID; that is the defence against
   epistemic bifurcation, and it costs one line.
2. **A lens narrows the default, never the reach.** Any reader may ask for a wider lens on the
   same session (`--lens detail`); the preset decides what is shown first, not what may be seen.
   Withdrawal of scaffold is the goal, so a preset carries `reviewed_at` and a review cadence.
3. **Honesty is constant across lenses.** Selection rule, omissions, receipts and frontier are
   floor-protected view fields; `minimal` renders them as one line, never as nothing.
4. **Providers are declared, and opaque ones say so.** A provider whose ranking cannot be
   explained sets `ranking_known: false`, and the view says so beside its candidates.
5. **A sample is judged by a second seat.** `mistaken_assertions` is `None` until a seat other
   than the reader sets it. A reader never grades itself.

## The three follow-ons as instances

**Standing reader (the measured quarter).** A `QuestionBank` artifact beside the recipe
(`.epr-meta/elohim/algorithms/recall-questions.json`): five to eight concrete repository
questions in the efficacy analysis's shape, each with its authoritative source and the assertion
that counts as reached. `epr flow memory recall sample --question <id> --reader <ref>` runs
`Sampler::run` with a context-reset reader and folds the middot; `… judge --sample <cid> --as
<seat>` fills mistaken assertions. A weekly routine runs the bank across the reader tiers the
actor sidecar has seen. The habit's third check reads a rolling window of folds, not the latest
one, so a green is a rate. Nothing new is stored: samples are folds and receipts already are.

**Bootstrapping head (the ceremony-door pass).** The recall `open` is the head. The SessionStart
headline becomes `Render(open(recipe, need=<session's top red>, scope='.'), lens=minimal)`; the
run-plane line becomes the same view at `simple`; the ceremony's grouped stale-edge view is
`open` at `detail` with no need. Three surfaces, one derivation, and the budget rows those
surfaces already declare become the `minimal` lens's `density_bytes`. The pass ends by retiring
the two bespoke renderers and declaring the head in `.claude/hooks/.epr-meta` as a rule.

**Reader lens (a preset per reader).** `ResolveLens` composes three inputs, in this order,
each with provenance: recipe defaults (declared, CID), stated (agent: the actor claim's
`role@model`, mapped by a declared table to a starting `LensLevel`; human: the imagodei
capability profile — `CapabilityTier`, `@capabilityMaxLens`, stimulus, textuality — the same
vocabulary graphos already renders), revealed (agent: this reader's own journey folds — a tier
that reaches authority at `simple` is offered `standard`; human: Sophia `Recognition` records —
mastery, resonance, reflection — as the evidence a wider lens is within reach). The resolved
preset is printed on the first screen (`lens: simple (stated: sonnet → simple; revealed: 3/3
reached at simple; recipe default standard)`) so it is contestable on sight. Model presets are
static per model per domain and change through the recipe's change authority. Human presets
are the reader's own private record, negotiated with imagodei and sophia, reviewed on a cadence
the reader sees, and revealed on every journey.

## P2P Design Gate

### Entity: Recipe (the algorithm artifact)
- **Classification**: Notarized (A) at graduation; Ephemeral (C) today as an EPRFS file whose CID is already the method pin.
- **Justification**: the protocol would be lying if the algorithm that ranked a reader's world changed silently; it is a thing in its own right, with authorship and change authority.
- **Head-Plane Cost Budget**: tens of recipes at seed, low hundreds at one year; well under the ~500 line. No bundling needed; each recipe is one head.
- **Network Stakes**: all four stages. Change authority is floor-protected (Constitutional); verification of a recipe's bytes is stage-priceable.
- **Content Address Strategy**: Content-Derived (CID, dag-cbor). A consumer names the recipe head it runs under; `latest` is never the answer.
- **Source of Truth**: Holochain DHT at graduation; EPRFS file until then.
- **Integrity Zome + DNA-hash class**: reuse the existing EPR content entry on the elohim DNA (`content_store_integrity`) with `contentType: algorithm` — DNA-hash-NEUTRAL; read `#[hdk_entry_types]` before implementation to confirm no new type is minted.
- **Coordinator Zome**: existing content-create path -> EntryHash.
- **Projections**: SQLite content projection (dht_anchor_hash: yes); Automerge sync: yes at reach tier `unresolved — reach vocabulary in declared drift`.
- **HTTP Route**: none new; served as content through the existing content routes declared in elohim-storage `build_manifest()`; `{id}` is the EntryHash/CID.
- **SDO/RWA Test**: the worst holder sees which recipes exist and their bytes — that is the point (inspectable). Boundary 4: a reader's counter-evidence against a ranking always reaches the recipe steward.
- **Anti-Pattern Check**: no UUID, no route-first, no second address format (CID everywhere).

### Entity: QuestionBank
- **Classification**: Linked (A2) — an attribute of a recipe.
- **Justification**: questions have no meaning without the recipe they measure.
- **Head-Plane Cost Budget**: none (link on the recipe).
- **Network Stakes**: all four; stage-priceable.
- **Content Address Strategy**: CID of the bank's bytes, linked from the recipe entry.
- **Source of Truth**: Holochain Link at graduation; EPRFS file until then.
- **Integrity Zome + DNA-hash class**: existing link type on `content_store_integrity` — DNA-hash-NEUTRAL.
- **Coordinator Zome**: existing create_link -> ActionHash.
- **Projections**: SQLite denormalized beside the recipe (dht_anchor_hash: parent); Automerge: n-a (Linked).
- **HTTP Route**: none.
- **SDO/RWA Test**: public by design; questions must not encode a participant's identity.
- **Anti-Pattern Check**: none.

### Entity: LensPreset
- **Classification**: Private (B) for the reader's own preset; Attested-Private (B2) when a guardian, steward or collective co-authors it (child, ward, hosted human), where the attestation is the review record, not the preset.
- **Justification**: how a person is shown their world is theirs; only the fact that it was reviewed by the party responsible for them needs peer verification.
- **Head-Plane Cost Budget**: attestations only for B2; expected low hundreds; no bundling.
- **Network Stakes**: all four; the B2 attestation is floor-protected (LocalRelationship).
- **Content Address Strategy**: Agent-Scoped Composite (`agent_cid`, recipe CID, `lens-preset`); the preset's own bytes have a CID for printing on views.
- **Source of Truth**: private source chain (B); attestation on the elohim DNA (B2).
- **Integrity Zome + DNA-hash class**: private entry, no integrity change; B2 rides the existing attestation content type — DNA-hash-NEUTRAL.
- **Coordinator Zome**: private create -> EntryHash; attestation via the existing content path.
- **Projections**: SQLite agent-scoped only; Automerge: no.
- **HTTP Route**: none exposed to other agents.
- **SDO/RWA Test**: the worst holder of presets could profile every reader's capability and intent — boundary 2 and 6: presets are held by the reader (or the reader's holon for B2), never aggregated across holons, never joined to journey samples by an outside observer.
- **Anti-Pattern Check**: the framing guard applies — the widest lens is `trace`, a density level, never an autonomy apex; a guardian-co-authored preset is mediated agency, not incapacity.

### Entity: Sample (one measured journey)
- **Classification**: Private (B) raw; Attested-Private (B2) outcome — `reached`, `mistaken_assertions`, byte counts — as a middot observation fold, which is the attestation shape the substrate already has.
- **Justification**: a journey is activity; only its outcome is the standing measure.
- **Head-Plane Cost Budget**: folds are local records today; at graduation the attested outcome is one observation per sample, thousands per year — bundled as a corpus digest per (recipe, reader tier, window) before it reaches a notarized plane.
- **Network Stakes**: all four; the attested outcome is stage-priceable; counter-evidence (a reader disputing a judge) is floor-protected.
- **Content Address Strategy**: CID of the observation; keyed by reader, question, recipe, lens as env.
- **Source of Truth**: private source chain (raw); DHT attestation (digest) at graduation; flows sidecar until then.
- **Integrity Zome + DNA-hash class**: existing attestation content type — DNA-hash-NEUTRAL.
- **Coordinator Zome**: existing content path -> EntryHash.
- **Projections**: SQLite folds (dht_anchor_hash: yes for the digest); Automerge: no.
- **HTTP Route**: none.
- **SDO/RWA Test**: journey samples are the dragnet's favourite food (who asked what, when, how well). Boundary 1 and 2: raw journeys never enter a notarized plane and are held by the reader's holon; only tier-level digests cross. Boundary 5: the reader in a sample is never string-joined to a transport or doorway identity.
- **Anti-Pattern Check**: no per-item heads (digest bundling declared).

### Entity: Projection (headline, run-plane, first screen, doorway page)
- **Classification**: Ephemeral (C). Rebuilt from a view and a lens; documented as derived; reconstruction is `Render(open(...), lens)`.

### Design constraints discovered
- The lens vocabulary already exists at the client seam (graphos `Lens`, `CapabilityTier`, the `@capability*` element contract); this design consumes it and must not mint a parallel enum. The CEM plugin's cited spec path is stale (correction recorded 2026-09-11).
- `recall.rs` is 5,073 lines; the interfaces above are its module boundaries. The split is station zero.
- Reach vocabulary is in declared drift; every graduation row above says so rather than naming a tier.
- The actor sidecar's `role@model` strings are an uncontrolled vocabulary (9 spellings over ~4 families measured 2026-09-10); the stated-lens table needs the controlled vocabulary the provenance plan already names.

## Concern-canon answers for the two decision predicates

`ResolveLens` and the candidate ranking are decision predicates and answer the canon at birth
(registration row: `elohim/eprfs/epr-cli/seam-registry.yaml`, create if absent).

| Class | ResolveLens | Ranking |
|---|---|---|
| C0 plane location | answered — client/SDK seam, local; recipe governs | answered — recipe stage `discover` |
| C1 anti-self-election | answered — a reader cannot widen its own stated tier; only revealed evidence or review does | n-a |
| C2 monotonic authority | answered — preset changes are new CIDs with provenance; no in-place mutation | answered — recipe versions by CID |
| C3 liveness | n-a | answered — budgets bound traversal; partial windows named |
| C4 honest absence | answered — a missing profile yields recipe defaults labelled `stated: none` | answered — unreadable/partial candidates counted and named |
| C5 evidence-not-authority | answered — revealed evidence widens an offer, never a standing | answered — "candidate, not authority" on every window |
| C6a bounded work | answered — resolution reads declared tables and the reader's own folds only | answered — declared budgets |
| C6b idempotent effect | answered — same inputs, same preset CID | answered — same inputs, same window |
| C7 advertise/serve symmetry | answered — the printed preset is the preset applied | answered — printed selection rule is the one run |
| C8 observability | answered — preset CID and provenance on every view | answered — recipe CID, hits, omissions on every view |
| C9 identity-lineage | partial — agent readers keyed by `role@model` until the controlled vocabulary lands | n-a |
| C10 contract evolution | answered — recipe version + CID; sessions `adopt` across method changes | same |
| C11 backpressure | n-a | answered — budgets are the declared backpressure |
| C12 consent | answered — a human preset is the reader's private record; guardian co-authorship attested | answered — a wider lens is always available on request |
| C13 graduated authority | answered — scaffold withdraws as revealed evidence accrues; `reviewed_at` cadence | answered — provider standing declared per recipe |
| C14 witnessed residual | answered — every view carries a frontier | answered — unread/partial candidates in the frontier |

## Graduation ladder (local-first → native search)

| Rung | What runs | Recipe | Lens | Sample | Providers | Reach |
|---|---|---|---|---|---|---|
| L0 eprfs local (today) | `epr flow memory recall` over one checkout | EPRFS file, CID pinned | none | folds on a plan | local lexical; optional MemPalace | this tree |
| L1 collective | same, readers registered in the actor sidecar | contributed to the collective (`epr flow memory contribute`) | agent presets from `role@model`; human presets private | folds keyed by reader; weekly bank | + `ContentGraphResolver` through `Provider` | repository (graduation is a reach rehearsal) |
| L2 native peer | the same stages executed by elohim-storage over its content projection | EPR content entry (A), served as content | private entry (B) / attested review (B2) | private raw, attested digest | + peer inventory as a provider with `ranking_known` | holon |
| L3 network | a household searches across the holons it participates in | published at a reach; steward-changed | negotiated with imagodei and sophia; revealed on every page | digests per tier per holon | + other holons' recipes, each named | earned reach |

What does not change across rungs: the four interfaces, the honesty floor, the CIDs on every
view, the second-seat judgment, the per-holon custody of journeys. What changes: the substrate
that stores the artifacts, the providers the recipe may declare, and the reach at which a
recipe or preset is visible. That invariance is the design's test: a station that would need a
new interface at L2 is a station that got the L0 interface wrong.

## MemPalace, read through the protocol (a consideration, not a station)

MemPalace is an appendage today: a separate index with its own freshness marker, its own
directory of wings, rooms and drawers, its own tunnels and diary, mined by a wrapper the kit
used to own. Under this design it is one declared `Provider` with `ranking_known: false`, and
nothing more is owed to it. But its concerns are real, and each has a protocol-native home the
long run will prefer:

| MemPalace concern | What it is in protocol terms | Home |
|---|---|---|
| drawers (mined chunks) | derived projections of contributions and content that already have CIDs | Ephemeral (C), rebuilt from the collective and the content projection, per holon |
| wings / rooms | the placement the epr-meta cascade already declares | the `.epr-meta` tree and, at L2, reach-scoped content sets |
| tunnels (knowledge graph) | authored links vs computed neighbours | authored: Linked (A2) via cites and links; computed: `ContentGraphResolver` (C, never stored) |
| `.last-mine` freshness | a stamp asserting currency | derived from content heads instead: an index is current when its input CIDs equal the heads it was built from |
| wake-up context (L0/L1) | the first screen at `minimal` | `Render(open(...), minimal)` |
| AAAK compression | a lens over a drawer | a `Lens` density level, not a storage format |
| diary | private thought | never imported, never a provider input (boundary 1) |
| semantic search | a provider whose embedding model and version are part of its method | a native provider with a declared model CID so `ranking_known` can become true |

The graduation rule for this row is the same as for the rest of the ladder: when a native
provider answers the same `Provider` interface with a declared model and an index that is a
rebuildable projection of notarized content per holon, MemPalace retires from the recipe's
provider list by a recipe version change, and nothing else has to move.

## Anti-capture invariants (the reason the recipe is governable)

- **Audience capture**: a preset never drifts without a review event with provenance; the reader sees the cadence and the last review.
- **Filter bubble**: what the lens hid is one command away, on the same session, for every reader; omissions are printed at every lens.
- **Rabbit hole / radicalization pipeline**: budgets bound every traversal; `repeated_reads` and the frontier make circling visible; a provider that cannot explain its ranking is labelled so on the page.
- **Epistemic bifurcation**: recipe CID and lens CID on every view; two readers, one comparison. Counter-evidence to a ranking is floor-protected and reaches the recipe's steward.

## Stations (for the implementation plan)

0. Split `recall.rs` into `journey`, `discovery`, `providers`, `lens`, `render`, `refusal`, `measure` behind the interfaces above, outputs byte-identical (pinned by the existing 51 tests plus a golden of the round-5 first screen).
1. `ReaderRef` + `ResolveLens` from the actor sidecar and a declared `role@model → LensLevel` table in the recipe; `Render` per lens; preset printed on every view; `--lens` widens on request.
2. Bootstrapping head: headline and run-plane become projections of `open` at `minimal`/`simple`; the two bespoke renderers retire; rule declared in `.claude/hooks/.epr-meta`.
3. Standing reader: question bank artifact, `sample` and `judge` verbs, weekly routine, habit check over a rolling window.
4. Human lens: imagodei capability profile as stated, Sophia `Recognition` as revealed, preset as a private entry with a graphos review-and-reveal element; design of the B2 co-authorship attestation.
5. Graduation L1→L2: recipe contributed and graduated; `ContentGraphResolver` behind `Provider`; the same a2o scenarios run against a household peer.

Each station ends with the rule it proved written where the concern lives (a `.epr-meta` rule or a habit check), and a fresh-reader sample folded on the recipe it changed.
