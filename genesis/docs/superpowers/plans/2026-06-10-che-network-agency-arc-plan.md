---
title: Che Network-Agency Arc — Implementation Plan (Stages A/B/C + dev-surface coherence + SDK complement)
id: che-network-agency-arc-plan
status: Draft
class: process-meta
process_subdomain: agents
sprint: unranked — born 2026-06-10 post-roadmap-regen; ledger/focus are the live source
cites:
  - che-network-agency-arc-design | the spec this plan implements — Stages A/B/C, dual-plane discipline, dev-surface coherence, SDK complement | sha256:2902184a7c95c0d0 | path: genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md
  - sprint1-zd-substrate-correct-deploy | owns the deploy-instance of delegates-compute + bounds_validator — Phase 4 consumes its rails behind the 4.1 readiness gate, never rebuilds them | sha256:fbb8a4a2885b0499 | path: genesis/docs/superpowers/plans/2026-05-28-sprint1-zd-substrate-correct-deploy.md
  - epr-acquisition-slice2a-rea-rails-plan | owns the foundational REA emit + commitment-graduation rails (active, D9) — the CommitmentCommitted 2a gap and graduation semantics live in its lane | sha256:62a490200c40f5d4 | path: genesis/docs/superpowers/plans/2026-06-08-epr-acquisition-slice2a-rea-rails-plan.md
  - genesis/docs/architecture/elohim-sdk.md
  - genesis/docs/superpowers/plans/2026-05-18-sdk-boundary-clarification.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md
---

# Che Network-Agency Arc — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the Che agentic developer network agency in three canonical states — hosted-session
(A), sovereign-peer (B), delegated (C) — while landing the work as SDK surface (not one-off
scripts) and making the startup scripts speak the same vocabulary (network profiles).

**Orientation (front-discovery, 2026-06-10):** class `process-meta` (no honest D#; process home =
SDK libraries + `genesis/a2o` tooling + `app/elohim-app/scripts` + gospel CLAUDE.mds). Semantic
lens unavailable this session — **degraded to lexical-only, stated explicitly**; lexical surfacing
found the two plans this one must compose with: **Z.D Sprint 1** (46 OPEN — owns the
deploy-instance of `delegates-compute`, the deploy-service-agent, and the Sprint-2
`bounds_validator`) and **Slice 2a REA rails** (21 OPEN, active, D9 — owns foundational
event-emit + commitment-graduation rails). **This plan builds NEITHER of those.** Stage C consumes
their rails with a *new instance* (agent content-authorship) and stops at an explicit checkpoint
if the rails aren't ready. Not on the regenerated roadmap (born today); the gap-item ledger is
the live source.

**Substrate already landed (verified via commits b2380b899 / 7f66391b6 / bf2efd191 + grep):**
`delegates-compute` payload schemas, mishpat integrity + coordinator commitment paths, storage
projection/signals, `ConductorCommitmentFetcher`. A checked box elsewhere is a claim — Phase 4
re-verifies the pieces it depends on before consuming them.

**In-flight simplification discipline (operator directive, 2026-06-10):** every task on this plan
carries a standing sub-obligation — *leave the touched surface simpler than found*. When a task
opens a file, name the accreted complexity in it (duplicated logic, dead flags, parallel
implementations, stale indirection); **simplify in-scope when bounded** (same files, no behavior
change beyond the task's own), otherwise **capture to backlog with file:line** — never absorb a
refactor that widens the task, never leave a discovery unrecorded. Known accretions on this
plan's surfaces (from the 2026-06-10 surveys), routed to the phases that touch them:

| Accretion | Routed to |
|---|---|
| a2o `doorway-client.ts` hand-rolled auth types ×3 consumers | Phase 1 (deleted by design) |
| `local-stack.ts` re-implements start/health/seed | Task 2.3 (deleted by design) |
| DNA build logic in 4 places (`hc-start.sh:137-163`, devfile, justfile, `hc-build.sh`) | Task 2.1 names ONE canonical path in CLAUDE.md; consolidation beyond docs → backlog if unbounded |
| Two seeders invoked differently (`seed.ts` vs `seed-sqlite.ts`) | Task 2.3 (one path + one documented exception) |
| `.hc_ports` file-based discovery coupling (silent staleness, `hc-start.sh:103-106` / `storage-start.sh:101`) | Task 2.1 in-scope if bounded (validate-or-regenerate), else backlog |
| devfile `start-doorway` hardcoded conductor URL (dead path) | Task 2.3 (deprecation comment) |

---

## Phase 0 — Stage A enablement (operator + verification; small)

### Task 0.1: Operator grants the scoped fixture-auth permission (OPERATOR-OWNED)

The 2026-06-10 classifier denial is the rail working; the grant must be explicit settings, not
inferred. **Files:** `.claude/settings.local.json` (operator applies).

- [x] Operator adds a permission rule allowing a2o fixture-auth flows against alpha (the
      `pnpm look * --as *` / `pnpm test:browser` command family from `genesis/a2o`). Use the
      `update-config` skill interactively; scope to the command shapes, not blanket Bash.
- [x] Record the grant + its intent in the journal/session notes (authorized-by line).

### Task 0.2: Verify `look --as Matthew` end-to-end (closes the L3 open item)

**Files:** none expected (verification; fix `look.ts` only if a defect surfaces).

- [x] Run: `cd genesis/a2o && pnpm look "https://doorway-alpha.elohim.host/lamad" --as Matthew --wait-testid <known-dashboard-testid> --out stage-a-verify`
- [x] `Read` the screenshot; confirm an authenticated surface (not the login page);
      `capture.json` shows `"as":"Matthew","ok":true`. If login fails, debug `device.login`
      against `auth_routes.rs` before any code change.
- [x] Check the box on the arc spec's Stage-A item + the L3 spec's Hygiene-#4 item; re-run
      `decompose.py` on both specs (claims now carry today's evidence).

### Task 0.3: Write-etiquette doc (the authorized-write rail)

**Files:** `genesis/a2o/CLAUDE.md` (gospel — cite-tooling discipline applies).

- [x] Add a short "Authorized writes on shared alpha" subsection under Tools: test-persona
      content only; no bulk seeding; no destructive flows; alpha state is operator-owned;
      writes happen under an explicit permission grant (Task 0.1), never inferred.
- [x] Commit Phase 0 (docs + any look fix).

### Task 0.4: Operator-viewable eyes (added in-flight, operator directive 2026-06-10)

The operator can't `Read` shot.png the way the agent does; the eyes must be symmetric. Serve
`genesis/a2o/reports/` on port 4201 — the devfile's already-public `ui-playground` Che endpoint
(zero devfile change needed). The doorway/ContentNode publishing path (`publish-results.ts`)
remains the Stage-C-aligned evolution for *reports*; screenshots ride the static route today.

- [x] `genesis/a2o/scripts/serve-reports.ts` — zero-dep static server (dir index, png/json mime,
      traversal guard) + `pnpm reports:serve` (verified 2026-06-10: index 200, shot.png 200
      image/png, `../` → 403)
- [x] Document operator view in `genesis/a2o/CLAUDE.md` Tools (Che `ui-playground` endpoint)
- [ ] Follow-up decision (with Stage C): publish look captures as ContentNodes with blob-backed
      screenshots so results are doorway-presentable beyond the workspace lifetime

---

## Phase 1 — SDK: `DoorwaySessionClient` in `@elohim/identity` (Stage A consolidation)

Auth walking exists in ≥3 places around one wire shape (`app/elohim-app/.../auth.service.ts:46-63`,
`genesis/a2o/src/framework/api/doorway-client.ts:15-88`, `browser-device.ts:45-63`). The SDK canon
(§3.4) names `@elohim/identity` as the home. **Scope discipline:** this phase migrates the **a2o
framework** (the agent's tooling) onto the client. The Angular `auth.service.ts` migration is
captured to backlog — bigger blast radius, own plan.

### Task 1.1: `DoorwaySessionClient` (TDD)

**Files:** `app/elohim-library/projects/elohim-identity/src/lib/doorway-session-client.ts` (+ test).

- [x] Failing tests first (node-fetch/undici-agnostic transport injected): `login`, `register`,
      `logout`, `me`, `exchangeSessionToken`, `restoreSession`; one consolidated `AuthResponse`
      model (field-for-field match with `doorway/doorway-service/src/routes/auth_routes.rs` —
      camelCase wire, no transforms).
- [x] Implement; framework-free (no Angular DI in the core class — Angular wraps it later).
- [x] `pnpm --filter @elohim/identity test` green; lint clean. Commit.

### Task 1.2: Migrate the a2o framework onto it

**Files:** `genesis/a2o/src/framework/api/doorway-client.ts`,
`genesis/a2o/src/framework/devices/browser-device.ts` (+ `playwright-device` login path).

- [x] `DoorwayClient` delegates its auth methods (login/register/logout/me/exchange) to
      `DoorwaySessionClient`; delete the duplicated request/response type definitions (import
      from `@elohim/identity`); non-auth methods unchanged.
- [ ] `pnpm test:unit` in a2o green; one `@browser` cucumber scenario with `@auth` passes
      locally (Stage-A grant from Phase 0 required for the alpha-target run).
- [x] `look --as` re-verified through the migrated path (same command as Task 0.2). Commit.

### Task 1.3: Capture the complementary follow-ups (backlog, NOT this plan)

- [x] Backlog: migrate Angular `auth.service.ts` + doorway-app `auth-state.service.ts` onto
      `DoorwaySessionClient` (`genesis/data/timeline/backlog/angular-auth-onto-doorway-session-client.md`)
- [x] Backlog: doorway auth wire shapes into the view-schema contract system (today hand-authored
      in 3 places; drift risk) (`genesis/data/timeline/backlog/doorway-auth-view-schema-contract.md`)
      — source of truth: operational session state (Category C; doorway-local, reconstructable by
      re-auth); the schema describes the HTTP wire shape only, no new storage.

---

## Phase 2 — Developer-surface coherence (network profiles)

### Task 2.1: Name the profiles; thread network config through `hc-start.sh`

**Files:** `app/elohim-app/scripts/hc-start.sh`, `app/elohim-app/CLAUDE.md`.

- [x] **Investigate first (one step, journaled):** how the pinned `hc sandbox generate`
      (hc-start.sh:196) accepts network config — CLI flags vs generated conductor-config
      (reference shape: `elohim/holochain/edgenode/conductor-config.yaml`). Record the answer in
      the plan journal before coding.
- [x] Add `NETWORK_PROFILE=isolated|join-alpha` (default `isolated` — today's behavior,
      byte-identical when unset). `join-alpha` threads `CONDUCTOR_BOOTSTRAP_URL`
      (default `https://doorway-alpha.elohim.host/bootstrap`) + `CONDUCTOR_SIGNAL_URL` into the
      conductor network config.
- [x] Document the THREE profiles (`isolated` / `live-data` / `join-alpha`) in
      `app/elohim-app/CLAUDE.md` Starting Development — one table, the agent's single orientation
      point. (`live-data` = L3's `start:alpha`, already landed.)
- [x] Verify: `NETWORK_PROFILE=isolated` run is unchanged (existing smoke path); `join-alpha`
      config renders the URLs (full join proof is Phase 3). Commit.

### Task 2.2: Deployed-DNA artifact sourcing (the parity rail)

**Files:** `app/elohim-app/scripts/fetch-deployed-dna.sh` (new), wired into `hc-start.sh`
`join-alpha` path.

- [x] **Investigate first:** where the DNA pipeline archives `.dna`/`.happ` artifacts
      (`elohim/holochain/dna/Jenkinsfile:602-641` builds them; find the archived-artifact URL —
      public Jenkins artifact paths per the pipeline-diagnostics skill). Journal the source of
      truth + how to pin the build that alpha actually runs.
- [x] Script: fetch the deployed bundle set to a cache dir; verify hashes (`hc dna hash` against
      the fetched files); refuse to proceed on `join-alpha` with locally-built bundles.
- [x] Verify: fetched-bundle hash list printed; `isolated` profile never touches the fetch path.
      Commit.

### Task 2.3: De-duplicate one seam: `local-stack.ts` consumes `hc-start.sh`

**Files:** `genesis/a2o/scripts/local-stack.ts`.

- [ ] Replace its re-implemented start/health/seed logic with: invoke `hc-start.sh` (or probe the
      ports it documents) + the SAME seeder entrypoint hc-start uses (`seed.ts`); keep the
      `steward`/`seed-sqlite.ts` mode as the documented exception with a comment saying why.
- [ ] a2o `pnpm test:api` smoke passes against the stack it starts. Commit.
      (devfile `start-doorway` hardcode: leave; add a deprecation comment pointing at hc-start —
      devfile edits propagate on workspace rebuild and are operator-visible.)

---

## Phase 3 — Stage B spike: the workspace joins alpha as a peer

**Precondition:** Phase 2 (profile + artifacts). All steps read-only toward others' data: the new
agent authors only its own test entries.

- [ ] **Spike join:** `NETWORK_PROFILE=join-alpha pnpm hc:start:conductor` → conductor up with
      fetched bundles; confirm peer visibility: alpha doorway `/health` `p2p.peerCount` increments
      OR agent-info appears via `/bootstrap` query. Journal latency + WebRTC/signal behavior from
      the pod (the empirical pass the spec calls for).
- [ ] **Prove agency:** the Che agent authors one DHT entry (its own source chain — e.g. a
      node-registry presence or minimal content entry per what the installed DNAs allow);
      a household peer (or the doorway projection) reads it back. Quote both sides' evidence in
      the journal.
- [ ] **Lifecycle doc:** `app/elohim-app/CLAUDE.md` join-alpha row gains: conductor data dir on
      the `/projects` PVC (key continuity), teardown etiquette (stop conductor; the agent key
      persists — do NOT mint a fresh key per session), one-peer-per-workspace rule.
- [ ] Check Stage-B boxes on the arc spec; re-decompose. Commit.

---

## Phase 4 — Stage C: delegated agency (consumes Z.D/slice2a rails; PAIRED verification)

### Task 4.1: Rail-readiness checkpoint (verify, don't trust)

- [ ] Verify what Stage C consumes, quoting evidence: (a) `delegates-compute` commitment
      create/accept via `/api/v1/commitments` round-trips on the local stack (`rea_commitments.rs:27-66`);
      (b) CID discipline: create returns / is keyed by `entry_hash` (`commitment_fetcher.rs`,
      the bounds-gate keying — the action_hash-as-CID trap); (c) where bounds enforcement stands
      in Z.D Sprint 1 / slice2a TODAY (read their gap ledgers; ask: can an event citing an
      out-of-scope commitment get rejected anywhere yet?).
- [ ] **Decision gate:** if bounds enforcement is not yet consumable, Stage C's full loop
      (Task 4.4) BLOCKS-ON-COMPOSE — journal it, finish 4.2/4.3 (SDK + vocabulary, which don't
      block), and hand the dependency to the slice2a/Z.D lane instead of rebuilding it here.

### Task 4.2: Scope vocabulary for the agent content-authorship instance

**Files:** extend the existing Z.D payload schema set (compose — same files/pattern as commit
b2380b899), NOT a new schema family. Source of truth: **Holochain DHT** — the `Mishpat::Commitment`
entry (existing type; gate verdict carried from the spec); these schemas describe its wire payload
only. No new table, no new entry type, storage rows remain projections keyed by `dht_anchor_hash`.

- [ ] Define the instance row: scope = content-authorship event class(es), bounds fields
      (content types, path/epic scope, max events per period, TTL), reciprocity (attribution +
      audit trail), composed from `rea-compute-commitment-primitive.md`'s generalization table.
- [ ] `pnpm run schema:test` green. Commit.

### Task 4.3: `CommitmentService` in `@elohim/rea-runtime` (TDD)

**Files:** `app/elohim-library/projects/elohim-rea-runtime/src/lib/commitment.service.ts` (+ test).

- [ ] Failing tests: `createCommitment` (with a `delegatesCompute(...)` builder),
      `acceptCommitment`, `queryCommitments`, `revokeCommitment`, transition guards
      (proposed→accepted→fulfilled/revoked; no backward transitions), **all keyed on
      CID = `entry_hash`** — a test that would FAIL if action_hash were returned as CID.
- [ ] Implement over `@elohim/storage-client` wire types + `/api/v1/commitments`; note in-code:
      fresh grants read via `ConductorCommitmentFetcher` semantics until `CommitmentCommitted`
      storage subscription lands (the 2a gap — slice2a's lane).
- [ ] `pnpm --filter @elohim/rea-runtime test` green. Commit.

### Task 4.4: The grant loop, exercised in BOTH planes (the arc's destination)

**Precondition:** Task 4.1 gate open. **Files:** one a2o feature + one sweettest.

- [ ] Mint: Matthew (Stage-A session, Phase-0 grant) creates a `delegates-compute`
      content-authorship commitment → recipient = the agent's identity (Stage-A registered
      account or Stage-B peer key) via `CommitmentService`.
- [ ] Exercise: the agent authors one content item within bounds; the act's `EconomicEvent`
      cites the commitment CID; audit trail readable (provider, recipient, scope, event).
- [ ] **Doorway-plane scenario:** a2o `.feature` (API or browser) — out-of-scope write rejected
      at the doorway/storage gate; in-scope write accepted. Tag per a2o conventions.
- [ ] **Peer-plane scenario:** sweettest — validation rejects an event citing a revoked/absent
      commitment with zero doorway in the loop. Same verdicts as the doorway plane — any delta
      is a truth-layer bug (file it as such, don't paper over).
- [ ] Revoke: provider revokes; both planes now reject; journal the paired evidence. Commit
      (scenario + implementation together, story-first rule).

### Task 4.5: Provenance decision + Stage-A retirement

- [ ] Decide (journal a one-paragraph ADR in the arc spec): is performer-key visibility enough,
      or does the agent's account carry an "agent-operated" `Attestation` (existing type)?
      Implement only if the decision says attestation.
- [ ] Update `genesis/a2o/CLAUDE.md` etiquette: routine agent writes go through the Stage-C
      grant path; Stage-A credentials remain a test-fixture path only.
- [ ] Check Stage-C boxes on the arc spec; re-decompose spec + this plan; `placement-audit.py
      --ledger` reflects the drained items. Final commit.

---

## Execution log (2026-06-10, subagent-driven, two-stage reviews)

- **0.4** operator-viewable eyes: 388454d4f + lint-clean follow-up; serving verified (200/200/403).
- **1.1** DoorwaySessionClient: a91dc88 + f205533ac; 67 tests; spec ✅ / quality APPROVED (reviewers re-ran suites).
- **1.2** a2o migration: 0026de6b1; 107 unit / 514 dry-run scenarios; spec ✅ / quality APPROVED. Two LOW
  behavioral notes for future step-authors: `exchangeSession()` now PERSISTS the exchanged session in the
  shared store (old code was read-only); `logout()` clears the local session even when the server call fails.
  ESM/CJS interop at the import site is load-bearing until `elohim-identity-type-module-esm-interop.md` lands.
- **2.1** NETWORK_PROFILE: 0c61c5d00; isolated wrapper byte-identical (cmp); spec ✅ incl. `.hc_ports` guard
  caller-trace; quality pass controller-direct. Investigation's clap order was WRONG — corrected empirically
  (HAPP positional precedes the `network` subcommand).
- **2.2** fetch-deployed-dna: bc66edd35 + ae65418d2 (hardening: password-stdin, temp trap, curl-exit
  distinction); live-fetched the deployed bundle (Jenkins fallback; oras absent); 5 deployed DNA hashes printed;
  spec ✅ / quality APPROVED.
- Backlog captures (in-flight discipline): hc-start-storage-dir-dead-override, hc-seed-ports-file-path-drift,
  join-alpha-skips-local-dna-build, elohim-identity-type-module-esm-interop, angular-auth-onto-doorway-session-client,
  doorway-auth-view-schema-contract.
- Grant-gated (operator): Phase 0 Tasks 0.1-0.3; Task 1.2's alpha-target `@browser` + `look --as` re-verify.

**Phase 0 completion (2026-06-10):** grant applied via the permissions dialog (operator's first use —
prompt-grants for the look-as-Matthew family); `look --as Matthew` verified against live alpha
(`ok:true, as:Matthew`, via the MIGRATED DoorwaySessionClient path — typed-error login would have
thrown), closing the arc Stage-A verification AND the 1.2 deferred re-verify box. Findings:
`/dashboard` is not a deployed route (L1-plan example URL was fictional; rendered Page Not Found);
the manifesto `/db/content/manifesto` 403 REPRODUCES authenticated-as-admin-fixture (strengthens
`backlog/alpha-manifesto-content-403.md` — not an anon-only reach gate); the Welcome surface renders
identically logged-in vs anonymous (no session chrome — observation only). L3's Hygiene-#4
(auth through the localhost:4200 proxy) remains genuinely OPEN — this verification was direct-to-alpha.

## Self-Review

- Composes, never rebuilds: bounds rails (Z.D Sprint 1 / slice2a) consumed behind an explicit
  readiness gate (4.1); SDK homes follow canon §3.4/§3.5; no new DHT entry types anywhere
  (p2p-design-gate verdict carried from the spec).
- Byte-identical defaults: `NETWORK_PROFILE` unset = today's behavior; `isolated` never fetches.
- Paired verification is structural (4.4 has both planes as separate checkboxes).
- Investigation-before-code steps where the survey flagged unknowns (hc sandbox flags, artifact
  URLs).
- Complementary work captured to backlog (Angular auth migration, auth schema contract), not
  absorbed.
