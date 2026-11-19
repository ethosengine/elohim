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

## Row 23 — a peer note waits for the receiver to look

2026-11-18: shem sent ethosengine a note at 05:09Z; ethosengine read it at 10:47Z. Its session
was alive and waiting for a differently worded note. Two followers on shem expired in between
and the sprint's live-red test waited five and a half hours. A berth note enters a session only
when that session next reads its inbox; nothing wakes an idle agent. Owed: an addressed note
wakes its session the way a gate red does (the `gate-live-inject` hook already injects reds on
the next tool call; an idle session needs the Monitor form), and a note that has gone unread
past a bound is visible to its sender (`berth say` prints nothing either way).

## Row 24 — brit gaps met while orchestrating (installed 0.2.0 on shem, pin 0.2.6)

Observed 2026-11-17/18, each on brit 0.2.0, so each needs re-checking on 0.2.6 before it is
called a gap: `brit commit` exits with a backtrace when a Git rebase or cherry-pick has left
`AUTO_MERGE`; no rebase onto a moved upstream when a gitlink moved; no cherry-pick. Built in the
brit submodule on branch `nt-gaps` (`f35d1b753`, not pushed, pin not moved): `brit log <ref>`
and ranges, `brit diff tree --name-only`. `epr bin follow --tool brit` refuses from a head that
is not published on `dev`, so a device whose sprint branch carries local commits cannot bring
its brit to the pin without a second checkout. Getting `nt-gaps` upstream and released is the
steward's lane (brit releases are incremental).

## Row 25 — a delegated agent's fresh worktree is refused by the gate preflight

2026-11-18: three builders in harness-made worktrees (`.claude/worktrees/agent-<id>`) each met
`gate-preflight` REFUSED: no submodules, no `node_modules`, no `.husky/_`. The cure is known
(`git submodule update --init elohim/brit elohim/rakia sophia`, `pnpm install`) and costs
minutes per agent. `just dev prep` does it; nothing runs it when the harness makes the
worktree. Owed: the worktree-create hook runs `just dev prep`, or the preflight's refusal names
that one command.

## Row 26 — nothing on a device knows which system libraries it builds against

Reviewed 2026-11-18 at the steward's request (journal, "The sysroot review"). The two devices
run different images on purpose, and every image tag floats. Open, in order of consequence:

1. A gate verdict's key holds nothing of gcc, glibc or the image, so a green stands in across
   two systems and across an image update, and the reused line does not say so. Proposal:
   record the sysroot identity on the verdict as evidence, outside the key. Steward decision.
2. An image update changes the link inputs silently. Built, awaiting review
   (`claude/nt-sysroot-visible`): `epr device status` prints the identity and a `degraded
   sysroot: changed since …` line.
3. The workspace images are not pinned by digest and their build records no package list
   (`che-devworkspaces`: `buildDevspaceImage.groovy`, the Dockerfiles' `FROM … :latest`).
   Steward's repository; a change is not prepared.
4. No workspace build can run on the fleet: floor glibc 2.39 against bookworm's 2.36. The
   cure is a declared target floor that a build links against (`--sysroot` and `-B`, or static
   musl); `sysroot pack|put` exists and nothing consumes it. Next sprint's subject.
5. The glibc floor counted libc's symbol versions only. Built with item 2.

## Row 18, addition (2026-11-18)

Every worktree on a `claude/<name>` branch resolves to the one family `claude`. Two gates in
two such worktrees at once removed each other's build tree (journal). The per-worktree record
keeps a family across a rename; it does not give a new worktree its own. Owed: a delegated
worktree gets a family of its own at creation, or the gate refuses a slot another live gate
owns and does not clean it.

## Row 27 — known limits of a sibling's live reds (review loop closed 2026-11-18)

Left after three independent reviews (receipts `codex-e4-review`, `codex-e5-review`,
`codex-e6-review` under `genesis/local-dev/native-throughput/`), recorded and not scheduled:
one recognised diagnostic suppresses the stand-in for a separate opaque failed step; delivery
addresses a session or an eight-character prefix, and an unknown local device id admits a
note; a killed delivery worker leaves no receipt and `unsent` counts only queued reds; a
subscriber to finals only can get silence when discovery fails; a push to two destinations
announces one ref; `GATE_TAP=off` observes nothing; overlapping runs in one session can emit
an early `unknown`; the hook drops a late note below an older overflow floor. A directed gate
event that the gate hook rejects for a device mismatch is shown by the berth-note hook as an
ordinary untrusted note.

## Row 28 — the read audit misses real reads and says nothing

Found 2026-11-18 while reviewing a narrowing of `gate-input-audit.mjs` (journal). Reads the
gates really perform and the audit does not find, with zero reported unresolved:
`elohim/eprfs/epr-cli/tests/flow_memory_recall_index.rs:208` (the embedding model manifest
under `.epr-meta`), the cite verifier's dependence on every cited path
(`epr-cli/src/flow/cites.rs:1277`), `genesis/orchestrator/gate-live-schema.test.mjs:20` (the
event schema), `doorway/doorway-service/tests/schema_contract.rs:31` (a path built in two
steps). Each is in its gate's key only through a dependency's test file that reads the same
place and does not run in that gate. Consequences: a verdict's soundness rests on coincidence;
removing the coincidence (which would save about 40 minutes on a rebase like shem's) is
unsound until the real reads are keyed. In flight: lane E9 declares these reads. Open design,
for the steward: the gate's own envelope observes what the recipe opens and which variables
it reads, and the verdict is keyed on the observation; the scan becomes a cross-check. Rows
20 and 21 (enrolment) wait on the same answer.

Delta 2026-11-18 (lane E9, widening only, landed with this note): the four reads are keyed at their
home. eprfs: `common::live_model_manifest()` declares the model manifest where its path is built (the
audit follows neither a cross-module `common::repo_root()` nor a helper's argument; the test file
produced no audit row at all). eprfs-live-corpus: both live cite legs share `legacy_read_paths()`
(cited directories count) and assert their targets are declared; 26 research cite targets are now
declared `gate-reads(eprfs-live-corpus)`, so a new cite target is declared or the leg is red.
doorway: the audit resolves a zero-argument helper of exactly `let NAME = <manifest chain>;` and a
tail on NAME. orchestrator: the audit reads Rust only, and the orchestrator suite reads 171 paths
outside its step sources (whole-repository registry and read-set walks, `.husky`, `scripts/ci`,
`genesis/scripts/ci`, `app/elohim-app/scripts/__tests__`, `genesis/a2o`, the event schema, and some of
its own test files), so `UNAUDITED_RECIPE_PROJECTS` keys it on the whole tree. Measured cost: a landing
whose tree differs from the attested one runs the gate, 188 s (run 20261118T174617Z), where such
landings reused it at 50 s and 51 s of push wall (6a8a19767, 5449615ce). Follow-up that buys it back:
the suite's outside reads declared in a list the suite itself enforces, hashed into its key.

Covered only by another crate's test read and NOT verified to be read by the gate's own tests
(survey: repo paths named by string literals in compiled files): for elohim-storage,
`.claude/epr-meta/recipes.yaml` (`bridges/jenkins/src/observe.rs:26`), `.epr-meta/collective.json`
(`bridges/cargo/src/translate.rs:113`), `genesis/agentic/agent-runtimes.json`
(`elohim/eprfs/device-runtime/src/bin_store/install.rs:148`) and `genesis/orchestrator/manifests/runtime/`
(`bridges/k8s/src/lib.rs:364`); for doorway and steward-node, `elohim/sdk/storage-client-ts/src/generated`
(`crates/elohim-views/src/acquisition.rs:13`). Doorway, elohim-storage and steward-node each hold an
include reference the audit cannot read, so the narrowing held on `self/nt-audit` would skip nothing
for them today.

## Row 29 — admission lets a v3 device's C code onto a v2 device

Found 2026-11-18 20:28Z (journal). Shem (x86-64-v2) drew `epr` from ethosengine at
`78cd0c0e5`; admission passed; the peer daemon started from that binary dies with SIGILL in
bundled SQLite. Admission (`epr-cli/src/artifact/extract_declared.rs`, since `7f48672f8`)
takes the build's level from the Rust target and flags, baseline when none. The C driver has
its own default: `gcc -Q --help=target` prints `-march=x86-64-v2` on shem and, expected, v3 on
RHEL 10. Owed, in order: (1) the declared level is the maximum of the Rust level and the C
driver's, read from the driver's predefined macros the key already probes, so a v2 host
refuses before fetch; (2) the gate builds C at a declared baseline so one device's release is
another's; (3) a check that runs a drawn binary's daemon path before it becomes current, since
`--version` succeeding proved nothing here. Related: row 26 item 4 (the deploy floor) is the
same question for glibc. Until (1) lands, shem follows only its own `epr` builds.

Delta 2026-11-19 (row 29): item 2 is answered, and the answer changes item 1. The gate builds C
at the Rust target's baseline, `-march=x86-64` on `x86_64-unknown-linux-gnu`, set by a per-triple
record in `pool-policy.json` for the `elohim/eprfs` workspace, with cc >= 1.11.1 in its lock
(journal, 2026-11-19). The level is then a constant the build sets and three checks confirm
(compiler probe before the build; the recorded flag and ring's compiler-written flags at put;
an x25519 and SQLite exercise on the receiving device before ingest and before current), so
item 1's predicted level is dropped: `claude/nt-isa` does not land. A sibling release without
the `env:isa-c` marker is refused before fetch as unmeasured. Item 3 is that exercise. The
correction to this row: the trap was `vpbroadcastd` in ring's C x25519, not SQLite. Open, each
its own row when taken: the bin-store judge (held releases, pins, bootstrap) waits for a
deliberate Edge roll because its hook is a storage build input; whether ethosengine withdraws
its marker-less releases; removing the crashed release from shem's store (no evict verb; `gc`
takes it on or after 2026-12-02); labelled non-baseline variants; the first aarch64 record.

Delta 2026-11-19 (row 28): an audit of the reuse-enrolled Rust gates confirmed 53 reads outside
a gate's key, 34 claims refuted (results: `genesis/local-dev/native-throughput/wf-row28-audit.result.json`
on shem). Worst first: eprfs (`genesis/agentic/session_envelope_test.py` and `berth`, run by its
tests, read `.claude/hooks/*`, `.claude/scripts/_lib/*`, `.epr-meta/elohim/lenses/`, the recall
question bank and root `.gitignore`); `genesis/agentic/bin/pool-lib.sh`, sourced by
`run-local-gate.sh:28` for every cargo gate; elohim-storage's read of
`elohim/rakia/schemas/v1/release-manifest.schema.json` (`release_adoption/verify.rs:1784`),
which a `// gate-reads` line cannot name and whose submodule checkout drift the dirty check
does not see; scripts the storage recipe spawns (`scripts/ci/check-image-include-targets.py`,
`genesis/agentic/bin/cargo-jobs-flag`); tool configuration above a crate (`rustfmt.toml`,
`clippy.toml`, the main checkout's `.cargo/config.toml` from a worktree); small crates whose
`cargo fmt` runs over a whole workspace; and ambient inputs no scan sees (`EPR_BIN` for the
orchestrator suite, `RUSTC`, `CARGO_ENCODED_RUSTFLAGS`). Confirmed sound: path-dependency
crates outside a gate's directory are hashed. The cure is not 35 declarations: only an observed
trace of a real gate run closes the class, which is the open design above and what E11 builds
for one suite.

## Row 30 — a note silently fails to cross when the device is loaded

Found 2026-11-19. `berth say` writes its note through `_observation.emit`, whose limit is a
fixed 5 s (`.claude/hooks/_observation.py:56`). On a loaded shem the write took about 10 s, so
six sends in a row printed "said here, but the note record did not reach this device's channel
store" and no other device read them. The sender is told; nothing retries. Owed: the limit
scales or the write is retried, and a note that did not cross is queued, not dropped.

Delta 2026-11-19 (row 17): every editor terminal on shem is spawned with `EPR_BIN` naming the
2 November build (`bafyreibrqqhx3…`, epr 0.3.2); six shells opened between 2026-11-10 and today
carry it. The editor's pty host does not, and no shell startup file, repository setting,
workspace file or user settings file sets it, so it is injected at terminal spawn; the setter
is not found. Effects measured by session d9c6be9b: hooks run the old binary (`epr govern` 1.1
to 1.6 s against about 0.45 s; the cite-seal hook reporting cites the current `epr` verifies),
and commit attribution reads `UNMEASURED`. Station `Terminal-env` in the native-dev-tooling
plan, section 8.

## Row 31 — gate code reads a git refusal as a value

Found 2026-11-19 by brit sprint 3's declared-trust audit (session 0e0924d5). When git refuses
(a checkout it does not trust after a restart, a missing object), these return a value instead
of failing: `gate-attest.mjs:75` `worktreeDirty` returns false, so dirty reads as clean;
`gate-evidence.mjs:157` `gitRead` returns null for every failure (`manifestBlob`,
`projectTree`, `gitlinksAt`); `:269` `signerIdentity` falls back to the seed agent id;
`gate-runner.mjs:122` turns any failure into "attested commit unreadable";
`.claude/hooks/pre-flight-check.py:77` reports clean; `.claude/scripts/_lib/epr_meta_git.py:35`
fails open to "no changed files". With `RUST_BACKTRACE` set, brit's last stderr line is a
backtrace frame, so callers that report the last line hide the cause (`gate-evidence.mjs:91`,
`:401`). `gate-runner.mjs:186-188` deletes the inherited `GIT_CONFIG_*` block on purpose (the
`git -c core.hooksPath` of every landing leaked into fixture repositories), so trust cannot
ride that block. Proposed carrier, for the owner to take or decline: after the deletion the
runner sets, for its children only, `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=safe.directory`,
`GIT_CONFIG_VALUE_0=<declared root>`, which git reads today and brit will after its trust lane.
Owed: each site refuses by name instead of returning a value. Owner: ethosengine a222db7c,
after L2a.

Delta 2026-11-19 (row 29): waiting on a deliberate Edge roll, each a storage build input:
the bin store's own judge (held releases, pins, bootstrap), and cc >= 1.11.1 in
`elohim/Cargo.lock` (cc 1.2.52 today, with ring; gate projects elohim-epr, elohim-compute,
elohim-live-session and domain-types; nothing from that workspace is offered across devices).
cc applies `-march=x86-64` itself only while rustc is given no target-cpu, or `x86-64`: the eprfs
builds qualify because the gate and the bin store both build with `RUSTFLAGS=""`
(`device-runtime/src/bin_store/source.rs:193`, `:430`), and L1's pin test holds that condition.

Delta 2026-11-19 (row 18): `epr-cli/tests/bin_build_view.rs` reads `$CARGO_TARGET_DIR/debug/epr.d`,
which the gate's test step never refreshes; with one pool family shared by every `claude/*`
worktree, the eprfs gate went red twice on ethosengine because another worktree had built the
bin last. Cure in flight with L2b: the gate builds the bins before the tests, and the test names
the cause. The shared family itself is still this row.

## Row 32 — a pin-only submodule move selects nothing for a step that reads inside it

Found 2026-11-19 by brit sprint 3 (session 0e0924d5). `rakia-core` 0.1.1
`constellation::plan_from_changes` tests a changed path against a step's globs; a pin move
changes the path `elohim/rakia`, which does not match `elohim/rakia/schemas/**`, so the step is
not selected for that change. The matching fingerprint hole (a glob that crosses a gitlink
hashed nothing) is fixed in brit's `sprint3/build-inputs`: the gitlink's path and pinned commit
become a step input, so `cargo-build-storage` re-keys once when that brit is pinned, and genesis
`constants-sync`, whose globs reach all five submodules, re-keys on every pin move (its globs
may be wider than its reads). Also in rakia: `brit build affected|plan|baseline` open the
repository through `rakia-brit` on gix 0.81, which keeps a 32-slot pack-index ceiling at reduced
trust. Both are in `elohim/rakia`, in nobody's write set this sprint.

Delta 2026-11-19 (row 31): the line numbers above came from an audit on a checkout 142 commits
behind `dev`. On `dev` `2080441f5` the sites are: `gate-attest.mjs:74-76` (`worktreeDirty`: a
non-zero status reads as not dirty); `gate-evidence.mjs:157-164` (`gitRead` returns null on any
failure), `:269` and `:288` (`signerIdentity` falls back to the seed agent id);
`gate-runner.mjs:121-125` ("attested commit unreadable") and `:182-187` (the deliberate
`GIT_CONFIG_*` deletion); `.claude/hooks/pre-flight-check.py:89`;
`.claude/scripts/_lib/epr_meta_git.py:32-37` (`git()` returns None on an exception) and `:49`
(`changed_files` returns an empty list); `elohim/eprfs/epr-cli/src/artifact.rs:1109` (`brit()`
spawns `brit-build-ref` without the trust environment, so `epr artifact put` fails after a
restart; one line, travels with L2a).

Delta 2026-11-19 (row 32): brit's `sprint3/build-inputs` also keys a tracked symlink matched by a
step glob, by the link's own object id (retargeting one used to leave the fingerprint unchanged).
When a brit carrying both changes is pinned, sixteen monorepo steps re-key once: storage
`cargo-build-storage` (gitlink `elohim/rakia` and symlink `elohim/sdk/schemas/current`), app
`build-angular`, genesis `schema-codegen`, `schema-validate`, `validate-seeds` and
`constants-sync`, the DNA steps `schema-dna`, `build-dna-wasm`, `build-happ`,
`dna-extern-lints`, `manifest-hygiene` and `sweettest-check` (globs can match beneath the five
tracked `target` symlinks), `build-conductor-image`, `build-edge-image`, and the orchestrator's
`lint-jenkinsfiles`. These are local gate keys. The CI image closure key
(`scripts/ci/closure-input-digest.sh`) computes from git object ids itself and does not call
brit, so image keys do not move. Budget the pin move as one landing with cold storage, DNA and
app gates on each device. Still open in `rakia-core`: selection misses a pin-only move and a
symlink-only retarget, and `constellation.rs` `build_glob_set` keeps globset's per-platform
default (a backslash is a separator on Windows), which brit fixed on its own side.

Delta 2026-11-19 (row 29): the list of changes waiting on a deliberate Edge roll is now four.
(1) The bin store's own judge. (2) cc >= 1.11.1 in `elohim/Cargo.lock`. (3) A custody check on
`<config_dir>/layout.json`: `device-runtime/src/layout.rs` `read_saved` reads it with none, and
the file sets the bin-store, state, cargo-pool and project-root paths, so a directory a stranger
can write could redirect them (found by brit sprint 3 while holding the trusted-checkouts reader
to the key directory's custody rule). (4) Moving the gate-live event schema out of
`elohim/sdk/schemas/v1`: it is development tooling, and its place in the protocol schema
directory makes every edit a storage build input (row 33). Until that move, the `notice` kind
lives in an orchestrator-local addendum beside its only reader.

Delta 2026-11-19 (rows 5 and 24): `brit rebase` on 0.2.6 is not safe on a repository with full
history. Measured by brit sprint 3 on a full clone of brit (17,972 commits; the rebase's closure
is 218,090 objects, 2.19 GB): replaying one commit published after about 26 minutes and about
110 GB of temporary writes (git: 0.21 s), then cleanup ran 4.5 hours until stopped. Read in
`gitoxide-core/src/repository/replay/`: every pass hashes every object reachable from both tips;
cleanup lists the whole retained copy again for each object; and cleanup compares against a
1 GiB scratch limit, so above that size it cannot finish. Afterwards `rebase --abort` refuses,
`--continue` cannot end and `brit switch` refuses "pending replay", so every brit mutation in
that repository is blocked and no brit command clears it. Ethosengine's monorepo clone is
shallow and brit refuses a shallow rebase first. Shem's is not shallow: its own
`brit rebase origin/dev` (6 minutes, 29 GB of temporary writes) ended in a clean refusal only
because the tree holds gitlinks. Rule on both devices until a release carries the fix: rebase
with git under a declared fallback. Brit sprint 3 opened a recovery lane the same day (abort of
a `preparing` journal, a refusal before any spooling when the retained copy cannot fit, a way
back for a repository already stuck). Its redesign would change the retained-manifest format so
that an older brit refuses a copy a newer one wrote; that decision goes to the steward.

Delta 2026-11-19 (row 24): three more from the installed 0.2.6. `brit fetch -r origin dev` with
no destination reports up to date and leaves `refs/remotes/origin/dev` where it was; an explicit
destination refspec moves it. Each fetch adds a pack index and nothing consolidates them: shem
holds 28 and ethosengine 33, against a ceiling of 32 at reduced trust (`git repack -a -d` cures
it; the steward's store, not run). And a correction: `rev-parse`, `ls-remote` and a directory
option were reported from shem as missing, but 0.2.6 has them as `revision resolve`,
`remote refs` and `-r`. The census behind that report still stands (3,767 git invocations by
shem's agents since 2026-11-06, about 1,700 of the form `git -C <dir>`), and brit sprint 3 took
shem's sixteen patches for the git spellings into its integration branch on 2026-11-19
(`run-ci/sprint3-integrate` `9945331577`): `rev-parse`, `for-each-ref`, `ls-remote`,
`check-ref-format`, `merge-base --is-ancestor`, `rev-list --count`, a global `-C`, and a closed
reader ending a read-only command quietly. Two follow-ups stay with shem's lane: forward slashes
in the path answers on Windows, and admitting the remaining read-only commands to the closed
reader rule once each is checked.

## Row 33 — nothing at push time says a landing will roll the fleet

Found 2026-11-19. A stack about to land on `dev` carried a one-line edit to
`elohim/sdk/schemas/v1/gate-live-event.schema.json`. `elohim/sdk/**` is an input of the Edge
pipeline's `cargo-build-storage` step, so the push would have rebuilt storage and rolled the
seven alpha peers, with `build-angular`, the DNA schema step and three genesis schema steps
beside it. Ethosengine a222db7c caught it by running the dispatch walker over the stack's 43
paths by hand; no gate, preflight line or hook would have said so. The walker answers in 0.6 s.
The storage step's inputs are wide (46 globs, among them `elohim/sdk/**`, `crates/**`,
`bridges/**`, `vendor/**`, `elohim/epr/**`, `elohim/eprfs/Cargo.toml`, `eprfs-core`,
`eprfs-serve` and `device-runtime`), and the rule that nothing rolls the fleet except on purpose
is kept by memory alone.

In flight on shem (branch `claude/nt-fleet-roll-guard`): a preflight check, `fleet-roll`, under
the habit `careless-reds-cost-seconds`. A push to a deploying branch whose paths select an Edge
step other than `dataplane-validation`, or whose commits force one, is refused in the first
seconds unless a pushed commit declares the roll with a tag; the mode (`refuse`, `warn`, `off`)
is a key in `pool-policy.json`. Known limit: it reads the push's own paths and tags, so an Edge
step left stale by an earlier landing is not seen. Refusing by default is a decision taken for
the steward to overrule.

## Row 34 — an agent held on an unanswered approval still reads as running

Found 2026-11-19 on both devices. A delegated builder on shem showed `running` for four hours
with no commits; its transcript ended on a tool call with no result, and no such process
existed. Brit sprint 3 then found the same on ethosengine three times: two reviewers held two
and three hours on a compound shell command that had not started, and its own session held two
hours. The call was waiting for an approval while nobody was present. No completion notice
arrives, the agent list says `running`, and the lane is lost silently; shem's was found when a
peer asked why a hand-over ref had not moved. What works today: stop the agent and send it a
message, which resumes it with its context (tell it not to issue the held command again);
briefs that ask for short single-purpose commands under `timeout`; and no `pkill -f` or
`pgrep -f` waits (nine leaked `until ! pgrep -f` loops on ethosengine matched their own command
line; one `pkill -f "sleep 30"` on shem killed another agent's waiter). Owed: the wait made
visible where a coordinator looks (a berth line "waiting on an approval since …" would do), and
a time after which a held call in an unattended agent is refused rather than kept.
