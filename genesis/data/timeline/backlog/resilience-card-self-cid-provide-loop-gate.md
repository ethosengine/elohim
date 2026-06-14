---
id: "backlog-resilience-card-self-cid-provide-loop-gate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "EPR resilience card zeros — APP-LAYER (in-pod conductor cell-readiness + SELF_CID unset), NOT netpol; reseed is not the lever"
slug: "resilience-card-self-cid-provide-loop-gate"
written: "2026-06-14"
updated: "2026-06-14"
author: "agentic-developer (felt-status shift, operator OPEN #1 diagnosis — corrected after operator cluster-read + runtime-triage)"
status: "partially-resolved"
priority: "high"
ci_status: blocked
tags: [resilience, dht-anchor, self-cid, provide-loop, conductor, cell-readiness, durability-arc, app-layer, shard-manifest, shard-locations]
cites:
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/config.rs
  - elohim/elohim-storage/src/services/provide_reconcile.rs
  - elohim/elohim-storage/src/services/conductor_commitment_author.rs
  - elohim/elohim-storage/src/services/household_resilience.rs
  - elohim/elohim-storage/src/services/shard_manifest_backfill.rs
  - elohim/elohim-storage/src/services/peer_selection.rs
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/db/shard_manifests.rs
  - elohim/elohim-storage/src/db/shard_locations.rs
  - genesis/seeder/src/seed-provide-rows.ts
  - genesis/manifests/humans/matthew-manager.yaml
  - genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
relatedNodeIds:
  - backlog-resilience-tier-content-declared-floor
  - backlog-self-heal-doorway-startup-conductor-mint-serialization
---

# EPR resilience card zeros — the self_cid provide-loop gate + the conductor boot-race

Diagnosis of the sprinter handoff's OPEN #1 (`SPRINTER-HANDOFF-2026-06-14.md`),
done during the felt-status shift (the felt surface this lights: `feltStatus` on
ResilienceSnapshotView, commit 6a754f30f). Workstream D of the EPR Content
Durability Arc plan.

> **CORRECTION (2026-06-14):** an earlier draft framed this as "the durability
> arc's netpol gate → conductor reachable; operator runs netpol apply + reseed."
> **That was wrong** — propagated from the durability plan + handoff without
> independent verification. Operator cluster-read + the runtime-triage kill-log
> proved the cause is **app-layer, not netpol.** Corrected below.

## The chain (code-verified)

The card zeros (stewardingCollectives 0, commitmentBacked 0, no `content:<reach>`
provide rows) are gated, in order:

1. **`self_cid` is empty → the provide-loop never spawns.** `main.rs:959-966`
   spawns the P1 reconciliation WRITE half (the `replicates-content` authoring
   tick → writes the `content:<reach>` provide rows the snapshot reads) ONLY when
   `config.self_cid` is `Some` and non-empty. `self_cid` is sourced solely from
   the `SELF_CID` env (`main.rs:366-370`), which is **set in NO manifest** (the
   `elohim-storage` container env in `genesis/manifests/humans/*.yaml` has no
   `SELF_CID`). → permanently dormant; logs once: *"Slice-2b provide-loop
   authoring tick disabled: requires lamad HcClient + db pool + non-empty
   self_cid."*
2. **Even spawned, it authors via the in-pod conductor.**
   `provide_reconcile::reconcile_provides` → `conductor_commitment_author` Step 1
   notarizes via the lamad HcClient → the in-pod conductor.
3. **Content anchoring at import also needs the in-pod conductor.** genesis #1145
   seeder hit `reach circuit OPEN after 5 consecutive conductor-path failures →
   provenance-only stamping → rows stamp-failed` (cf. main.rs:616: failures happen
   "while the conductor's cells are still CellDisabled").

## The cause is APP-LAYER, NOT netpol (verified 2026-06-14)

- **elohim-storage → conductor is IN-POD over `ws://localhost:4444`**
  (`HOLOCHAIN_ADMIN_URL` in the human manifest; the `edgenode` conductor and
  `elohim-storage` are containers in the SAME pod). Not cross-pod, not
  cross-namespace → **no NetworkPolicy gates it.** The 5/5 failures are the in-pod
  conductor's cells being **CellDisabled during the boot/seed window** — app-layer
  readiness, not a blocked path.
- There is **no K8s `Service` named `conductor`**; cross-pod conductor admin is the
  per-pod `ws-proxy` socat on `:8444`. (That `:8444` path, with per-human URLs
  DNS-unresolvable for undeployed humans, is item 2 —
  `self-heal-doorway-startup-conductor-mint-serialization` — also app-layer.)
- `genesis/orchestrator/manifests/network-policies.yaml` (jenkins→conductor :8444,
  for CI *seeding* stages) is a DIFFERENT path and does not gate this card.

## CONFIRMED PRIMARY root causes — resolution status (2026-06-14 deep-dive)

The self_cid gate (below) is real but NOT the biggest blocker. A code-review +
direct schema verification found the snapshot's READ joins are broken at the
identity level — the card stays dark **regardless of seeding, self_cid, or
conductor.** These are the priority fix (all in `household_resilience.rs`):

- **R1 — steward join namespace mismatch [CONFIRMED → RESOLVED by identity-contract
  verification].** The join `shard_locations.peer_id == humans.agent_pub_key` is
  actually correct — `peer_statuses.peer_id` IS the Holochain agent key (heartbeat.rs
  sets it to AgentPubKey), and `distribute_shards` writes `shard_locations.peer_id`
  from `peer_selection::SelectedPeer.peer_id` which comes directly from `peer_statuses`
  (not libp2p). **The join namespace is coherent; R1 was based on a stale reading.**
  The real constraint: `shard_locations` rows don't exist because `distribute_shards`
  has never run with live peers online — no REST write path exists for these rows
  (see "Seed write path investigation" below).
- **R2 — commitment-join action mismatch [CONFIRMED → RESOLVED].** The working-tree
  parallel effort (`household_resilience.rs` diff on feat/frontend-eyes-sprint) updated
  the action filter to `eq_any(["provide", "replicates-content", "replicates-commons"])`.
  The seeder (commit 3bd72299f) writes `action: 'provide'` which now matches. After
  a reseed, `commitment_backed_collectives` will light.
- **R3 — provider identity [CONFIRMED → RESOLVED].** Commit 3bd72299f
  (`genesis/seeder/src/seed-provide-rows.ts`) seeds `provider = agent_pub_key` from
  each pod's `/auth/me` — the Holochain agent key (`uhCAka…`), matching
  `humans.agent_pub_key`. The join is correct; the seeder identity is correct.
- **R4 — content_reach default [STILL OPEN].** The `snapshot()` fallback to `"commons"`
  on `content.reach` lookup error remains. For `commons`-reach seeded content this is
  a no-op; for non-commons content this still misses. Low priority for current alpha
  corpus (most seeded content is commons reach).
- **R5 — commitment_backed_replication stub [STILL OPEN].** `compute()` always returns
  `CommitmentBackedReplication::default()`. Not addressed in current sprint.

**THE IDENTITY CONTRACT — NAILED DOWN (2026-06-14 deep-dive verification):**

The canonical Holochain agent key (`uhCAka…`) is the join key for ALL three tables:
- `humans.agent_pub_key` — set by conductor identity seeder + MembershipProjected signal
- `peer_statuses.peer_id` — set by heartbeat.rs to AgentPubKey (confirmed in code)
- `shard_locations.peer_id` — set by `distribute_shards` via `SelectedPeer.peer_id` which
  comes from `peer_statuses.peer_id` (same value → same namespace → join is correct)
- `rea_commitments.provider` — seeder (3bd72299f) writes agent_pub_key from `/auth/me`

The libp2p PeerId (`12D3KooW…`) does NOT appear in any of these join columns.
`peer_identity_bindings` is not needed for this join. The identity is settled.

**PROOF GATE:** A deterministic unit test that seeds coherent substrate rows and
asserts `snapshot()` returns `measured` + non-zero stewards + non-zero
commitment-backed + named collectives. It must FAIL today (no shard_locations) and
pass after real P2P distribution runs.

## Seed write path investigation — shard_manifests + shard_locations (2026-06-14)

The task was to investigate whether a seed stage could write `shard_manifests` and
`shard_locations` rows to light `distribution_state` and `stewardingCollectives`.

### shard_manifests — NO direct REST write path for inline-body content

Write paths that exist:
1. `POST /db/content` — spawns async task to call `record_manifest_from_bytes` ONLY when
   `blob_hash` is non-null AND blob bytes are in the BlobStore. No-op for existing rows
   (insert-only, not upsert).
2. `shard_manifest_backfill::run_once` — fires at storage boot. Same prerequisite:
   `content.blob_hash IS NOT NULL` AND blob bytes in BlobStore. Zero-cost to run
   repeatedly; idempotent.
3. `PATCH /db/content/{id}` with `blob_hash` — requires lamad conductor bridge
   (`patch_needs_conductor` returns true for blob_hash changes). Fails with
   "Conductor bridge unavailable" if conductor cells not yet enabled.

**Constraint for the alpha corpus:** Of 3431+ content files in
`genesis/data/lamad/content/`, only 1 has `blobHash` set (`evolution-of-trust.json`).
All assessments, sophia-quiz-json content, and FCT concepts use inline `contentBody`
or `content` — `blob_hash` stays NULL in the content row after seed. The backfill
finds ZERO candidates for this corpus. `distribution_state` stays `unmeasured`.

**What would fix it:** A new `PUT /admin/seed/shard-manifest` endpoint in
`elohim/elohim-storage/src/http.rs` that accepts explicit `{content_id, shard_hashes,
encoding, reach}` params and calls `db::shard_manifests::upsert_manifest` with a
synthetic manifest (encoding="none", one shard = the content body hash). This requires
an elohim-storage/src code change. The seed stage could then call it after content
creation.

### shard_locations — NO REST write path at all

The ONLY write path for `shard_locations` rows is `distribute_shards` in the P2P
runtime (`p2p/mod.rs`). It requires:
- A live P2P handle (not available in a seed stage)
- `peer_statuses` rows with online peers that have active `rea_commitments`
- Actual libp2p shard push to succeed (network transport)

There is no admin bypass, no seed endpoint, no SQLite-level write path from a genesis
seed stage. **`stewardingCollectives` can only light when real P2P distribution runs
with live peers.** This is a runtime concern — the seed cannot substitute for it.

### What a reseed DOES light after parallel effort lands

After the working-tree parallel effort merges (household_resilience.rs action filter
fix + related storage changes):

| Card column | After reseed | Constraint |
|---|---|---|
| `commitmentBackedCollectives` | **lights** (N = distinct households with provide rows) | provide-rows seeder writes agent-keyed rows; action filter now includes "provide" |
| `distribution_state` | stays `unmeasured` for inline-body content | no shard_manifest write path for inline content |
| `stewardingCollectives` | stays 0 | no shard_locations write path; requires real P2P distribution |
| `regionalDistribution` | collectives with `region` set in collectives.json are seeded by seed-collectives.ts | data present for 12 of 62 collectives |
| `feltStatus.reassurance` | will read from commitment-backed count | lights once commitmentBacked > 0 |

## The self_cid / conductor causes (real, but secondary to the join fixes above)

1. **`SELF_CID` config gap → provide-loop dormant** [code-verified]. No
   startup-derive; only the unset env. **Fix:** derive `self_cid` at startup from
   the in-pod conductor/agent identity (or inject it), so the loop isn't silently
   off.
2. **Reach-circuit boot-race → provenance-only** [corroborated by the item-2
   kill-log class]. The seed/import hits the in-pod conductor before its cells
   enable; the circuit OPENs after 5 and latches provenance-only for the whole
   run. **Fix:** gate the seed on conductor cell-readiness, AND/OR make the reach
   circuit recover (backoff-retry + re-stamp once cells enable) instead of
   latching.
3. (observability) surface the dormant provide-loop (`self_cid` empty) + the
   latched reach-circuit as `/p2p/status` flags, so "the card is dark because the
   loop is off / the circuit latched" is visible without log scraping.

## What a reseed lights now vs what requires runtime work

**A reseed NOW (after 3bd72299f provide-rows seeder + parallel effort action-filter fix):**
- `commitmentBackedCollectives` — lights (seed writes agent-keyed provide rows; filter matches "provide")
- `regional_distribution` — lights for households with region in collectives.json (12/62 collectives)
- `distribution_state` — stays `unmeasured` (no shard_manifest write path for inline content)
- `stewardingCollectives` — stays 0 (no shard_locations write path; P2P distribution required)

**What requires elohim-storage code changes (NOT a seed stage concern):**
- `distribution_state: measured` for inline-body content → needs `PUT /admin/seed/shard-manifest`
  endpoint (or content-upload-with-manifest path for non-blob content)
- `stewardingCollectives > 0` → needs live P2P `distribute_shards` with real peers online
  (runtime fix: self_cid derive at startup + conductor cell-readiness + mesh connectivity)

**Recommended sequence for full card lighting:**
1. Merge the parallel effort (household_resilience.rs action filter + related fixes)
2. Trigger a reseed — `commitmentBackedCollectives` lights
3. Land self_cid-derive-at-startup in elohim-storage (removes the provide-loop gate)
4. Wait for conductor cells to enable on live alpha pods (runtime P2P distribution begins)
5. `stewardingCollectives` lights once distribute_shards runs against online peers
6. For `distribution_state: measured` — add `PUT /admin/seed/shard-manifest` to
   elohim-storage and a corresponding genesis seed stage (tracked here as the open item)
