---
id: steward-device-arrival-journal
title: Steward-device arrival — sprint journal and the laptop runbook
date: 2026-11-17
status: OPEN — train 2 landed and one device drew epr from a sibling; the laptop receipt is owed
author: shem session 76dc84be (Claude Fable 5.2), orchestrating; ethosengine session e57d1a58 (Claude Opus 5.5) on lane E and train 2
habits: [workspace-device-arrival, push-gate-pays-once]
---

# Steward-device arrival — sprint journal and the laptop runbook

## Why

On 2026-11-16 Matthew enrolled his daily-driver laptop (`think-p52s`: physical Linux, durable
`$HOME`, no `/projects`, outside Kubernetes) as a steward peer. He had to compile `epr` (25m34s),
carry public notes between machines by hand, and work round one Che assumption after another. The
laptop ended bound locally, unable to open a native session, unpublished, and with no verified
exchange. The steward asked for two things together: onboarding that an ordinary device can
complete, and the artifact store delivering `epr` itself between a person's devices.

## What landed on dev

| Commit | Device | What |
|---|---|---|
| `7c8cbb94e` | ethosengine | One device layout resolver (`epr device layout show --json`, `save`); a named install destination for queued fetches (`<bin_store>/incoming`, `install-destination-missing`); the envelope store accepts a model on re-open |
| `ef0b1db12` | shem | The eprfs gate no longer keys on `genesis/manifests/habits.yaml`; a home key is accepted on a declared, signed basis (`epr actor device persistence --declare durable-home`); a workspace container that announces itself is refused even with a declaration (enforced: elohim/eprfs/epr-cli/src/session_envelope.rs) |
| `521649ed0` | shem | Device key, device status and `berth` read the one layout (state directory added); `epr session open --model` from a live owner records the model, so commit attribution reads `measured` |
| `072d2bf62` | shem | Habit delta; backlog `steward-device-arrival-open-items` |
| `627627896` | shem | `roster bound` says whether the binding is published; a `sibling exchange` line; a last `next step` line; off a container the peer cure is `epr peer serve` |
| `367881e33` | shem | Backlog: a build-input view keys `epr` with no change to the generated-output scan |
| `ba165f566` | shem | `elohim/eprfs/bin-build-view.json` and a test that fails when a build of `epr` reads a tracked file outside it |
| `e6f4523b8` | shem | The cargo-pool library reads the saved device layout where no container pool exists, so gate scripts need no `CARGO_TARGET_POOL_ROOT` on a person's own machine |

| `a51dacfc8` | ethosengine, shem | Train 2: `epr bin follow` and `bootstrap` draw a sibling's release before compiling; `epr artifact put --release --release-bin`; scanner hardening; shem's review fixes (a release name cannot leave `incoming`; a host that builds reads nothing new; a store refusal falls back to a build) |

Gate cost on shem, for the record: eprfs ran in full six times (1741, 1562, 1537, 1313 s and two
runs that did not land); two docs-only landings took 45 s and 43 s.

## The first live draw (2026-11-17)

Shem recorded `epr` (153 MB) and `bin-launch` (13.5 MB) at `a51dacfc8` with `epr artifact put
--release bafyreic4kqcvcmg5ivr7lrhuxv7xp4cgopcwf7t3qjq6otlh7ttds5kzoa --release-bin <name>
--cross`, using the build-input view and the gate's recorded environment. Ethosengine, at the
same commit with `epr peer serve` running, ran `EPR_FOLLOW_SIBLING=1 epr bin follow --tool epr`:

```
follow: drawing epr from a sibling first: EPR_FOLLOW_SIBLING=1
follow: drew epr release bafyreic4kq…5kzoa from did:key:z6Mkm5de…bowj in 5.8 s
```

The release CID it ingested equals shem's, with `gateRun sibling:<shem>`, and the ring landed
with no local build of the release. Its first run drew and then refused `HEAD is not published
on dev` because its `origin/dev` was stale; after a fetch the second run landed in 5.05 s.

The producer ran by hand (`genesis/local-dev/steward-arrival/produce.sh` on shem, not tracked).
It is not yet a justfile leg, so a later landing offers nothing until it is repeated.

## Decisions for the steward

1. **Build notes.** Two devices pushing `refs/notes/brit/build/*` diverge; whichever pushes first
   wins and the other is rejected. No one has guessed a merge strategy over signed attestations.
2. **Stasis incidents** `76dc84be:12`, `:101`, `:249` and `:308` are parked.
3. **Shem's own `epr`** is still an old pin, so its commits print `UNMEASURED`. `epr device
   arrive` cures it and restarts the supervised peer daemon; it was left for the steward.
4. **The first `epr` on a device** (backlog row 15): the draw lives inside `epr`, so the laptop
   still needs one from somewhere. Either one local build as below, or a decision on a
   first-epr path.

Decided by the orchestrator on 2026-11-17, for the record: the three scan skips were dropped
and the hardening kept; ethosengine's branch crossed as `claude/steward-arrival-t2b` and shem
integrated and landed it.

## The laptop runbook

Nothing here has been run on the laptop. Each step names what it should print.

The laptop keeps its existing key (`~/.config/elohim/device/`), its local binding and its roster
commit `c6a5f57fc`. Do not re-enroll.

1. **Bring the checkout to dev** (`a51dacfc8` or later), keeping the local roster commit on top.
2. **Get an `epr` built from that commit.** The laptop's installed `epr` predates the draw, so
   this is still a local build, once. (Copying shem's binary by hand is not the store. If you
   try a draw later, first check `ldd --version` prints 2.39 or newer: shem's `epr` needs it,
   and an older library means the draw fetches, fails its probe and builds here.)

   ```bash
   env RUSTFLAGS= RUSTUP_TOOLCHAIN=1.100.0 CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 \
     CARGO_TARGET_DIR="$HOME/.cache/elohim-cargo-pool/family/bootstrap/elohim__eprfs" \
     cargo build --locked --release --manifest-path elohim/eprfs/Cargo.toml \
     -p elohim-epr-cli --bin epr --features blob-store
   ```

   Install the result where the old one is (`~/.cargo/bin/epr`), keeping the old binary aside.
3. **See where the device keeps its things:** `epr device layout show`. Expected on the laptop:
   config under `~/.config/elohim/device`, bin-store under `~/.local/share/elohim/bin-store`,
   state under `~/.local/state/elohim`, no cargo pool, the checkout it stands in. If the
   bin-store used on 2026-11-16 is elsewhere, name it once with `BIN_STORE=<dir> epr device layout
   save --field binStore`, so later shells need no variable.
4. **Read the status:** `epr device status --root <checkout>`. Expected: `degraded device key …
   native sessions cannot open here (envelope-persistent-config-required)` with the declare cure;
   `degraded roster bound … local only (in a local commit), not on origin/dev`; and a last line
   `next step: device key — …`.
5. **Declare the home durable, once:** `epr actor device persistence --declare durable-home`.
   It writes `persistence.json` beside the key and prints the basis. Status then reads `ok device
   key … (persistence declared-durable-home)`.
6. **Open the session:** `epr session open --model <the model in use>`. This is the step that
   refused on 2026-11-16.
7. **Publish the roster rows through the normal gate.** The pre-push hook builds the rust-style
   checker into the cargo pool. Name the pool once, and the gate scripts find it from then on:
   `CARGO_TARGET_POOL_ROOT="$HOME/.cache/elohim-cargo-pool" epr device layout save --field
   cargoPool`. `berth` finds its store through the layout and needs no `BERTH_DIR`. After the
   push and a fetch, status reads `ok roster bound … published on origin/dev`.
8. **Start the peer when wanted:** `epr peer serve --check`, then `epr peer serve`. The check
   prints the layout and refuses before launch if the install destination cannot be created.
9. **On a sibling, pull dev,** so it admits the laptop. Then on the laptop `epr device status`
   should move `sibling exchange` from `unknown` or `degraded` to `ok … acknowledges this device's
   head`, and the last line to `nothing owed`.
10. **Stop and start once more.** Same device key, same endpoint, no new authorization.
11. **Draw the next `epr` from a sibling.** After a later landing for which shem or ethosengine
    has put a release, with the peer running and `origin/dev` fetched: `epr bin follow --tool
    epr`. With no cargo pool saved it draws first without a variable; with one saved (step 7),
    set `EPR_FOLLOW_SIBLING=1`. Expected: `follow: drew epr release … in N s`. This is the
    laptop receipt the sprint still owes.

After this sprint the 2026-11-16 override list (`EPR_BIN`, `BIN_STORE`, `CLAUDE_PROJECT_DIR`,
`CLAUDE_CONFIG_DIR`, `BERTH_DIR`, `CARGO_TARGET_POOL_ROOT`) should be unnecessary in an ordinary
shell once the bin-store and the pool are saved in the layout; report any that is not. The
toolchain variables for the one bootstrap build (`RUSTUP_TOOLCHAIN`, `RUSTFLAGS`,
`CARGO_BUILD_JOBS`) remain until the sibling draw replaces that build.

## What the sprint measured, and what it did not

- A habit delta in the root `.epr-meta` plus a backlog edit lands in 45 s, because it selects no
  gate. That is not the case the fix was for. The case that cost 1323 s on `9be1cb048` was a
  habit atom inside `elohim/eprfs`, which selects the eprfs gate and then depends on its key.
- The key is not stable across ordinary work: eprfs was green at `ba165f566`, and three commits
  later (this journal, the cargo-pool library, a backlog edit) `GATE_EVIDENCE=reuse just gate
  eprfs` missed and began a full run. `epr-cli/tests/flow_cites.rs` reads all of `genesis/docs`
  and `.claude/memory`. The `habits.yaml` fix removes one cause; any document commit is another
  (backlog row 7, with a proposed split of the live-corpus legs into their own gate).
- Commit attribution: `measured` on shem session 76dc84be with the landed code, run with the
  gate-built binary.
- The device-arrival story: 15 scenarios, 115 steps, green against the landed build.
- Build baselines for the sibling draw to beat: 25m34s on the laptop, 29m48s on shem (cold).
- The live draw: 167 MB in 5.8 s between shem and ethosengine. `epr artifact put` with the
  build-input view succeeded for both bins.
- Not measured: anything on the laptop; a draw on a host with no cargo pool; a cold release
  build against the draw.

## Mistakes worth keeping

- The first push of the layout wiring went red in eprfs: the lane had run library tests only and
  an integration test asserted the status-name order. Since then every lane runs the full
  two-crate suites before reporting.
- The first push of the view test went red in the orchestrator gate: its `gate-reads`
  declaration sat in a helper, and the audit wants it beside each place the repository root is
  resolved. The audit test is a seconds check and had not been run.
- Killing a `git push` does not stop its hook. After one such kill the eprfs gate ran on and
  recorded a pass for a commit while the same worktree was being edited. Stop a run with
  `gate-live.mjs stop` before the long gate starts, or wait for the hook to exit.
- The first landing push of train 2 went red after 75 minutes on a sound commit: a session-start
  `cargo-pool enforce` deleted a build unit under the running `elohim-storage` doctest step.
  Fixed in `pool-lib.sh` (backlog row 13).
- The orchestrator told ethosengine to push a transport branch through the full local gate; it
  cost over an hour for a branch only shem would read (backlog row 9).
- Brit: `brit rebase` failed with "Directory staging is not supported"; `brit commit` refuses
  after a completed git rebase because of a stale `AUTO_MERGE`.

Open rows with probes: `genesis/data/timeline/backlog/steward-device-arrival-open-items.md`.
