---
title: "Contributor-Presence Commons Stewardship — witnessed residents, commons-held value, claim as negotiation"
id: contributor-presence-commons-stewardship
tier: spec
status: Draft
created: 2026-07-21
maintainers: Matthew Dowell + Claude Fable 5
class: protocol-canonical
topic: [contributor-presence, witnessed-ascription, commons-stewardship, claim-negotiation, identity-coherence, household-resilience, attestation, rea-events, lifecycle, fixture-humans, settlement-deferral]
context-tier: disclosed
sovereignty-frame: descriptive
steward: rust-architect
graduation-trigger: decompose-complete OR stage-2-witnessed-holder-fold-shipped
domain: D2
informed-by:
  - genesis/data/presences/presences.schema.json
  - genesis/a2o/features/dataplane/resilience-identity-coherence.feature
refines:
  - genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md
cites:
  - contributor-presence-bootstrap-whoswho-design | the presence bootstrap this spec extends — presences.json pipeline, standing gradient, recognition-before-registration; this spec adds the witnessed posture + commons stewardship + claim-as-negotiation on top | sha256:0b72f9cec8821810 | path: genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md
  - identity-head-key-lineage | supplies the chain-root primitive claimed_root anchors to (§4.2 re-anchors ClaimedAgentToPresence on the identity chain-root so a claim survives rotation) and the controller ontology guard | sha256:95950b918c8803bc | path: genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
  - attestation-consolidation-design | the Content-entry attestation convention WitnessedAscription reuses — attestation is a content_type discriminator, not an imagodei entry type; new kinds are manifest declarations, zero new DHT entry types | sha256:220c0a2a68c2a805 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - epr-rea-valueflow-fabric | the REA event plane settlement execution lands into — custody, flows, and corrections are append-only economic events walked by the process plane, which is what makes deferred valuation retroactively computable | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  - coherent-transport-identity-resolver-design | the identity-coherence stopgap (§3.4) whose live invariant this spec evolves from household_id implies agent_pub_key to household_id implies agent-key-or-witnessed-presence | sha256:63117b359cfa3891 | path: genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md
  - stewardship-over-sovereignty | the canon grounding the three-roles ontology — the commons pool holds standing in trust, mediated agency backstops the claimant; sovereignty is never the apex tier | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md
---

# Contributor-Presence Commons Stewardship

*Conductor-less residents of the substrate are re-grounded as **ContributorPresences** — witnessed
by real agents, their value stewarded by the **elohim-commons**, claimable later through a
**negotiated settlement** that executes in the standing floor/ceiling/flow shape. The design's core
discipline: **record facts, defer valuation** — every schema choice must leave every future
settlement interpretation retroactively computable.*

## 1. Problem — fixture humans claim presence; the orphan is ambiguous

The dataplane resilience-identity-coherence invariant (`household_id ⇒ agent_pub_key`,
`genesis/a2o/features/dataplane/resilience-identity-coherence.feature`) currently reds on **11
fixture humans** seeded with a `household_id` and a NULL `agent_pub_key`. They are mis-modeled:
a `humans` row asserts "an agent in the network," but these residents have no conductor and no
key — the test bench cannot host their conductors, and that is not a defect of the bench, it is
the *normal condition of a person not yet in the network*. Grandma's neighbor who contributed a
recipe, the open-source author whose library the commons relies on, the child not yet graduated
to her own key — all are **presences**, not agents.

Because the fixture default and the genuine defect share one shape (household-placed,
key-less), the orphan class is **ambiguous**: the invariant cannot distinguish "mis-modeled
resident" from "membership projection dropped the agent key" (the all-zeros resilience card root
cause). Re-grounding the 11 as presences makes the orphan a *detectable defect class* again.

The substrate already carries most of the machinery, in two disconnected halves:

- **DHT half (built, orphaned).** `ContributorPresence` is a real imagodei entry type
  (`elohim/holochain/dna/imagodei/zomes/imagodei/src/imagodei_integrity — lib.rs:501-544` entry
  definition, `:894-904` validation) with full coordinator functions:
  `create_contributor_presence` (coordinator `lib.rs:1748`), `begin_stewardship` (`:1824`),
  `initiate_claim` (`:1938`), `verify_claim` (`:1989`), plus anchors and links. **Nothing calls
  it**: elohim-storage never zome-calls the presence coordinators.
- **Storage half (live, DB-only).** elohim-storage runs a complete parallel implementation:
  `contributor_presences` table (`migrations/2026-01-08-000000_initial/up.sql:303-333` — whose
  header comment "Source of truth: DHT. Classification: A" is **aspirational, false today**), a
  4-state machine `unclaimed → stewarded → claiming → claimed`
  (`db/contributor_presences.rs:346,384,430`), a working claim flow with recognition transfer at
  verify (`:446-457` — a numeric copy, no double-entry, no source decrement), and `claimed_root`
  chain-root indirection via `identity_root_cid` (`:400-406`). No signal bridge exists;
  `dht_anchor_hash` is NULL on every row (`api/presence.rs:129-130` TODO).

And the presences **are already seeded**: `genesis/data/presences/presences.json` (158 entries,
generated from `genesis/data/presences/*.md`; schema `presences.schema.json`) flows through
`seed-presences.ts` (POSTs to `/db/presences`, idempotent on 409) in CI stage
`seed-presences.sh`, ordered after `seed-humans` and before `seed-accounts`. The bootstrap this
spec proposes is **the existing front door**, not a new pipeline.

## 2. P2P design gate output

1. **Entity class.** `ContributorPresence` is **A (notarized) as target** — the DHT entry type
   exists and is the declared source of truth; today it *runs* as a C-shaped DB projection (the
   split-brain debt, §7). This spec does not heal the split; it records the target class so no
   stage builds further away from it.
2. **DHT entry type exists?** YES, twice over. `ContributorPresence` is wired in imagodei
   integrity (headroom untouched — zero new entry types in this design). **WitnessedAscription is
   NOT a new entry type**: attestation is a Content-entry *convention* on the elohim DNA —
   `content_type = "attestation:<subtype>"`, issued via `issue_attestation`
   (`elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs:44-160`), with 23
   manifest-declared kinds validated at issue (`generated_attestation_kinds.rs`) and `subject_cid`
   a free string. WitnessedAscription = **a new attestation KIND**,
   `attestation:witnessed-ascription`, declared in the imagodei pillar manifest
   (`elohim/sdk/domains/imagodei/manifest.json` `attestations` map, following the
   `attestation:humanness` shape) with a subtype metadata schema
   (`elohim/sdk/schemas/v1/attestation/subtypes/witnessed-ascription-metadata.schema.json`,
   modeled on `stewardship-grant-metadata.schema.json`). The manifest's `subject_kinds` field is
   documentation-only (never enforced at issue) — noted as a debt, not silently relied on.
3. **Identity.** Presence identity is the **slug** — a justified slug exception: a presence is a
   *pre-existence identifier* for a person who has no key and no content yet; the slug is the
   address others cite (`presence-donella`), stable from seed through claim. The
   WitnessedAscription is **agent-scoped composite**: (steward agent, presence slug,
   ascription_type) — a real agent authors *about* the presence, signed with their own key, so
   the "identity is theirs from day one" discipline of `affirm_membership` stays intact and
   mediated agency (guardian co-authors) generalizes. At claim, `claimed_root` anchors to the
   claimant's **identity chain-root cid** (never a raw key), so the claim survives every
   rotation (`identity-head-key-lineage` §4.2).
4. **Coordinator fn / signal.** The coordinator surface already exists
   (`create_contributor_presence`, `begin_stewardship`, `initiate_claim`, `verify_claim`);
   ascriptions issue through `issue_attestation`. **No presence signal bridge exists** — storage
   projects attestations via signal + table (`signals.rs:786-796`) but presences have no
   projection path from the DHT; recorded as debt (§7), with the a2o invariant (below) as the
   forcing function that keeps it visible.

**Holder split (the observable).** The household-resilience holder relation splits **verified**
holders (agent_pub_key present — a live key joined to a peer) from **witnessed** holders
(presence-backed, ascribed by a steward). The a2o invariant evolves:

> `household_id ⇒ (agent_pub_key ∨ witnessed-presence)` — zero orphans.

A household-placed row with neither is once again *exactly one bug* (the membership-projection
key drop), never a fixture default.

**Ontology guard (imago-dei, structural).** Mediated agency, never a sovereignty apex: the
presence is held *for* the person by the commons and witnessed *about* the person by real
agents; the claimant steps into a steward web, not out of one. `sovereignty-frame: descriptive`.

## 3. Three roles — witness, commons steward, claimant

Operator-steered (2026-07-21, load-bearing): **three roles, not two.**

| Role | Who | What they hold |
|---|---|---|
| **Witness** | A real agent (household steward, contributor peer) | **Attests + custodies**: authors WitnessedAscriptions about the presence with their own key; may custody shards of the presence's data. Witnesses MAY earn incentive for holding commons shards — but what that incentive is worth is a settlement question, deferred (§8). |
| **Commons steward** | The **elohim-commons** — a commons pool | **Stewards the value**: all data and value flowing through a presence node is held by the commons *in trust for the eventual claimant* — never by the individual witness. The witness sees; the commons holds. |
| **Claimant** | The person the presence is about | **Steps in**: claiming is "I step into my stewardship of all this" — an Intent that opens a negotiation (§4), not a withdrawal from an account. |

Separating witness from value-steward is what prevents witnessing from becoming capture: the
agent who attests never becomes the owner of what accrued. The commons pool is the steward of
record between seed and claim — and again after death (§5, Phase 3).

## 4. Claim = Intent opening a negotiated settlement

A claim is **never an atomic transfer**. Filing a claim is an REA **Intent** that convenes a wide
review + negotiated settlement with the tending stewards. The motivating example: a critical
open-source contribution (git history → brit content-addressed attribution) the commons has
relied on for years — the backfill negotiates against the human's needs, desires, and limits,
bounded by the constitutional frame. **Dignity floor and accumulation ceiling are boundary
conditions on the negotiation, not the answer.**

The system CONVENES and WITNESSES the settlement — then execution happens, in the standing
three-layer shape:

- **Deterministic floor** — mechanical legs execute mechanically: identity binding (membership
  stamp), presence state transitions (`stewarded → claiming → claimed`), `claimed_root`
  anchoring to the chain-root, event emission. Never judgment at the floor.
- **Elohim ceiling** — judgment and ceremony: the convening, the negotiated backfill, appeals —
  agentic inference calls, judgment ceremonies. Never a computed payout at the ceiling.
- **Execution into the flow** — the settlement lands as **REA events** in the valueflow fabric.
  Append-only: **corrections are new events**, which is what makes every settlement appealable
  and renegotiable *by construction*, continually re-evaluated within the steward web the
  claimant exits INTO (the Mishpat negotiated-consequences frame — restored capability, never
  punishment).

Settlement **semantics** stay uninterpreted (§8): the design records who witnessed, what the
pool held, what custody events occurred, what flows moved — and defers what any of it is
*worth* to emergent liquid consensus in the constitutional values system.

## 5. Lifecycle phases 0–3 — gertrude joins, lives, dies

The bootstrap mandate: model the lifecycle as **a2o forcing functions, within genesis**. The 11
bench presences resolve via the presence model itself (no conductor needed — that IS their
resolution). Plus **one live human lifecycle on shem** (+1 conductor, in resource budget):
**gertrude**.

| Phase | Story | Deterministic now | Ceremony-stub |
|---|---|---|---|
| **0 — joins (as presence)** | gertrude exists as a commons-stewarded presence, witnessed by her household steward; zero orphans | presence rows, ascriptions, holder split, invariant | — |
| **1 — claims** | conductor provisioned on shem; claim Intent filed; convening convened + witnessed; floor stamps identity binding; presence → claiming → claimed; claimed_root chain-root-anchored; her household count flips witnessed → verified; backfill recorded as events | state machine, membership stamp, chain-root anchor, card flip, append-only events | convening; negotiated backfill |
| **2 — lives** | recognition and economic standing accrue to the claimed presence; one appeal corrects a settlement by appending events — the original is never mutated | accrual, correcting-event append | the appeal judgment |
| **3 — dies** | key retirement via rotation lineage; standing returns to the commons-stewarded **witnessed** posture; legacy negotiated | key retirement, posture return | legacy negotiation |

Presence is the **symmetric cradle-AND-grave posture**: you enter the network through the same
door you leave it through, per the cradle-to-grave canon.

**Ceremony-stub convention.** Scenarios assert the ceremony was **convened**, was **witnessed**,
and **produced recorded events** — never outcome content. Deterministic assertions around the
stubs are e2e-checkable now; the ceiling's judgment stays unasserted until it exists. This is
how story-first proceeds without pretending the constitution is written.

## 6. Staged plan

1. **Stage 1 — seeder bootstrap (fixture residents become presences).** Seed the 11 fixture
   humans as presences through the EXISTING pipeline (`genesis/data/presences/*.md` →
   `presences.json` → `seed-presences.ts` → `/db/presences`, CI `seed-presences.sh`), state
   `stewarded` (commons-stewarded), plus steward-authored `attestation:witnessed-ascription`
   attestations for their household residency. Remove their bare `humans` rows or mark them
   presence-backed — a household-placed `humans` row with no key and no backing presence must
   stop being seedable.
2. **Stage 2 — storage fold: the holder split.** The household-resilience holder relation splits
   verified (agent_pub_key) vs witnessed (presence-backed) holders; the a2o invariant evolves to
   `household_id ⇒ (agent_pub_key ∨ witnessed-presence)`, zero orphans. Phase-0 feature
   un-pends here.
3. **Stage 3 — claim ceremony stubs.** Wire the deterministic floor of the claim
   (intent → claiming → claimed, membership stamp, claimed_root chain-root anchor) with
   convening/backfill as ceremony-stubs; un-pend the Phase-1 scenarios that assert
   convened/witnessed/events-recorded.
4. **Stage 4 — gertrude, live.** +1 conductor on shem; run the Phase 1–3 lifecycle as live legs
   (`@requires:shem`); card flips witnessed → verified on claim.
5. **Stage 5+ — debts (§7).** Split-brain heal (storage zome-calls the orphaned presence
   coordinators; dht_anchor_hash stamps), presence signal bridge, double-entry recognition
   transfer, route drift fixes.

## 7. Known-debts inventory (recorded, not fixed here)

1. **Storage/DHT presence split-brain.** The imagodei presence coordinators are orphaned;
   storage runs the parallel DB-only implementation; the migration header's "Source of truth:
   DHT. Classification: A" is aspirational. Healing = storage zome-calls
   `create_contributor_presence` / `begin_stewardship` / `initiate_claim` / `verify_claim` and
   stamps `dht_anchor_hash` (`api/presence.rs:129-130` TODO).
2. **Angular route drift.** `presence-api.service.ts` calls `/steward` and `/verify` where the
   backend serves `/stewardship` and `/verify-claim`; its `state` query param vs the backend's
   `presenceState` silently no-ops filtering.
3. **Stewardship-allocation seeding stopgap.** Seeding uses `human_id` as
   `steward_presence_id` (`http.rs:9848-9887`) — a placeholder identity conflation to unwind
   once fixture residents are presences.
4. **No presence signal bridge.** Attestations project via signal + table
   (`signals.rs:786-796`); presences have no DHT→storage projection path.
5. **Recognition transfer lacks double-entry.** `verify_claim`'s transfer
   (`db/contributor_presences.rs:446-457`) is a numeric copy with no source decrement and no
   event pair — must become REA events before any settlement semantics could ever be layered on
   it (§8's retroactive-computability constraint already fails on this one surface).
6. **`subject_kinds` is documentation-only.** Attestation kind manifests declare subject kinds
   but issue-time validation never checks them; `subject_cid` is a free string — the
   witnessed-ascription kind leans on convention until that gate exists.

## 8. NON-GOALS — settlement semantics are deliberately uninterpreted

- **Record facts, defer valuation.** Full provenance — witness, pool, custody events, flows —
  lands as immutable REA events such that **ANY future constitutional settlement story is
  retroactively computable**. A schema choice that *forecloses* a settlement interpretation is
  wrong; one that merely doesn't *implement* one is right.
- **No constitution, no settlement agents, no rates, no distribution rules.** Witness incentive
  for shard custody is acknowledged as a question and answered by no field in this design;
  settlement resolves through emergent liquid consensus(es) in the elohim constitutional values
  system, which this spec does not define.
- **Boundary conditions only.** Dignity floor and accumulation ceiling constrain negotiations;
  they are never encoded as the settlement function.
- **Appealability is structural, not semantic.** Append-only corrections guarantee settlements
  can be revisited; the design says nothing about when they *should* be.
- **Memorial-crystal horizon — marked, NOT designed.** The imagined absolute end state (a
  memorial compaction: the protocol compacting the witness of a whole life from the commons her
  data returned to, replayable in memoriam) is noted so nothing built here forecloses it —
  record-facts + content addressing + commons-return-at-death already point there. Do not build
  toward it.

## 9. Open questions

1. **Presence slug ↔ human id collision discipline.** Stage 1 must decide whether a fixture
   resident's presence slug equals its old `humans.id` (smooth for joins, risks conflation) or
   is minted fresh with a mapping table (clean, more churn). Leaning fresh-slug + mapping.
2. **Ascription revocation.** Attestation kinds have granted/revoked signals — does revoking the
   only residency ascription return a presence to orphan (defect) or to bare commons-stewarded
   (legitimate)? Needs a rule before Stage 2's zero-orphans gate can be strict.
3. **Witness plurality.** One steward ascription suffices for Phase 0; does claim verification
   (Stage 3) want a witness quorum, mirroring `required_witness_count` in recovery?
4. **Commons pool representation.** Is the elohim-commons a `Collective` (group-controlled
   identity head — the wired qahal primitive) from day one, or a well-known slug until the
   first settlement ceremony needs it to act? Leaning Collective-from-day-one since it is
   already wired and avoids a later identity migration.
5. **Death trigger.** Phase 3's "key retirement via rotation lineage" — is death a
   `KeyRotation` to a tombstone controller-policy, or a distinct lineage terminator? Defer to
   the identity-head arc, but the a2o story should not encode either shape prematurely.
