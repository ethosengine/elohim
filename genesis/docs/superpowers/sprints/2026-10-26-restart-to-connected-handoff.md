---
title: "Handoff — restart-to-connected after R3, its follow-ups and the household bundle"
id: restart-to-connected-handoff-20261026
date: 2026-10-26
status: handoff
author: "claude-opus-5-5 (Che workspace, ethosengine), at the operator's direction"
habits: [workspace-device-arrival, devices-share-one-berth, push-gate-pays-once]
cites:
  - restart-to-connected-sprint-plan-20261026 | Sprint plan | sha256:d2c664288eedeaed | path: genesis/docs/superpowers/sprints/2026-10-26-restart-to-connected-sprint-plan.md
  - gate-merge-sprint-plan-20261025 | Sprint plan | sha256:ebca07662b59ca75 | path: genesis/docs/superpowers/sprints/2026-10-25-gate-merge-sprint-plan.md
  - genesis/data/timeline/backlog/security-session-successor-admitted-before-facet-release.md
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
---

# Handoff — restart-to-connected after R3, its follow-ups and the household bundle

**What this work is.** After a workspace restart, a device should return to the latest build and
to its peers in one step. The sprint plan `2026-10-26-restart-to-connected-sprint-plan.md` holds the
lanes (R1–R5) and their probes; the device-runtime plan
`/projects/.claude-config/plans/shiny-booping-map.md` is the authority above it. This handoff
carries only what the plan does not: what landed, live state, decisions pending and traps.

**Start here.** The operator pastes this into a NEW Claude Code session on ethosengine:

> Carry on with the restart-to-connected sprint; read
> genesis/docs/superpowers/sprints/2026-10-26-restart-to-connected-handoff.md first. Push the
> unpushed batch on fix/r3-followups, then take R1.

## What landed on `dev`

| Commit | What |
|---|---|
| `ddee90f30` | Slice M: one offline writer of the device's runtime declaration (receipt ran on `household-receipt-m`, adam cast as a fourth peer) |
| `66e8ea380` | R3: a resumed session opens a successor generation after its owner died |
| `b1ac8d486` | Seeder: an operator 401 names both causes, never "credential drift" |
| `bd7b1094a` | epr-cli test helpers' rust-style debt 15 → 0 |
| `b276d3f0a` | Schema codegen `--verify` notes a stale gitignored cache instead of refusing |
| `d44a9b8f2` | Habit `push-gate-pays-once` DELTA: reuse live on ethosengine (7126 s → 1475 s on one commit) |
| `aadc7f8bb` | bin-launch run under its own name refuses instead of exec-looping |
| `826d8f361` | install-launcher retires a plain launcher-name file that cannot identify itself |
| `9292dc7f1` | A successor may resume in another checkout of the same device |
| `19a8f0e05` | berth status names a holder on another device, never "free" |

Shem's session was told of the last four by bridge message; delivery was not confirmed.

## Not pushed: the batch on `fix/r3-followups`

Worktree `/projects/elohim/.worktrees/device-runtime-r3`, on `19a8f0e05`:

- `7fd61b01e` security backlog item, the R3 item's landed note, a `TODO(security)` pointer at the
  successor admission check in `device-runtime/src/envelope.rs`
- `37ed7a522`, `1edcde2ab`, `b7670045c` the operator's direction for a dev release channel (seed)
- this handoff, then its decisions and these follow-ups
- `b65e13604` hc-mesh: a resumed household refuses a bundle that does not reproduce its cells
- the epr declared-trust fix (`git.rs`, `covenant.rs`, the recall fold) and its review fixes
- the docs naming `MESH_ALLOW_BUNDLE_SKEW` (root gospel package and the hc-dev-orchestrator skill)

The eprfs changes select the eprfs gate, so the push runs the full pre-push (the last one took
4717 s). From the worktree: `GATE_EVIDENCE=reuse git push origin HEAD:dev`. Never
`git -c safe.directory=*`. After it lands, arrival installs the fixed `epr`; until then the
session-start `index:` line keeps reporting the fold as failed (2324 files behind at 2026-10-26,
a backlog of the old bug, not a new one).

## Live state that has no other home

- **Device (read 2026-10-26 ~21:30Z).** Store heads: epr `bafkreigbl2…`, brit `bafyreiai7…`. brit
  now HAS a landed release here, so the plan's R1 line "ethosengine has none yet" is stale. All
  three launchers (`epr`, `brit`, `brit-build-ref`) resolve from `/opt/rust/cargo/bin`, which a
  restart wipes; `bin-launch` is not on PATH. The peer daemon hears shem (fresh within seconds) but
  runs `bafkreifpvyu…`, behind the store's current.
- **Canonical household** (`genesis/local-dev/household-dowell`): stopped, identities intact. Its
  bundle was never lost (see Traps); it is kept at
  `artifacts/installed-20261007/elohim.happ` (sha256 `b4f86c90…`) with a provenance README.
  Its `environment.sh` now points all five bundle settings (`MESH_HAPP_PATH`, `STORAGE_HAPP_PATH`,
  the doorway's `HAPP_BUNDLE_PATH`, and the `MESH_RESTART_ENV_OVERLAY` / `MESH_DOORWAY_ENV_SET`
  strings) at the recovered bundle, at the operator's word. They previously named
  `genesis/local-dev/campaign-1.4/coordinators/elohim.happ` (sha256 `86ac6567…`), which matches
  imagodei, mishpat and node_registry but whose lamad and infrastructure integrity wasm differ
  (`uhC0keuLMY…`, `uhC0kYVFIp…`): a different DHT for every hosted human the doorway installed, not
  a coordinator hot-swap. The household's run log holds only the recovered bundle's five hashes.
  Nothing the household ran was given up. campaign-1.4's 2026-10-01 refresh (`e9e0b3b63`,
  "constrain FCT publication to witnessed device authority": new lamad `content_store` and `mishpat`
  coordinators) was applied to the Che device's cells (`genesis/local-dev/che/credential-coordinator-refresh-20261001.json`),
  never to the household's. To bring it here, the refreshed `mishpat` coordinator (`30066df3…`) was
  built against the same `mishpat_integrity` the household runs (`f45c04bb…`) and is a hot-swap
  candidate; the lamad one was built against a different integrity (`2b970c9a…` against the
  household's `094f4601…`), so rebuild it from `e9e0b3b63`'s coordinator source against the
  household's integrity first. The prior file is kept as `environment-before-cell-lock.sh` (older)
  and in session scratch.
- **Receipt household** `genesis/local-dev/household-receipt-m`: stopped, state kept. Removable once
  nobody needs slice M's receipt evidence.
- **Shift journal** `.claude/shifts/2026-10-26T04-35-land-slice-m-household-receipt.journal.md` in
  the main checkout, uncommitted. Its "lost install bundle" line is superseded by this handoff.

## Next, in order

1. **Push the batch** above.
2. **R1** per the plan. Remaining after slice M and today: `epr device arrive` installs every
   registered tool's launcher from `current` with no cargo, into the install directory the
   RuntimeManifest `tools` field declares (`~/.local/bin` survives a restart, `/opt/rust/cargo/bin`
   does not). brit's landed release is already here. This is also why gate reuse silently stops after
   a restart: without `brit-build-ref` on PATH no evidence is recorded, and `status device` shows it
   only as degraded.
3. **R2** per the plan: arrival lands dev's published release, or names the queued build.
4. **R4** per the plan: `epr peer up`. Today's daemon behind `current` is its "restart a stale
   daemon" case, a ready fixture.
5. **D3** (gate-merge plan): a reuse miss names the closure component that moved. It needs a brit
   verb that fetches a project's previous verdict without the exact key; until then, print
   "comparison unavailable". Evidence for why: push `2952 s` re-ran storage with no named cause.
6. **R5** and **C1b** are shem's (the restart probe; hook ledgers dirtying the tree so eprfs and
   orchestrator never reuse evidence).
7. **Security item** `security-session-successor-admitted-before-facet-release`: one of three fix
   directions, a regression for the half-closed close, then the re-review checklist in the item.

## Decisions with the operator

- **R3 threat model:** accepted for now, affirmation held until the security item lands and its
  re-review is done.
- **Household `environment.sh`:** repointed at the recovered bundle (see Live state). The defect
  that let it drift, no check that a bundle reproduces the household's cells before launch, is
  fixed in `hc-mesh.sh` preflight on this branch: readiness records `<mesh>/installed-cells.json`
  from storage `/health`, and a resume refuses a bundle whose role-modified hashes differ. The
  canonical household's record was written by hand from the verified run-log set; against the real
  bundles the check passes the recovered one and refuses campaign-1.4 on lamad and infrastructure.
- **adam as a standing peer:** no, not now. `join-peer` stages an unrecorded late joiner that
  `mesh stop` sweeps and `mesh start` never relaunches (`DEFAULT_HOUSEHOLD_PEERS` is
  matthew,jessica,james), so "standing" would mean a household roster redesign. adam is also the
  identity of a real provider machine (shem), which the operator's workspace should not simulate;
  no habit or a2o scenario needs a fourth peer. Revisit only when one does: a declared roster change
  proven on a receipt household first. A real provider goes through shem and the `--on adam`
  operator items.
- **Dev release channel design:** the P2P design gate runs after R2, on one entity (the seed's
  closing section says which). R2 must keep reading `publishedRef` and add no release head of its
  own; if it needs one, the gate runs first.

## Traps this run paid for

- **`hc dna hash` ignores hApp role modifiers.** elohim's happ.yaml sets
  `properties: {progenitor_pubkey: null}` on four roles, so a bundled `.dna`'s raw hash never matches
  an installed cell. Repack each DNA with its role's `network_seed` and `properties`, then hash. Two
  sessions called the household's bundle lost on this, and proposed a recast.
- **A conductor's `wasm.db` is SQLCipher-encrypted** (plaintext header, encrypted pages); code can't
  be read back offline.
- **After a context reset, verify landing state from `git log origin/dev`**, not from the summary:
  this run told the operator slice M was unlanded when it was on dev.
- **berth_test needs a missing device key** (`ELOHIM_DEVICE_KEY_FILE`); the real key made the real
  epr refuse fixture sessions (40 failures, fixed in `19a8f0e05`).
- **Hooks suggest `git config --global --add safe.directory`.** Never do it; run git from a
  worktree, which git trusts.
- **Two epr paths ignored the trust `epr device trust` declared**, so they failed "dubious
  ownership" after every restart: `epr covenant record` (its `brit-build-ref` child) and the recall
  fold's `git ls-files` (the session-start `index: skipped — fold status unreadable` line). Both
  now take it from `git.rs` (`trust_env` for a child that runs git itself, `trust_args` for a git
  spawned by another runner) on this branch. Until that lands in the installed `epr`, give the
  covenant call path-scoped trust:
  `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0=/projects/elohim`.
- **Memory index:** `MEMORY.md` is generated; never hand-append. The entry
  `project_hc_dna_hash_ignores_role_modifiers.md` is imported under the operator's
  steward-of-record affirmation, but the index will not re-project: it is 24536 bytes against the
  24000 hard watermark with or without that entry. That is the `cleanup:` trigger's backlog (155
  against 120): run `memory-stasis-loop`, which can also graduate this entry, since `hc-mesh.sh`
  preflight now enforces it. Making the 536 bytes means consolidating entries other sessions
  wrote, which needs that pass's maintenance authority, not a side edit.
