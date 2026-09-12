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
  - "social-reach-nervous-system | the receive-side contract this design inherits: reach floor, provenance, sense/respond, the five constraints of a legitimate user-side filter, the anti-bubble policy | sha256:d85af6961ce566c6 | path: genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md"
  - genesis/docs/content/elohim-protocol/social_medium/epic.md
  - genesis/docs/content/elohim-protocol/living_memory/epic.md
  - "search-epic | the native-search epic this ladder graduates toward: found-not-crawled, provider as a declared method, ranking as a governable artifact | sha256:7ca54d18d954aae4 | path: genesis/docs/content/elohim-protocol/search/epic.md"
  - genesis/research/search-discovery-incumbent-power-and-p2p-inversion-2026-09-11.md
  - "bounded-recall-mastery-sprint | the sprint whose five rounds and three fresh readers proved the journey this design generalizes | sha256:d4c6f66d1685d124 | path: genesis/docs/superpowers/plans/2026-09-11-bounded-recall-mastery-sprint.md"
  - "unified-memory-loop-design | the collective-memory owner split and the graduation-as-reach-rehearsal rule this ladder extends | sha256:07e941a325cc49c2 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md"
  - "private-thought-governed-fruit | the SDO/RWA boundaries every entity row answers — journeys are held by the holon, only outcomes cross | sha256:5b6f5cdb858277e4 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "cradle-to-grave-capability-gradient | the life-stage gradient a human lens preset must hold for — mediated agency, never an autonomy apex | sha256:1a5b2f7e6433230f | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md"
  - "elohim-seam-map-concern-routing | places the lens at the client/SDK seam and the recipe at the SDK seam; providers are bridges | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "ceremony-efficacy | the five fresh-start questions the question bank instantiates | sha256:23eb372f4ba66135 | path: genesis/docs/analysis/2026-09-09-memory-ceremony-efficacy.md"
---

# Governed discovery

One pattern, four roles, one graduation ladder — built from primitives the substrate already
has. The pattern is the bounded-recall journey the 2026-09-11 sprint proved on one question;
this design makes it composable so that the three follow-ons — the standing reader (a measured
quarter), the bootstrapping head (converge three surfaces into one), and the reader lens (a
preset per reader in its zone of proximal development) — are the same roles applied three
times, and so that the same roles carry unchanged when the reader is a human discovering
meaning in a community on the native network.

## Heart-level concerns (read before the interfaces)

Search and discovery are a receive-side surface. The social-reach epic and its nervous-system
architecture already govern receive-side surfaces, and this design inherits their commitments
rather than restating them in tooling vocabulary:

1. **Reach earned at authoring is the floor; discovery never becomes a central filter.** A
   recipe ranks what has already earned reach into the reader's scope. It runs at the edge, per
   reader, per holon. There is no central index that ranks the network; any provider that would
   be one is declared opaque and stays a candidate source, never authority.
2. **A lens is the legitimate user-side filter, and only if it meets all five constraints.**
   Values-forward (the reader's expressed preference, never network policy); time-limited (a
   preset expires and must be renewed); tended (periodic review, so values can shift); bounded by
   the anti-bubble policy (some content classes may not be filtered away — corrections, counter-
   evidence, accountability information and facts about the reader's own community always
   surface at every lens); and feeding collective wisdom (a preset that narrows the same content
   for many readers is a signal into the nervous system, not a private convenience). A lens
   that is set-and-forget, network-imposed, or able to hide broccoli is the email-collapse
   anti-pattern wearing a nicer name.
3. **Provenance travels with every candidate.** A result says who authored it, who endorsed or
   relayed it, and why it reached this reader (which recipe, which provider, which hits). Sense
   and respond runs backwards along the same chain: a reader who was misled by a ranking files a
   feedback signal against the projection, and enough of them quarantines the recipe version.
4. **Standing is a shape, not a score.** Provider rankings must never become a standing tally.
   The epistemic inputs a native provider may use — affirmations, dismissals, citations on a CID —
   are witnessed interactions aggregated on the object, substrate-denominated, and they are
   printed beside the result, never summed into a number about a person.
5. **Carrot before stick: the lens negotiation is the tender conversation.** For a human the
   elohim sits beside them at the moment of reading: "your lens hid three corrections about your
   neighbourhood this week — widen, keep, or renew?" It offers context; it never lectures, blocks
   or shames; the decision and its provenance stay with the reader.
6. **Local first, rings outward.** On the network a recipe's `scope` stage is a reach ring —
   Private, SelfScope, Intimate, Trusted, Familiar, Community, Public, Commons — walked outward
   from the reader, each ring priced by the reader's standing in it. Locally the same stage is a
   directory scope. That is the whole translation from L0 to L3.
7. **Memory has a comet shape; consolidation is judgment.** Recency is not authority: a recipe
   ranks by hits and provenance, and prefers canon (re-validated across seasons) over the bright
   head. When a second seat judges a journey, or a reader recognises two candidates as the same
   thing, that recognition is labour that carries flow, recorded as an event, never as janitorial
   cleanup.
8. **The whole human life.** A child's lens is mediated agency co-authored by a guardian, with the
   positive duty to surface age-appropriate opportunity, not only to block. An elder's protection
   from scams is the reach floor (scammers lack the web of trust), never a narrower lens. Neither
   is an autonomy tier.
9. **Constitutional revealability.** A reader's journeys are sealed against the self: private by
   default, recoverable only through governance with the reader's own elohim as counsel. Only
   attested outcomes and tier digests cross into any shared plane.

## The shape

```
                ┌──────────── ProcessSpec (the recipe; Manifest EPR, CID = method) ──────────┐
  Intent ─────▶ │ scope → discover → filter → group → select → read → judge                   │ ─▶ ProjectionRequest → view
  (need/scope)  │   Bounds (mishpat)   measures (middot)   providers (declared per stage)     │
                └──────────────────────────────────────────────────────────────────────────────┘
                        ▲                                          │
                 AttentionTending (the lens)                  FlowEvent + Verdict (the sample)
                 stated ⊕ revealed ⊕ recipe defaults,         Consume event fulfilling the Intent;
                 TTL-bound, tended, revealed on every view    second-seat Verdict; folds as Observations
```

Four roles. Every one is an existing kind; nothing in this design mints a struct.

| Role | Existing primitive (crate) | What the role adds | Graduates to |
|---|---|---|---|
| **Recipe** | `ProcessSpec {id, version, stages: Vec<StageSpec>, edges}` (`elohim-epr-rea` model) carried as a `Manifest` EPR whose CID is the method pin; budgets as `Bound {limit, unit, threshold_pct, sense: Ceiling, source: Declared \| Folded}`; providers as `StageSpec.artifact_kind` per stage | today's `recall-contract.json` becomes the ProcessSpec's declared form; `composition: [scope, discover, filter, group, select, read, judge]` IS `stages` | contributed (`Contribution`) and graduated (`Graduation {audience: Reach}`) to the collective, then published at a reach ring |
| **Need** | `Intent {action: ReaVerb::Consume, resource_spec, in_scope_of: <recipe process>, raised_by: AgentRef}` | the `--need` text is the resource spec's description; the question bank is a set of Intents in scope of the recipe | same kind; raised at a reach ring |
| **Lens** | `AttentionTending {filter_subject, classification: ValuesForward \| Fatigue \| ScopeMismatch \| Safety, ttl_seconds, tended_at, context}` (`elohim-epr` kind; private source chain, never gossiped) | a lens preset is an attention-tending record whose `filter_subject` names the recipe and the density parameters; its `ttl` and `tended_at` are the "time-limited, tended" constraints; the anti-bubble floor is applied at render | same kind; guardian co-authorship as an `Attestation` (B2) |
| **Sample** | `FlowEvent {action: Consume, provider: <reader>, receiver: <recipe steward>, resource: <intent cid>, quantity: Magnitude::Count{bytes}, process, fulfills: [intent], occurred_at}` + `Verdict {axis: "recall-journey", decision, witness: Witness{checks}}` from the second seat + `Observation` folds on the recall-journey measures | `reached` is `fulfills` being nonempty; `mistaken_assertions` is a `CheckWitness` in the seat's Verdict, never self-reported; `screens_to_shape` and byte counts are the Observations already declared | raw event private; the Verdict and a per-tier digest cross as Attestations |
| **View** | `ProjectionRequest {purpose, audience: Reach, inputs, omissions}` (`eprfs-agent` memory) → an ephemeral rendering; feedback on it is `Feedback {target, kind: PoorSelection \| MisleadingProjection \| OmittedContradiction \| StaleSource, passage, reason}` | the first screen, the SessionStart headline, the run-plane line and a doorway page are one request rendered at four lenses; a reader's dispute is a `Feedback`, and on the network a `FeedbackSignal {target_cid, signal_kind: Squelch \| Correction \| Retraction \| Quarantine}` against the recipe version | same kinds |
| **Standing of a recipe** | `EpistemicStanding {subject, affirmations, dismissals, citations}` and `EpistemicStatus: Emergent → Reviewed → Contested → Canon → Superseded` (`elohim-epr-rea` epistemic) | a recipe version's standing is the shape of the feedback on its projections; canonization is by review, never by usage count | same kinds, witnessed |

Two reach vocabularies exist today: `elohim_epr::Reach` (eight rings) and the collective's
`Reach {Private, Workspace, Repository}`. This design does not mint a third; it maps
Workspace → SelfScope/Intimate and Repository → Community for the L0/L1 rungs and names the
convergence as a frontier for the reach-vocabulary drift already declared.

## Interfaces that guide implementation

The roles above are data. The executor needs five seams, each a trait over those kinds, so
that a station can be implemented and reviewed against a signature rather than the prose.

```rust
use elohim_epr::{reach::Reach, verdict::{Verdict, Witness, CheckWitness}, witness::Magnitude};
use elohim_epr_rea::{Intent, ProcessSpec, StageSpec, Bound, FlowEvent, AgentRef, EpistemicStanding};
use eprfs_agent::memory::{ProjectionRequest, Feedback, FeedbackKind};

/// Stage 2 of the ProcessSpec. Reads at most `Bound`s worth of bytes; returns candidates with provenance.
pub trait Discover {
    fn discover(&self, spec: &ProcessSpec, intent: &Intent, scope: &Scope, lens: &LensView) -> Window;
}
pub enum Scope { Directory(PathBuf), Ring(Reach) }          // L0 vs L2+: the only translation
pub struct Window { pub candidates: Vec<Candidate>, pub omissions: Vec<String>,
    pub frontier: Vec<String>, pub usage: Usage, pub selection_rule: String }
pub struct Candidate { pub object: Cid, pub source: SourceRef, pub provider: ProviderId,
    pub matched: Vec<MatchKind>, pub located: Option<Located>,
    pub provenance: Provenance,               // authored / endorsed / relayed-by — the chain the reader may follow back
    pub standing: Option<EpistemicStanding>,  // printed beside the result, never summed into a score
    pub receipt: ReceiptKey }
pub enum MatchKind { Title, Description, Body, Habit, Provider(ProviderId) }
pub struct Located { pub heading: String, pub lines: (usize, usize), pub hits: Vec<(String, u32)> }

/// A provider is any candidate source a stage declares. Opaque ranking is declared, not hidden.
pub trait Provider {
    fn id(&self) -> ProviderId;
    fn candidates(&self, intent: &Intent, scope: &Scope, budget: &[Bound]) -> ProviderResult;
}
pub struct ProviderResult { pub ranked: Vec<Candidate>, pub ranking_known: bool,
    pub method: Option<Cid>, pub usage: Usage }

/// Who is reading. Agents from the actor sidecar; humans from imagodei.
pub enum ReaderRef { Agent(AgentRef), Human { agent_cid: String, tier: CapabilityTier } }

/// The lens as applied to one view. Derived from an AttentionTending record plus the recipe defaults.
pub struct LensView { pub level: LensLevel,     // graphos: minimal | simple | standard | detail | debug | trace
    pub choice_count: u8, pub density: Bound, pub scaffold: Scaffold,
    pub tending: Cid, pub expires_at: DateTime<Utc>, pub tended_at: Vec<DateTime<Utc>>,
    pub provenance: LensProvenance }
pub enum Scaffold { LocateAndHandOneCommand, RankAndLetChoose }
pub struct LensProvenance { pub stated: Vec<Cid>, pub revealed: Vec<Cid>, pub defaults: Cid }
pub trait ResolveLens {
    fn resolve(&self, reader: &ReaderRef, spec: &ProcessSpec, requested: Option<LensLevel>) -> LensView;
}
/// Floors the lens may not touch. Content-class floor from the anti-bubble policy; honesty floor
/// from the view contract. Both are checked at render, never configurable.
pub struct RenderFloor { pub unfilterable: Vec<ContentClass>,   // corrections, counter-evidence, accountability, own-community facts
    pub always_printed: [&'static str; 5] }                    // recipe cid, lens cid, selection rule, omissions, receipts
pub trait Render { fn render(&self, request: &ProjectionRequest, window: &Window, lens: &LensView, floor: &RenderFloor) -> String; }

/// One measured journey.
pub trait Sampler {
    fn run(&self, intent: &Intent, reader: &ReaderRef, spec: &ProcessSpec) -> FlowEvent;   // Consume; fulfills = [intent] iff reached
    fn judge(&self, event: &FlowEvent, seat: &ReaderRef) -> Verdict;                        // CheckWitness "mistaken-assertions" etc.
    fn fold(&self, event: &FlowEvent, verdict: &Verdict) -> Vec<Cid>;                       // Observations on the recall-journey measures
}
/// A reader's dispute of a projection. Local: Feedback. Network: FeedbackSignal against the recipe version.
pub trait Dispute { fn file(&self, target: &ProjectionRequest, kind: FeedbackKind, passage: &str, reason: &str) -> Cid; }
```

Rules the seams encode:

1. **Every view carries its method.** Recipe CID and lens CID on every projection at every
   lens. Two readers, one comparison. This is the defence against epistemic bifurcation.
2. **A lens narrows the default, never the reach or the floor.** Any reader may ask for a wider
   lens on the same session; the unfilterable content classes and the five always-printed
   fields render at `minimal` as one line, never as nothing.
3. **A lens expires and is tended.** `expires_at` and `tended_at` come from the
   AttentionTending record; a lapsed lens falls back to recipe defaults and says so.
4. **Providers are declared, and opaque ones say so.** `ranking_known: false` is printed beside
   the candidates it produced.
5. **A sample is judged by a second seat.** The Verdict's witness is authored by a seat other
   than the reader; a reader never grades itself.
6. **Provenance is printable, and the chain runs both ways.** A candidate's provenance is a
   traversal, and a `Feedback` against a projection travels back along it.

## The three follow-ons as instances

**Standing reader (the measured quarter).** A question bank is a set of `Intent`s in scope of
the recipe process (an EPRFS artifact beside the contract: `.epr-meta/elohim/algorithms/recall-
questions.json`), five to eight concrete repository questions in the efficacy analysis's shape,
each naming its authoritative source and the assertion that counts as reached. `epr flow memory
recall sample --intent <id> --reader <ref>` runs `Sampler::run` with a context-reset reader;
`… judge --event <cid> --as <seat>` produces the Verdict; both fold. A weekly routine runs the
bank across the reader tiers the actor sidecar has seen. The habit's third check reads a rolling
window of folds so a green is a rate. Judging is labour that carries flow: the seat's Verdict is
a `FlowEvent` too, so consolidation-as-judgment holds here as it does in living memory.

**Bootstrapping head (the ceremony-door pass).** `open` is the head. The SessionStart headline
is `Render(ProjectionRequest{purpose: bootstrap, audience: Private, inputs: [top red, WIP
fence, scope line]}, window, lens=minimal)`; the run-plane line is the same request at `simple`;
the ceremony's grouped stale-edge view is `open` at `detail` with no intent. Three surfaces, one
derivation. The pass ends by retiring the two bespoke renderers and declaring the head as a rule
in `.claude/hooks/.epr-meta`.

**Reader lens (a preset per reader).** `ResolveLens` composes three inputs, in order, each with
provenance: recipe defaults (declared, CID); stated (agent: the `ActorClaim`'s `role@model`
mapped by a declared table in the recipe to a starting `LensLevel`; human: the imagodei
capability profile — `CapabilityTier`, `@capabilityMaxLens`, stimulus, textuality — the same
vocabulary graphos renders); revealed (agent: this reader's own journey folds — a tier that
reaches authority at `simple` is offered `standard`; human: Sophia `Recognition` records —
mastery, resonance, reflection — as the evidence a wider lens is within reach). The resolved
lens is printed on the first screen (`lens: simple · stated sonnet→simple · revealed 3/3
reached at simple · default standard · renew by 2026-12-11`), so it is contestable on sight.
Model presets are static per model per domain and change through the recipe's change
authority. A human's preset is their own AttentionTending record: negotiated with imagodei
and sophia, TTL-bound, tended on a cadence they see, revealed on every journey, and for a child
or ward co-authored by the guardian with an attestation of that review.

## P2P Design Gate

### Entity: Recipe (ProcessSpec as a Manifest EPR)
- **Classification**: Notarized (A) at graduation; today an EPRFS file whose raw CID is already the method pin.
- **Justification**: the protocol would be lying if the algorithm that ranked a reader's world changed silently; it is a thing in its own right with authorship and change authority.
- **Head-Plane Cost Budget**: tens at seed, low hundreds at one year; under the ~500 line; one head each.
- **Network Stakes**: all four stages. Change authority is floor-protected (Constitutional); byte verification is stage-priceable.
- **Content Address Strategy**: Content-Derived (CID, dag-cbor). A journey names the recipe head it runs under; `latest` is never the answer.
- **Source of Truth**: Holochain DHT at graduation; EPRFS file until then.
- **Integrity Zome + DNA-hash class**: the existing `Manifest` EPR kind on the elohim DNA (`content_store_integrity`) — DNA-hash-NEUTRAL; confirm against `#[hdk_entry_types]` before implementation.
- **Coordinator Zome**: existing content-create path -> EntryHash.
- **Projections**: SQLite content projection (dht_anchor_hash: yes); Automerge sync: yes at reach tier `unresolved — reach vocabulary in declared drift`.
- **HTTP Route**: none new; served through the existing content routes in elohim-storage `build_manifest()`; `{id}` is the EntryHash/CID.
- **SDO/RWA Test**: the worst holder sees every recipe's bytes — the point (inspectable). Boundary 4: counter-evidence against a ranking (a `FeedbackSignal` of kind Correction) is floor-protected and reaches the recipe's steward.
- **Anti-Pattern Check**: no UUID, no route-first, one address form.

### Entity: QuestionBank (Intents in scope of the recipe)
- **Classification**: Linked (A2) — attributes of the recipe process.
- **Justification**: a question has no meaning without the recipe it measures.
- **Head-Plane Cost Budget**: none (links on the recipe).
- **Network Stakes**: all four; stage-priceable.
- **Content Address Strategy**: CID of each Intent, linked from the recipe entry.
- **Source of Truth**: Holochain Link at graduation; EPRFS file until then.
- **Integrity Zome + DNA-hash class**: existing link type on `content_store_integrity` — DNA-hash-NEUTRAL.
- **Coordinator Zome**: existing create_link -> ActionHash.
- **Projections**: SQLite beside the recipe (dht_anchor_hash: parent); Automerge: n-a.
- **HTTP Route**: none.
- **SDO/RWA Test**: public by design; an Intent must never encode a participant's identity.
- **Anti-Pattern Check**: none.

### Entity: Lens preset (an AttentionTending record)
- **Classification**: Private (B); Attested-Private (B2) when a guardian, steward or collective co-authors it, where the `Attestation` records the review, not the preset.
- **Justification**: how a person is shown their world is theirs; only the fact of responsible review needs peer verification. `AttentionTending` is already declared private, never gossiped.
- **Head-Plane Cost Budget**: B2 attestations only; low hundreds; no bundling.
- **Network Stakes**: all four; the B2 attestation is floor-protected (LocalRelationship).
- **Content Address Strategy**: Agent-Scoped Composite (`agent_cid`, recipe CID, `lens`); the record's bytes have a CID for printing on views.
- **Source of Truth**: private source chain; attestation on the elohim DNA for B2.
- **Integrity Zome + DNA-hash class**: existing private entry and existing attestation content type — DNA-hash-NEUTRAL.
- **Coordinator Zome**: private create -> EntryHash; attestation via the existing content path.
- **Projections**: SQLite agent-scoped only; Automerge: no.
- **HTTP Route**: none exposed to other agents.
- **SDO/RWA Test**: the worst holder of presets could profile every reader's capability and intent. Boundaries 2 and 6: presets are held by the reader (the reader's holon for B2), never aggregated across holons, never joined to journey events by an outside observer. The collective-wisdom signal a preset feeds is anonymous and aggregate, per the nervous-system architecture.
- **Anti-Pattern Check**: the widest lens is `trace`, a density level, never an autonomy apex; guardian co-authorship is mediated agency, not incapacity.

### Entity: Sample (a FlowEvent, a Verdict, and Observations)
- **Classification**: Private (B) raw event; Attested-Private (B2) outcome — the seat's Verdict and the Observation folds, which are the attestation shape the substrate already has.
- **Justification**: a journey is activity; only its outcome is the standing measure.
- **Head-Plane Cost Budget**: folds are local today; at graduation one Observation per sample, thousands per year — bundled as a corpus digest per (recipe, reader tier, window) before any notarized plane.
- **Network Stakes**: all four; the attested outcome is stage-priceable; a reader's dispute of a Verdict is floor-protected (CounterEvidence).
- **Content Address Strategy**: CID of the event / verdict / observation; env keys reader, intent, recipe, lens.
- **Source of Truth**: private source chain (raw); DHT attestation (digest) at graduation; flows sidecar until then.
- **Integrity Zome + DNA-hash class**: existing attestation content type — DNA-hash-NEUTRAL.
- **Coordinator Zome**: existing content path -> EntryHash.
- **Projections**: SQLite folds (dht_anchor_hash: yes for the digest); Automerge: no.
- **HTTP Route**: none.
- **SDO/RWA Test**: journey events are the dragnet's favourite food. Boundaries 1 and 2: raw journeys never enter a notarized plane and are held by the reader's holon; only tier digests cross. Boundary 5: the reader is never string-joined to a transport or doorway identity. Boundary 9 above: sealed against the self, recoverable only through governance with the reader's elohim as counsel.
- **Anti-Pattern Check**: no per-item heads (digest bundling declared).

### Entity: View (a ProjectionRequest rendering)
- **Classification**: Ephemeral (C). Rebuilt from a request, a window and a lens; reconstruction is `Render(request, window, lens, floor)`. Feedback against it targets the request's CID.

### Design constraints discovered
- Every role maps to an existing kind: `ProcessSpec`/`Manifest`, `Intent`, `AttentionTending`, `FlowEvent`/`Verdict`/`Observation`, `ProjectionRequest`/`Feedback`/`FeedbackSignal`, `EpistemicStanding`. The recall contract's `composition` list is already a `stages` list; the port is a re-declaration, not a redesign.
- The lens vocabulary exists at the client seam (graphos `Lens`, `CapabilityTier`, the `@capability*` element contract). The CEM plugin's cited spec path is stale (correction recorded 2026-09-11).
- `recall.rs` is 5,073 lines; the seams above are its module boundaries. The split is station zero.
- Reach vocabulary is in declared drift; every graduation row says so rather than naming a tier. The two `Reach` enums are a convergence frontier, not a third enum.
- The actor sidecar's `role@model` strings are an uncontrolled vocabulary (9 spellings over ~4 families measured 2026-09-10); the stated-lens table needs the controlled vocabulary the provenance plan already names.
- The anti-bubble content classes are qahal-governed policy per the nervous-system architecture; locally they are declared in `.claude/epr-meta/policies.yaml` as a floor rule the render checks.

## Concern-canon answers for the two decision predicates

`ResolveLens` and the candidate ranking are decision predicates and answer the canon at birth
(registration row: `elohim/eprfs/epr-cli/seam-registry.yaml`, create if absent).

| Class | ResolveLens | Ranking |
|---|---|---|
| C0 plane location | answered — client/SDK seam, local; recipe governs | answered — recipe stage `discover` |
| C1 anti-self-election | answered — a reader cannot widen its own stated tier; only revealed evidence or review does | answered — a provider cannot promote itself; standing is printed, never summed |
| C2 monotonic authority | answered — preset changes are new CIDs with provenance; no in-place mutation | answered — recipe versions by CID |
| C3 liveness | answered — a lapsed lens falls back to defaults and says so | answered — budgets bound traversal; partial windows named |
| C4 honest absence | answered — a missing profile yields recipe defaults labelled `stated: none` | answered — unreadable and partial candidates counted and named |
| C5 evidence-not-authority | answered — revealed evidence widens an offer, never a standing | answered — "candidate, not authority" on every window |
| C6a bounded work | answered — resolution reads declared tables and the reader's own folds only | answered — declared Bounds |
| C6b idempotent effect | answered — same inputs, same lens CID | answered — same inputs, same window |
| C7 advertise/serve symmetry | answered — the printed lens is the lens applied | answered — the printed selection rule is the one run |
| C8 observability | answered — lens CID, provenance and expiry on every view | answered — recipe CID, hits, provenance, omissions on every view |
| C9 identity-lineage | partial — agent readers keyed by `role@model` until the controlled vocabulary lands | n-a |
| C10 contract evolution | answered — recipe version + CID; sessions `adopt` across method changes | same |
| C11 backpressure | n-a | answered — Bounds are the declared backpressure |
| C12 consent | answered — a human preset is the reader's private record; guardian co-authorship attested; the tender conversation, never a block | answered — a wider lens is always available on request; unfilterable classes always render |
| C13 graduated authority | answered — scaffold withdraws as revealed evidence accrues; TTL and tending cadence | answered — provider standing declared per recipe; canonization by review |
| C14 witnessed residual | answered — every view carries a frontier | answered — unread and partial candidates in the frontier; disputes travel back along provenance |

## Graduation ladder (local-first → native search)

| Rung | What runs | Recipe | Lens | Sample | Providers | Scope |
|---|---|---|---|---|---|---|
| L0 eprfs local (today) | `epr flow memory recall` over one checkout | EPRFS file, CID pinned | recipe defaults | Observations on a plan | local lexical; optional MemPalace (opaque) | `Scope::Directory` |
| L1 collective | same, readers registered in the actor sidecar | `Contribution` to the collective | agent lens from `role@model`; human lens private | folds keyed by reader; weekly bank; seat Verdicts | + `ContentGraphResolver` behind `Provider` | Directory; `Reach::Workspace/Repository` |
| L2 native peer | the same stages executed by elohim-storage over its content projection | `Manifest` EPR (A), served as content | AttentionTending (B) / attested review (B2) | private event, attested digest | + peer inventory as a provider with `ranking_known` | `Scope::Ring` from Intimate outward |
| L3 network | a household searches across the holons it participates in | published at a reach ring; steward-changed; `EpistemicStatus` by review | negotiated with imagodei and sophia; revealed on every page; feeds the nervous system | digests per tier per holon | + other holons' recipes, each named with provenance | rings priced by standing |

What does not change across rungs: the four roles and five seams, the honesty and content
floors, the CIDs on every view, the second-seat judgment, the per-holon custody of journeys,
provenance on every candidate. What changes: the substrate that stores the artifacts, the
providers a stage may declare, and the scope (directory → reach ring). That invariance is the
design's test: a station that would need a new seam at L2 got the L0 seam wrong.

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
| wake-up context (L0/L1) | the first screen at `minimal` | `Render(bootstrap request, minimal)` |
| AAAK compression | a lens over a drawer | a `LensLevel`, not a storage format |
| diary | private thought | never imported, never a provider input (boundary 1) |
| semantic search | a provider whose embedding model and version are part of its method | a native provider with a declared model CID so `ranking_known` can become true |

Retirement happens by a recipe version change when a native provider answers the same
`Provider` seam with a declared model and an index that is a rebuildable projection of
notarized content per holon.

## Anti-capture invariants (the reason the recipe is governable)

- **Audience capture**: a preset never drifts without a tended review with provenance; the reader sees the cadence and the last review; a preset that narrows the same content for many readers becomes an anonymous collective signal, not a private convenience.
- **Filter bubble**: what the lens hid is one command away, on the same session, for every reader; omissions print at every lens; the anti-bubble content classes render regardless of lens.
- **Rabbit hole / radicalization pipeline**: Bounds bound every traversal; `repeated_reads` and the frontier make circling visible; a provider that cannot explain its ranking is labelled so on the page; reach rings price amplification outward.
- **Epistemic bifurcation**: recipe CID and lens CID on every view; two readers, one comparison. A `FeedbackSignal` of kind Correction against a projection is floor-protected and reaches the recipe's steward; enough of them quarantines the recipe version.

## Stations (for the implementation plan)

0. Split `recall.rs` behind the five seams (`journey`, `discovery`, `providers`, `lens`, `render`, `refusal`, `measure`), outputs byte-identical, pinned by the existing 51 tests plus a golden of the round-5 first screen; the recall contract is re-declared as a `ProcessSpec` with `Bound`s, same CID semantics.
1. `ReaderRef` + `ResolveLens` from the actor sidecar and a declared `role@model → LensLevel` table in the recipe; `Render` per lens with the honesty and content floors; lens printed on every view; `--lens` widens on request.
2. Bootstrapping head: headline and run-plane become `ProjectionRequest`s rendered at `minimal`/`simple`; the two bespoke renderers retire; rule declared in `.claude/hooks/.epr-meta`.
3. Standing reader: the question bank as Intents, `sample` and `judge` verbs producing a `FlowEvent` and a `Verdict`, weekly routine, habit check over a rolling window; a `Feedback` verb for disputes.
4. Human lens: imagodei capability profile as stated, Sophia `Recognition` as revealed, the preset as an `AttentionTending` record with TTL and tending, a graphos review-and-reveal element, and the B2 co-authorship attestation; the anti-bubble classes declared in policy.
5. Graduation L1 → L2: recipe contributed and graduated as a `Manifest` EPR; `ContentGraphResolver` behind `Provider`; `Scope::Ring`; the same a2o scenarios run against a household peer.

Each station ends with the rule it proved written where the concern lives (a `.epr-meta` rule
or a habit check), and a fresh-reader sample folded on the recipe it changed.
