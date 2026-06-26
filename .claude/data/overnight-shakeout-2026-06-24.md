# Overnight shakeout — 2026-06-24

Autonomous session. Goal: land eve-fix, integrate loose changes, shake out frontend
console/nav/link issues, flip genesis stages to success, /shift the pipeline.

## TL;DR — the big finding

**Almost every "frontend error" on alpha is deploy-lag or stale-seed state, NOT a code bug.**
The code on `dev` is correct; alpha is behind. The genuine repo bugs were: the eve-fix
(landed), the `look` tooling default (landed), and two a2o test-drift fixes. The remaining
genesis-green blockers are **operator-owned seeding/deploy actions** (listed below) — they
cannot be fixed from the repo.

## 1. eve-fix — LANDED on dev (`fb3fea6fd`)

Root cause (operator-diagnosed, code-confirmed): the embedded conductor's admin-websocket used
holochain_client's default 60s request timeout; the cold first hApp install (single-threaded
wasm compile + genesis) exceeds 60s on shem's per-core speed → `install_app ... Websocket error:
Timeout` → crash-loop (eve, 25 restarts, >12h). Warm-cache peers (incl. required-remote adam)
were unaffected.

Fix: `HAPP_INSTALL_TIMEOUT_SECS` env (default 180), threaded into `ConductorManager` via
`AdminWebsocket::connect_with_config`; wired into the edgenode template + adam's manifest.
`cargo check` green. Pushed to dev → orchestrator #1304 dispatched **elohim-edge** (deploys it →
heals eve on next cold install) + **elohim-genesis**.

- **Immediate heal option (operator):** pre-seed eve's empty PVC with the compiled wasm cache
  from a healthy same-node peer (caleb, also on shem). Otherwise eve heals when the edge
  deploy lands (cold install now gets 180s, completes once, cache warm forever).

### POST-DEPLOY VERDICT (edge #1110 landed UNSTABLE, ~05:00Z)

The fix deployed (all nodes now on `elohim-storage:1.0.0-dev-fb3fea6f`) and **healed 3 of the
4**: adam, nancy, terrance are `ready=1` (warm caches + the 180s budget let them settle). **eve
is still crash-looping** (`reason=Error`, `ready=0`, fresh pod restarting). Probe config rules
out a probe-kill (readiness initialDelay 120s; liveness 180s + 5×30s = 330s window); the binding
constraint is the **`install_app` 180s timeout itself** — eve's cold wasm-compile under shem
contention (11 conductors cold-started together) **exceeds 180s**, so storage exits before
install commits. The fix is correct; the budget is just still too small for eve's cold path.
**Two clean cures (operator):** (1) **PVC wasm-cache pre-seed** from caleb — immediate,
guaranteed (sidesteps the cold compile); or (2) **bump `HAPP_INSTALL_TIMEOUT_SECS` 180→300** in
the edgenode template + adam manifest (operator-tunable, one line; redeploy) — 300 sits safely
under the 330s liveness window. Recommend the pre-seed for eve now + raising the shipped default
to 300 for durability.

### Navigation audit (completing the frontend ask)

Rendered the real top-level routes (`/community`, `/map`, `/shefa`, `/doorway/elohim`,
`/avodah/projects`): all render real content (no in-app 404s) — **navigation is healthy on
alpha**. Only the global wasm deploy-lag 404 + one minor route-specific
`404 /api/v1/economic-events/appreciations?for=current` on `/shefa`. The bare routes my first
sweep guessed (`/learn`, `/explore`, …) correctly 404 — they're child/lazy paths, not real
top-level routes; not app bugs.

## 2. Frontend console/nav audit (16 public routes rendered via `look`)

No JS exceptions anywhere. Findings, all root-caused via parallel investigators:

| Error | Verdict | Action |
|---|---|---|
| `404 /wasm/elohim-cache-core/elohim_cache_core.js` (every route) | **Already fixed on dev** (3caa81b7d — `preferWasm:false` gate); app-image deploy lag | redeploy app to alpha |
| `404 /version.json` | **Not a bug** — app serves it from its OWN host (alpha.elohim.host nginx); I hit the doorway host | none |
| `404 /api/v1/epr/elohim-host-landing/nav-context` | Route works; **seeding gap** — no seeder writes an `epr_atoms` row for the landing; app degrades gracefully | seed epr_atoms atom |
| `403 /db/content/manifesto` | Repo source/seeder/gate all correct; **live alpha row has stale non-commons reach** | re-seed manifesto |

Navigation: app uses nested per-pillar routes (`/identity/*`, `/shefa/*`, `/community/*`,
`/avodah/*`) + cross-bundle `/lamad/*` (served by doorway, `<lamad-root>`). SPA-fallthrough means
every path is HTTP 200; "404" is client-side (`NotFoundComponent`). The bare top-level routes I
guessed (`/learn`, `/explore`, etc.) correctly 404 — they're child/lazy paths, not my-bug. No
broken app-authored nav links found among the real routerLink targets.

## 3. Genesis CI a2o failures (#1195 UNSTABLE: 71 scenarios — 41 failed, 9 undefined, 20 passed)

(#1196 was a transient git-checkout network flake, not tests.) Genesis runs a2o AGAINST alpha, so
it sees the same stale-seed state. Failure clusters:

- **Seeding-caused (operator):** `doctrinal docs manifesto/... seeded as commons markdown EPR`
  (exploration-sidebar.feature) — fails because the live alpha `manifesto` row reach is
  `community` not `commons` (same as the 403 above). The other three docs serve 200.
- **Test-drift (REPO — fixed below):** assessment-start selector; feedback-gate Lit migration.
- **Distribution (`content-alpha` to ≥2 households):** needs multi-household — env precondition.

## 4. Repo fixes made this session

- **`fb3fea6fd`** eve-fix (dev). 
- **`look` E2E_DOORWAY_ALPHA default** (`genesis/a2o/package.json`) — `look --as` no longer
  crashes (exit 2 → 0). VERIFIED.
- **discovery-assessment step** (`steps/ui/discovery-assessment.steps.ts`) — `I start the
  assessment` no longer hard-waits the pre-assessment-only `assessment-start` gate; clicks it
  if present then waits for `sophia-question` (matches the working `navigateToQuizStep`).
- **feedback-gate** (PENDING verification) — `selectors.ts` `ARTIFACT_TEXTAREA`
  `artifact-textarea`→`feedback-modal-textarea` and `ARTIFACT_SUBMIT`
  `artifact-submit`→`feedback-modal-submit`; steps rewrite the `<dialog>`/`:modal` assertions
  (component is now a Lit `<div class="modal-overlay">` in shadow DOM) to Playwright
  locators + overlay/z-index checks.

## 5. OPERATOR ACTIONS to flip genesis green (cannot be done from repo)

1. **Re-seed alpha `manifesto` reach** `community`→`commons` (`--content-only --ids=manifesto`;
   the stampProvenance PATCH reconcile already exists). Fixes manifesto 403 + exploration-sidebar
   a2o failures.
2. **Seed an `epr_atoms` atom for `elohim-host-landing`** (no seeder writes it today). Fixes
   nav-context 404. (Repo improvement candidate: add the atom step to the landing seeder.)
3. **Redeploy the app image to alpha** so the `preferWasm:false` fix ships (kills the wasm 404).
4. **eve:** PVC wasm-cache pre-seed for immediate heal, or let edge #1304 deploy heal it.

## 6. Loose changes (not swept — separate strands)

- `.claude/memory/*` (per-host deploy-lag correction) + the PII plan doc — clean docs; safe to land.
- Submodules `rakia`/`sophia`/`holochain-conductor` — **uncommitted changes INSIDE each** (a
  build-manifest fixture snapshot; a Jenkinsfile no-op; the jemalloc Cargo.lock). Must be
  committed inside each submodule first; do NOT fold the `-dirty` gitlinks into the monorepo.
- `.claude/data/*` ledgers — machine state.

## 7. Known gap

`look --as <human>` runs but the fixture-login session isn't carried into the rendered page
(redirects to `/identity/login`, `401 /auth/me`) — a deeper look.ts session-injection fix.
`test:browser` uses the full fixture-login path which DOES carry the session, so genesis browser
scenarios authenticate fine.
