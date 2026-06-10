---
id: che-live-peer-dev-loop-design
title: Che Browser Feedback — L3 Live-Peer Dev Loop, local UI × deployed alpha data
status: Draft
class: process-meta
topic: [che, browser, playwright, look, alpha, doorway, dev-proxy, live-data, agentic-eyes]
process_subdomain: build-and-test
cites:
  - che-browser-feedback-foundation-design | L1 of this series — the look primitive + Che browser wiring this loop renders through; L3 refines it with a live-data backend | sha256:db154b7bbc93ba3b | path: genesis/docs/superpowers/specs/2026-05-30-che-browser-feedback-foundation-design.md
  - che-browser-completion-oracle-design | L2 of this series — the visual done-gate; L3 rail #3 keeps gates on deterministic fixtures, live-peer loop is polish/diagnosis only | sha256:355cc8523a03f33b | path: genesis/docs/superpowers/specs/2026-05-30-che-browser-completion-oracle-design.md
  - doorway-access-tier-patterns | canonical doorway access model — governs what an unauthenticated/fixture-authed dev proxy may read from alpha and why writes are deliberate acts | sha256:f862d55525b442c3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md
informed-by:
  - genesis/docs/superpowers/specs/2026-05-30-che-browser-feedback-foundation-design.md
refines: genesis/docs/superpowers/specs/2026-05-30-che-browser-feedback-foundation-design.md
---

# Che Browser Feedback — L3 Live-Peer Dev Loop: local UI × deployed alpha data

> Spec 3 in the Che Browser Feedback series. L1 gave the agent eyes (`look` + the browser wiring);
> L2 gave the `/shift` loop a visual done-gate. Both are **landed and re-verified working**
> (2026-06-10: `pnpm look https://doorway-alpha.elohim.host/` → `ok:true`, screenshot read by the
> agent). L3 closes the last gap in the polish loop: seeing **in-progress local UI code** rendered
> against **live deployed-peer data**, with no local stack and no mocked data.

## The gap

After L1/L2, the agent can see two things cheaply:

| Surface | Data | Cost |
|---|---|---|
| Deployed alpha app (`look https://doorway-alpha…`) | live | free — but renders *deployed* code, not your edit |
| Local dev server (`look http://localhost:4200`) | local | requires `hc:start[:seed]` — conductor + storage + doorway + seeding, the "too big a pain" path |

The polish loop needs the cross-quadrant: **local code × live data**. The seam is one line — the
Angular dev proxy hardcodes `http://localhost:8888`. Everything else already exists: the app's
Che strategy routes all API calls same-origin through the dev proxy, and the alpha doorway is
reachable from the Che pod (verified 54 ms, `/health` 200, conductor connected, 9 peers).

## Design

A third, additive proxy target — no change to the default local-stack path:

- **`app/elohim-app/proxy.conf.alpha.mjs`** — same contexts as `proxy.conf.mjs`
  (`/api /db /blob /apps /epr-head /account /health /p2p`), `target` =
  `process.env.DOORWAY_TARGET ?? 'https://doorway-alpha.elohim.host'`, `secure: true`,
  `changeOrigin: true`.
- **`pnpm start:alpha`** in `app/elohim-app/package.json` — `ng serve` with the alpha proxy.

The loop this enables (the "eyes + hands" cycle for UI work in Che):

```
pnpm start:alpha                 # once; HMR keeps it live
  ── edit component/template ──▶ HMR rebuild (seconds)
  ── pnpm look http://localhost:4200/<surface> [--wait-testid …]
  ── Read shot.png + capture.json ──▶ judge, adjust, repeat
```

### Feasibility — proven by spike, 2026-06-10

- Local `ng serve` on :4201 with the alpha proxy rendered the Welcome surface **pixel-equivalent
  to deployed alpha** (`ok:true`, title matches).
- Live data flows: `curl localhost:4201/health` returns alpha's live health JSON (same `node_id`,
  conductor connected); `/api/v1/cache/stats` → 200.
- The spike immediately paid for itself by catching three real findings (backlogged — see below).

## Hygiene rails (the "agentic developer hygiene" half of the ask)

1. **Read-mostly discipline.** Alpha is shared deployed state owned by real peers. The default
   loop is unauthenticated GET renders. Authenticated flows (`look --as <FixtureHuman>`) against
   alpha are deliberate acts; never drive seeding, bulk writes, or destructive flows through this
   proxy. The repo-manifest rule applies: alpha state is operator-owned.
2. **Write guard (open).** Add a proxy-level guard rejecting non-`GET/HEAD/OPTIONS` unless
   `DOORWAY_ALLOW_WRITES=1`. Needs verification that the Angular 19 Vite-based dev server honors
   `bypass`/`configure` in proxy config; if not, document the convention rail instead.
3. **Gates pin to deterministic data.** Live-alpha content drifts; L2's visual *done-gate*
   (`validatedRegressed == 0`, two consecutive renders) keeps running against the local-stack /
   fixture path. The live-peer loop is for **polish and diagnosis**, not assertions.
4. **Auth'd session through the proxy (open).** `--as` login via `localhost:4200` → alpha sets
   cookies across the proxy boundary; `changeOrigin` should cover it, but Secure/SameSite flags
   need one verification pass before the auth-aware loop is documented as supported.

## Deliverables

- [x] `app/elohim-app/proxy.conf.alpha.mjs` — alpha-target proxy config (landed with this spec)
- [x] `app/elohim-app/package.json` `start:alpha` script (landed with this spec)
- [x] Feasibility spike: local render pixel-equivalent + live data through proxy (2026-06-10)
- [ ] Write guard or documented convention rail (Hygiene #2)
- [ ] One verification pass of `look --as <FixtureHuman>` against `localhost:4200` (Hygiene #4)
- [ ] Propagate the loop into gospel surfaces: `app/elohim-app/CLAUDE.md` (Starting Development)
      and `genesis/a2o/CLAUDE.md` (`look` Tools bullet gains the live-peer recipe)

## Captured complementary findings (NOT in scope — backlogged)

The first three renders of the spike surfaced real issues, filed in
`genesis/data/timeline/backlog/`:

- `welcome-fullpage-gradient-inflation.md` — `/` full-page render is ~14,000 px of mostly empty
  gradient between hero and footer (identical local and deployed → app property, not deploy artifact).
- `hardcoded-localhost-8888-health-probe.md` — something client-side requests the absolute URL
  `http://localhost:8888/health`, bypassing the same-origin proxy strategy.
- `look-capture-4xx-response-urls.md` — `capture.json` console entries show `404` without URLs;
  `PlaywrightDevice` captures `requestfailed` but not 4xx/5xx responses.
- L1/L2 plan ledgers are stale (all items OPEN despite landed + verified) —
  `che-browser-plans-ledger-stale.md`.
