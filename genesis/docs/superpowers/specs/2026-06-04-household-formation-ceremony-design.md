---
title: "Household Formation Ceremony — recognition-of-the-given, emergent reciprocity"
id: household-formation-ceremony-design
status: Draft
class: protocol-canonical
domain: D7
topic: [qahal, household, formation, ceremony, affirm-membership, custody-blob, delegates-compute, stewardship-grant, seeder, realism-ladder, quiltPolicy, a2o]
cites:
  - qahal-epr-household-lattice-design | the umbrella doctrine seed this spec derives from — lattice, two flows, reach doctrine, drive doctrine; formation is its active spine | sha256:ed5c1d3d2698b567 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - qahal-architecture-vision | gospel discriminator — implementation work in the qahal pillar resolves to claims expressed there; the rubric concept this spec configures for households | sha256:6a519b464b586832 | path: genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md
  - qahal-collective-membership-dht-design | the existing Collective+Membership entry types and request/attest/revoke coordinator family that affirm_membership joins as the recognition-flow sibling | sha256:8d7b9704f7aa9ca0 | path: genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md
  - mutual-storage-replication-dwelling-hub-design | replicates-dwelling + mutuality-audit + intent-first/observed-second — the pattern the ambient-custody responder reuses; gertrude cross-dwelling counterparty stays distinct from intra-household custody | sha256:5596799dbb456bc2 | status: stale — target content moved on; re-verify | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
  - recovery-protocol-phase-2-revised-design | StewardshipGrant/DevicePolicy primitives instantiated for kid devices at formation; Phase 2b stub scope is verification flag V2 | sha256:9d1844484ed64de4 | path: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
  - genesis/docs/plans/2026-03-15-qahal-community-directory-design.md
  - d1-through-d5-node-and-household-canon | D2: household reuses collectives, no new entry types — the constraint the entity model honors | sha256:5ee9472bbefad806 | path: genesis/docs/content/elohim-protocol/history/2026-04-19-d1-through-d5-node-and-household-canon.md
  - tiered-quilt-stewardship-design | §4 v0.2 quiltPolicy classes + pledge clamp — §8 declares qahal/household and ties the ceremony custody commitments as the backing pledges | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - rea-compute-commitment-primitive | rea-compute-commitment-primitive | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md
  - admin-key-lifecycle-dev-to-production | X-API-Key displacement direction — stage 3 of the drive architecture instantiates its commitment-backed delegation for the seeder service-agent | sha256:44dc9b49dec9d439 | path: genesis/docs/superpowers/specs/2026-06-03-admin-key-lifecycle-dev-to-production.md
  - genesis/data/timeline/backlog/qahal-household-collective-first-class.md
derived_from: genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
---

# Household Formation Ceremony

A family — each member with a device, hub or not — forms a household and immediately
sees the end-to-end benefits of the protocol among themselves: streamlined
onboarding, automatic balancing, zero-config discovery. This spec lands the
**formation ceremony** as the canonical act that mints the household and its default
reciprocity bundle, and converts the seeding/testing apparatus to drive that act as
the personas' real agents.

Companion lattice seed (doctrine home): `2026-06-04-qahal-epr-household-lattice-design.md`.

## 1. Settled decisions (the session record, 2026-06-04)

| Decision | Verdict |
|---|---|
| Seeded vs emergent reciprocity | **Emergent + explicit interim fixtures.** The ceremony is the only canonical mint; rung-1 fixtures with loud provenance light the views until it lands, then retire (hard gate, §7). |
| Default bundle composition | **Custody ambient, authority explicit.** Mutual `custody-blob` across member pairs is the household default (shown, pre-accepted, opt-out per pair). `delegates-compute` is explicit opt-in per member — bounded authority is never ambient. quiltPolicy applies collective-wide at formation. |
| Kid devices (james) | **Own agent + sponsored membership.** The kid's conductor authors his own `Membership` (non-repudiable identity from day one) with `sponsor_cid` = a parent's Steward membership; a StewardshipGrant-shaped commitment governs device policy; compute contribution is parent-consented. |
| The member verb | **`affirm_membership`** — recognition-of-the-given, the sibling of `request_membership` (graduated flow). Rubric-selected per the lattice seed §3. |
| Household character | **Declared at init, not derived from member count.** "Begin your household" is the intent; the collective is born household-class via charter/rubric. A one-member household is a household awaiting its members. |
| Creation gating | Minting a household-class collective is in-principle gated (graduated-capability surface) but the bar is near-floor: Human entry + device. **This spec assumes the permission.** |
| Orchestration | **Choreographed, substrate-as-state (deterministic floor).** No ceremony entity, no orchestrator; "formation complete" is a derived view state. Elohim facilitation is the ceiling: narrates, attests, never gates. |
| Reach | Household intended-intimate reach is **self-validating via the affirmation set** (lattice §4). Never bake `reach:"household"` — no such Reach variant exists (verified); household-ness is `governanceLayer:'family'` projection. |

Corrections carried from discovery (verified): the historical custody drift was to
**terrance** (not gertrude) and is already healed (`seed-commitments.ts:177-188`
seeds matthew↔jessica, restored 2026-06-04); the DeliveryPeer changes exist as a
clean-applying prepared patch (`genesis/data/timeline/backlog/patches/deliverypeer-household-enrichment.patch`),
blessed by this design (§5.5).

## 2. Entity model (p2p-design-gate output, condensed)

Zero new DHT entry types. Two new coordinator functions. One action-gate extension.

| Entity | Category | Entry type | Identity | New code |
|---|---|---|---|---|
| Household Collective | A (notarized) | `Collective` (imagodei, exists) | `collective:{action_hash}`; slug = display alias only | charter carries household character at init |
| Membership (affirm) | A (notarized) | `Membership` (imagodei, exists) | (member_cid × collective_cid) tuple | **`affirm_membership` coordinator (new)** |
| Invitation | C (operational, deliberately NOT on DHT) | — (signed single-use TTL token) | token id | token-hash recorded as a link on the Collective against replay |
| Reciprocity bundle | A (notarized) | REA Commitment / `Mishpat::Commitment` (exist) | (provider, receiver, action, scope); `in_scope_of: collective:{action_hash}` | **action-gate extension (§5.3)** |
| Kid device stewardship | A (notarized) | StewardshipGrant (recovery Phase 2 primitive, exists) | grant tuple | first non-recovery instantiation |
| Interim fixtures | C (operational, explicitly fake) | — (rea_commitments rows, `dht_anchor_hash: NULL`) | — | provenance marker + retirement gate (§7) |

Anti-patterns caught and corrected by the gate: the current `/db/collectives` write
path is REST-first/projection-only (household writes move to the conductor);
custody-blob's diesel-direct path is a live missing-source-of-truth declaration
(fixed by §5.3); slug-vs-hash dual identity declared (alias at the edge).

## 3. The ceremony — a device-setup story (deterministic floor)

Formation is woven into first-run device onboarding, not an admin task.

**Founder (matthew, first device):** first-run → "Begin your household" → names it →
`create_collective` fires on his conductor (founder Steward membership atomic,
household character in charter, intended-intimate reach) → his screen becomes the
invite surface: single-use, founder-signed, TTL'd tokens as QR / LAN / deep-link.

**Adult member (jessica):** first-run → "Join your household" → scans QR →
`affirm_membership` fires on **her** conductor (sponsor chain in the token) → her
ambient custody participation activates: her agent authors "I provide custody-blob
for" each already-affirmed member; reciprocal halves complete **asynchronously** —
each existing member's device, on observing her `MembershipAffirmed` signal,
auto-authors its reciprocal (consent to ambient custody was given at their own
affirm; per-pair opt-out honored locally before authoring). Pairwise reciprocity
emerges from the set of affirmations; nobody sequences an N×N handshake
(intent-first / observed-state-second, per the dwelling-hub pattern). → One explicit
screen: "Delegate compute toward [hub-ish node]?" (suggested by archetype, never
assumed) → done.

**Kid (james):** first-run with a parent present → parent's sponsor QR →
`affirm_membership` on **james's** conductor (his agent, his identity, `sponsor_cid`
= parent) → StewardshipGrant minted parent→james (device policy; `age-bounded,
capacity-conditional` bounds vocabulary from the rea-compute guardianship row) →
custody ambient like everyone; the compute-contribution consent goes to the
**parent's** device.

**Minute one:** every affirmation immediately lights the cluster/reciprocity views
(via the DeliveryPeer enrichment, §5.5): member tiles appear, custody pairs draw in
as reciprocals complete, "2 of 3 affirmed" shows as honest partial state, first
stewarded bytes flow over the LAN (`network: "lan"` already on DeliveryPeer). The
view IS the formation progress.

**Hub-optional floor:** a one-device household forms fine — household awaiting its
members; zero custody pairs until a second affirmation; no delegation screen.

## 4. Coordinator contracts

### 4.1 `imagodei::affirm_membership` (new)

- input: `{ invite_token, collective_cid, role }`
- validates: token signed by a current Steward of the collective; TTL unexpired;
  single-use (token-hash link on the Collective; replay rejected); for minors,
  sponsor holds Steward membership and the StewardshipGrant is created atomically
  with the membership.
- creates: `Membership` authored by the **caller's agent** (member_kind: Person,
  role from input — Steward for adults of the household rubric, Contributor for
  sponsored minors), `sponsor_cid` from token chain.
- emits: `MembershipAffirmed` post-commit signal.

### 4.2 `create_collective` household-init (existing coordinator)

Charter/rubric carries household character + the rubric's membership-acquisition
flow selection (recognition-of-given). Expected: no signature change — verify
charter field capacity rather than extend (verification flag V1, §11).

### 4.3 Ambient-custody responder (client/runtime, not zome)

On `MembershipAffirmed`: each already-affirmed member's device authors its
reciprocal `custody-blob` commitment unless that pair is locally opted out. The
existing `mutuality_audit_service` detects never-completed halves
(reciprocity-imbalance FeedbackSignal) — no new audit machinery.

## 5. Substrate deltas

1. **`affirm_membership`** coordinator (§4.1) — the largest genuinely-new piece.
2. **Household-init charter** (§4.2) — verify-only, likely no change.
3. **Action-gate extension** — `rea_commitment_service.rs:45` routes only
   `project-epr` through `create_via_conductor` today; `custody-blob`,
   `delegates-compute`, `replicates-dwelling` join it. Resolve the three documented
   wire-shape divergences (`medium_of_exchange_id` storage-only; `in_scope_of`
   `Option<String>` vs `Vec<String>`; f32 vs f64). **Cross-cutting dependency:
   without this, ceremony commitments land unanchored.**
4. **Projection wiring** — collectives projection gains `dht_anchor_hash` via
   `CollectiveCreated` / `MembershipAffirmed` signal handlers (the gate's textbook
   "missing anchor in projection" case).
5. **DeliveryPeer enrichment patch applied** — `household_id` via
   `hub_resolver::resolve_peer_dwelling_hub`, `commitments` via
   `active_provide_reaches` (prepared patch verified clean-applying; soft-fail
   enrichment, never a 500).
6. **StewardshipGrant instantiation** parent→kid at formation (verification flag
   V2: grant-entry writability vs recovery Phase 2b stub scope).
7. **`conductor_writes` wrappers** (`call_create_collective`,
   `call_affirm_membership`) — needed only for the stage-2 doorway path; stage-1
   seeder drives per-conductor WS directly.

## 6. Drive architecture (realism ladder, staged; same choreography every stage)

- **Stage 1 — seeder as ceremony driver (lands with this spec):** new
  `seed-household-formation.ts` on the existing per-conductor harness
  (`seed-conductor-identities.ts` pattern; Jenkins `runProbedSeeder` probes per-
  persona admin WS): matthew `create_collective` → jessica affirm → james affirm +
  grant → per-agent custody reciprocals → optional delegates-compute toward the
  hub-ish node. Runs after identity/binding seeding. **Genesis data becomes "this
  family ran the ceremony"; CI exercises the real multi-agent path nightly.**
- **Stage 2 — doorway headless persona auth + thin proxy (production-shaped):**
  the one unspecced auth piece: a headless grant (dev posture:
  `adminBootstrapKey`-gated `/auth/service-login` issuing the same JWT a browser
  login yields) + doorway proxying ceremony calls to the persona's conductor via
  the proven `AgentProvisioner` machinery (`auth_routes.rs:704` pattern). **Zero
  ceremony state in doorway** (D4 intact). Browser a2o scenarios and remote
  members drive here.
- **Stage 3 — service-agent standing (admin-key displacement):** the seeder/CI
  agent holds `delegates-compute` commitments (scope: `run-formation-ceremony`,
  `seed-fixtures`) granted by operator/personas — bounded, revocable, validated by
  the existing seven-check `bounds_validator`. X-API-Key retires on this surface
  per admin-key-lifecycle Stages 3–4. The test apparatus's authority is modeled in
  the substrate it tests.
- **Ceiling — elohim facilitation (held, designed-for):** narrates, sequences,
  optionally attests completion; never gates; floor remains fully operable.
- **Standing policy (adopted):** every seed module header declares its **rung and
  why**. Corpus = rung 0 forever by design; inter-party agreements / consent /
  identity-bindings = rung 3 mandatory; content authorship/attestation splits off
  the corpus and climbs. Convenience visible, never ambient.

## 7. Interim fixtures + retirement gate

- Triad bundle seeded at rung 1 now; every row self-describes:
  `metadata_json: {"fixture":"formation-output","retireAt":"ceremony-landing"}`.
  Views MAY badge fixture provenance (optional).
- **Hard landing condition on the ceremony PR:** fixture seed path deleted;
  household custody pairs come only from `seed-household-formation.ts`; validation
  scenarios assert ceremony output. Not a follow-up — a gate.
- The M1 matthew↔jessica named-pair scenario survives unchanged in assertion,
  changing only in provenance; its anti-drift duty transfers seamlessly.
- Mechanical: fix `collectives.json` family-dowell "Terrance"→James when the
  fixture work touches that file; fix the `genesis/Jenkinsfile` ~line 1815 echo
  when that stage is next touched.

## 8. Household quiltPolicy class

- Declared as `vocabulary.quiltPolicies.household` in the **qahal domain manifest**
  (household is qahal-owned vocabulary; other pillars reference `qahal/household`).
  Verification flag V3: confirm the qahal manifest file exists to receive it; if
  not, this spec's plan creates it with this block as its first entry.
- Shape: `defaultTierFloor: stocked` for household-reach content; `holdWarmMin`
  elevated for recovery-critical bundles; `preferDestinations:
  ["peer-cellar://household/{any}", "federated-dwelling://family/{family-id}"]`
  (targets the tiered-quilt §4 v0.2 amendment already names).
- **The pledge-clamp tie:** §4's clamp requires a declared floor be backed by a
  steward's active pledge tier — the ceremony's custody-blob commitments ARE those
  pledges, through the CommitmentFactory gate. Policy and ceremony validate each
  other.

## 9. Scenario architecture

**Spine — `genesis/a2o/features/qahal/household-formation.feature` (new):**
- Matthew begins a household (household character at init; honest singleton state)
- Jessica affirms into the household (own-agent membership; reciprocals complete
  asynchronously; the pair draws itself on the reciprocity view)
- James affirms with a sponsor (own agent + sponsor_cid + StewardshipGrant; parent
  consent for compute)
- Partial formation is honest ("2 of 3 affirmed")
- An invite cannot be replayed / an expired invite declines gracefully
- A member opts out of one custody pair
- Delegation is never ambient (explicit opt-in only)
- A one-device household forms (hub-optional floor)
- Tags: `@requires:household-nodes` (adopted as the convention — declared in
  cluster-state, mechanically gating, previously used by zero scenarios) +
  `@stage1-structural`; `@browser-only` variants arrive with onboarding UI.

**Consequence layer — `features/resilience/household-reciprocity.feature` grows the
five validation intents** (steady-state mesh · james-contributes-compute ·
member-offline continuity · reciprocity view · **grandma-standard recovery**: 2-of-3
household quorum, under five minutes, no seed phrase — the executable home for
`social-recovery-with-help-from-family`; intimate-circle leg on household-nodes,
cross-doorway leg stays shem-gated). M1 named-pair flag persists.

**Corrections (land with this spec):**
- `features/shefa/human-resilience.feature`: scenarios "Matthew + Susan — household
  reciprocation", "Maria builds resilience through first connection", "Degradation —
  Matthew goes offline", "Recovery — after-action review" retag `@requires:shem` →
  `@requires:household-nodes`. "Matthew + Susan + Pete" splits (household arm →
  household-nodes; congregation arm keeps shem). "Full network — 5 conductors"
  keeps shem (genuinely multi-tenant).
- `held/features/lamad/love-map-negotiation.feature` → `@requires:household-nodes`
  (verified single-doorway dyad), returns to live.
- **Persona reconciliation**: household scenarios converge on the canonical triad
  (matthew/jessica/james) per the named-pair anti-drift principle.

## 10. Edges

- Member declines an invite → no entry; absence is the record.
- Founder device dies mid-formation → collective + memberships persist on DHT; any
  Steward re-issues invites; resume = read what's affirmed, continue.
- Duplicate affirm → tuple identity rejects; idempotent.
- James ages up → StewardshipGrant supersession chain, never reset (the Jasmine
  principle; designed-for, built in the capability-arc work).
- Household split / dissolution → held seams-work (lattice §6); cold-archive
  terminus gap noted in MAP Gap Ledger.

## 11. Testing & verification flags

- Sweettests: `affirm_membership` (two-conductor create + affirm + signal + replay
  rejection); action-gate extension (custody-blob anchors; wire shapes reconciled);
  StewardshipGrant instantiation.
- The stage-1 seeder step is the nightly integration test of the full choreography.
- Schema contract tests for any view additions; mutuality-audit asserts reciprocal
  completion; DeliveryPeer enrichment covered by the patch's included tests.
- **V1**: charter field capacity for household character (no coordinator signature
  change expected). **V2**: StewardshipGrant entry writability vs Phase 2b stub
  scope (stubs gate recovery *authority paths*, believed not the grant entry).
  **V3**: qahal domain manifest existence for the quiltPolicies block.

## 12. Build order (dependency-ordered, from the seed-realism audit)

1. `affirm_membership` coordinator (+ sweettests)
2. Action-gate extension at `rea_commitment_service.rs:45` (+ wire-shape
   reconciliation)
3. Projection wiring (`dht_anchor_hash` on collectives; signal handlers) + apply
   DeliveryPeer enrichment patch
4. `seed-household-formation.ts` (stage-1 driver) + interim-fixture retirement
5. `household-formation.feature` spine + retags + persona reconciliation
6. quiltPolicy `qahal/household` class + pledge-clamp tie
7. Stage-2: headless persona auth (`/auth/service-login`) + doorway thin proxy +
   `conductor_writes` wrappers
8. Stage-3: service-agent `delegates-compute` standing (admin-key displacement)
