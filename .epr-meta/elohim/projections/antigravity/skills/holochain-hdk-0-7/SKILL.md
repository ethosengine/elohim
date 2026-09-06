---
name: holochain-hdk-0-7
description: Reference for writing Holochain hApps on hdk 0.7.0 / hdi 0.8.0 — source chain, membranes (genesis_self_check + AgentValidationPkg), countersigning, Sweettest multi-conductor testing, the 0.6→0.7 upgrade break list, troubleshooting, cryptography, scheduling. Adapted from Sacha Pignot's holochain-agent-skills (Apache-2.0).
metadata:
  runtime: antigravity
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/holochain-hdk-0-7.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/holochain-hdk-0-7"
---
# Holochain HDK 0.7 Reference

ATTRIBUTION: Adapted from Sacha Pignot's `holochain-agent-skills` (Apache-2.0) —
https://github.com/Soushi888/holochain-agent-skills @ 28b00fc0ef7f5a25d04e2acdcb0c97e2f8ffb54a.
Full license text: `LICENSE` asset. Version pin: hdk 0.7.0 / hdi 0.8.0 / holonix ref=main-0.7.

## References (load on demand)

- `references/membranes.md` — genesis_self_check + AgentValidationPkg membrane-proof patterns.
- `references/source-chain.md` — source chain structure, actions, and chain queries.
- `references/countersigning.md` — countersigning session lifecycle and preflight requests.
- `references/testing.md` — Sweettest multi-conductor testing patterns.
- `references/troubleshooting.md` — common HDK/conductor error diagnosis.
- `references/cryptography.md` — sign/verify, x25519 encryption, key derivation.
- `references/scheduling.md` — scheduled function registration and persistence.
- `references/workflows/upgrade-holochain-0.7.md` — the 0.6→0.7 upgrade break list.

## Landmines (surface before designing)

1. `strategy: clone_only` on a role leaves it unprovisioned — the 0.7 conductor panics
   assembling AppInfo when no provisioned instance exists for that role. Use a
   provisioned role + `clone_limit` instead of a clone-only role.
2. Sweettest has no membrane-proof support today. Decide the holon / multi-conductor
   test harness BEFORE building a membrane, not after — retrofitting it is expensive.

## Not included here (adapt/write elsewhere, not verbatim adoption)

Deliberately NOT copied from upstream because they need elohim-specific rewrites:
`cell-cloning.md`, `networking.md`, `design-data-model.md`, `review-zome.md`,
`access-control.md`. Do not port these verbatim; they encode elohim's own DHT
entry-type census, P2P transport, and access-control conventions instead.
