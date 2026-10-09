---
title: The elohim-operator loop when a peer wedges — refuse and announce, govern in absence, relieve
id: elohim-operator-loop-wedged-peer-design
status: Draft
class: architecture
serves: pain-is-answered
date: 2026-10-09
context-tier: disclosed
steward: human:matthew
graduation-trigger: on the household mesh a peer that reaches its band edge writes an `approaching` state on its own pledge, neighbours pre-position without moving a byte, the peer's conductor is stopped and the pre-agreed relief executes with a recorded gate-decision that reaches the peer's steward, a sweep from a neighbour receives `unverifiable` rather than a cached head, and a carried successor head stays beneath the owner's earned head until the owner returns
cites:
  - "algedonic-feedback-signal | the algedonic ontology (Approach/Breach as a viability feedback type, evidence stock/limit/bound_ref, CounterEvidence floor) this design carries to a DHT home; its §3 hash-neutral claim corrected here | sha256:cf902beac8dd4aec | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md"
  - "device-footprint-residency-carrying-capacity-design | the band shape (own/dwelling/collective/commons, a full band refuses by name) the stock is measured against; this design adds the runtime band and the band × primitive cell | sha256:99ab8f8330cc3f24 | path: genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md"
  - "compute-envelope-tevah | the berth-offer → commitments-rebalanced chain (\"every party to the rebalance is a party to a commitment\") that relief reuses, and the ark envelope whose ResourceQuota declares the per-primitive bounds | sha256:d427a367f3b01a6e | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "stewardship-over-sovereignty | the canon that authority is qualified and witnessed through commitments and that the social quorum proceeds when the cryptographic one cannot — the frame for governing in a peer absence and for the counsel/operator roles | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "cradle-to-grave-capability-gradient | the recovery-authority ladder (intimate quorum → community → governance act → network witness) the Refer route pins its layer names to; Flow 2 is the absent-adult case verbatim | sha256:5aa31bed9f44db8c | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md"
  - "elohim-seam-map-concern-routing | routes resource governance (§3.15) and the algedonic channel (§3.16) to the elohim-operator; this design is that seat made concrete | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "substrate-trust-contract-runbook | the probes and the per-red decision tree the seat reads today by hand; the half-alive seam (storage answering with a cached head after its conductor died) is named here as a C4 answer | sha256:eb5f8342e17c361f | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - genesis/a2o/features/dataplane/operator-answers-pain.feature
  - genesis/a2o/features/stewardship/collective-steward-answerable.feature
  - elohim/elohim-storage/.epr-meta/pain-is-answered.habit.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - elohim/elohim-storage/.epr-meta/liveness-has-one-source.habit.md
  - elohim/elohim-storage/.epr-meta/blob-durability.habit.md
  - elohim/holochain/dna/imagodei/.epr-meta/custodial-authority-answerable.habit.md
  - elohim/epr/src/verdict.rs
  - elohim/epr/src/algedonic.rs
  - elohim/elohim-storage/src/services/holds.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
  - elohim/elohim-storage/src/liveness_contract.rs
  - elohim/ark/core/src/verdict.rs
  - genesis/data/timeline/backlog/alpha-adam-peer-meta-store-disk-io-error.md
---

# The elohim-operator loop when a peer wedges

**Operator direction (2026-10-09), in the operator's own frame.** When adam's and
gertrude's conductors wedge: (1) they should not accept something that will wedge
them — stop, refuse, and let peers know; (2) the network should not stop governing
because adam is offline — recognise he cannot decide, and have a process (next of
kin, a peer with affinity, a third party) affirm that matthew may continue; (3)
peers should be able to step in, offload something from adam or gertrude, and
rebalance. And the frame around all three: *anything not working pleasantly should
be able to invoke an agent that figures out what is going on, plans, implements and
resolves* — the elohim-operator, acting **through** the system and making mishpat
as it goes, rather than a human reading logs and driving the system from outside.
**This session was that loop run by hand.** The spec transcribes it.

Fear is the right reading of the band edge. Approach is "keep an eye out for me,
something is off": peers raise care in proportion to a limit. Breach is "something
broke": peers step in, because they were already watching and knew what to do. And
the pool of support that makes a novel commitment a reasonable risk instead of a
gamble is the aggregate of the bands the household enrolled — gertrude's dashboard
showing 80G "free" while she passively supplies 20G to the commons is that shape in
use.

## §0 — The trace is the specification

| Step | What we did by hand this week | Substrate object it becomes | Exists today? |
|---|---|---|---|
| Sense | Read `/p2p/status.provideLoop` (dead 16, failed 833), genesis #1637, the ZFS quota line in the backlog, 9–10 crash-loop restarts | `approaching\|ts` / `breached\|ts` state links on the peer's own pledge commitment, written while it still can (the Approach is the last self-authored act); an ark death witness crossed by the household into a notarized attestation; third-party `attestation:device-health{unreachable}` as corroboration only | Local types only (`elohim/epr/src/algedonic.rs`). No DHT carrier. No free-space probe on the conductor dataset. The death witness is ingested as private content. Attestors are doorways only, issuer accept-all. |
| Orchestrate | Named parties (adam, eve, gertrude, matthew, jessica, the hardware steward), remedies (quota, prune, restart, conductor-roll, relief), ordering | A relief proposal (governance-proposal content-type on the lamad DNA, voted by `attestation:proposal-vote`) naming parties, remedy, scope, window — "every party to the rebalance is a party to a commitment" (tevah) | Proposal/vote machinery exists; no relief kind, no producer. |
| Judge | Decided which moves are ours (code) vs the operator's (quota) vs blocked | `Decision{Permit, Refuse, Refer(layer, reason)}` (`elohim/epr/src/verdict.rs`); the seat's standing authority bounded by an `operates-household` Mishpat Commitment | Verdict type exists; `ReferQuestion.layer` is an open string nothing serves; no seat commitment. |
| Resolve | Pushed the announce seam; filed the backlog; left the quota to the operator | Neighbours' own `custody-blob` commitments (salvage); a collective-held `attestation:stewardship-grant` for carry-forward; `attestation:steward-of-record-transfer` for a dead doorway slot | Salvage never fires on a wedged-but-gossiping peer. Grant schema lacks `authority_basis` and the absent subject. Transfer kind exists. |
| Inform | RUNBOOK entry 21:25Z, habit DELTA, this chat | `attestation:gate-decision` (allow / block / pending) with `AttestationToSubject` links to each party, delivered on the `CounterEvidence` floor to each party's counsel | Kind exists; routing to counsel absent; floor class unassigned outside `trust/pricer.rs`. |
| Settle | Nothing settled; adam's unfulfilled pledge and matthew's extra carrying unrecorded | REA: a reliever's custody fulfils its own commitment (exists); the Breach state IS the under-delivery against the pledge; the ledger settles by construction | Provide/custody events exist; no Breach state on the pledge today. |

The dev-plane half of the loop already runs and ran this session: `runtime-harvest`
captured a self-reported exhaustion fingerprint, the hook dispatched `runtime-triage`,
the agent did the RCA from `/p2p/status` and the backlog, found the cause (adam's and
eve's datasets at zero free) and stopped at "blocked on an operator disk action" —
exactly the point where the protocol has no actuator. The C14 context capsule is the
RCA input; today the agent reconstructs it from three sources.

## §1 — Operator controls as system code

This week's hand actions, and the actuator the seat needs for each. The human
operator is the C13-labelled scaffold at every gate; the seat under its commitment is
the named successor.

| Hand action (human, outside) | Actuator inside the system | Home | Exists? |
|---|---|---|---|
| Notice adam is wedged (logs, Jenkins, `/p2p/status`) | Breach state projected → seat dispatch with a C14 capsule | storage projector + seat dispatcher (same shape as `runtime-harvest` → agent) | capsule partial (`runtime-findings.jsonl`); seat absent |
| `zfs set refquota=20G quota=none` on shem | `hardware-custody` ceiling change by the hardware steward, requested by the seat via Refer to `hardware-steward` | ark/tevah envelope + `attestation:hardware-custody` (schema exists) | absent (no ark actuator) |
| Delete the pod / restart the conductor | grant-gated restart command on the supervising ark | `elohim/ark/core/src/verdict.rs` — `RestartRequest`/`RestartGrant`/`RestartGovernor` already decide a restart from a `DeathRecord` under `ChildPolicy`; `rea.rs` projects an open incident as REA | partial: self-granted on death only; no seat-sent `RestartRequest` for a child that is alive but wedged |
| `[conductor-roll]` push | CI dispatch tag | dev plane | exists |
| Prune compiled wasm | inline on `put_dna_def` (fork `3c1e80525`) | conductor | exists; does not shrink the file |
| Ask peers to hold more | salvage self-selection firing on an honest "conductor unreachable" | `reconcile/custody.rs` + inventory gossip | absent (detection gap) |
| Decide who may carry adam's rows | Refer → proposal → collective-held grant → carried head at staging | lamad DNA + coordinator cross-cell verification into imagodei | absent |
| Tell adam/eve/matthew what was decided | `attestation:gate-decision` on the `CounterEvidence` floor to each party's counsel | lamad DNA + floor routing | kind exists; routing absent |
| Record who carried what for whom | REA events against commitments | mishpat + `rea_economic_events` | exists for custody; Breach state absent |

## §2 — Two stages, two consumer contracts

Approach and Breach are not two severities of one alarm. They are two requests with
two obligations.

**Approach — "keep an eye out for me."** Consumers raise care in proportion to
`threshold_pct`, and nothing moves: watchers raise their probe cadence on that peer;
the closest-N for its custody is pre-computed and each candidate pre-authors (does
not activate) its `custody-blob` intent; the seat drafts the relief proposal and the
parties — **including the peer in pain** — sign it now, while its conductor can
still sign. The Approach is the only window in which the wedged peer is a party to
its own relief (C12 consent at the acting node, obtained before it is needed).
Proportional care is the hysteresis: at 70% watch, at 85% pre-position, at 95%
refuse new carrying.

**Breach — "something broke."** Nobody decides anything new. The watchers execute
the plan they signed: pre-authored custody intents activate, the seat's
gate-decision cites the Approach-time proposal, the parties are informed of an
outcome they already consented to. A Breach with no Approach-time plan (sudden
death, no band edge) takes the slow path: Refer up the ladder with the evidence set.

Consequences: the relief proposal is minted at Approach, not Breach; a peer that
never declares Approach gets the slow path, which is the incentive to declare early;
pre-positioning is bounded work (closest-N for one peer's holds), never a fleet
sweep; the feature carries both paths and the household can stage both.

## §3 — Bands, the commons pool, and risk

The stock a peer measures pain against is the **band** it enrolled, not "the disk" —
and **compute is the whole profile**: disk, memory, CPU, bandwidth, and whatever
else the device's archetype declares. The footprint spec §5 declares the band shape
— one derived ceiling per device, split own / dwelling / collective / commons, "a
full band refuses new commitments by name and signals the steward" — and the ark
envelope already declares the primitives: `RuntimeEnvelope.bound` is a
`ResourceQuota{cpu_millis, memory, disk_bytes}` (`elohim/ark/core/src/manifest.rs`),
with bandwidth to be added beside them. A stock is therefore one cell of
**band × primitive**, and the evidence names both (`stock.kind ∈ {disk, memory, cpu,
bandwidth, …}`). This week's two wedges were two different cells: adam's and eve's
`runtime × disk` (dataset at zero free) and the hosted-agent heap at ~786 MB per
hosted human (`runtime × memory`), which is why "grow the disk" alone is not the
cure for gertrude.

**Two instruments, two jobs — do not confuse the nerve ending with the surgeon.**

- *Sense in production = the gauges against declared bounds, on the peer, every
  tick.* The store-footprint walk (300 s), the capacity reporter's free-space read,
  the holds gauges, the conductor's own `hc_*` series on its metrics port, and the
  ark envelope's declared `ResourceQuota` are per-primitive stocks measured
  continuously at near-zero cost. Approach and Breach are derived from those against
  the enrolled band limits, nothing heavier; this is what writes `approaching|ts` on
  the pledge. Cheap, always on, **fail-closed to Unmeasured** — a reading that is
  missing, truncated or censored is reported as such, never as zero or as 100% free
  (the C4 rule the current free-space probe violates).
- *RCA on dispatch = the runtime-performance capture*
  (`genesis/a2o/scripts/runtime-performance.ts`, `resource-profile.json`), run by
  the seat's agent only after a Breach or a Refer, bounded and authorized per run
  (target, workload, duration, byte budget), to answer "which cost concentrated
  where." Its own text says it is "discoverable guidance, not a daemon, alert
  subscription or Jenkins stage." It is the elohim-operator's instrument, not the
  peer's nerve ending, and the C14 capsule the seat hands the RCA agent is that
  bounded capture plus the pledge it threatens.

No parallel pain pipeline on either side.

Three consequences:

1. **`bound_ref` is the enrollment pledge.** The capacity variant of
   `replicates-content` (`services/conductor_commitment_author.rs`, the byte pledge
   with no counterparty) IS the commons band's declared limit, and the Approach and
   Breach states hang on it. Each band has its own pledge and its own pain. The
   conductor dataset is the `runtime` band and has no declared disk limit today —
   that is the cell adam and eve breached; the hosted-agent heap is the `runtime ×
   memory` cell gertrude's slot sits in.
2. **The aggregate pool is the seat's allocation input.** Σ(enrolled commons bands)
   − Σ(held) across the household is what the household can allocate without anyone
   taking a risk. `SalvageCapacityAd` (`p2p/salvage_gossip.rs`) already gossips
   per-peer capacity; the fold over those ads is the pool. Pre-positioning draws only
   from it, so relief never pushes a neighbour toward its own band edge. The shefa
   view shows the steward both numbers: free-in-theory (own band) and
   passively-supplied (commons band, held against pledged).
3. **Risk is affordable in proportion to support.** A novel commitment is admissible
   when the pool's slack covers its cost; the band edge is where support runs out.
   The seat's Permit set is bounded by pool slack, not by a fixed list alone: an
   adds-holders move that would take a neighbour past its own Approach is a Refer.

## §4 — P2P design gate

### E1 — Algedonic state (Approach / Breach / Recovered)
- **Classification:** Linked (A2). Pain is a state of the promise it threatens, so it
  rides the pledge Commitment as a `CommitmentByState` link
  (`mishpat_integrity/src/lib.rs:385-396`: `EntryHash(Commitment) → ActionHash`, tag
  `state|signed_at`, shape-validated only) with tags `approaching|ts` /
  `breached|ts` / `recovered|ts`. The evidence (`stock{kind, value, unit}`, `limit`,
  `bound_ref`, `threshold_pct`) is the link target, a small signed record authored by
  the peer; `stock.kind` names the compute primitive (disk, memory, cpu, bandwidth)
  and `bound_ref` the envelope or pledge that declared the limit (§3).
- **Justification:** the community must witness a peer's declared pain (it is
  counter-evidence against the peer's own pledge), and it has no meaning without the
  pledge — A2 by the gate's own test.
- **Head-plane cost:** no new head; one link per transition, at most three per
  incident; 1-year order 10² fleet-wide. The unbundled shape to avoid is on record:
  27,317 of 30,787 lamad entries were doorway probe attestations.
- **Network stakes:** all four stages; floor `CounterEvidence` — a peer's pain is
  never priced down.
- **Address:** the link's ActionHash; the evidence record is content-derived. Dedup
  keys stay bare fingerprints, persisted in SQLite so a restart does not re-emit
  (`should_emit` in `elohim/epr/src/algedonic.rs` is caller-held memory).
- **Zomes:** `mishpat_integrity` untouched; coordinator `mishpat` writes the link.
  DNA-hash-NEUTRAL. (The FeedbackSignal path in the 2026-08-10 spec is
  DNA-HASH-MOVING: `SIGNAL_KINDS` is validated inside `content_store_integrity`.
  `EconomicEvent` is on the lamad DNA, not mishpat; its `action` is unvalidated
  beyond non-empty, so a settlement event there is also hash-neutral if wanted.)
- **Projections:** `mishpat_commitments` state write-through (exists); Automerge:
  n-a (link).
- **Route:** none new; `GET /p2p/status` gains the open-pain summary from the
  projection.
- **The Approach is the last self-authored act.** A Breach is a source-chain write on
  a full dataset and fails on the wedged peer by construction; the conductor dataset
  grows by DHT ingest the arc controls, which storage admission cannot throttle (no
  arc lever on this conductor line). "Reserve" is therefore declared, not enforced.
  Absence evidence is the subject's own signed Approach (unforgeable by colluders) or
  an ark `DeathWitness` crossed by the household into a notarized attestation (today
  private content, `services/spool_ingest.rs`). Third-party
  `attestation:device-health{unreachable}` is corroboration only: issuer
  authorization is accept-all (`attestation_validator.rs:66-69`) and the roster is
  doorways.
- **Refusal (C11):** no `DeferReason` exists in storage. The refusal reuses
  `policy/evaluator.rs` (`accepting_stewardship_reserves`, `accept_general_traffic`
  from `min_free_storage_pct` / `max_storage_pct`); the missing input is the
  conductor-dataset free-space probe, fail-closed as Unmeasured (today
  `heartbeat.rs` defaults to 100% free on probe failure). Counted on the existing
  shed counters with `reason=capacity`.
- **SDO/RWA:** the worst holder sees that a peer is at capacity; stock and limit are
  bytes, never content; reach is the pledge's reach. Boundary 6 bounds it.
- **Anti-pattern check:** none apply; the per-host "mode" write is explicitly not
  this (the state is authored once by the peer through its conductor).

### E2 — Absence judgment, relief proposal, carry-forward grant, the act
- **The absence itself is not an entity.** `liveness-has-one-source` requires that no
  code path write a liveness fact to a chain. Absence is a witnessed *judgment* that
  cites evidence (E1), never a presence fact.
- **Relief proposal:** Notarized (A) via the existing governance-proposal
  content-type on lamad, voted by `attestation:proposal-vote`. One per incident.
  CID-addressed. Minted at Approach (§2).
- **Carry-forward grant:** Notarized (A) via `attestation:stewardship-grant` (lamad,
  existing kind; integrity checks kind membership only, so new fields
  `authority_basis` — closed set plus `absence-quorum` — and `absent_subject` are
  integrity-neutral). Grantor = the household Collective, grantee = the acting
  steward member, scope `content:<head_ref>…`, capability `declare-successor-head`,
  `expires_at` and `review_at` mandatory. **Cross-DNA:** `Collective` and
  `Membership{role: Steward}` live on imagodei; `must_get_*` is DNA-local, so C12
  verification at the receiving peer is a coordinator cross-cell call (precedent
  `mishpat/src/device_enrollment.rs`, `CallTargetCell::OtherRole("imagodei")`), not
  integrity validation. Named weakness, not papered over: membership validation
  binds nothing to the author today. The act names the member — the same write path
  the custodial-authority habit's defect (4) needs.
- **The act reaches staging only.** The live election
  (`content_store/src/lib.rs:3199-3213`) is `ArbitrationKey{tier: is_earned, clock,
  tiebreak}`, author-filtered; the earned tier is restricted to the root author, a
  device it delegated, or the progenitor (`:6197-6300`). The carried successor is a
  `declare_canonical_head` at staging tier, surfaced as the staging candidate beneath
  the earned head (`:2747-2766`, `declares_lineage_over` `:6073`): visible and
  servable, never crowned. **The owner's return wins** by tier and clock exactly as
  today, cited or not. Promotion to earned is the owner's own citing declaration or a
  top-layer gate-decision executed via the progenitor path. The carrier's contest,
  if the return discards the carried work, is the appeal — whose admission today
  accepts any caller with an `advocate_id` (`imagodei/src/stewardship.rs:1175`),
  a one-line guard.
- **Never-returns arm (C3):** the Breach state, the dead pledges (re-loaded as live
  wants on every salvage pass) and the grant (re-minted each expiry) need a terminal
  state. It exists: a collective-authored `Membership.withdrawn_at_block_height`
  plus `attestation:steward-of-record-transfer{transferReason: incapacitation,
  authorizationPath: succession-quorum}` closes the state with `withdrawn|ts`,
  supersedes the pledges, and promotes the staging successor via the gate-decision
  path.
- **Head-plane:** proposals one per incident; grants one per incident per window;
  carried heads one per surface. 1-year order 10².
- **SDO/RWA — the cheapest capture and what stops it.** Matthew plus one accomplice
  mint two `device-health{unreachable}` about adam (accept-all issuer, free doorway
  registration, self-asserted household column), a grant citing the collective CID
  (no issuer check), and a successor declaration. *Theatre:* "two attestations from
  distinct households" (identities are free to mint), "the grant expires" (re-mint),
  "M-of-N witnesses" (free keys), "the appeal is floor-protected" (the appellant is
  absent by hypothesis). *What actually stops it:* the earned tier is root-author-
  signed, so the capture lands at staging and never displaces adam's earned head —
  which is why the carried head MUST stay at staging; and the subject's own signed
  Approach is the one evidence class colluders cannot forge. The act only adds,
  never revokes, never signs as the subject. **A design that promotes a carried head
  to earned on third-party evidence is a refusal.**

### E3 — Relief
- **Conductor reachability in inventory gossip:** Ephemeral (C), a transport-level
  field `conductor: Reachable | Unreachable | Unmeasured` on the salvage capacity ad
  (short TTL; consistent with one liveness source). Salvage's "honored" predicate
  (`reconcile/custody.rs:458-520`, today: fresh inventory rows) becomes fresh
  inventory AND conductor reachable; `Unreachable` held N rounds → custody
  unverifiable → not honored → salvage fires. No DHT write. This is the detection
  gap: a wedged peer's storage keeps gossiping fresh inventory, reads as honored,
  and salvage never fires.
- **Reliever custody:** the existing `custody-blob` commitment, authored by the
  reliever for itself. Nothing new. Fills-never-moves holds: a neighbour can only
  add, never retire the wedged peer's root-author-only rows.
- **Withdrawal on return:** no new hold rung. The owner authors `revokes-commitment`
  on its own custody rows → row withdrawn → blob drops to `OwnUnnamed` →
  `may_let_go` (`services/holds.rs`). The one addition: the retention tick checks
  replicas ≥ `salvage_target_replicas` before the owner revokes.
- **Hosted-agent re-homing** (gertrude's dead slot; ~786 MB conductor heap per hosted
  human): `steward-of-record-transfer` plus a compute grant. Frontier — named, not
  designed here. **DHT arc relief:** no lever on this conductor line; the arc resets
  to Empty on every restart and promotes only after a clean gossip round, so a
  crash-looping conductor is never an authority. Frontier.

### E4 — The operator seat and the gate-decision
- **Born narrow, widened only on evidence.** The seat's first commitment permits
  alone ONLY moves that add holders and remove nothing from anyone: relief custody,
  the announcement, informing parties. Every other move — restart, ceiling change,
  carry-forward of a human's heads, anything on hardware — is a Refer, and the top
  of the ladder is the human operator. Widening is a habit flip with evidence, never
  a config edit. The rollout changes who reads the logs first, not who decides.
- **Seat authority:** a Mishpat `Commitment{action: "operates-household"}`
  (coordinator-only new action; `delegates-compute` is the template) naming the
  Permit set, the Refer set, and bounds including pool slack (§3). C13 label at the
  gate: scaffold = human operator; successor = the seat.
- **Decision record:** `attestation:gate-decision` (exists; `gate_kind` free-form)
  with `gate_kind ∈ {capacity-refusal, absence-carry-forward, custody-relief}`. ONE
  entry per decision plus `AttestationToSubject` links to each party, never one per
  party. Delivered on the `CounterEvidence` floor to each party's counsel. 1-year
  order 10².
- **Refer route and the liveness hole (C3):** `ReferQuestion.layer` is pinned to the
  `RecoveryAuthority` ladder names (`intimate-quorum`, `community-consensus`,
  `governance-act`) plus `hardware-steward`; a Refer becomes a proposal addressed to
  that layer; unanswered past its window it escalates one layer; at the top layer,
  expiry mints `gate-decision{decision_outcome: "pending"}` — a witnessed
  non-decision with a C14 capsule, counted as `refer_window_expired_total`. The
  ceiling law holds: an expired Refer never collapses to Refuse. The human is a
  reader of pending decisions, not the only exit.
- **`unverifiable`:** a storage whose conductor is unreachable answers
  `unverifiable` for its heads, never a cached head (C4), and that arm joins the C3
  table in `elohim/elohim-storage/src/liveness_contract.rs` (today only `Timeout` is
  modelled as transient) so the harness cannot pass while lying.

## §5 — Mintable nodes (the deliverable's heart)

```
chain dataplane-convergence / between "peer at capacity" → "peers converge without it"
  missing node: the `runtime` band (conductor dataset) has a declared limit and its own pledge commitment as bound_ref / probe: elohim_node_store_free_bytes{scope=conductor} exists, is never Unmeasured-as-100%, and bound_ref resolves / state: absent
  missing node: household commons pool = fold over SalvageCapacityAds, read by the seat (allocation) and the shefa view (free-in-theory vs passively-supplied) / probe: gertrude's two numbers reconcile to her enrollment pledge / state: absent (ads exist)
  missing node: Approach at band edge declines NEW carrying (custody, pledge, hosted registration) by name, counted / probe: the capacity-labelled shed counter climbs before the dataset is full / state: absent (admission is concurrency-only)
  missing node: Approach is notarized as `approaching|ts` on the pledge while the peer can still write; it is the last self-authored act / probe: the link's dht_anchor_hash is set on a neighbour; open-signal key persisted so a restart does not re-emit / state: absent (no DHT carrier)
  missing node: a death witness is crossed by the household into a notarized attestation / probe: kill a household conductor; a neighbour holds a notarized record citing the witness / state: absent (private content today)
  missing node: a storage whose conductor is unreachable answers `unverifiable`, never a cached head, and that arm is in liveness_contract.rs's C3 table / probe: a2o — kill the conductor, sweep from a neighbour, assert no head adopted / state: absent (the half-alive seam, adam this week)
chain custodial-authority-answerable / between "steward member" → "act names the member"
  missing node: collective-held grant over a content surface with authority_basis absence-quorum, verified at the receiving peer by coordinator cross-cell call into imagodei / probe: a carried head arrives with a grant the receiver resolves to a steward member / state: absent (grant authorizes device flags only)
  missing node: the carried head reaches STAGING only and is surfaced as the staging candidate; the subject's return wins by tier as today / probe: a2o — carry, return uncited, assert the earned head is the subject's and the carried work is visible beneath it / state: absent (no cross-root staging declaration path for a steward member)
  missing node: Refer route — a Refer lands as a proposal at the named layer, escalates on timeout, and at the top layer expires into gate-decision{pending}, counted / probe: ReferQuestion.layer resolves to a producer; an unanswered Refer yields a witnessed pending / state: absent (open string, nothing serves it)
  missing node: never-returns arm — collective-authored Membership withdrawal + steward-of-record-transfer{incapacitation} closes the breach, supersedes dead pledges, promotes the staging successor / probe: a2o on the household / state: absent
chain blob-durability / between "custody honored" → "re-placed without loss"
  missing node: honored requires conductor reachable; Unreachable held N rounds fires salvage / probe: wedge a household peer's conductor, assert a neighbour custody-blob appears / state: absent (detection gap)
  missing node: on return the owner revokes its own custody rows only when replicas ≥ target; existing revoke → OwnUnnamed → let-go path / probe: holds gauge shows the released bytes after return, replicas never below target / state: partial (path exists, precondition absent)
chain pain-is-answered / between "approach declared" → "breach executed" → "parties informed"
  missing node: care proportional to threshold_pct — watchers raise probe cadence and pre-compute closest-N for the approaching peer's holds, bounded to that peer / probe: a2o — Approach at 85% yields pre-authored (inactive) custody-blob intents on ≥ salvage_target_replicas neighbours within one tick, zero bytes moved / state: absent
  missing node: relief proposal minted and signed at Approach by every party incl. the peer in pain / probe: the proposal's votes include the approaching peer's; Breach cites it; no new consent sought after Breach / state: absent
  missing node: Breach-only slow path — no Approach-time plan → Refer with the evidence set, never a silent drop (C14) / probe: a2o — sudden kill yields a Refer at the named layer / state: absent
  missing node: operator seat commitment (operates-household) bounding Permit vs Refer, Permit bounded by pool slack / state: absent
  missing node: gate-decision reaches the subject's counsel on the CounterEvidence floor / state: absent (floor unassigned outside pricer.rs)
```

## §6 — Slices

1. **Household red (this pass).** The feature, the habit atom, this spec. No Rust.
2. **Sense and refuse (storage, coordinator-only).** Conductor-dataset free-space
   probe; fail-closed Unmeasured; declared `runtime` band limit in runtime-config;
   Approach/Breach emission with the doorway's two-round hold promoted into
   `algedonic.rs`; capacity decline through the policy evaluator, counted;
   `unverifiable` when the conductor is unreachable. Gate: `just gate elohim-storage`.
3. **Announce (DHT carrier).** `CommitmentByState` tags from the mishpat
   coordinator; storage projection and persisted open-signal key; death-witness
   crossing. Gate: the mishpat DNA plus sweettest.
4. **Relief detection.** Conductor reachability in inventory gossip; the salvage
   predicate; the return-time revoke precondition. Household chaos drill extends
   `blob-durability`.
5. **Carry-forward.** Grant schema fields; proposal kind; cross-root staging
   declaration by a steward member with coordinator cross-cell verification; Refer
   route, escalation and `pending` expiry; appeal admission guard; never-returns
   arm. Lands under `custodial-authority-answerable`'s feature.
6. **Operator seat.** `operates-household` commitment; gate-decision routing to
   counsel; settlement by REA events. The automated seat replaces this session.

## §7 — Non-goals and open frontier

- No DNA-hash move in any slice. FeedbackSignal algedonic kinds stay a labelled
  successor carrier for the next planned lamad integrity bump.
- No new hold rung, no new verdict value, no new `ReferReason` member.
- Hosted-agent re-homing and DHT-arc relief are named frontiers, not designed.
- The grant's issuer authorization (accept-all today) and membership author binding
  are weaknesses this design names and depends on being cured in slice 5; until
  then the staging-only rule is the load-bearing bound.
- The cure for adam and eve this week remains the operator's: refquota on shem,
  restart, one conductor-roll push. This design is why that should be the last time
  it is done from outside the system.
