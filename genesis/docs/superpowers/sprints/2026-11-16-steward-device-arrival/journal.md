---
id: steward-device-arrival-journal
title: Steward-device arrival — sprint journal and the laptop runbook
date: 2026-11-17
status: OPEN — seven landings on dev; the laptop receipt and the sibling draw of epr are owed
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

Gate cost on shem, for the record: eprfs ran in full six times (1741, 1562, 1537, 1313 s and two
runs that did not land); two docs-only landings took 45 s and 43 s.

## What is built and not landed

Ethosengine, local branch `feat/steward-arrival-t2`, unpushed. Its permission layer refused to
record a grant the steward had stated to shem and not to it; it builds and tests, and lands
nothing until the steward confirms there.

- Consumer half of the sibling draw: `epr bin follow` and `epr bin bootstrap` look for a
  sibling's crossed Produce at the published head, fetch by output CID into
  `<bin_store>/incoming`, ingest with `gate_run sibling:<did>`, refuse unless the release CID
  equals the producer's. Sibling-first where the host cannot build; elsewhere behind
  `EPR_FOLLOW_SIBLING=1`. Tested with an injected fetch (positive draw, tampered release refused).
- Scanner hardening from three reviews: raw C strings (`cr"\"`) mis-lexed, a pre-existing bypass
  of the whole generated-output scan; a leading shebang or byte-order mark; flags that enable
  `cfg(test)` unseen; a tracked symlink from `src/` into `tests/`.
- Three scan skips (top-level `#[cfg(test)]` modules; `tests/`, `benches/`, `examples/`; a
  narrowed literal rule marked PROPOSAL). The build-input view makes all three unnecessary.

## Decisions for the steward

1. **On ethosengine, state the train 2 grant directly** (write-set: `epr-cli/src/bin.rs`,
   `epr-cli/src/artifact.rs`, `epr-cli/src/artifact/**`, `epr-cli/tests/artifact.rs`, `justfile`).
   Then it rebases onto `ba165f566` and lands.
2. **Drop the three scan skips, keep the hardening.** Recommended. With
   `--sources elohim/eprfs/bin-build-view.json#bin-build`, `describe` and `key` succeed for `epr`
   and `bin-launch` on unmodified scan code. The skips were ruled by the session model in the
   steward's absence and loosen an integrity check; nothing needs them now.
3. **Build notes.** Two devices pushing `refs/notes/brit/build/*` diverge; whichever pushes first
   wins and the other is rejected. Shem's verdicts for `ef0b1db12` onward are local only. No one
   has guessed a merge strategy over signed attestations.
4. **Stasis incidents** `76dc84be:12` (silence stall), `:101` (a rebase, later finished at the
   steward's word) and `:249` (raised by the Stop hook with no description) are parked.
5. **Shem's own `epr`** is still the old pin, so its commits print `UNMEASURED` although the
   landed code measures. `epr device arrive` cures it and restarts the supervised peer daemon,
   which carries the berth notes between devices; it was left for a waking steward.

## The laptop runbook

Nothing here has been run on the laptop. Each step names what it should print.

The laptop keeps its existing key (`~/.config/elohim/device/`), its local binding and its roster
commit `c6a5f57fc`. Do not re-enroll.

1. **Bring the checkout to dev** (`ba165f566` or later), keeping the local roster commit on top.
2. **Get an `epr` built from that commit.** Until the sibling draw lands this is still a local
   build, once:

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
   checker, which still takes its pool from the environment: export
   `CARGO_TARGET_POOL_ROOT="$HOME/.cache/elohim-cargo-pool"` for the push (the gate scripts do not
   read the device layout yet). `berth` now finds its store through the layout and needs no
   `BERTH_DIR`. After the push and a fetch, status reads `ok roster bound … published on
   origin/dev`.
8. **Start the peer when wanted:** `epr peer serve --check`, then `epr peer serve`. The check
   prints the layout and refuses before launch if the install destination cannot be created.
9. **On a sibling, pull dev,** so it admits the laptop. Then on the laptop `epr device status`
   should move `sibling exchange` from `unknown` or `degraded` to `ok … acknowledges this device's
   head`, and the last line to `nothing owed`.
10. **Stop and start once more.** Same device key, same endpoint, no new authorization.

Still needing a hand-set variable on the laptop after this sprint: `CARGO_TARGET_POOL_ROOT` for
any push that selects a cargo gate. Everything else in the 2026-11-16 override list
(`EPR_BIN`, `BIN_STORE`, `CLAUDE_PROJECT_DIR`, `CLAUDE_CONFIG_DIR`, `BERTH_DIR`) should be
unnecessary; report any that is not.

## What the sprint measured, and what it did not

- A habit delta in the root `.epr-meta` plus a backlog edit lands in 45 s. The case that cost
  1323 s on `9be1cb048` was a habit atom inside `elohim/eprfs`, which selects the eprfs gate; the
  key fix is unit-proven and that reuse path is still unmeasured on a real push.
- Commit attribution: `measured` on shem session 76dc84be with the landed code, run with the
  gate-built binary.
- The device-arrival story: 15 scenarios, 115 steps, green against the landed build.
- Build baselines for the sibling draw to beat: 25m34s on the laptop, 29m48s on shem (cold).
- Not measured: anything on the laptop; a draw through a live `epr peer serve`; `epr artifact
  put` with the build-input view; fetch against build.

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
- Brit: `brit rebase` failed with "Directory staging is not supported"; `brit commit` refuses
  after a completed git rebase because of a stale `AUTO_MERGE`.

Open rows with probes: `genesis/data/timeline/backlog/steward-device-arrival-open-items.md`.
