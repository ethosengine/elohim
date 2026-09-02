---
title: "Trust-priced sync edge — a peer edge's verified class keys its catch-up budget, so trusted edges quiesce first"
id: trust-priced-sync-edge
tier: spec
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: >
  Station 1 lands AND the household mesh reads at least two peer classes on the sync edge
  (a2o @concern:trust-priced-sync scenario 1 green: household edges priced `trusted`, no edge
  priced `public` on a pure-household mesh) AND one fleet confirm reads two classes on alpha
  (ratchet rung P7) — OR superseded by a fresh reader contesting §3's floors on evidence.
created: 2026-09-01
maintainers: Matthew Dowell + Claude Fable 5.1
domain: D5
refines: trust-as-efficiency-signal
habits: [dataplane-convergence]
topic: [trust-gradient, sync, quiesce, catch-up, peer-class, fetch-window, backoff, provider-order, admission, verification-memo, reach-ceiling, handshake, dataplane, local-first]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md (the CANONICAL principle this spec puts on the sync plane — every mechanism below is one of its §3 clauses made a predicate)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-iroh-libp2p-complementarity-design.md (D5 owning seed — the two planes the edge budget must apply to identically)
  - genesis/docs/superpowers/specs/2026-08-28-ratchet-to-delivery-dataplane-sdk-lanes-design.md (move M3 "trust-priced sync, design-gated" — this spec IS that design gate; rungs P7, P8, F7, S8)
  - genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md (L5 built the `trust/` seam this spec composes — stage, pricer, memo; the sync edge is the second consumer, never a second pricer)
  - genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md (the per-(receiver, publisher, stream) vocabulary the budget rides beside — epoch, position, caught-up)
  - genesis/docs/superpowers/specs/2026-08-22-verification-as-memoized-derivation-guidestar.md (Law I–III — how this spec's measure is keyed and why absence stays absence)
cites:
  - "trust-as-efficiency-signal | The CANONICAL principle this spec refines onto the sync plane — every knob in §3 is one of its §3 clauses (verification amortizes, distribution fast-paths, local peer selection) made a predicate, and its §5 bidirectionality is a floor here | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md"
  - "trust-compute-gradient-brainstorm | Design source — §3.2 seven-layer gradient supplies the validation and cold-fetch rows this spec implements, with their mandatory low-standing fallback floor | sha256:89c493c73ff6b06b | path: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md"
  - "ratchet-to-delivery-dataplane-sdk-lanes | Execution scaffold that names move M3 (trust-priced sync, design-gated) and rungs P7/P8/F7 — this spec is M3 design gate output; the ratchet is an execution principle, not a design authority | sha256:162f2cde07f0de8e | path: genesis/docs/superpowers/specs/2026-08-28-ratchet-to-delivery-dataplane-sdk-lanes-design.md"
  - "head-plane-trust-gradient-program-plan | L5 built the trust/ seam (stage, pricer, memo) this spec composes as its second consumer — the sync edge feeds PricingInput, it never grows a second pricer | sha256:aee96a34080d4efa | path: genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md"
  - "sync-state-contract | The per-(receiver, publisher, stream) vocabulary the edge budget rides beside — epoch, position, caught-up — so class never masquerades as sync position | sha256:cea95b17140d2fd6 | path: genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md"
  - "verification-as-memoized-derivation-guidestar | Law I–III govern §6 — the measure is declared outside the run, keyed by env with the SUT, and quiesce-sooner is a discovery never a grade | sha256:b4a0a2e087e67c12 | path: genesis/docs/superpowers/specs/2026-08-22-verification-as-memoized-derivation-guidestar.md"
  - "epr-acquisition-pull-queue-design | R-F is the standing rule this spec honours — standing modulates ordering, never retrievability — with the relationship class standing in until T19 | sha256:24aad9240361c0a4 | path: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md"
  - "mutual-storage-replication-dwelling-hub-design | Owns the replicates-dwelling commitment that lifts the doorway/co-steward edge to trusted, and the mutuality audit that keeps §3.3 bidirectionality economic | sha256:1acbeeec8b7a3956 | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md"
  - genesis/plans/2026-03-23-ambient-trust-verification-design.md
  - "substrate-trust-contract-runbook | The invariants no budget may cross — canonical channels alone move declared heads, heal fills never moves — and the probes that would catch a leak | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - genesis/data/timeline/backlog/sync-edge-susan-timeouts-per-edge-observability.md
  - genesis/data/timeline/backlog/edge-quiesce-gate-timeout-aborts.md
  - genesis/a2o/features/dataplane/trust-priced-sync.feature
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
---

# Trust-priced sync edge

**One sentence:** the class a receiver derives for a peer edge from its own DHT view
(household membership, a declared relationship, an active replication commitment) keys
that edge's catch-up budget — fetch window, patience, provider order, admission
priority, and re-verification depth — so the edges a household actually depends on
converge first, strangers are never starved, and quiesce is reached sooner for the
content that matters.

## 1. Why — resiliency should feel effortless once you have a community

The operator's frame (2026-09-01): a developer on the protocol writes a local-first app
with the SDK and it just works; when a tornado takes the house laptop and the hub, the
household still reaches everything over the web through a doorway it registered with.
"k8s-like powers over the p2p dataplane" is what that feels like from the inside: the
dataplane is simple and performant enough that convergence is a property, not a project.

That promise is carried by a small number of *edges*: the household's peers to each
other, and the household to the co-steward and doorway it chose. The canonical principle
(`trust-as-efficiency-signal` §1, §3) says those edges must measurably cost **less** to
converge than a stranger's — verification amortizes, peer selection prioritises, catch-up
lands first — and that a design where they do not "has a leak."

Today the sync plane has that leak, and the evidence is in the register. The habit
`dataplane-convergence` is red on the fleet leg; the 2026-08-28 delta records the first
fleet read of the sync edge by trust class: `elohim_sync_request_outcomes_total`
`{peer_class="public"}` ok 1,959 / timeout 38, `{unverified}` ok 558 / timeout 13 — **no
edge above `public`**. The susan atom measured one shem peer as the fleet's expensive
edge (274 outbound sync failures / 6 h, 93 hers) and found that "nothing in the dataplane
prices that edge": the fetch window is global, backoff reacts to failure only, no timeout
varies per peer. And the quiesce gate atom (2026-09-01) records three healthy edge
deploys dying ABORTED inside the catch-up window. Trust is consulted nowhere on the path
that decides how fast a peer catches up.

So this spec is the design gate for ratchet move **M3 — trust-priced sync**. It does not
make sync fast in general; it makes the *right edges* fast, honestly labels the rest, and
names the measure that says whether it worked.

## 2. Ground truth — what is wired, what is a costume (grounded 2026-09-01)

**The handshake is a stub end to end.** `TrustHandshake` is a real wire message
(`elohim-storage/src/p2p/trust_protocol.rs:32`, `/elohim/trust/1.0.0`), sent on every
`ConnectionEstablished` (`p2p/mod.rs:5416`), but the sender fills every credential
vector empty (`p2p/mod.rs:5418-5423`) and presents its libp2p peer id as the agent key.
Both receiver arms hard-code `agent_verified: true, reach_ceiling: "public"`
(`p2p/mod.rs:6675-6684`, `:6706-6715`); the iroh arm ignores its argument entirely
(`trust_service.rs:47-53`); the iroh auth backend does the same
(`p2p_iroh/auth_backends.rs:190-191`). The real verifier `verify_trust_context` and
`calculate_reach_ceiling` (`trust_verification.rs:93`, `:141-170`) have zero production
callers, and the four `verify_*_cids` helpers return `Ok(vec![])` (`:176-230`).
Consequence, and it is a **C4 violation** not a mere gap: every edge on every mesh is
asserted *verified public*. "Public" today is a lie the metric repeats.

**The label is wired; nothing reads it.** `PeerTrustCache::peer_class_for`
(`p2p/trust_cache.rs:99`) clamps the cached ceiling to `CORE_REACH_LEVELS`
(`generated_enums.rs:315-324`) ∪ `{other, unverified}` and feeds only the
`elohim_sync_request_outcomes_total{peer_class,result}` label (`metrics.rs:1687-1691`;
`p2p/mod.rs:6480`, `:6501`). No scheduling, verification, or fetch decision consults it.

**Every knob is global or failure-keyed.** `DEFAULT_FETCH_WINDOW = 32`
(`p2p/sync_round.rs:190`, `config.rs:612`); the per-peer window map exists
(`p2p/mod.rs:795`, opened at `:8017`) but is sized from the global default.
`ReplicationScheduler` backs off per peer on transport failure only (60 s → 15 min,
`replication_schedule.rs:35-104`). One `request_timeout` (30 s, `p2p/mod.rs:681`) is
applied to every protocol and peer (`behaviour.rs:374-444`). The iroh sync driver walks
the peer book in book order (`p2p_iroh/sync_driver.rs:52-65`). Provider choice is
mechanical round-robin (`rotated_peer_index`, `p2p/mod.rs:299-315`;
`reconcile_peers.rs:49-65` returns swarm order); `select_path`
(`p2p/transport_paths.rs:216`) chooses a *transport*, not a peer. The pull-queue spec R-F
(`2026-06-07-epr-acquisition-pull-queue-design.md:161-164`) said this on purpose:
"Standing modulates ordering, never retrievability … scoring MUST degrade gracefully to
trust-agnostic availability scoring today."

**Verification: the cheap checks are unskippable and stay that way; the expensive one is
the conductor round-trip.** Automerge changes are chained by hash inside the format
(`sync/mod.rs:86-106`); blob bytes are sha256-recomputed on arrival
(`p2p/mod.rs:2553-2562`). Neither is a policy and neither is priced. What costs quiesce
is one uncancellable conductor round-trip per head per sweep (`head_adoption.rs:577`;
head-plane plan §1). The seam that prices it **already exists and is live**:
`TrustGradient` + `VerificationPricer` (`trust/pricer.rs`, `trust/adoption_pricing.rs`,
called from `projection_reconcile.rs:5071`, chosen at `main.rs:5279-5286`) — but its only
discount today fires at the declared `Simulacra` stage; at Bootstrap and above
`DeclaredStakesPricer` equals `InertPricer` exactly (`pricer.rs:110-112`). It prices by
*stage*, never yet by *edge*. A narrower signal is also live: courier delivery health
reorders an already-known adoption slice (`order_eligible_by_trust_gradient`,
`projection_reconcile.rs:4762-4828`) — delivery history, not relationship trust.

**The DHT already holds the facts an edge class needs.** `Membership` / `Collective`
(`imagodei_integrity/src/qahal.rs:24-49`), `HumanRelationship`
(`imagodei_integrity/src/lib.rs:377-411` — with a same-shaped duplicate on the elohim DNA,
`content_store_integrity/src/lib.rs:922-963`, a drift captured below), the REA
`Commitment` with `replicates-dwelling` / `replicates-commons` / `delegates-compute`
(`content_store_integrity/src/lib.rs:1383-1414`; projected to `rea_commitments`, queryable
by agent id at `db/rea_commitments.rs:384-395`), and `AgentPeerBinding`
(`imagodei_integrity/src/agent_peer_binding.rs:89-97`, minted by a running node at
`main.rs:3008`, projected at `reconcile/controller.rs:549`). Bindings are self-asserted
(`STAGE1_SIGNATURE_SENTINEL`) — enough to decide where to dial, never whom to credit.

**The quiesce predicate** is `converged_blocked_by{divergent_actionable | unmeasured} == 0`
sustained, with `divergent_actionable = divergent_anchor − divergent_refused`
(`metrics.rs:4311-4351`; `projection_reconcile.rs:1458-1522`), on sweep budgets of
120 s / 45 s / 120 s / 25 s (`projection_reconcile.rs:181,461,483,487`).

## 3. The design

### 3.1 One derived class per edge — reuse the ceiling, mint nothing

An **edge** is the pair (this receiver, that publisher peer), keyed by the publisher's
transport id exactly as the sync-state contract keys its streams. Its **class** is the
`reach_ceiling` the receiver derives for the peer from the receiver's *own* DHT view at
handshake — the value `calculate_reach_ceiling` already computes, with one addition:

| class (label value) | derived from | budget row |
|---|---|---|
| `unverified` | no handshake, expired context, or a presented agent key that resolves to no verifiable binding | floor |
| `public` | verified agent key; no relationship, membership, or commitment with us | baseline (= today's global defaults) |
| `community` | a consented `Membership` in a collective we are also a member of | +1 |
| `trusted` | a `HumanRelationship` at intimacy ≥ trusted with consent, **or a household `Membership`**, **or an active `replicates-dwelling` / `replicates-commons` `Commitment` between the two agents** (the addition — the doorway/co-steward edge) | +2 |
| `intimate` | mutual intimate relationship | +2 (same row as trusted; intimacy is a reach fact, not a bandwidth fact) |

The vocabulary is `CORE_REACH_LEVELS ∪ {other, unverified}` — the closed set
`peer_class_for` already emits — so the metric label contract does not move. This spec
does **not** pick a canonical reach vocabulary (that drift is declared and owned
elsewhere); it reads the DNA-notarized enum the ceiling function already returns.

The class is **Ephemeral (C)**: derived, cached with the existing TTL in
`PeerTrustCache`, reconstructable at the next handshake, never stored as a number, never
written to the DHT. The receiver verifies every CID the sender presents against its own
conductor (**C5 — evidence, not authority**); a peer cannot raise its own class
(**C1**); a presented credential that does not verify leaves the edge `unverified`, not
`public` (**C4 — honest absence**).

### 3.2 One predicate — `EdgeBudget`

```
EdgeBudget::for_edge(class: EdgeClass, health: EdgeHealth) -> EdgeBudget {
    fetch_window:       usize,      // SyncChanges window at open time
    request_patience:   Duration,   // caller-side deadline on the windowed request
    backoff_cap:        Duration,   // ReplicationScheduler cap for this peer
    admission_weight:   u8,         // slots per round in the catch-up admission ring
    reverify:           VerificationDepth,  // input to the EXISTING pricer, never a second pricer
    reason:             EdgeBudgetReason,   // typed; every knob decision counts through it (C8)
}
```

Pure — no clock, no diesel, no tokio — so it lives beside `trust/pricer.rs` and is
registered in `seam-registry.yaml` as a `pure-decision-predicate` with contract tests.
`EdgeHealth` is the *other* axis and already exists in pieces: the scheduler's failure
streak and `PathObservation`'s RTT/success state. The budget row comes from the class;
health only ever **scales a row down** (a trusted peer that is timing out gets a smaller
window and a shorter patience, and the scaling is counted with its own reason). That is
the susan cure: an expensive edge shrinks its own budget instead of re-arming a fixed-cost
timeout storm against everyone.

Default table (the numbers are the *initial* declaration, config-overridable, pinned by a
drift test the way `DEFAULT_FETCH_WINDOW` is pinned today):

| class | window | patience | backoff cap | admission weight | reverify |
|---|---|---|---|---|---|
| `unverified` | 32 | 30 s | 15 min | 1 | FullChain |
| `public` | 32 | 30 s | 15 min | 1 | FullChain |
| `community` | 64 | 45 s | 10 min | 2 | FullChain |
| `trusted` / `intimate` | 128 | 60 s | 5 min | 4 | DeltaVerify when memo matches (§3.4) |

`unverified` and `public` share a row on purpose: a fleet peer one build behind never
presents credentials, and a rolling deploy must not starve it. The class widens; it never
gates.

### 3.3 The floors — what trust may never buy

- **Liveness (C3).** Every class has `window ≥ 1`, `patience > 0`, and
  `admission_weight ≥ 1`: the admission ring is weighted round-robin, so a stranger edge
  gets a slot every round. Brainstorm §3.2, cold-fetch row: "if no high-standing provider
  exists, low-standing fallback is mandatory." Retrievability is never class-gated.
- **Authority (C2).** No budget moves a head. Declared heads move only through the own
  conductor's `declare_canonical_head` and the election, exactly as today
  (`head_adoption.rs:1617`); the federation-deploy scenario's rule stands verbatim — *an
  election obeyed, never a trust-the-peer copy*.
- **Integrity floors.** Blob sha256 and Automerge hash-chain checks are not in the table
  because they are not priceable. `floor != None ⇒ FullChain` at every stage remains the
  pricer's invariant; a `Constitutional`, `LocalRelationship`, or `CounterEvidence`
  decision point is never cheapened by an edge class.
- **Bidirectional (canon §5).** A peer that receives a trusted budget owes one: the
  budget applies to the edge, and both ends derive it from the same DHT facts, so a
  household peer serving its co-steward gets the same row it receives.
- **No stored score.** `EdgeClass` is an enum over reach levels; `EdgeBudget` carries no
  ranking number a peer could accumulate. The `trust/` seam's `no-stored-score` rail
  covers the new module.

### 3.4 Where the four canon §3 clauses land

1. **Distribution fast-paths → window + patience.** The per-peer window map
   (`p2p/mod.rs:795`) is sized by `EdgeBudget.fetch_window` at open; the caller-side
   deadline is `request_patience`, enforced at the inflight map (`sync_fetch_inflight`),
   not by adding a per-peer libp2p `request_response` timeout (that config is
   per-behaviour). `ReplicationScheduler` reads `backoff_cap` per peer.
2. **Peer selection is local → provider order.** `rotated_peer_index` rotates over a
   candidate list *ordered by class then health*, and the iroh driver walks the book in
   the same order — with the weighted ring so order never becomes exclusion. R-F's
   "standing modulates ordering, never retrievability" is honoured with the relationship
   class standing in for `Standing` until T19 lands a standing writer; when it does, it
   enters as a second ordering input, never displacing the `LocalRelationship` floor.
3. **Catch-up lands first → admission priority.** Both sweep legs (content, REA) admit
   work items from the ring by weight. Divergent heads sourced from a trusted edge are
   worked first inside the same budgets; nothing new is scheduled, the order of existing
   work changes. This is the lever that moves `divergent_actionable` toward zero for the
   content a household depends on before the stranger backlog.
4. **Verification amortizes → reverify depth.** `PricingInput` gains `edge: EdgeClass`.
   On a trusted edge, a re-delivered head whose `VerificationMemo` (already built:
   `trust/memo.rs`) records *our own* full-chain verification in the current epoch prices
   `DeltaVerify` — the conductor round-trip is skipped because **we** verified it and
   nothing moved, and the trusted edge is what stops the re-delivery re-entering the queue
   at full price. On a `public` edge the same re-delivery prices `FullChain`. The head-plane
   corpus digest (L2) composes here: a trusted peer's digest agreement short-circuits the
   per-head diff for the rows it covers; a stranger's digest is recorded, never acted on.
   The write side is untouched.

### 3.5 What this deliberately is not

Not a new DHT entry type, link, or coordinator write — every fact is read. Not a new wire
message in stations 1–4: `TrustHandshake` already carries the credential vectors this
spec fills, and both planes already run the handshake. Not reach enforcement on the CRDT
plane (ratchet F7 — a positional-msgpack wire-shape decision, sequenced separately). Not
standing (`services/standing.rs` is `Unknown` until T19). Not economic attribution —
self-asserted bindings decide where to dial and what budget an edge gets; they never
credit anyone (`elohim-storage/CLAUDE.md` identity coherence rule). Not a doorway concern:
the doorway's *storage peer* is the trusted edge; the doorway projects what that peer
converged.

## 4. P2P Design Gate

### Entity: `EdgeClass` (per (receiver, publisher) edge)
- **Classification**: Ephemeral (C)
- **Justification**: derived from Notarized facts the receiver reads itself (Membership A,
  HumanRelationship A, Commitment A, AgentPeerBinding A); delete it and the next
  handshake rebuilds it.
- **Network Stakes**: all four stages; budget widening is stage-priceable, every floor in
  §3.3 is floor-protected and stage-invariant.
- **Content Address Strategy**: n/a — cache keyed by transport peer id, resolved to
  `agent_cid` through the binding projection, never string-joined across namespaces.
- **Source of Truth**: the DHT facts above; the cache is a projection with TTL.
- **Integrity Zome + DNA-hash class**: none touched — DNA-hash-NEUTRAL. The one zome
  change is coordinator-only: `get_relationship_by_action` beside
  `get_membership_by_action` (`qahal_coordinator.rs:596`) — hot-swap.
- **Projections**: `PeerTrustCache` (in-memory, existing). No SQLite row, no Automerge doc.
- **HTTP Route**: none new; `/p2p/status` gains the per-peer class in its existing
  `irohPeers` / peer rollup (a read projection, declared in `build_manifest()` as today).
- **Anti-Pattern Check**: cross-namespace identity — the class is looked up by transport
  id and *derived* from `agent_cid`-keyed facts through `peer_identity_bindings`; no raw
  compare. Stored-score — none. Amber/green — untouched.

### Entity: `EdgeBudget` (decision predicate) + `EdgeBudgetReason` (reason enum)
- **Classification**: Ephemeral (C) — a pure function's output; nothing persists.
- **Concern-canon answers (Step 4)**: C0 answered (plane: sync/replication, receiver
  side); C1 answered (class derived by the receiver, never presented); C2 answered (no
  budget moves a head — pinned by a test that the budget module has no write path); C3
  answered (weighted ring, `window ≥ 1`); C4 answered (`unverified` is a class, and
  becomes the truthful default); C5 answered (every presented CID re-derived against the
  own conductor); C6a answered (budgets bound work per round; the ring never exceeds the
  existing sweep budgets); C6b answered (re-running a budget decision is idempotent);
  C7 n-a (no advertised capability); C8 answered
  (`elohim_sync_edge_budget_total{peer_class,knob,reason}` — bounded labels); C9 partial
  (bindings self-asserted; a re-keyed agent re-handshakes and re-derives — the
  cross-signed control proof is owned by `identity-cross-signed`); C10 answered (wire
  unchanged; an old sender presents empty vectors and lands `unverified` = baseline
  row); C11 answered (health scales the row down under an observed timeout streak, counted
  by reason); C12 answered (consent is read from the DHT facts themselves — an
  unconsented membership does not lift a class); C13 answered (the class ladder is
  labeled as a bandwidth gradient, never an authority tier); C14 n-a.
- **Registration**: one `seam-registry.yaml` row, `pure-decision-predicate`, contract
  tests: floor table, monotone-widening, health-only-narrows, old-sender-lands-baseline.

### Design constraints discovered
- `HumanRelationship` exists on two DNAs with the same shape (imagodei and elohim); the
  class reads the imagodei one (the one `calculate_reach_ceiling` was written against)
  and the duplication is captured as a backlog row, not resolved here.
- A per-peer libp2p request timeout is not available per request in `request_response`
  0.54; patience is a caller-side deadline on the window, which is why it lives at the
  inflight map.
- The iroh `TrustService` and the libp2p handshake arm must derive the *same* class for
  the same peer, or a dual-plane peer would price two ways; one derivation function,
  two callers, one contract test.

## 5. Stations — the first slice is named, not coded

**Station 1 (first implementation slice) — the honest handshake.** The sender fills
`TrustHandshake` with its steward agent key and the membership, relationship, and
commitment CIDs it holds; the receiver calls `verify_trust_context` through
`hc_registry.imagodei_client()` (`hc_client_registry.rs:151`) using
`get_membership_by_action` and the new coordinator-only `get_relationship_by_action`, and
`calculate_reach_ceiling` gains the commitment input; both the libp2p arms and
`TrustService::handle` cache the derived context; an edge that presents nothing verifiable
is `unverified`. **Write-set:** `p2p/mod.rs` handshake arms, `trust_service.rs`,
`trust_verification.rs`, `p2p_iroh/auth_backends.rs`, the qahal coordinator zome
(hot-swap). **Not in this slice:** no knob reads the class yet. **Falsifier:** a2o
`trust-priced-sync` scenario 1 — on the pure-household mesh `peer_class="trusted"` ≥ 1
and `peer_class="public"` == 0. **Mixed-version guard:** a fleet peer on the previous build
presents empty vectors and is priced `unverified` — the same budget it has today.

**Station 2 — `EdgeBudget` on window, patience, backoff.** The predicate, its
registry row and tests; the window opener, the inflight deadline, and the scheduler read
it. Metric `elohim_sync_edge_budget_total{peer_class,knob,reason}`. Falsifier: scenario 3.

**Station 3 — provider order and admission ring.** Class-ordered candidates under
`rotated_peer_index` and the iroh driver; weighted admission in both sweep legs.
Falsifier: scenario 2 (a stranger late joiner still syncs and prices `public`).

**Station 4 — reverify depth.** `PricingInput.edge`; `DeltaVerify` on memo hit over a
trusted edge; composes L2's corpus digest when it lands. Falsifier: on a warm restart of a
household peer, `elohim_projection_heal_outcomes_total` does not grow for heads the memo
already covers, and the recovery timeline's warm number does not regress.

**Station 5 — fleet.** P7 on alpha (≥ 2 classes); susan's edge shrinks its own budget
(`{peer_class,result="timeout"}` falls on her edges while ok holds elsewhere).

## 6. Measurement — what "quiesce sooner" means here (Law I–III)

The measure is declared outside the run and keyed by environment with the system under
test in the key. Household mesh first, fleet confirms.

- **Household (act I):** `just mesh recovery warm jessica` before and after station 3,
  same corpus, `io_baseline` within 2× (the 2026-08-28 rule: a contended measure is a
  discarded measure). Warm recovery is cadence-bound on a LAN (≈58–62 s measured) and is
  predicted to stay inside noise — that is the honest prediction, recorded before the run.
  The claim the household lane *can* make: classes are real (scenario 1), strangers are
  served (scenario 2), decisions are counted (scenario 3).
- **Fleet (act II):** the quiesce gate's `time_to_verdict` and `best_window` on the next
  edge deploy, and the per-class timeout rate on the susan edges. No number is promised;
  the prediction is written to the run's report before the deploy so the comparison is a
  discovery, never a grade.
- **Never:** a ratio over a denominator that includes NOT MEASURED, or a green from a
  scoped run that exercised one class.

## 7. Gaps (each serves habit `dataplane-convergence`)

1. Honest handshake: sender presents steward agent key + credential CIDs — habit
   dataplane-convergence, station 1, OPEN.
2. Receiver verifies presented CIDs against its own conductor; `unverified` is the truthful
   default; `calculate_reach_ceiling` reads commitments — habit dataplane-convergence,
   station 1, OPEN.
3. Coordinator-only `get_relationship_by_action` on the imagodei/qahal coordinator
   (hot-swap, DNA-hash-neutral) — habit dataplane-convergence, station 1, OPEN.
4. One class derivation shared by the libp2p arm and iroh `TrustService`, contract-tested
   equal — habit dataplane-convergence, station 1, OPEN.
5. `EdgeBudget` predicate + `EdgeBudgetReason` + seam-registry row + floor/monotone tests —
   habit dataplane-convergence, station 2, OPEN.
6. Window opener, inflight patience, scheduler cap read the budget; metric
   `elohim_sync_edge_budget_total` — habit dataplane-convergence, station 2, OPEN.
7. Health scales a row down (timeout streak → smaller window, shorter patience), counted
   by reason — habit dataplane-convergence, station 2, OPEN.
8. Class-ordered provider candidates under `rotated_peer_index` and the iroh driver, with
   the weighted ring — habit dataplane-convergence, station 3, OPEN.
9. Weighted admission in the content and REA sweep legs — habit dataplane-convergence,
   station 3, OPEN.
10. `PricingInput.edge` + `DeltaVerify` on memo hit over a trusted edge — habit
    dataplane-convergence, station 4, OPEN.
11. `/p2p/status` projects the per-peer class — habit dataplane-convergence, station 2,
    OPEN.
12. Fleet confirm: P7 ≥ 2 classes on alpha; susan-edge timeout rate falls — habit
    dataplane-convergence, station 5, OPEN.
13. a2o `@concern:trust-priced-sync` scenarios 2–5 un-`@wip` as their stations land (the catch-up-order scenario is station 3's falsifier) —
    habit dataplane-convergence, OPEN.

## 8. Captured, not absorbed

- `HumanRelationship` duplicated on the imagodei and elohim DNAs — vocabulary/shape drift
  for the reach-ontology work, not this spec's.
- Reach enforcement on the CRDT plane (F7) — wire-shape decision, separate.
- Standing (`T19` writer) — enters §3.4 clause 2 as a second ordering input when it exists.
- The quiesce gate's own timeout/warn-only bounding — the 2026-09-01 backlog atom; CI
  surface, not dataplane.
- The economic side of canon §5 (a trusted edge that fails to reciprocate owes a
  `reciprocity-imbalance` FeedbackSignal) — the dwelling-hub spec's mutuality audit
  already owns it.
