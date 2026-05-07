# Topology Substrate Completion — Design Spec

**Status**: design (brainstormed 2026-05-07)
**Sprint codename**: topology-substrate-completion
**Predecessor**: [2026-05-01-light-up-the-topology-design.md](./2026-05-01-light-up-the-topology-design.md), delivered to `delivered-shape` for D2/D3, `delivered` for D4/D5, `error_state` for D1/D6 (see [sprint-result.md](../../../../.claude/shifts/light-up-the-topology-deliver-2026-05-07T04-20/sprint-result.md))
**Successor target**: full 6/6 `delivered` for resilience and social compute epics

## Problem statement

The `light-up-the-topology` sprint delivered the substrate code for all six topology surfaces but only 2 of 6 surfaces are fully `delivered` because:

1. The seeders that produce the data those surfaces aggregate over are unwired and (more critically) **wrong-shape** — they would still not light up the surfaces if wired as-is. Specifically:
   - `seed-commitments.ts` writes `action="provide"` with `provider="human-matthew-manager"` and `receiver="network"`. The reciprocity_view, cluster_view, and distribution_view SQL predicates filter on `action="custody-blob"` AND `provider IN (peer_id_set_from_AgentPeerBindings)`. Wiring the existing seeder would write rows that the views silently filter out.
   - `seed-agent-bindings.ts` and `seed-conductor-identities.ts` are not wired into any deploy pipeline; they require in-cluster CONDUCTOR_URLS that no current job builds.

2. `peer_topology_view::build_local_slice` returns hardcoded `{connected_peer_households: []}` and `cluster_view::build_local_slice` returns hardcoded zero-valued metrics. Even with bindings + commitments seeded correctly, both views aggregate empty federation slices.

3. The alpha cluster has only 3 active conductors (matthew/jessica/timothy) after shem's decommissioning. Cross-household reciprocation pairs from matthew's perspective collapse to `{timothy}` — not enough graph for D3 to feel real.

4. No ContentNode on alpha has a non-null `blobHash`, so D1 (distribution-badge) and D6 (resilience-snapshot in content-viewer) correctly hide per their conditional render logic but the surfaces never light up.

## Sprint scope (substrate-driven, all the way)

The user has chosen substrate-driven delivery: real DHT writes, real REA flows, real libp2p connected-peer queries, real blob transfers between alpha pods. No fixture-stubbed slices. No render-driven shortcuts.

**Out of scope** (named explicitly to prevent drift):
- Long-term keypair-derived peer_ids (Stage 2 of the AgentPeerBinding work). Stage 1 deterministic peer_ids are acceptable per existing seeder comment.
- Replacement of shem hardware (multi-week project, separate ops sprint).
- Visitor-no-auth distribution-badge scenario (separate handoff brief).
- Surface-3 imagodei account management UI (separate sprint).

## Approach: vertical slice, then broaden, then polish

Three milestones; each ends in a stable fresh-trigger-verifiable state. No partial deliveries.

### M1 — Vertical slice (matthew↔timothy)

Land all 6 surfaces real end-to-end for one cross-household pair. Skips adam entirely. Smallest exercise of every layer (DHT bind → REA write → libp2p slice → federation aggregate → UI render).

**Components**:

#### M1-A. `cluster_view::build_local_slice` real implementation
Returns this peer's live device-tile contribution. Replaces the stub at `elohim/elohim-storage/src/services/cluster_view.rs:252`.

Field sources:
- `display_name` ← env `ELOHIM_DISPLAY_NAME` (already supported at line 255 — needs to be set on each pod from `deployments.json.humanLabel`)
- `storage_used_bytes` ← `du`-equivalent on the BlobStore filesystem path
- `storage_total_bytes` ← `statvfs`/`statfs` on the same path (mounted PVC capacity)
- `memory_used_bytes` / `memory_total_bytes` ← `/proc/self/status` (VmRSS) and `/proc/meminfo` (MemTotal)
- `hosting_count` ← `SELECT COUNT(*) FROM peer_blob_inventory WHERE peer_id=local_peer_id`
- `projecting_count` ← `SELECT COUNT(*) FROM content WHERE source_peer_id=local_peer_id` (or equivalent local content count — confirm field name during implementation)
- `beacon_age_ms` ← `now - libp2p.last_beacon_ts` (if exposed via Swarm event-bus snapshot; else 0)

#### M1-B. `peer_topology_view::build_local_slice` real implementation
Replaces the stub at `elohim/elohim-storage/src/services/peer_topology_view.rs:215`.

Returns `{connected_peer_households: [{ household_id, display_name, online, last_sync_sec, my_cids_hosted_by_them, their_cids_hosted_by_me }]}`.

Resolution chain:
1. `Swarm.connected_peers()` → `Vec<peer_id>`.
2. For each peer_id, look up `peer_identity_bindings WHERE peer_id=? AND valid_until_micros IS NULL OR valid_until_micros > now`, picking `MAX(valid_from_micros)` row.
3. For each `agent_cid` from step 2, look up `humans WHERE id=?` to resolve `household_id` and `display_name`.
4. CID counts: `my_cids_hosted_by_them` = count of peer_blob_inventory rows where peer_id=other AND blob is one we authored; `their_cids_hosted_by_me` = the inverse. (Authored-set resolution to be designed during implementation; default to "blob present in our local store" if no authoring metadata.)
5. `last_sync_sec` ← `MAX(updated_at)` from peer_blob_inventory for that peer_id.

Failure modes (silent edge skip, log warning):
- Step 2 returns 0 rows → peer has no binding → skip edge.
- Step 3 returns 0 rows → agent has no Human entry on this peer → emit edge with `display_name: None`.
- `household_id` is NULL → emit edge with `household_id: agent_cid` as fallback identifier.

#### M1-C. `seed-commitments.ts` rewrite for custody-blob shape
Replaces the existing capacity-style seeder shape with custody-blob shape.

POST body to `/api/v1/commitments`:

```json
{
  "id": "<deterministic-id-per-pair-and-blob>",
  "action": "custody-blob",
  "provider": "12D3KooW<peer_id_suffix>",
  "receiver": "12D3KooW<peer_id_suffix>",
  "resourceConformsTo": "blob",
  "resourceClassifiedAs": "sha256-<blob_hash>",
  "resourceQuantity": { "hasNumericalValue": <bytes>, "hasUnit": "B" },
  "note": "<provider> commits to host blob <hash> for <receiver>",
  "metadata": { "seedGeneration": "genesis", "blobHash": "sha256-<hash>" }
}
```

Shape contract (must match consumer SQL predicates):
- `action == "custody-blob"` (hyphen, lowercase)
- `provider` and `receiver` are `12D3KooW<sha256(humanId:archetype)[:38]>` peer_ids — not human-* agent_cids
- `resourceClassifiedAs` is the raw blob hash with `sha256-` prefix (matching `reconcile/custody.rs:132` read convention)
- `resourceQuantity.hasNumericalValue` is bytes as an integer, `hasUnit: "B"`

**Shared peer_id derivation utility**: extract `deterministicPeerId(humanId, archetype)` from `seed-agent-bindings.ts:119` into a shared module `genesis/seeder/src/peer-id.ts`. Both bindings seeder and commitments seeder import it. Drift here = silent empty SQL results.

#### M1-D. Genesis Jenkinsfile wiring
Add three sequential stages after the existing `seed-accounts` stage at `genesis/Jenkinsfile:870-925`:

```
stage('Seed Conductor Identities') { ... runs seed-conductor-identities.ts ... }
stage('Seed Agent Peer Bindings') { ... runs seed-agent-bindings.ts ... }
stage('Seed Custody Commitments') { ... runs seed-commitments.ts (rewritten) ... }
```

CONDUCTOR_URLS resolution: build from `getHumanStorageUrls('alpha')` (existing helper at line 79), filtered to `agencyPhase IN (node, doorway, device)` AND `!suspended`. Pattern: `ws://elohim-${name}-alpha:4445`. K8s socat convention already provides admin port = app port - 1.

Idempotency:
- seed-conductor-identities short-circuits via `get_my_human` check (already correct).
- seed-agent-bindings creates duplicates per Stage 1 — view aggregation MAX(valid_from_micros) handles this correctly. Acceptable.
- seed-commitments must use **distinct ids per pair+blob+direction** so 409s only fire on genuine re-runs, not on shape changes during development.

**Error policy**: change seed-commitments to **fail-fast on non-409 HTTP errors** (currently swallows all errors with `[!]` warnings). CI must reject silent shape drift.

#### M1-E. Blob-backed content pick
Pick one existing manifesto chapter (e.g., `genesis/docs/content/elohim-protocol/manifesto/02-fruit-back-on-the-tree.md`) and reimport it through the `account-package` upload path so it lands as a blob-backed ContentNode with non-null `blobHash`.

Verify EPR head response hydrates both `distribution` (replica count, hosting peers — sourced from `distribution_view.rs`) and `resilience` (cliff state — sourced from quilt distribution layer). Either or both may have hydration gaps that surface during M1 verification — those become M3 line items.

#### M1-F. Real fetch orchestration
Add a CI step that, after seeders run, triggers a real blob fetch from timothy's pod requesting matthew's manifesto-chapter blob. The substrate's `p2p/blob_fetch.rs:206` then auto-emits a `serve-blob` EconomicEvent on success.

Implementation: `curl -s 'http://elohim-timothy-alpha:8090/blob/<hash>'`. Forces the fetch path. Verify post-fetch that `economic_events WHERE action='serve-blob'` has at least one row matching the (provider=matthew_peer_id, receiver=timothy_peer_id) pair.

#### M1-G. Verification harness
Local Playwright probe (per prior sprint precedent) that logs in as matthew, screenshots all 6 surfaces, and asserts non-empty data shape. Accept-criteria:
- D1: `<elohim-distribution-badge>` renders (manifesto chapter has blobHash + hydrated distribution).
- D2: at least 1 device-tile in /shefa/cluster.
- D3: at least 1 peer-household-card in /shefa/peers (timothy's household).
- D4: at least 1 inflow row + 1 outflow row in /shefa/reciprocity (matthew↔timothy bytes).
- D5: doorway dashboard topology tab unchanged from prior `delivered`.
- D6: `<elohim-resilience-snapshot>` renders side-by-side with `<elohim-distribution-badge>` in content-viewer for the manifesto chapter.

a2o coverage: write Gherkin scenarios as the verification step (per project memory `feedback_a2o_is_human_experience_not_dev_bugs`) — describe the human experience, not the data-shape correctness.

### M2 — Broaden (adam reactivation + cross-household graph)

#### M2-A. Adam reactivation
Edit `genesis/orchestrator/data/deployments.json` adam entry: set `suspended: false`, change `nodeTypes` from `["remote", "performance"]` to `["operations", "performance"]` (alpha labels available after shem's loss). Preserve `genesisPeer: true`.

Conductor genesis: fresh agent key (seeded chain lost with shem PVC), empty source chain. seed-conductor-identities runs against adam's pod and creates the Human entry pinned to `id="human-adam-firstman"` from humans.json.

Identity-drift mitigation: keep humans.json's `id` as the canonical reference (existing pattern). The Holochain agent_cid — different from `id` — is internal to the conductor's source chain. View predicates that join on `humans.id` continue working unchanged. Verify in M1 that this pattern actually works for matthew↔timothy before M2 lands.

If M1 surfaces that view predicates expect agent_cid (not humans.id) on certain joins, M2 documents the gap and either (a) re-keys downstream seed data, or (b) lands a `humans.holochain_agent_cid` column to bridge the two identifiers.

#### M2-B. Cross-household custody-blob seeding
Extend the rewritten seed-commitments to emit pairs across the 3-household graph:
- matthew↔adam (new — household-matthew × household-adam)
- timothy↔adam (new — household-timothy × household-adam)
- matthew↔timothy (already from M1)

Volume per pair: 1-3 distinct blob_hashes per direction. Not load-testing; validating shape multiplies cleanly.

#### M2-C. Cross-pair fetch orchestration
Extend M1-F's CI fetch step to trigger fetches across all 3 pairs (6 directional fetches total). Each successful fetch emits a serve-blob event; D4 then shows 3 inflow rows + 3 outflow rows for matthew, similarly for adam and timothy.

#### M2-D. Verification
Playwright probe extended to assert: ≥2 peer-household-cards in D3, ≥2 inflow rows + ≥2 outflow rows in D4. Other surfaces unchanged.

### M3 — Polish

#### M3-A. distribution-badge peer-coverage detail
Surface per-peer hosting state in the badge (which household holds this CID, replica count, last sync). Data already in `distribution_view.rs` if hydrated; verify hydration path for the M1 manifesto chapter.

#### M3-B. resilience-snapshot real cliff calculation
Compute `resilience_cliffs` (currently `vec![]` per `peer_topology_view.rs:182`) from real quilt distribution. A "cliff" = a CID with sole replica, or a peer whose disappearance would orphan N CIDs.

Depends on quilt distribution layer surfacing per-CID replica sets (existing TODO in same file).

#### M3-C. Reciprocity counterparty annotation
Resolve `display_name` for each reciprocity row's counterparty (currently `None` per `reciprocity_view.rs:160`), and annotate `online` from libp2p connected-peers (currently `None` at line 163).

#### M3-D. Reach-stretch goals (optional)
- `capacity_available_bytes`: device totals minus committed (existing TODO at `reciprocity_view.rs:78`).
- `freshness AllOffline` graceful handling on the surface when no peer is Live.

## Source-of-truth boundaries

Unchanged from `2026-05-01-light-up-the-topology-design.md`:

| Layer | Role | New entities introduced this sprint |
|-------|------|-------------------------------------|
| Holochain DHT | Notary | None (extends AgentPeerBinding, REA Commitment custody-blob, EconomicEvent serve-blob, Human — all existing entry types) |
| libp2p runtime | Operational data plane | None (uses existing Swarm.connected_peers() API) |
| SQLite projection | Read-optimized cache | None (uses existing peer_identity_bindings, rea_commitments, economic_events, peer_blob_inventory, humans tables) |
| Doorway | Web2 projection | None (uses existing /api/v1/commitments POST, /api/v1/cluster, /api/v1/peer-topology, /api/v1/reciprocity) |

P2P design gate (Section 3 of [skills/p2p-design-gate](../../../../.claude/skills/p2p-design-gate/SKILL.md)) confirmed: no new entity types, no new HTTP routes, no DNA capacity concerns. Lamad ~73/~100, Mishpat 11/~100 unchanged. This sprint is operationalization, not entity introduction.

## Cross-component shape contracts

These contracts MUST hold or surfaces silently render empty.

### Contract C1 — peer_id derivation parity
Single source: `genesis/seeder/src/peer-id.ts` (new shared module). Formula:
```
peer_id = "12D3KooW" + sha256(`${humanId}:${archetype}`).hex.slice(0, 38)
```

Both `seed-agent-bindings.ts` and `seed-commitments.ts` import this. Drift here = silent empty SQL for cluster + reciprocity views.

### Contract C2 — custody-blob commitment field shape
Receiver-side (`reciprocity_view`, `cluster_view`, `distribution_view`, `reconcile/custody.rs`) read convention:
- `action == "custody-blob"`
- `provider`, `receiver` are peer_id strings (12D3KooW...)
- `resource_classified_as` is the raw blob hash with `sha256-` prefix (no `blob:` prefix per `reconcile/custody.rs:132`)
- `resource_quantity_value` is f32 bytes count

Producer-side (`seed-commitments.ts` POST body camelCase per `views.rs::CreateReaCommitmentInputView`):
- `action: "custody-blob"`
- `provider`, `receiver`: peer_ids
- `resourceClassifiedAs: "sha256-<hex>"`
- `resourceQuantity: { hasNumericalValue: <int>, hasUnit: "B" }`

Schema contract test in `tests/schema_contract.rs` extended to assert this shape round-trips through `CreateReaCommitmentInputView`.

### Contract C3 — agent_cid ↔ household_id resolution chain
Read by `peer_topology_view::build_local_slice`:

```
Swarm.connected_peers()                       → Vec<peer_id>
peer_identity_bindings.peer_id = ?            → agent_cid
humans.id = agent_cid                         → household_id, display_name
```

Requirements:
- humans table populated on each peer's storage with `id`, `household_id`, `display_name` (`humans` table exists per migration `2026-04-19-000002`; verified)
- `humans.household_id` populated for each seeded human (currently nullable; adam reactivation must set this explicitly)
- `peer_identity_bindings` populated by seed-agent-bindings on each peer's DHT (projected via post-commit signal to local SQLite — verified existing path)

## Error handling

Per project memory `feedback_haiku_observe_only_no_specifics`: errors must be loud and shape-attributable.

| Failure | Where | Detection / response |
|---------|-------|---------------------|
| Doorway POST 400 (custody-blob shape mismatch) | seed-commitments | **Fail-fast**, exit non-zero, log full HTTP body |
| 409 from doorway | seed-commitments | Treat as idempotent re-run, log `[=]`, continue |
| seed-agent-bindings can't reach conductor | admin WS connect | Existing `[X] failed` + non-zero exit — no change |
| build_local_slice panic on missing libp2p Swarm | elohim-storage runtime | Wrap panic → return empty payload with `freshness: AllOffline`, log warning |
| peer_id ↔ agent_cid join returns 0 rows | peer_topology_view | Log `WARN: peer_id <X> has no binding`, skip edge silently |
| Real blob fetch fails (peer offline) | M1-F / M2-C | CI step exits non-zero → milestone unverified, do not advance |
| Adam conductor genesis fails | M2-A | Pre-flight check before seeders run; abort seed stage if conductor unreachable |

Anti-pattern to prevent: silent `[=] already exists` masking shape drift. Use distinct commitment IDs per `(pair, blob, direction)` tuple so 409s only happen on genuine re-runs.

## Testing strategy

Layered per `feedback_a2o_is_human_experience_not_dev_bugs`:

- **a2o (Gherkin)**: extend topology scenarios in `genesis/a2o/features/` to assert human-experience promises:
  - "matthew sees timothy's household card on his peer-topology page" (M1)
  - "matthew sees adam's household card on his peer-topology page" (M2)
  - "matthew's reciprocity page shows ≥1 inflow row from timothy" (M1)
  - "matthew sees a distribution badge on the manifesto chapter content viewer" (M1)
  - "matthew sees the resilience snapshot side-by-side with the distribution badge" (M1)
- **Unit / integration tests in elohim-storage**:
  - `cluster_view::build_local_slice` covered with fixture libp2p Swarms (mock peer set) + fixture SQLite databases
  - `peer_topology_view::build_local_slice` covered with same fixture pattern
  - Schema contract test in `tests/schema_contract.rs` extended for custody-blob commitment shape
- **Pre-push hooks**: shape contract violations caught before merge (per project policy)
- **Fresh-trigger probe**: per prior sprint precedent, local Playwright against alpha + CI Playwright via genesis pipeline — both must pass before declaring milestone delivered.

## Risks and known unknowns

1. **Adam identity-drift unknown until M1 lands**: the assumption that humans.id (not agent_cid) is the canonical join key for view predicates is tested implicitly by M1's matthew↔timothy slice. If M1 surfaces that views actually expect agent_cid on certain paths, M2 documents the gap and adapts.

2. **resilience-snapshot hydration gap for M3**: `resilience_cliffs` calculation depends on the quilt distribution layer surfacing per-CID replica sets, which is itself a TODO. M3 may surface a substrate gap that defers cliff computation to a later sprint; the spec acknowledges this is acceptable.

3. **`projecting_count` field source unverified**: `cluster_view::build_local_slice` lists this as `SELECT COUNT(*) FROM content WHERE source_peer_id=local_peer_id` but the field name needs confirmation during implementation. May not exist; falls back to 0 with note.

4. **`source_peer_id` for "my CIDs" determination is unspecified**: M1-B's CID-counting logic may need to fall back to a "blob present locally" heuristic if there's no authoring-peer metadata. Acceptable for M1; M3 designs a more accurate authoring-set resolution if surface counts feel wrong.

5. **alpha node label availability**: M2-A assumes alpha nodes carry `operations` and `performance` labels. Verify before adam's pod schedule. If only `performance` is available (single node label after shem), adam's pod competes with matthew's existing pod for that node.

## Memory candidates (write after operator validates spec)

- `feedback_custody_blob_shape_contract` — action="custody-blob", peer_id-keyed provider/receiver, resourceClassifiedAs is raw blob hash with sha256- prefix; SQL filters drop wrong-shape rows silently
- `project_topology_substrate_completion_2026_05_07` — vertical-slice approach; M1 = matthew↔timothy, M2 = adam reactivation, M3 = polish; substrate-driven (no fixture stubs)
- `feedback_peer_id_derivation_shared_utility` — single source `genesis/seeder/src/peer-id.ts`; both bindings + commitments seeders import; drift = empty SQL

## Out-of-scope cleanup (named so we don't drift)

- Stage 2 keypair-derived peer_ids
- Shem hardware replacement
- Visitor-no-auth distribution-badge scenario
- Surface-3 imagodei account management UI
- Path-viewer fallback for cluster-A (separate handoff brief)
