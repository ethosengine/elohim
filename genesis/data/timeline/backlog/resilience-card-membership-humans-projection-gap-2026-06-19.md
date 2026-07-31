# Resilience card dark: membership→storage-`humans` projection gap + non-matthew self-heal session

status: open
discovered: 2026-06-19 (shift `2026-06-19-ci-shepherd-card-deploy`)
domain: D-resilience / storage projection
relates: [[project_resilience_snapshot_humans_junction]] · `genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md` · seed-provide-rows.ts (Workstream D)

## Symptom
`GET /api/v1/resilience/evolution-of-trust/household` → `commitmentBackedCollectives: 0`, `stewardingCollectives: 0`, `onlinePeers.live: 0`, "No household is holding these yet" — after a full clean deploy (prof conductor + genesis self-heal env) and a genesis #1176 seed that completed with no 503s.

## What was PROVEN to work (this is NOT the gap)
- The **genesis self-heal mechanism functions**: for matthew, `/auth/me → HTTP 200`, `agent_pub_key healed`, provide row `content:commons` **CREATED + ACTIVATED** (genesis #1176 log).
- Substrate healthy: matthew `/health` peerCount 13, `caughtUp:true`, `projection.writer:true`, conductor 14/14 pools.
- `content.db` is a **PVC with `whenDeleted/whenScaled: Retain`** — persistent across restart/recreate (so this is NOT a restart-wipe).

## The gap (two distinct issues)
1. **matthew's successful, persistent seed is not reflected in matthew's reads.** Seed wrote `agent_pub_key` heal + active provide-row to matthew's storage (`elohim-matthew-alpha:8090`); the doorway reads of the SAME node show `/db/humans={items:[],count:0}`, `/peers/delivery=[]`, card=0. The storage `humans` table the snapshot joins on stays empty despite a caught-up projector. Per `resilience_snapshot_humans_junction`, `humans` is substrate-owned (DHT-membership-projected; "no HTTP create surface sets them") — so the **membership→storage-`humans` projection is not populating the table**, leaving nothing for the heal/commitments to join against. Household formation only `1/3 affirmed` (matthew); REA-commitments/custody stage did NOT run (`CONTENT_BLOB_HASH unset`).
2. **Self-heal session minted for matthew but NOT adam/jessica.** Both adam (explicit manifest) and jessica (consolidated template) returned `/auth/me → HTTP 401` "pod has no local session yet" → their provide-rows SKIPPED. The self-heal env deployed but only matthew ended with a session — non-uniform session minting (matthew may have had prior session state; or the self-heal only fires on a specific node role/condition).

## Morning options (operator/design — outside autonomous deploy-shepherd scope)
- **Inspect matthew's content.db / projection state directly** (cluster) — why is `humans` empty and matthew's seeded provide-row not readable, given a persistent PVC + a successful seed + caughtUp projector? This is the load-bearing unknown.
- **rust-architect: the membership→storage-`humans` projection** — does household-formation affirmation emit a membership signal that projects into the storage `humans` table? Trace `on_membership_projected`.
- **Non-matthew self-heal session** — why adam/jessica got no session despite the env (their pod role / remote-shem / the self-heal gate). See `genesis_self_heal.rs`.
- **RESET_STORAGE / manual storage reset** (user-offered) — may clear inconsistent state but is unlikely to fix the projection gap on its own.
- **REA-commitments/custody stage** — gate on `CONTENT_BLOB_HASH` left it unrun; confirm whether the card needs it.

## Evidence
shift journal `.claude/shifts/2026-06-19-ci-shepherd-card-deploy.journal.md` (iterations 4–7); genesis `elohim-genesis/dev` #1176 seed log (per-step quotes); `/health` + `/db/humans` + `/peers/delivery` + card reads on `alpha.elohim.host` 2026-06-19 ~06:00 UTC.

---

## RESOLVED-DIAGNOSIS (2026-06-19, operator cluster probes U1–U4) — the "projection-empty" framing above is SUPERSEDED

The morning probe (operator `kubectl cp` of each pod's `content.db`, read-only) **refutes this
card's load-bearing hypothesis** ("the membership→storage-`humans` projection isn't populating the
table"). The table is **not** the matthew blocker:

- **U2:** matthew's `humans` row is **fully healed** — `agent_pub_key` non-NULL (= the `uhCAk…`
  that equals his provide-row `provider`), `household_id = household-dowell` non-NULL,
  `h_app_id=imagodei`. `genesis_self_heal` worked. The empty `/db/humans` read was a **read-scope
  artifact** (`register` writes `imagodei`; `GET /db/humans` hard-forces `lamad` — `http.rs:3637`),
  not an empty table — it **misled this card's diagnosis.**
- **U1 (the real card-zero):** matthew's provide row is present/`active`/matthew-attributed/`lamad`
  — the **sole** deviation is `rea_commitments.resource_classified_as = ["content:commons"]` (JSON
  list) vs the card's scalar `.eq("content:commons")`. An action-polymorphic serialization bug, not
  an identity/projection gap.
- **U3 (the count=2 gap, this card's issue #2 — CONFIRMED):** adam's `humans` table = **0 rows**.
  The seed single-targets matthew via doorway; adam's pod never got its own `register` INSERT → his
  self-heal `NotFound`-skips → no session → 401. Fix = per-pod registration (NOT a projection gap).
- **U4:** content reach `commons` both sides — the U1 mismatch is format-only.

**Root cause + fix now owned by:** the `2026-06-13-non-commons-provide-commitments-design` **§11
addendum** (DECIDED: uniform JSON-list classification + typed accessor — Option A) and the
**`2026-06-19-resilience-card-lighting-plan`** (Sprint 1 lights matthew with no reseed; Sprint 2 the
per-pod work for adam). The `/db/humans` read-scope artifact + the steward-gate formation-1/3
circularity are captured as complementary items (home: `qahal-collective-cid-formation-projection-
gap.md`). **status stays `open`** (work not landed) but the diagnosis is settled — do not re-chase a
"projection isn't populating humans" cause; matthew's row is healed.

---

## UPDATE 2026-06-27 — the imagodei-write / lamad-read SCOPE leg is RESOLVED (plan: humans-projection-scope-reconciliation)

The **scope split** named in U2 (production writes `humans` under `h_app_id="imagodei"`; the
household-join readers filtered under the operating content scope `"lamad"`, so every join silently
emptied) is **reconciled** by
`genesis/docs/superpowers/plans/2026-06-27-humans-projection-scope-reconciliation-plan.md` (landed on
`feat/frontend-eyes-sprint`):

- **Single source of truth:** `elohim-storage` now has `pub const HUMANS_HAPP_ID: &str = "imagodei"`
  (`db/context.rs`), re-exported as `crate::db::HUMANS_HAPP_ID`. Every humans-projection reader filters
  by it; the two production writers (`api/identity.rs::register_human`,
  `services/genesis_self_heal.rs`) reference it (flip-both-together drift guard).
- **Readers fixed (4):** the ingest peer-selector (`services/peer_selection.rs`), salvage placement
  (`services/salvage_commitment_author.rs` — also retired the threaded `h_app_id` param from
  `run_salvage_pass`/`build_salvage_candidates`), the doorway public-humans cache
  (`db/cache_queries.rs::list_cacheable_humans`), and `GET /db/humans`
  (`http.rs::handle_list_humans` — the exact read-scope artifact U2 flagged as misleading this card).
- **Monotonic-safe:** every affected read returned empty before (humans are imagodei, the filter was
  lamad); flipping to imagodei is empty→correct only — no production data is mis-selected (writers
  already write imagodei).

**Two gates remain OPEN — `status` stays `open`; nobody may re-assert "diversity works in production"
until both clear:**

1. **NULL `agent_pub_key` population** (U3) — the DHT humans-replayer is a stub; only
   `genesis_self_heal` fills the self pod. Other pods (adam, …) need per-pod registration.
   Owner: `2026-06-19-resilience-card-lighting-plan` Sprint 2 / the humans-replayer arc.
2. **Transport-id vs `agent_cid` namespace** — `self_cid` / `salvage_capacity.agent_cid` may be a
   libp2p/iroh transport id unless `SELF_CID` pins the agent key. Owner: the **blocked**
   `2026-06-15-coherent-transport-identity-resolver-design`, or `SELF_CID` per deployment.

The U1 JSON-list serialization bug is unrelated to this plan (owned by
`2026-06-13-non-commons-provide-commitments-design` §11).

---

## UPDATE 2026-07-31 — a THIRD cause found and FIXED: the fossil-key custody strand

Live RCA on both alpha doorways (operator report: "resiliency cards aren't converging").
The headline correction: **`stewardingCollectives: 0` is NOT caused by the NULL
`agent_pub_key` gate above.** Gate 1 is real but is not what darkens the card.

### Live evidence (2026-07-31, read-only)

| Probe | doorway-alpha (matthew) | elohim.host (adam) |
|---|---|---|
| `/api/v1/resilience/elohim-host-landing/household` | `stewardingCollectives: 0`, `distributionState: "measured"` | `stewardingCollectives: 0`, `"measured"` |
| holder `shard_locations.peer_id` | `uhCAkYi1CWDUW9YgQ…` | `uhCAk_hiBZZedIfYqp…` |
| that key ∈ any `humans.agent_pub_key`? | **no** | **no** |
| that key ∈ live conductor agents (`/db/p2p/conductor-diagnostics`)? | **no** | **no** |
| `GET /blob/{that shard hash}` | **200, 9 994 802 B** | **200, 9 990 975 B** |

So both nodes **demonstrably hold the bytes** and record a holder row — under an
`agent_cid` that is neither live nor any human's. The one human whose key IS live
(matthew on A, adam on B — confirmed against conductor-diagnostics) has **no** holder
row. The join `shard_locations.peer_id == humans.agent_pub_key` therefore matches
nothing, and the card reports zero stewards for content the node is serving.

The key is a **fossil**: a non-prod DNA reinstall mints a new `AgentPubKey` per pod,
and `humans` was healed forward (`genesis_self_heal`) while the `shard_locations` row
was left behind under the dead key.

### Root cause (code-verified) — the repair path was structurally unreachable

`reconcile::custody::manifest_backfill_pass` is the producer of self-held custody
evidence. Its pre-filter admits a blob as a *candidate* only when the blob has **no**
manifest; an already-manifested blob is counted `skipped_existing` and `continue`d.
`record_self_held_shard` was reachable **only** from inside the candidate loop. So:

- once a manifest exists, the node never re-claims that blob again — forever;
- a manifest written by the ingest path (`put_blob_bytes`) records no self-held row
  at all, and can never be repaired later;
- after a re-key the node keeps holding the bytes and keeps *not* claiming them under
  its live identity.

The pass's own doc comment encoded the stale assumption: *"a blob that gained a
manifest (and self-held row) on a prior pass is no longer a candidate on the next"* —
true only if the node's `agent_cid` never changes.

### Fix (landed) — the re-claim arm

`elohim/elohim-storage/src/reconcile/custody.rs`: the pre-filter's existing-manifest
arm now re-asserts self-held custody under the node's **current** `agent_cid`.
Honesty-gated three ways — `encoding == "none"` (single shard IS the blob), the
manifest's shard hash equals the blob hash being iterated, and the hash came from the
local store listing (the same listing-based evidence `BlobStoreSnapshot` supplies to
the main reconcile pass). `record_self_held_shard` independently refuses any non-
`agent_cid` identity, so a transport id can never reach the join-key column. Steady
state is a true no-op (skips when the live-key claim already exists); the fossil row
is left inert rather than deleted — moving it is `membership_identity_reconcile`'s
rekey cascade, not this arm's job. Re-claim work is capped at `batch_cap` per pass.

New counter `ManifestBackfillOutcome.self_held_reclaimed` (internal struct, no wire
shape changed) separates "re-attributed old custody" from "manifested new bytes" in
the pass log. Regression tests: `reclaims_self_held_custody_under_a_new_agent_key`
(the re-key scenario end-to-end, plus idempotence) and
`reclaim_refuses_a_transport_identity`.

`self_agent_cid` resolves on a live pod — `identity::resolve_agent_pubkey` prefers the
conductor cell key over the transport id — so the arm fires on the next edge deploy
without any operator action.

### Verdicts on the other two hypotheses tested in this pass

- **NULL `agent_pub_key` (gate 1 above) — real, but NOT the card-zero cause.** On
  doorway-alpha 7 of 14 humans have `household_id` set with `agent_pub_key` NULL
  (caleb, daniel, emma, ezra, nancy, pete, terrance) and 6 more carry fossil keys;
  only matthew's is live. Fixing all of them would still not have lit the card,
  because the holder side pointed at a key belonging to nobody. Gate 1 stays open and
  keeps its owner (per-pod registration / humans-replayer) — it caps *how many*
  households can ever be counted, not whether the count is zero.
- **Omnibar endpoint mismatch — REFUTED.** Suspected that the SSR omnibar client calls
  `/api/v1/resilience/{slug}` without `/household` (a different handler branch
  returning `ResilienceView`, which has no `stewardingCollectives`). Both the tree and
  the **deployed** asset (`/chrome/omni-element.<sha>.js`, byte-identical across both
  doorways) build `var base = '/api/v1/resilience/' + encodeURIComponent(slug)` and
  then `fetch(base + '/household', opts)`, falling back to base only on failure. The
  suffix is appended at the call, not the assignment. No fix needed. (The base-shape
  fallback cannot populate the card — `applyResilience` requires `protectionStatus` /
  `feltStatus`, which the base shape lacks — so it degrades to the honest `unmatched`
  marker, never to a wrong number.)

### Residual, deliberately NOT fixed here

The two doorways hold **different bytes** for the same slug (`sha256-30011cff…` vs
`sha256-de394363…`, 9 994 802 B vs 9 990 975 B) and render pages of different size.
Once the re-claim arm lands, each doorway will honestly report its own single steward
— non-zero, but still not the SAME footprint. That divergence is a
replication/declared-head concern, not a resilience-projection one (see
`feedback_reach_head_replication_distinct_planes`); it needs its own item.

Also observed: `GET /` on both doorways intermittently sheds `503
{"status":"catching-up"}` and succeeds on retry — the known projector catch-up flap
(`alpha-a-projector-chronic-catchup-flap.md`).

### The single probe that proves convergence (post-deploy)

After an edge deploy carrying this fix, on **both** doorways:

```
curl -s https://doorway-alpha.elohim.host/api/v1/resilience/elohim-host-landing/household | jq '.stewardingCollectives, .details.stewardingCollectives'
curl -s https://elohim.host/api/v1/resilience/elohim-host-landing/household        | jq '.stewardingCollectives, .details.stewardingCollectives'
```

Expect `stewardingCollectives >= 1` with a named collective (`household-dowell` on A,
`household-adam` on B) within one `MANIFEST_BACKFILL_SECONDS` tick (default 300s) of
pod start. Still `0` after two ticks ⇒ `self_agent_cid` is not resolving on that pod —
check the pass log line for `self_held_reclaimed` and the resolved cell key, not this
arm.
