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
