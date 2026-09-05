---
title: "Hosted-human lifecycle E2E — register, live, close, from the portal, leaving the doorway clean (MVP, no graduation)"
id: hosted-human-lifecycle-e2e-plan
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
domain: D2
habits: [hosted-human-lifecycle]
graduation-trigger: every `- [ ]` task below is checked with its evidence line in the habit atom's ledger, and `@concern:hosted-human-lifecycle` passes on the household mesh with no scenario left @wip (the habit's runnable check)
topic: [auth, hosted, agency-phase, doorway, provisioning, account-closure, a2o]
informed-by:
  - genesis/a2o/features/auth/hosted-human/05-leaving.feature (the finish line and its stations)
  - genesis/a2o/features/browser/doorway-portal-login.feature (the sign-in station this plan reuses, steps already defined)
  - genesis/a2o/features/auth/agency-pipeline-coherence.feature (the account-page pipeline contract)
  - genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md (the authority chain this MVP stops short of)
cites:
  - genesis/a2o/features/auth/hosted-human/05-leaving.feature
  - doorway/doorway-service/.epr-meta/hosted-human-lifecycle.habit.md
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/conductor/provisioner.rs
  - doorway/doorway-service/src/routes/admin_users.rs
  - doorway/doorway-service/src/routes/admin_conductors.rs
  - doorway/doorway-app/src/app/components/account/doorway-account.component.ts
  - doorway/doorway-app/src/app/components/register/threshold-register.component.ts
  - genesis/a2o/steps/ui/doorway-portal-login.steps.ts
  - elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
---

# Hosted-human lifecycle E2E — MVP plan

**Habit served:** `hosted-human-lifecycle` (declared `red` 2026-09-04 in `doorway/doorway-service/.epr-meta/`).
**Scope fence:** the hosted stage only. Nothing here exports a key, installs an app, or confirms stewardship. Graduation is a later story.
**Operator decision this plan does NOT make:** whether the fleet keeps `DEV_MODE=true`. Task 2 removes provisioning's dependence on that flag so the decision no longer changes whether a registrant gets a cell.

## 1. What the 2026-09-04 baseline showed

- A fresh `POST /auth/register` on doorway-alpha answered with the operator's own profile (display name, bio, affinities). Every deployed doorway runs `DEV_MODE=true`; the hosted branch of `handle_register` skips `provision_agent` under it and calls `create_human` on the singleton conductor, which reports "agent already has a Human profile", and the handler recovers that existing Human. Every hosted registrant shares one human id and one agent key.
- Closing an account exists only as `DELETE /admin/users/{id}` — a soft-delete of the credential row. No cell is uninstalled, no session ends, no self-service surface exists in either portal.
- The a2o lifecycle story stops at logout; its After-hook does a best-effort admin soft-delete, so cleanup is a harness side effect rather than a product behaviour, and the fleet accrues one throwaway account per pipeline run (the Act II portal-login twin says so in its own preamble).
- The provisioner already has both halves of the reclaim primitive: `provision_agent` (find-or-install on the least-loaded pool conductor) and `deprovision_agent` (uninstall the app, unregister the agent).

## 2. P2P Design Gate: hosted-human account closure

### Entity: HostedAccount (doorway credential row)
- **Classification**: Ephemeral (C) — doorway-operational. The doorway is the federation layer riding over the substrate; the row is the doorway's own record of whom it hosts.
- **Justification**: no peer validates it; if lost, the human re-registers (their notarized Human entry survives on the DHT). Already exists (`UserDoc` in Mongo); this plan adds no field beyond reusing `is_active` / `metadata.is_deleted` and a closure timestamp.
- **Network Stakes**: all stages; no floor-protected cost (nothing here is constitutional or a relationship reach).
- **Content Address Strategy**: Slug — the gateway-scoped identifier (`localpart@gateway`). Justified: operational entity keyed by the credential a human types; the notarized identity (agent key) is a separate field and is never the row's identity.
- **Source of Truth**: doorway operational store.
- **Integrity / Coordinator zome**: none — no DHT entry.
- **Projections**: none beyond the doorway's own `GET /auth/account`.
- **HTTP Route**: `POST /auth/close-account` on doorway-service. Doorway-specific by the `doorway/CLAUDE.md` criterion: every resource reclaimed (credential row, sessions, transfer tokens, pool cell) is owned by the doorway, so this is not substrate logic wearing a doorway costume.
- **Anti-pattern check**: no UUID as notarized identity (the row is not notarized); no per-host authoring of a notarized field; identity namespaces kept apart (agent key vs identifier vs conductor registry entry — joined only through the registry's own agent→app mapping).

### Entity: HostedCell (installed app on a pool conductor)
- **Classification**: Ephemeral (C) — a conductor-side resource tracked in the doorway's `ConductorRegistry`, reconstructable by re-provisioning.
- **Justification**: the cell is the compute the doorway lends; the human's source chain inside it is the human's (Private/B, already governed by the conductor). Uninstalling the app on closure is what "the doorway stopped hosting" means; the DHT keeps what the cell published.
- **Content Address Strategy**: Agent-scoped composite in practice — `(agent_pub_key, conductor_id, installed_app_id)`; the registry already keys on the agent key.
- **Source of Truth**: the conductor's admin interface (`list_apps`), mirrored by the registry.
- **HTTP Route**: none new. The story's "no pool conductor holds a cell" assertion reads the existing admin conductor listing; Task 4 confirms which route and adds the per-agent lookup if it is missing.

### Entity: AccountClosure marker (deferred, not in the MVP story)
- **Classification**: Notarized (A), reusing the existing `governance-action:key-revocation` Content entry on the elohim DNA, written through imagodei's `create_self_revocation` (coordinator-only; DNA-hash-NEUTRAL if a new `reason` value is added to `REVOCATION_REASONS`).
- **Head-plane cost**: one entry per closed account; bounded by closures (tens per year on alpha, well under the ~500 bundling threshold).
- **Why deferred**: the MVP finish line is doorway cleanliness judged from the UI. The revocation is the right notarized trace and is Task 7, after the story is green, so that a failing zome bridge never blocks a human from closing their account.

### Design constraints discovered
- Ordering inside closure must be reclaim-safe and idempotent: end sessions → uninstall cell → mark row closed. A second call on a closed row answers "already closed" (200 with `alreadyClosed: true`), never 404, so the story's idempotency station holds.
- Closure must be authorised by the human's own bearer AND a typed confirmation of their identifier; the admin soft-delete stays admin-only and unchanged.
- Provisioning must not depend on `dev_mode`. The correct predicate is "a conductor pool is configured" (`state.conductor_registry.is_some()`). `dev_mode` keeps its meaning for auth posture only.
- The fixture After-hook in `auth-lifecycle.steps.ts` should call the same close route the human uses once it exists, so every a2o registration is its own cleanup and the Act II portal-login twin's "not swept afterwards" caveat retires.

## 3. Tasks

Each task names its evidence. A task is checked only with the evidence line written into the habit atom's ledger.

- [ ] **Task 1 — Step definitions for the story (a2o).** New file `genesis/a2o/steps/ui/hosted-human-lifecycle.steps.ts`. Reuse from `doorway-portal-login.steps.ts`: "the browser opens the doorway sign-in portal", "the human signs in through the portal", "the portal shows a sign-in error", "the doorway confirms a session for that human", "the doorway confirms no session for that human", "a hosted human is registered on doorway". Define: registration portal open/render, "creates an account through the portal with the display name", "the doorway names that human", "holds a cell / no pool conductor holds a cell / that cell belongs to no other account / two humans hold different cells" (read the doorway's admin conductor listing through the existing admin client), "opens their doorway account page", "agency pipeline marks … current / no later step completed", "closes their account through the portal / begins closing / confirms with …", "returns to the signed-out doorway landing", "the doorway's account store holds no active account", "a second browser registers another newcomer", "has closed their account", "the same closure is requested again". Every selector must be a real `data-testid` (page-model skill). Evidence: `npx cucumber-js --dry-run features/auth/hosted-human/05-leaving.feature` reports 0 undefined steps; remove `@wip` only from scenarios whose steps all exist.
- [ ] **Task 2 — Provisioning no longer gated on `dev_mode` (doorway-service).** In `handle_register` (hosted branch) and `handle_login` (auto-provision on first login), replace the `!state.args.dev_mode` conjunct with "registry configured". Keep the no-registry fallback (singleton `ZomeCaller`) exactly as it is for a doorway with no pool. Unit test: a pure predicate `should_provision(registry_configured, dev_mode) -> bool` pinned true for `(true, true)`. Evidence: `just gate doorway` green; on the household mesh, registering two humans through the portal yields two distinct agent keys (Task 1's "two different people" scenario).
- [ ] **Task 3 — `POST /auth/close-account` (doorway-service).** Bearer-authorised; body `{ "confirmIdentifier": "<identifier>" }`; mismatch → 400 `CONFIRMATION_MISMATCH` and nothing changes. On match: (1) revoke every session-transfer token and OAuth code for the human, drop the custodial key from the session cache; (2) if the row has a conductor assignment, `deprovision_agent` (uninstall + unregister), failure logged and reported in the response, not fatal; (3) set `is_active=false`, `metadata.is_deleted=true`, `closed_at=now`. Response `{ closed: true, cellUninstalled: bool, alreadyClosed: bool }`. `handle_login` already refuses inactive rows; `handle_me` already refuses suspended rows — assert both in a unit test. Declare the route in the auth discovery document (`auth_discovery.rs` — add `closeAccount`, and the `AUTH_OWNED_PATHS` symmetry guard in `server/http.rs`). Evidence: `just gate doorway` green; contract test for the discovery document updated.
- [ ] **Task 4 — Cell visibility for the story (doorway-service, read-only).** Confirm the admin conductor routes expose "which conductor, if any, holds an app for agent X" (`admin_conductors.rs`); if only per-conductor listings exist, add `GET /admin/conductors/agents/{agentPubKey}` returning the registry entry or 404. The story's pool-clean assertion reads this. Evidence: unit test + the story's "no pool conductor holds a cell" step green on the mesh.
- [ ] **Task 5 — Close-account surface in doorway-app.** On `/threshold/account`, a "Close this account" section (below the graduation CTA, shown for every signed-in human): explains what is reclaimed and what the network keeps, asks the human to type their identifier, calls the route, then signs out locally and navigates to `/threshold/` (`data-testid`: `account-close-begin`, `account-close-confirm-input`, `account-close-confirm`, `account-close-error`). Also show the display name from `GET /auth/account` if the wire carries it; if it does not, add `displayName` to `AccountResponse` (doorway-service) since the story asserts the name the human typed. Evidence: doorway-app vitest green; `pnpm look http://localhost:8888/threshold/account --as <fixture>` shows the section.
- [ ] **Task 6 — Harness cleanup uses the product path.** In `genesis/a2o/steps/auth-lifecycle.steps.ts`, the After-hook for API-registered ephemeral humans calls `POST /auth/close-account` with that human's bearer (falling back to the admin soft-delete only if the route is absent). Update the preamble of `features/browser/doorway-portal-login-neighbourhood.feature` to retire the "not swept afterwards" caveat. Evidence: a full auth-lane run on the household mesh leaves the doorway's user count unchanged before/after (assert in the run log).
- [ ] **Task 7 — Notarized closure marker (deferred hardening).** After Tasks 1–6 are green: closure step (1½) calls `create_self_revocation` on the human's cell with reason `account-closed` (add to `REVOCATION_REASONS`, coordinator-only) BEFORE uninstalling; response carries `revocationCid`. New station scenario "closing leaves a revocation the network can see". Evidence: sweettest or mesh run showing the `governance-action:key-revocation` entry for the closed human's key.
- [ ] **Task 8 — Flip the habit.** Run `just test mesh features/auth/hosted-human/05-leaving.feature` with no `@wip` left; write the one-line delta into the habit atom; `python3 .claude/scripts/habits-project.py`; status `unwired → red` at the first real run and `red → green` on a passing run. Fleet confirmation comes later from an edge build carrying the same commit, run with the cluster-state override and `A2O_ALLOW_DESTRUCTIVE=0`.

## 4. Sequencing and ownership

Tasks 1, 2, 3, 4 are independent and can run in parallel on three workers (a2o; doorway-service Tasks 2+3; doorway-service Task 4 + doorway-app Task 5). Task 6 follows 3. Task 7 follows 6. Task 8 closes.

Prerequisite already in flight: the 2026-09-04 fix branch (threshold-register camelCase keys, register validation ordering, hosted-steward arm in the agency service, account-page hosted-steward banner). Land it first; Task 5 builds on the same account component.

Out of scope, named so nobody re-derives it here: the DEV_MODE fleet posture decision; the elohim-app side of the arc (the agency badge after OAuth sign-in already has its own story in `agency-context-labels.feature` and is blocked on the shell incident today); anything past the Hosted stage.
