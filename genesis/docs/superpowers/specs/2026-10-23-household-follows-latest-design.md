---
title: The household follows latest — continuously built binaries reach ring zero by head election
id: household-follows-latest-design
status: Draft
class: architecture
serves: [dataplane-convergence]
date: 2026-10-23
context-tier: disclosed
steward: human:matthew
author: "Claude (orchestrator, Opus 5.5), at the operator's direction"
tags: [release-channel, auto-update, adoption, household, binaries, epr, exec-floor, session-pin]
graduation-trigger: >-
  After a green gate on a followed binary's project, a session that starts on this workspace runs
  that build everywhere it reaches the binary — hooks, gate-runner, a2o steps, smokes, the mesh
  harness — and `epr --version` names the same CID each of them ran. A session already open when
  the head moved keeps its pinned CID and is not refused. A build that fails its health check
  leaves every device on the previous CID with a typed refusal. On the household mesh, a green
  elohim-storage gate reaches all three peers' running processes with no `just mesh build`, no
  hand restart and no per-host copy.
cites:
  - "runtime-artifacts-elected-content | channels, elected heads, stewarded auto-adoption and the adoption controller this extends to the tooling and binary classes | sha256:eaa2716381075140 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - "compute-envelope-tevah | the exec floor: spawn only from a verified release, re-hash at spawn, record the CID run | sha256:d427a367f3b01a6e | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "workspaces-cooperate-as-devices-design | §8 device-to-device build artifacts by rakia input CID; slice 4's transport | sha256:88c203269b34f963 | path: genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md"
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - genesis/a2o/features/delivery/runtime-upgrade-propagation.feature
---

# The household follows latest

## 1. The problem, as a smoke showed it

A first-round smoke passed its last check, "released lane reads free", by luck: it ran a stale
`epr`. The gate had built the current code, but the smoke never ran it.

Recon on 2026-10-23 found three `epr` builds in use at once, each reached by a different lookup:

| Build | Where | How it gets there | Who runs it |
|---|---|---|---|
| `891893b2…` (17:27) | `/opt/rust/cargo/bin/epr` (PATH) | `cargo install` by the SessionStart hook `.claude/hooks/epr-evaluator-guard.py`, only at session start, judged by raw worktree mtime | `gate-runner.mjs` (attested gate), a2o `run-plane.steps.ts`, `delivery-view.ts`, `household-attestation.ts` — anything that falls back to bare `epr` |
| `21350110…` (21:49) | `/tmp/eprfs-gate-target/debug/epr` | `just gate` (`cargo.targetDir` in `elohim/eprfs/build-manifest.json`) | `.claude/hooks/_observation.py`, `habits-status.py`, `delivery-scoreboard.py`, the recall executor |
| `3b45457a…` (10-18) | `family/dev/elohim__eprfs/dev/debug/epr` | an older pool build | `device-preflight` and `_lib/epr_client.py` when PATH misses (newest pool glob) |

The same session showed the opposite failure. The gate rebuilt `/tmp/eprfs-gate-target` partway
through, and the recall executor, which pins the sha256 of its own bytes per session
(`epr-cli/src/flow/memory/recall/mod.rs`), refused every later turn: "executor bytes changed since
this session began". A binary can be older than the newest build, or it can change under a running
session. Both come from the same missing rule. Nothing says which build a device should be running,
or when it may switch.

`epr` cannot report its identity either. `epr --version` fails, though
`govern::evaluator_identity()` already computes `{id, version, cid}` from its own bytes.

The mesh binaries have a partial guard. `hc-mesh.sh` preflight refuses a storage or doorway binary
older than its tracked source (`MESH_ALLOW_STALE_BINARY`), and `just mesh build` installs into
`<pool slot>/mesh-bin/`. But refusing is not adopting: a developer still rebuilds and restarts by
hand.

## 2. What the operator decided (2026-10-23)

1. **Auto-update is a trust the network holds.** This is the pattern people know from Chrome and
   Claude Code, where most devices update by default. In Elohim the authority is not a vendor's. It
   is the stewarded auto-adoption of `runtime-artifacts-elected-content` §4: the network holds
   upgrade authority, adoption is automatic by default, silent staleness is a harm the network
   heals, and a peer's own say covers scheduling only.
2. **We follow `latest`.** We build the binaries, so we run the newest head first.
3. **The household is ring zero.** A new build reaches the household mesh, and this workspace as
   one of matthew's devices, before anything else.
4. **This serves `dataplane-convergence`**, through head selection. The habit's invariant ("a peer
   that missed a deploy heals") applies to binaries as it does to content.
5. **Reaching the head presupposes the reach gates.** A build becomes a head a follower can resolve
   only after it has passed the EPR reach gates for that channel's audience. The follower does not
   judge trust a second time. It checks that the head was resolved through its own conductor or
   fold, and that the fetched bytes match the head's CID.

## 3. What already exists, and is reused

This design adds no new updater. It extends what is there:

- **Channels and elected heads** (`runtime-artifacts-elected-content` §3). A channel is a `Content`
  id named `runtime:<class>:<network>:<name>`. Its head is elected, staging below earned. Reach is
  scoped at birth and can only narrow.
- **The builder never promotes** (same spec, §12.4; `services/release_attestation.rs`). CI and a
  green gate publish at the staging tier. Earned promotion needs evidence from peers that ran the
  build.
- **The adoption controller** (`elohim/elohim-storage/src/services/release_adoption/`) runs watch,
  fetch, verify, apply on every peer, in modes `observe | canary | apply`. In `canary` it adopts
  staging heads. It already refuses binaries unless the peer declares Simulacra (developer/test)
  stakes (`binary_stakes_not_simulacra`). It stages a storage binary as `elohim-storage.next` and
  never runs it.
- **The exec floor** (`compute-envelope-tevah` §4, "Delivery of bytes"). The envelope spawns only
  from a `VerifiedRelease`-class value, re-hashes the bytes at spawn, records the CID it actually
  ran, and keeps the stakes gate. It already supersedes `steward/node/src/update/`. `ark` does the
  re-hash at spawn today (`elohim/ark/supervisor/src/native.rs`), but on the mesh only conductors
  launch under `ark` (`MESH_CONDUCTOR_LAUNCH=ark`). `hc-mesh.sh` still starts storage and doorway
  directly.
- **Device-to-device artifacts** (`workspaces-cooperate-as-devices` §8). Build artifacts are served
  between a steward's devices, keyed by rakia input CID.
- **The canary story** (`runtime-upgrade-propagation.feature`). James canaries, promotion follows
  his evidence, the household converges with no "update" click, and revert is a re-election.

What is missing:

- **Developer tooling is not a release class.** `epr` belongs to no channel, and the workspace
  follows nothing.
- **Adoption stops before execution.** Nothing switches at a boundary, commits only after a health
  check, or falls back on its own.

## 4. Lessons from the field this design adopts

From a survey of Chrome/Omaha, Claude Code's installer, TUF/Uptane, Sparkle/Tauri, Nix/Guix,
Android A/B, Fuchsia, OSTree, Radicle and Bitcoin's Guix attestations:

- **Authenticity and targeting are separate questions.** The CID and its attestations say what the
  artifact is. The elected head says what this ring should run. (TUF targets vs timestamp; Uptane
  image repo vs director.)
- **Authority lives in the history it governs.** Who may move a head changes only by acts the
  current authority signs. Guix's `.guix-authorizations` and Radicle's delegate quorum work this
  way. Here, `HeadDelegation` already does.
- **Each version gets its own directory, and a pointer flips atomically.** Claude Code, Nix and
  Squirrel all do this.
- **A version in use is never deleted.** Claude Code keeps the version a live session runs.
- **New versions apply only at a boundary**: next launch, restart or reboot. Nothing mutates a
  running process's bytes.
- **Installed is not committed.** A health check commits a version. Failure falls back on its own
  (Android A/B, Fuchsia's committer, greenboot).
- **Rings promote on reported outcomes, not on time** (Omaha events, Chrome early stable, SRE
  canaries).

## 5. Design

### 5.1 Every followed binary names itself

Every binary in a followed class answers `--version` with its CID (sha256 of its own bytes, the
`evaluator_identity()` value) and its package version. This identity is what every later check
compares. Never mtime, path or a file's age against its source.

Provenance lives in the release record, not in the binary: the source commit, whether the tree
was clean, and the gate run that built it. It sits beside the bytes in the device's store, in the
shape of the release manifest's `provenance.buildInfo`. A binary that carried its own commit would
need a rebuild for every stamp, and its CID would stop naming only its behaviour.

### 5.2 A green gate publishes. It never promotes.

When a followed binary's gate project goes green, the gate publishes the built artifact as a
**staging** release on that class's household `latest` channel, as matthew's delegated device. A
red gate publishes nothing. Publishing is the reach act for household audience (decision 2.5). The
gate's own tests keep running the candidate by explicit path (today's `EPR_BIN`). That is the one
legitimate way to run a build that is not the head: testing it before it becomes one.

*Calls made after the builder check (2026-10-23; the operator can overturn them):*

- **Two rings, by audience** *(operator, 2026-10-23: "if it's in the dev context, say between two
  peers, that can be a channel, but publishing to dev publishes to the wider audience")*.
  - **The dev-context channel.** Any green gate makes its build *available* there: in a worktree,
    on a branch, even on a dirty tree. Availability is content-addressed and narrow in reach. A
    device or peer runs a dev-context build only by pinning its CID, so a spike reaches exactly
    who asked for it.
  - **Landing on `dev` publishes to the wider audience.** Only a green gate on a clean tree whose
    commit lands on `dev` (the pre-push to `dev` today, or the local fast-forward) publishes the
    staging head on household `latest`. Two worktrees therefore cannot flip that head between
    them.
- **Head order is `dev` ancestry.** A new staging head must descend from the current one. Moving
  backward happens only as a revert election, never because a build is newer.
- **Source-input CID** is computed from that clean commit.
- **The artifact published is the one the runtime runs.** Some classes ship a different build from
  the one the gate tests. elohim-storage's gate runs `cargo test` on default features, while the
  mesh runs a `--bin` build with `p2p p2p-iroh`. For those classes the published artifact is the
  declared runtime build (today's `just mesh build` recipe, with the gate's pinned toolchain),
  built from the same commit after its gate is green. The canary health check (§5.6) is what
  proves that artifact. The gate does not grow the mesh feature set: it already runs at
  `CARGO_BUILD_JOBS=1` for RAM.
- **CI is a builder and a witness, not a follower.** A Jenkins container runs the `epr` it built
  from the commit under test, by explicit path. After slice 1, `EPR_BIN` survives only as that
  candidate-under-test path, for the gate and for CI. It stops being a general override.

### 5.3 Household devices follow `latest` in canary mode

Each household device declares Simulacra stakes. Who adopts the landed staging head first
*(the reconciliation the operator accepted with "carry on", 2026-10-23)*:

- **The steward's own devices adopt it at once.** That is this workspace, and later shem. The
  developer is the first ring.
- **Inside the mesh, `runtime-upgrade-propagation.feature`'s order stands.** james follows
  `latest` in `canary` mode and adopts staging. matthew and jessica follow in `apply` mode and
  adopt the earned head once james's evidence promotes it. The household keeps a soak.

Where heads live depends on whose they are:

- **The dev-context channel among one steward's devices** rides the steward-device channel of
  `workspaces-cooperate-as-devices` §3.3. It is device-signed and needs no mesh running.
- **Network-runtime classes** (storage, doorway) use the DHT election through the existing follow
  set (`ELOHIM_RELEASE_CHANNELS`, `POST /admin/runtime-config/follow`).

Either way, resolution goes through the device's own conductor or fold (I1). A hint or a signal
only triggers a local resolve.

### 5.4 One store, one pointer, one resolver

- Each device holds a content-addressed binary store: one directory per CID.
- Each binary name has one pointer to the CID the device currently follows, flipped atomically.
- Every consumer reaches the binary through that pointer: hooks, `gate-runner.mjs`, a2o steps,
  smokes, `device-preflight` and the mesh harness. The three lookup orders in §1 are **retired, not
  kept as fallbacks**. So are the PATH copy that `cargo install` writes and the "newest pool glob"
  rule.
- The pointer is reached through a **stable launcher on PATH** that resolves the CID and `exec`s
  it. Claude Code's `~/.local/bin/claude` → `versions/<v>` works the same way. Because of `exec`,
  `current_exe()` is the CID binary itself, so self-hashing checks (the recall executor's digest,
  `evaluator_identity()`) see the real bytes. Resolution order: `EPR_BIN` (the candidate under
  test), then an explicit dev-context pin, then the session's pin, then the committed pointer.
- **The launcher must be the `epr` PATH reaches.** On 2026-10-24 the second workspace showed two
  ways around it. A `~/.local/bin/epr` linked to an Oct-10 pool build shadowed it from earlier on
  PATH, and a hand `cargo install` wrote a plain binary over it.
  - A link into a build output that sits ahead of the launcher is drift. The SessionStart guard
    repoints it at the launcher.
  - Any other shadow is reported by name and never touched.
  - A hand-installed binary is kept as a dev-context build, runnable by pinning its CID, before
    the launcher takes its path back.
  - `just status device` refuses while anything shadows the launcher.

### 5.5 Sessions pin. Switches happen at boundaries.

- A session pins the CID the pointer named when the session started. Sessions are a Claude Code
  session, a mesh lane, a smoke run or a gate run. The recall executor's per-session digest is the
  first instance of this and stays.
- A new head takes effect at the next session start. It never swaps under a running one.
- **A nested session inherits its parent's pin** *(call made after the builder check)*. A gate
  run, mesh lane or smoke started from a Claude session runs the CID the outer session pinned,
  carried in the environment. Every hook and tool inside one session therefore runs one CID. A
  gate that publishes a new head does not adopt it mid-session. Its candidate tests already ran
  the new build by explicit path.
- **A daemon follows the device head, never the session that started it.** `epr peer serve` is
  the same `epr` binary running as a long-lived process. The registry declares it in
  `bin-tools.json` under `longRunning`. Two rules apply:
  - The launcher resolves `current` for it and ignores the launching session's pin.
  - It writes a lease keyed by the daemon's own pid and start time. `exec` keeps the pid, so gc
    keeps that build for exactly as long as the daemon runs.
- **The plain-terminal gap.** A human terminal with no Claude process above it has no session pin,
  so each call resolves `current`, and two consecutive commands can straddle a landing. A shell-
  or TTY-level pin closes this later. Until then, that is the one place a mid-sequence switch can
  happen.
- Long-running processes (mesh storage, doorway) switch at a restart. The device schedules the
  restart, which is the "scheduling" residual of §4.2. Spawning goes through the exec floor: only a
  verified release, re-hashed at spawn, with the CID that ran recorded.

### 5.6 Commit after health. Fall back on failure.

- A new CID is **installed** when it is in the store and the pointer names it. It is **committed**
  only after its class's health check passes on that device. For `epr`, at minimum `--version` plus
  one read-only flow command. For storage and doorway, the readiness ladder.
- On failure the pointer returns to the previous committed CID. The device records a typed refusal,
  and that attestation is the evidence for contest or revert (the soak-attestation rail).
- Garbage collection keeps the committed CID, the previous committed CID, and any CID a live
  session has pinned.

### 5.7 Staleness is visible, then healed

`just status device` and mesh preflight report **running CID vs followed head**. A lane running a
CID behind the head with no declared pin is the harm of §4.3: the device heals it at its next
boundary, and the lane reports it in the meantime. Once the head rule covers a class, the mtime
rules for that class (`epr-evaluator-guard.py`, `assert_binary_newer_than_source`) retire. A
deliberate run of an older CID stays possible as a declared pin, recorded the way
`MESH_ALLOW_STALE_BINARY` overrides are today.

### 5.8 brit follows its attested pin

brit is the second followed tool, and it shares the same store and launcher (`bin-store --tool brit`).
Only what moves its head differs:

- **The head is a pin.** brit's head is the `elohim/brit` gitlink on published `dev`.
- **Moving it needs an attestation.** The pin may move only when `gate-attest.mjs` judges `Tests pass`
  a success on that SHA. A "claimed" result from an unreachable read is not enough. `land` enforces this
  itself, so every path inherits it.
- **The release holds three bins plus an alias:** `brit`, `brit-build-ref` and `brit-verify`, plus
  `rakia`, which is `brit` exec'd under the name `rakia`. The release is addressed by the hash of its
  canonical `SHA256SUMS`.
- **Where the bytes come from, in order:**
  1. The prerelease `brit-build-<sha12>`, which brit's own CI publishes after `ci` succeeds on main
     (ethosengine/brit PR #3). Every checksum is verified, and `brit --version` must name the pin.
  2. Otherwise a local build. The cargo credential for Nexus is `CARGO_REGISTRIES_ELOHIM_TOKEN`, or
     the device's `NPM_TOKEN` passed as `Bearer …`. It travels in the environment only and is never
     printed.
  3. Otherwise a refusal naming both cures.
- **What a session means for brit.** brit's natural session is a gate run or a push. Pinning there
  is about evidence rather than convenience: one push runs one brit from start to finish.

## 6. Slices, each admitting the next

The ladder is the elected-content spec's: local → hybrid → mesh → cluster.

1. **Local: `epr` on this workspace.** Covers §5.1, §5.4, §5.5 and §5.6 for `epr` on one device.
   The local rung's "channel" is this device's own store and pointer, and the green gate flips it.
   Delivers the first sentence of the graduation trigger, and cures the smoke incident and the
   recall refusal.
   *Added 2026-10-24 (operator: "resolve them"):* a device that only **pulls** must still
   converge. So at session start the guard runs `bin-store --tool epr follow`, which builds and lands the
   checkout's HEAD when all four hold:
   - HEAD is contained in published `origin/dev`;
   - HEAD descends from the landed head's commit;
   - epr's own sources changed between the two;
   - those sources are clean.

   Until slice 4 moves bytes between devices, this is how the second workspace follows the first.
   It costs one background cargo build per device for each published change to epr's sources.
2. **Channel publish.** The same CID published as a staging release on `runtime:tooling:household:latest`
   (exact naming is the builder's call, inside §3's grammar), and the workspace follows it through
   the adoption path instead of a local flip.
3. **Mesh binaries.** elohim-storage and doorway follow their household `latest`. The `.next` slot
   is spawned through the exec floor at a scheduled restart, ending the "staged, never executed"
   state. Delivers the last sentence of the graduation trigger. This slice includes moving storage
   under `ark` on the mesh, which is new launch work. Doorway follows once storage is green.
   `ArtifactClass` has no doorway-binary class and the doorway runs no adoption controller, so the
   doorway's follower is the device envelope beside it. That settles gamma, which has no storage
   peer of its own. *(Call made after the builder check.)*
4. **Second device.** The other workspace (shem) follows the same channel and fetches bytes from
   ethosengine by CID (`workspaces-cooperate-as-devices` §8).

Promotion beyond the household (alpha, the fleet, other humans' devices) is out of scope here. It
is the elected-content spec's earned promotion, fed by the attestations slices 2–4 produce.

## 7. Goals, non-goals, anti-goals

**Goals.** Every household device runs the head of `latest` with no hand step. Every run names the
CID it ran. A bad build costs one failed health check and nothing else.

**Non-goals.**

- Fleet binaries self-updating (`runtime-artifacts-elected-content` §9 stays).
- DNA and integrity moves (rung 6).
- The conductor fork, pinned by gitlink today; it can join later as its own class.
- Per-update prompts, or any per-device opt-out of staying current.

**Anti-goals** (failures even if the mechanism works):

- A second updater beside `release_adoption`.
- Old lookup orders kept as fallbacks.
- mtime or path standing in for identity.
- A binary swapped under a running session.
- A gate that promotes its own build.
- Extending `steward/node/src/update/` instead of retiring it.

## 8. Side-findings recorded

- `steward/node/src/update/download.rs` `verify_signature` logs a warning and returns `Ok(())`: an
  unsigned download is accepted. The tevah spec already marks this updater for replacement by the
  exec floor. It should be retired, not repaired. This needs a backlog home if none exists.
- `device-preflight`'s "epr binary" line checks presence only. §5.7 replaces it.

## 9. Left to the builder

- **Slice 3 work for the `epr` daemon:**
  - restart onto a new head at a boundary, under `ark` and spawned from a verified release, together
    with `elohim-node`;
  - `epr peer status` naming the daemon's CID;
  - tolerance between a pinned CLI and a daemon on another build.
- **Carrying landed heads on the steward channel.** `epr peer serve` already gossips `ChannelHead`,
  which is the natural transport for slices 2 and 4.

- `epr integrate`'s atomic fast-forward (`elohim/eprfs/epr-cli/src/flow/integrate.rs`) is a second
  way to land on `dev`. Slice 1 wires the pre-push, because the main tree has `dev` checked out
  and lands by push. Slice 2 should make `integrate` publish too.
- Slice 1's store and launcher are Python (`.claude/scripts/_lib/bin_store.py`). That is
  deliberately a sketch of the local rung, not the durable home: the operator's 2026-10-22 ruling
  gives the lowest-level primitives first-class Rust. They retire into native code at slice 2, when
  following moves into the adoption path and the launcher becomes a native exec. The native launcher
  also removes the interpreter's per-call cost, measured on 2026-10-24 at about 40 ms over the
  bare binary.

- The pointer and store layout and the exact path.
- The health-check command for each class.
- Channel naming within §3's grammar.
- Whether slice 1's local flip lives in `epr` itself or in the gate step.
- How the a2o scenario that proves the graduation trigger is phrased, and its `@concern:` tag,
  which joins `dataplane-convergence`'s `checks:`.

## 10. The device runtime, slice A: the native core (design note, 2026-10-24)

Slices 1 and 2a proved the local rung in Python. Slice A moves its primitives into native library
modules with thin `epr-cli` adapters, and deletes the copies other subsystems grew. The session
and bin-store modules live in their own strict crate, `elohim-device-runtime`
(`elohim/eprfs/device-runtime`), which `epr-cli` and `bin-launch` both depend on; arrival stays in
`epr-cli`. It lands in four commits, each passing the device-runtime plan's clean-code gates on
its own.

**A.1 Session identity (`elohim_device_runtime::session`).** `session::resolve()` answers one value,
`(runtime, session, owner pid + start time?, source)`, in a fixed order:

1. a declared `ELOHIM_SESSION_ID` only when its readable `ELOHIM_SESSION_OWNER_PID` is at or below the nearest registered runtime ancestor, or when neither an owner nor a runtime ancestor exists; otherwise the nearest registered runtime wins;
2. the nearest ancestor whose `comm` is listed in `genesis/agentic/agent-runtimes.json`, labelled
   with that runtime's native session variable when it is set;
3. the controlling terminal's session leader (pid + start time), which closes §5.5's
   plain-terminal gap;
4. `none`, which is an answer and exits 0.

The registry, not the code, names runtimes. It starts with claude-code, codex, gemini and
antigravity. `epr session id --json` renders the value under the `session-identity` view schema.
Python and TypeScript reach it through one stdlib shim each (`.claude/scripts/_lib/session.py`,
`genesis/a2o/src/framework/session.ts`). Each shim calls `epr session id --json` and falls back
to its own re-implementation only when `epr` is absent; a parity test runs both against the same
fixture `/proc`. Every resolver the census counts moves to a shim in the same commit, and its
private copy is deleted. `genesis/agentic/bin/dup-census` lands here with the slice-A baseline.

**A.2 Arrival (`epr device status`).** `device::status() -> Vec<Check>` has three states: `ok`;
`degraded`, where an enhancement is missing and the line names its cure; and `REFUSED`, where the
device cannot work. It exits 1 only on REFUSED. It absorbs `device-preflight`'s six lines, adds
`repository trusted`, and reads cross-device facts as `unknown` when they are unreachable. The git
layer takes one declared trust rule for the device's own checkout, which retires the per-call
`safe.directory=*`.

**A.3 The bin store (`elohim_device_runtime::bin_store`, `epr bin <verb>`, `bin-launch`).** A
behaviour-for-behaviour port of `bin_store.py`, whose test cases become Rust tests. Release
identity becomes CIDv1 (`bafkrei…` for one file, `bafyrei…` for a sums manifest), and
`evaluator_identity()` changes with it. The bootstrap order is local-first: verified `current`,
then any verified CID in the store, `epr bin ingest --closure`, `cargo build --offline`, a
networked build, and finally a refusal naming every cure. `bin-launch` is a separate tiny binary.
It does no hashing for session-pinned CLI tools; daemons keep the exec floor.

**A.4 Receipts and carry-overs.** The gate-receipt leg, the daemon's running CID in its lease,
and the fix for the pre-push renew-loop race.

Slice A decides nothing that crosses devices (slice B), nothing about the manifest (slice M) and
nothing about supervision (slice C).

## 11. The session envelope, slice A2 (design note, 2026-10-25)

Slice A gave a session one identity (§10 A.1). Slice A2 makes that identity the only key an
agent's facets are found by, so no verb takes a session id by hand and every facet closes once.
The envelope is a key and a read model. It holds no facet's authority: a claim stays a fold, a
lease stays a berth record, a covenant clause stays steward-granted, a tier claim stays an actor
claim.

**The envelope id is the P1 session string.** No second id is minted. `epr session open` writes
one Private (B) record, `<device dir>/sessions/<id>.json`, beside the device key, because the device
directory survives a `$HOME` wipe and never travels. The record binds:

- identity: the P1 value (runtime, session, owner pid + start time, source), the device
  did:key (read from the key, never minting one), the steward (the roster's standing human), and
  the model — from `--model`, else the runtime row's optional `modelEnv` in
  `genesis/agentic/agent-runtimes.json`, else absent;
- context: the recall session (the envelope id, opened through the existing recall verb) and the
  tool CIDs this session pinned in the bin store.

`open` is idempotent for the same live owner. `show --json` (view schema
`session-envelope.schema.json`) joins the live facet stores by the id: berth leases, valueflow
claims, covenant clauses, the capability tier, standing, recall and pins. It copies none of them.

**Re-keying.** berth `moor | claim`, `epr flow claim`, `epr covenant`, recall `open`, the bootstrap
recall session and the tier gate stop resolving sessions themselves; an omitted `--session` means
the envelope id. Their record formats do not change. berth's private `/proc` ancestry walk is
deleted for the Python shim, so the census family `proc session ancestry` reaches 0.

**Ending.** `epr session close` releases the berth leases the envelope holds, seals its covenant
record and writes one `epr flow note` witness. gc finds envelopes whose owner (pid + start time) is
dead and closes them through the same function, so a killed session is released by liveness, the
rule pins and leases already use.

**The start hook** for every projected runtime becomes `epr device arrive && epr session open`.
A runtime with no hook gets the envelope opened lazily on its first `epr` call.

**Why every session's recall bootstrap refuses (diagnosed 2026-10-25).** The UserPromptSubmit
hook prints `executor bytes changed (recorded 769a28fc1ca4…, running bafkreibvo7g…)` on every turn.
The running executor is `/tmp/eprfs-gate-target/debug/epr`, whose raw CID is `bafkreibvo7g…`. The
hooks Claude Code runs come from the main checkout, and that checkout's `_observation.resolve_bin`
prefers that cargo target path to `PATH`. The path is a mutable build output, so every
`cargo build` there changes the executor under a live session. The recorded value is also spelled
as bare sha256 hex from before §10 A.3. A CIDv1 can never equal it, even for the same bytes.
Slice A already removed the fallback on dev: the client asks the stable launcher, which execs the
session's pinned artifact. A2 makes the bootstrap recall session the envelope id and takes the
executor's identity from the session's pinned CID. A test runs two hook invocations against one
fixture store, rebuilds the cargo target between them, and shows both read the same executor.
A continuation recorded under the old spelling is adopted explicitly through the printed
`adopt` pointer. There is no silent compatibility arm.

**The scenario** serves `dataplane-convergence`, through this spec's graduation trigger (one
session runs one pinned CID everywhere it reaches the tool), not `devices-share-one-berth`, whose
invariant is steward-scoped claims across devices (slice B). A Codex session and a Claude session
each open an envelope under fake runtime ancestors (`EPR_PROC_ROOT`), hold their own leases and
claims, and close; a killed one is released by liveness.

A2 decides nothing about what crosses devices: the envelope travels in slice B.

## 12. The declared target, slice M (design note, 2026-10-25)

Slices A and A2 made a session run one pinned build and hold one envelope. Slice M says what a
device should follow and run, in one record, so the follow set stops living in fifteen files. The
plan's §M ruling stands: ark's `RuntimeManifest` is extended, and no sibling manifest is minted.

**The record.** `RuntimeManifest` (`elohim/ark/core/src/manifest.rs`) already carries `supersedes`
(the previous manifest's CID), `processes`, `envelope` and `archetype`. It gains two fields, both
`#[serde(default)]` and skipped when empty:

- `follows: Vec<Follow>`, where `Follow { channel: String, mode: FollowMode, pin: Option<String> }`
  and `FollowMode` is `observe | canary | apply`. `channel` is `runtime:<class>:<net>:<name>` or the
  dev-scope `bin:<tool>`. The grammar is the one `parse_followed_channels`
  (`elohim/elohim-storage/src/services/release_adoption/state.rs:171`) reads from
  `ELOHIM_RELEASE_CHANNELS` today: entries separated by `,`, newline or `;`; `channel=mode`, with a
  bare channel meaning `observe`; an empty channel or unknown mode refused; a duplicate with the same
  mode collapsed and one with a conflicting mode refused. That parser and its renderer move into
  ark-core beside `Follow`, and storage calls them, so the rendered line and the field cannot
  disagree.
- `tools: BTreeMap<String, ToolSpec>`, the registry `genesis/agentic/bin-tools.json` holds today,
  field for field: `bins: Vec<String>`, `aliases: BTreeMap<String, String>`,
  `candidate_env: BTreeMap<String, Vec<String>>`, `long_running: Vec<Vec<String>>`, and
  `source: ToolSource { kind: repository | gitlink, manifest: Option<String>, path: Option<String>,
  published_ref: Vec<String>, repo: Option<String>, release_prefix: Option<String>,
  attestation: Option<Attestation { provider, repo, check }> }`. The rendered `bin-tools.json`
  keeps today's camelCase keys, so `Registry::load` reads it unchanged. `tools.epr.bins` gains
  `bin-launch`. A release then carries its launcher, and arrival's `install-launcher` installs
  the landed one. Today a release carries only `epr`, so arrival reinstalls the launcher that is
  already there, and `bin-launch` changes only by hand (measured 2026-10-25: the 16af7710f landing
  left the 12:00 launcher in place).

Adding them leaves `MANIFEST_SCHEMA` (same file) at 1 and moves no existing manifest's CID: a test
pins every committed manifest's CID, as `manifest_cid_is_unmoved_by_the_child_spec_rename` does for
the `ChildSpec` rename. A change of declaration is a new manifest whose `supersedes` names the last.

**Truth class and homes.** Private (B), owned by the steward's device. It is not an HTTP resource,
it never crosses as authority (slice B may carry a version to the steward's other devices only as a
submission they may adopt), and a secret appears only by name. Both homes work offline:

- *A workspace device* keeps its chain in `<device dir>/manifest/` (the device dir holds the device
  key, survives a `$HOME` wipe and never travels): each version as DAG-CBOR named by its CID, plus a
  `head` file naming the current one. Its first version supersedes a committed base,
  `genesis/orchestrator/manifests/runtime/workspace-device.manifest.json`, which M creates: `tools`
  copied from today's `bin-tools.json`, and `follows` = `bin:epr=apply`, `bin:brit=apply`. A device
  with no chain reads the base, so a fresh device needs no network and no hand-edited registry.
- *A fleet human* keeps the committed manifest `deployments.json` already names
  (`humans[].runtimeManifest {cid, path}`, under `genesis/orchestrator/manifests/runtime/`). Its
  follows move into that file. The manifest is the source: an edit to it re-renders both the
  pinned `cid` and the `runtimeConfig.ELOHIM_RELEASE_CHANNELS` string in `deployments.json`, never
  the other way round.

**One writer, and where it lives.** `epr device follow <channel> --mode <m> [--pin <cid>]
[--unfollow]`, `run <tool> [--stop]`, `show [--json | --render runtime-config | --render
bin-tools]` and `apply`. Each act takes the chain's lock, reads `head`, writes the superseding
version, then moves `head` by atomic rename; a crash leaves the old head. `--root <device dir>`
addresses another device's directory, which is how the mesh harness declares each peer. The code
splits in two:

- *Pure* (types, grammar, `supersede(head, act) -> RuntimeManifest`, the three renderers): ark-core,
  which must stay free of filesystem and runtime dependencies (its `boundary` tests).
- *The chain store and `apply`* (lock, versions, `head`, writing the projections): a
  `manifest_store` module in `elohim-device-runtime`, beside `bin_store`, reusing its lock and
  atomic-write helpers. `epr-cli` calls it, and elohim-storage takes `elohim-device-runtime` as a
  dependency for the follow route. That is within rail R-E3 (placement-contract interfaces spec,
  2026-10-20, §rails), which only forbids eprfs-storage, eprfs-local, eprfs-serve and eprfs-agent
  depending on elohim-storage, and elohim-storage depending on eprfs-storage or eprfs-local. If
  elohim-storage's own boundary test refuses the new edge, the builder stops and reports it rather
  than copying the writer.

**Rendering, and what is deleted.** `apply` (run at boot by `epr device arrive`, and after every
act) renders from the head and is the only writer of each projection:

- the watched runtime-config file's `ELOHIM_RELEASE_CHANNELS` line. `app/elohim-app/scripts/hc-mesh.sh`
  and `hc-start.sh` stop writing it and call `epr device follow --root <peer>` then `apply`;
- `deployments.json`'s `runtimeConfig.ELOHIM_RELEASE_CHANNELS`, through the existing render path
  (`genesis/orchestrator/runtime-config-render.test.mjs`), with a freshness test that refuses a
  hand edit;
- `bin-tools.json` beside the launcher, where `epr bin install-launcher` already puts it. The
  hand-edited `genesis/agentic/bin-tools.json` is deleted; `Registry::discover`'s checkout fallback
  reads the committed base manifest instead.

The follow route (`elohim/elohim-storage/src/api/runtime_config_follow.rs`,
`POST /admin/runtime-config/follow`), `ConfigEprVehicle`
(`elohim/elohim-storage/src/services/release_adoption/apply.rs:1911`, which applies a config-epr
release) and `genesis/agentic/compute/measure.sh` (which writes a worker's runtime config) become
callers of the act: a config-epr release is an input to a new manifest version, never a second
write of the file.

**Modes, and the notice.** `apply` adopts a new head for new sessions, as today; `canary` adopts
after the health check; `observe` fetches and verifies but adopts nothing. Every mode tells a
running session when its pin falls behind. The bin store already keeps
`<store>/<tool>/current.json` (`{cid, commit, landedAt, previous}`, written by `epr bin land`) and
each session's pin under `<store>/<tool>/sessions/`. The UserPromptSubmit hook compares the two
with no process spawned and prints one line per new head per session, naming the old and new CIDs,
the commit subject and the switch (`/clear`, or `epr bin pin --cid`). A session pinned to a
dev-context build sees the same line when a landed head is available. Slice B adds peers' heads
and peers' artifacts as further sources of the same line.

**Census** (`genesis/agentic/bin/dup-census`). The `follow-set declarations` family reads 15 and
`runtime-config writers` reads 5. After M, every file `follow-set declarations` still counts either
reads the one declaration or is its writer: the hand-written declarations in `bin-tools.json`,
`hc-mesh.sh`, `hc-start.sh`, `genesis/a2o/scripts/device-peer-receipt.ts` and the
`deployments.json` strings are gone, so it falls to at most 10. `runtime-config writers` falls to
1, the `manifest_store` module. The builder reports both path lists.

**The scenario**, `genesis/a2o/features/devflow/device-declared-target.feature`, tagged
`@concern:dataplane-convergence`, serves that habit through this spec's graduation trigger: a green
gate reaches a device only through what the device follows. With fixture stores and no network, a
device declares `bin:epr` at `apply` and a runtime channel at `observe`; `show --render
runtime-config` prints the line the storage watcher then reads; a pin falling behind prints the
notice once; deleting the device's chain returns it to the committed base, not to an empty follow
set.

M decides nothing about what crosses devices. Slice B carries presence, heads and the rakia
artifact inventory between the steward's devices, and slice C the supervised restarts.
