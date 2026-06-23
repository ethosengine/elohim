---
title: Contributor-Presence Bootstrap & the Who's-Who Knowledge Graph
id: contributor-presence-bootstrap-whoswho-design
status: Draft
class: protocol-canonical
domain: D2
topic: [contributor-presence, attribution, who-is-who, recognition, seeding, migration, valueflows]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
cites:
  - init-authoring-native-seeding-design | the shared seeding=init-through-the-front-door principle; this spec is its presence-layer sibling | sha256:c8efe09b9262401b | path: genesis/docs/superpowers/specs/2026-06-12-init-authoring-native-seeding-design.md
  - elohim-facings-crate-extraction-plan | the landed select->fold->aggregate crate the Wave-2 reflexive aggregator composes as a new lens | sha256:d301f34b3b7e66d4 | path: genesis/docs/superpowers/plans/2026-06-19-elohim-facings-crate-extraction-plan.md
  - rea-economic-facing-lens-design | the REA facing-lens pattern the contributor-reflexive view follows (not the unbuilt fold) | sha256:b83ead21be13bbaa | path: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
  - genesis/plans/2026-03-13-recognition-pipeline-plan.md
  - genesis/data/presences/presences.schema.json
  - .claude/data/contributor-presence-whoswho-grounding-2026-06-21.md
  - .claude/data/contributor-cohort-seed-source-2026-06-21.md
refines:
  - genesis/docs/superpowers/specs/2026-06-12-init-authoring-native-seeding-design.md
---

# Contributor-Presence Bootstrap & the Who's-Who Knowledge Graph

## The reframe (what grounding corrected)

The "distributed-EPR CONTRIBUTORS file" we set out to build is **~80% already wired** — under the
name **presences**, not "contributors." The bootstrap convention exists end-to-end:

- **Authorable manifest**: `genesis/data/presences/*.md` (127 files), YAML frontmatter,
  `presenceType: person | organization`, schema-governed (`genesis/data/presences/presences.schema.json`),
  validated (`genesis/seeder/src/validate-presences.ts`), generated to `presences.json`
  (`build-data.ts`).
- **Front door**: `genesis/seeder/src/seed-presences.ts` POSTs each to `/db/presences`
  (idempotent on 409) — this *is* "seeding = init through the front door" (the `init-authoring`
  principle), already applied to presences.
- **Persistent target**: the `ContributorPresence` DHT entry type
  (`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:499`; mirror
  `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:1215`) →
  storage projection (`elohim/elohim-storage/src/db/contributor_presences.rs`) →
  `ContributorPresenceView` (`elohim/elohim-views/src/imagodei.rs:82`). Lifecycle
  `unclaimed → stewarded → claimed`. **"Recognition before registration."**
- **Consistency**: the `humans-presences-sync` triple (`.claude/file-relationships.json:143-162`)
  already governs humans ↔ presences ↔ content-citations bidirectional coherence.

**Therefore the deliverable is NOT "build the machinery." It is: instantiate the cohort, add the
small net-new pieces, and compose the read-side.** This spec is a **sibling** of `init-authoring`
(shared front-door principle) and a **consumer** of the landed `elohim-facings` crate.

### The two layers (the operator's framing, confirmed on-grain)

1. **Bootstrap layer (build now)**: the presences `*.md` manifest is the git-CONTRIBUTORS-shaped,
   reviewable artifact. Seeding it = `init` through the front door — it creates *real* entries, not
   throwaway fixtures.
2. **Persistent layer (later, mostly wired)**: the `ContributorPresence` DHT entry the seed flows
   into; accrual, claim/opt-out, and the reflexive view grow on top.

### The collective-as-attractor model (a wired mechanic, not a metaphor)

`presenceType: organization` already exists. "Google-Maps-crowdsourcing → an attractor" is the
**literal** `ContributorPresence` mechanic: anyone creates the placeholder pin for an absent
person/collective → it accrues recognition while unclaimed → the real entity **claims** it and
receives the accrued recognition. This is **recognition-before-registration as a growth mechanism**,
and it is the engine behind the eventual who's-who skill and the Canteen claim-invites. For
collectives the pull is amplified: claiming one presence draws a whole group onto the network.

## The standing gradient (replacing the real/fixture wall)

A hard `real/fixture` boolean is the one binary in a substrate that deliberately refuses binaries
about people (graduated stewardship; `unclaimed→stewarded→claimed`; the `private→…→commons` reach
gradient). Instead, a **non-notarized, seed-layer `standing` field** (Category C) names the
*provenance/origin* of the presence:

| standing | meaning | example |
|---|---|---|
| `inspirational` | cited in the vision; honored, no dependency, no contact | the elohim.host "Inspired by" cohort |
| `prior-art` | code/ideas we actually compose | Khan/Perseus, Holochain, libp2p, iroh, Automerge, hREA, gitoxide, Eclipse Che |
| `interlocutor` | engaged-real (a confirmable channel exists) | Jordan Swim (commented); Canteen / Stephen Lewis |
| `operator-persona` | dev/test persona — **this is where real-vs-fixture safety lives, as a tier** | the adam + matthew bootstrap pair, household personas |

**The hard edge moves off the *person* and onto the *consequence*.** `standing` is orthogonal to
`presence_state` (which is DNA-notarized and validator-locked — it must NOT carry realness). The
**two-layer rule** governs behavior:

- **Layer 1 — witnessing & recognition: ungated, internal, reversible.** Accrues for *every*
  standing, including `operator-persona`. Generosity is the safe default — no flag needed to honor.
- **Layer 2 — value receipt (claim / opt-out→commons) + outbound claim-invite: the only
  irreversible legs.** Gated on a **confirmable fact** (a real contact channel / a claimed agent
  key), not on the `standing` label. You cannot email a fixture — it has no channel — so the harm
  (minting real commons value, billing a real person for a test persona's accrual) is prevented by
  the *absence of a real receiver*, not by a wall. The gate moves forward as the relationship
  matures — exactly the prototype→production journey.

## On-grain "initial allocation" (the grain correction)

In this substrate **recognition accrues from witnessing — it is not granted by fiat.** The create
path hard-zeros recognition (`db/contributor_presences.rs:230`; `NewContributorPresence` leaves the
numeric fields to DB DEFAULT 0; the TS seeder's `recognitionScore`/`affinityTotal` fields are
**dead** — they never reach the DB). And **reach attaches to content, not to a presence** —
"initial reach for a contributor" is a category error.

So "honor them with an initial allocation" is expressed **on-grain** as:
1. **Seeded citation edges** — wire each presence as `source_of` / `derived_from` the content it
   inspired (`establishingContentIds` / `observations[].contextContentId`), with the elohim.host
   manifesto as the establishing content. Recognition then accrues along these edges through the
   wired path.
2. **(Optional) seed-time `StewardshipAllocation` rows** — the `account-package` import precedent
   (`elohim/elohim-storage/src/http.rs:8658`) creates these at seed time (content_category +
   allocation_ratio + contribution_type). This allocates *stewardship of content* (who accrues when
   content is engaged), which is the legitimate seedable quantity.

A deliberate seeded recognition *balance* remains net-new and is **out of scope** — it fights the
grain.

## Wave plan

### Wave 1 — Bootstrap the inspired-by cohort (NOW)
- [ ] Add an additive, optional `standing` field to `genesis/data/presences/presences.schema.json`
      (enum: inspirational · prior-art · interlocutor · operator-persona). Existing 127 files
      validate unchanged.
- [ ] Confirm `seed-presences.ts` threads `standing` (top-level extraction, mirroring
      `establishingContentIds`) or rides it via the `metadata` blob; pick the zero-risk path.
- [ ] Backfill `standing` on the existing 127 presences (default `inspirational` for cited
      content-inspirers; `operator-persona` for the dev/test personas; `prior-art` for the
      library presences).
- [ ] Author the elohim.host "Inspired by" cohort as presence `*.md` files
      (`.claude/data/contributor-cohort-seed-source-2026-06-21.md`): ~11 `person` + ~30
      `organization`, each with `externalIdentifiers`, `standing: inspirational`, and a `source_of`
      edge to the manifesto establishing content. Skip the 3 that already exist (P2P Foundation,
      Holochain, ValueFlows) — backfill `standing` on those instead.
- [ ] Author the `prior-art` technical-dependency cohort (Khan/Perseus, libp2p, iroh, Automerge,
      hREA-as-software, gitoxide, Eclipse Che) — the ones the grounding found absent.
- [ ] Validate via `validate-presences.ts`; confirm the `humans-presences-sync` triple stays
      coherent (observer/steward refs resolve to humans).
- [ ] Seed through the front door and verify projection rows land.
- [ ] a2o scenario: `genesis/a2o/features/<pillar>/contributor-presence-seeded.feature` — a seeded
      inspirer appears in the who's-who graph with its establishing-content edge.

### Wave 2 — Reflexive ValueFlows aggregator ("how the network sees a contributor") [after accrual]
- [ ] New fold `elohim/elohim-facings/src/folds/contributor_reflexive.rs` (DB-free mirror row over
      `economic_events` by `contributor_presence_id`; reuse `bucket_by`/`distinct_count_by`; follow
      `folds/epr_content.rs` as the concrete pattern, NOT the unbuilt rea/reach folds).
- [ ] `ContributorReflexiveView` (additive, ts-rs) in `elohim/elohim-views/src/imagodei.rs`;
      *assemble* the already-folded presence accumulators (`recognition_score`, `citation_count`,
      `affinity_total`, `unique_engagers`) rather than re-folding.
- [ ] Storage loader `services/contributor_reflexive_facing.rs` + one GET route with **both** the
      match arm AND an `is_service_path` arm + a routing unit test (EPR-router-shadow trap).
- [ ] Defer the reach-back-prop-to-inspirers fold (a graph traversal over provenance edges, not a
      flat fold) and the VF-GraphQL projection (the bridge is M1/fixture-only; reports are M5).

### Wave 3 — Opt-out → commons (heavy; net-new) [DEFERRED]
> **Status: DEFERRED (operator 2026-06-22 — "Wave 3 can wait").** Behavior documented below as the
> intended design; **not scheduled**. It is DNA-notarized/heavy and gated on a canonical
> commons-receiver agent. Revisit after the two visual-surface sprints (presences-on-EPR · imagodei
> profile) that consume Waves 1–2.
- [ ] Define a canonical **commons receiver agent** (`agent_cid`) — replace the `commons-pool` magic
      string (cross-namespace identity rule).
- [ ] Model opt-out as a **relinquish action** that emits a commons-redirect `EconomicEvent`
      (action=`transfer`, receiver=commons agent) on the recognition/claim path — the existing shape
      (`route_economic_event_under_collab`) is the template; the recognition path has no commons
      branch today.
- [ ] Decide whether an `opted-out` marker is a separate record (preferred) or a new
      `PRESENCE_STATES` value (a **DNA-hash-moving** integrity change — avoid if possible).
- [ ] Do NOT route opt-out through `FeedbackSignal` (category mismatch — it signals about *another*
      EPR's standing).
- [ ] Layer-1 recognition stays untouched (opt-out declines *receipt*, never un-sees the person).

### Wave 4 — Symmetric export leg (the missing migration half) [later]
- [ ] Build the export path that serializes accrued recognition state back to presence frontmatter
      (today: import-only; presences are portable because agent-key-free until claimed, but nothing
      serializes accrual back). Account for the lossy projection (drops `endorsements_json`,
      `invitations_json`, etc.) and the cross-layer numeric divergence.
- [ ] This completes the "import/export schema for migrating elohim protocol networks" framing.

### Wave 5 — The who's-who networking skill (the 4th lens) [culminating]
- [ ] A `who-is-who` skill = **consult** (query presences + traverse `derived_from`/
      `establishing_content_ids` from an EPR/path to its source presences) **+ credit** (accrual
      rides the wired path; receipt resolves via claim or opt-out→commons). Sits alongside the
      builder trio (`atlas-grounding` · `concept-mapping` · `app-port`). `concept-mapping` feeds it;
      `app-port` should name the prior art it leans on.

## Decisions locked (this session)
1. Seed into the canonical `genesis/data/presences/*.md`; the 9-file `lamad/presences/*.json` is
   divergent legacy to reconcile, not extend.
2. "Initial allocation" = seeded citations (+ optional `StewardshipAllocation`), **not** a fiat
   recognition balance.
3. Wave 1 is the scope-now; Waves 2–5 are named follow-ons, not a flat build.

## Drift corrections to the prior who's-who grounding doc
- All Rust paths need the `elohim/holochain/dna/` and `elohim/` prefixes (the prior doc dropped
  them).
- Lifecycle: DHT has 3 states; storage has a 4th transient `claiming`.
- `opted-out` is genuinely absent (confirmed net-new). `commons-pool` is a magic string, not an
  agent.
- "flows to commons = reach=commons" is **wrong**: reach=commons is content *visibility* (ordinal 8);
  the commons here is a value *receiver*.
- Cohort-1 seed list was aspirational: only P2P Foundation, Holochain, ValueFlows exist as
  presences; the rest is net-new authoring.
