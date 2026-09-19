---
name: holochain-hdk-0-7
description: Reference for writing and AUDITING Holochain hApps on hdk 0.7.0 / hdi 0.8.0 — coordinator/integrity architecture, entry and link patterns, CRUD and validation, source chain, membranes, capability grants, progenitor, cell cloning, countersigning, DNA migration and InitProperties, error handling, the TypeScript client, Sweettest multi-conductor testing, conductor debugging, the 0.6→0.7 upgrade break list, and a severity-graded zome review checklist. Adapted from Sacha Pignot's holochain-agent-skills (Apache-2.0); defers to p2p-design-gate for data modelling and to the root CLAUDE.md for networking, build and deploy.
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/holochain-hdk-0-7"
---
# Holochain HDK 0.7 Reference

ATTRIBUTION: Adapted from Sacha Pignot's `holochain-agent-skills` (Apache-2.0) —
https://github.com/Soushi888/holochain-agent-skills @ 28b00fc0ef7f5a25d04e2acdcb0c97e2f8ffb54a.
Full license text: `LICENSE` asset. Version pin: hdk 0.7.0 / hdi 0.8.0 / holonix ref=main-0.7.
Every adopted file is byte-identical to that commit; upstream HEAD was equal to the pin
for all of them when last checked (2026-09-19), so the pin is current, not stale.

## Where this skill yields

Upstream is generic Holochain. This repo is not. Where they disagree, **the repo wins**:

- Data-entity and identity design is gated by `p2p-design-gate`, not by upstream's
  data-model workflow. Invoke that gate first; use these references for HDK mechanism only.
- Networking, bootstrap and relay are governed by the root CLAUDE.md and the substrate
  trust-contract runbook — we run a pinned conductor fork behind a real iroh-relay.
  Upstream's networking advice does not describe this fleet.
- Build, test and deploy go through the eight `just` verbs. Upstream's `hc scaffold` and
  holonix packaging flows do not apply; our DNAs are not scaffolded.

## References (load on demand)

Mechanism — HDK/HDI surface:

- `references/architecture.md` — coordinator/integrity split, DNA structure, multi-DNA layout.
- `references/patterns.md` — entry/link types, CRUD, update-chain walk, `must_get_*`, signals.
- `references/source-chain.md` — source chain structure, actions, and chain queries.
- `references/error-handling.md` — thiserror + WasmError; `ExternResult` discipline.
- `references/cryptography.md` — sign/verify, x25519 encryption, key derivation.
- `references/scheduling.md` — scheduled function registration and persistence.

Membership, permission and lifecycle:

- `references/membranes.md` — genesis_self_check + AgentValidationPkg membrane-proof patterns.
- `references/access-control.md` — capability grants: the three CapAccess tiers and lifecycle.
- `references/progenitor.md` — progenitor pattern via DNA properties; coordinator guard.
- `references/countersigning.md` — countersigning session lifecycle and preflight requests.
- `references/cell-cloning.md` — clone creation, addressing, enable/disable, constraints.
- `references/migration.md` — DNA migration and `InitProperties` (NOT `modifiers.properties`).

Client, test and diagnosis:

- `references/client.md` — `@holochain/client`: auth tokens, callZome, signals, action/data split.
- `references/testing.md` — Sweettest multi-conductor testing patterns.
- `references/debugging.md` — inspecting a running conductor; `hc sandbox call` is gone in 0.7.
- `references/troubleshooting.md` — common HDK/conductor error diagnosis.

Workflows:

- `references/workflows/upgrade-holochain-0.7.md` — the 0.6→0.7 upgrade break list.
- `references/workflows/review-zome.md` — the zome audit checklist, severity-graded
  BLOCK/WARN/NOTE. It loads `architecture.md` + `patterns.md` as its context. Use it as the
  yardstick for auditing our zomes; where a check restates an upstream convention rather than
  an HDK invariant, record it as NOTE and defer to the repo.

## Landmines (surface before designing)

1. `strategy: clone_only` on a role leaves it unprovisioned — the 0.7 conductor panics
   assembling AppInfo when no provisioned instance exists for that role. Use a
   provisioned role + `clone_limit` instead of a clone-only role.
2. Sweettest has no membrane-proof support today. Decide the holon / multi-conductor
   test harness BEFORE building a membrane, not after — retrofitting it is expensive.
3. A coordinator-only change never moves the DNA hash; an integrity change always does.
   Upstream does not make this distinction load-bearing — here it decides whether a fix
   hot-swaps or needs a re-key. See the root CLAUDE.md before shipping either.

## Deliberately NOT adopted from upstream

- `networking.md`, `deployment.md`, `scaffolding.md`, `frameworks/*` — describe vanilla
  holonix, `hc scaffold`, and a Svelte UI. All three contradict this repo's substrate.
- `workflows/design-data-model.md`, `workflows/design-access-control.md` — `p2p-design-gate`
  owns entity and identity design here.
- `workflows/scaffold.md`, `workflows/manual-scaffold.md`, `workflows/implement-zome.md`,
  `workflows/package-and-deploy.md` — scaffold-first flows we do not run.
- `wind-tunnel.md` — a load-test framework we do not use.
- `assets/templates/*` and `references/example-happ/*` — boilerplate for greenfield hApps;
  434 KB of it is a Cargo.lock and a logo. We have a real codebase to read instead.
