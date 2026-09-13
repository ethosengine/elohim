---
title: "Served under standing — failover finished on both planes, the doorway's commons standing measured, hostnames become heads"
id: served-under-standing-sprint
status: Draft
domain: D8
sprint: "2026-09-12 operator sprint — ratchet of five rungs, one named red each; drains doorway-failover, blob-durability, dataplane-convergence; births served-under-standing"
cites:
  - doorway/doorway-service/.epr-meta/served-under-standing.habit.md
  - doorway/doorway-service/.epr-meta/doorway-failover.habit.md
  - elohim/elohim-storage/.epr-meta/blob-durability.habit.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - app/elohim-app/.epr-meta/epr-atom-home.habit.md
  - genesis/a2o/features/dataplane/served-under-standing.feature
  - genesis/a2o/features/federation/name-routing.feature
  - genesis/a2o/features/resilience/chaos-peer-churn.feature
  - genesis/a2o/features/dataplane/doorway-apex-transition.feature
  - "doorway-federation-failover-sprint-plan | Doorway Federation & Failover Sprint | sha256:c66fd04c3b4f16e2 | path: genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md"
  - "doorway-federation-three-reds-to-green-plan | Doorway federation | sha256:d2b8f066817b690f | path: genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md"
---

# Served under standing — the sprint

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Every rung names the red it moves and the run id or build number that moves it. Never push while a fleet roll is in flight; one push per batch; a habit atom delta is the deliverable.

**Goal:** finish failover on both planes on the household mesh and the fleet, give the doorway's commons standing a measured habit (born red 2026-09-12), and begin the topology transition in which a hostname is a head channel served by every doorway rather than a doorway's identity.

**Design source:** the four-pass operator ruling of 2026-09-12 under WS3 of the 2026-07-31 failover plan (parked branch `sprint/apex-multi-a-ingress`): doorways are the federated load balancer at the WAN addresses; a public name is a DHT fold of projection contracts × reach × requester standing × liveness; multiple projections are the CDN tier; a doorway's public projection is a commons standing the public can challenge. Hosted-human cost is anti-capture pressure, not a bottleneck; doorways are a bridge toward device stewardship.

**Ratchet:** five rungs, each disjoint in write set, each moving one named red. Rungs 1 and 2 run in parallel in isolated worktrees on the sprint cargo slots; rung 3 follows the habit; rung 4 follows a design-gate pass; rung 5 is the fleet and waits on the operator's ingress classes.

---

## Rung 1 — Storage: the three chaos reds (blob-durability · dataplane-convergence)

Evidence today (runs 20260912T145758Z, 20260912T152941Z): custody rows converge but their `state` does not travel on the wire; `humans.agent_pub_key` has no reconcile arm; `onlinePeers.live` reads a 900 s heartbeat so a killed peer stays live. Write set: `elohim/elohim-storage` only, branch `sprint/2026-09-12-served-under-standing-storage`.

- [ ] **Step 1: custody state on the wire** — `CommitmentWireFields` carries `state`; `project_commitment_from_wire` threads it; insert paths stop hardcoding `proposed` when the wire carries a state; the `rea_commitments` arm's divergence test compares state and re-heals already-wrong rows; cross-peer test. Evidence: gate EXIT lines + the chaos flapping drill's settled-record step green on the mesh.
- [ ] **Step 2: a `humans` reconcile arm** — divergent agent-key bindings converge to the DHT's current binding; InSync / KeyDivergent / Missing classified and counted; reuse `membership_identity_reconcile` as the write path. Evidence: cascade Given seeds 3 and the fold counts 3.
- [ ] **Step 3: liveness from the connected-peer view** — `onlinePeers.live` reflects the p2p connected set (ping timeout ≈35 s), `known` stays the heartbeat join; no wire change. Evidence: simultaneous-loss drill reads at-risk within the ping timeout.
- [ ] **Step 4: deploy and measure** — feature-full bin from the sprint slot pinned under the scratchpad, `just mesh storage-restart`, `just test mesh features/resilience/chaos-peer-churn.feature` and `features/resilience/doorway-footprint-convergence.feature`; deltas in both storage atoms with run ids.

- [ ] **Step 5: custody is authored by the provider's own peer** — the drill-custody seeder, the spool seeder and the chaos drill POST and activate each pair against the provider's own storage peer (never another peer's doorway); the drill's idempotence gate uses the assertion's own predicate. Evidence: a clean cold start's prologue leg seed-drill-custody EXIT=0 with 9/9 activated; the cascade Given seeds 3 and the fold counts 3.
- [ ] **Step 6: one root per commitment id (coordinator zome)** — `create_rea_commitment` returns the existing root under a commitment_id anchor instead of minting a second; DNA hash unchanged; hot-swapped onto the household mesh by `hc-mesh.sh coordswap`. Evidence: two callers with one id yield one root; the forked ids never recur.
- [ ] **Step 7: the doorway never masks a permanent error as backpressure** — a storage 503 without Retry-After/X-Available-Permits is relayed verbatim (1df5ae3a2). Evidence: the seeder fails fast with the storage sentence instead of 12 retries.

## Rung 2 — Doorway: name routing (served-under-standing · doorway-failover)

Evidence today: `fetch_from_remote_doorway` is defined and never called; a doorway that does not host a name answers 404. Write set: `doorway/doorway-service` only, branch `sprint/2026-09-12-served-under-standing-doorway`.

- [ ] **Step 1: registry fold** — candidate holders for a root the local doorway cannot serve = DHT-registered doorways holding a live hosting contract for it, ordered health-first (status classification) then owner order; a named hook for attested RTT.
- [ ] **Step 2: one-hop forward** — relay to the first live holder with the loop-prevention header and a budget of one; try the next holder on a shed before answering the shed; preserve the original 404 when every holder fails; add the served-by origin header so the sticky client goes direct next time; wired as DeliveryRelay's final tier; dead code and the FEDERATION.md claim reconciled. Tests first.
- [ ] **Step 3: measure** — doorway bin from the sprint slot, restart the household doorways, glue for `features/federation/name-routing.feature` (drop `@wip` per scenario as glue lands), run under the lane lock; delta in served-under-standing with the run id.

**Routing dimensions (operator steering 2026-09-12 — the feature must cover the standard range; the fold/selector/relay shape makes each a field or a term, never a rewrite):**

| Dimension | Rung 2 (today) | Generalizes at |
|---|---|---|
| Path / mount prefix | yes — segment-boundary match, same rule as the local router | — |
| Host (virtual hosts; hostnames as head channels) | fold key is `RouteKey { host: Option, path }`, host unconstrained | rung 4 — contract gains host + channel |
| Health-based failover | yes — serving / shedding / uncertain / unreachable; next holder on a shed | — |
| Reach and requester standing | enforced at the holder, not in selection | rung 3 — eligibility fold |
| Nearest (attested RTT, region) | named hook; owner order today | selector term when the advertisement lands |
| Weighted / canary | no | the candidate channel is the canary; weight is a selector term if wanted |
| Relay mode | `Proxy`, one hop, served-by hint; `Redirect` declared, unimplemented | small addition |
| Sticky | client-side via the hint and the shipped fallback client | stays client-side |
| Rewrites | no — contracts declare mounts | not planned |

**Adaptation model (operator steering 2026-09-12 — mature frameworks, SSR runtimes such as PHP/LAMP and Node, nginx/Apache, ingresses and proxies must be able to attach at their foreseeable depth):** the hosting contract is the routing intermediate representation. Its vocabulary is a deliberate subset of the Kubernetes Gateway API HTTPRoute shape (hostnames; path, header, method and query matches; redirect, rewrite and header filters; weighted backends) plus the terms only this protocol has (reach, requester standing, channel, mode: proxy | redirect | render | static). Attachment by seam, per the seam map: frameworks and SSR runtimes attach at the SDK seam (the app package manifest declares mounts and render mode; the packaging adapter is where a non-Angular bundle describes itself; the execution engine is a mod under the render registry, as V8 rendering is today); foreign routing declarations (nginx/Apache config, Ingress, Caddy, Traefik) attach at the bridge seam as crates that translate them INTO contracts; foreign execution engines attach outward — contracts project to an nginx config or a Gateway controller, so the doorway is one controller of the contracts, not the only one. Guard: the doorway core never learns a framework — a change that names PHP, Next, nginx or Ingress inside the router or the fold is in the wrong seam; the contract grows a field, a bridge or adapter grows the knowledge. Rung 4's design gate decides the contract vocabulary against this shape.

## Rung 3 — The standing story (served-under-standing · epr-atom-home)

Evidence today: the habit is born red with two READY stories and no glue; the chrome's five affordances are named as epr-atom-home's commons checks.

- [ ] **Step 1: reach at serve time** — the doorway re-asks reach against the current EPR head on every serve of a warm shell or cached bundle (bytes stay warm; permission is never cached); a collective's reach ruling propagates on the next reconcile. Scenarios 1–3 of `served-under-standing.feature` get glue; first honest reds recorded.
- [ ] **Step 2: the fair-trade receipt and the owed response** — the chrome names what was exchanged for this serve and who was credited (REA), and a visitor's challenge becomes a witnessed commitment with a due window. Scenarios 4–5; epr-atom-home's commons checks measured.
- [ ] **Step 3: the transition count** — a humans-served sibling counts hosted now, stewarding now, crossed this period, so the bridge has a number the chrome can show.

### Rung 3 step 2 design (gate answered 2026-09-13)

Everything scenarios 4 and 5 need is already notarized. The slice mints **no entry type, no link type, no DNA-hash move, no new storage route**. Three REA `Commitment` actions already exist and are already seeded on the household: `project-epr` (which doorway projects which EPR at which path and reach), `operate-doorway` (who operates a doorway), `hosting-agreement` (the reciprocal term — an in-kind compute allocation scoped `host:…` + `epr_root:…`). Those three are scenario 4's three clauses, as three separate records — which is what makes "credits the holder separately from the doorway" structurally true. `REA_ACTIONS` in `content_store_integrity` is declared and never validated, so a new action value is DNA-hash-neutral; `create_rea_commitment` already threads `due`, `clause_of`, `provider`, `receiver`, `note`, `metadata_json` and is ensure-not-create.

**Entity R — the receipt: Ephemeral (C), a reading.** A per-serve `serve-blob` `EconomicEvent` on the DHT is REFUSED at the gate (10⁴–10⁶ heads/year on a plane whose one measured anchor is ~3,469 heads ≈ 2.5 h to quiesce). Successor named: one periodic aggregated `serve-blob` event `fulfills`-ing the `hosting-agreement`, sourced from the existing `infrastructure:blob-served` observations — a next scenario, "A period's serves are booked once against the hosting agreement, not once per visitor". The receipt names who was credited, never who was served; no per-visitor serve record may land on any notarized plane. Route: `GET /api/v1/receipt/{eprId}` on the DOORWAY (the answer depends on which doorway was asked and which holder relayed — doorway-operational state, same class as `routes/federation.rs`), unauthenticated, `?form=protocol` returns the three `ReaCommitmentView`s verbatim (the protocol form one request away). Honest absence: a missing record ⇒ that field `null` AND its name in `unrecorded[]`; never a synthesized number. Relay: the asked doorway fetches the holder's own receipt over the existing one-hop budget and merges — `exchanged` + the `held` credit come from the holder's answer, the `projected` credit is the asked doorway's own `operate-doorway` binding; a doorway crediting itself for a holder's work is unreachable, not discouraged. Wire on a 200 serve: `x-elohim-receipt: /api/v1/receipt/<eprId>` beside `x-elohim-standing: served;reach=…` (step 1's named red — prerequisite) and `x-elohim-served-by`; `ChromeContext.receipt {sentence, href}` (`skip_serializing_if`, byte-stable for today's shape).

**Entity O — the owed response: Notarized (A) on the existing `Commitment` entry, action `respond-to-challenge`.** Id = agent-scoped composite `respond-to-challenge-<sha256(challengerHumanId|respond-to-challenge|contractId|challengeId)[:16]>` so ensure-not-create makes a replay mint nothing twice; `cid` = entry hash, `action_hash` only ever `dht_anchor_hash`. `provider` is copied VERBATIM from the contract's `responsiveReach.party` — the doorway never sets who owes; when the contract declares no term nothing is minted (`owed: null` + why). `receiver` = the challenger's humanId from the SAME verifier the fold used to refuse them (`serve_eligibility::standing_from_request`); the contract challenged is read from the doorway's own live router projection, never a client-supplied id. `clause_of` = the hosting contract; `note` = the visitor's words verbatim; `metadata_json` carries `challengeId, term, reach, partyLabel, withinHours, declaredBy, declaredAt, carriedBy` — `carriedBy ≠ provider` is the record that the doorway did not answer for the collective. No refused path, IP, session id, user-agent or prior anonymous browsing may ride this commitment. Anonymous challenge ⇒ 401 with a named reason (a challenge needs someone to answer TO) — a next scenario, not silent attribution. `due` is structurally immutable (`handle_update_state` reconciles only state/finished/metadata) — it moves only by supersession. Route: `POST /api/v1/challenge` (doorway; the write goes through storage's EXISTING `POST /api/v1/commitments`), `GET /api/v1/challenge/{commitmentId}` re-read live (state/finished/overdue current) — this is how James reads the owed response and due date back from the same chrome. `WHERE_TO_BE_HEARD` (`hear`) repoints from `/api/v1/feedback/operations` to `/api/v1/challenge` (a reach challenge targets an REA Commitment, not content — `create_feedback_signal`'s content-action binding would fail at the conductor; the correction-outbox linkage is a backlog row). `ChallengeOutcome` closed at birth: 201 witnessed · 200 witnessed+owed:null · 200 replay · 401 anonymous · 409 same challengeId different bytes · 503 shed.

**Entity P — the responsive-reach term: two keys inside the existing hosting contract.** `responsiveReach: {party, partyLabel, withinHours}` in `metadata_json` on the collective's own `project-epr` contract, beside `urlPath`/`mode`/`reach`/`gateHints` — authored by the collective's steward, never a doorway; its `created_at`/`dht_anchor_hash` predate any challenge, which is what makes "declared before the visitor sent it" checkable. Both keys join `PROJECTION_RELEVANT_FIELDS` (a change re-grants through supersession). `EprProjectionView` gains `responsiveReach: Option` and `hostingAgreementId: Option`, serde-defaulted (an older doorway reads `owed: null`). Successor (C13): a collective-scoped `mishpat_integrity::Precedent` (`PrecedentByScope` links exist) so one policy governs every EPR a collective stewards, the contract term reading as override — backlog row `responsive-reach-policy-home`. The refusal JSON gains `owed: {party, partyLabel, withinHours, declaredBy, declaredAt} | null`; scenarios 1–3's glue ignores unknown keys.

**Commits, each gate-able alone:** C1 doorway pure `services/serve_receipt.rs` (`Receipt`, `Credit`, `ChallengeOutcome`, `build_receipt`, `receipt_header_value`; seam-registry rows: `serve_receipt::build_receipt` verdict-fn, `Receipt` boundary-answer-type, `owed_response::due_from_responsive_reach` pure-decision-predicate, `ChallengeOutcome` reason-outcome-enum) · C2 views + storage additive (`responsive_reach`, `hosting_agreement_id`, `commitment_to_projection_view`, `PROJECTION_RELEVANT_FIELDS`, schema + ts-rs regen) · C3 doorway receipt wiring (`x-elohim-receipt` on the Serve branch in `dispatch_to_projected_epr`, `ChromeContext.receipt`, the `GET /api/v1/receipt/{eprId}` arm reading own `operate-doorway` + `hosting-agreement` via the existing `GET /api/v1/commitments?action=…&state=active` filtered on `inScopeOf`, the one-hop holder fetch through name_routing's fold, header on the relay's verbatim-copy list) · C4 doorway challenge ceremony (`services/owed_response.rs`, `routes/challenge.rs`, `Refusal.owed`, `WHERE_TO_BE_HEARD`; storage const `RESPOND_TO_CHALLENGE_ACTION` with drift test and the comment that the `REA_ACTIONS` append rides the next DNA move) · C5 glue in `served-under-standing.steps.ts` (`buildMetadata` gains `responsiveReach` for all scenarios; `stageRoot` also stages a `hosting-agreement` scoped `["epr_root:{contentId}","doorway:{doorwayId}"]`, torn down with the root; scenario 5 narrows to `private` on purpose — `classify_reach('private') → BeneficiaryOnly` refuses authenticated James without a false premise, licensed by the feature's own text; teardown leaves the owed-response commitment standing — withdrawing it would be the test answering for the collective) · C6 habit delta. Owed and registered partial: C7 a contract test pinning that the challenge route mints exactly the window the refusal advertised; C8 `doorway_serve_receipt_total{outcome}` / `doorway_challenge_total{outcome}`; C14 the overdue arm has no witnessed home yet (next scenario "An owed response that comes due is visible as overdue to everyone").

**Sequencing:** scenarios 4 and 5 need a device-persona bearer through a doorway and scenario 4 needs the `x-elohim-standing` serve header — step 1's two named reds. Step 2 is buildable and unit-gated before they cure; it cannot go green on the mesh until they do.

## Rung 4 — Hostnames become heads (design gate first)

### Rung 4 design (gate answered 2026-09-13)

**The words this section uses**, fixed here so every claim below is checkable without leaving the
document (the wider vocabulary is `genesis/a2o/features/dataplane/served-under-standing.feature`'s own
definitions block, which this section deliberately does not restate):

- **EPR** — a published resource the protocol addresses by its own identity (a page, a site, an app
  bundle), carrying its own declared reach.
- **head** — WHICH version of an EPR is current. Not "the newest": a head is *declared*, and which
  declaration wins is decided by an election, not by a timestamp race.
- **election** — the notarized, deterministic rule every peer runs over the competing head
  declarations for one EPR. Two tiers: an **earned** declaration beats every **staging** one
  unconditionally; within a tier, the newest notarized declaration wins with a fixed tiebreak.
- **channel** — WHICH TIER of that election a hostname serves. `converged` = the earned winner.
  `candidate` = the staging declaration standing beneath it (the next version awaiting promotion).
  A channel is a label on a contract, never a stored version.
- **contract** — the notarized promise (an REA `project-epr` Commitment) that a named doorway will
  project a named EPR at a named address. A doorway's licence to answer at all; it cannot write
  itself one.
- **reach** — WHO may receive an EPR, declared by whoever stewards it. `commons` is the widest rung.
  Independent of head and of who holds the bytes.
- **standing** — what a requester can show for themselves at the moment of the request, as their own
  conductor states it.
- **fold** — the four-term function a doorway runs per request to decide whether it may answer:
  contract × liveness × reach × standing. It is recomputed on every serve and never cached beside
  the bytes.

**Target:** `elohim.host` = the converged (elected) head at commons reach, served by every doorway;
`alpha.elohim.host` = the candidate head — the STAGING declaration standing beneath the earned
winner — at stewards-or-collective reach, served by every doorway; promotion is the collective's
election, never a doorway's config.

**What the gate found before anything was designed: most of this is already notarized, and the
missing parts are fields, not planes.** The hosting contract is an REA `Commitment` on the elohim
DNA (`content_store_integrity::EntryTypes::Commitment`), authored by `content_store::create_rea_commitment`
under action `project-epr`. The channel already exists as two tiers of ONE notarized election on the
`Content` entry's `canonical_head` anchor — link tags `canonical-head:staging` and
`canonical-head:earned`, both riding `LinkTypes::IdToContent`, arbitrated by the pure
`select_canonical_winner` / `select_staging_candidate` pair that every peer runs identically. And the
doorway's fold already carries a host term it never narrows on (`RouteKey { host, path }`,
`HolderContract::host`, `host_matches`, `mount_specificity`). Rung 4 adds two contract fields, one
router key change and one storage read arm. **It mints no entry type, no link type, no DNA-hash move
and no new HTTP route.**

#### Entity A — the hosting contract growing `hostnames` and `channel` (keeping `reach`)

*Not a new entity: an entry type that already exists, gaining two metadata keys. "Entity" is the
design gate's word for a decision point, not a claim that something is being minted.*

*(The rung's title says "host"; the field is a LIST, `hostnames`, matching Gateway API. Empty list = any
host, which is exactly today's behaviour. "host" below always means one entry of that list.)*

- **Classification: Notarized (A), on an entry type that already exists.** The contract is not new —
  it is the `project-epr` `Commitment`. `hostnames` and `channel` are keys inside that entry's own
  `metadata_json: String`, exactly as `urlPath`, `mode`, `reach`, `baseHref`, `entryFile`,
  `spaFallback`, `redirectsFrom`, `redirectTemplates`, `routeClaims`, `gateHints`, `deadEnd` and
  `stewardDirectEndpoint` already are. They are therefore notarized by construction and cost nothing
  new. **A channel does NOT need its own entry** — see Entity B: an election with two tiers already
  exists, and a `Channel` entry would be a second authority over it (C1).
- **Justification:** the protocol would be lying if a doorway's licence to answer for a hostname
  could change silently — that is the whole of `served-under-standing`. But the licence is not a new
  thing in its own right; it is the same promise the contract already carries, now stating *at which
  name* and *on which channel* it holds.
- **Head-plane cost budget:** contracts today = 3 EPRs × 2 doorways = 6 rows. The rule this design
  fixes: **one contract per (doorway, EPR); `hostnames` is a LIST inside it and `channel` is a single
  field — neither multiplies rows.** At 1 year with ~20 EPRs × 4 doorways that is ~80 contracts, each
  one `Commitment` entry plus three anchor links (id / provider / receiver). Against the ~3,469
  A-class content heads measured at genesis quiesce, this is order-of-magnitude noise — roughly one
  sweep tick at 200 heads/tick, not a new quiesce regime. Order-of-magnitude reasoning from that
  single anchor, not a computed extrapolation. Well under the ~500 fence; the bundling shape held in
  reserve if a per-host split is ever forced is **composite root** (one contract naming many
  hostnames — which is what `hostnames: []` already is).
- **Network stakes:** must behave under all four declared stages (`Simulacra < Bootstrap < Coordinated
  < Enforced` — `elohim/elohim-storage/src/trust/stage.rs`; verification cost is priced against the
  declared stakes, and a *floor-protected* cost never cheapens at any stage). Floor-protected and
  never stage-priceable:
  the reach/standing refusal (Constitutional — the contract is manifest-class) and the challenge path
  a refusal points at (CounterEvidence). Stage-priceable: full-chain re-verification of an
  already-witnessed, digest-matching contract head.
- **Content address strategy: Agent-Scoped Composite** — unchanged. `id =
  project-epr-<sha256(stewardPeerId|project-epr|doorway:{id}|epr:{id})[:16]>`; the tuple is (steward
  agent, target, type discriminator), which is Option 2 exactly. **`hostnames` and `channel` stay OUT of
  the digest** — they are contract *terms*, not contract *identity*, and the existing re-grant
  ceremony (`supersedes` → `mark_superseded`, successor id `-r<regrantFingerprint>`) is what carries
  a change to them. Add both to `PROJECTION_RELEVANT_FIELDS` and the drift detector supersedes
  correctly on day one. If two contracts for one (doorway, EPR) with genuinely different hostnames
  are ever needed, the scope grows a third ref `host:{name}` and the digest follows — a new id, a new
  row, deliberately. Note: the a2o lane's `testCommitmentId` hashes `doorway|mount|runStamp|scenarioNonce`
  — that is a *test-fixture* address over the MOUNT, minted to avoid reactivation churn, and it must
  not be read as the production shape.
- **Transport affinity:** n/a — a contract carries no bytes. The bundle blob it points at already
  carries its own `transport_affinity`.
- **Source of truth:** Holochain DHT (the `Commitment` entry). The `rea_commitments` SQLite row and
  the doorway's `EprRouter` table are both read-optimised projections.
- **Integrity zome + DNA-hash class:** `content_store_integrity` (elohim DNA, packed from
  `dna/elohim/`, named `lamad` in `dna.yaml`) — **DNA-hash-NEUTRAL.** No entry type, no link type, no
  validation change: `metadata_json` is an opaque `String` to the integrity zome.
- **Coordinator zome:** `content_store::create_rea_commitment(CreateReaCommitmentInput) ->
  ReaCommitmentOutput { action_hash, entry_hash, commitment }`. Unchanged, including its fork guard
  (ensure-not-create). The `cid` of this commitment is the **entry hash**; `action_hash` is only ever
  the `dht_anchor_hash`.
- **Projections:** SQLite `rea_commitments` (`dht_anchor_hash`: yes, stamped by the post-commit
  signal). Automerge sync: **no** — a contract is not broadcast content. Reach tier for that
  projection: *unresolved — reach vocabulary in declared drift* (and moot here, since it does not
  project).
- **Signal:** unchanged and already wired end to end — post-commit `ProjectionSignal::ReaCommitmentCommitted`
  → `rea_projection::project_signal` (upsert with `dht_anchor_hash`) → `StorageEvent::ProjectionRegistered`
  → SSE `projection.registered` → the doorway's `storage_events_subscriber` → re-fetch
  `GET /db/rea_commitments?action=project-epr&doorwayId=…` → `EprRouter::replace_all`.
- **HTTP route:** none new. `POST /api/v1/commitments` and `GET /db/rea_commitments` are already
  declared in elohim-storage's `build_manifest()`; `{id}` carries the commitment id (a slug-shaped
  content-addressed string), never a hash.
- **Anti-pattern check — three caught:** (1) minting a `HostRoute` / `Channel` entry type, refused by
  the "is a type already there / is this an attribute" tests and by Entity B; (2) keying the router
  from a doorway-local host allowlist, refused by the four-term fold — a doorway cannot write itself
  a contract; (3) the amber/green class — writing a per-host *mode* rather than deriving it. `channel`
  is READ from the election on every reconcile; nothing per-doorway is ever stamped.
- **SDO/RWA Test** (social-dominance-orientation × right-wing-authoritarianism: *if the dominant few
  and the many who would enforce for them held this store, what could they see, join and compel?* —
  the aggregator is the danger, never the sensor): a hosting contract is deliberately public — it is the licence the public must be
  able to read back in order to challenge it (boundary: standing, not property). Worst holder sees
  which doorway serves which EPR at which hostname under which reach; it names no requester, so
  boundary 2 (activity ledgers held by the holon they describe) and boundary 6 (participation is
  sensitive) are not crossed. The refusal this test produces: **no per-visitor serve record may ever
  land on the contract plane.** A contract says who may serve, never who was served.

#### Entity B — the head channel itself

*Not a new entity either, and not even a new field on the DHT: the two tiers a channel names are
already notarized and already shipped. What Rung 4 adds is a contract that SAYS which tier a
hostname serves.*

- **Classification: not a new entity — a LABEL on the contract naming a tier of an election that is
  already notarized (Linked / A2, and already shipped).** `declare_canonical_content_head` writes the
  `canonical-head:staging` tag; `declare_earned_canonical_head` writes `canonical-head:earned`; both
  are `create_link` on the `canonical_head` `StringAnchor` reusing `LinkTypes::IdToContent`.
  `run_election` returns `{ winner, staging_candidate }` from one bounded link gather;
  `select_canonical_winner` arbitrates on (tier, DHT link-creation `Timestamp`, create-link
  `ActionHash`) — **earned beats all staging unconditionally, and never on recency**. So:
  `channel: converged` ⇒ serve the winner; `channel: candidate` ⇒ serve the staging candidate.
- **Head-plane cost:** **zero new heads.** The staging declaration exists already for every candidate
  release; naming it from a contract adds no row, no Kad record, no election candidate, no sweep work.
- **Identity — correcting the plan's own phrasing:** a candidate is addressed by the **ActionHash of
  its canonical-head declaration** (`ContentHeadWire.staging_candidate: Option<HoloHashB64>`), not by
  a CID. The *bundle* that declaration names is addressed by a blob CID (`bafkrei…`). Both are true
  at different layers; the contract names neither — it names the channel, and resolution happens at
  serve time. Writing a head CID into a contract would be a frozen pin — the channel would stop
  following the election the moment it was written, which is the whole failure this rung exists to
  avoid.
- **Promotion with no per-doorway config:** the collective writes ONE notarized act —
  `declare_earned_canonical_head` on the new candidate. Because both hostnames resolve through the
  election rather than through configuration, `elohim.host` follows the new earned winner and
  `alpha.elohim.host` follows whatever staging declaration now postdates it. No redeploy, no manifest
  edit, no doorway restart, and no doorway gets a vote. That is the whole reason a hostname becomes a
  head: the promotion is the election, read identically by everyone on their next reconcile.
- **Reach per channel:** reach is NOT a property of the channel — it stays a contract field, declared
  per (doorway, EPR, hostname). The *default for a candidate hostname* is a restricted rung, never
  `commons`: `classify_reach` then yields `Restricted`, `serve_eligibility` refuses an anonymous
  visitor with the chrome's reason and the `WHERE_TO_BE_HEARD` pointer, and a steward or collective
  member whose standing satisfies the declared audience is served. Never-widen is already the module's
  law; the design only has to refrain from declaring `commons` on a staging name.
- **Honest absence (the one behaviour that must be designed, not inherited):** when `channel:
  candidate` and no staging declaration stands beneath the winner, the candidate hostname answers a
  named "no candidate staged" — it must **never** fall through to the converged head. Silently
  serving production bytes at the staging name is how a candidate channel stops meaning anything.
- **Coordinator zome / DNA-hash class:** nothing to change. `content_store::declare_canonical_content_head`
  and `declare_earned_canonical_head` already exist; `POST /db/content/{id}/canonical-head` is already
  declared in `build_manifest()` (edge-auth gated), which is how the household lane stages a candidate.
- **The one real gap:** `ContentHeadView` (what `GET /db/content/{id}/head` returns, built by
  `content_head_view_from_content`) exposes only `head_action_hash` / `declared` / `dht_anchor_hash` /
  `trust` / `blob_hash`. The staging candidate is visible today ONLY inside storage, on the conductor's
  `ContentHeadWire`, where the release-adoption watcher reads it. Slice 1's storage work is exactly
  this: surface `stagingCandidate` additively on the head read so the doorway can resolve a candidate
  over HTTP. Nothing else is missing.
- **SDO/RWA Test:** the candidate channel is the sharp one — the worst holder sees an unreleased
  build before the collective elected it. Bounded by (a) the candidate hostname's restricted reach,
  re-asked at serve time, and (b) the fact that staging and promotion are both public notarized acts,
  so nothing can be promoted quietly. The refusal: **no doorway may serve a candidate no steward
  staged** — a doorway inventing a candidate would be self-election (C1), and `classify_candidate_follow`'s
  existing defensive refusal (candidate == winner ⇒ Leave) is the precedent.

#### The routing IR vocabulary — a subset of Gateway API HTTPRoute, plus the terms only this protocol has

The hosting contract **is** the routing intermediate representation. A **contract field** is declared
by the steward and notarized; a **selector term** is derived at request time and ordered by
`SELECTOR_TERMS`, where index IS precedence. Keeping the two apart is what keeps *"bytes may be held
warm, the fold may not"* true: `reach` is a contract field whose *enforcement* is a fold term.

| IR field | Gateway API HTTPRoute analogue | Kind | Status | Lives today in |
|---|---|---|---|---|
| `hostnames: [String]` (empty = any host) | `hostnames` | contract field | **slice 1** | new metadata key; `RouteKey.host`, `HolderContract.host`, `host_matches`, `mount_specificity` already carry it |
| `channel: converged \| candidate` | — (protocol-only) | contract field | **slice 1** | new metadata key; resolves through the canonical-head election |
| `reach` | — (protocol-only) | contract field (fold-enforced) | shipped | `EprProjectionView.reach`, `classify_reach`, `serve_eligibility` |
| `standing` (requester) | — (protocol-only) | selector term, enforced at the holder | shipped (as enforcement); **later** as a selection term | `RequesterStanding`; `SelectorTerm::ReachStanding` declared, constant |
| `mode: proxy \| redirect \| render \| static` | partly `filters`/`backendRefs` | contract field | `proxy` + `render`/`static` shipped (`ProjectionMode::{Cached, StewardDirect}`, `RelayMode::Proxy`); `redirect` **later** | `RelayMode::Redirect` declared, unimplemented, skipped-with-warning |
| `match.path` (PathPrefix, segment-boundary) | `matches[].path: PathPrefix` | contract field | shipped | `urlPath`; `EprRouter::path_matches_prefix` == `mount_covers` |
| `match.path` Exact | `matches[].path: Exact` | contract field | later | — |
| `match.path` RegularExpression | `matches[].path: RegularExpression` | contract field | **not planned** | contracts declare mounts; a regex mount is the seam guard's wrong side |
| `match.headers` | `matches[].headers` | contract field | later | — |
| `match.method` | `matches[].method` | contract field | later | — |
| `match.queryParams` | `matches[].queryParams` | contract field | later | — |
| `filters.redirectsFrom` (alias → mount) | `filters.RequestRedirect` | contract field | shipped | `redirects_from` |
| `filters.redirectTemplates` (`/lamad/resource/{id}` → `/epr/{id}`) | `filters.RequestRedirect` (templated) | contract field | shipped | `redirect_templates` |
| `filters.urlRewrite` | `filters.URLRewrite` | contract field | **not planned** | contracts declare mounts, they do not rewrite them |
| `filters.requestHeaderModifier` / `responseHeaderModifier` | same | contract field | later | doorway-minted today (`x-elohim-standing`, `x-elohim-bundle`, served-by) — doorway-owned, not contract-declared |
| `backendRefs` (the holder set) | `backendRefs` | **derived, never declared** | shipped | the fold derives holders from contracts × liveness; a contract never names a backend |
| `backendRefs[].weight` | `backendRefs[].weight` | selector term | later | `SelectorTerm::Weight` declared, constant |
| `liveness` | — (Gateway leaves it to the controller) | selector term | shipped | `HolderLiveness` (serving / uncertain / shedding / unreachable) |
| `nearest` (attested RTT; region beside it, never above it) | — | selector term | later | `SelectorTerm::Nearest` declared, constant |
| `ownerOrder` | — | selector term | shipped | `SelectorTerm::OwnerOrder`, the stable final tiebreak |
| `baseHref`, `entryFile`, `spaFallback` | — (bundle-serving detail) | contract field | shipped | `EprProjectionView` |
| `gateHints` (the audience a reach names) | — | contract field | shipped | read by `serve_eligibility::AudienceTerm` |
| `routeClaims` (content-type → mount binding) | — | contract field | shipped | `RouteClaimGrant` |

**Status legend:** `shipped` = already in the tree today · `slice 1` = built by the first slice
below · `later` = a field the shape admits and nothing yet fills · `not planned` = deliberately
refused.

**The seam guard restated against this table:** the doorway core never learns a framework. A field is
added to the contract; a *bridge* crate translates nginx/Apache/Ingress/Caddy/Traefik declarations
INTO contracts, and contracts project OUT to an nginx config or a Gateway controller. Any diff that
names PHP, Next, nginx or Ingress inside `epr_router.rs` or `name_routing.rs` is in the wrong seam.

#### Concern canon (C0–C14), answered at birth for the three new decision points

New decision points, to be registered in `doorway/doorway-service/seam-registry.yaml` **when the code
lands, not after**: `EprRouter::dispatch` (pure-decision-predicate, now host-keyed),
`resolve_channel_head` (verdict-fn), `Channel` (boundary-answer-type).

The canon is sixteen recurring failure classes defined in `.claude/epr-meta/concerns.yaml` (and, for
the predicate-bearing ones, `.claude/epr-meta/policies.yaml`). Each is labelled below so the answer is
checkable without opening them.

**Answered** — logic plus a pin already exists, or the design makes it structurally true:

| Class | What it asks | Answer |
|---|---|---|
| C0 plane location | which plane owns this concern | contract on the DHT `Commitment`; channel on the `Content` canonical-head election. Neither in k8s manifests, neither in DNS — DNS names only doorways that can ROUTE |
| C1 anti-self-election | no component crowns what it authored | a doorway neither elects a head nor writes itself a contract; it reads both |
| C2 monotonic authority | never backwards, and name the clock | a head moves only by tier-then-notarized-`Timestamp` (never `declared_at`, never recency); a contract term moves only by supersession |
| C3 liveness | a legal move exists from every reachable state | empty `hostnames` matches every host, so no existing contract becomes unroutable when the field arrives; the move out of "no candidate" is to stage one |
| C4 honest absence | absent ≠ refused ≠ unreachable | `candidate` with no staging declaration answers a named absence, never the converged head |
| C5 evidence-not-authority | a claim confers only what the receiver re-derives | the candidate's record is re-proved in wasm by `validate_carried_head_record`; storage synthesises nothing |
| C6a bounded work | every loop respects a declared budget | one bounded link gather per election; `bundle_heads` on a 30 s tick with a 2 s read timeout — the host key multiplies the map, not the fetch |
| C6b idempotent effect | replay mints nothing twice | `create_rea_commitment` is an ensure with a fork guard; an identical re-seed is a quiet 409; the re-grant fingerprint is deterministic |
| C10 contract-evolution honesty | an unknown field is rejected or observed, never defaulted into meaning | both keys are serde-defaulted and `metadata_json` is opaque to the integrity zome, so an older doorway reads exactly today's behaviour rather than mis-serving; the view is not `deny_unknown_fields` |
| C11 externally-imposed backpressure | degrade by a declared, counted policy | unchanged — the shed path and `HolderLiveness::Shedding` already carry it |
| C12 consent/authorization | authority verified structurally at the acting node | reach + standing re-asked at serve time; the candidate hostname's restricted reach IS the authorization |
| C13 graduated authority | every scaffold names its successor at the gate | `hostnames: []` (any host) is the declared scaffold; its successor is a contract that names its hostnames, and the gate is the a2o scenario that asks each name of each doorway |

**Partial**, with the gap named and owed:

| Class | What it asks | Gap |
|---|---|---|
| C7 advertise/serve symmetry | what a surface advertises equals what it serves | a name advertised in the membership document must be backed by a contract someone actually holds. The household file sink is origin-keyed and knows exactly one name today, so a second name added without a matching contract advertises what nothing serves. Contract test owed |
| C8 observability-per-decision | every outcome increments a labelled counter through a typed reason | `ReplaceOutcome { installed, rejected }` counts rows, but nothing counts refusals or absences per (host, channel). A labelled counter owed |
| C14 witnessed residual | the outcome set closes with a witnessed arm | a reach refusal already lands in `WHERE_TO_BE_HEARD`; "no candidate staged" has no witnessed arm yet |

**n-a** — **C9 identity-lineage continuity**: nothing re-keys; a hostname is not an identity and no
state is orphaned by one.

#### Step 4's rename cascade — the inventory, taken before the pass

Token-boundary count (`(?<![a-zA-Z])alpha(?![a-zA-Z])`, so `alpha-a`, `ALPHA_DOORWAY_URL` and
`alpha.yaml` all count and `alphabet`/`alphanumeric` never do). **Zero occurrences are in generated,
lockfile or dist files — every match is hand-editable source.** The two large surfaces are
evidence-based bucket estimates from full token-frequency tables plus sampling, not a line-by-line read.

| Surface | Files | Occurrences | Excluded | Doorway-identity | Channel-name | Fleet-env name | Other |
|---|---|---|---|---|---|---|---|
| `genesis/a2o/` | 287 | 1871 | 22 | ~1310 | 0 | ~470 | ~91 |
| `app/elohim-app/scripts/` | 8 | 95 | 1 | ~55 | 0 | ~38 | ~2 |
| `app/elohim-app/src/environments/` | 5 | 33 | 0 | ~15 | ~9 | ~7 | ~2 |
| `genesis/orchestrator/data/deployments.json` | 1 | 17 | 0 | 0 | 0 | 17 | 0 |
| `genesis/manifests/**` | 8 | 226 | 0 | 34 | 0 | 192 | 0 |
| `genesis/orchestrator/manifests/**` | 36 | 586 | 0 | ~39 | 0 | ~547 | 0 |
| **Total** | **345** | **2828** | **23** | **~1453** | **~9** | **~1271** | **~95** |

**Four things this changes about Step 4.**

1. **It is three renames, not one, and only the first is in scope.** Doorway-identity (~1453, 51%)
   is what moves to a channel name. Fleet-env (~1271, 45%) — the `elohim-alpha` namespace, every
   `*-alpha` k8s object, `alpha.yaml` / `alpha-b.yaml` / `alpha-coturn-*.yaml`, `alpha-cluster-6peer`,
   the per-human `matthew-alpha` / `pete-alpha` PVC names — is the CLUSTER's name and must not move
   with it. A global substitution would silently rename the fleet.
2. **`deployments.json` is 100% fleet-env** (17 occurrences, all prose naming a human's deployed
   instance/PVC/pod). Step 4's own line below — "deployments.json loses alpha-versus-apex as doorway
   identity" — is already true there — the doorway-identity split lives in the a2o vocabulary and the manifests, not
   in that file.
3. **The channel name is empty ground — ~9 occurrences, all in `src/environments/`**, where `alpha`
   already sits on a `development → alpha → staging → production` ladder in `environment.types.ts`
   beside its own doorway-address constants. That ladder is the only existing reading of "alpha as a
   tier", and it is the natural anchor for the new one.
4. **A live release-channel vocabulary already exists and must not be collided with.**
   `deployments.json` runs `ELOHIM_RELEASE_CHANNELS` with `=observe` / `=canary` / `=apply` on channel
   ids like `runtime:coordinators:elohim:workspace`. That is the *runtime-artifact* channel plane
   (Entity B's `release_adoption`); the head channel this rung introduces is the *content-head* plane.
   They share the election primitive and must not share a namespace by accident — resolve how
   `channel: candidate` on a hosting contract reads beside `=canary` on a release channel BEFORE the
   rename, not during it.

**The target vocabulary (proposed here, confirmed at Step 4 — an implementer cannot do a
~1453-occurrence rename without it).** Three buckets, three fates:

| Today's use of "alpha" | Becomes | Why |
|---|---|---|
| **doorway identity** — `alpha-elohim-host` / `apex-elohim-host` as a doorway's id, `doorway-alpha.elohim.host` as its address, `E2E_DOORWAY_ALPHA`, `doorway "alpha"` in a2o steps | a **premise-named doorway id** — `doorway-ethosengine` / `doorway-shem`, matching the ingress classes Rung 5 Step 1 creates (`public-ethosengine`, `public-shem`) | a doorway's identity should name the premise that houses it, which is the one fact about it that does not change. It must stop appearing in a hostname at all |
| **channel name** — `alpha.elohim.host` as a public address | **stays the literal string `alpha.elohim.host`**, but its meaning changes from "doorway A's address" to "the hostname whose contract carries `channel: candidate`" | nothing to rename; the word is already right once the contract says what it means. This is the only bucket where "alpha" survives |
| **fleet-env name** — `elohim-alpha` namespace, `*-alpha` k8s objects, `alpha.yaml` / `alpha-b.yaml` / `alpha-coturn-*.yaml`, `alpha-cluster-6peer`, `matthew-alpha` PVCs | **untouched** | that is the cluster's name, not a doorway's and not a channel's |

**Open before the pass, not during it:** `channel: candidate` (this design, the content-head plane)
sits beside `=canary` (`ELOHIM_RELEASE_CHANNELS`, the runtime-artifact plane). They share the election
primitive and must not silently share a word. Decide whether the hosting contract's `channel` values
stay `converged`/`candidate` or align to the release vocabulary BEFORE renaming anything.

**Sequencing that falls out:** the rename is a vocabulary pass over ~1453 doorway-identity uses, and it
is only safe once the host key is live (slice 1) — until a doorway can be asked for a name, "alpha" as
a doorway's identity is the only thing that works. Do it after Step 3 is green, in its own push, with
`prod.yaml` retirement (`genesis/orchestrator/manifests/{doorway,edgenode,elohim-app}/prod.yaml`) as a
separate commit inside it so a routing regression and a vocabulary churn can never be confused.

#### The smallest first slice

**`host` + `channel` as contract metadata, the router keyed by host, one candidate channel gated by
reach — measured at home.** Six changes, none of them a new plane:

1. **Contract** — `project-epr` metadata grows `hostnames: string[]` (default `[]` = any host, i.e.
   byte-for-byte today's behaviour) and `channel: "converged" | "candidate"` (default `"converged"`).
   Both join `PROJECTION_RELEVANT_FIELDS`, so a change re-grants through the supersession ceremony
   that already exists. No DNA change.
2. **View** — `EprProjectionView` gains `hostnames: Vec<String>` and `channel: Channel`, both
   serde-defaulted; `commitment_to_projection_view` reads them with the same defaults; ts-rs
   regenerates (sha256-verify the generated TS).
3. **Router** — `EprRouter.table` is keyed by `RouteKey { host, path }` instead of a bare path string,
   and `dispatch` takes the request's Host header (already extracted — `RelayContext::from_request`
   captures it today and threads it into the federation fold). The three shapes are one thing at
   three layers: a contract declares `hostnames: []` (a list); the router indexes each hostname as
   its own `RouteKey { host, path }` row; `host: None` is the any-host wildcard. A contract with empty `hostnames` installs under
   `host: None` and still matches everything. `mount_specificity` already ranks host-bound above
   any-host, so the tie-break rule is not invented here.
4. **Storage read arm** — `ContentHeadView` gains `stagingCandidate: Option<String>` (additive),
   projected from the same conductor read the release-adoption watcher already performs. This is the
   only genuinely new plumbing in the slice.
5. **Head resolution** — `bundle_heads` is keyed by `(slug, channel)`; `converged` resolves the
   declared/earned head as today, `candidate` resolves `stagingCandidate`, and absence is absence.
6. **Home measurement surface** — the household membership authority gains a SECOND public name
   (`MESH_MEMBERSHIP_NAME` becomes a list: `elohim.local` plus a candidate sibling), so both household
   doorways carry both names and the lane can ask each name of each doorway. The candidate name's
   contract declares a restricted reach; `serve_eligibility` refuses an anonymous visitor with the
   chrome's reason, and a steward is served.

**Measured by:** `just test mesh features/dataplane/served-under-standing.feature` — its scenarios run
against the candidate hostname at home, with `features/federation/name-routing.feature` re-run
unchanged to prove the host key did not regress path-only routing. Delta in
`doorway/doorway-service/.epr-meta/served-under-standing.habit.md` with the run id.

**Deliberately NOT in slice 1:** header / method / query matches, weighted backends, the redirect
relay mode, any nginx / Ingress bridge crate, and the "alpha" rename (Step 4 is its own pass, and
doing it inside this slice would make a routing regression and a vocabulary churn indistinguishable).

#### Design constraints discovered during the gate

- **The coherence digest is a wire contract, and the host fold reads through it.** `install_name_routes`
  mints `HolderContract`s from `CoherenceManifest.heads` — a `Vec<EprHeadFingerprint { url_path,
  epr_id }>` that is *also* the dag-cbor preimage of the cross-edge coherence digest (`mint_head_set_digest`).
  Adding `hostnames`/`channel` INSIDE that struct changes the digest for everyone and makes two
  mixed-version edges report content divergence they do not have. Slice 1 must therefore carry the new
  terms as a **separate additive field on `CoherenceManifest`**, leaving the digest preimage alone —
  or version the digest explicitly. This is C10 with a live blast radius.
- **`in_scope_of` is immutable after creation** (`handle_update_state` reconciles only
  `state`/`finished`/`metadata_json`). Any design that wanted host in the scope string would need a new
  commitment, not a PATCH — which is another reason `hostnames` belongs in metadata.
- **Two independent authors can reach one commitment id.** `create_rea_commitment`'s fork guard closes
  the wide window, not the narrow one; the durable cure is still one author per undertaking. A
  per-hostname contract seeded from two places would reopen exactly the fork this sprint's Rung 1
  Step 6 is closing.
- **Rung 2 already landed the host-shaped fold.** `RouteKey`, `HolderContract.host`, `host_matches`,
  `mount_specificity` and `SELECTOR_TERMS` are in the tree and every contract is currently any-host.
  Slice 1 is populating a seam that was built to be populated — if it needs to re-key the fold, its
  table or its callers, something has gone wrong.

- [x] **Step 1: design gate** — p2p-design-gate run 2026-09-13 on the hosting contract growing `hostnames` and `channel` (keeping `reach`) and on the head channel itself; both entities, the routing IR vocabulary, the concern-canon answers and the rename inventory are the section above. Verdict: no new entry type, no new link type, no DNA-hash move, no new route.
- [ ] **Step 2: contracts and registry** — seed writes host + channel + reach; the doorway keys its route registry by host and picks the head by channel (reuse the bundle-heads reconciler and the canary's long-lived candidate channel).
- [ ] **Step 3: the candidate host is reach-gated** — anonymous at alpha refused with the chrome's reason; a steward served; the A/B test as head. Measured by served-under-standing's scenarios against a real second channel.
- [ ] **Step 4: rename cascade** — the a2o vocabulary's "alpha" moves from doorway identity to channel name (env names, fixtures, LAYERS) in one deliberate pass; `prod.yaml` retired; deployments.json loses alpha-versus-apex as doorway identity.

## Rung 5 — Fleet: the apex (doorway-failover flips)

- [ ] **Step 1 (operator):** create the two premise-pinned ingress classes (`public-ethosengine`, `public-shem` proposed) with controllers bound to their nodes.
- [ ] **Step 2:** land `sprint/apex-multi-a-ingress` on dev (class-keyed conflict checker, both doorway ingresses accepting the apex, both beacon legs contributing it); one push, the roll it triggers is the measurement.
- [ ] **Step 3:** apex-transition measured on the fleet (Task 21 of the 2026-09-10 plan); `doorway-failover` red → green with the build number, by `epr flow note --kind ruling`.

---

**Done means:** every rung's story green, or red for a named mesh reason written into its atom; deltas in every touched atom; the register re-projected; one push per batch with the fleet confirming what the household proved.
