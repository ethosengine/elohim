---
id: "backlog-steward-device-arrival-open-items"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Steward-device arrival sprint: what four landings left open"
slug: "steward-device-arrival-open-items"
written: "2026-11-17"
author: "claude-fable-5-2 (shem, steward-device-arrival sprint, session 76dc84be)"
status: "open"
priority: "high"
tags: [device-arrival, artifact-store, gate, brit, berth, onboarding]
---

# Steward-device arrival sprint: what four landings left open

Matthew enrolled a physical laptop (durable `$HOME`, no `/projects`, outside Kubernetes) as a
steward peer on 2026-11-16 and had to compile `epr` for 25m34s to do it. The sprint that followed
landed four changes on `dev` across shem and ethosengine: `7c8cbb94e` (one device layout
resolver, install destination, envelope model re-open), `ef0b1db12` (eprfs gate key fix,
durable-home persistence contract), `521649ed0` (layout wiring into device key, device status and
berth; the session-open model fix). The habit `workspace-device-arrival` stays red: nothing has
been tried on the laptop yet. The rows below are what the sprint found and did not close.

## Row 1 — a device cannot yet draw `epr` from a sibling

`epr bin follow` takes bytes only from a record already in the local store, an explicit closure
directory, or a local `cargo build` (`device-runtime/src/bin_store/lifecycle.rs`). Train 2
(ethosengine, branch `feat/steward-arrival-t2`, unpushed on 2026-11-17) adds the consumer half:
`follow` and `bootstrap` look for a sibling's crossed Produce at the published head, fetch by
output CID into `<bin_store>/incoming`, ingest with `gate_run sibling:<did>` and refuse unless
the release CID equals the producer's. The producer half records the Produce where the bytes
are built, in the eprfs gate's ingest step.

Landed 2026-11-17 in `a51dacfc8` (consumer, `--release`/`--release-bin` on put, review fixes).
First live draw the same day: shem put `epr` and `bin-launch` at `a51dacfc8` with `--cross`
(release `bafyreic4kqcvcmg5ivr7lrhuxv7xp4cgopcwf7t3qjq6otlh7ttds5kzoa`), and ethosengine, through
its running `epr peer serve`, ran `EPR_FOLLOW_SIBLING=1 epr bin follow --tool epr`: both bins
(167 MB) drawn in 5.8 s, the same release CID ingested with `gateRun sibling:<shem>`, the ring
landed, no local build of the release. Logs on ethosengine:
`genesis/local-dev/steward-arrival-e/draw-follow*.log`.

Still open:

- The producer ran by hand (describe, key, the gate's own bytes, `put --release --release-bin
  --cross`, with `--sources elohim/eprfs/bin-build-view.json#bin-build` and the gate's recorded
  env). It is not in `_eprfs-ingest-around`, so the next landing offers nothing until someone
  repeats it. The justfile is in every root gate's key; the leg should land with its own test.
- No draw has run on a host with no cargo pool (the laptop), and none without the variable.
- No cold release build was timed against the draw. The only local figure is a warm
  incremental debug build of the consumer, 21.6 s on ethosengine, which is not comparable.

## Row 2 — no eprfs-view build could be keyed on any device

`epr artifact key` refused every method over the eprfs workspace: the generated-output scan
(`epr-cli/src/artifact/generated.rs`) lexes tracked Rust sources for string literals that resolve
into ignored generated directories, and it read its own unit-test fixtures and the workspace's
integration tests. Train 2 carries a bounded cure in separate commits (skip the body of a
top-level `#[cfg(test)] mod`, then files under `tests/`, `benches/`, `examples/` under five
conditions each). Two reviews on ethosengine found and closed: a build script printing
`cargo:rustc-cfg=test`; a pre-existing bypass of the whole scan through raw C strings
(`cr"\"`); a leading shebang or byte-order mark; `RUSTFLAGS=@file` and `--config`.
Residual named there: identifiers are matched as alphanumeric, not by XID.

This loosens an integrity check. It was ruled by the session model in the steward's absence and
lands only with the steward's confirmation.

**Finding 2026-11-17 (shem, read-only, `dev` 627627896, unmodified code): no skip is needed.** The
method's view was the whole eprfs workspace (`elohim/eprfs/build-manifest.json#rust-build-test`),
tests included, and the ignored generated directories (`elohim/epr/bindings`,
`elohim/eprfs/eprfs-ts/dist`) sit inside it. With a view limited to what a binary build reads, they
are not in the view and the scan has nothing to refuse:

- A scratch manifest outside the repository, one step, 69 entries: the workspace `Cargo.toml` and
  `Cargo.lock`, `.cargo/config.toml`, each member's and path dependency's `Cargo.toml` and
  `src/**`, `genesis/orchestrator/rust-style` and its `scan` crate, and
  `genesis/orchestrator/manifests/runtime/workspace-device.manifest.json` (the binary embeds it).
  No `tests/`, `benches/` or `examples/`.
- Under the mesh build's scrubbed environment (every `CARGO_*`/`RUST*` unset except `CARGO_HOME`
  and `RUSTUP_HOME`; `RUSTUP_TOOLCHAIN=1.100.0` and `PATH` recorded with `--env`):
  `epr artifact describe --project eprfs --bin epr --rustflags "" --crate elohim/eprfs/epr-cli
  --sources <scratch>#epr-bin-build` exits 0, and `epr artifact key` exits 0 with
  `key bafyreidul2t5c5…askm head 627627896`.
- A real `cargo build --locked -p elohim-epr-cli --bin epr` under that environment (29m48s cold,
  one job) wrote a dep-info naming 394 files inside the repository; all 394 lie inside the view.
  The first 66-entry view missed two (the embedded device manifest and the rust-style `scan`
  crate), which is what the dep-info check is for.

Not run: `epr artifact put` on that build (it writes a custody record and a build attestation),
and the same for `bin-launch`.

Recommendation for the steward: declare a build step for the `epr` and `bin-launch` binaries whose
sources are the build-input view (a manifest decision; step sources also drive CI dispatch), keep
the scanner hardening from the reviews as standalone integrity fixes (raw C strings, shebang and
byte-order mark, fail-closed flags, tracked symlinks), and drop the three skips (top-level
`#[cfg(test)]` modules, `tests/`/`benches/`/`examples/`, and the narrowed literal rule) as
unnecessary. The 69-entry list is derivable from `cargo metadata` plus the dep-info; a hand list
will drift, so the step's sources should be checked against a build's dep-info in the gate.

Landed since: the view as its own file, `elohim/eprfs/bin-build-view.json` (step `bin-build`),
outside any build manifest so it selects no gate and dispatches no pipeline, and
`epr-cli/tests/bin_build_view.rs`, which reads the dep-info cargo writes beside the `epr` binary
and fails naming any tracked file the build read that the view leaves out (negative control:
removing the embedded device manifest from the view fails the test with that path). So the
producer can pass `--sources elohim/eprfs/bin-build-view.json#bin-build` with no manifest
decision. Still the steward's: whether the three scan skips are dropped. Not covered by the test:
`bin-launch`'s dep-info (the test sees only its own package's binary).

### What this means for the native-dev-plane joins

Join F2 (a sibling fetches a binary from a peer's tier instead of rebuilding) is, for `epr`
itself: consumer built and tested on ethosengine with an injected fetch (positive draw and
tampered-release refusal); producer unblocked in principle by the build-input view above; the live
draw through `epr peer serve` and the fetch-versus-build wall-clock unmeasured. The build
baselines to beat: 25m34s on the laptop, 29m48s on shem (cold, one job).

## Row 3 — two devices' build notes diverge on origin

`git push origin 'refs/notes/brit/build/*:refs/notes/brit/build/*'` fast-forwards only for the
device that pushes first. Neither device has a notes fetch refspec or a merge strategy. After
ethosengine pushed 32 refs for `7c8cbb94e`, shem's push for `ef0b1db12` was rejected on about 40
refs; shem's verdicts for `ef0b1db12` and `521649ed0` are local only. No one should guess a merge
strategy over signed attestations. Needs a ruling: how per-device verdict notes combine.

Ruled 2026-11-17 (R4, `genesis/docs/superpowers/sprints/2026-11-17-native-throughput/journal.md`):
they do not combine. Built 2026-11-18 on `claude/nt-r4-device-notes`:
`genesis/orchestrator/device-notes-publish.mjs` (`just dev notes` dry run, `just dev notes
publish`) pushes local `refs/notes/brit/build/<step>` to
`refs/notes/brit/devices/<did>/build/<step>`, the DID in brit's ref segment encoding
(`did%3Akey%3Az6Mk…`, reversible, method-agnostic, the encoding step names already use). The
DID comes from `epr actor current --device`. Every destination the forge holds is compared first;
a rewound, diverged or unseen forge tip refuses the whole publish with a named cure, and the push
is `--atomic` with no `+`. The legacy shared refs are never a destination. The root gospel now
names this command instead of the shared push. Tests (`device-notes-publish.test.mjs`, scratch
forge) cover two devices publishing divergent notes with neither rejected, the shared-ref
rejection as a control, rewound and diverged refusals, and brit push as the transport
(`DEVICE_NOTES_PUSH=brit`, file remote only).

Still open:

- Nothing has been published to origin yet; the first real publish from each device is the receipt.
- No reader fetches the device mirrors. Verified 2026-11-18: origin's only fetch refspec is
  `+refs/heads/*`, Jenkins runs `brit-helper.sh verify` and `plan --since origin/dev` on a heads
  checkout, and `delivery-series.mjs`, `epr flow walk`, ark's `brit_index` and brit read local
  `refs/notes/brit/build/` only. Sibling verdicts cross on the peer channel.
- The publish includes the `ci:*` notes the harvest writes (`brit-attest.mjs`), as the shared push
  did. Whether a device mirror should carry only `gate:*` is not ruled.
- The frozen shared refs on origin are not deleted; deleting them needs the steward.

## Row 4 — a fresh worktree's preflight refusal does not stop the push (by declared policy)

Both devices pushed from a fresh worktree on 2026-11-16 and met a preflight `REFUSED` (first the
uninitialised submodules, then the missing `elohim/sdk/storage-client-ts/dist/index.js`). The
refusal was printed, with its cure, in the first seconds. The push carried on because
`gate_on_red` in `genesis/agentic/pool-policy.json` is `continue`: one run reports every red and
records every green, and the live outlook line names the cost of continuing and the stop command.
On ethosengine that run took 4,490 s and ended refused; its twelve green verdicts were then
reused and the re-push took 295 s, so the time was not lost. Shem had run `just dev prep` first.

Corrected reading (the first version of this row called it a gate defect; it is not): the check
exists and is live. What is missing is habit: `just dev prep` before the first push from a new
worktree, and an agent that acts on the outlook line. Open question for the steward only if this
recurs: whether a preflight refusal, whose cure is seconds and after which the push cannot land,
should stop the run even under `continue`.

## Row 5 — Brit gaps met while rebasing

- `brit rebase origin/dev` failed with "Directory staging is not supported; name explicit files"
  and left no active rebase.
- After a completed `git rebase`, `brit commit` refuses with "merge metadata exists without
  MERGE_HEAD" because of a stale `AUTO_MERGE` in the worktree's git directory.

## Row 6 — path guessers still outside the layout

`genesis/agentic/bin/berth` `envelope_directory()` restates the device-key path rule, and
`genesis/agentic/shift_lease.py:215` keeps its own `/projects/.claude-config` default. The gate
looks for `ark` in the pushing worktree's own pool family and falls back to the direct path when a
new family has none.

The cargo-pool library now reads the saved layout where no container pool exists (`e6f4523b8`:
the variable, else `/projects/.cargo-target-pool` where that tree exists, else
`epr device layout save --field cargoPool`, else the old default).

**No gate runs the pool library's tests.** That push selected only `address-profile-lint`
(18 s). `_gate-agentic-shift` runs `shift*_test.py` and `berth_test`; `pool_preflight_test.py`,
`pool_artifact_hashes_test.py`, `lsp_attach_test.py` and `lsp_flycheck_test.py` (36 tests that
source `pool-lib.sh`, a library every session hook sources) run in no gate and passed locally
only. Adding them means editing the `justfile`, which is in every root gate's key. Probe: a
push that changes `genesis/agentic/bin/pool-lib.sh` selects a gate that runs those four files.

## Row 7 — eprfs keys on the whole documentation tree and on the memory directory

Three live-corpus legs inside the eprfs cargo gate declare reads far wider than the workspace:
`epr-cli/tests/flow_cites.rs` reads `genesis/docs`, `.claude/memory` and every `CLAUDE.md` (it
verifies the live documents' cites); `flow_scope.rs` reads `genesis/docs/superpowers/specs`,
`plans`, `held`, `genesis/docs/plans`, `genesis/docs/held` and `genesis/a2o/features`;
`live_manifest_integrity.rs` reads every `.epr-meta` manifest. The reads are honest: those tests
do open those files. The cost is that any documentation commit moves the eprfs key.

Measured 2026-11-17 on shem: eprfs was green at `ba165f566`; three commits later (a sprint
journal under `genesis/docs/superpowers/sprints/`, the cargo-pool library, a backlog edit),
`GATE_EVIDENCE=reuse just gate eprfs` printed `evidence: miss … not reused: no attestation for
this closure yet` and began a full run. The docs-only pushes themselves took 43 to 45 s because
they do not select eprfs; the next push that does select it pays 26 minutes whether or not any
Rust changed. So the `genesis/manifests` fix in `ef0b1db12` removes one cause (a habit delta
rewriting `habits.yaml`), and the saving holds only while no document changed since the last
green. On a normal working day that is rare.

Proposed shape (not built; it changes a build manifest and the `justfile`): the live-corpus legs
become their own gate project, a seconds-scale `cargo test --test flow_cites --test flow_scope
--test live_manifest_integrity` (or the `epr` commands they wrap) keyed on those document reads,
and the workspace gate keeps the fixture tests and keys on the workspace. Then a documentation
commit costs seconds and a Rust commit is not re-run for prose.

Unexplained, and a trap if `GATE_NARROW=on` is ever enabled: the first two pushes of the sprint
(ranges `9be1cb048..5ada8a1b4` and `7c8cbb94e..ef0b1db12`, both changing
`epr-cli/src/{actor,device_key,session_envelope}.rs`) printed `eprfs narrow (shadow):  (0 of 13
members)`, which under `on` would have tested no package. Re-running `affectedPackages` offline
on the second range with the same manifest returns `["elohim-epr-cli"]`, and two later pushes
from the same worktree printed correct sets (3 of 13, 1 of 13). The hook's changed-file list for
those two runs was not kept, so the cause is not established; `pkg.dir` would come out as
`.worktrees/<name>/…` and match nothing if the runner's root were the main checkout while cargo
resolved manifests in the worktree. Probe: keep `GATE_CHANGED_FILES` beside the run log, and add a
test that a non-empty changed list under a selected workspace never yields an empty package set
without a stated reason.

## Row 8 — the laptop itself

Owed before the habit can turn: on the laptop, an `epr` at `521649ed0` or later;
`epr actor device persistence --declare durable-home`; `epr session open`; the roster commit
`c6a5f57fc` published through the normal gate; a verified head exchanged with a sibling; a
start, stop and restart that keep the same device key.

## Row 9 — a branch that only carries work to the integrator pays the full local gate

Measured twice on ethosengine: `feat/steward-arrival-e` spent 75 minutes in local gates on
2026-11-16, and `feat/steward-arrival-t2b` spent over 60 on 2026-11-17 (eprfs 1,425 s, then
elohim-storage) while Jenkins ran nothing for either. The landing push on shem gates the same
commits again.

Two causes. The defer-to-CI rule only fires for a CI-built branch, and
`CI_BRANCH_PATTERN` in `genesis/orchestrator/gate-cost.mjs:9` accepts `feat-…` and `claude/…` and
not `feat/…`, the spelling both devices used. And the orchestrator asked for a fully gated
feature push where a transport was wanted (shem's instruction, corrected 2026-11-17).

Until changed: name a hand-over branch `claude/<name>` or `feat-<name>`. Probe before widening
the pattern: confirm in `genesis/orchestrator/Jenkinsfile` that a `feat/…` branch is discovered
and built; a pattern that defers to a build Jenkins never runs leaves the long gates unrun.
Steward's suggestion (2026-11-17): let forge branch namespaces model the governance boundaries
now, and graduate each to a native peer channel. He named `stewarded/`, `compat/` and `dev2/` as
examples and asked shem for a design aligned with the native-dev-plane spec. Shem's proposal,
not ruled:

**One rule.** A branch name states the reach of the work and which rung is asked to carry it
(spec §4). It never selects a weaker gate. A verdict is a derivation record keyed by closure: it
is paid once, by whichever admitted device runs it, and it is demanded only where a head others
adopt moves, which today is `dev`. A push that moves no adopted head demands nothing.

| Namespace | Rung and reach | What the push does today | Native form it retires into |
|---|---|---|---|
| `dev` | the space head's mirror | full closure of verdicts, from this device or an admitted sibling | F3: a compare-and-swap on the signed space head; the forge only fast-forwards |
| `self/<name>` (the steward's `stewarded/`) | R1, one steward's devices, reach `self` | preflight and seconds checks; records a hand-over intent naming the head; gates already run on the sender cross as sibling verdicts, so the integrator pays only what nobody ran | A7b: a sibling's git objects over the peer channel, the intent on `epr peer intents`; the branch becomes an optional mirror |
| `carry/<name>` (the steward's `dev2/`) | R2, an enrolled carrier (Jenkins, a think-p node), reach `self`, zero independence | the existing defer-to-CI rule, under a name that says what it is; `feat-…` and `claude/…` are this lane today | F4: one `TaskSpec` offered over the peer channel, the result a record with basis `UnderGrant`, `NotCarried` when no carrier answers |
| none for `compat/` | a build for another architecture is a different method CID, not a lane | `carry/` with the method named (`epr artifact describe --target-isa …`) | the same F4 offer; placement matches the method to a device that can run it |
| `trusted/` and wider | R3 and up | not offered: no second principal with standing exists (spec §5, F6) | the graduation verb |

Graduation steps, each with its receipt:

1. Names only. The hook reads the prefix. Receipt: a `self/` hand-over in seconds, and the
   integrator's landing printing `independence: sibling` for every gate the sender ran.
2. The hand-over is a record, not prose over berth: head, closure keys and verdict CIDs as a
   landing intent both devices read. Receipt: the integrator lands from `epr peer intents` alone.
3. Carriers fold records back. Jenkins writes a derivation record for what it built, so a
   deferred gate ends as a reuse hit on the device and not as a promise. Receipt: a local hit
   whose signer is the carrier.
4. Objects cross natively (A7b) and offers replace branches (F4). Receipt: the spec's section 0
   test, a hand-over and a carried build with the forge unreachable.

What this would have changed on 2026-11-17: ethosengine's push carries no gate and takes
seconds; its eprfs pass (1,425 s) is a sibling verdict shem reuses at landing; elohim-storage
runs once, on a carrier or at landing, not twice. Unverified and owed before step 1 is trusted:
that a sibling verdict recorded in a linked worktree crosses (G1 on 2026-11-14 crossed from the
main checkout), and what state A7b is in.

## Row 10 — a red in the first minutes does not stop the long gates (second occurrence of row 4)

2026-11-17, ethosengine: the orchestrator gate went red about 5 minutes into the t2b push and
elohim-storage was still running 50 minutes later toward a push that could not land. Row 4 said
to raise this with the steward if it recurred. It recurred. Proposal for his ruling: under
`gate_on_red: continue`, a red whose gate is cheap (p90 under 600 s) and after which the push
cannot land stops gates that have not started; gates already running finish and record. Smaller
step that needs no ruling: order the run so cheap gates finish before a long one starts.

Steward's direction (2026-11-17): a red is a subscribable event wherever the gate runs, and the
verdict still comes at the end so one run reports every red. Locally this exists (`gate-tap.mjs`,
the `gate-live-inject` hook, `gate-live.mjs follow`; habit `gate-failures-arrive-live`). The run
log stays on the device that ran the gate, so shem learned of this red 52 minutes late, in
prose. Owed, in order: (a) a sibling's reds reach the other device's agents over the peer
channel, the same event shape, so an orchestrator hears a delegate's red as it prints; (b) a
Jenkins run exposes the same events as a WebSocket subscription; (c) rakia's executor emits
them natively, so local, sibling and carrier runs are one stream with one reader.

The stream is shaped for a subscriber's attention (steward, same day): two events are owed to
every subscriber and the rest are opt-in. The floor is the first red of a run, sent the moment
it prints, with the gate, the line and the cost of continuing. The ceiling is one thin final
report at exit: the verdict, each red once, what was reused and what ran. Reds between the two
are available to a subscriber who asks for them and are never pushed by default, so a watcher
is interrupted at most twice per run.

A subscriber states its intent when it subscribes (steward, same day): "for this run, every
red", or every nth event, in place of the default two. Asking up front is how the stream
respects the watcher's attention without guessing it. One event is owed by every
subscription with no opt-out: the subscription itself, recorded as an EPR, naming who asked
for what on which run. That record is the floor governance reads to judge whether the
watcher was suitably cared for and whether the watching was abuse. It is near-frictionless
among one person's own devices; its friction is meant to rise with reach (a collective, a
repository, a deployment). Not designed here: the record's shape, where it resolves, or what
the friction is at each reach.

## Row 11 — `just dev prep` exits 0 with an empty submodule checkout

2026-11-17, ethosengine worktree `steward-arrival-t2`: prep could not read rakia's pinned tree
(`f9787c9e`), left `elohim/rakia` with every file staged as deleted and `sophia` without its
`build-manifest.json`, and exited 0. The orchestrator gate then failed with
`registry.get(name).run` undefined, an hour into a push. Reported by ethosengine, not reproduced
on shem. Probe: after prep, `test -f <submodule>/build-manifest.json` for each attested
submodule; prep and `gate-preflight.mjs` should refuse naming the submodule and the fetch cure.

2026-11-18, ethosengine, two more occurrences in the native-throughput sprint. (a) A fresh
worktree `nt-e2`: the main checkout's `elohim/rakia` did not hold the pin `f9787c9e`, so the
local-reference clone failed `unable to read tree`; prep by then refused (exit 1), and the cure
was a hand fetch of the pin from the `nt-e1` worktree's rakia. (b) After a rebase moved the
`elohim/brit` gitlink (0.2.0 to 0.2.6), `nt-e1` kept its initialized brit at the old commit;
prep only warned `checked-out commit differs from the pin` and exited 0, and the stale gitlink
widened gate selection and kept the genesis evidence dirty. Cured in
`genesis/orchestrator/worktree-prep.mjs` (branch `claude/nt-prep`): when the main checkout
cannot serve a missing submodule, prep tries every sibling worktree's copy that holds the pin,
over local file transport only, and names the one it used; with none, it refuses naming each
checkout tried and the remote cure. A clean initialized submodule at another commit is moved to
its pin (fetching the pin from a sibling first when its own copy lacks it); a dirty one keeps
the warning and is left untouched. Network fetch stays out of prep by design. Tests:
`worktree-prep.test.mjs`, six `row 11` cases.

Live receipt, 2026-11-18, worktree `nt-dod2`, the exact shape this row opened with: the main
checkout's rakia lacked `f9787c9e`, prep named the sibling holding it, and the retry failed again
because the first attempt's half-made submodule repository was reused and every file was left staged
as deleted. Three more cures, each tested: the retry first fetches the pin into that leftover
repository; the moved set is read from `git submodule status` itself (the preflight row stops at
the first missing submodule, sophia, and never named rakia); and a checkout whose every status line
is a staged deletion is an aborted checkout, not work, so it is restored to its pin with
`submodule update --force`. The fourth prep exited 0 with rakia at its pin and gate selection back
to the two projects the branch touches.

## Row 12 — the sibling draw fetches before it reads what the binary requires

2026-11-17, landed in `a51dacfc8`: `sibling::select` picks an offer by commit and release only.
A device learns that a sibling's `epr` is not for it by fetching 153 MB and failing the ingest
probe; since `3af74caed` that falls back to a local build (`store-refused`) and no longer
refuses the follow. The record to decide earlier exists: `epr artifact put` extracts the ELF
requirements and `epr artifact requires --output CID` reads them, but only from the device's
own store, since requirements are not yet served to siblings (K3 part 2). Shem's `epr` at
`a51dacfc8` needs glibc 2.39 or newer and the baseline x86-64 instruction set (`readelf`,
`objdump -T`, 2026-11-17). Owed: requirements cross with the Produce, and `select` skips an
offer this device cannot run, printing which requirement failed. Probe: on a device with an
older glibc, `EPR_FOLLOW_SIBLING=1 epr bin follow --tool epr` prints a requirement gap and
fetches nothing. The steward's framing (same day): an artifact must be honest about what it
needs, so a device can refuse it from its own statement before running an instruction.

## Row 13 — the pool sweep deleted a build unit under a running gate

2026-11-17 17:02Z, shem: the session-start hook (`.claude/scripts/clean-caches.sh`) ran
`cargo-pool enforce --yes` while the landing push's `elohim-storage` gate was in its test
phase. Hash-gc removed `build/cozo/235c866c800aacef` (older than the six-hour floor), and the
doctest step failed 29 minutes in with `extern location for cozo does not exist`. The gate
recorded a red for a commit whose code was sound; the re-push passed (1385 s). Cause:
`family_busy` saw only cargo's build lock and a working directory inside the family, and
`cargo test` holds neither while tests run. Fixed here: a live process with
`CARGO_TARGET_DIR` inside the family makes it busy (`genesis/agentic/bin/pool-lib.sh`; probed
against a scratch pool: idle 1, live 0, other family 1). Not fixed: a process this user
cannot read is unseen; the superseded-unit grouping (target, path, features, profile) may treat
two live variants of one crate that differ only in their dependency graph as old and new
(inferred from the code, not reproduced); and
the red verdict for `a51dacfc8` stays in the gate history. No test covers `family_busy`.

## Row 14 — the draw runs before the published check

2026-11-17, ethosengine, first live draw: with `refs/remotes/origin/dev` stale, `follow` drew
167 MB and then refused `HEAD is not published on dev`. `brit fetch origin dev` had not moved
the remote-tracking ref; `brit fetch -r origin` did. Owed: check published before drawing, or
name the stale ref and its cure in the refusal. Probe: with a stale `origin/dev`,
`EPR_FOLLOW_SIBLING=1 epr bin follow --tool epr` fetches nothing and names the ref. The fetch
behaviour is a Brit gap for row 5.

## Row 15 — the first `epr` on a device still has to come from somewhere

The draw lives inside `epr`. A device whose installed `epr` predates `a51dacfc8`, or that has
none, cannot draw; ethosengine built the consumer locally before its draw, and the laptop's
runbook still starts with one local build. Owed: a first-epr path that needs no `epr` (a small
fetcher, or `bin-launch` carrying the draw), decided against spec §5 before it is built.

## What the native-throughput sprint changed (2026-11-17, `bea497110`)

Journal: `genesis/docs/superpowers/sprints/2026-11-17-native-throughput/journal.md`.

- **Row 1.** The producer is in `_eprfs-ingest-around` (`genesis/orchestrator/eprfs-produce.sh`,
  test `eprfs-produce.test.mjs`). It ran in the gate of the `bea497110` landing and put both
  bins with `--cross`. Still open: a draw on a host with no cargo pool, and a draw without the
  variable.
- **Row 3.** Ruled (R4): each device publishes its build notes to
  `refs/notes/brit/devices/<device-id>/build/*`; no forge-side reader of the shared refs was
  found. Not built.
- **Row 6.** The four pool and LSP test files run in `_gate-agentic-shift`, and a change to
  `pool-lib.sh` selects that gate.
- **Row 7.** The live-corpus legs are the `eprfs-live-corpus` gate project, keyed on their own
  reads through `// gate-reads(<project>): <path>`. Warm wall time 58 to 60 s. Still open: the
  empty narrow set under `GATE_NARROW`.
- **Row 9.** Step 1 is built: a push whose every destination is `refs/heads/self/*` keeps the
  seconds checks, skips the project gate recipes and records the hand-over. Still open: the
  first timed hand-over, whether Jenkins builds `self/*` (the repository does not show the
  controller's branch filter), and steps 2 to 4.
- **Row 13.** `family_busy` has a test (idle, live, a family named with the same prefix).
- **Row 15.** Ruled (R5): a forge release asset carrying the exact pair, verified against a
  steward-authenticated pin before first run. Not built.

## Row 16 — a warn-only leg prints `REFUSED`

The security-paydown leg prints `REFUSED <manifest>: N patchable alert(s), no paydown` for each
manifest and returns success unless `SECURITY_PAYDOWN=strict`. An orchestrator read six such
lines as a push that could not land and stopped the run. Probe: a warn-only leg's lines say
`WARN`, or say on the line that the push is not bound by them.

## Row 17 — a stale exported `EPR_BIN` shadows the device's head

A terminal that exports `EPR_BIN` to an old store path makes every `epr` in that session the old
binary; `epr bin status` says only `candidate`. On 2026-11-17 this left commit attribution
`UNMEASURED` and the CLI different from the peer daemon until commands ran with it unset. Related:
`epr bin land` needs a clean checkout whose HEAD is the commit, so a release already on
`origin/dev` cannot be landed from a main checkout that is behind or dirty. Probe: `epr device
status` names an `EPR_BIN` that resolves to something other than the head.

## Row 18 — the pool family follows the branch prefix

A branch named `claude/<name>` builds in pool family `claude`, cold, while the worktree's
previous branch `feat/steward-arrival` had a warm `steward` family. The first
`eprfs-live-corpus` run compiled for 9 min 18 s for that reason alone. Probe: a worktree keeps
its family across a branch rename, or the gate prints that it is starting a cold family and
names the warm one.

## Row 19 — the governance fence refuses a sibling's verdict for the whole push

`SIBLING_REUSE_GOVERNANCE_PATHS` (`genesis/orchestrator/gate-evidence.mjs:442`) is read as one
diff over the push. A change to `epr-cli/src/peer.rs`, `eprfs-peer/**`, any `gate-*.mjs`,
`pool-policy.json` or `measures.yaml` refuses sibling reuse for every project, including ones
whose tests cannot read the changed file. Landing 2 of the native-throughput sprint paid 162
minutes on shem for a head ethosengine had gated in 48. Until the steward rules, the device
that gated a fenced head lands it (ruling R6 in that sprint's journal). Decision for the
steward: keep the fence whole, or scope it per project by the gate's own input closure. Probe
if scoped: a `peer.rs` change reuses a sibling's `elohim-storage` verdict and still runs `eprfs`.

A read-only Codex review (gpt-6.2-astra, 2026-11-18, receipt
`genesis/local-dev/native-throughput/codex-fence-design.receipt.md`) recommends a split, and
only after one thing is proven. Paths that change how a verdict is produced, keyed, enrolled or
named (`gate-*.mjs`, the compiler gate inputs, `pool-policy.json`, `measures.yaml`) stay fenced
for the whole push. A change confined to `peer.rs` or `eprfs-peer/**` would be scoped to the
projects whose closure reads it, with `eprfs` and `orchestrator` always run locally. The
condition: a crossed verdict names no executable today, so the fence reads source history and
cannot say which `epr` and which daemon emitted, folded and read the row. A candidate `peer.rs`
selected through `EPR_BIN` could read a stored red as green. The receipt owed before narrowing
is a two-device run that names the builds at emission, crossing, fold and read, and refuses a
candidate override and an unknown build. On landing 2 the scoped rule would have saved 75
minutes (elohim-storage and steward-node) and still run eprfs. The review also notes that two
specs carry a decision D5: the 2026-10-27 landing-plane design is the one that requires local
judgment here.

## Row 20 — five gates write no verdict, so no device can reuse or list them

`epr-storage`, `rust-style`, `rakia-validate`, `pipeline-list-fresh` and `cargo-coverage` print
`evidence: unavailable (gate-only inputs, no step fingerprint)`. `epr-storage` cost 1272 s on
landing 2 and is not enrolled for reuse; `genesis-a2o` (229 s) and `cargo-bridge` (63 s) record
a verdict and are not enrolled either. Probe: each has a step fingerprint and a `gate_reuse`
entry, or a written reason it cannot.

## Row 21 — the hand-over record omits reused verdicts

`handover-record.mjs` reads a build note at the pushed head only. A verdict that stood in from
an older commit with the same closure is not listed, so the record under-reports what the
sender holds. Probe: the record lists a verdict by closure, with the commit it was minted at.

## Row 22 — two devices never compute the same artifact key for one commit

At `6616ce547` both devices put `epr` from their own gate build, and the two Produces differ in
method and in input (shem method `bafyreibxw36w55…`, ethosengine `bafyreiadpm2vjn…`), so a key
hit across devices did not happen: the draw works by release at a head, never by key. The cause
is not established. The producer (`genesis/orchestrator/eprfs-produce.sh:62-71`) already skips
`CARGO_TARGET_DIR`, `CARGO_HOME` and `RUSTUP_HOME`; the method still carries the literal ordered
`PATH`, the full `rustc -vV`, and any set `CC`, `AR`, `PKG_CONFIG_PATH` and similar, and the
input carries resolved compiler, linker and startup-object paths with their content CIDs
(`epr-cli/src/artifact.rs:408,553,714`, `artifact/host.rs:56`). Whether the two builds are
byte-reproducible is not shown, because the declared inputs differ. `epr artifact inputs diff`
cannot compare the two today: it requires the same method and commit, or the same key
(`artifact/inputs.rs:178,259`). A read-only Codex review (gpt-6.2-astra, 2026-11-18) is at
`genesis/local-dev/native-throughput/codex-key-design.receipt.md`.

Probe: `epr artifact inputs diff` accepts two Produce keys, verifies each document against its
CID and prints every differing field or line with whether it can change the bytes. Then, once
two devices agree on both CIDs, two cold builds per device give four equal output CIDs or a
named section that differs. Not to be done: dropping a variable from the record without
evidence scoped to this recipe, or reading equal keys as proof of a deterministic build.

Measured 2026-11-18 with `epr artifact inputs diff <key-a> <key-b>` (branch
`claude/nt-inputs-diff`, not landed; receipt
`genesis/local-dev/native-throughput/inputs-diff-shem-ethosengine-6616ce547.txt`). Source
paths, cargo config and the gcc `--version` digest are equal on both devices. Eleven input lines
differ and each can change the bytes: the gcc predefined-macro digest at the same `/usr/bin/gcc`,
and ten link inputs with different content at identical paths (`crt1.o`, `Scrt1.o`, `libgcc.a`,
`libc_nonshared.a`, `ld-linux-x86-64.so.2`, `libc.so.6`, `libgcc_s`, `libm.so.6`, `libmvec.so.1`).
The two workspaces carry different glibc and gcc runtime builds, so the keys rightly differ.
Five method fields differ only in `PATH` (extra and reordered entries on shem); whether those
reach the build is unresolved. A key hit across devices needs the same sysroot on both, which is
what the recorded `sysroot-pack` artifact is for; it does not need a change to the key.
