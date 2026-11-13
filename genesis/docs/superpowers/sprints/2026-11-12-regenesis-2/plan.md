---
id: regenesis-2-plan
title: Re-genesis 2 — the toolchain crossing (Holochain 0.7.1, Rust 1.100, security, discipline)
date: 2026-11-12
status: DRAFT v0 — scope and pre-clear wave proposed; orchestration ruled (split by lane, §5.6); decisions 1-5 open
author: shem session 3dd97e3d (Claude Opus 5.5); friction review by Codex gpt-6-astra (read-only)
habits: [dataplane-convergence, push-gate-pays-once, pin-attestation, addresses-are-declared-profiles]
---

# Re-genesis 2 — the toolchain crossing

## 0. Why

Steward ruling, 2026-11-12: "we're still in dev, we just did a re-genesis sprint, we should review that
for friction we can clear out in advance of this one, and then we can re-genesis the network again.
Being in dev gives us that privilege, and if we can clear some security backlog and get a huge rust
version bump, and drive consistency and discipline across the codebase. We should do that." And:
"this should be a big one, and that's ok."

The reason to bundle: Holochain 0.7.1 forces hdi 0.8.1, which moves all five DNA hashes; Rust 1.100
changes the wasm bytes anyway; upstream 0.7.1 cannot read capability grants already on a chain. Each
of these alone costs a partition, a reinstall and a re-key. Together they cost it once, and a
re-genesis removes the need to migrate any of them.

## 1. Grounding (true at dev `0088755ec`)

- **Holochain 0.7.1** (tag `holochain-0.7.1` = `fdaa67878`, 2026-10-14). Breaking for us: capability
  grants restructured (`CapGrantEntry { tag, constraint, capability }`, `Capability::ZomeCall |
  DirectSignal`; old grants unreadable), `Action::signer` removed and every action signed by its
  author, direct-signal wire and signature change (mixed 0.7.0/0.7.1 peers drop each other's
  signals), `CellDisabled` → `CellNotRunning(cell_id, reason)` (about 20 string matches in doorway and
  storage). Upstream adds six of the fork's eight `elohim_*` indexes under other names and network
  entry-hash verification. Upstream still lacks the cross-relay preflight fix.
- **The fork.** The deployed pin `993cf597a` is 34 linear commits on 0.7.0 and is NOT on fork branch
  `elohim-0.7` (`7e553f9c3`), whose 11 further commits (iroh receive throttle, diagnostics, entry-hash
  skip, …) have not been deployed. Port so far (local, `.worktrees/hc-fork-071`): `elohim-0.7.1-sync` =
  13 patches + the cross-relay fix re-vendored on kitsune2_transport_iroh 0.5.1 (78/78 tests);
  `explore/0.7.1-sync-plus-clean` = 25/34 patches, `cargo check` green. Stopped at the cap-grant
  series (8 commits) because upstream rewrote `valid_cap_grants`. Evidence:
  `.worktrees/deps-hc-071/genesis/local-dev/hc-071-sync/` (DNA hashes before/after, conflict diff).
- **DNAs.** hdk 0.7.1 pins `hdi =0.8.1`; the bump also moves holo_hash 0.7.1 in all six
  `elohim/sdk/domains/*/types` crates (shared with storage, doorway, sweettests). content_store and
  mishpat coordinators match on the old `CapGrant`/`CapAccess` shapes in their authorisation checks.
- **Rust 1.100.0** (released 2026-11-12): never-type fallback to `!` on editions < 2024, a new cargo
  build-dir layout (hash moves to parent directories), cargo `unused_dependencies` warn-by-default,
  `min-publish-age` cooldowns. Under H13 posture B (ratified 2026-11-12: "keys equal only with equal
  compilers") both devices move compilers together, and every gate closure re-keys on `rustc -vV`.
  Branch `deps/rust-1.100-20261112` in preparation.
- **Security.** npm branch `deps/npm-security-20261112` landing now (0 critical after). Rust `zip`
  (critical, 7 lockfiles) branch `deps/zip-security-20261112` in preparation; `elohim/Cargo.lock`
  rolls alpha. Open: Angular 22.2 patch line (`@angular/platform-server` GHSA-w739, router #909),
  backlog `security-angular-22-patch-line-coordinated-bump`; the delivery gate reports 474 live
  dep/sec findings.

## 2. Pre-clear wave (lands BEFORE the crossing, re-genesises nothing)

From the regenesis-1 friction review (`evidence/regenesis-1-friction-review-codex.md`, 24 items, 9
confirmed open; shem spot-checked five cited lines, all matched). Ranked by attention saved:

| # | Item | Pre-clear | Size |
|---|---|---|---|
| F01 | Failed installs mint a new agent key per attempt (`elohim-storage/src/happ_manager.rs` `generate_agent_pub_key` per fresh attempt); lair stores churned, two operator wipes | keep install intent and key across attempts; reconcile an indeterminate install in-process; reproduce against the kept forensic copies | L |
| F03 | Broken old-revision StatefulSet pods block replacement; operator deleted them (600 s timeouts) | bounded replacement of an unready old-revision pod after a successful apply (`elohim/holochain/Jenkinsfile`, `scripts/ci/rollout-watch.sh`) | M |
| F05 | Doorway accounts keep retired conductor keys; repair took two rounds | orphan-binding probe and an identifier-aware, inventory-driven repair plan (backlog `doorway-orphaned-binding-after-regenesis`) | M |
| F04 | The reset erased the app release channel; needed a steward tag | declare the required channels and reconcile them in the existing ceremony (`scripts/ci/app-release-stage.sh`) | M |
| F08 | Serving receipts key all of `genesis/a2o`, so any a2o edit re-runs a 20–40 min receipt | key on the feature/step import closure, conservative fallback (`genesis/a2o/scripts/lib/sut.ts`) | M |
| F06 | Peers pull `elohim-happ:${HAPP_VERSION}` by floating tag during a partial roll | resolve the digest once per rollout and pull by digest (`_edgenode-conductor.template.yaml`) | M |
| F07 | hApp freshness compares mtimes, so a licence-only edit reads stale until an unrelated source commit | build-input fingerprint (`app/elohim-app/scripts/hc-mesh.sh`) | M |
| F10 | Rust style debt surfaced only at push | run rust-style before the expensive gates in the crossing preflight | S |

Qualification checks to carry into the crossing preflight (cures already on dev; keep them proven):
F02 cold-space single-flight regression is a mandatory fork-port test; F14 storage-before-conductor
boot; F15 delayed-readiness binding mint; F16 doorway readiness across a conductor restart; F17
seeder deferred-write retry; F21 upload-contract self-test; F23 HEAD/GET parity. Procedure: F22 —
one integration base, gitlinks recorded, before any worktree is cut.

From ethosengine's side (notes 2026-11-12, its lanes unless marked shem):

| # | Item | Pre-clear | Owner |
|---|---|---|---|
| E1 | Peer daemons read `pool-policy.json` from a main checkout behind dev and denied every Produce crossing (K9) | sync both main checkouts to dev before the crossing; landing 20 warns at serve/status start | both |
| E2 | Device epr release lags (`bin-store refused epr candidate compare-and-swap`) | move both devices to a released epr before the crossing | ethosengine |
| E4 | Fresh worktrees fail late in elohim-app (unbuilt element packages; 3402 s push, red at 64 s in) | worktree prep step or a fail-fast gate check | shem |
| E5 | Dev moved during a notice window and aborted landing 18 | hold the lane for a declared window during the crossing | both |
| E6 | Three artifact-class defects passed unit tests and two reviews; only a household build found them | schedule the household serving receipt on the final tree before the last push | shem |
| E7 | CLAUDE.md landing exception looks stale (notes refspec passed on landings 17–20) | re-check on brit 0.1.14 and retire it in the discipline work | shem |
| E8 | `pgrep -f`/`kill` matched its own shell and killed the daemon | `ps -eo args` with a bracketed grep pattern | note |

Landing shape: pre-clears are cut in separate worktrees from one recorded base and landed as ONE
branch, so the pre-push hook is paid once (`push-gate-pays-once`).

## 3. The crossing (one migration)

Tracks, each prepared on its own branch, merged into one crossing branch:

1. **H — conductor.** Finish the fork port on 0.7.1. Because nothing is migrated, the cap-grant
   series reduces to: port the one-query-per-class read and the secret-keyed lookup onto
   `CapGrant { constraint, capability }`, filtering by capability BEFORE the secret match (a
   DirectSignal grant must not satisfy a ZomeCall check; the port adds the test that holds it); drop the convergent backfill and the old-shape
   decode; drop the fork's copies of the six indexes upstream now builds; review FK-787 against
   upstream #6020; decide the 11 undeployed `elohim-0.7` commits (§5). Run the fork's conductor and
   holochain_state suites, F02's regression, release binaries, `-candidate` tools. Steward publishes
   the fork branch; conductor image `elohim-edgenode:conductor-<hc12>`; gitlink move.
2. **C — clients.** doorway, storage, hc-dbtool, sweettests on holochain_client 0.9.1 and the 0.7.1
   type crates; the new `GrantZomeCallCapability` payload; `CellDisabled` → typed `CellNotRunning`
   matching (not a renamed string).
3. **D — DNAs.** hdi 0.8.1 / hdk 0.7.1 / holo_hash 0.7.1; port the content_store and mishpat
   authorisation checks; new `dna-hashes.baseline`; `[dna:migrate]`. Candidate integrity additions
   that only a hash move admits (§5): AgentPeerBinding cryptographic-authorship validation
   (`imagodei_integrity/src/agent_peer_binding.rs`; `arch-dataplane-borrows-backlog` row 25).
4. **T — toolchain.** Rust 1.100.0 at every pin home together; build-dir-layout fallout in pool and
   mesh tooling; never-type and lint fallout. Both devices switch compilers in the same window.
   Brit moves to 1.100.0 in its own repository (the Brit session, steward request 2026-11-12) and
   arrives here through its next pin; the monorepo branch leaves `elohim/brit` alone.
   Prepared on `deps/rust-1.100-20261112` (12 commits; every Rust gate run green under 1.100.0:
   storage 5921, doorway 2033, eprfs 2391 tests). Still owed in the crossing:
   - **DNA builds have no toolchain pin.** Locally they use the rustup default `stable`; CI uses
     the holonix flake. 1.100.0 moves the lamad integrity wasm and DNA hash (measured: 1.99.0
     `uhC0kIIVeo6…` → 1.100.0 `uhC0knnvLw1…`). Pin the DNA workspaces to the same rustc CI uses,
     so a local household build and CI's bundle agree. The devspace image (che-devworkspaces
     `33f5ca7`) now defaults to 1.100.0, so a restarted workspace rebuilding DNAs before the
     crossing would make household bundles skew from their recorded cells.
   - `pool-lib.sh list_stale_artifact_hashes` walks `deps/`, which Cargo 1.100's build-dir layout
     removes: it reclaims nothing on 1.100 slots. Old-layout slots double until pruned.
   - Service images pinned below the workspace: storage `rust:1.94.0` (Dockerfile and
     `scripts/ci/build-storage-image.sh`; its Dockerfile copies `deps/`, `build/`, `.fingerprint`,
     which 1.100's layout breaks), doorway `rust:1.93`, steward/node `rust:1.94.0`, edgenode
     zombie-fix `rust:1.91`.
   - `epr-cli/src/device/supervise.rs:692` prints a stale `RUSTUP_TOOLCHAIN=1.96.1` cure.
   - Cargo's new unused-dependency warning names `eae`, `gate-client`, `gate-client-zome`,
     `tower-http` in `elohim/Cargo.toml` (K track).
5. **S — security.** `zip` lockfiles (six of seven fixed on `deps/zip-security-20261112`, zip 9.x; the seventh, sweettest #1044, is held by `holochain_metrics` 0.7.x requiring `zip ^8.6`, so the fork port bumps it to 9.x and the sweettest pin follows); Angular 22.2 patch line if it fits; the dep/sec ledger drained
   by `/deprecation-stasis` where a fix is bounded.
6. **K — consistency and discipline.** Measured, not aspirational: cargo `unused_dependencies` to
   zero in touched crates; rust-style paydown in crates the crossing touches (the no-new rule already
   requires one finding removed per touched crate with debt); `elohim-storage/src/main.rs` (7,030
   lines) boot consumers extracted before 0.7.1 branches are added (F20).

Activation order (regenesis-1 runbook): crossing lands → CI builds conductor image, DNAs, storage and
doorway → both households recast (`MESH_RESET=1`) and measured → alpha: `DNA_MIGRATION_INTENT` armed
with the five new hashes on every conductor (both genesis peers need `ALLOW_DNA_REINSTALL`), stores
cleared, re-seeded, targeted account rebinding (F05's procedure) → validation.

## 4. Verification

- Fork: conductor and holochain_state suites green; F02 regression; cross-relay preflight tests;
  release `holochain`/`hc` built and installed as `-candidate`.
- Households (both devices, same head): resilience/chaos-peer-churn 3/3, `@concern:blob-durability`
  at least regenesis-1's 8/0/3/3, dataplane/epr-app-deliverability 7/7, custody announce refused_* 0,
  direct signals delivered between all peers (new: proves the 0.7.1 wire on one version).
- Alpha: live cells are exactly the five new hashes; all conductors on the new pin; both doorways
  200 on `/` and `/lamad/` and resolve one landing head; Dataplane Validation green
  (`[build:edge] [edge:validate-only]`); `pnpm look` 0 page errors.
- Security: `pnpm audit` 0 critical; Dependabot criticals closed for `zip`.

## 5. Steward decisions

1. Cap-grant port: the reduced design in §3.1 (no backfill, capability filter before secret match),
   or drop the fork's grant patches and measure upstream's grant reads first?
2. The 11 undeployed `elohim-0.7` commits: fold them into the 0.7.1 port, or leave them out?
3. Integrity additions beyond the version bump (AgentPeerBinding authorship): in this crossing or not?
4. Angular 22.2 patch line: in this crossing or its own landing?
5. Cargo `min-publish-age` cooldown: adopt after the crossing (it would hold back fresh security
   fixes during it)?
6. ~~Orchestration~~ **Ruled 2026-11-12 (steward, via ethosengine): split by lane.** shem orchestrates
   the whole and owns H (fork 0.7.1) and D (the five DNAs); ethosengine owns T (Rust 1.100), epr/eprfs
   and the artifact-store re-record on both devices, moving compilers in one window (H13 posture B).
   One landing lane, coordinated as today. shem's prepared `deps/rust-1.100-20261112` is input to T.

## 6. Risks and stop conditions

- iroh 1.1 against the mesh's iroh-relay 1.0.3 is unverified: test before the image is built.
- Upstream 0.7.1 schema migrations on databases already carrying the fork's indexes are unverified;
  re-genesis clears alpha stores, but households resume in place unless recast.
- F01 (key churn on failed installs) unfixed makes the reinstall step the most likely place to lose
  an afternoon: it is first in the pre-clear wave for that reason.
- A mixed fleet (some conductors 0.7.0) silently drops direct signals: the gitlink move and the DNA
  migration land in the same crossing.
- Stop if any DNA role fails to MATCH its new baseline in CI, or if a household lane regresses below
  regenesis-1's numbers.

## 7. Branches now

| Branch | Where | State |
|---|---|---|
| `deps/npm-security-20261112` | `.worktrees/deps-npm-20261112` | landed (`ab4ed3b9d`) |
| `deps/zip-security-20261112` | `.worktrees/deps-zip-20261112` | in preparation |
| `deps/rust-1.100-20261112` | `.worktrees/deps-rust-1100`, origin branch | handed to ethosengine for track T |
| `deps/dependabot-harvest` | `.worktrees/dependabot-harvest` | landing with this plan and the brit 0.1.14 pin |
| `elohim-0.7.1-sync` (fork) | `.worktrees/hc-fork-071` | 13/34 + iroh re-vendor; local |
| `deps/holochain-0.7.1` | `.worktrees/deps-hc-071` | no commits; evidence only |
| `regenesis-2/plan` | `.worktrees/regen2-plan` | this plan |

## 8. Friction seen while drafting (2026-11-12)

- PreToolUse hook budgets of 2–3 s (`brit-first.py`, `native-seam-guidance.py`,
  `claude-md-structural-signal.py` at 2 s; the Edit/Write set at 3 s) overran by 12–43 ms at load
  average 13 with three cargo builds running. The crossing runs that many builds at once, so either the
  hooks get faster or the budgets get measured headroom, before it starts.
