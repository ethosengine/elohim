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
4. `ChainQueryFilter` pushes NOTHING down on our conductor fork. `SourceChain::query`
   (holochain_state/src/source_chain.rs) loads the author's WHOLE chain and, with
   `include_entries(true)`, every entry on it, then filters in Rust afterwards — so
   `entry_type(..)` and `sequence_range(..)` shrink nothing that is read. Only
   `include_entries(false)` does. Pattern: headers-only query with the filter, then
   `get_details(hash, GetOptions::local())` for the survivors. `get_details`, not `get`:
   `query()` returns superseded records and `get()` returns only live ones.
5. `GetStrategy::default()` is `Network`, but the read verbs differ (holochain_cascade).
   `get()` tries the local store FIRST and returns on a hit, so converting a read-after-write
   `get` to `Local` buys nothing. `get_links()`, `get_link_details()`, `get_details()` and
   `count_links()` have NO local short-circuit — they are local today only because every node
   runs full arc. In the ~20-minute window after a conductor comes back up, or under any
   fractional arc, each one is a network round trip against a 60 s conductor timeout. Every
   new `get_links` names its strategy: a reader whose `None` means "not in my view yet" is
   `Local`; one whose `None` is an authoritative absence stays `Network`.
6. `zome_info()` is not an accessor — the ribosome re-runs the integrity zome's `entry_defs`
   callback, a live wasm invocation, on EVERY call, and the derive-generated
   `EntryTypes::deserialize_from_type` calls it internally. Hoist it out of loops
   (`resolve_entry_type` in content_store's `post_commit` is the template).
7. "Latest state wins" is safe on the caller's OWN source chain and nowhere else. Integrity
   update arms here discard `action`, so nothing checks who authors an Update; resolving the
   newest record across the DHT lets any agent append one. See
   `arch-authority-in-integrity-backlog` row 14 before writing such a reader.
8. Never mint a per-relationship `Assigned`/`Transferable` cap grant. The fork's
   `valid_cap_grants` loads every grant in the access class on every zome call — already the
   fleet's dominant measured call cost. If a Holochain-native signal lane is ever wanted, it is
   ONE `CapAccess::Unrestricted` grant on `recv_remote_signal` with the handler verifying the
   payload. The fast lane already exists on the byte plane (`validate_carried_head_record`).

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
