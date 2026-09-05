---
title: "Rung 5 from the workspace — every update class propagates over p2p, orchestrated from the developer's own peer"
id: rung5-workspace-orchestration-plan
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
domain: D2
habits: [runtime-upgrade-propagation]
graduation-trigger: every `- [ ]` task below is checked with its named evidence line in the runtime-upgrade-propagation habit's DELTA ledger, and the rung-5 Stations 1–9 stay green on the household mesh with a lineage window open
topic: [release-channel, adoption-controller, runtime-config, packager, alpha-fleet, workspace-peer, a2o]
informed-by:
  - genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md (§5, §6, §11 — the lineage work this plan must compose with)
  - genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md (rung 5: the channel, verify, vehicles and receipt chain)
  - genesis/a2o/features/delivery/runtime-upgrade-propagation.feature (Stations 1–9)
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
cites:
  - "holochain-evolution-epic | Holochain Evolution Epic | sha256:8f8e2a7dcedea7aa | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "runtime-artifacts-elected-content | Runtime Artifacts as Elected Content | sha256:48ff8d7f46d423b9 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - genesis/a2o/features/delivery/runtime-upgrade-propagation.feature
  - elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md
  - genesis/a2o/scripts/epr-release-package.ts
  - genesis/a2o/scripts/release-ceremony.ts
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/services/release_adoption/verify.rs
  - genesis/orchestrator/data/deployments.json
---

# Rung 5 from the workspace — implementation plan

## Why this plan exists

The operator's mandate (2026-09-05): a developer in this workspace should propagate EVERY class of
update over p2p to the household mesh and on to the alpha fleet, with no Jenkins pipeline in the loop,
so delivery performance is measured between real peers delivering real updates. A read of the landed
code (2026-09-05, Opus) found the exact stopping point: a workspace-minted release reaches a peer's DHT
and is peer-pulled fine, but **no peer follows a channel unless someone edits its runtime-config by
hand** — on the mesh the a2o fixture rewrites the TOML, on alpha only a Jenkins render writes it.
That one line of config is the whole gap between "proven on three peers" and "propagates to the fleet".

A second finding is a live regression risk created by the lineage epic itself: the packager derives
`appliesTo` from a role's base cell and ignores the AUTHORING cell of a crossed role, while the
adoption controller already prefers the authoring cell — so the first coordinator release published
on a crossed peer would refuse `coordinator_lineage_mismatch` everywhere.

## Global Constraints

- Every task composes with the landed rung-5 and lineage pieces; nothing is reinvented (`ApplyRegistry`,
  `verify_envelope`, `runtime_config` watcher, `release-ceremony.ts`, `release-attestation-probe.ts`).
- The dev workspace orchestrates through the peer's OWN conductor and storage; the fleet is observed
  through the attestation probe, never through pod access.
- Nothing applies on alpha in this plan; the fleet legs stay `observe`. Any write to alpha's DHT uses a
  throwaway channel id and never runs while an edge deploy is in flight.
- Gate before commit: the touched tree's gate (`just gate` or the box's honest storage recipe), the a2o
  lint/tsc on touched files, and `genesis/orchestrator/runtime-config-render.test.mjs` for Task 3.
- One implementer per crate at a time; claims, fulfils and verdicts through `epr flow`.

---

### Task 1: the packager reads the authoring cell of a crossed role

**Files:**
- Modify: `genesis/a2o/scripts/epr-release-package.ts` (`roleBindingFrom` and the `--applies-to-from`
  derivation: when a role's passport carries `lineage.authoringDnaHash` /
  `lineage.authoringCoordinatorWasmHashes`, those are the `appliesTo` values; role-level values are
  the fallback; the derivation logs which cell it read)
- Test: `genesis/a2o/scripts/__tests__/` (a passport fixture with a crossed role → authoring values;
  an un-crossed role → base values; byte-identical manifest for the un-crossed case)

- [ ] **Task 1 deliverable: a coordinator release packaged on a crossed peer declares the authoring cell's hashes, and rung-5 Station 9 stays green on a mesh with a lineage window open.**

### Task 2: `POST /admin/runtime-config/follow` — a peer joins a channel through its own API

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (operator-seat route beside `/admin/runtime-config/reload`:
  body `{ "channel": "<id>", "mode": "observe|canary|apply", "remove": false }` → rewrites the
  `ELOHIM_RELEASE_CHANNELS` line of the WATCHED runtime-config file atomically (temp + rename), then
  reloads; refuses a malformed channel id or mode by name; 503 without a watched file)
- Modify: `genesis/a2o/steps/delivery/runtime-upgrade-propagation.steps.ts` and
  `happ-lineage-migration.steps.ts` (the fixture calls the route instead of rewriting the TOML; the
  byte-restore in `AfterAll` stays as the safety net)
- Test: unit tests for the line rewrite (present/absent/duplicate channel, remove); a2o dry-run clean.
- P2P design gate: the follow-set is node-local operator configuration (Ephemeral, class C, per the p2p-design-gate) — which channels THIS peer watches; it has no DHT entry type by design (the channel and its heads are the notarized entities, already on the DHT via `release-ceremony.ts`); the route writes the same watched file the boot config and the Jenkins render write, and projects nothing.

- [ ] **Task 2 deliverable: `hc-mesh.sh` and the fixtures enrol every peer in a channel through the API; no step rewrites a runtime-config file; Stations 1–9 stay green.**

### Task 3: alpha channel enrollment as data

**Files:**
- Modify: `genesis/orchestrator/data/deployments.json` (`ELOHIM_RELEASE_CHANNELS` per human under
  `runtimeConfig`: shem hosts `observe`, james `canary`, matthew and jessica `apply`, on the channel id
  the workspace ceremony will use; humans without the field render byte-identical)
- Test: `genesis/orchestrator/runtime-config-render.test.mjs` (a human with and without the field)

- [ ] **Task 3 deliverable: the next edge render enrols the alpha peers; until it lands, the rendered output is proven byte-identical for every human without the field.**

### Task 4: a workspace peer publishes to alpha in observe mode, measured

**Files:**
- Create: `genesis/a2o/features/delivery/workspace-to-fleet-release.feature`
  (`@concern:runtime-upgrade-propagation @act:iii @requires:shem`: a coordinator release minted here,
  published through `just dev conductor alpha` on a throwaway channel, is `admissible` on the alpha
  peers that follow it, observed through `release-attestation-probe.ts`; nothing applies)
- Create: `genesis/a2o/steps/delivery/workspace-to-fleet-release.steps.ts` (compose the device-peer
  recipe, `release-ceremony.ts`, the probe)

- [ ] **Task 4 deliverable: one receipt showing a workspace-minted release observed as admissible on alpha with no Jenkins act after Task 3's render; the manual steps a developer runs are five commands or fewer.**

---

## Self-review (2026-09-05)

- Task 1 is the only regression risk the lineage epic left in rung 5; it lands first.
- Task 2 deletes the fixture's file surgery on both stories and is the mesh half of the enrolment gap;
  Task 3 is the fleet half and needs one push and one render to take effect.
- Task 4 is the measurement of the mandate; it is gated on Task 3's render having landed and on no
  edge deploy being in flight.
