# Self-Healing P2P Dataplane — Campaign Design

**Date:** 2026-04-19
**Status:** Design (review pending)
**Owner:** Matthew Dowell
**Successor to:** `2026-04-19-p2p-dataplane-visibility-design.md` (Phase 1 — visibility)
**Composes with:** `2026-04-19-gate-challenge-and-indemnification-design.md` (evidence hooks at rung e)

---

## 1. What triggered this doc

The p2p-dataplane-visibility sprint closed the gap between "design exists" and "dashboards show real data." That work was deliberately about honest reporting — `PeerStatus` lives on DHT, node-shape boot self-registers, resilience tooltip is household-first, four production surfaces show their data.

This doc is about **closing the loops behind the dashboards**. Today, distribution happens but diversity isn't contract-aware; verification is a named stub at `/api/v1/resilience/{id}/verify` with no scanner behind it; reconstruction doesn't exist; trust in the shard census is peer-claimed, not attested. The dashboards are honest about the state of the dataplane — the dataplane itself is incomplete.

The acceptance horizon remains *grandma-grade P2P* — Terrance offline, household survives, recovery is autonomous and visible, operator nudges are economic signals rather than error logs. Five sequenced plans get us there. Each plan is independently valuable; the user can pause between plans without stranding value.

---

## 2. North-star architecture with policy/mechanism separation

The self-healing dataplane is **five rungs on the layer-cake already in place**. Nothing new at L1 (content addressing), L5 (elohim-node topology), or L4 (PeerStatus). All work lives at **L2 (shard distribution)** and **L3 (identity-driven replication)** with UI pull-through at each rung.

```
┌─────────────────────────────────────────────────────────┐
│  DHT — POLICY (contracts set the bounds)                 │
│                                                          │
│  REA commitments (existing entry types; rea_commitments  │
│    projection in elohim-storage):                        │
│    household A → household B: N GB storage, SLA, scope   │
│    → libp2p reads these to know WHERE it may distribute  │
│                                                          │
│  ShardHoldingAttestation (rung e; B2 on existing         │
│    imagodei Attestation entry type):                     │
│    "I attest I am holding shard X per commitment Y"      │
│    → proof-of-fulfillment, not a new contract primitive  │
└─────────────────────────────────────────────────────────┘
                        ▲
                  reads │ respects
                        │
┌─────────────────────────────────────────────────────────┐
│  libp2p + SQLite — MECHANISM                             │
│                                                          │
│  Rung a: shard_locations honestly reflects reality        │
│  Rung b: diverse-peer selection respecting commitments    │
│  Rung c: periodic verification scanner (tokio)            │
│  Rung d: reconstruct gaps on diverse replacement peer     │
│    All traffic on /elohim/shard/1.0.0                    │
│    (existing Push/Get/Have + new Reconstruct variant)    │
└─────────────────────────────────────────────────────────┘
```

### Key design claims

- **Contracts are the policy; libp2p is the mechanism.** Rungs a-d create **zero new DHT entities**. Rung e **reuses** the imagodei `Attestation` entry type — no new entry type, no DNA capacity cost.
- **Diversity selection cannot exceed contract scope.** If household X has no commitment for content Y, Y doesn't land on X. The selector reads commitments first, lifecycle state second, household/archetype diversity third.
- **Dataplane anomalies are structured economic signals**, not error logs. Placement gaps, verification breaches, reconstruction events, and over-extended commitments surface in shefa for planning — where to recruit peers, where subsidies flow, who needs recovery.
- **Holochain stays the notary** (trust claims, accountability evidence). libp2p stays the dataplane (distribution, verification, reconstruction). The DHT's narrow scope is protected.

### Where each loop runs

- **Distribution + verification scanners**: tokio tasks inside elohim-storage.
- **Reconstruction decisions**: elohim-storage (mechanical: which diverse peer wins arbitration). elohim-agent-mediated policy (gated/budget-aware reconstruction) is deferred to downstream guardian-loop work.
- **Peer-to-peer shard traffic**: `/elohim/shard/1.0.0` libp2p request-response (already has `Push`, `Get`, `Have`; gains `Reconstruct` enum variant in Plan 3).
- **Diversity inputs**: `peer_statuses` + node archetype (L4/L5, already shipped) + `humans.household_id` projection (Plan 1 blocker fix) + active `rea_commitments` (existing).

---

## 3. Campaign decomposition

Five plans. Each ends on a working checkpoint where the dashboards tell a progressively richer, still-honest story.

| # | Plan | Exit criterion (short) | Primary touches |
|---|------|-----------------------|-----------------|
| **1** | **Observable + auto-distribute** (rungs a+b) | Ingest → shards on N diverse households within contract bounds; shared `<elohim-resilience-snapshot>` live across shefa + doorway + content-viewer | elohim-storage peer selection + placement gaps; `elohim-library` resilience component; UI pull-through |
| **2** | **Periodic verification** (rung c) | Tokio scanner detects missing/bit-rot shards; UI surfaces verification state; cross-peer Have-checks catch lying peers | elohim-storage verifier; cadence policy; UI state |
| **3** | **Auto-reconstruction** (rung d) | Detected gap → reconstruction completes on diverse replacement peer; stampede defended; unrecoverable surfaces explicitly | elohim-storage reconstruction coordinator; libp2p shard protocol gains `Reconstruct` variant |
| **4** | **Attested holdings** (rung e) | B2 attestations on imagodei Attestation entry type for held shards; trust-verified census; evidence hooks for gate-challenge | imagodei coordinator fn addition; elohim-storage projection + publisher; UI attested vs claimed |
| **5** | **Chaos demo on shem** | 8 a2o chaos scenarios green on shem with real peers; grandma demo runs end-to-end; four dashboards stay honest during chaos | chaos admin endpoints (gated); a2o feature files + step defs; shem manifests; drain guard (Module J); H2 JWT scope |

### Review checkpoints

Each plan ships → user reviews running behavior → next plan starts. Plans 1-4 are Rust/TS backend + UI pull-through. Plan 5 is the heavy a2o + shem orchestration.

### Deferred out of the campaign

Model-diversity axis (L7 guardian loop); private-content encryption for pull-mode replication; appliance packaging (single-container grandma install); contract renegotiation workflows; region-aware placement (region is display-only in this campaign).

---

## 4. Cross-cutting UI: `<elohim-resilience-snapshot>`

Built in Plan 1, extended at each rung. A shared, contextual Angular component in `elohim-library`, consumed by both `elohim-app` (shefa pillar, content-viewer, resource pages) and `doorway-app` (admin views). Three display densities:

| Density | Placement | Content |
|---------|-----------|---------|
| **Icon + tooltip** | Content list rows, content-viewer header, admin grids | Icon (green/yellow/red/grey); tooltip shows "4 households / 3 regions / HA: yes / verified 42m ago" |
| **Context-menu panel** | Right-click or menu-trigger on content | Adds shard breakdown, last recovery event, placement gap summary |
| **Full card** | Content detail / resource page, Signals tab | Household list with archetype badges + timeline link + attestation status |

All densities read the enriched `/api/v1/resilience/{id}/household` endpoint.

### Geographic distribution

Operator-declared household metadata (a `region` field on the collective entity — e.g., `na-west`, `eu-central`). Not GeoIP. Surfaces in `regionalDistribution: { local, regional, global, unknown }` classified by the resilience service (not the client). **Display-only in this campaign**; region-aware placement is a downstream policy extension.

### Doorway-app parity

Doorway admin surfaces (`doorway-alpha/threshold/dashboard`, `/admin/routes`, future content-admin views) render the same component by importing from `elohim-library`. Each plan's acceptance criteria gain: *"Render is present and live in both shefa and doorway admin for the affected surfaces."*

### Per-plan evolution

| Plan | Icon+tooltip | Context panel | Full card | Doorway admin |
|---|---|---|---|---|
| 1 | ✓ — real data from ingest | ✓ | ✓ (Signals card) | ✓ |
| 2 | verification state folds into color | adds "verified Xm ago" | adds verification history | ✓ |
| 3 | no visual change | adds "last recovery" | adds recovery timeline | ✓ |
| 4 | attestation badge (B2 trust) | "attested by N peers" | attested census vs claimed census | ✓ |

---

## 5. Plan 1 — Observable + auto-distribute (rungs a+b)

### P2P Design Gate

```
### Entity: humans.household_id (projection column — existing nullable, needs wire-up)
- Classification: Operational (C) — projection of existing humans DHT entry.
- Current state: nullable TEXT column shipped in migration
  2026-04-19-000002_humans_add_household_id with index idx_humans_household.
  Populated today ONLY by D3 seeder pipeline from humans.json fixture;
  CreateHumanInput currently sets household_id: None for non-seeder paths.
  household_resilience.rs contains stub comments ("Until humans.household_id
  projection column lands") that still need replacing with real household-id-
  aware logic.
- Justification: householdId is already a HumanFrontmatter field in the humans
  DHT entry (commit 540a5620). Wire-up finishes a half-landed feature.
- Source of Truth: Holochain DHT (humans entry).
- What Plan 1 must do: (a) add household_id to CreateHumanInput and thread it
  from DHT imports; (b) one-shot backfill pass for existing humans rows from
  their DHT entries at startup; (c) replace household_resilience.rs stubs with
  real household-grouped logic; (d) keep column nullable (legacy humans may
  genuinely have no household).
- Anti-Pattern Check: no new entry type; no new table; nullable is correct here
  (truth: not every human belongs to a household).

### Entity: placement_gaps (local, Category C; shefa signal)
- Classification: Operational — surfaced as structured shefa signal per
  project_placement_signals_are_shefa_inputs.
- Justification: Gaps are derivable from the diff between requested vs achieved
  placement. Persisting the history gives shefa a queryable signal surface
  (where to recruit peers, where subsidies flow).
- Fields: content_id, shard_hash, requested_household_count,
  achieved_household_count, contract_coverage, gap_kind
  (under-committed / contracts-short / peers-unavailable / unrecoverable /
  attested-breach), first_seen_at, last_seen_at.
- Rebuild strategy: recompute from shard_locations + rea_commitments at startup.
- Anti-Pattern Check: No dht_anchor_hash (operational); reconstruction strategy
  documented; no DHT entity.

### Entity: diversity score (runtime only)
- Classification: Operational — computed per-call, never persisted.
- Justification: Derivable from peer_statuses + humans.household_id + archetype +
  active rea_commitments. Persisting would lie about the current state.
- Anti-Pattern Check: No table, no row, no DHT.

No new DHT entry types. No new entry links. One table addition (placement_gaps),
one projection column (humans.household_id).
```

### Components

1. **`humans.household_id` wire-up + backfill + stub replacement** — column already exists (migration 2026-04-19-000002). Needed: (a) extend `CreateHumanInput` with `household_id`, (b) thread from DHT humans entries through all import paths (not just D3 seeder), (c) one-shot backfill at startup for existing rows with null household_id that have a DHT entry carrying householdId, (d) replace the "Until humans.household_id projection column lands" stubs in `household_resilience.rs` with real household-grouped logic. Stays nullable (legacy humans may have no household).

2. **Contract-aware diverse-peer selector** (`elohim-storage/src/services/peer_selection.rs` new) — inputs: `content_id`, `data_shard_count + parity_shard_count`, current placements. Reads `rea_commitments` (filter: `action=provide`, active, content scope matches) ⋈ `peer_statuses` (filter: `lifecycle=accepting`) ⋈ `humans.household_id`. Diversity rule: maximize distinct households, tiebreak by distinct archetypes, tiebreak by distinct nodes. Output: ranked list. Caller picks N.

3. **`distribute_shards` upgrade** (`p2p/mod.rs:551`) — replaces current selection with the new selector. On ingest, for each shard, `ShardRequest::Push` to N diverse peers, record acks in `shard_locations`. Failures that leave placement below contract minimum → `placement_gaps` row (kind=`peers-unavailable` or `contracts-short`) + structured log. Place-what-we-can + flag; no refuse-and-drop.

4. **`/api/v1/placement-gaps`** endpoint (paged, filterable by `gap_kind`) — shefa Signals card reads this.

5. **Resilience view enrichment** — `/api/v1/resilience/{id}` and `/household` gain `commitmentBackedHouseholds`, `diversityScore`, `placementGaps` (requested-vs-achieved), `regionalDistribution`. Schema-first: JSON schema added in `elohim/sdk/schemas/v1/views/` **before** Rust struct.

6. **`<elohim-resilience-snapshot>` component v1** (`elohim-library/projects/elohim-service/src/components/` new) — three densities described in §4. Exports via barrel for both `elohim-app` and `doorway-app`.

7. **Shefa Network Health tab enhancement** — groups peers by household, shows household row with peer count + archetype badges + active commitment count + placement gap badge. Reuses `@elohim/storage-client`. (Full graph viz deferred.)

8. **Content-viewer resilience tooltip** — upgraded to render `<elohim-resilience-snapshot>` in icon+tooltip density, reading the enriched `/household` endpoint.

9. **Shefa `/shefa/dashboard` Signals card** — reads `/api/v1/placement-gaps`, surfaces "3 contents need more household coverage; 1 household is over-extended" style summaries.

### Cadence + dev-mode controls (per project_cadence_archetype_tunable_with_dev_overrides)

Distribution retry budgets and failure-replay intervals get the four-layer control: archetype default table → `peer-policy.toml` override → `ELOHIM_DISTRIBUTE_*` env → `POST /api/v1/admin/chaos/redistribute-now` synchronous trigger (the chaos endpoint lands in Plan 5 but the handler hook is introduced here).

### Acceptance criteria

- Ingest content X on shem → within ≤30 s, `/api/v1/resilience/{id}` shows shards on ≥3 distinct households (contract-permitting).
- `/api/v1/placement-gaps` returns non-empty when distribution is short; empty when full.
- `shefa/dashboard` Network Health tab renders real household grouping with non-zero commitment counts.
- Content-viewer tooltip shows household-first claim with geographic distribution.
- Doorway admin content-list renders `<elohim-resilience-snapshot>` icon+tooltip.
- ≥95% of seeded humans have `household_id` populated after backfill (legacy humans without household designation may remain null).
- `household_resilience.rs` stubs replaced; household-grouped resilience is computed from real projected data.
- Existing tests pass + one new sweettest covering diverse placement.

**Non-goals for Plan 1:** no verification scanning; no reconstruction; no graph viz; no attestations; region-aware placement (display-only).

---

## 6. Plan 2 — Periodic verification (rung c)

### P2P Design Gate

```
### Entity: shard_verifications (local, Category C)
- Classification: Operational — verification is a local act; the authoritative
  fact is the shard's existence/hash on disk. Row is a timestamped observation.
- Fields: shard_hash, verifier_peer_id, status (ok/missing/corrupt),
  verified_at, notes.
- Rebuild strategy: rerun the scanner.
- Anti-Pattern Check: No dht_anchor_hash (operational); no DHT entity.

### Entity: verification_breaches (local, Category C; shefa signal)
- Classification: Operational, surfaced as structured shefa signal.
- Fields: peer_id (the peer failing), shard_hash, content_id, breach_kind
  (missing/corrupt/stale/have-but-no-get), first_seen_at, last_confirmed_at.
- Rebuild strategy: recompute from shard_verifications + shard_locations.
- Anti-Pattern Check: no DHT entity; no CID FK.

No new DHT entry types.
```

### Components

1. **Verification scanner** (`elohim-storage/src/services/shard_verifier.rs` new) — tokio task at boot. Runs every T (archetype-default 6h for edge, 1h for archival; four-layer controls apply). Iterates `shard_locations WHERE peer_id = self`; `blake3(disk_bytes) == shard_hash` check; writes `shard_verifications`. Mismatch → `verification_breaches` row + structured log. Priority scans (recently-touched content, recent commitments) every 15 min (also tunable).

2. **Cross-peer Have-check** — bounded-sample subset per scan tick (default 10% random). For each sampled shard, issue `ShardRequest::Have { hash }` to peers `shard_locations` says hold it; cross-check with follow-up `Get` for newly-added or suspect peers. A `Have=true / Get=fail` pattern → `verification_breaches` row with `breach_kind=have-but-no-get` against the lying peer.

3. **`/api/v1/resilience/{id}/verify` filled in** — existing stub now triggers synchronous verify for the content's shards, returns `VerificationReportView[]`. Bounded. For admin/chaos use.

4. **Resilience view gains** `lastVerifiedAt`, `verificationStatus` per shard.

5. **`<elohim-resilience-snapshot>` v2** — verification state folds into icon color and adds "verified Xm ago" + verification history to fuller densities.

6. **Shefa Signals Breach card** — reads `verification_breaches`, shows peer breach counts and trends.

### Cadence + dev-mode controls

Four-layer control applies: archetype defaults (Level-0 edge = 6h full scan + 1h priority; Level-5 archival = 1h full + 15m priority; intermediate archetypes interpolated). Operator overrides via `peer-policy.toml`. Dev overrides via `ELOHIM_VERIFY_INTERVAL=30s` and `ELOHIM_VERIFY_PRIORITY_INTERVAL=5s`. Synchronous trigger via `POST /api/v1/admin/chaos/trigger-verify` (Plan 5 ships the admin endpoint; Plan 2 wires the handler).

### Acceptance criteria

- Scanner runs on cadence; each ingested content's shards verified within first full-scan window.
- Bit-rot simulation (manually flipped byte on disk) → verification finds it → `<elohim-resilience-snapshot>` icon flips to red within one scan cycle (or immediately via admin trigger).
- Peer-lying simulation (peer claims Have, Get fails) → `verification_breaches` row against lying peer.
- `/verify` endpoint returns honest report on-demand.
- No auto-reconstruction yet (breaches remain as signals).

**Non-goals for Plan 2:** auto-repair; attestation; DHT touchpoint.

---

## 7. Plan 3 — Auto-reconstruction (rung d)

### P2P Design Gate

```
### Entity: reconstruction_intents (local + libp2p gossip, Category C)
- Classification: Operational — ephemeral claim of intent for stampede defense.
  Broadcast to neighbors; garbage-collected at expiry.
- Justification: DHT would be massive overkill (thousands of reconstructions per
  month in a healthy mesh). Claim is interesting only within its expiry window;
  truth of reconstruction is the shard appearing.
- Fields: shard_hash, claimant_peer_id, target_peer_id, expires_at, status.
- Rebuild strategy: re-derive from verification_breaches + shard_locations diff.
- Anti-Pattern Check: no DHT entity.

### Entity: recovery_events (local, Category C; shefa signal)
- Classification: Operational, surfaced as structured shefa signal.
- Fields: content_id, shard_hash, from_peer_id, to_peer_id, household_changed
  (bool), trigger (breach/peer-gone/cadence), duration_ms, occurred_at.
- Not authoritative for trust (rung e's job); this is history for operator insight.
- Anti-Pattern Check: no DHT entity; no CID FK.

No new DHT entry types.
```

### Components

1. **`ReconstructionCoordinator`** (`elohim-storage/src/services/reconstruction.rs` new) — tokio task. Triggered by (a) `verification_breaches` insert, (b) `peer_statuses` → `absent`, (c) cadence scan. For each gap: runs the Plan 1 selector for a replacement candidate, broadcasts `ReconstructionIntent`, waits T seconds for competing intents, executes on win.

2. **Pull-mode reconstruction flow** — the **replacement** peer pulls K-of-N surviving shards via existing `ShardRequest::Get`, Reed-Solomon decodes the missing shard, holds it. Detector coordinates only; it never carries the payload. This is the "sovereignty through embeddedness" shape.

3. **Stampede defense** — deterministic arbitration: eligible peer with lowest `blake3(shard_hash || peer_id || epoch)` wins. All peers compute the same ordering; only one reconstructs. `ReconstructionIntent` gossip confirms the decision; losers stand down.

4. **libp2p protocol extension** — add `ShardRequest::Reconstruct { shard_hash, surviving_locations: Vec<PeerId>, deadline }` as enum variant on `/elohim/shard/1.0.0`. Replacement acks, then pulls via existing `Get`. No new protocol ID.

5. **Failure escalation** — surviving shards below recovery threshold (< K of N) → content unrecoverable via dataplane → `placement_gaps` row with `gap_kind=unrecoverable`. Shefa-grade signal for human/operator escalation. No silent data loss; no runaway retry.

6. **Shefa Signals Recovery card** — reads `recovery_events` (last 24h: N reconstructions across M households). Per-content drilldown shows timeline.

7. **`<elohim-resilience-snapshot>` v3** — adds "last recovery" to context panel; recovery timeline to full card.

### Cadence + dev-mode controls

Four-layer control applies to coordinator cadence and intent expiry. Synchronous trigger: `POST /api/v1/admin/chaos/trigger-reconstruction`.

### Acceptance criteria

- Chaos test: kill a peer holding shards of content X → within ≤60 s, `shard_locations` reflects new replacement peer, verification passes, household diversity restored.
- Stampede test: two peers detect same gap within same tick → exactly one reconstruction occurs (log + `recovery_events` prove it).
- Under-recovery: RS 4+3 content reduced to < 4 surviving shards → `placement_gaps` emits `unrecoverable` within one coordinator cycle.
- Household-operator notification: when reconstruction succeeds, `recovery_events` is queryable and shefa surfaces "household X lost a peer; mesh healed N shards." Contract-renegotiation (Option (c) in brainstorm) deferred to downstream shefa work.

**Non-goals for Plan 3:** attestation; contract renegotiation (reconstruct within existing commitments only).

---

## 8. Plan 4 — Attested holdings (rung e)

### P2P Design Gate

```
### Entity: ShardHoldingAttestation (B2 — Agent-Scoped with Notarized Attestation)
- Classification: B2. Raw holding data (on-disk location, verify history) is
  agent-scoped (private); the fact-of-holding per commitment is publicly
  verifiable via a signed attestation.
- Justification: At this rung we need cross-peer trust in the census — the ability
  to prove, without taking a peer's word, who holds what against which commitment.
  B2 is the exact shape: private raw state + public attestation of outcome.
- DHT Entry Type: REUSE existing Attestation entry type in imagodei DNA (per
  project_depin_contracts_are_policy — no new entry types; imagodei stays ≤28).
- Content Address Strategy: Agent-Scoped Composite —
  (AgentPubKey, shard_hash, commitment_hash). Uniqueness: one active attestation
  per (agent, shard, commitment) tuple.
- Address Justification: Not CID (claim has no intrinsic content beyond fields);
  not slug (author identity is load-bearing); composite because two agents
  attesting the same shard against different commitments are both meaningful.
- Source of Truth: Holochain DHT (imagodei Attestation entry).
- Coordinator Zome: imagodei::record_shard_holding_attestation,
  imagodei::get_holding_attestations_for_shard (extend existing Attestation
  readers if they don't cover the shape).
- Storage Projection: shard_holding_attestations (dht_anchor_hash: YES) — local
  read-optimized table via post-commit signal.
- HTTP Route: served through the existing resilience endpoints — augments
  /api/v1/resilience/{id} with attestedHoldings.
- Anti-Pattern Check: no new entry type (reuses Attestation); composite address
  prevents duplicate claims; dht_anchor_hash present; source-of-truth comment in
  migration.
```

### Components

1. **Attestation publisher** (`elohim-storage/src/services/attestation_publisher.rs` new) — tokio task. Iterates `shard_locations WHERE peer_id = self AND verification_status = verified`; composes a `ShardHoldingAttestation` per (shard, commitment) pair; calls the imagodei coordinator fn. Rate-limited: one attestation per (shard, commitment) per T (default 24h) unless state changes.

2. **Post-commit signal handler** — on `AttestationRecorded` for shard-holding shape, elohim-storage writes `shard_holding_attestations` projection. Mirrors the PeerStatus pattern.

3. **Attested-vs-claimed census** — `/api/v1/resilience/{id}` gains `attestedHoldings: { peer_id, household_id, commitment_hash, attested_at }[]` alongside existing `shards[].peer_ids`. Trust-divergence surfaces when a libp2p-gossip claim has no matching attestation.

4. **Breach-of-commitment signal** — if `verification_breaches` finds a shard with an outstanding attestation from the same peer (attestation says hold; verify says no) → high-priority `placement_gaps` row with `gap_kind=attested-breach`. This is the evidence hook for the gate-challenge-and-indemnification spec. Campaign does **not** file challenges; it makes the evidence available.

5. **`<elohim-resilience-snapshot>` v4** — attestation badge on icon; "attested by N peers" in context panel; attested census vs claimed census on full card. Doorway admin surfaces `attested-breach` prominently.

### Cadence + dev-mode controls

Four-layer control on publisher cadence. Synchronous trigger: `POST /api/v1/admin/chaos/trigger-attestation`.

### Acceptance criteria

- Ingest → distribute → verify → attest: each held shard gains a B2 attestation within one publisher cycle.
- Kill a peer → verify catches gap → the dead peer's outstanding attestation becomes `attested-breach` in `placement_gaps`.
- Resilience view distinguishes claimed-only (libp2p gossip) from attested-and-verified (DHT).
- Gate-challenge spec's evidence hook is populated (Phase 11 consumer not built; its input surface is).
- No new DNA entry type added; imagodei DNA stays ≤28 entry types.

**Non-goals for Plan 4:** filing challenges; indemnification flows; universal-band review. Those belong to the gate-challenge spec.

---

## 9. Plan 5 — Chaos demo on shem

### Components

1. **Chaos admin endpoints** (`elohim-storage/src/api/chaos.rs` new) — authenticated + gated by `CHAOS_ENABLED=true` env/manifest flag; production refuses with 403. Endpoints:
   - `POST /api/v1/admin/chaos/kill-peer` — graceful halt of named peer's conductor
   - `POST /api/v1/admin/chaos/corrupt-shard` — flip a byte at shard's on-disk location
   - `POST /api/v1/admin/chaos/drain-peer` — trigger maintenance choreography
   - `POST /api/v1/admin/chaos/trigger-verify` — synchronous scan (Plan 2 endpoint under /admin umbrella)
   - `POST /api/v1/admin/chaos/trigger-reconstruction` — force coordinator pick
   - `POST /api/v1/admin/chaos/redistribute-now` — force distribute_shards
   - `POST /api/v1/admin/chaos/trigger-attestation` — force publisher cycle
   
   These are the synchronous hooks the cadence memory insists on — chaos tests never wait for timers.

2. **Drain guard / maintenance choreography (Module J)** — peer draining advertises `PeerLifecycleState::Leaving` in PeerStatus; holds conductor-stop until replacement holdings are attested (or cadence timeout); HeartbeatControl stops last. Terrance-graceful-offline path.

3. **H2 — /admin/users JWT scope fix** — doorway config aligns JWT scope requirements to grant `users:read` to authenticated admins. Config-only; no code.

4. **A2O chaos feature files** (`genesis/a2o/features/resilience/*.feature`):
   - `terrance-offline-household-survives.feature` — the grandma demo canonical flow
   - `peer-kill-reconstruction.feature` — kill + healing
   - `drain-guard-orderly-handoff.feature` — graceful leave
   - `bit-rot-detection-and-repair.feature` — verification + reconstruction loop
   - `attested-breach-evidence.feature` — rung e evidence production
   - `placement-gap-recruits-subsidies.feature` — shefa signal surface
   - `chaos-endpoints-prod-refused.feature` — safety test
   - `four-dashboards-live-on-shem.feature` — Module L's original intent

5. **Chaos step definitions** (`genesis/a2o/support/steps/chaos-steps.ts` new) — Cucumber step definitions bridging Gherkin to chaos admin endpoints. Includes bounded-poll helpers (*"Then content X recovers within 60 seconds"* → polls `/api/v1/resilience/{id}` with timeout).

6. **Shem manifest pass** — ensure env publishing for DEVICE_ARCHETYPE, HOUSEHOLD_ID, NODE_ROLE (already landed); add CHAOS_ENABLED on dev pods only; ensure diverse archetypes across pods so diversity selection has room. Cycle pods.

7. **Shem acceptance run** — full a2o suite on shem, dashboards watched during execution. Recording cut of the Terrance-offline scenario.

### Acceptance criteria

- All 8 chaos scenarios green on shem (not localhost).
- All four dashboards (`/shefa/devices`, `/shefa/resources/category:content`, `/shefa/dashboard` Network Health, `doorway-alpha/threshold/dashboard`) show real data during + after a chaos run.
- `<elohim-resilience-snapshot>` on content-viewer accurately flips icon color through a healing cycle.
- Terrance-offline demoable live — peer drops, UI goes yellow, reconstruction fires, UI goes green, recovery event in shefa Signals.
- Safety test: any environment without `CHAOS_ENABLED=true` refuses chaos endpoints with 403.
- Module L intent folded in — no Phase-1 regression scenarios left orphaned.

**Non-goals for Plan 5:** expanding beyond shem (prod validation, grandma-mini-PC install); video production beyond internal; contract-renegotiation workflows.

---

## 10. Cross-cutting concerns

### Testing strategy (layered)

| Layer | What it proves | Per plan |
|---|---|---|
| **Rust unit (`cargo test --lib`)** | Peer selector diversity ordering; RS decode math; stampede arbitration determinism | 1-4 |
| **Sweettest** (Holochain integration) | Rung e attestation lifecycle on a multi-conductor sandbox | 4 |
| **elohim-storage integration** | Distribute → verify → reconstruct flow against real SQLite with simulated peers | 1-3 |
| **Vitest (Angular)** | `<elohim-resilience-snapshot>` renders correctly across densities | 1-4 |
| **Cypress (BDD)** | Dashboard surfaces update when resilience data changes | 1-4 |
| **A2O on shem** | Chaos scenarios end-to-end with real peers | 5 |

### Error handling principles

- **Distribution failures**: not silent drops — become `placement_gaps` rows.
- **Verification disagreements**: breach recorded; trust-divergence surfaced in attested-vs-claimed (rung e).
- **Reconstruction under-quorum**: explicitly `unrecoverable` with surviving shard count; human-escalation signal.
- **Protocol mismatches** (peer speaks older shard protocol): fall back gracefully; log once per peer; don't crash.

### Observability

- `recovery_events`, `verification_breaches`, `placement_gaps` are the three primary signal tables — all surfaceable via `/api/v1/...` endpoints.
- Structured logging with `tracing` spans around distribute / verify / reconstruct / attest.
- No new metrics backend in-scope; stdout + shefa signals is the observation surface.

### Schema-first (per feedback_schema_first_ioc)

Every new view gets a JSON schema in `elohim/sdk/schemas/v1/views/` **before** the Rust struct is written. Schema contract test catches drift. New views:

- `ResilienceSnapshotView` (Plan 1 enrichment)
- `PlacementGapView` (Plan 1)
- `VerificationReportView`, `VerificationBreachView` (Plan 2)
- `RecoveryEventView`, `ReconstructionIntentView` (Plan 3)
- `AttestedHoldingView` (Plan 4)
- `ChaosTriggerInputView` + responses (Plan 5)

### Cadence + dev-mode defaults (initial proposal)

| Operation | Edge (L0-2) | Intermediate (L3) | Archival (L4-5) | Dev override |
|---|---|---|---|---|
| Verification — full scan | 6h | 3h | 1h | `ELOHIM_VERIFY_INTERVAL` |
| Verification — priority scan | 1h | 30m | 15m | `ELOHIM_VERIFY_PRIORITY_INTERVAL` |
| Cross-peer Have sample rate | 5% | 10% | 20% | `ELOHIM_VERIFY_SAMPLE_RATE` |
| Reconstruction coordinator cadence | 10m | 5m | 2m | `ELOHIM_RECONSTRUCTION_INTERVAL` |
| Reconstruction intent expiry | 60s | 30s | 15s | `ELOHIM_INTENT_EXPIRY` |
| Attestation publisher cycle | 24h | 12h | 6h | `ELOHIM_ATTESTATION_INTERVAL` |

All are operator-overridable via `peer-policy.toml`. All have synchronous admin triggers. Initial numbers are conservative; tune after shem observation.

### Schema + DNA impact

| DNA | Delta | Entry type count |
|---|---|---|
| infrastructure | none | 6/~100 (unchanged) |
| lamad | none | ~73/~100 (unchanged) |
| mishpat | none | 11/~100 (unchanged) |
| imagodei | zero new entry types; may extend coordinator fns on existing Attestation | 28/~100 (unchanged) |

---

## 11. Sequencing + review checkpoints

```
Plan 1 ships → user reviews running behavior on dev cluster →
Plan 2 ships → user reviews; chaos endpoint handlers stubbed in advance of Plan 5 →
Plan 3 ships → full self-healing visible on dev →
Plan 4 ships → attested census visible; gate-challenge evidence hooks populated →
Plan 5 ships → chaos suite green on shem; grandma demo runs end-to-end
```

Each plan is independently valuable:

- **Plan 1 alone** makes the dashboards honest.
- **Plan 2** adds the observability the verification claims.
- **Plan 3** closes the repair loop.
- **Plan 4** adds trust-verified census.
- **Plan 5** proves the whole thing on real peers.

User can pause the campaign between plans without stranding value. Each plan gets its own implementation plan via `superpowers:writing-plans`, executed via `superpowers:executing-plans` or `superpowers:subagent-driven-development`.

---

## 12. Explicit non-goals (campaign-wide)

- Model-diversity axis (L7 guardian loop).
- Private-content encryption for pull-mode replication.
- Appliance packaging (single-container grandma install).
- Contract renegotiation workflows (shefa-initiated).
- Region-aware placement (region is display-only in this campaign).
- Cross-protocol challenges + indemnification (belong to the gate-challenge spec; this spec only populates the evidence hook).
- elohim-agent-mediated reconstruction policy (deferred to guardian-loop work).
- Metrics backend / Prometheus / Grafana integration.

---

## 13. Success definition

**The campaign succeeds when, on shem, a Gherkin scenario can say:**

```gherkin
Given Terrance's household has a node with content provisioned
And a remote family household holds reciprocal storage commitments
When Terrance's node goes offline
Then within 60 seconds content resilience is restored on a diverse replacement peer
And the content-viewer resilience icon flips red → yellow → green
And a recovery event appears in shefa Signals
And the household operator can see "household X lost a peer; mesh healed N shards"
And an attested-breach signal is recorded if the offline node had outstanding attestations
And production refuses all chaos endpoints with 403
```

...and the scenario is green in CI, reproducible on demand, recorded as a demo.

That is grandma-grade P2P — not a claim, a proof.
