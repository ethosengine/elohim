---
id: "backlog-holochain-docs-island-harvest-residue"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Holochain docs island harvest residue — conductor Origin-header trap + Community Web Gateway narrative + Playground rail + two open questions"
slug: "holochain-docs-island-harvest-residue"
written: "2026-06-11"
author: "holochain docs island recompose (ops-guides liveness harvest, code-verified)"
status: "backlog"
priority: "low"
tags: [holochain, island-recompose, harvest, ops, conductor, doorway, devfile, storyteller]
derived_from:
  - elohim/holochain/docs/claude.md               # retired to git 2026-06-11 (holochain docs island recompose; legacy pre-reorg gospel, 1195 lines)
  - elohim/holochain/docs/DEPLOYMENT-RUNTIMES.md  # retired to git 2026-06-11 (holochain docs island recompose)
  - elohim/holochain/docs/DEVELOPMENT.md          # retired to git 2026-06-11 (holochain docs island recompose)
cites:
  - .claude/skills/hc-dev-orchestrator/SKILL.md
  - doorway/CLAUDE.md
  - elohim/holochain/edgenode/conductor-config.yaml
  - devfile.yaml
  - app/elohim-app/scripts/hc-start.sh
  - genesis/data/timeline/backlog/deprecation-devfile-start-doorway-dead-command-retire.md
  - genesis/docs/content/elohim-protocol/architecture/2026-06-11-doorway-two-axis-scaling.md
shift_objective: |
  Place the three LIVE-BUT-UNHOMED residue items from the retired holochain ops docs: (1) add the
  conductor WebSocket Origin-header trap to hc-dev-orchestrator troubleshooting (and/or doorway
  gospel for deployed); (2) hand the Community Web Gateway narrative to the storyteller as a
  story-tier candidate (do NOT bless as architecture); (3) decide the Playground rail's fate — one
  troubleshooting line or let it die — and flag the orphaned devfile ui-playground endpoint for the
  operator-gated devfile-cleanup pass. Resolve the two open questions only when their areas are
  next touched.
---

# Harvest residue from the retired holochain docs/ ops guides

Code-verified residue from the retired `elohim/holochain/docs/{claude.md, DEPLOYMENT-RUNTIMES.md,
DEVELOPMENT.md}` (git history). Everything else still-true in those files is already homed
(hc-dev-orchestrator + seed-workflow + holochain-import skills, doorway gospel, access-tier
patterns, session-bridge design, resilience README Parts V/VI, two-axis scaling, D1-D5 canon).
The socat PTY rationale — the fourth residue item — was placed directly as a comment block above
`app/elohim-app/scripts/hc-start.sh:306` in the same recompose (it survives with the code). Only
the items below are homed nowhere live.

## 1. Conductor WebSocket Origin-header trap (ops)

The conductor returns **400 "Missing `Origin` header"** on WebSocket connects without one.
Browsers send it automatically, so the trap only bites CLI/script testing — `wscat` needs
`--origin https://...`. Live allowed-origins config: `elohim/holochain/edgenode/conductor-config.yaml:10-23`
and `genesis/orchestrator/manifests/edgenode/prod.yaml:42`. The operational trap is documented
nowhere live (grep of doorway gospel, edgenode README, skills — only the config lists exist).
**Suggested home**: hc-dev-orchestrator SKILL.md §Troubleshooting (local) and/or doorway/CLAUDE.md
(deployed).

## 2. Community Web Gateway narrative (story-tier)

The retired claude.md (:669-709 git) elaborated multi-community DNS gateways as curated views over
one DHT — `localchurch.org`/`neighborhood.net` as SEO-able Stage-1 presence — plus the
bus-factor/"infrastructure that outlives individuals" stewardship-succession story. The core
concept is homed (two-axis scaling :83 — node steward may run a doorway for their own community;
the original doorway's role shrinks to DNS/recovery). The narrative elaboration appears nowhere in
genesis/docs (grep "Community Web Gateway" = 0 hits). **Storyteller candidate** — adjacent to the
patron-CDN succession themes in resilience Part VI; do NOT bless as architecture.

## 3. Holochain Playground rail (recorded, not blessed)

`npx @holochain-playground/cli ws://localhost:8888/admin` (+ a headless `xdg-open` stub) was the
documented visual-DHT-introspection rail. Zero live consumers anywhere. The devfile still carries a
`ui-playground` endpoint on port 4201 (`devfile.yaml:179`) — orphaned/repurposed, since 4201 now
serves `pnpm reports:serve` (Frontend Eyes). Either add one troubleshooting line to
hc-dev-orchestrator ("visual DHT introspection exists via @holochain-playground/cli; the devfile
endpoint name is stale") or let the rail die — and in both cases the stale devfile endpoint joins
the operator-gated devfile-cleanup pass already queued by
`deprecation-devfile-start-doorway-dead-command-retire.md`.

## Flags + open questions (resolve when the area is next touched)

- **Edgenode docs are stale siblings, NOT this island**: `elohim/holochain/edgenode/README.md`
  (holostrap diagram, docker-compose quick start), `edgenode/conductor-config.yaml:6` (header
  comment claims holostrap while the body uses doorway URLs), `edgenode/docker-compose.yml`, and
  `doorway/doorway-service/ARCHITECTURE.md` still mention the dead `holostrap.elohim.host`
  bootstrap/signal hosts (live: `doorway-alpha.elohim.host/bootstrap` +
  `wss://signal.doorway-alpha.elohim.host`). The edgenode docs island should get its own pass.
- **OPEN QUESTION (branch→build-config mapping)**: DEPLOYMENT-RUNTIMES claimed dev/feat-*/claude/*
  → alpha, staging → staging, main → production. No `--configuration=alpha/staging/production` or
  `BUILD_CONFIG` strings exist in the root Jenkinsfile (grep 2026-06-11). Re-derive from the app
  pipeline before restating the mapping anywhere.
- **OPEN QUESTION (ContributorPresence home)**: the claimed/unclaimed/stewarded lifecycle concept
  is homed (rea-economics skill, shefa domain gospel §Cross-Pillar Coupling) but the TYPE exists
  only in `app/lamad/src/app/models/` + generated types — zero hits in `elohim/sdk/domains/`.
  Whether it lands as an sdk-domain type or stays an app-layer notion is undecided — do not invent
  a home.
