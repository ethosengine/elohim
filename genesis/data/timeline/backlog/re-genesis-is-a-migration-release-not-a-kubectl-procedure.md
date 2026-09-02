---
id: "backlog-re-genesis-is-a-migration-release-not-a-kubectl-procedure"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Re-genesis is a migration RELEASE, not a kubectl procedure — rung 6 of upgrade propagation: an integrity-changing hApp adopted through the same channel ceremony as a coordinator release, whose vehicle does what the 2026-09-02 hand procedure did (refuse-if-current, witness first, stop, clear databases/ + ks/, restart, first-install, attest the key lineage) with the intent carried by the manifest and consent carried by the channel follow"
slug: "re-genesis-is-a-migration-release-not-a-kubectl-procedure"
written: "2026-09-02"
author: "shift 2026-09-02T02-20-land-rung5-batch (operator question)"
status: "envisioned"
priority: "high"
domain: "D-runtime-operations"
roadmap_rung: "upgrade-propagation arc — rung 6 (integrity migration by election), above rung 5 (coordinator-only) proven 2026-09-02 r12"
relatedNodeIds: []
tags: [re-genesis, migration, release-channel, adoption-controller, dna-hash, integrity, identity-lineage, operator-surface, death-witness, clear-conductor-state, rung-6]
cites:
  - elohim/elohim-storage/src/conductor/process_manager.rs
  - elohim/elohim-storage/src/happ_manager.rs
  - elohim/elohim-storage/src/services/release_adoption/watch.rs
  - elohim/holochain/dna/dna-hashes.baseline
  - scripts/ci/dna-hash-guard.sh
  - genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md
  - genesis/data/timeline/backlog/death-witness-runtime-harvests-a-dying-conductors-last-words.md
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
---

## The question (operator, 2026-09-02 21:1xZ)

"This procedure — should it be a setting/procedure available from within the runtime, like the
runtime operator surface, or is this really a one-time procedure?"

## Answer: a runtime verb, and more precisely a release class

What the k8s dev executed by hand today (scale to 0, wipe `databases/` + `ks/`, scale to 1, verify
the first install) is, step for step, `ConductorManager::clear_conductor_state` — the runtime's own
**node-repair primitive** — plus the judgment the runtime does not yet hold: *when* it is safe,
*what* the node is migrating to, *who* consented, and *what happened to the identity*. Today only
the reseedable self-heal path calls the primitive, implicitly, with none of those.

The north star (rung 5 proven on the household mesh 2026-09-02: coordinator-only releases by
election) has an obvious next rung: **an integrity-changing hApp is a release too**, adopted
through the same channel ceremony, whose apply vehicle is re-genesis.

### The shape

- **Manifest.** A release of artifact class `happ-bundle` with `migration: re-genesis` and
  `appliesTo` naming the DNA hashes it supersedes and `provides` the hashes it installs — the
  values `DNA_MIGRATION_INTENT` stood in for today, and the values `dna-hashes.baseline` now pins
  in CI. The DNA Hash Guard is the build-time half; this manifest is the runtime half.
- **Ceremony.** Staged → canary re-genesises first → soak → attestation → promote → converge →
  revertible only in the sense the channel can re-elect a prior migration (a re-genesis is
  one-way for data; the manifest says so and the verify arm prices it as such).
- **Vehicle** (`release_adoption` apply arm for this class): refuse if the running DNAs already
  match `provides`; refuse without the manifest's intent covering every drifted role
  (the `happ_manager` gate landed today, 867e4bf9b, is this check); write the **witness first**
  (a death-witness-shaped atom: why, `appliesTo`/`provides`, the agent key being retired, the
  authored-DB sizes it is about to discard); stop the conductor; `clear_conductor_state`
  (`databases/` + `ks/` — keeping `ks/` would fork the old key onto an empty chain); restart;
  first-install the fetched bundle; report readiness; then **attest the identity lineage** —
  old agent key → new agent key, signed by the node and countersigned by its steward/custodian —
  so custody commitments, hosted-agent bindings, and fixture humans' `agentPubKey` follow the
  human across the break instead of silently dangling (the gap this migration leaves open today).
- **Consent.** The channel follow, in the mode the steward chose — auto-adoption is the default
  (memory `project_upgrade_authority_constitutional_elohim`); `canary` for the peers that go
  first; `observe` for a peer that wants to see it land elsewhere before it moves.
- **Operator surface.** One action with a receipt on the runtime-config / `/admin` surface:
  "this channel is offering a migration to <hashes>; this node will re-genesis at <time> unless
  held" — and the death witness of each re-genesis readable at `/epr/{cid}` afterwards. No
  `kubectl`, no PVC hand-work, no ordering that lives only in a person's head.

### What today's manual run taught the vehicle

- Order matters for the DHT: bootstrap pair first (adam, matthew), then the rest — the vehicle's
  canary order.
- A node that survived the incident (jessica: Ready on the OLD DNAs) still has to migrate, or it
  is alone on an orphaned DHT — the manifest's `appliesTo` is what makes that decidable, not a
  human noticing.
- The hApp must be *published* before any node re-genesises (Harbor `dev-latest` lagged the
  guard by a build); the release CID is the artifact, so this class cannot lag by construction.
- The image the installer runs must carry the vehicle (the conductor pod, post-split); a mutable
  `dev-latest` tag is not an address — the release manifest is.

## Done when

The household mesh migrates lamad to a deliberately integrity-changed DNA through a channel:
publish → james re-genesises (witness written, lineage attested) → promote → matthew/jessica
follow → the observed version matrix shows the DNA hash move and the key lineage per peer — as
an a2o scenario beside Stations 1–8 in `features/delivery/`, tagged `@concern:runtime-migration`,
bound to a habit under `elohim/elohim-storage/.epr-meta/`.
