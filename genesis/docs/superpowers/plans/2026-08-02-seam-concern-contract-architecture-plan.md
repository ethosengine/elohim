---
title: "Seam-Concern Contract Architecture — first-class concern clusters that cascade to every boundary"
id: seam-concern-contract-architecture-plan
status: Active
class: protocol-canonical
domain: D1 (cross-cutting envelope/IoC seam; tooling legs are process-meta and say so per-task)
sprint: proposed-new-rung (post-saga architecture sprint; not an existing Sprint-N)
requires_env: [household-nodes]
cites:
  - epr-meta-policy-registry-measure | the define-once-bind-many mechanism this plan binds concern policies through — registry rows, version pins, measure-tier dispatch precedent | sha256:474eee1686e3123b | path: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
  - epr-meta-compose-gate | the P1 edit-time gate whose cascade/class ladder the concern validators evaluate under | sha256:6052ce071bfec509 | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - elohim-seam-map-concern-routing | the concern-routing atlas naming the seams the decision-point registry enumerates | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - substrate-trust-contract-runbook | the invariant-to-probe home each live-metered concern gets a row in (P0.2) | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - elohim-epr-integrator-compatibility-contract | the D1 IoC seam lineage the seam-contracts crate implements (pure predicates wired by services) | sha256:fa4fe159d019cd08 | path: genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-epr-integrator-compatibility-contract.md
  - sealed-contract-edges-governor-frontier | prior art for conformance seals on graph edges — the registry census is its code-seam sibling; also the pin resolution mode's cite-seal instance | sha256:ace1788fa44a293f | path: genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md
  - genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md
  - identity-head-key-lineage | the declared-head-over-DAG primitive (third instance) and the authorized resolution mode — controller sets, refusal-not-outvoting | sha256:95950b918c8803bc | path: genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
  - genesis/data/timeline/backlog/content-head-election-vs-reach-fork-arbitration.md
  - substrate-convergence-five-defect-arc | the closest structural precedent — five stacked concern classes behind one red outcome measure; source of the suspect-a-stack-not-a-miss discipline the precedent record carries | sha256:c43ac25e6f6ac6e9 | path: genesis/docs/content/elohim-protocol/history/2026-07-12-substrate-convergence-five-defect-arc.md
  - ci-orchestrator-recurring-anti-patterns-museum | format precedent for the concern canon — frequency-ranked recurrence + severity-starred admission + memory_anchors convention the matrix ranking adopts; also holds C4/C7/C2 instances in the CI family | sha256:0e325f2f174689ae | path: genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
memory_anchors:
  - project_inventory_exchange_not_byte_replication
  - feedback_reach_head_replication_distinct_planes
  - project_principle_p1_reconciliation_controller
  - project_full_arc_authority_disables_network_get
  - project_dna_hash_blind_to_coordinator_zomes
  - project_closed_loop_ingest_drain_prior_art
  - feedback_deterministic_flag_agent_canon_stasis_pattern
  - project_epr_router_empties_on_poisoned_scope
  - project_mishpat_commitment_cid_is_entry_hash
  - project_conductor_signal_msgpack_decode_class
  - feedback_verify_the_measure_before_the_ranking
---

# Seam-Concern Contract Architecture

## Why (the evidence that forced this)

The 2026-08-02 conductor-plane convergence shift cured a fleet deadlock in five serial
waves. The post-mortem shape was not five problems — it was a small set of **concern
classes**, each rediscovered bespoke at a new seam, each invisible until the previous
cure added the meter that exposed the next wall:

| # | Concern class | Where it bit (this arc alone) |
|---|---|---|
| C1 | **anti-self-election** | decide_head_action Hold, gapfill_would_self_elect, zome no-chain gate, witness-bootstrap routing, contest quiescence |
| C2 | **monotonic authority** (never backwards; which clock) | stamp guard provably_newer, selector tiebreak, three-clocks-in-one-column |
| C3 | **liveness** (a legal move exists from every state) | the two-way-declared deadlock — four locally-correct refusals composing into no-legal-move |
| C4 | **honest absence** (absent ≠ timeout ≠ refusal) | head-record responder, LocalResolve::Unresolved split, merged contest-failure log line |
| C5 | **evidence-not-authority at transfer** | carried-record validation, peer-as-courier, target-id gate |
| C6 | **bounded work / quiescence** | sweep budgets, per-tick caps, (id,target) idempotence ledgers |
| C7 | **advertise/serve symmetry** | disconfirmed at this arc's inventory/head-record reach filters — but confirmed at three other seams (see the precedent record) |
| C8 | **observability-per-decision** | canonical consumed-but-never-counted; tier/class/reason labels added wave-by-wave |

Safety rules accreted incident-by-incident, and the instruments could not discriminate
which plane was refusing. **Liveness was nobody's job.** Each concern was hard-fought
once — this plan makes each one *stay solved*: solved-at-one-seam ⇒ prompted-at-every-seam.

### The precedent record — the classes are older than this shift

Mining the corpus (git history, timeline records, MemPalace) shows every class above
has **at least two dated instances before this shift, at seams this arc never
touched** — several in substrate families that are not the dataplane at all. No concern class in
the mined corpus is younger than 2026-07-04. A compressed record (dates are commit or
record dates; the full mining detail lives in the ceremony chronicle):

| Class | Prior instances (other seams, other substrate families) |
|---|---|
| C1 | `b91ee0f95` 2026-07-28 content write path (`upsert_with_anchor` self-declared on every own-conductor commit); `2175f2b60` 2026-07-28 heal_content third site — a *terminal* write invisible to both guarded sweeps; `8844cd1d6` 2026-07-11 boot re-projection stamping the root-author fallback over the adopted canonical, 2,838 rows per restart; `f9ab9b853` 2026-07-24 custody plane (self-held custody: gossip-publish only, never a self-dial) |
| C2 | `a83b35079` 2026-07-29 **epr-cli/REA sidecar**: "latest" by append order vs by occurredAt — commit body: *"the exact `8a1c531dc` bug, reintroduced via replay"* — an uncanonized concern re-minting itself; `25921a07a` 2026-07-10 zome selector ordered by `get_links` arrival until the `(timestamp, create_link_hash)` tiebreak; museum #12 — first-that-matches degenerating to index-0-always-wins |
| C3 | `192f77cd0` 2026-07-31 acquisition pull-queue — the only transition out of an exhausted pin was a human HTTP DELETE; `c16daaa79` 2026-08-01 doorway breaker — a dropped half-open trial future latches shed-forever (`halfopen_without_record_deadlocks_forever`); `74fbdf2d7` 2026-07-29 heal starving the gossip that would have ended it; the 2026-07-11 over-broad GapFill guard that froze convergence (the pre-wave-5 shape, one month earlier) |
| C4 | `dd1824e03` 2026-07-22 identity reconcile — *unreadable ≠ absent*, cure = abstain + observe; `270dbafac` 2026-07-11 `ice_servers`/`iceServers` — absent-because-misspelled ran the fleet with zero ICE servers since inception; `d6c88e385` 2026-07-23 shefa facing — *unmeasured ≠ zero*; museum #1 — NOT_BUILT reads as 0 failures |
| C5 | `b91168724` 2026-07-26 carried-record validation — the **entry↔action binding** clause closes the swap attack that signature + hash checks alone admit; `9e9c9d023` 2026-07-24 **eprfs/cite-seal family**: `BlobCid::verifies` hardcoded the wrong codec, so self-verification was inert for every real fingerprint — and the existing test computed both sides the same wrong way, staying green; `4d90bf276` 2026-07-25 **policy family**: an unresolvable validator may not *harden* an advisory rule — unevaluatable evidence confers nothing, in either direction |
| C6 | `6369721fd` 2026-07-04 view-federation codec — the responder rejected its own oversized reply; cure trims *before* signing; `74fbdf2d7` — a retry ladder against an uncancellable call created unbounded work **with no loop anywhere in the diff**; `1b445b481` 2026-08-01 — a responder budget compile-asserted strictly below the requester's |
| C7 | `4e1f34f1e` 2026-07-25 **policy family**: *"15 rules name validators that do not exist… a rule that names a validator it does not have misrepresents itself to every reader"*; `192f77cd0` provide-loop advertising gated on a serve-side fact it didn't check; `da8975176` this shift — `carried_present` mislabeled "peer answered" as "bytes present" |
| C8 | `486982bb8` 2026-07-25 — a leg that *"fails 100% and says nothing"* (its only signal a dropped `debug!`); `b19f12014` 2026-07-31 — a gauge blind to the distinction between correct-refusal and failure (*"not a conservative gauge; an unreadable one"*); `98a3c5f1e` 2026-08-01 — a label hardcoded 0 "for symmetry": structurally constant is worse than absent |

The closest structural precedent to this whole shift is the five-defect convergence arc
(2026-07-12 history record): five stacked classes behind one red outcome measure —
*"when an outcome measure stays red through multiple correct fixes, suspect a STACK,
not a miss."* And the cross-family fact matters most: C2, C4, C5, and C7 each have
confirmed instances in **non-dataplane substrate families** (the REA
developer-valueflow sidecar, the `.epr-meta` policy family, the eprfs cite-seal
family, the CI family). The concern classes are not dataplane phenomena that might
someday generalize — they have already generalized; only the cures haven't.

**The pattern is already emerging bottom-up** — this plan formalizes, it does not invent:
pure decision predicates (`decide_head_action`, `canonical_move_verdict`,
`declared_divergence_should_route_to_contest`, `select_canonical_winner`); typed reason
enums (`StaleReason`, `LocalResolve`, contest failure classes); `.epr-meta` inject rules
firing on exactly the right diffs (advisory-only: "validator not registered"); the policy
registry's define-once-bind-many + flag→agent→canon→stasis dispatch (Implemented,
2026-07-02 spec). What is missing is the **binding surface** (which code points answer
which concerns), the **executable contracts** (property harnesses per concern), the
**cascade** (a concern change fans out as a worklist), and the **inheritance surface**
(the SDK shape by which a peer runtime outside this repo receives all of the above).

## Goal

A concern solved anywhere becomes: (1) a **versioned policy** in the registry,
(2) a set of **registered decision points** bound to it, (3) a **property-test contract**
each bound point must pass, (4) an **inject/validator** that fires on diffs touching a
bound point, (5) a **dispatch worklist** when the policy version bumps, and (6) a
**published SDK artifact** so external peer runtimes inherit the same contracts — so
every sprint compounds instead of rediscovering.

**Success criterion (how we'll know it worked):** the next architecture-scale shift
finds its walls already ranked as `unexamined` cells in the concern×seam matrix —
forecast hits, not bespoke production discoveries — and the calibration ledger (design
surface 6) measures exactly that, including the canon's own growth rate.

## Non-goals

- No retrofit of every historical seam in one sprint (adopt at the freshly-cured
  convergence seam first; cascade outward by dispatch, not by big-bang).
- No WASM/content-addressed validator resolution (the policy-registry spec's brit/eprfs
  graduation owns that; v1 validators follow its transitional guidance).
- No change to any live decision semantics — every adoption leg is behavior-neutral,
  verified by the existing test suites.
- No new DHT entry types (Precedent lineage per the policy-registry spec).
- No entity-resolution/merging of genuinely duplicate authored objects — two agents
  independently authoring the same real-world thing is a *merge* problem, a distinct
  future design (operator, 2026-07-11), never a head-election problem.

## The frame — brit as governance architecture

This is not a git analogy; it is **brit** — covenant: rules that bind by witnessed
standing, not by force, which is why the three properties below are inseparable. The
head-authority machinery these sprints keep building — content-addressed immutable
objects, declared heads over a CIDv1 DAG, deterministic candidate arbitration during
adoption, staged-then-earned authority tiers, verification on transfer — is the brit
primitive (declared-head-over-DAG; the identity-head spec names itself its third
instance, this shift's canonical-content-head election is the fourth). The concern
architecture must therefore be **brit-native at birth, not brit-graduating later**:
every artifact this plan creates has (a) a content-addressable body, (b) lineage
carried inside the hashed bytes, and (c) **standing** (binding force, scope,
supersession) held on a separate plane that cites the content — the two-plane split
the policy-registry spec already pins. (This frame is presented first because it
governs every surface below; its *full* realization — actual CIDv1 residency on the
eprfs layer — is deliberately last, P5-held, with P0.4's liftability gate guaranteeing
the lift is a move, never a rewrite.)

**One shape, four resolution modes.** The four substrate families this ceremony
tested (p2p dataplane, EPR content, REA value/event, cite-seal governance edges)
genuinely share the brit *shape* — but they resolve "which version is current" in
four mutually exclusive modes, and naming them distinctly is what makes the concern
architecture four-family-native instead of dataplane-native with three families
bolted on:

1. **Elected** — competing *candidates* arbitrated deterministically during adoption,
   with tier precedence and, within a tier, a **named notarized clock** — the DHT
   link timestamp, "the one clock that can order two declarations," never a
   conductor-local clock — with a deterministic tiebreak on the create-link hash
   (`select_canonical_winner` is newest-within-tier on that clock; the dataplane's
   contest→election→obey arc is this mode's concrete instance, and the `Arbitrated`
   harness in surface 3 formalizes it).
2. **Pinned** — the *consumer* declares which version applies: "a declared dependency,
   never recency." Policy bindings and cite-seals live here; a new version moves
   nothing until each binding re-declares. The cascade (surface 5) exists *because*
   this mode exists — stale pins are what it fans out over. Governing artifacts
   resolve by pin, not by election.
3. **Authorized** — a controller set (self → steward-set → recovery quorum) permits
   the move; an unauthorized move is *refused*, never out-voted (identity heads).
4. **Appended** — no head exists at all: REA events are immutable facts; corrections
   are new events; "current state" is a derived fold, never a moved pointer.

Standing correspondingly takes four shapes — **record** (a declared head record),
**attestation** (a Precedent citing a policy CID), **fold** (REA's derived state — on
the DHT the event *is* the standing; a separate standing object exists only in
projection), and **pin** (a sealed fingerprint, re-blessed by judgment, never
auto-blessed). Every artifact this plan creates declares its standing shape; an
artifact whose standing is a fold cannot carry a supersession field without
duplicating truth, and one whose standing is a pin cannot be elected without breaking
its consumers.

**Genuine competition never auto-resolves.** The R1 decision (2026-07-31, the
content-head fork-arbitration record) is canon here: when two conductors hold
genuinely competing *declared* heads at the projection plane, no local rule
arbitrates — `declared_at` is the receiving conductor's clock, not globally
comparable (LAW-3 — itself an instance of C2's which-clock guarantee) — divergence
escalates to a fresh authority declaration, whose live interim channel is the deploy
declare-cycle R1 made load-bearing. This *composes* with mode 1 rather than
contradicting it, and the reconciliation is exactly the plane distinction C0 exists
to force: `select_canonical_winner` arbitrates DHT-witnessed candidates on one shared
anchor at the **truth** plane, where the link timestamp is notarized; R1 governs
cross-conductor declared rows at the **projection** plane, where no comparable clock
exists. For governance artifacts, this plan proposes the standing plane as the
analogous escalation channel — the governance-artifact *extension* of R1's rule, not
something R1 already authorizes. Routine version bumps travel **supersession
lineage**: the new version declares what it supersedes, so any holder of both
resolves the *artifact's tip* deterministically — no competition exists; but which
version *binds a consumer* is still mode 2's pin — the tip moves, bindings move only
as each re-declares, and the cascade (surface 5) is the worklist that closes that
gap. A community's variance is a different *standing* attached to the same
content-identical CID (which is precisely why variance is not a competing head); and
only genuine competition — two declarations, no supersession edge — escalates.

**Authority graduates along the social-reach axis.** The protocol's notarized reach
vocabulary (`REACH_LEVELS`: private → self → intimate → trusted → familiar →
community → public → commons) applies to the *authority criteria themselves*:
today's canonical-head declarer is a labeled god-mode scaffold (dev tier) whose
recorded successor is earned-tier arbitration (reach-cohort + signature, the R2 arc);
ratification of governing rules today is the operator+agents pre-p2p community, whose
successor is council affirmation over the p2p substrate. The apex of this ladder is
communal backstop — authority never terminates in an individual key-holder (the
stewardship-over-sovereignty canon). The canon makes this a first-class concern
(C13 below) rather than a comment convention.

## The concern canon (sixteen classes at authoring, measured over four substrate families)

The convergence shift surfaced eight; widening to the four substrate families (this
ceremony's mining) added five, split one, the social-reach lens added one more (C13),
and the residual-channel lens added the last (C14 — the class that covers what no
class yet covers). Each class is stated in **guarantee form** — even
where the concern reads as a practice, the policy states it as a falsifiable clause.

| Id | Class | Guarantee (the falsifiable form) |
|---|---|---|
| **C0** | **plane location** (evaluated first) | Before any concern binds, the symptom names its plane — custody/availability · reach/audience · head/version, plus the truth-vs-projection distinction. (When C0 asks for a plane, the answer vocabulary is exactly this guard set — the R1 three-plane guard extended by the atlas §7 truth/projection split. The content/standing two-plane split is the policy-registry spec's own established sense, and "dataplane" is a proper noun; both stand. The ceremony's four *substrate families* and the matrix's §7 column grouping are different axes and are never called planes.) A cure applied to the wrong plane is a defect even when locally correct. *This is the one class a sharp authority canon actively worsens* — eight authority concerns make every symptom look like an authority problem; the deleted `content_head_election.rs` (2026-07-09, operator ontology-guard) is the type specimen, and REA's C2 guard living in the projection (not the DHT truth) is where C0 and C2 meet. |
| C1 | anti-self-election | No component crowns what it authored — except through the *guarded legitimate arm*: self-candidacy is admissible exactly when `not_retrievable` itself proves a local chain exists (`134331c83`). A flat prohibition would forbid a cure the fleet depends on. |
| C2 | monotonic authority | Selection is decided by declared authority tiers and, within a tier, a **named notarized clock** with a deterministic tiebreak — never a conductor-local clock, never arrival order. (LAW-3; `select_canonical_winner`'s newest-within-tier on the DHT link timestamp, tie-broken on `create_link_hash`; the append-order-vs-occurredAt replay.) |
| C3 | liveness | From every reachable non-terminal state, at least one *automated* transition is enabled. A human hand (or a deploy) as the only exit is a liveness hole, not an escape hatch. |
| C4 | honest absence | Absent, unreachable, refused, and unverifiable are distinct answers, and unreadable/unmeasured never reads as zero/complete. On a full-arc fleet a local `get` miss maps to **Unreachable** (gossip failed), never Absent — the first adoption of `Answer<T>` must not re-merge what wave 4 separated. |
| C5 | evidence-not-authority (may-believe) | A transferred claim confers only what the receiver re-derives — signature over the action **and** the entry↔action binding (the clause that closes the swap attack). Evidence that cannot be evaluated confers nothing, in either direction (an unresolvable validator neither hardens nor softens). |
| C6a | bounded work | Every loop, sweep, and retry ladder carries a declared budget it provably respects — **a retry policy against an uncancellable call is a loop** even when no `loop` token appears. New pacing binds the existing drain kernel (`drain_publish_queue`) rather than minting a fourth vocabulary. |
| C6b | idempotent effect | Replaying a decision against settled state mints nothing and double-applies nothing — while never licensing lazy acceptance: P1 (storage as reconciliation controller) stays eager; quiescence is about *effect*, not effort. |
| C7 | advertise/serve symmetry | What a surface advertises equals what it serves — capability, inventory, validator, coverage. Confirmed at three seams (policy rules naming phantom validators; provide-loop vs `caughtUp`; `carried_present` mislabeling); the class needs no further probation. |
| C8 | observability-per-decision | Every decision outcome increments a labeled counter through a typed reason (`label()` from an enum, never a raw string); failures are counted beside successes (a ratio is readable, a bare failure count is not); no label is structurally constant; and every meter names its semantics — census or sample — so a rotating-page gauge is never read as a drain. |
| C9 | identity-lineage continuity | Every reference to an agent resolves through a lineage root stable across key rotation; a re-key never silently orphans state keyed on the old identity — it fails loud or re-anchors (the fossil-key custody-zero is the type specimen). |
| C10 | contract-evolution honesty | A wire/config/schema contract change is rejected or observed, never defaulted — an unknown or renamed field fails the parse or increments a counter (`ice_servers`/`iceServers`; diesel decode change green under `cargo check`). |
| C11 | externally-imposed backpressure | Under load it did not schedule, a seam degrades by a declared, counted policy — defer-with-retry-after, shed, or decline, each naming its reason — never by unbounded queueing or OOM. Distinct from C6a: C6a is "does my sweep respect my budget"; C11 is "do I survive traffic I didn't choose" (`DeferReason` already discriminates the two). |
| C12 | consent/authorization (may-act) | Authority to act is verified structurally against a notarized grant at the acting node — never inferred from a carried claim, a self-asserted header, or a substring match. C5 constrains belief; C12 constrains action. |
| C13 | graduated authority (scaffold-with-successor) | Every authority gate serving a lower social tier is *labeled* as a scaffold and names its successor criterion and graduation trigger on the notarized reach vocabulary (`REACH_LEVELS`). The live two-rung instance of graduated authority is the staging→earned canonical-tag pair — a distinct vocabulary — whose recorded successor criterion (R2) is reach-cohort-based. A scaffold that outlives its tier silently becomes capture; the apex is communal backstop, per the stewardship-over-sovereignty canon. (Done right twice by hand: the god-mode canonical-head declarer — labeled, with R2 recorded as successor — and R1's deploy declare-cycle, made load-bearing as the labeled interim authority channel. Both labels live in docs/backlog rather than at the gate sites; this class requires the label and successor *at the gate*.) |
| C14 | witnessed residual (exception metabolism) | Every decision point's outcome set closes with a residual arm that is **witnessed, never dropped**: an exception that fits no class is captured with a context capsule sufficient for cross-node RCA (inputs digest, decision state, the clocks/lags/peer-states in play — distributed causes arrive as stacks), carried as evidence (C5 applies to the capsule), delivered to the developer-facing RCA report, and dispositioned — cure, declared variance, or a **new concern class** via the calibration ledger's miss-of-canon intake. A residual only a `debug!` sees is C8's "fails 100% and says nothing," aimed at exactly the realities we did not predeclare. |

**Folds and rejections** (tested and declined, so the canon stays small): partition /
split-brain healing is the *composition* C1∧C2∧C3, not a class (the two-way declared
stalemate is its type specimen; the `MongoK2Store` genesis-pair islanding is the same
composition at the discovery layer). DNA-hash-neutrality classification (integrity vs
coordinator diffs) has no runtime predicate — it becomes the third registered
validator (surface 4), not a class. Deploy/build-provenance coherence is real and
recurring but its home is the `declarative-desired-state` spine rung — it enters as a
forecast row, not a canon row.

**The canon is open, and its growth is measured.** The calibration datum from this
shift — five waves, zero new classes — was measured over one seam-cluster in one
substrate family. Widening to the four families (plus the reach and residual lenses)
grew the canon 8 → 16. The honest generalization:
the canon is stable *within* a family and grows when a family is added — so the canon
states its measured scope beside its count, and the calibration ledger (surface 6)
records every `miss-of-canon` as the evidence a new class must present — and the
residual channel (surface 8) is that growth's intake organ. The matrix costs
|concerns|×|seams| and the birth rule costs |concerns| questions per new
predicate — canon growth is a budgeted event, not a free one.

## Design (seven surfaces, each composing an existing mechanism)

1. **Concern Canon** — the sixteen classes are canonized across the registry's
   **two homes**, following the precedent the repo has already adjudicated
   (`governors.yaml` header: *"policies.yaml's fail-loud validation (rightly)
   rejects non-enforcement classes, and a fake predicate would corrupt that
   discipline"*): a class with an evaluable predicate lands as an enforcement row
   in `.claude/epr-meta/policies.yaml`; the predicate-less classes land as
   Precedent-shaped rows in a new sibling registry, `.claude/epr-meta/concerns.yaml`
   (its own file for the same reason governors.yaml is; its named consumers are the
   census (P3.2) and the canon projection (P4.6)) — **never** as phantom-validator
   rows (the plan's own C7 violation) and never as consumer-less rows, which would
   be silently-unloaded by another name. Every
   row carries statement, scope predicate, binding class, why, dispatch-agent,
   `established_by`, and two fields adopting the museum's recurrence/severity
   discipline as structured data (the museum keeps them as a prose column and a
   star footnote): `recurrence` (unit: distinct shifts) and `severity_class`
   (loud-fail vs silent-corruption) — canon admission bar: ≥2 dated instances at
   distinct seams, deliberately below the museum's ≥3-shift bar because a canon
   row is cheaper than a museum row. Each class also declares a **findings-ledger
   fingerprint namespace** so future production self-reports land pre-classified
   (the runtime ledger may be empty at canonization — this is forward wiring, not
   a claim of current data). Pins update via `epr-meta-pin.py --write`; the
   registry stays under its 64KB fail-loud cap. Probes split by a
   **two-home rule** honoring the trust-contract runbook's own doctrine ("every
   trust claim gets a probe"): a concern with a *live meter* gets a runbook
   invariant→probe row (C6a mint-then-quiet; C7 advertise/serve counters; C8
   `elohim_content_canonical_answers_total{tier}`; C11 shed/breaker counters); a
   *design-time* concern (C3's harness, C4's compile shape) cites its contract test
   in the seam registry instead — no probeless runbook rows. Policy = semantics;
   `.epr-meta` bindings = placement — the define-once-bind-many contract.

2. **Decision-point registry** — schema-first: `seam-registry.schema.json` is
   published in the SDK schema family (surface 7), and each participating crate
   carries a `seam-registry.yaml` conforming to it — enumerating every pure decision
   predicate / verdict fn / boundary answer type, the concern ids it must answer, and
   its contract-test citations. Source of truth: each crate's registry is authored
   in-crate as class-C operational metadata (a repo/process artifact — no DHT entry
   type, per Non-goals); the published schema is its contract; the census and the
   matrix are derived read-models, never authored. ("Seam" in the registry names a crate/module locus;
   the atlas's composition-seams and participation tracks are matrix vocabulary —
   surface 6 — not registry vocabulary.) The census (wired into
   `placement-audit.py --epr-meta`) verifies both directions: every registered point
   cites passing contract tests; every concern lists its bound points. The registry
   **seeds from four crates on day one** — the pattern is already four-crate-emergent,
   unregistered: elohim-storage + content_store zome (the four named predicates),
   doorway-service (`decide_reconcile` with `ReconcileDecision::Unreachable` — an
   independent third derivation of the honest-absence type; `Disposition`,
   `SsrFallbackReason`, `FallbackOutcome`, `K2PutOutcome` as reason-labeled
   outcomes), and steward/node (`ConsensusOutcome` — a four-way honest absence at a
   leader-election surface where C1 is the definitional question and nothing binds
   it; `DeferReason` — C6a/C11/C8 already discriminated). A matrix seeded from one
   crate forecasts its own blind spot. Census loading degrades **per-row** — one
   malformed registry row flags itself; it never empties the binding set (the
   EprRouter poisoned-scope lesson: fail-closed `collect` over rows degrades to
   silence). Convention over macros: registration is a YAML row + a
   naming-convention lint, no proc-macro.

3. **`crates/seam-contracts`** (published name `elohim-seam-contracts`, lib name
   `seam_contracts`) — the IoC/DI leg (D1 integrator-compatibility lineage), born
   SDK-grade:
   - **A leaf crate**: zero first-party deps; std-only default; features
     `serde` / `ts` (wire + ts-rs export) / `harness` (property harnesses,
     default-off) — the shape that keeps the WASM zome path open
     (`doorway-client`'s hdk-optional pattern; CI asserts
     `cargo build --target wasm32-unknown-unknown --no-default-features`). MSRV
     declared — match the highest `rust-version` already in-tree, verified at
     P1.1 (`crates/` has no root workspace; each crate is its own workspace root,
     so the checks live in-crate and CI covers them via the existing `crates/**`
     watch globs). A `no_heavy_deps_in_dep_tree` boundary test (the
     `elohim-facings` pattern) asserts its own lockfile stays clean. `crates/elohim-sdk`
     re-exports it as `pub mod contracts`.
   - `Answer<T>` three-way enum (Present / Absent / Unreachable) for boundary
     returns — C4 as a type. Rust-generic internally; **monomorphic on the wire**:
     a per-view envelope validated by `schemas/v1/objects/answer.schema.json`,
     source-of-truth per the View Schema Contract (schema first → Rust struct
     conforms → `schema_contract.rs` catches drift → TS generated, never hand-edited)
     (no ts-rs generics — the repo has none and won't get its first in a flat
     458-file generated dir; the `DiversityHint` tag/content pattern is the proven
     emission shape). The distinction has been re-derived bespoke three times
     (`FetchOutcome`/`RenderTerminal`, the freshness enum's `unverifiable`,
     `epr-pull-status`'s documented "tri-state" nullable) — 54 of 105 view schemas
     sit on the bare-nullable collapse surface, so view CONVENTIONS gains
     **Rule 11 (honest absence)**: a field whose absence has two provenances must
     not be a bare nullable.
   - `ReasonLabel` trait (`label()`, `all()`) + conformance test (unique, stable
     label sets) — C8 as a compile-shape. Wire-facing discriminations (the answer
     triad + reasons a client must render) graduate to schema-governed enums
     (`answer-state`, `answer-reason` — deliberately without `_dna` metadata:
     vocabulary the DHT never validates must not move the DNA hash). Substrate-
     internal reason enums (`StaleReason`, `LocalResolve`, contest classes) stay
     crate-local implementing `ReasonLabel` — a Prometheus label vocabulary is not
     a protocol-versioned artifact. One warning from history: hash-typed fields in
     signals must stay typed (`HoloHashB64`) — a `String` mirror at the msgpack
     boundary makes the meter go dark while the decision succeeds; the class has
     recurred once already.
   - `Arbitrated` property harness: permutation-invariance + tiebreak determinism
     (C2) — the abstract form of the dataplane's contest→election→obey instance;
     its regression fixture already exists in history (`get_links` arrival-order).
   - `Quiescent` harness: decision replayed against settled state mints nothing
     (C6b/C1) — with the explicit clause that quiescence never licenses lazy
     acceptance (P1 stays eager).
   - `Liveness` table harness (the genuinely new artifact): enumerate a predicate's
     input state-space, assert every non-terminal state has ≥1 enabled transition —
     run at design time (C3).
   - Doc convention: every public predicate carries the concern ids it answers, its
     contract-test citation, and the incident that forced it —
     `canonical_move_verdict` is already written in this spirit (three-tier
     rationale plus dated forcing incidents; the convention adds the structured
     fields — concern-id tag, contract-test line); it is the exemplar, not a new
     invention.
   Decision predicates stay side-effect-free plain-data functions; services wire
   them — inversion of control at the seam the integrator-compatibility contract
   already names.

4. **Validator registration** — register **three** concrete `epr:validator-*`
   entries so standing prose nags become evaluated gates: `heal-fills-never-moves`
   (diff adds a `StampMode::Declare` call site outside the canonical-channel list,
   or touches a stamp guard without citing its contract test); `bounded-work` (a
   new loop/sweep **or retry ladder against an uncancellable call** without a
   registered budget — the scope history demands); and `dna-hash-neutrality` (an
   integrity-zome diff classified and labeled: hash-moving vs coordinator-only).
   Scoping evidence for the wider registration debt: the 2026-07-25 hazard sweep
   found 15 rules naming phantom validators, of which two are mechanizable (the
   first two above — `dna-hash-neutrality` is new, not one of the 15); the live
   count at ceremony time has grown to **27 occurrences across 6 phantom ids,
   including one inside policies.yaml itself** (`epr:validator-eprfs-meta-domain-
   neutrality`) — re-measure at execution and drop every phantom key. Follow the
   policies.yaml transitional note for concrete validator code.

5. **Cascade protocol** — resolution for policy bindings is **pinned** (mode 2), so
   a policy version bump leaves every bound point still pinned to the old version —
   the census diffs pins and each stale pin becomes a fingerprinted finding →
   background dispatch (the existing flag→agent→canon→stasis machinery, exactly as
   the measure tier already does for LoC) → each dispatch re-conforms the point or
   documents variance. Two hard-won clauses: the cascade keys on **registry pins,
   never bundle/DNA hashes** — a coordinator-only zome change moves no hash, so a
   hash-keyed cascade is structurally blind to exactly the hot-swap class; and
   **graduation events cascade too** — when a C13 scaffold's successor lands (god-mode
   → earned-tier), every cell that scaffold touched re-examines. Cross-repo, the
   cascade has a compile-time shape (surface 7): pin constants with `#[deprecated]`
   on superseded versions, so an external consumer's build names the exact policy
   that moved.

6. **Forecasting surfaces — respond to patterns before re-colliding with them.**
   **The `unexamined` cells ARE the forecast.** Cascade (surface 5) is reactive;
   forecasting is the proactive dual, four instruments:
   - **The concern × seam matrix** (generated by the census, never hand-written):
     rows = concern classes; columns = the atlas §3 seam catalog grouped by its §7
     planes (control/truth · data · projection), **plus a governing/meta-plane
     column** (C4, C5, and C7 have confirmed precedents there, and C2's strongest
     precedent — the epr-cli replay — sits in the adjacent developer-tooling
     territory; a track-keyed matrix would score all those cells `n-a` when they
     are `unexamined`) **and a bridges column** (a translation seam, not a
     participation track). Participation track (T1–T4) and reach tier are **cell
     attributes**, not axes. Cell states: `conformant` (contract test cited +
     passing) / `variant` (documented, negotiated waiver — variance is standing,
     not violation) / `unexamined` / `n-a`. Unexamined cells rank by
     recurrence × severity × proximity-to-the-next-rung, on the museum's own
     frequency-ranking convention — not flat. Authority split with the atlas: the
     atlas §4 concern-routing table remains the *routing* authority (where a
     concern lives); the canon is the *guarantee* authority (what must hold) —
     when they disagree, route by the atlas, then correct whichever recorded the
     world wrong.
   - **Trajectory-crossed ranking**: cross unexamined cells with the roadmap's and
     spine's next rungs = the forecasted-collision list, surfaced at ceremony time
     beside the Gap Ledger. The instrument ships **pre-seeded** (below), not empty.
   - **Calibration ledger**: every forecast row is fingerprinted; when a spine node
     flips or a new red lands, the ledger records **hit** (forecast row) /
     **miss-of-ranking** (known class, unforecast seam) / **miss-of-canon** (new
     class). The birth rule becomes tunable and the canon's growth rate becomes
     measured. Spot-check the census resolver against disk before trusting any
     ranking — a mis-resolving audit doesn't degrade, it *reorders*.
   - **Design-time gate (birth rule)**: extend/bind the `p2p-design-gate` skill and
     the `.epr-meta` compose-gate so a NEW decision predicate, boundary answer type,
     sync message, or route cannot be born unregistered — its design answers, per
     concern class, "how does this seam answer it, or why n-a" (the same at-birth
     law `doc-frontmatter-at-birth` applies to docs). The questions are asked once,
     cheaply, at design — instead of answered expensively, serially, in production.

   **The seeded forecast (first worklist for P4.4)** — spine/roadmap rungs × cells
   unexamined there, ranked; two cells are already *confirmed live*, found by this
   ceremony's own mining:

   | # | Rung × seam | Class | Predicted (or found) collision |
   |---|---|---|---|
   | 1 | reach-enforced-everywhere × doorway egress | C7+C12 | `should_serve_response` exists, re-exported, **zero production call sites, fail-open no-reach branch** — enforcement will land on a dead predicate and read green while serving unauthorized. |
   | 2 | reach-enforced-everywhere × the other four egress surfaces (DHT, CRDT sync, shard, federation) | C7 | Each surface needs advertise-filter ≡ serve-filter; the CRDT surface is already recorded as "broadcast-only fail-closed = exclusion, not enforcement." |
   | 3 | doorway-failover | C4 | The invariant demands serving\|shedding\|dead — three-way — with no `Answer<T>` behind it; clients will conflate shedding (retry) with dead (fail over); the live red already has this shape. |
   | 4 | identity-cross-signed + recovery | C9+C2 | A rotation's authorizing clock is the quorum's decision time, not the key's mint time — three-clocks recurs at the identity seam; the chain-root cid does not exist yet, so there is no stable subject for monotonicity to be about. |
   | 5 | declarative-desired-state | C1+C6a | A peer authoring the signed manifest it then obeys is self-election at the config surface; an unbudgeted desired-state reconcile replays the heal-throughput smell at config scale. |
   | 6 | operator-runtime-surface | C5+C12 | A commitment-gated restart must verify the `delegates-compute` grant in the peer's own conductor — I1 has only ever been proven for heads, never permissions; today's gate sits at the courier. |
   | 7 | **bridges (confirmed live)** | C5 | did:web resolution never checks `doc.id == did` — a host can return another domain's DID as a successful resolution; outbound SSRF is guarded, inbound response is not. |
   | 8 | **bridges (confirmed live)** | C4+C9 (+C12 predicted) | `identity_head → None` conflates never-declared with **revoked**; absent controller means implicit self-control — a revoked identity degrades silently to self-controlled. Plus C12, predicted not live: valueflows' `extensions_opted_in` widens disclosure on a self-asserted header — the translate layer already takes the bool; no resolver passes it until M2 wiring, which is the collision date. |
   | — | runners-up | C3; C1 | notary-authority (the active red): the REA face has no heal and no contest path — no legal move by construction; steward/node `consensus.rs` leader election — C1 at the seam where self-election is the definitional question, entirely unbound. |

   Calibration datum, honestly scoped: this shift's five waves introduced zero new
   concern classes *within its substrate family* — every wall was a known class at
   an unexamined seam. Classes are few and stable per family; seams are enumerable;
   their product is small enough to examine exhaustively — and each new family's
   admission re-measures the canon.

7. **SDK inheritance surface** — the concern architecture as something an external
   peer-runtime developer *receives*, not re-derives:
   - **Data artifacts, schema-first, no Python**: `seam-registry.schema.json`
     (in `elohim/sdk/schemas/v1/manifest/`) so a third-party node declares its own
     decision points and validates with standard tooling — the atlas's "a new app =
     a new manifest" move applied to peer runtimes; and
     `schemas/v1/registries/concern-classes.json` — the canon as
     `{id, statement, version, contentHash}`, **generated from both registry homes
     (policies.yaml + concerns.yaml), never hand-written** (the
     `.epr-meta/elohim/packages` projection pattern). The matrix
     generator reads the published projection, so an external consumer's matrix uses
     the same input.
   - **Wire honesty**: the `answer` envelope object + `answer-state`/`answer-reason`
     enums (surface 3) mean C4 survives the Rust→TS boundary instead of collapsing
     to null.
   - **Cross-repo cascade**: `CONCERN_CANON_VERSION` + per-class
     `PolicyPin { id, version, content_hash }` constants; a version bump ships the
     new pin **plus `#[deprecated]` on the old** — the external build emits a warning
     naming the exact policy that moved (the compile-time analog of a fingerprinted
     finding) — and `concern-canon-changelog.json` (`{policy_id, from, to,
     migration_note, affected_contract}`) lets an external census diff pins with no
     Claude tooling. Semver rule: tightening = minor (new pin, old deprecated);
     removing a pin = major; never silently redefine a pin's semantics.
   - **Publish honesty (C7 applied to ourselves)**: the plan must not *claim* an SDK
     surface while the crate family is unpublishable — `crates/elohim-sdk` today
     references a README that does not exist and carries no `publish` field.
     Prerequisites are explicit P1 tasks.
   - **Conformance harnesses for implementors**: shipped behind the default-off
     `harness` feature (one crate, one version); split a `-testkit` crate if and
     only if P2's Liveness table needs `proptest`/`arbitrary`. First external-facing
     conformance contract: `HeadRecordFetcher`'s prose contract ("None for EVERY
     failure mode… MUST NOT retry-loop") becomes `Answer<T>` — after a call-site
     audit confirms neutrality (P2, not P1).

8. **The residual channel — metal to dashboard.** The canon, matrix, and forecast
   govern the *anticipated*. A distributed, diverse fleet also guarantees the
   **unanticipated**: lags between EPR nodes, real-world interruptions, hydraulic
   failures upstream of every abstraction — expected inputs, not anomalies. (The
   Red Plenty parable is the type specimen: a dirt-mover's failed brakes destroy
   the one machine a whole planning system silently depends on — and because the
   plan had no channel for the unplanned event, reality re-entered the system as
   corrupted assumptions instead of as a report.) The residual channel is that
   organ, composed from mechanisms that already run rather than invented:
   **capture** (C14's context capsule at the seam — the metal end records the
   clocks, lags, and peer states because distributed causes arrive as stacks, per
   the five-defect arc) → **carry** (the capsule travels as evidence, never
   authority) → **collect** (the findings ledgers; `runtime-findings.jsonl` +
   runtime-triage is the live instance of this exact pipeline for *known*
   exhaustion classes — bind it, don't fork it) → **report** (residuals delivered
   into the developer-facing RCA surface beside the forecast — never parked at a
   log level the deployment drops) → **recover** (triage dispatch; the response to
   deviation is restored capability — Mishpat, never punishment — the same grace
   the protocol extends to people) → **learn** (each disposition feeds the
   calibration ledger; miss-of-canon is the canon's admission intake, so the
   fleet's **exception-metabolism rate** — novel residual → cure-or-named-class —
   becomes a first-class health measure beside uptime). Peers are there for each
   other in this like good-faith friends: the heal plane is already mutual aid
   (fills, salvage, carried records, witness bootstrap), C5's verification is what
   makes that good faith *safe to extend* — generosity without gullibility — and
   the residual channel extends the same friendship to failure itself: peers
   witness and carry each other's exception capsules so no node RCAs its own
   crash alone. This is the living-system counterweight to surface 6: forecasting
   narrows the unanticipated; the residual channel metabolizes what remains —
   learn, recover, exercise, evolve, grow, sustain in balance.

## Phases (each bounded, each independently verifiable)

**P0 — Canonize the concern classes.** Write the sixteen policies (versioned,
Precedent shape, guarantee form, recurrence/severity/fingerprint-namespace fields)
across the **two registry homes** (enforcement rows in policies.yaml only where an
evaluable predicate exists; `concerns.yaml` rows for the rest — new sibling per the
governors.yaml precedent) + the two-home probe split (runbook rows only for
live-metered classes). Verify: `.claude/scripts/memory-kit/placement-audit.py
--epr-meta` green; `validate_meta` accepts; the census reads both homes (a
concerns.yaml row without a consumer fails the census); the existing heal-fills-never-moves inline rule becomes a
binding of the C2 policy (bind-don't-redefine advisory disappears). *(process-meta)*

**P1 — seam-contracts crate, adopted behavior-neutrally at the convergence seam.**
The leaf crate (features, MSRV, wasm check, boundary test) with `Answer<T>`,
`ReasonLabel`, `Arbitrated` + `Quiescent` harnesses; first adoption at
**`LocalResolve`** (split 2026-08-02 with "behaves identically today" — provably
neutral; its 40-line CONTRACT DEVIATION comment becomes a type), then
`head_adoption` / `projection_reconcile`; second adoption at doorway's
`ReconcileDecision`. Typed `ContestFailure` replaces the stringly
`inc_contest_failed(&str)` (label strings unchanged ⇒ dashboards intact). SDK publish
prerequisites (README, `publish` fields, versioned path deps). Zero behavior change
(existing storage suite green — count re-verified at execution, not trusted; no
change to *existing* wire contracts — P1.4's schema additions are additive-only;
clippy baseline). *(protocol-canonical, D1)*

**P2 — Liveness harness.** Table-driven state-space check over `decide_head_action` +
admission predicates. Verify by regression demonstration against **two**
independently-dated historical predicate sets: the pre-wave-5 set (this shift's
deadlock) and the 2026-07-11 over-broad-GapFill set (the guard that froze
convergence) — FAILS on both, PASSES current — proving the harness generalizes
rather than being fitted to one deadlock. Then the `HeadRecordFetcher → Answer<T>`
retrofit (post call-site audit). *(protocol-canonical, D1)*

**P3 — Decision-point registry + census.** `seam-registry.schema.json` authored
first (the schema is the source of truth; per-crate registries conform to it; census
and matrix stay derived); `seam-registry.yaml` for the four seed crates; census check in
placement-audit (both directions; per-row degradation; fixtures proving
missing-registration and missing-contract-test each fail loud — and an **anti-mirror
fixture**: a contract test that computes both sides via the same helper is flagged,
because a passing test can be measuring the mirror). Optional pull-forward from P5:
extract `select_canonical_winner`'s ordering into the crate as a plain-data
predicate, zome becomes an HDK adapter — **coordinator-only, no DNA-hash move, ships
via `update_coordinators`**; do not defer it on reinstall fear. *(process-meta +
protocol-canonical)*

**P4 — Validators, cascade, matrix, forecast, birth rule, cross-repo canon.** The
three validators evaluated in the resolver; pin-keyed cascade with sentinel dispatch
(suppression/debounce per the measure tier); the matrix + seeded forecast +
calibration ledger; p2p-design-gate/compose-gate birth rule; and the cross-repo
canon artifacts (P4.6). Verify: fixture diff triggers the class-ladder verdict;
census worklist matches bound-point set; forecast rows carry fingerprints; an
external-consumer fixture build emits the `#[deprecated]` warning naming the exact
policy that moved. *(process-meta)*

**P5 — HELD (follow-up rung): brit residency + wider adoption.** Zome-side adoption
beyond content_store; steward/node + bridges seam registration (did-bridge first —
it has the trait seams and two confirmed-live cells); and the brit lift itself —
policies/registry rows/matrix snapshots become CIDv1 EPR atoms under declared heads
with Precedent standing (owned by the policy-registry spec's graduation trigger;
P0.4's liftability gate guarantees the lift is a move, not a rewrite). Not planned
here.

## Tasks

- [ ] P0.1 Author the sixteen concern-class policies (C0–C14, C6 split) across the two registry homes: enforcement rows in `.claude/epr-meta/policies.yaml` only for predicate-bearing classes; Precedent-shaped rows in the new sibling `.claude/epr-meta/concerns.yaml` for the rest (per the governors.yaml precedent; consumed by the census + canon projection — no phantom validators, no consumer-less rows). Each: statement / scope predicate / binding class / why / dispatch-agent / established_by / recurrence (distinct shifts) / severity_class / findings namespace. Pins via `epr-meta-pin.py --write`; stay under the 64KB fail-loud cap.
- [ ] P0.2 Two-home probe split: for the live-metered classes only (C6a, C7, C8 via `elohim_content_canonical_answers_total{tier}`, C11), extend the runbook's §1 invariant table (numbering continues I7+) AND its §2 probe table, in a clearly-scoped concern-canon subsection — the runbook is dataplane-scoped reference, so extend without rewriting I1–I6 and state the rescope in its header; design-time classes cite contract tests in the seam registry instead.
- [ ] P0.3 Convert the inline `heal-fills-never-moves` rule in `elohim/elohim-storage/src/.epr-meta` to a binding of the C2 policy: the rule's incident prose moves to the C2 policy's `why` (cross-family statement), the storage-specific placement survives as the binding's `when:` override + `why` (bindings carry no validator key), and the bind-don't-redefine advisory disappears (`placement-audit.py --epr-meta` green).
- [ ] P0.4 Brit-liftability gate: every artifact declares its standing shape (`record | attestation | fold | pin`). The LIVE pin mechanism is untouched — canonical-JSON `sha256` per `epr_meta.py`/`epr-meta-pin.py`; P0 changes no pin format (a mismatched pin converts every binding to ask-routing noise). Liftability is proven by FIXTURE only: the artifact's body re-addresses as CIDv1(dag-cbor, sha2-256) via the `epr::compute_cid` path without rewriting — a codec-wrapping proof owned by P5's lift. Identity is the *content* address (the entry-hash discipline — never the provenance/anchor hash); an unrecognized codec is an unverified claim, never a passing one (fail-closed).
- [ ] P1.1 Create `crates/seam-contracts` (published `elohim-seam-contracts`, lib `seam_contracts`): leaf crate, `default = []`, features `serde`/`ts`/`harness`, MSRV declared (highest already in-tree; verify), `#![forbid(unsafe_code)]`, wasm32 no-default-features CI check, `no_heavy_deps_in_dep_tree` lockfile test; `Answer<T>`, `ReasonLabel` + conformance test, `Arbitrated` (permutation-invariance) and `Quiescent` (settled-state fixed point, lazy-acceptance clause) harnesses; re-export from `crates/elohim-sdk` as `contracts`.
- [ ] P1.2 Behavior-neutral adoption, in order: `LocalResolve` (provably neutral — the CONTRACT DEVIATION comment becomes a type), `head_adoption` / `projection_reconcile`, doorway `ReconcileDecision` as second independent site. Full storage suite green, no wire change, clippy baseline unchanged.
- [ ] P1.3 Typed `ContestFailure` enum implementing `ReasonLabel` replaces `metrics::inc_contest_failed(&str)` — two raw literals plus a variable class at the other two call sites; derive the full variant set from the call-site audit. Label strings unchanged so `elohim_content_contest_failed_total{class}` dashboards keep working.
- [ ] P1.4 Wire honesty: `schemas/v1/objects/answer.schema.json` (monomorphic envelope, tag/content emission per the `DiversityHint` pattern), `answer-state` + `answer-reason` enums (no `_dna`), view CONVENTIONS **Rule 11** (dual-provenance absence must not be a bare nullable) + `schema_contract.rs` assertion; proving adoptions: `epr-pull-status`, render `FetchOutcome` (both already three-way ⇒ neutral).
- [ ] P1.5 SDK publish prerequisites: write `crates/elohim-sdk/README.md`, add `publish = ["elohim"]` to elohim-sdk + elohim-storage-client, version the path deps.
- [ ] P2.1 Implement the `Liveness` table harness (state-space enumeration; every non-terminal state has ≥1 enabled automated transition) over `decide_head_action` + admission predicates.
- [ ] P2.2 Regression demonstration ×2: harness FAILS against the reconstructed pre-wave-5 predicate set AND the 2026-07-11 over-broad-GapFill set; PASSES current.
- [ ] P2.3 `HeadRecordFetcher::fetch → Answer<CarriedHeadRecord>` retrofit after a call-site audit confirms all `None` paths are treated uniformly today; full-arc mapping: local miss ⇒ `Unreachable`.
- [ ] P3.1 Author `seam-registry.schema.json` (SDK schema family) then `seam-registry.yaml` for the four seed crates: elohim-storage, content_store zome, doorway-service, steward/node (every pure decision predicate + concern ids + contract-test citations).
- [ ] P3.2 Census in `placement-audit.py --epr-meta`: both directions; per-row degradation (one malformed row flags itself, never empties the set); fixtures for missing-registration, missing-contract-test, and the anti-mirror case, each failing loud.
- [ ] P3.3 (pull-forward, optional) Extract `select_canonical_winner`'s ordering into `seam_contracts` as a plain-data predicate; zome keeps an HDK adapter — coordinator-only, no DNA-hash move, lands via `update_coordinators`.
- [ ] P4.1 Register + evaluate the three validators (`heal-fills-never-moves`, `bounded-work` incl. retry-ladder-against-uncancellable-call scope, `dna-hash-neutrality`); drop every phantom `validator:` key (15 flagged 2026-07-25; 27 occurrences across 6 ids at ceremony time, including policies.yaml's own `epr:validator-eprfs-meta-domain-neutrality` — re-measure at execution); fixture diff triggers the class-ladder verdict.
- [ ] P4.2 Cascade: pin-keyed (never bundle/DNA-hash-keyed), one fingerprinted finding per stale pin, sentinel dispatch with measure-tier debounce; graduation events (C13 scaffold→successor) cascade over the scaffold's cells.
- [ ] P4.3 Concern × seam matrix generated by the census from the published `concern-classes.json`: rows C0–C14 (16 rows — C6 splits into C6a/C6b); columns = atlas §3 catalog grouped by §7 planes + governing/meta plane + bridges; track and reach-tier as cell attributes; cells conformant/variant/unexamined/n-a; ranking = recurrence × severity × rung-proximity; surfaced by `placement-audit.py --epr-meta` beside the Gap Ledger.
- [ ] P4.4 Trajectory-crossed forecast pre-seeded with the eight rows + runners-up above; fingerprinted rows; calibration ledger recording hit / miss-of-ranking / miss-of-canon on every spine flip or new red; census resolver spot-checked against disk before any ranking is trusted.
- [ ] P4.5 Design-time birth rule: bind p2p-design-gate + the compose-gate so a new decision predicate / boundary answer type / sync message / route cannot be born unregistered — its design answers each concern class or declares n-a.
- [ ] P4.7 Residual channel: a `ResidualWitness` capsule convention in `seam_contracts` (context sufficient for cross-node RCA — inputs digest, decision state, clocks/lags/peer-states; never a bare log line); capsule intake into the findings-ledger namespaces (bind the live runtime-findings + runtime-triage pipeline, don't fork it); residuals delivered on the developer RCA report beside the forecast; disposition wiring into the calibration ledger's miss-of-canon intake, with the exception-metabolism rate reported as a fleet health measure.
- [ ] P4.6 Cross-repo cascade artifacts: `CONCERN_CANON_VERSION`, per-class `PolicyPin` consts with `#[deprecated]` supersession, `concern-canon-changelog.json` (crate + `schemas/v1/registries/`), semver rule documented; `registries/concern-classes.json` generated from both registry homes (policies.yaml + concerns.yaml) with a freshness check.

## Verification track note

The claims in "the pattern is already emerging" and the precedent record are
code- and history-verified as of 2026-08-02 (this shift's diffs + this ceremony's
four-lens mining; commit hashes cited inline). P1/P2 adoption sites are live code
with green suites — any CLAIMED suite status is re-verified by ci-investigator at
execution time, not trusted. Two standing verification disciplines from the mining:
a passing contract test can be measuring the mirror (compute the two sides
independently), and a drift audit can be the dominant source of its own findings —
spot-check the measure before trusting the ranking.
