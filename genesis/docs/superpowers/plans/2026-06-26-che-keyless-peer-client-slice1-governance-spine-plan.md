---
title: "Che keyless peer-client — Slice 1: the delegates-compute op-gate governance spine (Wave-1.3 unblock)"
id: che-keyless-peer-client-slice1-governance-spine-plan
status: In-progress
class: protocol-canonical
landing_state: |
  Offline spine landed, live legs held (verified against the tree 2026-08-05 during the worktree
  commit sweep). The delegates-compute op-gate primitive is in-tree at
  elohim/elohim-storage/src/services/bounds_validator.rs:360, with commitment_fetcher.rs and
  rea_commitment_service.rs alongside it. The live-mesh legs remain held because every doorway
  deploy runs DEV_MODE=true, so there is no valid Che-facing enforce node. The 40 unticked steps
  below understate progress — re-verify each against the tree before picking work.
domain: D8
sprint: substrate-validation   # durability/dataplane-actuation rung; de-risks roadmap Sprint 2 (grandma-standard recovery) by lighting the dataplane through a governed path
topic: [doorway, op-gate, delegates-compute, bounds-validator, capability-dispatch, keyless-peer-client, eclipse-che, distribute-shards, stewarding, dogfooding, T4-projection, substrate-validation]
refines:
  - genesis/docs/superpowers/specs/2026-06-26-che-keyless-governed-peer-client-design.md
  - genesis/docs/superpowers/plans/2026-06-26-live-distribute-shards-household-observation-plan.md
cites:
  - che-keyless-governed-peer-client-design | the spec this plan implements (Slice 1, §10) under its §14 hardening | path: genesis/docs/superpowers/specs/2026-06-26-che-keyless-governed-peer-client-design.md
  - live-distribute-shards-household-observation-plan | COMPOSE-WITH — the Wave-1.3 driving loop + blob-gated trigger + Phase-0 observe-first discipline + resilience observation; this plan adds the governed-authorization front-half its driving loop assumed open | path: genesis/docs/superpowers/plans/2026-06-26-live-distribute-shards-household-observation-plan.md
  - admin-key-lifecycle-dev-to-production | stage-3 commitment-backed delegation — the seeder graduates from the omnipotent admin key to a scoped delegates-compute grant by the same mechanism | path: genesis/docs/superpowers/specs/2026-06-03-admin-key-lifecycle-dev-to-production.md
  - rea-compute-commitment-primitive | the delegates-compute primitive this op-gate enforces (one substrate primitive, a fourth costume) | path: genesis/docs/architecture/rea-compute-commitment-primitive.md
  - stewardship-over-sovereignty | the identity-ontology floor — custody is community-grounded stewardship; no self-sovereign tier | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - elohim/elohim-storage/src/services/bounds_validator.rs
  - elohim/elohim-storage/src/services/commitment_fetcher.rs
  - elohim/elohim-storage/src/services/rea_commitment_service.rs
  - elohim/elohim-storage/src/api/rea_commitments.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json
  - genesis/seeder/src/seed-commitments.ts
  - doorway/doorway-service/src/server/http.rs
  - doorway/doorway-service/src/routes/storage_proxy.rs
  - doorway/doorway-service/src/routes/auth_routes.rs
# Mixed-env plan (CLAUDE.md scope convention): NO doc-level requires_env. Every task is testable on the
# household-nodes class — a local hc:start:seed M/J/J stack (conductor + storage + doorway) or the live
# matthew pod via the deployed doorway. Nothing here needs shem / alpha-cluster-6peer / harbor.
---

# Che keyless peer-client — Slice 1: the delegates-compute op-gate governance spine

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans, task-by-task, two-stage review. Steps use checkbox (`- [ ]`) syntax.
> This is a **BUILD plan** — the spec (`§14`) is explicit that **none of this enforcement exists today**
> (the doorway forwards the `Authorization` header without verify/capability/commitment work; `distribute_shards`
> is an ungated side-effect). The work is to **build the governance spine** that lets a keyless Eclipse Che
> drive `distribute_shards` on the live mesh through a real, revocable, **bounded** `delegates-compute`
> contract. **The p2p-design-gate is ALREADY SATISFIED** (spec §7 — no new DHT entity, no new coordinator;
> the commitment is existing Notarized-A, the op-gate is Operational-C); do **not** re-run it.

> **HARDENED 2026-06-26 by an 8-CRITICAL adversarial review** (`wf_72bd3c26-65d`, 5 lenses × verify ×
> synthesis). Verdict: *architecture sound, no structural rethink; apply the CRITICALs and it is executable.*
> The corrections are folded in below and flagged inline as `[Cn]`/`[In]`. The headline correction (**C1**):
> the seed and the authorizer are on **two different tables** — `POST /api/v1/commitments` writes
> `rea_commitments` (NULL anchor, gate fields dropped), but the op-gate reads `mishpat_commitments` (written
> only by the post-commit projection — **no client HTTP write path**). Slice 1 therefore seeds through a
> **new gated, ingress-only storage endpoint that writes `mishpat_commitments` directly**, never through
> `/api/v1/commitments`.

**Goal:** Matthew, in Eclipse Che holding only a verified JWT (no key), POSTs blob-backed content through
the deployed doorway; a **doorway pre-dispatch op-gate** authorizes the request against a **seeded, bounded
`delegates-compute` commitment in `mishpat_commitments`** (per-request, fail-closed, `performer == recipient`-
bound from the verified credential); the custodian conductor (matthew pod) runs `distribute_shards` on the
live mesh; the resilience card reads `stewardingCollectives > 0` — and **revoking the commitment denies the
next request**.

**Architecture:** Authority flows **up** from the human's seeded compute-contract → enforced at the doorway
(the verified-credential trust boundary) → consulted against the `mishpat_commitments` projection in storage
(where `bounds_validator` + the projection live). The doorway has no DB, so the op-gate **derives the
performer from the verified JWT** and calls a **new storage authorize-operation endpoint** that reuses the
existing 7-check `bounds_validator` through an `EventForValidation`-shaped operation adapter. The commitment
must be in `mishpat_commitments` with a **non-NULL `dht_anchor_hash`** because the fail-closed fetcher
(`commitment_fetcher.rs:176`) the spec mandates rejects un-notarized rows — and because **no client route
writes that table**, Slice 1 seeds it through a flag-gated, ingress-only dev endpoint (the `seed_shard_manifest`
honesty model: the flag IS the dev/prod boundary).

**Tech Stack:** doorway-service (Rust, hyper, jsonwebtoken HS256), elohim-storage (Rust, diesel/SQLite,
`bounds_validator`/`ProjectionCommitmentFetcher`, `mishpat_commitments`), genesis/seeder (TypeScript/tsx),
`delegates-compute.schema.json`, a2o (Cucumber/TypeScript), `pnpm look` / `GET /api/v1/resilience/{cid}`.

## Global Constraints

Copied verbatim from spec §14 (binding) + the composed Wave-1.3 plan + repo policy + the review hardening.
Every task's requirements implicitly include this section.

- **Per-request re-check, fail-closed.** The op-gate consults `mishpat_commitments` on **every**
  `POST /db/content` and `/db/content/bulk` (not at issuance) and DENIES on conductor-unreachable /
  un-notarized (NULL anchor) / not-found / out-of-bounds. The JWT is **not** the revocation surface
  (`handle_logout` no-op; `is_active` wired only to `/auth/me` and fails *open*) — revocation lives in the
  per-request **commitment** consult.
- **Bind `performer == recipient` from the VERIFIED credential — in the op-gate ONLY.** `[C3]` Do **NOT**
  add this check to the shared `bounds_validator`: it is also used by the live provide-reconcile loop
  (`conductor_commitment_author::author_commons` emits `performer = provider` against `replicates-commons`
  commitments that have **no** `recipient` — the check would fail every tick and darken the very card this
  slice lights). Enforce the bind **only** inside `authorize_operation` (Task 3), where the lookup already
  filters `recipient == performer` structurally, plus one explicit guard.
- **Derive the performer from a Phase-0-VERIFIED JWT claim.** `[C8]` The performer `agent_cid` is the JWT
  claim whose value equals the seeded `commitment.recipient` — **resolved empirically in Phase 0** (do NOT
  assume `agent_pub_key`; `resolve_agent_cid_from_request` returns `claims.human_id` at `http.rs:1041`, and
  `agent_pub_key` is documented as a hex string in a *different* namespace). Bind the op-gate filter, the
  seed `recipient`, and `MATTHEW_AGENT_CID` to **that one resolved claim**. Never the client-set
  `X-Agent-Cid` header (trusted verbatim at storage `account.rs:1000`).
- **Seed bounded (minimum-bounds guard).** `bounds_validator` defaults `rate_per_hour`/`rotation_ttl_days`
  to `u64::MAX` (`:269`, `:296`). The seed factory MUST reject `epr_scope:["*"]` unless **both** `rate_per_hour`
  and `rotation_ttl_days` are explicit finite values, and MUST require `reach_elevation_acknowledged:true`
  whenever `reach_ceiling ∉ {commons, community}` (`[C4]` — an allowlist, not a rank ladder).
- **Compose capability AND scope (conjunction).** The route's required capability (`orchestrate-node`, a
  **fixed constant** for `/db/content` in Slice 1 — not a route→capability lookup) and the commitment's
  `scope` must agree; the lookup pushes `scope == capability` into SQL `[I1]`.
- **The op-gate is a DOORWAY-LAYER control only.** `[I9]` Storage's own `POST /db/content`/`/bulk` remain
  ungated and trust `X-Agent-Cid` verbatim — a caller reaching storage **directly** bypasses the gate. This
  is **accepted for Slice 1 under ingress isolation** (storage is not publicly routable; only the doorway
  reaches it) and MUST NOT be read as substrate enforcement. Substrate-side enforcement is later work.
- **Normal routes ONLY.** Slice 1 gates `POST /db/content` and `POST /db/content/bulk` (both call
  `distribute_shards` — `http.rs:4301`, `:4487`) `[C5]`. Do **NOT** touch `/admin/*` routes (ingress-isolated
  today); capability-scoping them is later work that ADDS the gate while RETAINING ingress isolation.
- **Rate-limit (bounds check 6) is INERT on the op-gate path.** `[I6]` `bounds_validator`'s rate check
  counts `economic_events`, which `authorize_operation` never emits — it cannot trip here (it still enforces
  on the real emit path). It is **not** a Slice-1 enforcement surface; a real operation-counter is backlogged.
- **`distribute_shards` is the node's own side-effect.** The op-gate authorizes the client *request* at the
  doorway; once authorized+forwarded, `distribute_shards` runs as the node's own authorized spawn. In-flight
  fan-out completes; revocation denies the **next** request.
- **No `dev_mode` on the Che-facing deploy.** `auth_routes.rs:1573` returns `Admin` for any creds when Mongo
  is absent (and mints `uhCAk-dev-mode-agent-key` ≠ the real performer). The Che-facing deploy refuses to
  boot with `dev_mode` AND `gate=enforce` `[I4]`, and refuses `enforce` unless `jwt_secret` is present and
  ≥32 chars `[I5]` (HS256 + a weak/known secret forges any performer).
- **Carrier honesty (Slice-1 scope).** The Slice-1 credential carrier is the existing **HS256 JWT** — NOT
  web2-provenance-verifiable. §0's "verifiable by web2" is a **Slice-2** property (the DHT attestation). This
  plan does **not** mint `attestation:device-client-authorization` (Slice 2) and does **not** extend JWT
  claims; the *authorization* becomes commitment-backed **because the op-gate consults the commitment
  per-request**.
- **Seed honesty.** The Slice-1 commitment is written by a **flag-gated dev endpoint** (`ALLOW_SEED_DELEGATES_COMPUTE=1`,
  403 by default) with a **synthesized** `dht_anchor_hash` + a `dev-seed` provenance marker in `bounds_json` —
  the flag is the dev/prod boundary (the `seed_shard_manifest` model). A genuinely DHT-notarized
  `delegates-compute` (the conductor mishpat-role commit) is the honest follow-up (D1=A), not Slice 1. Also
  assert `ALLOW_SEED_SHARD_MANIFEST` is **unset** on the Che-facing + local-acceptance deploys so the card
  lights through the governed `distribute_shards` push (`status="announced"`), not a `seed`-claim.
- **Blob-gated trigger (from Wave-1.3).** `distribute_shards` fires on a content POST only when `blob_hash`
  is present (`http.rs:4301`) AND storage is built `--features p2p` `[I3]` (the spawn is `#[cfg(feature="p2p")]`).
- **Join key is `agent_cid`.** Never raw-string-compare across identity namespaces.
- **Commit-only; integrator pushes.** Shared worktree — selective staging. Never `kubectl`. elohim-storage
  keeps ambient `RUSTFLAGS` (getrandom custom); doorway/steward use `RUSTFLAGS=""`. `CARGO_TARGET_DIR` = pool
  slot (fall back `/tmp/<name>-target` on fingerprint ENOENT); plain `cargo test`; never pipe a gate's exit code.

---

## Provenance & framing

Surfaced 2026-06-26 from the operator's directive to make Eclipse Che a keyless, governed dogfooding
peer-client (spec approved + §14-hardened, then this plan reviewed by an 8-CRITICAL adversarial workflow the
same day). This plan implements **Slice 1** — "B's governance spine (the Wave-1.3 unblock)" (spec §10).

**Composes from (born-linked):**
- **Lexical floor** (`spec-coherence-index`, 8 matches): the Che spec; `admin-key-lifecycle-dev-to-production`
  (the seeder-graduation compose-home); the CANONICAL `compute-commitment-substrate-floor`; the
  `jenkins-seed-bearer-gate` plan (the authorization-gate pattern).
- **COMPOSE-WITH (do not fork):** `live-distribute-shards-household-observation-plan` (Wave 1.3) — its
  Phase-0-observe-first discipline, blob-gated trigger, `agent_pub_key` junction note, and resilience-card
  observation are the **back half** of this plan's driving loop. **Task 5 here = Wave-1.3 Task 0 Steps 4–5 +
  Task 1 with the gate in `enforce`.** Run them together; do not duplicate the observation scenarios.
- **Semantic lens (MemPalace): STALE → degraded to lexical-only** (last mined 2026-06-11; this work born
  2026-06-26). §4.4 guard.
- **MAP-PATH:** Domain **D8** (Web2 Projection & Doorway, the Track-4 boundary). Composes with **D9** (the
  REA commitment) and unblocks **D5** (`distribute_shards` actuation). No Gap-Ledger row closed; adds a new
  D8 capability. Reading order: doorway → storage.
- **ROADMAP:** `substrate-validation` rung; de-risks roadmap Sprint 2 (grandma-standard recovery). BUILD
  plan (OPEN gaps), not verify-track, not BLOCKED-BY-ENV.

## Ground truth (verified file:line 2026-06-26; corrected by review `wf_72bd3c26-65d` — do NOT re-derive)

| Claim | Verdict | Evidence |
|---|---|---|
| **Two-table split (THE crux, `[C1]`)** | TRUE | `POST /api/v1/commitments` → `handle_create` → `ReaCommitmentService::create`; `delegates-compute ∉ CONDUCTOR_SOFT_ACTIONS (=["custody-blob"])` (`rea_commitment_service.rs:142-146`) → `create_via_diesel` → **`rea_commitments`** (NULL `dht_anchor_hash`; `scope`/`bounds`/`valid_from`/`valid_until` are **not** columns of `CreateReaCommitmentInputView`/`NewReaCommitment` → silently dropped). The op-gate reads **`mishpat_commitments`** (`commitment_fetcher.rs:214`), written **only** by the `CommitmentCommitted` post-commit projection (`signals.rs:903`) — **no client HTTP write path.** ⟹ seed `mishpat_commitments` directly via a gated endpoint. |
| Doorway dispatch seam | TRUE | `server/http.rs ~3726-3765` (`Disposition::StorageProxy { endpoint }`); agent resolution `~:3734`; forward `~:3754`. **The path binds as `p`; the storage URL is `endpoint`** `[C5]` — gate on `p`, not `endpoint`. Insert AFTER any `/blob/` early-return. |
| Doorway verifies the JWT inline; which claim is the agent_cid is **UNSEALED** | TRUE (claim TBD) | `resolve_agent_cid_from_request` (`http.rs:1023-1042`) returns **`claims.human_id`** as the forwarded agent_cid (`:1041`); `Claims` (`jwt.rs:71-114`) also carry `agent_pub_key` (documented hex, different namespace). **Phase 0 resolves which claim == the seeded recipient** `[C8]`; bind everything to it. |
| `bounds_validator::validate` signature + input | TRUE | `bounds_validator.rs:107`: `async fn validate<F: CommitmentFetcher, R: RateHistory>(event: &EventForValidation, fetcher: &F, rate_history: &R) -> Result<BoundsChecksView, BoundsViolation>`. `EventForValidation { action, performer, bounded_by, target_epr_id, reach, signed_at }` (`:68-81`). |
| `BoundsViolation` is a **struct**, variants live in `ViolationKind` (separate `elohim-views` crate, ts-rs) | TRUE `[C2]` | `struct BoundsViolation { kind: ViolationKind, commitment_cid, summary, checks }`; `ViolationKind` in `elohim-views/src/bounds.rs` (`#[derive(TS)]`). ⟹ Slice 1 adds **no** new variant (the performer bind is the op-gate service's own deny, not a `BoundsViolation`). |
| Fail-closed on un-notarized | TRUE | `record_from_row` (`commitment_fetcher.rs:176`) → `NotarizedRequired` when `dht_anchor_hash IS NULL`; infra failure → `ConductorUnreachable` (503). |
| Shared validator used by the **live provide loop** | TRUE `[C3]` | `economic_event_emit_service::emit` builds `EventForValidation { performer: input.provider, … }`; `author_commons` emits against `replicates-commons` (no `recipient`). A performer==recipient check inside `bounds_validator` breaks this. |
| `mishpat_commitments` columns + write helper | TRUE | `models.rs:3571-3586` (cid, action, scope, provider, recipient, bounds_json, valid_from, valid_until, revoked_at, state, dht_anchor_hash, created_at, updated_at); upsert via `db::mishpat_commitments::upsert_with_anchor` / `get_by_cid` (verify exact name Phase 0). |
| `delegates-compute.schema.json` fields | TRUE | required: action, scope, provider, recipient, bounds{epr_scope[],reach_ceiling,rate_per_hour≥1,rotation_ttl_days≥1,reach_elevation_acknowledged?}, valid_from, valid_until; `reach_elevation_acknowledged:true` required when `reach_ceiling ∉ {commons,community}`. |
| `distribute_shards` is a blob-gated, `#[cfg(feature="p2p")]` side-effect of `POST /db/content` AND `/bulk` | TRUE | single `http.rs:4288-4319` (`:4301` blob gate); bulk `http.rs:4487` spawns the same fan `[C5]`. |
| `CommitmentClient` location | TRUE `[I7]` | defined `class CommitmentClient` at `seed-commitments.ts:268` (NOT `doorway-client.ts`), **not exported** — Slice 1's seed factory targets a *new gated storage endpoint*, not `/api/v1/commitments`, so it uses a plain authed client; if reused, export it first. |
| `/auth/login` request shape | TRUE `[I2]` | `LoginRequest.password` is **non-optional**; an empty password 400s before any token. The Task-5 curl must send a non-empty password and target a real account (or the deployed matthew pod). |

**P2P-design-gate status: ALREADY SATISFIED (spec §7).** No new DHT entity / coordinator / sync dialect.
The commitment is existing **Notarized-A** (`Mishpat::Commitment`); the gated seed writes its **projection**
(Operational-C); the op-gate is **Operational-C**. Do not re-run the gate.

---

## Open Decisions (resolve at pre-flight review; defaults stand if silent)

| # | Decision | Recommendation |
|---|---|---|
| **D1** | **How is the `delegates-compute` commitment in `mishpat_commitments` produced?** `[C1]` **(A) Honestly notarized** — fund the mishpat-role conductor commit so the `CommitmentCommitted` projection fires a real DHT-anchored row (stage-2; NOT reachable through any create route today). **(B) Gated dev-seed** — a flag-gated, ingress-only storage endpoint inserts the row directly via `upsert_with_anchor` with a **synthesized** anchor + `dev-seed` provenance marker. | **Slice 1 = B** (the `seed_shard_manifest` honesty model — the flag is the dev/prod boundary; production nodes 403 the endpoint and hold no seeded grants). **A is the honest follow-up**, backlogged. The op-gate stays fail-closed either way; B only lets a *flag-on* node mint an authorizable row. |
| **D2** | **Op-gate enforcement default.** | Three-mode env flag `DELEGATES_COMPUTE_OP_GATE = off \| observe \| enforce`, default **off** (today's passthrough). `observe` logs would-deny. The **Che-facing alpha deploy sets `enforce`** and refuses to boot otherwise `[I4]`. |
| **D6** | **Auth model of the new storage `authorize-operation` endpoint** `[C6]` — there is **no** "doorway service credential" (it doesn't exist; `auth_required` is decorative — `classify_dispatch` never reads it). Left as the plan first wrote it, the endpoint is an **unauthenticated verdict oracle** (DB lookup + full `bounds_validator` per call = DoS amplifier) if it reaches the public manifest. | **The doorway forwards the user's original `Authorization` header** on the authorize call (it already holds it) and passes the doorway-VERIFIED performer in the body; the endpoint stays `.auth_required()` (any valid token) and is **NOT** registered in the public doorway proxy manifest (ingress-isolated, read-only, no write power). **Residual (named):** storage trusts the body-supplied performer the same way it trusts `X-Agent-Cid` — acceptable under ingress isolation, hardened in a later slice. |

---

## Phase 0 — verify the seams before building (FIRST move; gates D1, the identity claim, and the build features)

### Task 0: Stand up the mesh, resolve the identity claim, confirm the two-table reality

**Files:** none (observation). **Produces:** (a) the verified performer JWT claim `[C8]`; (b) confirmation
`mishpat_commitments` has no client write path and `upsert_with_anchor` exists `[C1]`; (c) confirmation
storage is `--features p2p` `[I3]`; (d) the four confirmed seam line numbers. Composes with Wave-1.3 Task 0
(reuse its Steps 1–3 to stand up the M/J/J stack + baseline the card).

- [ ] **Step 1: Stand up the seeded local stack** (from `app/elohim-app/`): `pnpm run hc:start:seed`.
  Confirm storage was built `--features p2p` (the `distribute_shards` spawn is `#[cfg(feature="p2p")]` `[I3]`):
  `cargo tree -e features` on `elohim-storage` or grep the build invocation. Record.

- [ ] **Step 2: RESOLVE the performer claim `[C8]`.** Log in and decode the JWT; find which claim equals the
  value the seeder will use as `recipient`:

```bash
TOKEN=$(curl -s -X POST localhost:8888/auth/login -H 'content-type: application/json' \
  -d '{"identifier":"matthew","password":"<dev-password>"}' | python3 -c "import sys,json;print(json.load(sys.stdin)['token'])")
python3 - "$TOKEN" <<'PY'
import sys,base64,json
p=sys.argv[1].split('.')[1]; p+='='*(-len(p)%4)
print(json.dumps(json.loads(base64.urlsafe_b64decode(p)), indent=2))
PY
# Compare each claim against the agent_cid the seeder/humans table uses for matthew:
sqlite3 <storage content.db> "select id, agent_pub_key from humans where id like '%matthew%';"
```

  Record **`<PERFORMER_CLAIM>`** = the JWT claim whose value equals matthew's `agent_pub_key` (the
  `mishpat_commitments.recipient` namespace). Candidates: `human_id` (what the doorway forwards as
  `X-Agent-Cid`) vs `agent_pub_key`. **Bind the op-gate filter, the seed `recipient`, and `MATTHEW_AGENT_CID`
  to this one value.** If NO claim matches, STOP — the gate would deny-all; reconcile the seeder/identity
  namespace first.

- [ ] **Step 3: Confirm the two-table reality `[C1]`.** Verify `delegates-compute ∉ CONDUCTOR_SOFT_ACTIONS`
  (`rea_commitment_service.rs:142-146`), that `mishpat_commitments` has no HTTP create route, and that the
  mishpat write helper exists:

```bash
grep -n "CONDUCTOR_SOFT_ACTIONS" elohim/elohim-storage/src/services/rea_commitment_service.rs
grep -rn "fn upsert_with_anchor\|fn upsert\|fn insert" elohim/elohim-storage/src/db/mishpat_commitments.rs
grep -rn "mishpat_commitments" elohim/elohim-storage/src/http.rs   # confirm: no client POST/PATCH write route
```

  Record the exact mishpat write fn name (Task 2 uses it). Confirm only the projection (`signals.rs:903`)
  writes the table.

- [ ] **Step 4: Confirm the doorway insertion seam + the bounds API.** Read `server/http.rs ~3726-3765`
  (confirm the path binds as `p`, the storage URL as `endpoint` `[C5]`), `bounds_validator.rs:107`/`:68-81`,
  `commitment_fetcher.rs:176`. Record current line numbers (the codebase moves).

- [ ] **Step 5: Commit the Phase-0 evidence** — append `<PERFORMER_CLAIM>`, the mishpat write fn name, the
  `--features p2p` confirmation, and the four seam line numbers to `.claude/data/dev-intent.jsonl`.

---

## Task 1: `seed-delegates-compute` factory (bounded) — targets the gated storage endpoint

**Files:**
- Create: `genesis/seeder/src/seed-delegates-compute.ts`
- Modify: `genesis/seeder/package.json` (add `seed:delegates` scripts)
- Test: `genesis/seeder/src/seed-delegates-compute.test.ts` (the minimum-bounds guard is the asserted unit)

**Interfaces:**
- Consumes: the **new gated storage endpoint** `POST /admin/seed/delegates-compute` (Task 2), the suspended-
  persona guard + `deployments.json` pattern from `seed-commitments.ts`, `delegates-compute.schema.json`.
  (Does **NOT** use `POST /api/v1/commitments` — that lands in the wrong table `[C1]`.)
- Produces: `seedDelegatesComputeCommitments(client, pairs)`, `buildDelegatesComputeBody(pair)`,
  `assertBoundedMinimum(bounds)`, `defaultDelegatesComputePairs()`, the `DelegatesComputePair` type. The
  Matthew→Che self-contract (`provider == recipient == MATTHEW_AGENT_CID`, the Phase-0 `<PERFORMER_CLAIM>`
  value) is the default pair.

- [ ] **Step 1: Write the failing minimum-bounds guard test** (`[C4]` — allowlist, not rank ladder):

```typescript
import { describe, it, expect } from 'vitest';
import { assertBoundedMinimum } from './seed-delegates-compute';

describe('assertBoundedMinimum (spec §14 minimum-bounds guard)', () => {
  it('rejects epr_scope ["*"] with omitted rate', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['*'], reach_ceiling: 'commons', rotation_ttl_days: 30 } as any))
      .toThrow(/rate_per_hour/i);
  });
  it('rejects epr_scope ["*"] with omitted ttl', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['*'], reach_ceiling: 'commons', rate_per_hour: 60 } as any))
      .toThrow(/rotation_ttl_days/i);
  });
  it('rejects reach_ceiling outside {commons,community} without acknowledgement', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['epr:x'], reach_ceiling: 'public', rate_per_hour: 60, rotation_ttl_days: 30 }))
      .toThrow(/reach_elevation_acknowledged/i);
  });
  it('accepts community ceiling without acknowledgement', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['epr:x'], reach_ceiling: 'community', rate_per_hour: 60, rotation_ttl_days: 30 }))
      .not.toThrow();
  });
  it('accepts a bounded wildcard contract (finite rate + ttl + commons)', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['*'], reach_ceiling: 'commons', rate_per_hour: 60, rotation_ttl_days: 30 }))
      .not.toThrow();
  });
});
```

- [ ] **Step 2: Run → FAIL.** `cd genesis/seeder && pnpm exec vitest run src/seed-delegates-compute.test.ts`.

- [ ] **Step 3: Implement the factory** (`[C4]` SAFE_CEILINGS allowlist; `[C8]` recipient = `<PERFORMER_CLAIM>`
  value; posts to the gated storage endpoint):

```typescript
import { createHash } from 'node:crypto';

/** spec §14 schema rule: ack required when ceiling is NOT commons/community. */
const SAFE_CEILINGS = new Set(['commons', 'community']);

export interface DelegatesComputeBounds {
  epr_scope: string[];
  reach_ceiling: string;
  rate_per_hour?: number;
  rotation_ttl_days?: number;
  reach_elevation_acknowledged?: boolean;
}
export interface DelegatesComputePair {
  scope: string;                 // commitment.scope, e.g. 'orchestrate-node'
  providerAgentCid: string;      // the <PERFORMER_CLAIM> value of the granting steward (Matthew)
  recipientAgentCid: string;     // the <PERFORMER_CLAIM> value the client acts AS (Matthew, self-contract)
  bounds: DelegatesComputeBounds;
  validFromIso: string;
  validUntilIso: string;
  fixture?: string;
}

export function assertBoundedMinimum(b: DelegatesComputeBounds): void {
  const hasWildcard = b.epr_scope.includes('*');
  const rateOk = typeof b.rate_per_hour === 'number' && b.rate_per_hour >= 1;
  const ttlOk = typeof b.rotation_ttl_days === 'number' && b.rotation_ttl_days >= 1;
  if (hasWildcard && !rateOk) throw new Error('minimum-bounds: epr_scope ["*"] requires a finite rate_per_hour (>=1)');
  if (hasWildcard && !ttlOk) throw new Error('minimum-bounds: epr_scope ["*"] requires a finite rotation_ttl_days (>=1)');
  if (!b.reach_ceiling) throw new Error('minimum-bounds: reach_ceiling is required');
  if (!SAFE_CEILINGS.has(b.reach_ceiling) && b.reach_elevation_acknowledged !== true) {
    throw new Error(`minimum-bounds: reach_ceiling '${b.reach_ceiling}' outside {commons,community} requires reach_elevation_acknowledged=true`);
  }
}

function delegatesComputeId(pair: DelegatesComputePair): string {
  const d = createHash('sha256').update(`${pair.providerAgentCid}|${pair.recipientAgentCid}|${pair.scope}`).digest('hex').slice(0, 16);
  return `delegates-compute-${d}`;
}

/** Body for the gated storage seed endpoint (Task 2). The endpoint writes mishpat_commitments DIRECTLY
 *  (scope/bounds_json/valid_from/valid_until/recipient/provider + synthesized anchor) — NOT /api/v1/commitments. */
export function buildDelegatesComputeBody(pair: DelegatesComputePair) {
  assertBoundedMinimum(pair.bounds);
  return {
    cid: delegatesComputeId(pair),
    action: 'delegates-compute' as const,
    scope: pair.scope,
    provider: pair.providerAgentCid,
    recipient: pair.recipientAgentCid,
    bounds: { ...pair.bounds, _provenance: 'dev-seed' },  // honesty marker (Task 2 stores in bounds_json)
    validFrom: pair.validFromIso,
    validUntil: pair.validUntilIso,
  };
}

/** POST each pair to the gated storage seed endpoint (ALLOW_SEED_DELEGATES_COMPUTE=1; 403 if off). Idempotent. */
export async function seedDelegatesComputeCommitments(storageUrl: string, token: string, pairs: DelegatesComputePair[]): Promise<void> {
  for (const pair of pairs) {
    const body = buildDelegatesComputeBody(pair);
    const res = await fetch(`${storageUrl}/admin/seed/delegates-compute`, {
      method: 'POST', headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` }, body: JSON.stringify(body),
    });
    if (res.ok) { console.log(`[+] delegates-compute ${body.cid} (active)`); continue; }
    if (res.status === 403) { console.error('[x] ALLOW_SEED_DELEGATES_COMPUTE is not set on this node — refusing to seed'); process.exit(1); }
    const text = await res.text();
    if (res.status === 409 || /exists/i.test(text)) { console.log(`[=] delegates-compute ${body.cid} (idempotent)`); continue; }
    console.error(`[x] delegates-compute ${body.cid}: ${res.status} ${text}`); process.exit(1);
  }
}

export function defaultDelegatesComputePairs(): DelegatesComputePair[] {
  const m = requireEnv('MATTHEW_AGENT_CID');   // the Phase-0 <PERFORMER_CLAIM> value
  return [{
    scope: 'orchestrate-node',
    providerAgentCid: m, recipientAgentCid: m,           // self-contract (spec §2 self-custody)
    bounds: { epr_scope: ['*'], reach_ceiling: 'commons', rate_per_hour: 60, rotation_ttl_days: 30 },
    validFromIso: requireEnv('SEED_NOW_ISO'), validUntilIso: requireEnv('SEED_VALID_UNTIL_ISO'),
    fixture: 'che-dogfood-self-contract',
  }];
}
function requireEnv(k: string): string { const v = process.env[k]; if (!v) { console.error(`missing env ${k}`); process.exit(1); } return v; }
```

- [ ] **Step 4: Add the CLI entry block** (mirror `seed-commitments.ts:399-429`): read `STORAGE_URL`
  (default `http://localhost:8090`), the auth token, optional `DELEGATES_PAIRS_JSON`; assert pairs not
  suspended via `deployments.json`; health-check; `seedDelegatesComputeCommitments(...)`; exit 0. (The seed
  endpoint writes the row already `active` — no separate activate step, since it's a direct projection write.)

- [ ] **Step 5: Add package.json scripts:**

```json
"seed:delegates": "npx tsx src/seed-delegates-compute.ts",
"seed:delegates:dev": "STORAGE_URL='https://doorway-alpha.elohim.host' npx tsx src/seed-delegates-compute.ts"
```

- [ ] **Step 6: Run the guard test → PASS** (all five cases). Commit.

```bash
git add genesis/seeder/src/seed-delegates-compute.ts genesis/seeder/src/seed-delegates-compute.test.ts genesis/seeder/package.json
git commit -m "feat(seeder): bounded seed-delegates-compute factory targeting the gated mishpat seed endpoint (Che op-gate Slice 1)"
```

---

## Task 2: gated storage seed+revoke endpoint writing `mishpat_commitments` directly `[C1]`

**Files:**
- Create: `elohim/elohim-storage/src/api/seed_delegates_compute.rs` (the gated handler)
- Modify: `elohim/elohim-storage/src/db/mishpat_commitments.rs` (ensure an `upsert`/insert + a `set_revoked_at`
  helper exist; reuse `upsert_with_anchor` if present — confirmed in Phase 0)
- Modify: `elohim/elohim-storage/src/http.rs` (register `POST /admin/seed/delegates-compute`, ingress-class
  like the existing `/admin/seed/*` routes — NOT in the public doorway proxy manifest)
- Test: `elohim/elohim-storage/tests/seed_delegates_compute.rs`

> This task REPLACES the original "performer check in bounds_validator" (deleted per `[C3]` — that check is
> now isolated in Task 3). It supplies the missing **write path** to `mishpat_commitments`.

**Interfaces:**
- Consumes (all CONFIRMED to exist 2026-06-26 — do NOT re-add): `db::mishpat_commitments::upsert_with_anchor`
  (**2-arg**: `(conn, NewMishpatCommitment)` — `dht_anchor_hash` and `revoked_at` are FIELDS of
  `NewMishpatCommitment`, `models.rs:3596`, **not** a separate parameter), `db::mishpat_commitments::set_revoked_at`
  (`:141`), `db::mishpat_commitments::get_by_cid` (`:99`), `p2p::identity_handshake::synthesise_dht_anchor_hash`
  for the synthesized dev anchor.
- Produces: `POST /admin/seed/delegates-compute` gated by `ALLOW_SEED_DELEGATES_COMPUTE=1` (403 otherwise),
  accepting `{cid, action, scope, provider, recipient, bounds, validFrom, validUntil}` (insert, state=`active`,
  synthesized `dht_anchor_hash`, `bounds_json` carries `_provenance:"dev-seed"`) AND a revoke variant
  `{cid, revoke:true}` (sets `revoked_at`). The seed factory (Task 1) and the revoke step (Task 5) consume it.

- [ ] **Step 1: Write the failing endpoint test** (`tests/seed_delegates_compute.rs`): with the flag SET,
  POST a bounded delegates-compute body → assert a `mishpat_commitments` row exists with non-NULL
  `dht_anchor_hash`, `state="active"`, `recipient`/`scope`/`bounds_json` persisted; then POST `{cid, revoke:true}`
  → assert `revoked_at` is non-NULL. With the flag UNSET → assert 403 and no row.

- [ ] **Step 2: Run → FAIL** (`CARGO_TARGET_DIR=/tmp/che-opgate-target cargo test --test seed_delegates_compute`).

- [ ] **Step 3: Implement the gated handler** (model on `handle_seed_shard_manifest` — flag check + 403):

```rust
// api/seed_delegates_compute.rs
pub async fn handle_seed_delegates_compute(req: Request<Incoming>, pool: &DbPool) -> Result<Response<Full<Bytes>>, StorageError> {
    if std::env::var("ALLOW_SEED_DELEGATES_COMPUTE").as_deref() != Ok("1") {
        return Ok(response::forbidden("delegates-compute dev-seed is disabled (ALLOW_SEED_DELEGATES_COMPUTE != 1)"));
    }
    let body: serde_json::Value = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    if body.get("revoke").and_then(|v| v.as_bool()).unwrap_or(false) {
        let cid = body.get("cid").and_then(|v| v.as_str()).ok_or_else(|| StorageError::Validation("cid required".into()))?;
        db::mishpat_commitments::set_revoked_at(&mut conn, cid, &now_iso())?;
        return Ok(response::ok_json(&serde_json::json!({"cid": cid, "revoked": true})));
    }
    let cid = str_field(&body, "cid")?;
    let bounds_json = serde_json::to_string(body.get("bounds").unwrap_or(&serde_json::Value::Null))?; // carries _provenance:"dev-seed"
    // Synthesized anchor: a CLEARLY dev-seeded anchor so the fail-closed fetcher accepts the row on a
    // flag-on node ONLY. The flag is the dev/prod boundary (seed_shard_manifest model).
    let anchor = crate::p2p::identity_handshake::synthesise_dht_anchor_hash(&str_field(&body,"recipient")?, &cid);
    // upsert_with_anchor is 2-arg: anchor + revoked_at are FIELDS of NewMishpatCommitment (models.rs:3596).
    db::mishpat_commitments::upsert_with_anchor(&mut conn, NewMishpatCommitment {
        cid: cid.clone(),
        action: "delegates-compute".into(),
        scope: str_field(&body,"scope")?,
        provider: str_field(&body,"provider")?,
        recipient: str_field(&body,"recipient")?,
        bounds_json,
        valid_from: str_field(&body,"validFrom")?,
        valid_until: str_field(&body,"validUntil")?,
        revoked_at: None,
        dht_anchor_hash: Some(anchor),  // synthesized dev anchor — flag-gated; the fetcher accepts non-NULL
        state: "active".into(),
    })?;
    Ok(response::ok_json(&serde_json::json!({"cid": cid, "state": "active", "provenance": "dev-seed"})))
}
```

  (Adapt `str_field`/`response::*`/`NewMishpatCommitment` exact field set to the real helpers. `set_revoked_at`
  (`:141`) and `get_by_cid` (`:99`) already EXIST — do not re-add them.)

- [ ] **Step 4: Register the route** in `http.rs` alongside the other `/admin/seed/*` routes (ingress-class,
  NOT surfaced into the doorway public proxy manifest — assert this in the test). Run the test → PASS.

- [ ] **Step 5: `cargo fmt` + `clippy -D warnings`; commit.**

```bash
git add elohim/elohim-storage/src/api/seed_delegates_compute.rs elohim/elohim-storage/src/db/mishpat_commitments.rs \
        elohim/elohim-storage/src/http.rs elohim/elohim-storage/tests/seed_delegates_compute.rs
git commit -m "feat(storage): gated dev-seed endpoint writing delegates-compute into mishpat_commitments (Che op-gate Slice 1)"
```

---

## Task 3: storage operation-authorization service + endpoint

**Files:**
- Create: `elohim/elohim-storage/src/services/operation_authorization.rs`
- Modify: `elohim/elohim-storage/src/db/mishpat_commitments.rs` (add `find_active_delegates_compute`)
- Modify: `elohim/elohim-storage/src/http.rs` + `src/api/` (register `POST /api/v1/authorize-operation`)
- Test: `elohim/elohim-storage/tests/operation_authorization.rs`

**Interfaces:**
- Consumes: `bounds_validator::validate` (`:107`), `ProjectionCommitmentFetcher::new(pool)`, `DieselRateHistory`,
  `EventForValidation`, the `mishpat_commitments` model.
- Produces: `authorize_operation(pool, req: AuthorizeOperationRequest, signed_at: String) -> AuthorizeOperationResult`
  where `AuthorizeOperationRequest { performer, capability, target_epr_id: Option<String>, reach }` and
  `AuthorizeOperationResult { allowed: bool, commitment_cid: Option<String>, reason: String }`; the HTTP
  endpoint `POST /api/v1/authorize-operation` (`.auth_required()`, see D6 — NOT in the public proxy manifest).

- [ ] **Step 1: Write the failing service test** (`[I8]` real stubs `MockCommitmentFetcher.seed(...)` +
  `MockRateHistory::new()`; `[C7]` 3-arg calls with `signed_at`; `[I1]` scope pushed into the lookup). Seed
  an `active`, notarized (non-null anchor) row `recipient=provider="uhCAk-matthew"`, `scope="orchestrate-node"`,
  bounds `{epr_scope:["*"],reach_ceiling:"commons",rate_per_hour:60,rotation_ttl_days:30}`, then:

```rust
let now = "2026-06-26T12:00:00Z".to_string();
// (a) authorized
let r = authorize_operation(&pool, req("uhCAk-matthew","orchestrate-node"), now.clone()).await;
assert!(r.allowed, "{}", r.reason);
assert_eq!(r.commitment_cid.as_deref(), Some(&*seeded_cid));
// (b) wrong capability → denied (no grant with that scope; lookup filters scope in SQL)
assert!(!authorize_operation(&pool, req("uhCAk-matthew","node:wipe"), now.clone()).await.allowed);
// (c) performer with no grant → denied (fail-closed)
assert!(!authorize_operation(&pool, req("uhCAk-stranger","orchestrate-node"), now.clone()).await.allowed);
// (d) performer != recipient (explicit guard) → denied. Seed a SECOND grant recipient="uhCAk-alice"
//     with a DISTINCT scope="alice-only-op" (distinct so matthew's own orchestrate-node grant can't match);
//     ask as performer "uhCAk-matthew", capability "alice-only-op" → no row for matthew → deny.
// (e) after revoke → denied on the NEXT call (spec §13)
db::mishpat_commitments::set_revoked_at(&mut conn, &seeded_cid, &now);
assert!(!authorize_operation(&pool, req("uhCAk-matthew","orchestrate-node"), now.clone()).await.allowed);
```

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Add the scoped diesel lookup `[I1]`** (scope filter in SQL — avoids a newer differently-scoped
  grant shadowing a valid one):

```rust
/// The active, not-revoked, NOTARIZED delegates-compute grant for (recipient, capability), newest first.
/// `provider` is intentionally NOT filtered in Slice 1 (the self-contract has provider==recipient); a
/// provider filter is a Slice-3 multi-party concern — documented here, not enforced.
pub fn find_active_delegates_compute(
    conn: &mut SqliteConnection, recipient_cid: &str, capability: &str,
) -> Result<Option<MishpatCommitment>, diesel::result::Error> {
    use crate::db::diesel_schema::mishpat_commitments::dsl as c;
    c::mishpat_commitments
        .filter(c::action.eq("delegates-compute"))
        .filter(c::recipient.eq(recipient_cid))
        .filter(c::scope.eq(capability))
        .filter(c::state.eq("active"))
        .filter(c::revoked_at.is_null())
        .filter(c::dht_anchor_hash.is_not_null())
        .order(c::created_at.desc())
        .first::<MishpatCommitment>(conn)
        .optional()
}
```

- [ ] **Step 4: Implement the service** (`[C2]`/`[C3]` performer bind is the service's OWN deny, not a
  `BoundsViolation` variant; the `EventForValidation` adapter reuses `bounds_validator`; `[I6]` rate-limit is
  inert here):

```rust
use crate::services::bounds_validator::{validate, EventForValidation};
use crate::services::commitment_fetcher::ProjectionCommitmentFetcher;
use crate::services::rate_history::DieselRateHistory;

pub struct AuthorizeOperationRequest { pub performer: String, pub capability: String, pub target_epr_id: Option<String>, pub reach: String }
pub struct AuthorizeOperationResult { pub allowed: bool, pub commitment_cid: Option<String>, pub reason: String }

pub async fn authorize_operation(pool: &crate::db::DbPool, req: AuthorizeOperationRequest, signed_at: String) -> AuthorizeOperationResult {
    let mut conn = match pool.get() { Ok(c) => c, Err(e) => return deny(None, format!("db pool: {e}")) };
    let commitment = match crate::db::mishpat_commitments::find_active_delegates_compute(&mut conn, &req.performer, &req.capability) {
        Ok(Some(c)) => c,
        Ok(None) => return deny(None, "no active delegates-compute grant for (performer, capability)".into()),
        Err(e) => return deny(None, format!("lookup: {e}")),
    };
    drop(conn);
    // performer == recipient: structural (the lookup filtered recipient.eq(performer)); explicit guard for clarity. [C3]
    if commitment.recipient != req.performer {
        return deny(Some(commitment.cid), "performer is not the grant recipient".into());
    }
    let event = EventForValidation {
        action: req.capability.clone(),                 // bounds check 4 compares event.action == commitment.scope
        performer: req.performer.clone(),
        bounded_by: commitment.cid.clone(),
        target_epr_id: req.target_epr_id.unwrap_or_else(|| "*".into()),
        reach: req.reach.clone(),
        signed_at,
    };
    let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
    let rate = DieselRateHistory::new(pool.clone());     // [I6] rate check is inert here (counts economic_events; none emitted)
    match validate(&event, &fetcher, &rate).await {
        Ok(_) => AuthorizeOperationResult { allowed: true, commitment_cid: Some(commitment.cid), reason: "ok".into() },
        Err(v) => deny(Some(commitment.cid), format!("{v:?}")),  // fail-closed: any BoundsViolation denies
    }
}
fn deny(cid: Option<String>, reason: String) -> AuthorizeOperationResult { AuthorizeOperationResult { allowed: false, commitment_cid: cid, reason } }
```

- [ ] **Step 5: Register the endpoint** (`POST /api/v1/authorize-operation`, `.auth_required()` per **D6** —
  the doorway forwards the user's Bearer; performer comes from the body, set by the doorway from ITS verified
  JWT). Handler supplies `signed_at = now_iso()`, returns `{allowed, commitmentCid, reason}` with **200 +
  allowed:false** for a verdict and **503** only on `ConductorUnreachable`. **HARD REQUIREMENT `[C6]`:**
  register the handler in `http.rs` but DELIBERATELY KEEP IT OUT of `build_manifest()` — the storage
  convention auto-promotes any `build_manifest()` route to a PUBLIC doorway proxy route, which would reopen
  the unauthenticated-verdict-oracle DoS C6 closes. Add a test assertion that `/api/v1/authorize-operation`
  is ABSENT from the public proxy manifest; treat a failing assertion as a release blocker, not a warning.

- [ ] **Step 6: Run the service test → PASS** (all cases incl. revoke-denies-next). `fmt` + `clippy`; commit.

```bash
git add elohim/elohim-storage/src/services/operation_authorization.rs elohim/elohim-storage/src/db/mishpat_commitments.rs \
        elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/api elohim/elohim-storage/tests/operation_authorization.rs
git commit -m "feat(storage): operation-authorization gate over mishpat_commitments reusing bounds_validator (Che op-gate Slice 1)"
```

---

## Task 4: doorway pre-dispatch op-gate on `POST /db/content` + `/db/content/bulk`

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (insert the gate after the `/blob/` early-return,
  before forward `~:3754`; the path is `p`, the storage URL is `endpoint` `[C5]`; add a verified-claims
  resolver returning the Phase-0 `<PERFORMER_CLAIM>`, and `call_authorize_operation` forwarding the user's
  Bearer per **D6**)
- Modify: doorway config/args (`DELEGATES_COMPUTE_OP_GATE` mode flag, default `off`; the `CHE_FACING` /
  enforce + jwt_secret startup refusals `[I4]`/`[I5]`)
- Test: doorway unit test for the gate decision (mode matrix + fail-closed + bulk inclusion)

**Interfaces:**
- Consumes: the verified `<PERFORMER_CLAIM>` claim, the user's Bearer token (already on the request),
  `state.storage_proxy_client`, storage `POST /api/v1/authorize-operation` (Task 3).
- Produces: the gated dispatch path for `POST /db/content` AND `/db/content/bulk`.

- [ ] **Step 1: Add the mode flag + startup refusals.** `OpGateMode ∈ {Off, Observe, Enforce}` from
  `DELEGATES_COMPUTE_OP_GATE` (default `Off`). At boot, if `CHE_FACING=1`: refuse to start unless
  `mode==Enforce` `[I4]`, `dev_mode` is off, and `jwt_secret` is present and ≥32 chars `[I5]`.

- [ ] **Step 2: Write the failing decision test.** Extract the decision as a pure fn over
  `(mode, method, path, verdict)`. Assert: `enforce`+`allowed:false`→`Deny(403)`; `enforce`+`allowed:true`→`Allow`;
  `enforce`+storage-error→`Deny(403)` (fail-closed); `observe`+`allowed:false`→`AllowWithWarn`; `off`→`Allow`
  (no storage call); **`/db/content/bulk` is gated identically to `/db/content`** `[C5]`; a non-matching path
  (`/db/other`) is never gated.

- [ ] **Step 3: Run → FAIL.** `cd doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/che-doorway-target cargo test op_gate`

- [ ] **Step 4: Implement.** Insert in the `Disposition::StorageProxy` arm, after the `/blob/` early-return,
  before forward. **Gate on `p` (the path), match BOTH content routes** `[C5]`:

```rust
// http.rs — StorageProxy arm, AFTER agent resolution (~:3734) / the /blob/ early-return, BEFORE forward (~:3754).
let is_gated_write = method == Method::POST && (p == "/db/content" || p == "/db/content/bulk");
if mode != OpGateMode::Off && is_gated_write {
    // performer from the VERIFIED credential (Phase-0 <PERFORMER_CLAIM>), never the client X-Agent-Cid. [C8]
    let performer = match resolve_verified_claim_from_request(&state, &req) {
        Some(cid) => cid,
        None => return deny_403("op-gate: no verified credential"),   // fail-closed
    };
    // D6: forward the user's own Authorization on the authorize call; performer is the doorway-verified value.
    let bearer = extract_bearer(&req);
    let verdict = call_authorize_operation(&state, bearer.as_deref(), &performer, "orchestrate-node").await;
    match (mode, &verdict) {
        (OpGateMode::Enforce, Ok(v)) if v.allowed => { /* fall through to forward */ }
        (OpGateMode::Enforce, Ok(v)) => return deny_403(&format!("op-gate denied: {}", v.reason)),
        (OpGateMode::Enforce, Err(e)) => return deny_403(&format!("op-gate fail-closed: {e}")),  // infra → deny
        (OpGateMode::Observe, _) => warn!("op-gate OBSERVE would-{}: {:?}",
            if matches!(&verdict, Ok(x) if x.allowed) { "allow" } else { "deny" }, verdict),
        (OpGateMode::Off, _) => unreachable!(),
    }
}
// existing: forward_to_storage(req, &endpoint, p, client, breakers, ctx)
```

  `resolve_verified_claim_from_request` returns the Phase-0 `<PERFORMER_CLAIM>` value (a sibling to
  `resolve_agent_cid_from_request`, reading the resolved claim off the verified `Claims`).
  `call_authorize_operation` POSTs `{performer, capability, reach:"commons"}` to
  `<storage>/api/v1/authorize-operation` via `state.storage_proxy_client`, **forwarding the user's Bearer**
  (D6), parses `{allowed, reason}`, and treats any transport/parse error as `Err` (→ deny in enforce).

- [ ] **Step 5: Run the decision test → PASS** (incl. `/db/content/bulk` gated; `off` never calls storage).
  Add a Self-review invariant: *the gate's path set MUST equal the routes calling `distribute_shards`
  (`{/db/content, /db/content/bulk}` at `http.rs:4301,4487`).* `fmt`+`clippy` (`RUSTFLAGS=""`); commit.

```bash
git add doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/   # args/config + test
git commit -m "feat(doorway): pre-dispatch delegates-compute op-gate on /db/content + /bulk (fail-closed, 3-mode, CHE_FACING boot-refusal)"
```

---

## Task 5: the driving + display loop (COMPOSE WITH Wave-1.3 — the unblock)

**Files:** none new (drive + observe). **Consumes:** the seeded grant (Tasks 1–2), the gate (Tasks 3–4),
the Wave-1.3 observation steps. **Produces:** the captured green observation through the **governed** path +
the revoke-denies-next proof.

> This is **Wave-1.3 Task 0 Steps 4–5 + Task 1 with `DELEGATES_COMPUTE_OP_GATE=enforce`**. Do not re-author
> the observation scenarios — run the Wave-1.3 ones with the gate on.

- [ ] **Step 1: Bring the stack up `enforce` + seeded** (local M/J/J; matthew = ingest/custodian node;
  `--features p2p` per Phase 0; `dev_mode` OFF; `ALLOW_SEED_SHARD_MANIFEST` UNSET so lighting is governed).
  Task 1+2 already seeded the active Matthew→Che self-contract into `mishpat_commitments`.

- [ ] **Step 2: Authorized drive — POST blob-backed content holding only a JWT** (`[I2]` send a non-empty
  password; the account's `<PERFORMER_CLAIM>` must equal `MATTHEW_AGENT_CID` — provision a Mongo user, or
  target the deployed matthew pod, since `dev_mode` is OFF and mints a different agent):

```bash
TOKEN=$(curl -s -X POST localhost:8888/auth/login -H 'content-type: application/json' \
  -d '{"identifier":"matthew","password":"<dev-password>"}' | python3 -c "import sys,json;print(json.load(sys.stdin)['token'])")
curl -s -o /dev/null -w "%{http_code}\n" -X POST localhost:8888/db/content -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d @<blob-backed-content.json>    # expect 200 (gate allowed)
```

  Expected: **200**; the custodian conductor runs `distribute_shards`; `shard_locations` populates
  (`status="announced"`).

- [ ] **Step 3: Observe the card light through the governed path.**

```bash
curl -s "localhost:8090/api/v1/resilience/<contentId>" | python3 -c "import sys,json;d=json.load(sys.stdin);print('stewardingCollectives=',d.get('stewardingCollectives'),'distributionState=',d.get('distributionState'))"
# or render: from genesis/a2o → pnpm look <doorway resilience URL> ; Read reports/look/<slug>/shot.png
```

  Expected: `stewardingCollectives > 0`, `distributionState:"measured"`.

- [ ] **Step 4: Prove revocation denies the NEXT request (spec §13)** — revoke via the gated endpoint (Task 2),
  NOT `/api/v1/commitments` (`[C1]` wrong table):

```bash
curl -s -X POST localhost:8090/admin/seed/delegates-compute -H 'content-type: application/json' \
  -d '{"cid":"<delegates-compute-cid>","revoke":true}'
curl -s -o /dev/null -w "%{http_code}\n" -X POST localhost:8888/db/content -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d @<blob-backed-content.json>    # expect 403
```

  Expected: **403** — the per-request consult denies once revoked. Step 2's in-flight fan-out is unaffected.

- [ ] **Step 5: Capture the observation** — append to `.claude/data/dev-intent.jsonl`: the gate mode, the
  authorized-200 → `stewardingCollectives` value, and the revoke → 403 proof.

---

## Task 6: a2o regression + story-harvest the governance constraints

**Files:**
- Create/extend: `genesis/a2o/features/` — a `@governed-distribution @local` scenario (compose with
  `resilience/observable-distribution.feature`; do NOT fork it).
- Modify: `genesis/a2o/steps/` — step-defs for "given an active bounded delegates-compute grant", "when
  Matthew POSTs blob content holding only a credential", "then the card reads stewardingCollectives>0",
  "when the grant is revoked", "then the next POST is denied 403".

- [ ] **Step 1: Write the two-case scenario** — (a) *authorized governed distribution* (active grant → POST
  blob → `stewardingCollectives >= 1`); (b) *revocation denies the next request* (revoke → next POST 403).

- [ ] **Step 2: Implement the step-defs** reusing the Wave-1.3 `resilience.steps.ts` ingest/assertion helpers
  (`:186`/`:303`/`:403`); add only the gated-seed + revoke steps (hitting the Task-2 endpoint).

- [ ] **Step 3: Run green on the local mesh** (gate `enforce`):
  `E2E_MODE=local pnpm test:local --tags '@governed-distribution and @local and not @wip'` → PASS.

- [ ] **Step 4: Run `story-harvest`** to preserve the parameter-bearing constraints — the **bounded-minimum
  guard**, the **gated-seed/notarization-required** authorization, the **performer==recipient** bind, the
  **per-request fail-closed** revocation, and the **gate-covers-both-content-routes** invariant.

- [ ] **Step 5: Commit** scenario + step-defs together.

---

## Task 7: deploy-posture honesty gate

**Files:** Modify: the Che-facing deploy manifest/config (repo surface only — never `kubectl`).

- [ ] **Step 1: Assert the coherent Che-facing posture in the repo:** `dev_mode` OFF; `DELEGATES_COMPUTE_OP_GATE=enforce`;
  `CHE_FACING=1` (drives the boot-refusals `[I4]`/`[I5]`); `jwt_secret` present + ≥32 chars; `ALLOW_SEED_SHARD_MANIFEST`
  UNSET (governed lighting only); `ALLOW_SEED_DELEGATES_COMPUTE` set ONLY where the dogfood self-contract is
  intentionally seeded (and OFF on any production-class node).

- [ ] **Step 2: If the deploy path has no such coherence guard, add the startup assertion** (refuse to boot
  with an incoherent combination — `enforce` + `dev_mode`, or `enforce` + missing/weak `jwt_secret`).

- [ ] **Step 3: Commit** the manifest/config coherence change (the next pipeline reconciles it).

---

## Self-review (run before handoff)

- **Spec coverage:** §10 Slice 1 (a) bounded seed factory → Tasks 1+2; (b) doorway pre-dispatch op-gate
  per-request fail-closed performer-bound → Tasks 3+4; (c) Che drives via `/auth/login`, card reads
  stewardingCollectives>0 → Task 5. §14: per-request fail-closed (T3/T4); performer==recipient **in the
  op-gate only** (T3) `[C3]`; bounded-seed guard (T1, allowlist `[C4]`); normal-routes-only incl. `/bulk`
  (T4 `[C5]`); no dev_mode + jwt floor (T4/T7 `[I4]`/`[I5]`); capability∧scope conjunction with scope-in-SQL
  (T3 `[I1]`); distribute_shards-is-node's-own (Global Constraints + T5); JWT-not-web2-verifiable honesty
  (Global Constraints).
- **No new DHT entity / coordinator / sync dialect** (Operational-C seed-projection + op-gate over existing
  Notarized-A commitment).
- **Type/name consistency:** `EventForValidation` fields (T3); `AuthorizeOperationRequest/Result` (T3↔T4);
  `find_active_delegates_compute(conn, recipient, capability)` `[I1]` consistent T3; `authorize_operation`
  is **3-arg** (`pool, req, signed_at`) at every call site `[C7]`; `<PERFORMER_CLAIM>` is the one identity
  value across seed `recipient`, op-gate filter, and `MATTHEW_AGENT_CID` `[C8]`; no `BoundsViolation` variant
  added `[C2]`.
- **Two-table correctness `[C1]`:** the seed writes `mishpat_commitments` (Task 2 gated endpoint); the
  op-gate reads `mishpat_commitments` (Task 3); nothing in Slice 1 routes a `delegates-compute` body through
  `/api/v1/commitments`.
- **Composition:** Task 5 = Wave-1.3 observation with the gate in front (no forked scenarios).

## Land-now vs Held

- **LAND-NOW (household-nodes class):** all tasks (local M/J/J `hc:start:seed` stack or the live matthew pod
  via the deployed doorway). Nothing needs shem / alpha-cluster-6peer / harbor.
- **NOT IN SCOPE (later slices / backlog):** **D1=A** the genuinely DHT-notarized `delegates-compute`
  conductor commit (the honest follow-up to the gated dev-seed); minting `attestation:device-client-authorization`
  (Slice 2); the op-gate verifying the attestation instead of the JWT (Slice 2); admin-route capability-scoping;
  hoster-as-steward custody + recovery quorum + forker onboarding (Slice 3+); OIDC↔portal auto-binding;
  extending JWT claims; an **operation-counter** so the rate-limit bound becomes enforceable on the op-gate
  path `[I6]`; substrate-side enforcement closing the direct-to-storage bypass `[I9]`.

## Done (definition of Slice-1 landing)

- A **bounded, active `delegates-compute`** self-contract (Matthew→Che, `provider == recipient ==
  <PERFORMER_CLAIM>`, finite rate+ttl, `scope=orchestrate-node`, non-NULL anchor) sits in **`mishpat_commitments`**,
  written by the gated dev endpoint; the minimum-bounds guard (allowlist) is unit-green.
- The storage `authorize-operation` gate authorizes a valid request, denies wrong-capability (scope-in-SQL),
  denies no-grant, denies `performer != recipient`, and **denies the next call after revocation** — all green
  in `tests/operation_authorization.rs`. (The rate-limit bound is documented inert on this path `[I6]`.)
- The doorway pre-dispatch gate, in `enforce`, allows the authorized `POST /db/content` **and `/db/content/bulk`**
  and 403s a revoked one; `off` is a verified no-op; the `CHE_FACING` boot-refusals hold.
- Matthew, in Eclipse Che holding only a JWT (no key), drives `distribute_shards` on the live mesh through
  the gate; the card reads `stewardingCollectives > 0` (governed `distribute_shards` push, `ALLOW_SEED_SHARD_MANIFEST`
  unset). The governed-distribution a2o is green on the local mesh across two consecutive runs.
- The Che-facing deploy posture is coherent in the repo (dev_mode off, gate `enforce`, jwt floor,
  `ALLOW_SEED_*` postures correct).
- The dogfooding loop is closed: developing the p2p-dataplane now requires participating in it.

## Non-goals

- Does NOT add a DHT entity, coordinator, or sync dialect (P2P-gate already satisfied).
- Does NOT mint the DHT attestation credential or harden JWT→attestation (Slice 2), nor add the
  performer-check to the shared `bounds_validator` `[C3]`.
- Does NOT route a `delegates-compute` body through `/api/v1/commitments` (wrong table `[C1]`).
- Does NOT touch `/admin/*` proxy routes or substitute the gate for ingress isolation; does NOT claim the
  gate is substrate enforcement (`[I9]` — it is a doorway-layer control under ingress isolation).
- Does NOT remove query-string token acceptance (separate hardening), change blob encryption, or the
  `content_reach` derivation TODO; does NOT promise sustained live-alpha lighting (leak-gated; Wave-1.3 Task 6A).
