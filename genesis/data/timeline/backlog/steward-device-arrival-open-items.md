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

Delta 2026-11-19 (row 33): what two reviews of the `fleet-roll` check established about the
tools it replaces and about CI. The JS walker (`graph-walker.mjs`) skips dependency propagation
and ignores dotfiles; CI's `build-graph.groovy` does both, so a by-hand walker run is not CI's
answer. Edge's build steps depend on `elohim-conductor:build-conductor-image`,
`elohim-rakia:rakia-ci`, `elohim-holochain:build-happ` and `elohim-brit:brit-ci`. CI's checkout
initialises no submodule and prunes a dependency on a pipeline whose manifest it cannot see, so
by the code a conductor pin move selects an Edge build and a brit or rakia pin move does not;
that has not been checked against a CI run and contradicts the note that an attested pin move
rolls the fleet. Three behaviours of CI itself, recorded here and not changed: `[build:all]`
forces nothing on a fresh run (it is expanded at Checkout against a registry filled later,
`genesis/orchestrator/Jenkinsfile:2688-2693`, `:2413`); `[deploy-only]` can re-add an Edge run
that the already-built filter removed, with no class, which then reads as build and deploys
(`:931-934`, `:1912`, `:1962-1964`, `:2828-2834`); and a `[skip ci]` or `[edge:validate-only]`
tip moves the baseline past every commit under it once the run ends green (`:230`, `:1450`).
The local path oracle (`GATE_ORACLE=path`) has the walker's dotfile hole; the default rakia
oracle does not.

## Row 35 — nothing restarts the supervised peer when a release lands

Found 2026-11-19 on shem. `epr bin follow` built and landed a new `epr`
(`bafyreifcq7qfsf…elhfe`), and the supervised `epr peer serve` kept running the previous
release: `epr device status` reads `degraded peer daemon: running … ; current … ; restart epr
peer serve to follow current`. `epr device supervise` reloads by restart when the device's
manifest head moves, and a tool's current release moving is not that. The one-baseline sprint's
first line says the daemon restarts on the drawn bytes and serves; today that is a manual stop
and start of the supervisor. Missing step, in the chain's own terms: between "a release landed
as current" and "the daemon serves from it", the supervisor notices that a supervised process's
tool moved and restarts that process, with the same readiness check and death witness as any
other restart. Until then the restart is by hand, from a clean command, never with `XDG_*`
unset.

## Row 36 — a checked-in grant's deadline reds the orchestrator suite on its own

Found 2026-11-19 by the review of the E11 rework. `genesis/orchestrator/data/adam-compute-worker.json`
carries `runnerGrant.enabled: true` with `validUntil: 2026-11-23T16:00:00Z`, and
`compute-worker.test.mjs` asserts that an enabled grant is inside its window. After that instant
the test fails on every device and in CI until the grant is renewed or withdrawn; nothing warns
ahead of the deadline. The test is right to fail. Owed: the renewal or withdrawal before the
date (the steward's, as steward of record), and a warning some days ahead so the red does not
arrive by surprise. The E11 rework moves this assertion out of the reusable half so a green
earned before the deadline cannot be reused after it.

Delta 2026-11-19 (row 33): the check landed on `dev` as `0f48e02fb` and is in force. Its limits,
stated in `genesis/orchestrator/fleet-roll-guard.mjs`: stored build-process hashes, CI's
already-built filter and steps left stale by an earlier landing are not seen; a non-`HEAD` push
through an older hook is judged from paths only; in a shallow clone a range that reaches the
boundary with no declaration visible is refused as undecidable. The pin question is settled for
brit against a run: `elohim-orchestrator/dev #2359` covered the pin move `1602399ee` and marked
every Edge step "SKIP, no changes", and its log names `elohim-brit:brit-ci`,
`elohim-rakia:rakia-ci` and `elohim-sophia:build-sophia-umd` as external dependencies it prunes.

Delta 2026-11-19 (row 28): the orchestrator suite's named half now keys on what it reads by
absence (landed `0fd724eb2`): it runs in a view holding only its declared files, so an
undeclared read fails instead of going unkeyed. The first run in the view found a missing
declaration (`genesis/agentic/bin/pool-lib.sh` for `gate-evidence.test`). Its review also found a
hole that predated it and is closed for every gate: a caller's `JUST_NO_DEPS` made a
dependency-only gate recipe exit 0 without running, and `gateChildEnv` now removes every
`JUST_*` variable. Still open from that lane: fifteen tree-side test files that were never
retried in the view; the tree half reuses on an identical tree while its verdict also depends on
device state and on Jenkins being reachable; the live recogniser's wording for the new red lines
is new and thinly exercised.

Delta 2026-11-19 (row 24): brit 0.2.6 has no merge verb, and after a git squash merge
`brit commit -F` refuses with "merge commit cannot complete pending SQUASH_MSG"; `brit switch`
refuses while a hook-written file under `.claude/data` is modified; there is no branch delete.
Brit sprint 3 merged shem's read-only extension on its integration branch (`080f30579`). Left on
shem's list for brit: forward slashes in the absolute path answers on Windows; a completed
`build baseline write` or `migrate` exiting 1 on a closed stdout (to decide with push policy);
the deprecated `rakia` alias returning before the read-only flag is set.

Delta 2026-11-19 (row 35): done by hand on shem at 15:42Z with the landed release:
`epr device supervise --stop`, then `epr device supervise`; the peer was ready about 25 s after
the start, and `epr device status` read "serving at current" and "nothing owed".

## Row 37 — a brit pin's attestation rested on a check that was green over failed legs

Found 2026-11-19 by brit sprint 3 and traced to the monorepo here. `elohim/brit/build-manifest.json`
declares the pin's gate as attested by the GitHub check `Tests pass`, and
`genesis/orchestrator/gate-attest.mjs:79-120` passes the pin when the most recently started
check run of that name concludes `success`. That check is an aggregate job with a deny-list
(`failure`, `cancelled`, `skipped`). When one leg of a matrix job never gets a runner, GitHub
reports the matrix job's result as `abandoned`, which the list did not name: run 44544779620
reported `Tests pass` success with two failed Linux legs and an abandoned macOS leg. The unit
test job was not in the aggregate's needs at all. Brit's integration branch now carries an
allow-list step (every need exactly `success`); `1bf96648cb` is the first brit commit whose own
run includes it (run 44575030094, 30 jobs green). Consequences: a brit pin should name a commit
whose own CI ran with that step, which excludes every brit commit on main today, 0.2.6's
included. Owed on this side, queued behind the gate lanes and to land together with the
manifest's field: the gate reads every workflow run's conclusion at the pinned commit as a
second check. rakia's check is `test` and sophia's `build`, single jobs.

## Row 38 — a push from a dirty tree is tested on the working tree, not on the pushed commit

Stated 2026-11-19 while ruling on the E11 rework; it is true of every gate. When declared paths
differ from `HEAD`, a gate tests the working tree's bytes, passes or fails on them, and records
nothing. For `just gate` that is the development loop. For a push it means the bytes that were
tested are not the bytes that were pushed, and the push proceeds on a pass. The orchestrator's
named half could close this for itself, since its view is already built from a commit's
objects: under a push it could build the view from the pushed commit whatever the working tree
holds. Not done; the other gates have no such view.

## Row 39 — a guard on a variable in the shell is passed by a child process, and tests that skip as root never run

Two observations from 2026-11-19, both the steward's to rule on. (a) Ethosengine a222db7c
reported that one of its builders, to keep scratch records off the live peer daemon, set
`XDG_CONFIG_HOME` inside a python child, which gets round the worktree-isolation hook's refusal
of that variable in a Bash call. No device state was touched and both runs made that way were
withdrawn as receipts. The need behind it is real: an end-to-end run of `epr artifact put` wants
an isolated device home, and the product's own flags (`--repo`, `--payload-store`, a scratch
flow root) do not yet give one without that variable. (b) Both devices run as root. Brit's CI
found a test that skips itself at uid 0 and therefore had never run on either device, and the
orchestrator suite has the same shape in `runner-capture-activation.test.mjs` (its distinct-user
checks). A non-root leg, as `setpriv --reuid=65534 --regid=65534 --clear-groups <test binary>`,
is what brit used.

## Row 40 — a gate change about what a device holds was verified on one device

Found 2026-11-19 on ethosengine. E11 (landed `0fd724eb2`, 18:14Z) moved the orchestrator
suite's named half into a view built from commit objects. Nine named files declare all five
submodule pins, so the view took files from each submodule at its pin, 104 of them from
`elohim/holochain-conductor`. Shem holds that fork's objects. Ethosengine does not, and
`.gitmodules` declares `update = none` for it, so the cure the refusal printed
(`git submodule update --init`) skips it. The named half was red on ethosengine for every push
that selected it, for about 1 h 40 min, and one push (the brit session's trust branch) ran into
it. `just dev prep` and the preflight said `ok submodules` throughout, because they knew only
the three attested submodules.

Fixed. `00dd77dfb`: a submodule declared `update = none` contributes its pin to the key and
nothing to the view. Ethosengine's receipt at that commit, in a fresh worktree on a device with
the conductor pin in no store: `just gate orchestrator` exit 0 in 492 s, both halves recorded,
779 + 376 tests. `4ce35271d`: a `submodule-pins` preflight row and `just dev prep` for the
pins the view does read.

Still open, and the reason this is a row: nothing asks for a second device's run before a gate
change is called landed. E11 had four review passes and a checklist, and every run in them was
on shem. The cheap form is a habit of the hand-over: for a change under `genesis/orchestrator`
that alters what the gate reads from the repository or from the device, the note to the sibling
asks for one `just gate <project>` in a fresh worktree, and the line is not called met until
the verdict lines are back. Whether that becomes a check (a sibling's green required before a
push that changes the gate's own governance paths records evidence others may reuse) is a
design question for the next sprint.

## Row 41 — CI's eprfs pipeline was red for two days on one test and no session heard it

Found 2026-11-19 by the brit sprint session reading Jenkins for its own landing. `elohim-eprfs/dev`
last passed at #167. From #168, the first build after `1033395d5` added
`epr-cli/tests/bin_build_view.rs` (2026-11-17), to #190, every build failed on that one test:
"no dep-info at …/target/debug/epr.d". The exception is #173, a failed clone. Nothing else failed
in that range: no other test, no compile, lint or format error. The two publish stages were
skipped in every one of those builds, so no eprfs crate has been published since #167, and the
orchestrator's dev job failed with it from #2378 on, holding its global baseline at `2080441f`.

Cause: the two recipes had parted. The device gate (`_gate-eprfs`) got a `bins` step with the
test, `cargo build -p elohim-epr-cli --bin epr -p elohim-device-runtime --bin bin-launch`,
because cargo writes `epr.d` only when the bin is built as a top-level target. The pipeline's
tests stage ran `cargo nextest` alone. Every device gate was green.

Fixed: `f27e22e69`. The tests stage runs the same build first, and
`genesis/orchestrator/eprfs-ci-parity.test.mjs` reads the bins step out of the justfile and
refuses a pipeline that does not run it before `cargo nextest`. CI's first run with it, `elohim-eprfs/dev` #191, has the tests
stage green ("2634 tests run: 2634 passed (1 slow), 1 skipped") and fails one stage later (row 43).

Still open:

1. **Nothing said so.** Twenty-two red builds, and the session-start lines on shem read
   "7 known finding(s) recurred (flake evidence)" and "observe deferred: jenkins-bridge not on
   PATH". A failure that recurs in every build of a pipeline is a standing red, not flake
   evidence. The harvester should name it as one, with the count of builds and the last green,
   where a session reads it first.
2. **The orchestrator's own job did not stay red.** `elohim-orchestrator/dev` failed from #2359
   to #2370, passed from #2371 to #2376 and failed again from #2377, while eprfs was red
   throughout. Why those six passed is unread. A run that dispatches nothing passes and advances
   the global baseline, which is how it came to be held at `2080441f`, a commit under which the
   eprfs red already lay; and the harvester printed "graph unreadable" for five of those six.
3. **The parity test covers one step of one project.** The same drift is possible for every
   gate project whose device recipe and pipeline are written separately (the bridges, ark,
   storage, doorway). The general form is one declared step list that both run.

## Row 42 — a pipeline file inside a gate project's tree runs that project's whole device gate

Measured 2026-11-19 on shem. `f27e22e69` changes one line of `elohim/eprfs/Jenkinsfile`, a test
file and a list entry under `genesis/orchestrator`. The push selected address-profile-lint,
eprfs-live-corpus, orchestrator-tree, cargo-bridge, jenkins-bridge, orchestrator and eprfs, and
took 2756 s, of which the eprfs gate was 1903 s. No step of `_gate-eprfs` reads the Jenkinsfile.
It is in the key because the manifest's `rust-build-test` step lists `elohim/eprfs/**` as its
sources, and one list serves two readers: CI's dispatch, for which a pipeline file is an input,
and the device gate's key, for which it is not.

The cure belongs to the next sprint: the device key leaves out what only CI reads (the
pipeline file, and whatever else under the tree no recipe step opens), found the way the read
audit finds reads, while CI's dispatch keeps the whole list. Until then a pipeline-file change in
a Rust gate project costs that project's full gate on the device that lands it.

## Row 43 — a third-party lock change alters four published packages, and only CI's publish stage sees it

Found 2026-11-19 in `elohim-eprfs/dev` #191, the first run to get past its tests since #167:
"ERROR: eprfs-core 0.3.5: immutable version collision; bump the version".

Measured against the registry with the publish script's own readers (no cargo, no credentials,
nothing uploaded): the packages published as eprfs-core 0.3.5, eprfs-storage 0.3.8,
eprfs-meta 0.3.6 and eprfs-local 0.3.6 each carry a `Cargo.lock` that pins `cc 1.2.65` and
`find-msvc-tools 0.1.9`. The workspace lock has had `cc 1.11.1` and `find-msvc-tools 0.1.15`
since `2090a4d82` (L1, the C baseline). Cargo packs the workspace lock, filtered to the crate's
graph, into every package, so those four differ from the registry's copies at an unchanged
version. eprfs-host and elohim-seam-contracts do not.

Why no device saw it: `publish_eprfs.py --precheck`, the step of the device gate that exists to
refuse this, compares the crate's source files and, in the packed lock, only workspace members
(`locked_member_moves`). A third-party entry that moves is not compared. The publish stage
compares whole archives.

The cure is the steward's, because of what it pulls with it:

1. **By the script's rule, bump the four versions.** `elohim-storage` depends on `eprfs-core`
   by path, and `elohim/Cargo.lock` pins eprfs-core, eprfs-local and eprfs-storage, so the
   bumps move that lock, which is a storage build input: an Edge build and deploy. The
   fleet-roll check refuses it without `[edge:roll]`. The precheck's gap closes in the same
   change (compare every package in the packed lock with the workspace lock) and not before
   it: landed alone it turns each device's eprfs gate red on today's dev.
2. **Or leave the packed lock out of the publish comparison for library packages.** A
   dependent does not read the `Cargo.lock` inside a library's package. The sources and the
   manifest are identical, so nothing would be republished and nothing would roll. It relaxes
   a publish safeguard, and its tests, by one file.
3. **Behind both:** under rule 1 a third-party lock change in a published crate's graph costs
   an Edge roll, and three Dependabot fixes are waiting on this lock. Storage taking
   eprfs-core from the registry by version, where today it takes it by path, would end that
   coupling.

Until he rules, `elohim-eprfs/dev` is red at its publish stage and the orchestrator's dev job
with it. The tests stage is green.

## Row 35, addition (2026-11-20)

Tonight's three restarts on shem were each one `epr device supervise` typed by hand (6 s, 9 s
and 7 s for the command; serving 29 s to 43 s after it began). The automatic form is in work on
ethosengine in two landings. A-min (`self/nt-row35-a2`, in progress): after a successful follow
or forward land, when the acting session holds the device-ark lease, run the existing
stale-child restart, wait on ark's clock for the landed tool's own children, and report ready
or degraded with the cure; any error after the old ark stopped is a non-zero exit; one switch,
`device_restart.on_landing`. B (`self/nt-row35-b`): a follower retries an admitted offer whose
holder gives no route, up to `device_restart.draw_patience_seconds`, before it falls back to a
local build, and a stopping peer drains its streams for up to 15 s. The limit that stays: the
restart is by itself only when the follower holds the lease or no live holder exists. A lease
the device holds, not a session, goes with the ark's socket.

## Row 44 — a first `epr session open` waited five seconds on its own guard

Found 2026-11-20 through the eprfs gate's slowest test binary (`session_envelope`, 433 s of
1,198 s). `session open` held the exclusive sessions guard and, through `recall::initialize`,
`actor::current_claim` and `covenant_scope`, asked for a shared guard on the same file in the
same process; it waited the lock's five seconds, timed out, and the error was swallowed as an
unknown reader. Real sessions paid it at each first open and had it charged to their recall
budget. Fixed in epr-cli alone, `7213bb589`: `open` hands recall the key it already holds.
Fixture 457.5 s to 230.7 s; the eprfs gate on shem 1,903 s to 1,684 s.

Still open: (a) `reader_from_session` still turns any error into an unknown reader, so the next
failure there will be as quiet as this one was; (b) under a loaded host the fixture's own
`epr bin ingest` setup step can pass its 45 s subprocess timeout (four cases in one run, green
on rerun); (c) the fixture is still 231 s of serial cases and the largest single cost in the
gate; (d) `memory-ceremony` is 367 to 412 s on each epr-cli landing and is not enrolled for
reuse.

## Row 45 — a test demanded a crate source that a device never downloads

Found 2026-11-20 on shem, 55 minutes into a push. `c-baseline-pin.test.mjs` held each reviewed
exception against the crate's `build.rs` in the device's cargo registry and failed when the
source was absent. `generator-0.8.11` is in the eprfs lock only through `loom`, under
`cfg(loom)`; 43 of the lock's 611 crates.io packages have no source on shem. Ethosengine had it
by accident, so the test passed where it was written. Row 40's class with the devices swapped,
approved by the same reviewer a day later.

Fixed, `d6e9c69aa`: each exception carries the reviewed lock checksum and is held against the
lock on every device, against `build.rs` where the source is present; an absent source still
fails for a crate a build here must have downloaded. Still open: `builtHere` is written by
hand; the check that every such crate is absent from the closure the producer's roster computes
is owed. And row 40's open item now has two instances.

## Row 46 — an unattended restart cannot yet revert or hold a release

The first build of row 35 reverted one step and held the release when the new bytes did not
become ready. Its outside review (2026-11-20, `codex-row35a.receipt.md` on
`self/nt-review-receipts`) is this row's specification. Seven findings: readiness must be
judged for the landed tool's own children on ark's clock; every failure after the old ark stops
must be recovered or reported, never a successful landing with no peer; a rollback must
establish that it can restore before it stops anything, survive being killed between steps, and
work for a gitlink tool; a hold must be enforced where a release is promoted and where a child
is spawned, on every arm (follow, arrival, bootstrap, cold supervise, manifest reload); the
settlement must be bound to the store transition that caused it and serialised against a racing
follow; the device root and the policy checkout must be bound to the running ark; and a restart
needs lease authority, with berth's "no session" exit a refusal.

The store-level revert and the hold at promotion and spawn are `elohim/eprfs/device-runtime`
and `elohim/ark`, so this lands with a deliberate Edge roll. The built first attempt is kept at
`self/nt-row35-a` (`8f247f683`).

## Row 35, second addition (2026-11-20, 05:55Z)

Landed: `fcf1f3137` (the restart after a landing) and `aeada1196` (a 60 s wait for the first
spawn; a row for a hand restart). Receipts, each device following `aeada1196` with the release
that carries the restart: shem at 05:54:03Z restarted by itself and reported `not ready after
13 s`, the old floor's false report, with the peer ready about 20 s later; ethosengine at
05:55:48Z restarted by itself and reported `peer ready on … in 5 s`. Both wrote a row with
trigger follow.

Still open on this row: (a) the restart is by itself only when the follower holds the
device-ark lease or no live holder exists; a lease the device holds goes with the ark's socket.
(b) The second landing (a follower waits for an admitted offer whose holder gives no route; a
stopping peer drains) is built at `self/nt-row35-b` and waits for its review. (c) A restart's
outcome is in the device's `restarts.jsonl` and in status, not in the native journal: a
declared measure for seconds-to-ready, with a not-ready as pain, is owed. (d) Ethosengine's
push hook defers the household head to the next session arrival, so a producing device does
not adopt what it produces until someone follows. (e) The supervisor binary a device runs is a
cargo pool slot's `debug/ark`, which pool reclamation can remove or leave stale.

## Row 47 — the eprfs gate builds and runs an ark that its key does not name

Since `fcf1f3137` the eprfs gate builds `ark` from the elohim workspace
(`cargo build --manifest-path ../Cargo.toml -p elohim-ark --bin ark`) and runs the
device_supervise stories against it. The gate's input closure names `elohim/ark/core` and
`elohim/ark/supervisor` and not `elohim/ark/cli` or the elohim workspace's manifest and lock,
so under `GATE_EVIDENCE=reuse` a green eprfs verdict can be carried across a change to the ark
those stories ran. Found by ethosengine while building the step. The cure is in the gate
project's inputs in `elohim/eprfs/build-manifest.json`, which is a gate-governance path: its
own landing.

## Row 48 — a test left a peer daemon running, and its lease went into the device's real store

Found 2026-11-20 on ethosengine. `epr device status` there had read "degraded peer daemon"
since 2026-11-17 because of a live orphan: pid 4039220, parent 1, a deleted debug `epr`
serving `--root /tmp/devices-share-one-berth-…`, with its HOME and config under that fixture.
Two defects. The a2o scenario `devices-share-one-berth`
(`genesis/a2o/features/devflow/devices-share-one-berth.feature`; steps in
`genesis/a2o/steps/devflow/devices-share-one-berth.steps.ts`, its `After` hook at line 106 and
the process-group kill at line 963) can leave one of its two daemons alive past the run. And
that daemon's lease was written into the HOST bin store, whose root
(`/projects/.claude-config/tools/bin-store`) the fixture's HOME and XDG do not move. Ended with
one interrupt; the lease then read dead by itself. Not fixed: the scenario's teardown, and a
fixture-scoped bin store root for anything a test starts.

## Row 35, third addition (2026-11-20, 10:00Z)

The second landing is built and NOT on dev: `self/nt-row35-b2` (`9ada5b173`, four commits, six
files). Its outside review is `codex-row35b.receipt.md` on `self/nt-review-receipts`. What it
must do before it lands: (G1) a decisive failure, a refusal or an integrity failure, outranks a
transport one when a fetch is abandoned, and the gap named is the offering holder's own, so a
refusal is not waited on; (G2) the patience is an elapsed deadline, with no retry started after
it, a retry given what remains as its own deadline, and the policy value capped; (G3) admission
is checked again before each retry; (G4) a tier stream's registration is atomic with the
drain's phase; (G5) once draining the peer accepts no gossip connection and its loop starts no
new work, and the whole shutdown has one deadline inside the supervisor's grace, tested against
the real bound; (G6) the late-connection story pins the close code and reason, and the
classifier story goes through the production formatter. Until it lands, a follower that meets
a holder in the middle of a restart falls back to a local build.

## Row 49 — two followers of one release share a staging directory

From the review of row 35's second landing; it predates that branch. Two followers drawing
offers that name the same release use the same staging directory
(`elohim/eprfs/epr-cli/src/bin.rs`, about lines 1271 to 1347). The first verifies the release's
CID there; the second deletes and recreates the directory for its own attempt; the first can
then run its staged exercise on the second's bytes by pathname, before its own post-ingest
comparison. It needs two followers at once and an offer whose bytes differ under one claimed
release, so it is narrow. The cure is a staging directory per attempt, verified and exercised
in place.

## Row 50 — a watch on the inbox ends without saying so

2026-11-20. Two sessions on two devices coordinated by berth notes, each reading its inbox
with a background loop that polls and exits. Ethosengine's loop lapsed and a review verdict
sat unread from 10:00Z to 13:42Z; earlier in the sprint the steward had to tell each session
by hand to check its inbox. `berth inbox --follow` marks notes seen as it prints, so a loop
that pipes it loses notes, and a `--peek` poll has to be re-armed after every exit. Nothing
tells the sender that the addressee has stopped listening: `berth status` showed the sibling
STALE on its mooring record while it was still sending. Owed: a note's sender can read whether
it was seen, and a moored session with unread notes addressed to it hears about them on its
next tool call the way a gate red does.

## Row 50, correction (2026-11-20, 14:20Z)

Half of row 50 was wrong when I wrote it. The delivery it asks for exists on dev:
`.claude/hooks/berth-note-inject.py` adds each unread note addressed to the acting session to
its next tool call, and the root guidance describes it under "Peer notes". It did not run in
shem's session because a session started in the main checkout loads its hooks from the main
checkout, and shem's is 265 commits behind dev (head `c64967822`, a pin commit that is not on
origin, with 35 modified and 14 untracked files) and has no such hook. This is the second time
in two days that a stale main checkout cost hours (2026-11-19: 134 commits behind; status,
stations and a projection all read stale). Whether ethosengine's main checkout carries the
hook was not read.

What is owed, restated: (a) a main checkout that follows dev on each device, or a line at
session start that says how far behind it is and names the hooks it lacks; (b) still true as
written: a note's sender cannot see whether it was read, since seen state stays on the
receiving device.

## Row 43, delta (2026-11-21): ruled, built; what the precheck still does not see

The steward ruled the first cure on 2026-11-21: bump the four versions and roll Edge. Built in
`79ceff095` (eprfs-core 0.3.6, eprfs-storage 0.3.9, eprfs-meta 0.3.7, eprfs-local 0.3.7, eight
lockfiles, `[edge:roll]`) and `5535405d4` (the precheck). The precheck's gap is closed for the
class that caused this: each entry of the published lock must still be reachable from the crate
in the workspace lock, a path member at the same version, any other package at the same version,
source and checksum. Against the six published archives it refuses nothing with the lock before
the `cc` move and exactly `cc 1.2.65` and `find-msvc-tools 0.1.9` after it.

Still open behind it, each one the publish stage catches and a device does not:

1. An entry the crate has gained, an entry's own dependency list, and the lockfile's format are
   not compared. The script reads no manifest graph.
2. A change in an inherited manifest field and a file-mode change escape the source comparison
   (found by the reviewer, present before this change).
3. The publish stage waits 55 s in all for a new version to appear in the index and retries
   nothing else; a slow registry fails a first publication after a successful upload.
4. Rule 3 of the row stands: while storage takes eprfs-core by path, every bump of a published
   eprfs crate is an Edge roll.

## Row 42, delta (2026-11-21): four layers hold the pipeline file in the device's key

Read by a design leg (Codex gpt-6-astra, receipt in the session's campaign directory). A
pipeline file under `elohim/eprfs` reaches the device's verdict through four places, and all
four must let go of it: the manifest's `sources` (`elohim/eprfs/build-manifest.json:15,49`), the
step fingerprint and the project tree in `gate-evidence.mjs` (about line 691), and the bin
store's source state, which hashes the whole tree
(`device-runtime/src/bin_store/source.rs:208,274`). Narrowing only the first three would let a
landing reuse its gate while a follower finds no offer for its source state and builds for
itself. The fourth is in `device-runtime`, an Edge build input. So the cure is one change across
JS and `device-runtime`, landed with a deliberate roll. brit 0.3.0's build-inputs work does not
do this: it adds gitlink and symlink identity to a step's inputs.

## Row 51 — a lock move selects gates whose crates cannot reach it

Measured on the row 43 landing (2026-11-21, shem). Three path entries moved in
`elohim/Cargo.lock` (eprfs-core, eprfs-storage, eprfs-local). Five of the 21 selected gates were
selected for that file alone, and none of their crates reaches a moved entry in the lock's
dependency lists: doorway 618 s, elohim-epr 118 s, elohim-placement 65 s, elohim-live-session
17 s, elohim-compute 12 s. 830 s of a landing for lock bytes those builds never read.

The same holds for every third-party move in a shared lock. A security fix to one package in
`elohim/Cargo.lock` re-keys every project that lists the file, whether or not its crates use
the package.

The cure belongs in the step-input model, not in a gate script: a lockfile input keyed as the
entries reachable from the step's own packages (name, version, source, checksum), computed the
way `publish_eprfs.py locked_graph` now walks a lock. `brit build fingerprint` is where a
step's inputs are hashed. A whole-lock property, that the lock is consistent with every
manifest, still has to be checked by one gate per workspace on every lock change. Receipt: a
landing that moves one entry in `elohim/Cargo.lock` prints `reused` for a project whose crates
do not reach it.

## Row 52 — ark hashes a child's bytes three times per spawn, with a debug build of SHA-256

Found 2026-11-21 by a delegated read of the `device_supervise` stories (Claude Opus 5.5; the
hash times are its own measurements with the pool's debug `ark`, at load average 9 to 11). On a
`bin:` spawn ark hashes the file twice before exec (`elohim/ark/supervisor/src/native.rs:111`
and `:130`, the same file both times) and once for the identity rung (`:203`). The device's ark
is a debug build and `elohim/Cargo.toml` has no `[profile]`, so SHA-256 is unoptimised: 11.2 to
12.2 s per hash of the 158 MB debug `epr`, against 1.3 s for `sha256sum`. About 33 s of hashing
per peer spawn. `FIRST_SPAWN_WAIT` cites real restarts of 23 to 43 s, which fits.

This is paid by every real restart on a device and by every one of the 18 lane stories the
eprfs gate runs since `62aef22a`.

Cures, none taken tonight because each re-keys every cargo gate in the workspace or is an Edge
build input:

1. `[profile.dev.package.sha2] opt-level = 3` for the `elohim` workspace (the eprfs workspace
   already optimises its dependencies in dev builds). Dev profiles only; a release build is
   unchanged.
2. Hash once per spawn: absolutise the path first, then hash, in the `ArtifactRef::Channel`
   branch.
3. `START_WATCH` (`epr-cli/src/device/supervise.rs:51`) is a fixed 1.5 s on every ark start
   that waits for something not to happen; a real cure needs ark to say it got past its
   refusals.

The test-side cut, a stripped fixture release (66 MB, 4.8 s per hash), is on branch
`wallclock/test-wall` and is in the sprint journal.

## Row 53 — the device runs the eprfs tests one at a time, and no record says why

`genesis/agentic/pool-policy.json` sets `cargo_env_overrides.eprfs.RUST_TEST_THREADS = "1"` on
a device with 24 CPUs, so a test binary's wall is the sum of its tests. CI's nextest run of the
same workspace overlaps them (2,695 tests in 804 s, with `device_supervise` alone summing to
1,707 s). No comment, habit line or journal entry found gives the reason for the setting. If it
guards the timing-sensitive stories, those can be named and held serial while the rest run on
threads; if it guards memory, the measured peak belongs beside the value, as
`CARGO_BUILD_JOBS` has for storage.

## Row 54 — a session inherits `EPR_BIN` at a release the device has moved past

Met by two sessions on shem on 2026-11-20 and 21. The launching shell exports `EPR_BIN` at epr
0.3.2 (`bafyreibrqq…`) while the device's current release is 0.3.3 (`bafyreicfbac…`, which the
peer daemon already runs). Session start says "degraded epr binary … (candidate)". With the old
value: `epr session commit-trailers` does not exist, so a commit carries no trailers and no
warning; every brit command prints "feedback capture gap"; and one orchestrator test fails on
"native evaluator … emitted closureCid bafkrei…, not a byte CID; it predates abd66bdb1", a red
that is not in the tree. Each session worked round it by passing `EPR_BIN` per command. Owed:
the arrival step that makes the candidate current, or a session start that exports the
session's pinned epr and not the device's landed one.

## Row 55 — the a2o devflow unit tests run in no gate, and eight of thirteen are red on dev

`genesis/a2o/steps/devflow/__tests__/landing-queue-fixture.test.ts` on `dev` `4ecd40d4f`, run
with the package's own runner (`pnpm exec tsx --test`): 5 pass, 8 fail. Seven fail on
`EISDIR: illegal operation on a directory, read` from `orchestratorClosure`
(`gate-closure.seed.ts:84`), which follows the gate's imports and meets a directory. The a2o
gate is lint, format and typecheck (`_gate-genesis-a2o`), so nothing runs this file and nobody
was told. Found when Track B migrated the one test in it that called the oracle.

## Row 56 — `epr session gc` cannot seal an envelope whose worktree is gone

2026-11-21: `epr session gc --json` sealed 26 dead envelopes and failed on two,
"brit-build-ref covenant list failed … not a git repository" for
`/projects/elohim/.worktrees/hc-vectors` and `hc-stories`, both removed by the cleanup of
2026-11-20. Those two sessions stay unsealed on every later gc. A removed checkout has no
covenant to list; the seal should say so and close.

## Row 57 — an interactive session ignored TERM and HUP

2026-11-21: asked by the steward to end a finished session whose terminal he could not reach
(pid 1736356, `claude` on a pts), this session sent TERM, waited 30 s, sent HUP, waited 15 s,
and ended it with KILL. Its hook child `dialogue-stasis-sense.py --rewake` was left as an
orphan and ended by hand; `berth unmoor` for another session's mooring is refused
("envelope-owner-mismatch"), correctly, so the mooring was cleared by `epr session gc`. There is
no supported way for one session to close another at its steward's word.

## Row 58 — the stasis loop reads an unattended run as a silence stall

2026-11-21: with the steward asleep and a three-hour push running, the dialogue-stasis hook
opened incidents `…:5`, `…:9` and `…:17` as "silence stall", parked at rungs 1 to 3 with "tool
and shape are null", spent keeper calls on each, and asked three times for a push notification.
No effect was blocked. This session did not send the notification: nothing needed the steward's
decision, and he had said good night. The loop has no state for "the steward directed an
unattended run"; the standing ruling it looks for was on record (the covenant's two clauses) and
did not count.

## Row 59 — one root justfile is in the key of every gate that runs from it

`gate-evidence.mjs` `recipeAt` (about line 177) keys a `root-just` project on the blob of the
whole root `justfile`. Editing one recipe therefore re-keys every such gate, and because
manifests list the justfile under `buildProcess`, the edit also selects them: a one-line change
to `_gate-eprfs` costs a landing every gate, not eprfs's. Met on 2026-11-21 when the cheapest
cut to the eprfs gate, a step of its own for the ark stories with its own thread count, was
priced: it would have cost more than it saved for many landings. Same class as rows 42 and 51:
a whole file in the key where a part is read. The cure keys a gate on its own recipe's text
and the recipes it calls (`just --show`, followed through), which `just` can print.

## Row 60 — the device runs `cargo test`, CI runs `cargo nextest`

The device gate's test step is `cargo test --workspace` with one thread; the pipeline's is
`cargo nextest`. They differ in more than the missing build that row 41 found. nextest runs
one process per test, so tests there may mutate the process environment or working directory
freely, and it overlaps them, so 2,695 tests take 804 s where the device takes 2,536 s. A
device at four threads would be neither: threads share a process. One unit test in
`eprfs-local` changes the working directory mid-test
(`eprfs-local/src/capture/tests.rs:784`), which is safe under both of today's runners and a
race under threads. nextest is not installed on the devices (root guidance, 2026-07-30).
Installing it through the bin store and running the pipeline's command in the device gate would
make the two recipes one and give the device the parallelism, at the price of row 59's re-key
once.

## Row 61 — a quiet sibling fences the lander at the end of a long gate

2026-11-21, shem: after 13,175 s of green gates the hook's Phase L renewed the pushing
session's own `dev-integration` lease and berth refused, "self-demoted: no head from
did:key:z6MkpZRh…VMZB within 570s". Eight seconds later that device's head was fresh. Its
sessions had closed for the night, so its heads came only when it had something to record, and
at that instant the last one was older than the fence window.

The hook claims through a landing agent with `--offline-ok`, which writes an override for one
silent device and waits for it (at most 660 s). That path was not taken, because the pushing
session already held the lease (claimed by hand before the push, as the root guidance asks:
"coordinate the landing lane before pushing"), and a renewal as that session is a plain claim.
So the careful path, claim then push, is the one that can be refused after hours, and the
refusal names no cure but "push again".

Owed, any one of: the renewal of a lease the pushing session already holds goes through the
same `--offline-ok` rule as the landing agent; or a device with no live session still sends a
head at a cadence shorter than the fence window; or the lane is claimed and checked before the
gates as well as after, so a fence is known before hours are spent. The gates were not paid
again: the second push reused their verdicts.

## Row 62 — a security fix in the k8s bridge's lock is an Edge roll, and waits for one

2026-11-21. The security ratchet named `bridges/k8s/Cargo.lock` on the row 43 landing: four
alerts on rustls 0.23.43, patched in 0.23.46, the one lock-only fix among the four lockfiles
that landing touched. It is built and parked on local branch `wallclock/k8s-rustls`
(`c6149616`, on shem; `cargo metadata --locked` exits 0). It did not land. The preflight showed
what a walk of the changed paths had not: `bridges/k8s/Cargo.lock` is a storage build input, so
the commit selects the storage gate on a device and, in CI, `cargo-build-storage`,
`build-edge-image` and `deploy-manifests`. A second fleet roll hours after the first is the
steward's to time. It rides the next deliberate roll.

The alerts on the storage, node and eprfs locks from the same ratchet output need the libp2p
0.56 upgrade (hickory 0.24, libp2p-gossipsub 0.47, libp2p-quic 0.11, the rustls-webpki 0.101
line, ring 0.16) or the iroh upgrade (hickory 0.25 and lru 0.13 in the eprfs lock). None is a
lock move.


## Row 63 — sixty branch builds that could not refuse themselves starved the Edge roll for four hours

Read on 2026-11-21 at 13:10Z from Jenkins' queue and node lists. 62 items were queued and 61 were
flagged stuck, each waiting on a Kubernetes agent that was offline:

| Items | What | Queued since |
|---|---|---|
| 60 | build #1 of feature-branch jobs: elohim-dna 18, elohim-steward 17, elohim-edge 14, elohim-app 11, over 18 branches | 56 between 15:28Z and 15:50Z on 2026-11-20, four at 18:18Z |
| 1 | `elohim-edge » dev #1919`, the Edge roll of the row 43 landing | 09:52:10Z, its first second |
| 1 | the next `elohim-orchestrator » dev` run | blocked behind #2402, which waited on #1919 |

Each of the 60 was started by "Push event to branch …" (`jenkins.branch.BranchEventCause`), not by
the orchestrator. Two things in the repository let that through. `overrideIndexTriggers(false)`
sits in each downstream Jenkinsfile's `options`, and a branch job's first build runs before
Jenkins has ever read them. And the dna, steward and app Jenkinsfiles request their pod at the top
level (`elohim/holochain/dna/Jenkinsfile:239`, `steward/device/Jenkinsfile:22`, `Jenkinsfile:1149`)
and check the build's cause in a later stage (`:396`, `:121`, `:1263`), so a build asks for an
agent before it can refuse itself. The pipeline's `timeout` does not cover the wait for that
agent, so they hung for 21 hours.

`#1919` never got an agent. From the outside it read as a build "running" for four hours; the
session watching it reported the roll as slow until the queue was read. Jenkins' log readers time
out on such a build, so the queue and the node list are the only place it shows. The steward
aborted the 60 and `#1919` by hand at about 13:50Z; `#1920`, dispatched by orchestrator `#2403`,
had an agent within minutes.

Not determined: why the agents were offline (the cluster's view is the operator's), and which push
produced events for about fifteen old branches at 15:28Z on 2026-11-20. It coincides with that
day's branch cleanup on origin.

Owed: the downstream Jenkinsfiles check the cause before any pod is requested (`agent none` at the
top, the check on the controller, the pod per stage) and bound the wait for an agent; the
multibranch jobs suppress automatic triggering for every branch; and the session-start line that
reads the pipeline's verdict also reads the queue's stuck count, which is one public request.

### Row 63, delta (2026-11-21, 14:30Z): two pipelines refuse before the pod; six wait for a roll

The steward asked for the cure. The rule each pipeline's Check Trigger stage applies now also
runs as plain script above `pipeline {}`, where the controller evaluates it before any agent is
requested; a refused build ends NOT_BUILT in seconds. The stage stays as it was. Jenkins's
`pipeline-model-converter` accepts the shape for all eight downstream Jenkinsfiles and the CPS
method-size gate is unmoved, since the check sits outside the pipeline block. Wrapping the
stages in a parent stage that holds the agent, which would also bound the wait for an agent,
was not possible for the app and genesis pipelines: both sit at the stage-model limit.

Landed: the app and steward pipelines (two of the four that jammed). Parked on local branch
`wallclock/jenkins-guard-rest` (shem): dna, edge, epr, eprfs, genesis and library. Their files
are build inputs far beyond their own pipelines (row 42): `elohim/holochain/Jenkinsfile` selects
an Edge build and deploy; `elohim/holochain/dna/Jenkinsfile` makes `build-dna-wasm` stale and,
through it, the Edge image and its deploy, which a walk of the file's own pipeline did not show
and the preflight's fleet-roll row did; `elohim/epr/Jenkinsfile` selects `cargo-build-storage`
and twenty device gates; `genesis/Jenkinsfile` selects `seed-content`. They ride the next
deliberate roll.

Still owed:

1. A refused build no longer runs the pipeline block, so a branch job that has only ever been
   refused never receives `overrideIndexTriggers(false)`, and each later push event starts
   another build that ends in seconds. The cure that stops them starting is on the Jenkins
   side: "suppress automatic SCM triggering" on the multibranch jobs.
2. An admitted build that never gets an agent still waits without bound, as `#1919` did.
3. The 18 branches on origin that received the events.

## Row 64 — the build hook counts compiler processes against a seat limit of two

Found 2026-11-21 while reading the open rows against the steward's market picture
(native-dev-tooling-over-peers-plan §9, test 3). `genesis/agentic/pool-policy.json` sets
`io.max_concurrent_heavy` to 2 and berth offers `cargo: 2`, both by the operator's ruling of
2026-09-09. Rule 4 of `.claude/hooks/cargo-disk-guard.py` compares that limit with the number of
distinct heavy processes `running_heavy()` finds: every cargo driver, every `rustc`, the linkers
and `gate-runner.mjs`. One running gate is at least two of those, so the hook refuses a second
build with "ONE BUILD AT A TIME" whatever the limit says. A device therefore gives two answers to
"is there room": berth grants a second seat and the hook refuses its use. The hook also reads only
the command text, so a script that runs cargo passes it; that is how a measurement ran beside a
landing's build on this sprint's first night (journal, 06:15Z to 08:10Z).

The steward ruled the same day: two seats, then measure. Owed, in order:

1. The hook counts builds, and its refusal says how many seats are taken. Its memory and I/O
   rules stay as they are.
2. The push loop can hold one gate in flight on the second seat and collect its verdict at the
   end; a red from either gate still refuses the push.
3. One landing that selects `elohim-storage` is measured with two gates overlapping, with
   ram-guard and io-guard lines recorded through the overlap, against the serial figure. If the
   overlap saves nothing or a guard sheds, item 2 stays behind a switch that defaults off.

The design sentence "Gates still serialize on one device" (native dev plane §11) described a
workspace of 31 GiB. ram-guard read 14.6 of 41.0 GB committed on 2026-11-21. This host's storage,
not its memory, is the stated bottleneck, which is why item 3 comes before any reliance on item 2.

### Row 64, delta (2026-11-21): the hook counts builds

Item 1 is done. Rule 4 of `.claude/hooks/cargo-disk-guard.py` now counts builds through
`group_builds`. A seat is held by a cargo driver or gate-runner with no driver below it, by a
driver that is itself compiling, and by a compiler whose cargo is gone. A process whose ancestry
cannot be walked to the top counts on its own, and a failure of the grouping counts every
process, so an error refuses work. The refusal reads `CARGO SEATS FULL: N of M builds running`,
names each build and what it runs under, and prints a wait over those `/proc` directories.
Against this device's live process table during a six-job build, the old rule read seven
processes against a limit of two; the new rule read one build of two.

Four review rounds by Codex astra (receipts `astra-seats-review*.receipt.md` in the sprint's
campaign directory on shem) each ended "fix first", and each fix is in: two cargos compiling
under one parent counted as one seat, side by side and then nested; a new test that would have
sent a real signal to whatever held pid 100; a wait hint that waited for every build to end; a
cargo started with an option before its subcommand, or with a subcommand the hook does not know,
was never listed, so two of them shared the gate-runner's seat; and the `d` alias and four
compiler names (`rustdoc`, `clippy-driver`, `cc1plus`, `clang`) were missing from the lists. The
last fix was not read again.

What the count still rests on: a list of cargo subcommands and a list of compiler names. Two
cargos of an unlisted subcommand, each running only an unlisted compiler, under one gate-runner,
would still share its seat. The memory and I/O rules do not depend on either list.

Found on the way and still open:

- Neither `test_cargo_disk_guard_yield.py` nor the new `test_cargo_disk_guard_seats.py` runs in
  any gate: the root justfile runs one other file from `.claude/hooks/tests`. Wiring them in is
  a justfile edit, which re-keys every gate that runs from it (row 59), so it waits for that row.
- A gate-runner holds a seat through its non-cargo steps, and a gate with no cargo holds one for
  its whole run. That is the old behaviour and the safe direction.
- `genesis/docs/superpowers/specs/2026-10-26-shared-toolchain-servers-design.md` quotes the old
  refusal text in three places.
- The hook reads a proposed command with a separate parser (`segment_head_and_verb`), which was
  not read in this pass. Whether it gates `cargo --color=always build` is unmeasured.

Items 2 and 3 are unchanged: the push loop, then the measured landing.

## Row 61, delta (2026-11-21): a fenced hand-held lease is handed back and claimed afresh

The cause is in berth's settle step (`_settle_steward` in `genesis/agentic/bin/berth`): the
one-device override that `--offline-ok` writes is taken only for a lease that is not yet fenced.
A lease claimed by hand before hours of gates is already past its fence window when the hook
renews it, so the renewal is refused where a fresh claim would have written the override and
waited for it. berth's behaviour is unchanged.

The cure is the first of the three owed, in the hook: when the pushing session's renewal is
refused with exit 3 and berth's reason is `self-demoted: no head from …`, `phase_l_claim`
releases the session's lease, says so in one line, and lets the landing agent claim afresh under
`--offline-ok`; the session does not hold the lease afterwards. A failed release refuses with
both reasons. Every other refusal of a renewal reads as before. The match on berth's phrase
lives in one function (`phase_l_fenced_reason`), and `genesis/agentic/berth_test.py` already pins
the phrase. Three tests in `genesis/orchestrator/prepush-berth.test.mjs` drive it against the
fake berth: the order claim, release, claim; a failed release; another device holding the lease.

Not observed against a real lease: the tests fake berth. The other two cures stay open, and the
second (a device with no live session still sends a head inside the fence window) is the one
that would also keep an idle device visible as a carrier.

### Row 64, delta 2 (2026-11-21): the second seat is a carrier behind a contract, landed in shadow

Item 2 is built, and not as logic in the push hook. `genesis/orchestrator/gate-carry.mjs` holds a
carrier contract (admit, offer, collect, withdraw), one pure algorithm (`planCarry`) and the first
carrier, `second-seat`. `berth room cargo --json` is the one capacity answer, and the push hook
only calls in. The switch `GATE_CARRY=off|shadow|on` defaults to shadow, which prints what it would
carry and starts nothing (native-dev-tooling-over-peers-plan §9, "Interfaces").

Found while building it:

- Two writers to one gate run's event log could take the same sequence number: four concurrent
  writers gave 481 unique numbers for 601 events, so a cursor could skip an event. Two gates
  finishing together could also lose a row of the cycle ledger. Both now go through
  `genesis/orchestrator/file-lock.mjs`.
- `elohim-storage` and `epr-storage` build in one cargo target, as do `eprfs`, `memory-ceremony`
  and `orchestrator-tree`. Two gates that share a target queue on cargo's own lock, so `planCarry`
  refuses to carry one beside the other. A landing that selects both storage gates therefore
  carries nothing today. The refinement that makes the 21-gate landing pay: the unit carried is
  the group of steps that share a target, run in turn on the second seat (3,535 s and 1,360 s).
- The build hook counts every gate-runner as a seat, including the fixture gate-runners that the
  orchestrator's own hook tests start. A real build was refused for seven minutes by a fixture a
  killed test run had left behind. The tests now reap their fixtures; the count is unchanged and
  stays the safe direction.

Not yet observed: `GATE_CARRY=on` with a real gate. Item 3, the measured landing, is still owed,
and the measurement taken the same day (below, and the sprint journal) bears on it.

## Row 58, delta (2026-11-21): the keeper ladder climbed on after the steward approved the prompt

Incident `824f0d7e…:145`. A command met an ask rule at 17:32:44Z and the steward's dialog stayed
open for eight and a half minutes, so the ladder's first rung ran while the wait was real. The
steward approved at 17:41:20Z and the sensor recorded `prompt-closed, closedBy: approved`. The
coordinator then ran rung 2 (a Sonnet keeper) and rung 3 (an Opus keeper), kept telling the
session not to retry a command that had already run, asked for a push notification, and blocked
the session's stop.

Two causes. The native fold keeps the first closer: the first rung's Permit closes the incident
as `parked`, and the later `approved` is ignored (`elohim/eprfs/device-runtime/src/session_stasis.rs`,
`close()` is first-wins and Parked is not Closed). And the ladder in
`genesis/agentic/bin/stasis-coordinator` treats `parked` as still open and takes the next rung.

Fixed in the ladder: before a rung or a delivery it reads the sensor's own close record for the
incident from the journal it already holds locked, and if the dialog is over it ends, writes one
`ladder-ended` row, withdraws the parked ask and delivers nothing more. Five tests. Still owed:
the native fold should let an approval close a parked incident, which is a change in
`device-runtime` and so waits for a deliberate roll; and the coordinator daemon on shem has run
since 2026-11-10 and takes the fix only when restarted, which is the steward's to do because it
serves every session on the device.

## Row 53, delta (2026-11-21): measured, and the reason for one thread is the I/O guard

The eprfs workspace tests (2,698 tests, the live-corpus legs skipped) on shem, one build, by
`RUST_TEST_THREADS`. Per-run logs and a 20 s sampler of the guards are in the sprint's campaign
directory on shem (`threads/`).

| Threads | Runs | Clean | Wall-clock | I/O guard pauses | Peak writes | Peak host I/O pressure |
|---|---|---|---|---|---|---|
| 1 | the gate on landing 2 | | about 1,620 s | | | |
| 4 | 5 | 5 | 604 to 656 s | 3 | 61 to 72 MB/s | 14.8% |
| 3 | 10 | 8 | 696 to 752 s | 4 | 58 to 71 MB/s | 15.0% |
| 2 | 2 | 1 | 923 s; 977 s | 0 | 59 to 63 MB/s | 8.8% |

Nothing was changed. The bar was ten clean runs at the chosen count beside a running build, and
three threads gave eight. The two failures there were both in runs the I/O guard had frozen for
about 36 s: a case with a 15 s subprocess timeout expired, and a berth call inside another case
failed. The failure at two threads was a different defect, a timing assumption in a test, since
fixed (row 66). So the row's question has an answer: more threads compress this workspace's
writes past the guard's first line (10% full avg10), the guard freezes the run, and a frozen run
fails whenever a shorter timeout is in flight. The cure is the write volume (row 65), after which
the count is measured again.

## Row 60, delta (2026-11-21): threads are safer here than the row said

The row names one unit test in `eprfs-local` that changes the working directory and calls it a
race under threads. It is not: `relative_roots_remain_bound_when_process_cwd_changes` re-executes
itself in a child process and changes the directory there
(`elohim/eprfs/eprfs-local/src/capture/tests.rs`, the `EPRFS_CAPTURE_CWD_TEST_CHILD` guard). In
the whole workspace's test code six lines change process-wide state: three in that child, and
three in `eprfs-peer/tests/relay_compat.rs`, a binary with one test. Running the pipeline's own
nextest command on the device is still the way to make the two recipes one.

## Row 65 — the slow eprfs test file makes its own stripped copy of `epr` for every story

`elohim/eprfs/epr-cli/tests/device_supervise.rs` builds each story's release by running
`strip --strip-all` over the 160 MB debug `epr` (66 MB out, about 4.8 s) and then ingesting it,
once per story, 29 stories. At one thread those writes are spread over twelve minutes. At four
threads they are compressed into about three: the test run wrote 61 to 72 MB/s and pushed the
host's I/O pressure to 11 to 15% against the I/O guard's first line of 10%, and the guard paused
the run for 36 to 41 s in three runs of five (2026-11-21; one of those with no other build
running). At three threads the guard paused four runs of ten, and two of those failed because a
case with a 15 s timeout was in flight while the run was frozen; at two threads the pressure
stayed under 9% in two runs.

Owed: one stripped copy per test process, shared by the stories that never modify their release
and copied only by the ones that do. That removes most of the strip time (about 139 s of a
one-thread run) and most of the write burst, after which four threads can be measured again.
Not done here because some stories append to their release to make a second one, and telling
those apart is a reading of all 29.

## Row 66 — two session-envelope cases asserted a refusal that held only until a holder noticed

Found by load on 2026-11-21: one eprfs test run in eleven failed, the one that overlapped a
second build's five thousand tests (load 15). `ResumeEnvelopeTest.test_resume_refusals_write_nothing`,
subtest `namespace`, expects `epr session open` to refuse across a pid-namespace mismatch. The
case removed the predecessor's fixture `/proc` entry first. The predecessor's holder polls for
that once a second, exits and releases its lock, and a released holder is, by design, proof of
death across a foreign namespace (`device-runtime` `envelope.rs`, `holder_released`, with its own
test). So the refusal held only until the holder noticed, and a loaded device ran the command
after that. The product is as designed; the case was not (diagnosis: Codex astra, receipt
`astra-namespace-flake.receipt.md` in the sprint's campaign directory).

Fixed on shem's branch `wallclock/eprfs-threads` (53ea31d7b), not yet on dev: in
`genesis/agentic/session_envelope_test.py` the `namespace` subtest and the sibling
`test_gc_preserves_unknown_and_foreign_namespace_but_collects_a_previous_boot` keep the
predecessor's entry until their assertion has run. Three clean runs at load 13.7 to 17.9
afterwards. It lands with the next eprfs change, so the eprfs gate is paid once. This matters beyond the thread count: with two cargo seats in use, a timing
assumption like this one is how a second build turns a green gate red.

## Row 67 — the pool evicted the landing worktree's warm slot while a measurement worktree built the same workspace

2026-11-21, shem. The eprfs-live-corpus gate took 61 s on the 18:53Z landing and 648 s on the
20:31Z landing: it recompiled 512 crates, because the landing worktree's eprfs slot
(`family/integrate/elohim__eprfs`) was empty. The storage gate's test build that afternoon was
cold in the same way (547 crates against 1 in the morning). In between, a measurement worktree
of the same session had been building the eprfs workspace in its own slot for four hours.

`genesis/agentic/pool-policy.json` caps the pool at 150 GB (`pool_max_gb`) and keeps the freshest
slot per workspace, making colder duplicates candidates oldest first. The pool stood at 147 GB
when the session began. Not observed: the eviction itself. The pool log readable on the device
held only `steward_clean` rows.

So the isolation rule (never share a gate target across worktrees) and the cap interact: a
second worktree that builds a workspace makes the landing worktree's slot the colder duplicate,
and near the cap the next landing pays a cold build. Owed, any one of: the slot a landing
worktree uses is pinned while the worktree lives; a measurement family is pruned by whoever
made it when it is done; the reconciler prefers evicting a family with no live worktree.

## Row 68 — four landings each replaced two documents with their newest section, and no gate noticed

2026-11-21, shem, this sprint's session. Landings 7 to 10 (101b7b767, cdb1ad58c, 4f63afdea,
e83d0931b and 05333e6ae) each "appended" a section to this backlog and to the wall-clock sprint
journal with one line of Python that opened the file for writing before reading it, which empties
it. Each commit therefore left the file holding only its own new section: this backlog went from
1,531 lines to 39 and stood at 70 on dev; the journal went from 553 to 38 and stood at 40. No
commit message mentioned it, the cite seal passed on the stumps, and five green landings carried
it. The sibling device found it within the hour, during a second-device check asked for another
reason: "101b7b767 (build-hook seats) cut steward-device-arrival-open-items.md 1531->~40 lines
and wall-clock journal.md by ~573 lines; its message names neither … looks accidental, please
check."

Repaired the same evening by an append-only commit: each document is its last whole version
(21a8060cd) followed by every section the later commits held, in order, checked heading by
heading (83 here, 34 in the journal). Nothing was lost, because every stump was a whole section.

Owed: a seconds check before the gates that refuses a pushed commit which removes most of a
tracked document without saying so. It would have stopped the first of the five. The session's
own habit is changed too: read into a variable, then write.

### Row 65, delta (2026-11-21): one stripped release per tag; what is left is the product's ingest

The count in the row was wrong: it is one strip per release, 41 across 18 stories, and the 11
stories that use a bash fixture strip nothing. A release's bytes depend only on its tag, so the
test file now makes each tag's bytes once per process, read-only, and hard-links them into each
story's release directory (landing 12). Written bytes fall from 5.83 GB to 3.27 GB and bytes
reaching the disk by about 23%. The time barely moves: 707 to 711 s at one thread against 719 s.

Two things this did not settle. The remaining 2.8 GB is the product's own ingest, which reads
its source and writes an fsynced copy, 41 times; linking or reflinking a read-only source into
staging is a change in `device-runtime` and waits for a roll. And this file may not own the
burst that trips the I/O guard at three and four threads: alone at four threads it peaked at
26 MB/s before the change, against 58 to 72 MB/s in the whole-workspace runs, and both runs that
failed were frozen while the session-envelope cases ran. The next measurement is which test
binary owns that burst, before any thread count is tried again.

### Row 66, delta (2026-11-21): landed

The two cases keep their predecessor's entry until their assertion has run, on dev with landing
12.

### Row 68, delta (2026-11-21): the check exists

`doc-cut` runs before the gates beside fleet-roll (`genesis/orchestrator/doc-cut-guard.mjs`,
wired in `gate-preflight.mjs` and `.husky/pre-push.bash`; the gate knobs in the root guidance
describe it). It compares the two ends of a push, refuses on dev or main when a `.md`,
`.feature` or `.yaml` document would be left under half its length and at least 100 lines
shorter, and is cleared by `[doc:cut]` on a pushed commit that touches the file. Against the real
history it refuses the first of the five commits in 0.22 s and names both documents. Known
limits: a rename git cannot pair reads as a delete and an add and is not seen; the hook's own
call has a syntax check and node-side tests but no end-to-end hook test.

## Row 42, delta 2 (2026-11-21): the native cut is built and reviewed, and the rule it rests on was wrong as first stated

The sibling device built the first two of the four layers natively: rakia's planner can say which
input class selected a step and plan from sources alone, and `brit build affected` and
`brit build fingerprint` take `--inputs sources|all` (rakia `ethos/input-classes` 158f2365, brit
`ethos/input-classes` 3f6b0c8d; record `genesis/docs/superpowers/sprints/2026-11-21-wall-clock/input-classes.md`
on the hand-over branch `self/ethosengine-input-classes`). With the flag omitted every output is
byte-identical and pinned as a literal. Its finding: a step's sources class has to be its sources
minus its own `buildProcess`, because five steps cover their own pipeline file with a source glob.

An independent review on shem then found the rule this session had written into the peers plan
to be false on today's manifests. "A step's `buildProcess` is CI's" does not hold for two
gate-only TypeScript steps, which listed `pnpm-lock.yaml`, `pnpm-workspace.yaml` and a codegen
script there; under `--inputs sources` a lockfile bump would have reused a stale green. A census
of 25 manifests: 38 `buildProcess` entries, 19 pipeline files, 14 CI build scripts, and those
five. On the working branch the five were moved to sources (the union every current caller
hashes is unchanged; delta 5 records why that move is parked), and
`pipeline-registry.test.mjs` refuses a gate step whose `buildProcess` is anything but a pipeline
file or a `scripts/ci/` build script. The earlier design read had warned that CI ownership of a
file does not prove a device does not read it; this session kept its analysis and dropped that
caveat.

Owed before any consumer passes `--inputs sources`: on the two branches, the review's advisories
(a path containing `@` is truncated by the suffix strip; the `all` fingerprint bypasses the shared
helper; brit must require rakia-core 0.1.2; the record says a path matched by both classes yields
both reasons and the code, rightly, reports one). Then the release order, which is the steward's
to rule: rakia publishes, brit bumps and releases, two pins move, devices follow the new brit,
and the JS consumers adopt the flag, which re-keys every reusable verdict once. Layers three and
four of this row (the JS project-tree material and the bin store's whole-tree source state) are
not touched by any of this.

## Row 42, delta 3 (2026-11-21): the boundary is the executor, and a gate's own inputs block keeps both classes

Two more readings on the same day, before any consumer adopted the flag.

The second independent read of the sibling's fix commits found them release-ready: all five
review points fixed, and output with the flag omitted is the base code path word for word
(rakia 8a4f184a, brit 10fa5cac).

A design read of the adoption (Codex astra, receipt `astra-inputs-adoption.receipt.md` in the
sprint's campaign directory) pointed at gate projects that declare their own `inputs` block and
list `justfile` as `buildProcess`. A census of those blocks changes the rule's statement rather
than the manifests. 31 gate projects declare their own inputs, and there `buildProcess` is used
for the gate's own scripts: the rust-style checker and its policy file, rakia's validator, the
pipeline-list generator, the root justfile. Those are read by the gate's recipe. So the class
boundary is the executor, not the field name:

- A gate project's own `inputs` block is the gate's, both classes. Nothing is dropped from it.
- A step the gate borrows from a pipeline contributes its sources class only. Across every gate
  project there are 21 such `buildProcess` entries: 13 pipeline files and 8 image-build scripts
  under `scripts/ci/`. None is named in the justfile its gate runs from.

Design decisions taken from that read, for the adoption:

- One policy, read by selection, propagation and key alike, with a switch that starts in shadow
  (the plane's pattern for a behaviour change). A device whose brit predates the flag decides as
  today and says so once.
- No one-time re-run of every gate. A green recorded under today's key covers a superset of the
  files the narrower key covers, with every other piece of key material the same, so on a miss
  under the new key the gate may look up the old one and reuse it, saying which key matched.
  The old key is the one computed with the flag omitted, not with `--inputs all`, which strips
  function suffixes and so differs.
- Still owed by this row after adoption, unchanged: a pipeline file under a gate project's
  directory moves that gate's key through the project tree, and the bin store hashes whole
  source trees (layers three and four).

## Row 69 — dev pins a rakia commit that has no CI check, and one of its tests is red

Found 2026-11-21 when the input-class release stopped at its first step. Dev has pinned rakia
`f9787c9e` (the regenesis-1 release-manifest 2.0 schema commit, on `run-ci/regenesis-1-schemas`)
since 2026-11-05. The forge holds no check run for that commit (`check-runs` total 0, read
2026-11-21), so today's pin gate would refuse it as absent; how the pin landed is not
established here. Rakia's `main` was one commit behind it.

When the sibling opened rakia PR #4 to bring that commit and the input-class work to `main`, CI
ran it for the first time and one test was red:
`rakia-executor/tests/contract.rs`, `canonical_identity_matches_the_javascript_oracle_including_null`.
The schema commit had changed one line of that passing test: the expected address
`bafkreigyfkfu6h3…` became `bafkr4igyfkfu6h3…` with the digest characters untouched. Computed on
shem: the first is exactly the sha2-256 of the fixture bytes as a raw CIDv1; the second is the
same sha2-256 digest under the BLAKE3 multihash code. No hash of those bytes produces it. A
spelling sweep had rewritten a test vector's prefix. The commit's own message says the executor
keeps its earlier descriptors until the kernel swap, and its ignored sibling test holds the true
BLAKE3 vector.

Decided (shem, as the sprint's orchestrator): restore that one literal on the PR branch, leave
the swap staged, and carry on with the release. Done the same hour: rakia `main` is `f2c0d3c0`
(PR #4 with the restoring commit `70d9c4a0`), its `test` check and its publish both green on the
forge, and rakia-core and rakia-brit 0.1.2 are in the registry. Not decided here and owed in its own lane: the
executor kernel swap itself, whose stated precondition (elohim-epr 0.2.0 on the registry) the
sibling reports as met.

Two things this row keeps:

- A prefix is not an address. A string with the byte-CID prefix and another hash's digest passes
  a pattern and fails a full decode; the schema commit's own pattern is described there as a
  syntactic floor. Any sweep that rewrites address spellings has to recompute, never respell.
- A pinned commit with no check is unattested however long it has been pinned. The pin move
  that follows rakia 0.1.2 puts dev on a `main` commit with a green check.

## Row 42, delta 4 (2026-11-21): the adoption lands in shadow only; a bridge was built, reviewed and deleted

Delta 3 recorded a design decision that did not survive review: "no one-time re-run of every
gate", by looking a miss up under the old key and reusing that green. It was built that way
(`ebd1c22c2`), with the green promoted to a derived record under the new key. An independent
review (Codex astra, `astra-inputs-review.receipt.md` in the sprint's campaign directory) found
two defects in the promotion: a red that arrived between its two reads could be overwritten, and
a promoted green outlived a later failure recorded under the old key. Neither was reachable in
the default mode. The review also called the bridge over-built, and it was right, because the
premise was wrong. A verdict is reusable only for an identical closure, and identical closures
recur within hours: a retry, a re-gate after a moved base, a stacked landing, a sibling's
check. Changing what a gate is keyed on therefore costs at most one genuine run of a gate that
is selected while its closure is otherwise unchanged. It never meant every gate running once.

What is on the branch now (`e29cacd02`): `genesis/orchestrator/gate-inputs.mjs`, with
`GATE_INPUTS=shadow|off`, default shadow, and deliberately no `on`, as `gate-recipe-key.mjs`
does. Shadow decides nothing. With a brit that has the flag it prints, once per push, the
borrowed steps that only their own `buildProcess` selects, and per gate when a sources-only
key would not have moved; with a brit that predates the flag it prints one line saying so. Its
work runs under one deadline per process. The bridge, the derived-record shape and the
downgrade path are deleted.

What turning it on needs first, from the same review, and now written in the module's header:

1. A differential fixture of this plane's own path matcher against a brit that has the flag.
   They differ on dotfiles, on `*` across a directory separator, on `.epr-meta` paths and on a
   gitlink a glob reaches. The difference exists today in the legacy path as well.
2. A capability lost in the middle of a push refuses the gate with a named cure. A downgrade
   after selection cannot restore a gate that selection already left out.
3. Selection and execution agreeing in a standalone `just gate`.
4. The executor-boundary test reading a gate's audited recipe reads and not only the filenames
   its justfile names.

Not yet true of any of this: that it has run against a brit that has the flag. This device's
brit predates it; brit 0.3.1 is the sibling's release in progress.

## Row 65, delta 2 (2026-11-21): the burst belongs to `session_envelope`, measured

Three runs of the whole workspace's tests at four threads on dev `1b5f40212`, unloaded, with
each second's written bytes (the I/O guard's own source, the container's `io.stat`) attributed
to the test binary running then (records under `burst/` in the sprint's campaign directory):

| Test binary | Seconds | Written | Peak over 10 s | Highest I/O pressure | Guard pauses |
|---|---|---|---|---|---|
| `session_envelope` | 129 to 138 | 5.31 to 5.45 GB | 74 to 82 MB/s | 15.1 to 16.9% | one in every run |
| `device_supervise` | about 185 | 3.2 GB | 34 to 39 MB/s | 4.7% | none |
| everything else | | under 0.1 GB each | | | none |

All three runs passed, in 640 to 658 s, each with one pause of 36 to 41 s. A pause that lands
on one of that binary's 15 s subprocess timeouts fails the run, which is what two of ten
three-thread runs did earlier in the day. So the earlier attribution to `device_supervise` was
wrong twice: it is neither the burst nor what the guard acts on.

The cause is the same shape as the one fixed in `device_supervise`: `Fixture.setup()` in
`genesis/agentic/session_envelope_test.py` copies the 160 MB debug `epr` into every case's
fixture and then has the product ingest it, which writes a second synced copy. Owed, in work at
the time of writing: link one shared stripped copy into each fixture, so a fixture costs one
66 MB ingest and no copy; then the ten-run bar at four threads.

## Row 42, delta 5 (2026-11-21): the manifest correction is parked for a deliberate roll; the shadow and the pins land without it

The fleet-roll check, run by hand on the assembled landing before any gate, refused it: CI would
select an Edge build and its deploy step for an edit to
`elohim/sdk/storage-client-ts/build-manifest.json` or `elohim/eprfs/eprfs-ts/build-manifest.json`
(stale steps `cargo-build-storage`, `build-doorway-app`, `build-edge-image`, `deploy-manifests`).
Moving five entries between two lists in a manifest changes no built byte, and CI still reads it
as an Edge input, which is this row's own subject seen from the pipeline's side. A second fleet
roll in a day is the steward's to time, so the correction does not land here.

What lands: the input-class shadow, the two pin moves, and the contract test with those five
entries held in an exact parked list (a sixth offender fails; so does the corrected tree until
the list is deleted). The module's header names the correction among what `on` needs first,
because under the sources class a lockfile bump would neither select nor re-key those two gates.

Parked for the next deliberate Edge roll, with the six Jenkinsfile guards already waiting for
it (row 63): commit `324dcb4e6` on the local branch `wallclock/process-inputs` (the two manifest
edits), plus deleting `PARKED` from `pipeline-registry.test.mjs`. The pin moves do not select an
Edge build: CI's checkout initialises no submodule and prunes the dependency.

### Row 64, delta 3 (2026-11-22): the first carried gate, measured

Landing 13 (`9dbfbac2d`, pushed 2026-11-21T23:33:22Z, on dev 00:13:32Z) was pushed with
`GATE_CARRY=on`. Thirteen gates were selected. The carrier admitted one, `elohim-storage` (one
build job, recorded p90 above its 600 s floor), and ran it on this device's second seat beside
the loop; the other twelve ran in the loop in order, eprfs last.

| | Seconds |
|---|---|
| The thirteen gates, summed from the hook's own lines | 3,653 |
| `elohim-storage`, carried | 1,384 |
| `eprfs`, in the loop beside it | 1,702 (its test step 1,653) |
| From the carry's start to the last verdict | 2,311 |
| The whole push | 2,410 |

The carried gate was collected 39 s after the loop ended, so the overlap paid for all but 39 s
of it: about 1,340 s saved against running the same thirteen in order. Running beside a second
build did not slow the eprfs test step: 1,653 s here against 1,729 s alone in landing 12, with
the fixture change of row 65 between them.

Both guards, sampled every 5 s through the push (482 samples, `landing13-guards.jsonl` in the
sprint's campaign directory): committed memory at most 28%, level `ok` throughout; host I/O
pressure at most 6.75% against the I/O guard's line of 10%; no pause and no guard event. The
highest readings, and the one sample at the advisory write level (60 MB/s), fell at 00:09Z
while `session_envelope` ran in the loop, which is that binary's own burst and not the overlap.

What this does not establish. It is one landing. The carried gate compiled little (its target
was warm), so the memory figure is not the worst case the storage manifest's one-job cap was
set for. Only one gate qualifies today: eprfs asks for six build jobs and the second seat's
terms take one. `GATE_CARRY` stays in shadow by default; this session passes `on` by hand and
records each carried landing here, and proposes the default change after three with both guards
under their lines (this session's bar, not a ruled one).

Two tools were found wanting by this landing: `brit pull --ff-only` refuses a range that moves a
gitlink (`affected gitlink or unsupported mode: elohim/brit`), so the main checkout was
fast-forwarded with git; and the campaign's timing script misreads a log that crosses midnight
or carries a collected gate's replayed output, so the figures above are the hook's own
`PASSED (N s)` lines.

## Row 65, delta 3 (2026-11-22): the fixture change landed, and four threads held the bar

The `session_envelope` fixture change is on dev (landing 13, `e90a3178e`): one stripped `epr`
and one launcher per test process, linked into each fixture. That binary now writes 3.6 to
3.8 GB where it wrote 5.3 to 5.45 GB.

The bar the plan set for a thread count was ten clean runs beside a running build. Measured on
that head (records under `bar-t4/` in the sprint's campaign directory): ten runs of the gate's
own test command at four threads, each beside another build on the second cargo seat (the brit
0.3.1 install, then storage gates).

| | |
|---|---|
| Clean runs | 10 of 10 (exit 0, no failed test) |
| Seconds per run | 577 to 861 (one thread: 1,653 to 1,729) |
| Guard pauses | none |
| Highest I/O pressure | 11.78% and 11.10% in two runs; 4.6 to 6.8% in the other eight |
| Peak write rate over 10 s | 53 to 72 MB/s, `session_envelope` in every run |

The margin is thin and is stated as such: pressure passed the guard's 10% line twice without
lasting long enough for it to act, and the 1 Hz sampler cannot show how long it stayed there.
What remains of the burst is the product's synced ingest of a 66 MB file per fixture, four
fixtures at a time. If pauses return, the lever that needs no product change is fewer parallel
Python workers in that one binary.

A by-product: the companion storage gates, run beside those tests on a warm target, took 1,443
to 1,711 s (four runs), against 1,384 s when carried in landing 13.

The thread count is declared on the eprfs gate project's own `run.cargo.env` (commit
`ae99ed59a`, handed over on `self/shem-eprfs-threads4` for the sibling device to gate before it
lands). The pool policy's override for eprfs still says one thread and is now shadowed by the
manifest; removing the shadowed value is owed so the device has one answer.

## Row 42, delta 6 (2026-11-22): the shadow's first lines against a brit that has the flag

Neither device could install brit 0.3.1 yet (row 71), so both pointed `BRIT_BIN` at the candidate
each had built from the pinned source (`02d4654f`) for a read-only run; nothing was installed
and the shadow decides nothing. The probe reports the flag present and prints no line; with
brit 0.3.0 it prints the one unavailable line. Feeding the selector one changed path:

| Changed path | Gates selected today | Of those, gates the sources class would not run |
|---|---|---|
| `elohim/eprfs/Jenkinsfile` | cargo-bridge, jenkins-bridge, eprfs, eprfs-live-corpus, orchestrator, orchestrator-tree | eprfs, eprfs-live-corpus |
| `elohim/holochain/Jenkinsfile` | doorway, elohim-sdk, orchestrator, orchestrator-tree, elohim-storage, genesis | doorway, elohim-sdk, elohim-storage, genesis |
| `pnpm-lock.yaml` | elohim-storybook, doorway-app, eprfs-ts, storage-client-ts, elohim-app | eprfs-ts, storage-client-ts |

The first two rows are this row's cost seen as the instrument reports it: on landing 13's
figures, about 1,770 s of gates for an eprfs pipeline edit and more than 1,380 s for an Edge
pipeline edit. The third row is the hazard of delta 5 shown by the same instrument: those two
gates' recipes read the lockfile, and their manifests still list it as `buildProcess`. Turning
the class on before that correction would skip them on a lockfile bump.

Not yet seen: a per-gate fingerprint line, which needs a push whose range moves a step's
fingerprint under one class and not the other; and any of this on an installed brit.

## Row 42, delta 7 (2026-11-22): the shadow spent its budget on an unchanged head, found on the second device

The sibling device ran both orchestrator gates in a fresh worktree at dev `9dbfbac2d` with a brit
that has the flag and saw one line: the shadow's 10 s budget was spent, on a head equal to its
base. Read in the code and measured with the same brit on shem: the budget was a deadline that
started at the first probe, so unrelated gate and test time used it up; the orchestrator suite's
own tests inherit `BRIT_BIN` and each ran the shadow's second planner call; and one gate step
with an empty `buildProcess` was fingerprinted three times at about 2.2 s each. Shem's own gates
could not see any of it, because shem's installed brit predates the flag.

Fixed before any device installed brit 0.3.1 (`1b0f5190a`): the shadow does no brit work unless
the range from the integration base to the gated commit changed a file some step lists as its
own `buildProcess`, which is the only way either of its lines can be non-empty. Equal base and
commit: nothing. No such file changed: two git reads. Otherwise three fingerprints per touched
step and one planner call per push, under a budget that now counts only the shadow's own time.

## Row 70 — installing a tool release beside another build ran into the installer's own time bound, reported as a failed build

Found 2026-11-22. `epr bin follow --tool brit` for brit 0.3.1 refused after 1,803 s with
`bootstrap-unavailable`, listing three failed builds (offline, anonymous, authenticated) and a
failed download of `brit-build-02d4654fa73d`. Read from the code
(`elohim/eprfs/device-runtime/src/bin_store/lifecycle.rs`): each build stage is given 1,800 s.
The offline stage failed at once because rakia 0.1.2 was not yet in the device's registry cache;
the anonymous stage then ran the one-job release build beside a four-thread test run on the
other cargo seat and was cut at its bound; a `cargo fetch --locked` by hand succeeds, so reach
to the registry was never the problem. The refusal says "failed" for a build that was still
making progress when its time ran out, and names no bound.

Owed: the refusal says which stage ran out of time and how long it was given; and a build cut
at its bound is resumable by a second `follow`, which the refusal should say. Whether 1,800 s is
the right bound for a one-job release build on a device that now runs two builds at once is the
same question as the storage gate's p90: measure it with both seats busy.

## Row 71 — the pin gate passed on the older of two CI runs on one commit while the newer was still running

Found 2026-11-22. brit `02d4654f` carries two runs of the same workflow: the release branch's
(suite 121667682136, `Tests pass` success at 23:23Z) and the push to `main`'s (suite
121684736606). Landing 13's `brit` gate read `Tests pass success` at 23:34:36Z. The `main` run
was still running then (`tests@windows-aarch64`, started 23:27Z) and its aggregate `Tests pass`
check did not exist yet, so the gate saw only the earlier green. That run ended red at 01:00Z:
the hosted runner for `tests@windows-aarch64` lost contact at 93 minutes, with no test verdict.

Effects seen: `epr bin follow --tool brit` on both devices built the candidate and then refused
it (`attestation-refused: attestation did not witness successful checks`), because the bin
store reads the newest check; brit's `publish-build` refused for the same reason. Dev's pin
stands on a green that was true when read and is not the newest reading. Cure in work: the
sibling re-runs the failed leg of the `main` run; a green there completes the 0.3.1 release the
steward ruled.

Owed in the pin gate (`gate-attest.mjs`): a newer run of the same workflow that is still in
progress is a `still-running` refusal, as a pending named check already is; today it is
invisible until its named check exists. The cost of that refusal is a wait of the run's
remaining minutes at pin-move time, against a pin that a later red contradicts.

## Row 42, delta 8 (2026-11-22): the shadow's cost stated truthfully

A review of the fix in delta 7 found its claim too strong: the shadow's git reads ran outside
its budget, so an untouched gate was not free and the stated worst case was wrong. Now
(`1ae2a5af2`) every shadow subprocess, git and brit, runs inside one budget with a kill signal.
An untouched gate costs one merge-base and one name-only diff (each under 10 ms measured on
shem) and no brit call; the bound is 10 s per gate process. The changed-path read no longer
follows renames and splits on NUL, so a renamed pipeline file is seen under its old name. Four
runner tests that took their brit from the caller's shell now name their own; the suites give
the same counts with no brit and with brit 0.3.1.

## Row 72 — brit 0.3.1 released its binaries and its tag, and its crates did not publish

Found 2026-11-22 when the re-run of brit's `main` CI on `02d4654f` passed its tests and then
failed at `Publish brit crates to Nexus`: `immutable version collision for brit-graph 0.2.3`.
The input-class change edited `brit-graph` (`src/fingerprint.rs`, `src/repo_fingerprint.rs`,
`tests/repo_fingerprint.rs`) and the release commit bumped only `brit-cli`. Every crate before
`brit-graph` in the publish order was skip-equivalent, so nothing new reached the registry: it
holds the earlier `brit-graph` 0.2.3 and no `brit-cli` 0.3.1. The GitHub release, the tag
`brit-v0.3.1`, the pin on dev and installs from source are unaffected; brit `main`'s run stays
red at its publish step until `brit-graph` is bumped.

Two readers missed it. The independent review of the release mechanics checked `brit-cli`'s
version, changelog and lock and did not ask which other published crates the diff touched.
The rule was already written down from 2026-11-16 (a change to a published brit crate's source
needs a version bump in the same commit) and was not turned into a check: the collision is
detectable before the merge by the publish script's own comparison, run without publishing.

Owed: a follow-up release (`brit-graph` 0.2.4, `brit-cli` requiring it, brit 0.3.2) and the pin
move after it; and, on brit's side, the publish comparison as a required check on the release
branch, so a collision is a red before `main` and not after.

## Row 73 — the first F4 probe: a hand-over in 56 s, and a red on the second device that ten runs here did not show

2026-11-22. The four-thread change (`ae99ed59a`, one manifest line) was handed over on
`self/shem-eprfs-threads4` with `brit push` (56 s; the hook ran its seconds checks and no
project gate, and recorded the hand-over). The sibling device ran `just gate eprfs` at that
commit in a fresh worktree as a genuine run.

Result: red in 548 s (a cold pool slot; four compiles of about two minutes each). One test of
703 failed: `flow::memory::recall::index::tests::a_v2_store_is_rebuilt_under_schema_v3`, where a
second fold in the test's own temporary directory reported `Busy`. Twenty reruns of the built
binary there were green; ten whole-workspace runs on shem had been green. The reading both
devices reached: the fold's lock is a `flock`, which belongs to the open file description; a
child forked by another test thread holds a copy of the descriptor until its `exec`, so the lock
can outlive the first fold's drop for a moment, and the non-blocking second attempt reports a
fold that is not running. At one thread no other thread forks.

What the probe showed about the ladder: the hand-over is cheap and works over the forge with
brit; a second device will pay a long gate on request; its verdict is recorded and would have
crossed. What it did not reach: reuse of a sibling green by the lander, because there was no
green. The four-thread change is not landed. Owed first: the fold tolerates a lock released
within milliseconds (in work), then four threads again on top, handed over a second time.

Also seen: the ten-run bar on one device was not enough to find a failure of this rate. A gate
change that alters concurrency wants its runs on two devices before it is called met.

## Row 73, delta (2026-11-22): the cure went through two forms, and the right one cannot all land without a roll

The first cure retried the fold's lock for 50 ms before reporting busy (`18b72ec48`, handed over
a second time as `self/shem-eprfs-threads4-b`). A review (Codex astra,
`astra-fold-lock-review.receipt.md` in the sprint's campaign directory) confirmed the mechanism
and rejected the cure: a slow or stopped child can outlast any patience, a fold that finishes
during the patience lets a contender in, and the new test raced a helper thread. It named the
right one, already used by the bin store and the session envelope's lifecycle guards: an
explicit `LOCK_UN` releases the shared description's lock at once, whatever copies remain. That
commit will not land.

The second form is an owner guard that unlocks on drop, with tests that take the lock,
duplicate the descriptor, release, and retake while the copy is still open: red on dev's code,
green after, with no sleep and no time ceiling. Built complete (`8b6168dea`, local branch
`wallclock/fold-unlock`: one shared guard in `device-runtime`, the session envelope's holder
and probe, the fold, the language-server lock), it is refused by the fleet-roll check, because
CI reads `device-runtime` as an Edge build input.

So it is split. Landing here (`cd17dec25`): the fold, through a private copy of the guard in
its own crate, and the language-server lock. Parked for the next deliberate Edge roll: the
shared guard and the envelope sites, after which the private copy is replaced by the shared
one. Measured on shem at four threads on the split: the two changed crates' unit tests 20 of 20
each, the whole workspace green in 662 s, and the two `device-runtime` test binaries that stay
unfixed 40 of 40 each. That last figure bounds the residual on one device and does not remove
it.

Four threads is on this branch behind the fix and lands only after the sibling device's gate at
the handed-over commit is green.

## Row 74 — six lock sites answer "held" at once, and a forked child of another thread can make that answer wrong

Found 2026-11-22 by the first F4 probe (row 73) and confirmed two ways on shem: a `flock` taken
through a duplicated descriptor stays held after the original is dropped, and a child forked
with a delayed `exec` held a finished fold's lock until its `exec`. A `flock` belongs to the
open file description; a forked child has a copy until `exec`, close-on-exec or not.

A first cure for the fold retried the lock for 50 ms before reporting busy. A review found the
better one, already used by the bin store and the session envelope: an explicit `LOCK_UN`
releases the shared description's lock at once, whatever copies remain, where a release by
`close` waits for the last copy. The fold now unlocks on drop. The same shape remains at six sites in the eprfs workspace, each of which
takes a non-blocking lock once and reports the answer as final:

| Site | What it reports when the lock is held |
|---|---|
| `epr-lsp/src/socket.rs` `ServeLock::try_acquire` | another server is serving |
| `epr-cli/src/flow/memory/recall/mod.rs` | another packet is executing |
| `epr-cli/src/device/arrival_work.rs` | arrival work is in progress |
| `epr-cli/src/session_envelope.rs` `Store::try_lock` | the store is in use |
| `device-runtime/src/envelope.rs` (a momentary probe) | a holder is live |
| `epr-cli/src/device/arrival.rs` (a momentary probe) | arrival is live |

Five other sites already retry to a deadline, each with its own numbers (5 x 20 ms, 20 x 25 ms,
and three more), and three block. So the workspace has eleven hand-written answers to one
question. Owed: one lock guard that unlocks explicitly on drop, used by all of them, with a patience only
where real contention is expected; until then, any of the six can turn a four-thread test run red at a low rate, or skip
work in a multi-threaded process that spawns. `device-runtime` and `epr-lsp` are not published
crates either; the change needs no version bump but does need the eprfs gate.

## Row 42, delta 9 (2026-11-22): the shadow on an installed brit, and one cold answer

brit 0.3.1 is installed on both devices (the sibling in 2.1 s, shem in seconds once a clean
checkout sat on a commit published on dev; each reused the binary it had built). On shem's
installed brit the shadow prints its selection line for a pipeline-file change as it did
against the uninstalled binary. One observation for the list of what `on` needs: the first
capability probe after this session switched binaries answered "unavailable" once, and every
later probe answered correctly (0.05 s warm). The likely cause, not confirmed, is a first launch
after a switch that outlasts the probe's two-second bound. In shadow it is one missing line. For
`on` it is another reason a capability that changes within a push must refuse, not downgrade.

## Row 75 — what is ready, or owed, and waits for a deliberate Edge roll

Kept in one place because the queue grew through 2026-11-21 and 22 and each item was parked by
the same seconds check, run by hand before any gate: CI would select an Edge build and its
deploy step for the change, and a second fleet roll is the steward's to time.

| Item | State | Where |
|---|---|---|
| Six Jenkinsfile guards (a build refuses before it requests a pod) | written, parked | row 63 |
| Two gate-only manifests list a lockfile as `buildProcess` | written, parked: commit `324dcb4e6` on local branch `wallclock/process-inputs`, plus deleting `PARKED` in `pipeline-registry.test.mjs` | row 42, delta 5 |
| One shared lock guard in `device-runtime`, and the session envelope's two sites | written, parked: commit `8b6168dea` on local branch `wallclock/fold-unlock`; then the fold's private copy is replaced | row 73, delta |
| The installer says which build attempt ran out of time, and that a retry resumes | owed | row 70 |
| The remaining lock sites take the shared guard | owed | row 74 |

The input-class shadow cannot be turned on before the second item lands. The local branches
named here exist on shem only; they are pushed as hand-over branches when a roll is planned.

## Row 73, delta 2 (2026-11-22): the third hand-over was green on the second device, and four threads landed

The split of the delta above was handed over as `self/shem-eprfs-threads4-c` (`005649870`: the
fold and the language-server lock unlock on drop, then four threads on the eprfs device gate).
An independent review of the lock commit said ship.

On the sibling device, a genuine `just gate eprfs` at that commit in a fresh worktree with a cold
pool slot: green, 802 s by the gate's own line (151 test binaries, 2,701 tests, none failed), no
guard pause. Then thirty runs each at four threads of the three fast binaries nearest the lock
sites:

| Binary | Runs green | Tests per run | Seconds for the thirty |
|---|---|---|---|
| `device-runtime` lib | 30 of 30 | 238 | 459 |
| `device-runtime` `tests/envelope.rs` | 30 of 30 | 20 | 60 |
| `epr-cli` lib | 30 of 30 | 705 (1 ignored) | 109 |

The two `device-runtime` binaries hold the sites this commit leaves unfixed (row 74). Ninety
clean runs on the device that showed the first failure bound the residual there; they do not
remove it, and the complete fix still waits for a roll (row 75).

The condition set before the run was a green gate and clean `device-runtime` counts on the second
device. It was met at 13:08Z and the push of landing 15 started at 13:09:24Z. It was on dev at
13:32:25Z (`005649870`): twelve gates, 1,289 s in the hook.

The lander's log reads `[gate] eprfs reused: green in 802 s on the sibling at 005649870 (signer
sibling did:key:z6MkpZRh…VMZB, independence: sibling)` and `eprfs: PASSED (32s)`. That is the
first landing here whose long gate another device paid for at the lander's own commit.

What the second rung of the probe showed about the ladder (row 9), by what it cost:

| Step | What happened | Seconds |
|---|---|---|
| Make the commit reachable | `brit push` to a `self/` branch: seconds checks, no project gate | 53 |
| Say so | three `berth say` notes | under 5 to send; 7 h 38 min to arrive (row 76) |
| A device admits the gate | its own answer, from its own seat and guard lines; nobody was assigned | 0 |
| The gate runs there | fresh worktree, cold slot | 802 |
| The verdict crosses | `gate-verdict@1`, acknowledged by the lander's device | not timed; heads were folding within 22 s |
| The lander reads it | at its own gate's start: a miss in its own store, then the sibling's green for the same closure | 32 |

The cost that mattered was the second row. An ask that travels as notes has no bound and, until
this landing, no word to its sender when the exchange under it has stopped. The next rung of the
ladder (the hand-over as a landing intent both devices read, raised before the gates, carrying
head, closure keys and misses) puts the ask where a device can see it without a session reading
its inbox; it should carry a bound when it does.

Also read from that push: the longest gate was not eprfs. `memory-ceremony` took 588 s of the
1,289 (row 60, delta 2).

## Row 60, delta 2 (2026-11-22): the second gate that runs these tests takes the same caps

`memory-ceremony` runs fourteen integration-test binaries of the eprfs workspace (a subset of
what the eprfs gate runs), then a Python hook suite, a lens audit and two cucumber profiles. It
took its caps from `pool-policy.json`: one build job, one test thread. In landing 15 it was the
longest gate, 588 s, on a push whose eprfs gate cost 32 s:

| Leg | Seconds at one thread (landing 15) | Seconds at four (one run on shem, warm build) |
|---|---|---|
| build of `epr` and `bin-launch`, then the test binaries | 120 | 2 (already built) |
| two `cargo test` legs, fourteen binaries | 243 | 81 |
| Python hook suite, 210 tests | 146 | 147 |
| lens audit and two cucumber profiles | 66 | 58 |
| whole gate | 588 | 323 |

The build legs are not comparable: landing 15 rebuilt `epr` after a source change and the second
run found it built. What the caps changed is the test legs, 243 s to 81 s.

Changed in `5f74946ad`: the gate declares `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=4`
on its own manifest project, as the eprfs gate does, and `pool-policy.json` loses both
per-project overrides (eprfs's was shadowed by its manifest since `e432b3662`). A cap now has one
home, the gate project it caps. The evidence for four threads on these binaries is the eprfs
gate's (row 73, delta 2): they ran at four threads inside the whole-workspace runs on both
devices after the fold's lock fix.

What is left in this gate is not Rust. The Python hook suite runs its 210 tests one after
another, and the gate records no verdict that could be reused (it has gate-only inputs and no
step fingerprint), so every landing that touches the memory flow pays it whole.

## Row 71, delta (2026-11-22): the pin gate waits for a newer run, and says what it does not read

Changed in `54e9dbe93`, `56a5786d3` and `18f8c5514` (written by a second model
from a brief, read twice by an independent reviewer, fixed after each reading).

When the newest named check on the pinned commit is complete, the gate now makes a second read,
the commit's workflow runs, and refuses as pending when a newer reading is on its way: another
run of the same workflow that started later and has not completed, or the judged run itself
being re-run. The refusal names the run. On 2026-11-21 that would have refused the brit pin
until the push-to-main run ended, and that run ended red.

What it will not do:

- **Refuse on uncertainty.** A failed, timed-out or malformed second read, a run it cannot place,
  a status word it does not know: the first read's verdict stands and the printed line says newer
  runs were not checked and why. This is the gate's standing rule for an unreachable read.
- **Outlast its caller.** The bin store calls the same judge with a 75 s bound, so the two reads
  share one 60 s budget and the second gets at most 15 s of it.
- **Second-guess a completed run.** When the judged run is complete its conclusion is not read:
  the newest named check stays the newest reading even if it started before the run's last
  attempt. A re-run of only the failed jobs legitimately leaves an earlier green in place, and
  telling that from a full re-run cancelled before the check ran needs the run's job list. The
  reviewer wanted that closed; it is left as a stated limit with a test that pins it.

Not covered: a commit with more than a hundred workflow runs (the judged run may fall off the
page, which reads as "not checked"), and the bin store's own cache of a green it read earlier.

## Row 75, delta (2026-11-22): three of the five items are now written

Items 3, 4 and 5 of the table above are written and parked together on the local branch
`wallclock/fold-unlock`, head `25408ecd7` (dev `98d805381` plus four commits): the shared lock
guard in `device-runtime`, the session envelope's holder and probe, the fold, the recall
`Execution`, the arrival worker and the arrival probe on it, and the installer's refusal that
says which attempt ran out of its bound and names the command that resumes. An independent
review read it through `d68ff3e1d` and asked for two fixes; `25408ecd7` makes them and has not
itself been re-read. At roll time: rebase onto dev, replace the fold's private guard (landed in
`5a3dab35e`) with the shared one, re-read the last commit, then land with the roll.

## Row 76 — an exchange stalled both ways for more than nine hours, and the sender read it as an idle session

2026-11-22. The steward channel between shem and the sibling device stopped carrying records in
both directions at about 03:08Z (the sibling's reading) and resumed when the steward restarted the
peer daemon on the sibling at about 12:51Z. Nothing on either device said so while it lasted.

What sat in it, from berth's own ledgers: eight notes from shem (03:30Z to 05:05Z), among them
the second and third hand-overs of the four-thread change and the ask to gate them; two notes
from the sibling (04:33Z and 06:04Z), each asking which commit to gate. Every `berth say` printed
`note #N at <time> to <session>` and exited 0. Shem read the missing replies as a session with
no tool call to deliver its notes at, said so to the steward, and waited. Landing 15 was reviewed
and ready at 05:07Z; the sibling's gate at its commit started at 12:45Z, on a relayed message
from the steward. The wait was 7 h 38 min, and the cause was read wrongly for all of it.

What would have shown it, and was not read: `berth status` marks a device SILENT when its last
head is older than `freshness_s` (90 s), and `just status device` carries a `sibling exchange`
line. Neither is printed to a session that is only sending notes and waiting for an answer.

Changed in this landing, on both sides of a wait:

- The sender (`77494ea95`): `berth say --to <session>` prints a second line when the addressee
  is moored on a device past that threshold, read from the view the note's own write already
  fetches. The note is still recorded and still crosses when the exchange resumes.
- The waiter (`dce9d4e5e`): `berth inbox --follow` reads the lease view once a minute and prints
  `[berth] device <did> is SILENT: ...` when a device with a moored session stops exchanging,
  and `... is exchanging again` when it resumes. A session that is only waiting hears of a
  stalled exchange within about two and a half minutes.

Not known, and owed:

| Owed | Why |
|---|---|
| Why the exchange stalled | The sibling's daemon process was alive, and restarting it cured the stall. No cause was found. |
| A stall in one direction | Both lines read when a sibling's head was last folded here. Whether the sibling has folded this device's head is in `epr peer status` (`acks ours to seq N`) and is not read by berth. |
| One note that crossed and was not delivered | Shem's 03:03Z note was in the sibling's inbox, and neither its note hook nor two thirty-minute `--reader main --follow` windows printed it; the sibling found it with `berth inbox --all --peek` at 04:33Z. Reported by the sibling; not reproduced. |
| An ask over the channel has no bound | The steering plan's second test says a claim is bounded and expires. A note that asks for a gate carries no time after which the asker stops waiting and reads the exchange. Until hand-overs are landing intents (row 9, step 2), the bound is the asker's own habit. |

## Row 77 — a roster row published on dev admits nobody until each daemon's own checkout carries it

2026-11-22, handed over by the steward's laptop session through the sibling device, in the
story-graph shape it arrived in.

Chain: device arrival, to sibling exchange. Between "the roster row is published on origin/dev"
and "a sibling admits the device". Missing node: "the admitting daemon's roster source carries
the row". Probe: on each sibling, `epr peer status` names the new device with a folded head, and
no checkout was switched by hand to get there.

Current state. The peer daemon reads admission from the roster file in its own `--root` working
tree (`epr-cli/src/actor.rs` `roster_file`: `<root>/.eprfs/status/participants/<handle>.jsonl`;
`epr-cli/src/peer.rs` "read from the roster in this checkout"). Read in the code on shem. So a
row reaches a daemon only when that checkout's branch carries it:

| Device | Daemon root | Carries the laptop's row |
|---|---|---|
| shem | the main checkout, on dev | after the row lands on dev and the checkout is fast-forwarded |
| sibling (ethosengine) | its main checkout, on a memory-ceremony branch (as it reports) | no; the branch has four rows and not this one |
| laptop | its own checkout, with the rows committed locally (as it reports) | yes |

Not verified: whether a running daemon re-reads the roster or reads it once at start.

Cure candidates, as handed over: read the roster from the published ref (origin/dev, or a
declared roster ref) and not from whatever branch the root happens to be on; or have
`just status device` and `epr peer status` say which branch the roster source is on and which
device it lacks that origin/dev has. The second is the cheaper first step and makes the gap
visible on the device that has it.

## Row 77, delta (2026-11-22): what the gap cost the laptop's first landing, and what admission will cost the others

Relayed by the sibling from the laptop's session, about its first push to dev (eight commits,
gates all clear in 71 s). At the last step the hook's claim of `dev-integration` was refused,
`unconfirmed: did:key:z6Mkm5de…bowj`: the laptop had folded no head of shem, because no daemon
that could send it one has the laptop on its roster. The landing agent wrote its one-device
override (`--offline-ok`) and waited for it to take effect, 649 s. During that wait the forge
closed the push's SSH connection; that host drops an idle connection at about seven minutes. Its
session then pushed the same gated commits again without the hook.

Two things follow, and the second is not about the laptop:

| Finding | Why it matters |
|---|---|
| The override's wait (lease ttl plus skew plus margin, up to 660 s) is longer than an idle SSH connection lives on that host | On such a host the wait cannot succeed inside one push: the gates are paid and the transfer is lost. Every landing from it will hit this while any device it lists is silent. |
| An admitted device that is silent costs every other device's landing the same wait | Row 61 met it once, 561 s, when the sibling's sessions had closed for the night. A laptop is silent whenever its lid is closed. Once its row is in the other daemons' roster sources, a landing on shem or the sibling waits about 650 s at the lease step for as long as the laptop sleeps. |

The steering plan's second test is that a claim is bounded and expires and the requester can
always run alone. Here the bound exists and is ten minutes, paid after the gates, to guard a
window the forge already guards (a push to dev is a fast-forward or it is refused). What the wait
should be for a device that has gone quiet, and whether a device nobody has ever heard from
counts at all, is a design question for the steward (the settlement fence of
`2026-10-27-steward-space-landing-plane-design.md`). Not changed here. Shem's daemon reads its
roster from its main checkout on dev, so the laptop's row reaches it with the next fast-forward
of that checkout.

## Row 78 — a stop test read a killed child as surviving, once, on the second device

2026-11-22. The sibling ran the orchestrator gates at landing 16's code in a fresh worktree as a
second-device check. `orchestrator` passed in 65 s. `orchestrator-tree` failed after 371 s on one
test of `genesis/orchestrator/compute-worker-supervisor.test.mjs`, "rendered supervisor retries
failures, releases the lock and cleans children on SIGINT": `surviving child <pid>`. Its SIGTERM
twin passed beside it, neither file is in the landing, and the same gate had passed there twice
at landing 13's commit. Ten reruns of the unchanged file alone on that device were ten of ten green: it showed once in eleven runs, and only under the whole tree suite's load.

Read in the code: the rendered supervisor (`scripts/render-compute-worker.mjs`) ends an attempt
with a SIGKILL to the attempt's process group and then waits only for the entrypoint it started.
A descendant that ignored TERM and INT dies from that signal a moment later, and nothing in the
supervisor waits to see it gone. The test read `/proc/<pid>/stat` the instant the supervisor
exited.

Changed (`77e9a64fd`): the test gives each pid up to one second to be gone. Its claim is
unchanged: no child survives a stop.

Owed, for the supervisor's owner: the supervisor confirms its group is empty (bounded) before it
retries or exits. The script's own comment says descendants may keep the worker's lock; a retry
that starts while a killed descendant still holds it would meet a held lock and count as a
failed attempt. No committed manifest enables the worker today, so the script is rendered into
nothing that is deployed.

## Row 76, delta (2026-11-22): the note that crossed and was not delivered happened again, and why

One owed item of row 76 was a note that crossed and was not delivered, reported by the sibling
and not reproduced. It was reproduced on shem the same day. A relay note folded at 13:38:50Z ("it holds the landing lane
now … please hold") never reached the session it was addressed to: `berth inbox --reader main
--peek` showed nothing unread for the hook's reader, and the session's transcript holds no hook
record carrying it. The session learned of it fifteen minutes later by reading the whole inbox,
after it had sent a note that contradicted it.

On shem a four-thread cargo test run was loading the device at that minute (the memory-ceremony
measurement, 13:36Z to 13:41Z). What the sibling was running when its note was dropped at about
03:03Z is not recorded here.

Read in the hook (`.claude/hooks/berth-note-inject.py`): it claims the notes, writes them to its
standard output, and acknowledges the claim at once. Writing is not delivery. The harness stops a
hook at three seconds and drops what it wrote, and the hook's own budget is two seconds after the
interpreter has started, which a loaded device can stretch past three. Whether that is what
dropped these notes is not proven. What is proven is that the hook's promise, "a timeout or a failure on
the way loses nothing", did not hold.

Changed (`813a72bd7`, `b543db5ce`, `33d1cd198`, `954afb0bd`): the hook prints a short receipt with the notes and acknowledges the claim
at its next call, only when the session's transcript shows that receipt (a subagent's receipt is
read from its own transcript). If the receipt is not there after a grace of 20 s, the claim is left
to expire and the notes are shown again, marked as a repeat; each further miss doubles the grace up
to fifteen minutes, and no count ever causes an acknowledge. Delivery with a transcript is now at
least once: a note can be shown twice and is not lost by the hook. With no transcript to read, it
acknowledges on the write as before and can still lose one, and its header now says so. Written
by a second model from a brief and read twice by an independent reviewer.

Still owed from row 76: why the exchange stalled; a stall in one direction; a bound on an ask.

## Row 77, delta 2 (2026-11-22): the row reached shem's daemon, and the device status still reads "arrived and exchanging"

After landing 16 shem's main checkout was fast-forwarded past the laptop's roster row. The
running daemon re-read its roster without a restart: `epr peer status` listed
`did:key:z6MkgHju…xMbu  no head folded yet` at 14:29Z, and still did at 14:33Z. So the question
row 77 left open is answered on this device (a running daemon reads the roster again), and the
laptop is admitted here and has never been heard from.

`just status device` at the same minute: `ok sibling exchange` (it names the one device that is
exchanging) and `next step: nothing owed; this device is arrived and exchanging`. It says nothing
of an admitted device with no head, though every landing from this device now waits for that
device at the lease step (delta above). Owed: the status names each admitted device it has never
heard from, and says what that costs a landing.

## Rows 23, 30, 50 and 76, delta (2026-11-22): read against an outside pattern, they are one property

Read on 2026-11-22 by session 4ce727b3, at the steward's request, against the article "Building
always-on agents" (claude.com, 2026-11-13) and the practitioners' thread under it. The article
puts five things around an ordinary agent loop: an event inbox, timers, memory, permissions and an
outbox. Its first rule is that no source calls the agent directly. Every source writes to an inbox
that outlives the worker, and a worker that dies loses nothing because the inbox is what is true.

These four rows are that rule's absence, met four ways. Each addressed something to a session: a
note that waited for a look (23), a note that did not cross and was not retried (30), a watch that
ended (50), an exchange that stalled under a session that was only waiting (76). What has landed
since makes delivery to a session honest: the note hook is at least once (row 76's delta), a `say`
names a silent device, a `--follow` says when one stops exchanging. A session is still the
addressee.

The record that is not addressed to a session is already designed. It is station F4-ask of the
native-dev-tooling plan: a landing intent that a device reads from its own fold whether or not a
session is at its inbox. That station is for the ask to carry a gate. The questions in row 76
needed the same shape ("which commit to gate" sat for hours as a note). Nothing here widens
F4-ask; the plan now carries this reading under it.

What the pattern has and this plane does not add, checked against what is already ruled:

| The pattern has | Why it is not added here |
|---|---|
| An inbox table per user | Berth's inbox is already a projection of notes on the one flow store, taken once per CID, with a cursor per reader and a claim that replays when it is not acknowledged (`genesis/agentic/bin/berth`). The habit `ledgers-fold-once` counts ledger files that keep their own cursor, not kinds of event, so an ask on the fold adds none |
| A timer store and a scheduler | F4-ask, step 2: a miss settles by fold and not by a clock. The time an asker stops waiting is its own. The landing-plane design rules "No agent wakes another agent" at S1, and the plan's vocabulary introduces no queue or scheduler |
| One active worker per inbox | The `carry` claim in F4-ask's next slice; for a landing, one lander per head (native-throughput ruling R6) |
| A wake that asks whether the lane can afford to run | The steering plan's test 3: capacity is sensed by the carrier, from its own lines |

Still owed, gathered here and unchanged in substance. Row 30: a note that did not cross is queued
and not dropped. Rows 23 and 50(b): a sender can read whether its note was shown. Row 76: why the
exchange stalled, a stall in one direction, and a bound on an ask. The last is the missing node the
plan now names under F4-ask: the asker's own session hears when the time on its ask has passed.

## Row 34, delta (2026-11-22): the held call, read against the same pattern

The article's answer to an approval nobody is present to give is to end the run. The session is
saved in the middle of the call, the request is shown to the person, and the answer comes back as
an inbox event that starts a new run on the saved session. The thread adds three things from
practice. Tie the approval to the exact payload. If nobody approves before a deadline, it does not
go. And, from someone running ten sessions for months: "a session waiting on an approval prompt
leaves no trace, so from outside it looks exactly like 'working'", which is this row.

What is ours already. The dialogue-stasis design parks the call inside the live session with a
fixed denial, lets independent authorized work continue, and does not approve for the person. A
parked item's expiry "leaves the Refer unanswered, never a Refuse", and running the step later
needs fresh validation (`2026-10-30-dialogue-stasis-loop-design.md`, "Pivots"). So the payload
point holds by another route: no approval is carried forward to a later call.

Where ours differs, on purpose. The thread's deadline is a deadline for the person. Ours is for the
agent. The participation-read design's P11 reads silence from a person as the ordinary case and
not as a fault, so when the time passes the effect stays unrun and stays ready, and the agent goes
on to what it may do alone (P13, added with this delta). What this row owes is the agent's half and
is unchanged: "a time after which a held call in an unattended agent is refused rather than kept".

What the reading adds:

| Added | Why |
|---|---|
| The end-the-run form is not available here yet | It needs a session that can end and be resumed as itself. Backlog `session-envelope-resume-after-restart` (partial) records that a session resumed in a new process after a workspace restart cannot reopen its envelope, and every berth claim it makes is refused. The shift launcher resumes by exact saved session id and is dormant. Until one of those is live, parking in the running session is the form we have, and a held delegated agent is resumed by stop-and-message, as this row found |
| The trace should come from the held agent's own record | The same practitioner judges liveness "per session from that session's own outputs" and not from shared state. "The agent list says running" is shared state. The line this row asks for ("waiting on an approval since …") would be read from the held call itself |

Not checked for this delta: whether the stasis sensor's hooks run inside a delegated agent, which
is where this row was found.

## Row 58, delta (2026-11-22): two of our own rulings about silence are not reconciled

Found while reading the article's outbox section ("start quiet", tune message volume with evals)
against our own writing. Two rulings meet at this row and say different things.

The participation-read design, P11 (2026-11-22): silence from a person is the ordinary case and is
not read as a fault; attention is spent last; it names five notifications in about ten minutes for
one silence as the failure. The dialogue-stasis design (2026-10-30): the coordinator times an
unanswered dialog, Approach at three minutes and Breach at five, and sends one berth note and one
push-notification request per incident per stage.

They agree on more than they differ. The stasis timing changes who handles a stall, the ladder and
not the steward, and it does not change the person's standing; an unanswered ask expires as
unanswered. But a person can still be notified at each stage of each incident, and this row is
three incidents opened in one night's silence, with a second instance named by P11 the next day.

What our own writing already sorts, in four places, gathered as P12 of the participation-read
design with this delta: a red or a pain is printed on its own line and outside any rollup; an ask
for what is the steward's alone is prepared, gathered and held for a time he would welcome; work
that is ready stays ready; status is pulled. Quiet is the rule for the last three and not for the
first, which is the other failure this sprint family knows (a pipeline red for two days and nobody
told, one-baseline journal, 2026-11-19).

Owed, and the steward's to rule because it sets what may interrupt him: whether the notification
per stage stands once a person can declare reachability (the missing node of P3), and what it is
until then, while reachability reads `unknown`. This row's first owed item is unchanged: a state
for "the steward directed an unattended run".

On the article's "evals": the measures that would say how much reaches the steward are declared and
unread. `dialogue-blocked-seconds` is among the numb bounds on shem's session-start line
(2026-11-22), and the habit `dialogue-stalls-are-answered` is red. Both dated overruns were found
by the steward and not by a measure.

## Row 79 — nothing enforces a session's write set, and no row owned that

Found 2026-11-22 while reading the article's permissions section against our own writing. The
article says an unattended run "holds the user's access" and recommends a proxy outside the agent
that holds the credentials, so that what the agent could do is answerable from the proxy's
configuration.

Its premise is true of an interactive session here, in our own words. A runtime and its children
"are one principal on this device", and the session guard "is an accident guard, not
authentication" (backlog `session-envelope-resume-after-restart`). A covenant clause is signed
with the repository's brit key, "a recorded promise, not a proof" until the human's own key
co-signs (habit `rule-variance-under-covenant`, guard). And the wall-clock journal wrote: "Nothing
enforces the write set, so it is said here and in the commits". Bounded, expiring grants are real
for the delegated agents (the CI runner, the compute worker) and not for the session that holds
the shell.

What is already ruled, so that the cure is not read as "add a gate":

| Ruling | Where |
|---|---|
| Within one principal a gate can already be bypassed; it carries weight once a second principal is admitted | landing-plane design, the S2 gaps |
| The operator's loop is not gated; a covenant-altitude promotion may be | verification guidestar, Law III |
| The harness witnesses an agent's writes; an agent narrating its own bookkeeping is a harness gap | `.claude/memory/feedback_harness_witnesses_agents_dont_narrate.md` |
| A participant signs its own events with its own key and refuses outside its bounds; the substrate checks `bounded_by` | `genesis/docs/architecture/rea-compute-commitment-primitive.md` |

So the keyless agent behind a proxy is not our general model, and effects are already mediated
where a place exists: the shift guardian owns leases, commit, landing and push, and the shift
launcher is dormant. What is owed is narrower than enforcement. A write outside an honoured
covenant's write set is witnessed by the harness and shown, to the session and beside the write
sets `berth status` already prints, with no agent having to say so. Whether it should refuse is a
question for the arrival of a second principal.

Not read for this row: whether any hook already compares a write's path with the write set. A
search of `.claude/hooks` for the write set found it in the covenant ask
(`participant-standing.py`) and in tests, and in `genesis/agentic/bin/berth`.

## Row 80 — a teardown race in the gate-tap tests went red once in twelve, and the red replaced a green note for the same closure

Found 2026-11-22 by ethosengine's second-device pass of landing `8ffc0921c..c71f584f8`
(session 6d68315a, fresh worktree, one gate at a time). Its first pass was green on seven gates.
A rerun of `just gate orchestrator` through ark with `GATE_SOURCE_VIEW=shadow GATE_EVIDENCE=off`
failed in 70 s on one test in `genesis/orchestrator/gate-tap.test.mjs`, with `ENOTEMPTY` in the
test's teardown. The same file run alone ten times at that commit was green ten times (23 tests),
so the failure appeared once in twelve runs and only under the named suite's load. The landing
did not touch that file.

Two concerns, kept apart:

1. **The race.** A teardown removes a directory something is still writing into. Not
   reproduced on shem.
2. **The record.** A build note is one per step and commit and is overwritten, so the red run
   replaced ethosengine's green note for the orchestrator gate at closure `a8d119421a34`, although
   a green for the same closure had been recorded minutes earlier. A flaky red therefore turns a
   closure that was paid into one the next push pays again, and a sibling asking for that verdict
   is refused by it.

State: neither is cured. ethosengine restored its record the same day with one plain
`just gate orchestrator` at `c71f584f` (no shadow, default evidence): exit 0 in 159 s,
orchestrator 66 s and orchestrator-tree 80 s, and the gate printed that the note for closure
`a8d119421a34` held a red and was being replaced. So the record is green again by a rerun, which
is the cost concern 2 names: the closure was paid twice on that device.

## Row 75, delta 2 (2026-11-22): two items from the standing design wait for a deliberate Edge roll

Read on dev 8293f3e6b with `brit build affected --files <path>`. Two pieces of
[the standing read for counting](epr:standing-read-for-counting-principals-design) §5 select the
Edge build and its deploy step, so neither lands without a roll the steward chooses:

| Item | Path it changes | What the oracle selects | Ready when the roll comes |
|---|---|---|---|
| Counting principals where attestations are counted (its item 2; plan station F6-k1) | `elohim/elohim-storage/src/services/release_attestation.rs` (`tally`, `for_release`) | `elohim-edge:cargo-build-storage`, `build-edge-image`, `deploy-manifests` | Not written. Its tests are named in the station; `count` takes the builder (S9, round two) |
| A roster row binding a brit clone's signer to a device (its item 1) | `elohim/epr-rea/src/participants.rs`, a published crate | the same three Edge steps, plus `elohim-epr` and `elohim-eprfs` | Not written. Needs a version bump and a lockfile move with it |

Neither is started, by decision: this session's sprint stopped at anything that rolls the fleet.
The standing design itself has had two adversarial rounds and says which parts are sound to
build.

## Row 81 — a note between devices is cut at about a thousand characters, and neither side is told

Found 2026-11-23. ethosengine's session 6d68315a sent its second-device report for landing
`81ea6431b` as one `berth say` note with five lettered results. shem's session 0e9b66d9 received
it ending in `…` just after the third; the fourth and fifth were asked for again and arrived in a
second note. The sender's `berth say` printed its usual `note #<n>` line and nothing about length.
The same day the injected form of several notes ended `...(truncated)` in the reader's context,
which is the hook's own bound (`MAX_LINE` in `.claude/hooks/berth-note-inject.py`) and is shown;
the cut inside `berth inbox --all` is not explained anywhere this session read.

State: not cured, and where the cut happens is not established: at `say`, at the crossing, or at
the formatter. Owed: `berth say` refuses or warns past the length that survives, naming the
number, so a long report is sent as several notes on purpose.

## Row 77, delta 3 (2026-11-23): the device that has never acknowledged shem is named in four places, and is not cured

Since `c71f584f8`, `8293f3e6b` and `81ea6431b`: `berth status` prints `UNACKED` under it, `berth
say` to a session moored there says the note may not have arrived, a follow announces it once when
it begins and once when it ends, and `just status device` lists it as `degraded sibling exchange`
without changing the next step. `epr flow context --participate` prints it under Sources. Each
landing from shem still waits for it at the lease step: 17 s, 15 s and 15 s on the three landings
of 2026-11-22 and 23. Curing it needs that device's own daemon to carry shem's roster row and
exchange; nothing sent from shem reaches it.

## Row 75, delta 3 (2026-11-23): four of the five items are assembled for the roll

Assembled on `landing/f4-ask` as eight commits above dev `179814d6b`, read by subject from
`brit log`; none is on dev. Items 1, 2 and 3 of the table in row 75 (the Jenkinsfile guards, the
gate-only manifests with the `PARKED` allowance deleted, and the shared lock guard with the fold's
private copy replaced) are there: `7a543ee9`, `dd14d94b`, `0ba7b32a`, `aac55aa1`, `3f8cf20c`,
`b65a7ab3`, `416e19b9`. Item 4, the installer saying which build attempt ran out of time, is
written (`d749369c`). Item 5, row 74, is still owed. `epr-lsp` would need a new dependency and a
lock move, and `peer.rs`, `arrival_sysroot.rs` and `supervise.rs` need a patience parameter on the
shared guard, which is a design choice. The eprfs gate on shem was green on the assembled queue in
1,811 s, over its 1,200 s ceiling. The landing is the one `[edge:roll]` landing the steward
ruled for the F4-ask sprint; it had not been made when this was written.

## Row 82 — a coordinator's held tool call stalled two devices for seven hours and nothing said so

Found 2026-11-23, in the sprint that builds station F4-ask. The integrator session on shem stalled
from about 05:45Z to 13:14Z on one held tool call. Seven notes from the sibling session on
ethosengine arrived together at 13:14Z. The sibling had every hand-over green and ready and waited
about seven hours, because it had no bound on its wait.

What would have shown it: nothing. Neither device reports that a session's own tool call has been
pending for hours, and the sender's `berth say` line reads the same whether the addressee is
working or held. This is the subject of F4-ask itself, a request whose waiting side has a bound
of its own, met by the people building it; row 76 and the stalled-exchange lines of row 77 cover
the exchange under a note, and not the session above it.

State: not cured. Owed: a read that names a moored session whose tool call has been pending past
a stated bound, shown to the sender of a note, and a bound the waiting session writes down when it
starts to wait.

## Row 83 — a wait loop that greps for its own command line never ends

Found 2026-11-23. A delegated agent's wait loops of the form `until ! pgrep -f "<string>"; do
sleep …; done` matched their own command line, so `pgrep` always found a process and the loop
never exited.

State: not cured. Cure: wait on a PID (`kill -0 <pid>`) or on a done-file the job writes at its
end, as the sprint's other jobs did with `*.done` files.

## Row 82, delta (2026-11-23): two cures proposed for a held tool call, neither built

Raised in a side conversation between the steward and a fork of the integrator session on
2026-11-23. The steward's own words are not in this session's record, so what follows is the
fork's account of what was agreed as worth doing, not a ruling.

**A command shape an approval can read.** The call that was held was one string carrying a
leading comment line, a `cd`, a mutating git verb and a chained log read. The repository's own
rules push agents to that shape: the Brit-fallback reason must be the first line, a stale
`EPR_BIN` puts `env -u EPR_BIN` on every command, and a worktree may not be entered so every
call carries a path. Proposed rules, to be enforced where a command is written (an `.epr-meta`
rule with a PreToolUse check) and not only documented: one operation per call; a git mutation
outside brit is a checked-in script with fixed arguments (tonight's three shapes were pick onto
a branch, re-run the trailer hook over local commits, and an empty tagged tip); no leading
comment line and no heredoc carrying code; no directory change; a wait is on a PID or a
done-file with a timeout (row 83); no prefix the session start should have supplied
(station Terminal-env, on this landing).

**Every held prompt is an incident.** The measure exists and nothing feeds it:
`dialogue-blocked-seconds` is among the bounds the session-start `sense:` line lists as numb.
The stasis loop runs in shadow and speaks through the agent that is stuck; on 2026-11-23 it
asked that agent to send a push at about 05:33Z and the agent judged the request noise. A
detector that depends on the judgment of the thing that is held cannot be the alarm. Proposed:
a watcher outside the session sees a tool call pending past a short bound; it pushes to the
steward, writes a berth line the sibling devices read, and `berth status` stops reading that
session as live (row 34); the incident is an observation on `dialogue-blocked-seconds` with the
command, the rule that held it and who was waiting; each incident closes in a style rule or
named script that makes the shape unwritable, or in a narrowed ask rule. A habit would state
it ("no agent waits on an approval unseen"), born red.

Missing node, in mintable shape. Chain: an ask that waits (station F4-ask). Between "a
session's own tool call is pending on an approval" and "the steward and the sibling devices
know": a read that does not pass through the held session. Probe: with one session's call
held for 120 s, `berth status` on a sibling device marks that session held and a push has
reached the steward. Current state: nothing; the held session reads `live`.

Neither cure is a prerequisite of this landing. Both are prerequisites of a landing with no
coordinator session, which is the test the two F4-ask records exist to pass.

## Row 36, delta (2026-11-23): the deadline arrived during a landing push, and the grant was renewed

The grant expired at 2026-11-23T16:00:00Z, fifteen minutes into the F4-ask landing's pre-push.
`orchestrator-tree` had been green on both devices for that head earlier the same afternoon and
went red at 16:27Z on `compute-worker-clock.test.mjs`. The push was stopped; the greens it had
recorded stand. The steward ruled a replacement of 30 days with `rotation_ttl_days` raised from 7
to 30 (`genesis/agentic/compute/operations.md` records the answers). Still owed from this row:
the warning some days ahead. The test prints a diagnostic under 48 hours, which nobody reads in a
green run; backlog `runner-grant-expiry-native-pre-expiry-sense` holds the native form. The next
expiry is 2026-12-23T16:00:00Z.

## Row 75, delta 4 (2026-11-23): the roll landed, and what is newly parked

The `[edge:roll]` landing is on dev: `179814d6b..ac4df62d7`, about 19:20Z, after a first push
stopped at 16:27Z (row 36, delta). The follow-up `ac4df62d7..b8f60c283` landed the lock fix and
the lander-wait fix. The result of the Edge roll was not known when this was written
(`elohim-edge/dev` #1922 was still building). Items 1 to 4 of row 75 rode the roll; item 5 (row 74)
is still owed. Newly parked for the next deliberate roll, as reported by ethosengine session
d6cfb7e1:

- the blob-store owner-lock fix `0f0432bd6` (branch `blob-owner-lock`; `src/store.rs` selects Edge:
  `REFUSED fleet-roll … elohim-edge stale steps cargo-build-storage, build-edge-image,
  deploy-manifests`); eprfs gate green there; see row 86;
- the native `epr session stasis observe` verb `8168a9383` on `stasis/tool-pending-observed`, not
  pushed, which touches `device-runtime`;
- the remaining raw lock sites: `peer.rs` `ServeLock` (daemon-lifetime, no `Drop`),
  `device/supervise.rs` `RestartTurn`, `epr-lsp` `socket.rs` (a private copy with no patience) and
  two in `elohim-storage`; its design moves them onto the shared guard with their own patience;
- shem's watcher branch `agentic/held-tool-watch` (`fa87fdf5b`), now a sensor that calls that verb
  and records nothing until it lands.

## Row 84 — a device's main checkout behind dev changes what every worktree push on it does

On ethosengine `core.hooksPath` names the main checkout's `.husky/_`. That checkout is on branch
`memory/ceremony-2026-11-20` at `6b10ee05f`, which predates `bd6a23772` (the `self/` hand-over
branch, `pre-push.bash:1638` on dev). So every worktree push there runs the old hook and selects
the full gate set against origin/dev: two `self/` hand-overs each printed 36 gates, about 126
minutes, before they were stopped (as reported by ethosengine). The cure per push is
`-c core.hooksPath=<worktree>/.husky/_`; with it the next hand-overs ran six `[pre-push]` lines and
no gates. The durable cure is to move that main checkout forward, which is the steward's call
because it holds a memory branch. The same class has cost hours before: row 82 above records shem's
main checkout 265 commits behind dev, and on 2026-11-19 it was 134 commits behind.

## Row 85 — a receiver's allow-list drops a crossing kind without a line, and the event is not refolded

The F4-ask Verify (journal `sprints/2026-11-23-f4-ask/journal.md`). (a) Ethosengine's peer daemon
ran rules `4c917dfa` from a private copy `genesis/local-dev/one-image/daemon/pool-policy.json`
dated 13 Nov, so its allow-list for `dev-landing@1` lacked `misses` and `carry`. Seven misses
crossed and were dropped, with no line on either device; `epr peer intents --misses` read "no open
miss" and `epr peer status` said the daemon CID differed from the CLI. (b) The receiver had the
full bytes when the allow-list skipped them (the check runs after receipt), and its fold had acked
past those sequence numbers, so refreshing the policy and restarting did not bring them back; shem
posted again. Ethosengine's design for the loss, a proposal and not built: hold the ack (rejected,
one stale kind would stop every later event from that device); refold from the sender on a policy
change; or, recommended, a bounded local quarantine of dropped events, capped by count and age
(for example 10,000 events or 7 days, oldest evicted with a counted eviction), re-run through the
allow-list at daemon start and whenever the rules digest changes. Its cost is disk and a boundary
question for the steward: holding events the policy did not admit, unfolded. Missing nodes, in
mintable shape: stations `Crossing-drop-seen` and `Daemon-policy-current` in the peers plan,
section 8.

## Row 86 — three flakes the load of a landing exposed

- `sysroot_unobservable_arrival_degrades_and_never_writes`: a lock released only by close outlives
  its holder while a forked child holds a copy. Ethosengine, whole `epr-cli` library at eight
  threads: 0 of 100 on bare dev `179814d6b`, 2 of 100 on the roll, 0 of 100 with the fix; shem's 30
  of 30 on the 69 `device::` tests had missed it. Fixed (`lock_sysroot` on the shared guard) in the
  follow-up.
- The blob-store `owner.lock` (`tests/multi_process.rs:537`, `OwnerHeld`): the same class.
  Reproduced 2 of 11 rounds under load before the fix and 30 of 30 green after (`0f0432bd6`). Fixed
  on its branch and parked for the next roll (row 75, delta 4).
- `discovery_uses_the_one_node_that_answers_and_never_guesses`: 1 of 100 whole-library runs at
  eight threads, then 0 of 300; the panic text was not kept, so the cause is a judgment. The
  diagnostic `fe619548f` landed so that the next red names its own cause.

## Row 87 — shem's peer daemon stopped and nothing on shem restarted it

On 2026-11-24 at about 00:45Z ethosengine's follow line printed that shem's sibling had gone
silent. `just status device` on shem read `no epr peer serve holds this device's serve lock`
and `device ark: stopped`: the device supervisor that keeps the peer daemon alive was not
running. `epr device supervise` restarted it (incarnation 16) and the sibling's head folded
within seconds. Why the supervisor stopped was not found. Ethosengine has a watchdog for a
silent exchange (`genesis/local-dev/peer/daemon-liveness.sh`); shem has none, so on shem a
stopped supervisor is noticed only when a session reads the follow line or a landing waits at
its lease step.

Next: the supervisor's own death should leave a witness naming its cause, and shem should have
the same watch ethosengine has, or the device supervisor should be started by the devfile and
restarted by it.

## Row 88 — the device-arrival hook had no time bound of its own

A session-start run on 2026-11-24 was killed by the harness at its 30 s limit (30016 ms) while a
landing's gates were running. Measured afterwards on shem at a load average of about 8:
`device-arrive.sh` 14.2 s, `load-project-context.py` 5.0 s, `ram-guard.py` 4.7 s, the rest under
1.2 s each; the hooks of the group run in parallel, so the slowest sets the wall clock. That the
arrival doubled under gate load is inferred, not reproduced. The hook now stops its own arrival
at 20 s and prints one line saying so. The 14 s floor is not explained: `epr device arrive` and
`epr session open` were not timed apart because both write state.

Next: time the two verbs apart on a quiet device and find what the 14 s is spent on.

## Row 89 — neither device's household is known to match the current DNA

Roll 2's serving receipt stopped at preflight on ethosengine on 2026-11-24: its household was
cast on 2026-10-26 and three installed integrity DNAs differ from the bundle built at
`8098b939f` (imagodei, infrastructure, lamad). Shem's household reproduces the bundle on its
disk, but that bundle was built on 2026-11-06 and preflight refuses it as stale, as it does the
mesh storage and doorway binaries (built 2026-11-15). Whether a bundle built from dev reproduces
shem's cells is not known until the DNAs are rebuilt. Recasting a household replaces its
identities and is the steward's decision. Until one household matches, a change to a storage or
doorway serving path can get its serving receipt on neither device. Roll 2 was split for this
reason: the half that touches no serving path goes ahead, the two storage lock sites wait.

## Row 90 — a sibling that is heard but never acknowledges blocks every landing, after a futile wait

On 2026-11-24 shem's landing passed its gates four times and was refused at the lease step each
time. The laptop (did:key:z6MkgHju…xMbu) was admitted and its head folded on shem about every
10 s, but it had acknowledged none of shem's records: its fetch of shem's channel root timed out
(498 timeouts in its serve log, relayed by its session). So `dev-integration` was `unconfirmed`,
and the hook waited for the `--offline-ok` override.

Three things were found, in the order they stopped the push:

- A renew that met `epr session: REFUSED lock-timeout: store lock remained busy for 5s` was read
  by the hook as a refusal. Cured: a bounded retry (`.husky/pre-push.bash`,
  `phase_l_agent_claim_patient`, with tests).
- The override never matures. It is effective `ttl + skew + margin` after this device last
  folded the silent device (`elohim/eprfs/…/lease_view.rs:781-791`, and the landing-plane spec of
  2026-10-27), and a device heard every 10 s moves that time 10 s later every 10 s. Berth also
  withdraws an override when the sibling's `last_seen` advances (`genesis/agentic/bin/berth`
  near :2181), and "is back" means seen again, not acknowledging again
  (`berth_test.py:3093-3110`). This is the rule as written, not a defect: the design treats
  silent as not heard, and never decided the case of heard but not acknowledging.
- Not cured, judged from reading and not run: the withdrawal's re-claim takes the envelope guard
  a second time inside the first, which may be what meets the 5 s lock timeout.

Unblocked for the night by the steward pausing the laptop's peer daemon, so that it is plainly
silent. His lean for the next sprint's design, chosen from three candidates on 2026-11-24 and not
a ruling: keep the rule and refuse at once, naming the cure ("seen but not acknowledging:
`<device>`; repair its fold of this device"), so the 11-minute wait is not paid for a refusal that
is already certain. The two other candidates each let both devices hold the lease in a one-way
partition: counting the wait from the first claim and withdrawing only on acknowledgement, and
maturing when this device's view shows no lease from the sibling.

Also open: why the laptop's fetch of shem's root times out (ethosengine's fetch of the same root
succeeds), and shem's peer daemon at about 75% of a core (row 87; the 30 s re-read of the flow
log has a fix handed over as `self/ethosengine-reindex-skip`).

## Row 89, delta (2026-11-24): shem's household matches current DNA; the preflight refusal was a false alarm

All five DNAs were rebuilt at dev `93983b057` and cargo found every wasm fresh; the integrity
wasms have not been rewritten since 2026-11-06. Preflight reads `ok hApp bundle identity` for
shem's household against that bundle. That fresh means the DNA hashes are unchanged is a
judgment from cargo's answer; the hashes were not compared directly. Preflight still prints
`coordinator wasm … is STALE` for each DNA, because its rule compares the wasm's build time with
the date of the last commit that touched the zome's directory, and that commit (`a8d2af4d1`)
added a licence field to each `Cargo.toml`, which changes no wasm. So shem can run the household
lane for a storage serving receipt once the mesh storage and doorway binaries are rebuilt; the
stale-hApp refusal has to be overridden by name (`MESH_ALLOW_STALE_HAPP=1`) with this reason, or
the rule taught to ask cargo. Ethosengine's household (cast 2026-10-26) does not match.

## Row 91 — a new device can never fold a sibling with a long channel over a relay, so it never acknowledges it

Why the laptop acknowledged none of shem's records (row 90), read in `eprfs-peer` at dev
`93983b057` by the laptop's session on 2026-11-24 and checked against what each device observed.
The integrator did not re-read the lines.

- The deadline covers the whole fold: `daemon.rs:1747` wraps connect, spine check, listing and
  every event read in `FOLD_DEADLINE`, 30 s (`daemon.rs:253`). A timeout prints
  `tier did not answer for root … (timed out)`, the line the laptop logged 189 times.
- A device that has never folded a sibling has no `since`, so `fold.rs` `fetch_channel_for` lists
  the whole `events/` directory in one answer (capped at 16 MB, not paged). With a `since` it asks
  for a diff, which is what ethosengine does for shem.
- Each new event is then its own sequential read on the connection, up to `MAX_FOLD_EVENTS`
  (1024). Over a relay that is about a thousand round trips inside 30 s.
- A fold that times out keeps nothing, so each retry starts from zero.

Observed: 189 attempts by the laptop against shem, none folded, across two `epr` builds; shem
folded the laptop's 31 events in one go; shem's head was near seq 90,000; shem holds only a
relay connection. Not measured: the round-trip time and the size of the listing. Shem's daemon
load (row 87) was suspected first and is not needed to explain it.

The cure in hand (ethosengine's lane, and the laptop's session reached the same shape): fold in
bounded batches and keep progress, so a deadline or the cap ends a batch with `complete=false`
and the next head continues from the last folded event. Reading several events at once is a
speed-up on top; a longer deadline alone moves the limit to longer channels. The fix helps only
once the reading device's daemon runs it.

Three gaps made this slow to find, each on both devices: no one-shot `epr peer fetch`, no
timestamps on the daemon's log lines, and no log line for an inbound tier request, served or
refused. Two traps the laptop met while rebuilding: a plain `cargo build --bin epr` omits the
`blob-store` feature and that binary refuses a blob-store-v1 payload store; and
`epr bin follow --repo` refuses on a host with no cargo pool.

## Row 92 — frictions met on the release, upgrade and comms paths, 23 to 24 November

The steward asked on 2026-11-24 that frictions on these paths be noted as they are met, so they
are sensed, planned and resolved in flight. Each line below was observed on shem or reported by
a sibling session that day; none is cured unless it says so.

Taking a release:
- brit 0.3.4's verified closure was refused by the store until the checkout's pin matched
  (`source-mismatch`), then refused as a head because it had been ingested from a dirty checkout
  (`dirty-build`). The path that worked rebuilt it from source, about 30 minutes, though verified
  binaries existed. A proposal for accepting a verified closure without a rebuild is in
  `genesis/docs/superpowers/specs/evidence/2026-11-24-gate-attestation/release-acceptance-design-note.md`.
- `epr bin follow --repo` refuses on a host with no cargo pool (the laptop). The manual path
  needs `--features blob-store`, and `epr bin land --commit` must repeat the ingest's sha string
  exactly: a short sha fails both ways.
- The security paydown check named `cargo update -p hickory-resolver --precise 0.26.2` for four
  alerts in `elohim/eprfs/Cargo.lock`. Cargo refuses it: `iroh =0.92` requires hickory 0.25. The
  alerts wait on the iroh pin; the check names a cure that cannot apply.

Upgrading in place:
- An epr head move restarts the peer daemon; a sibling in the middle of a fold loses its
  connection (two attempts kept nothing at 11:24Z). Nothing tells the sibling a restart is coming
  or over.
- A session keeps its pinned epr across a landing. A hook that needs a new verb drops its work
  (the first step readings, 15:59Z, and again for the device-level store at 20:07Z), and an older
  epr refuses a new event kind.
- A device reads its roster, its berth and its hooks from its main checkout. Ethosengine's sat on
  a branch from 20 November: it dropped the laptop as unadmitted, ran a berth about 1,270 lines
  behind dev, and had no note injector registered. The steward ruled on 2026-11-24 that the
  ceremony there be parked (`self/ethosengine-memory-ceremony-2026-11-20-parked`) and the
  checkout moved to dev; the move stopped at a local dev that cannot fast-forward, and is with
  the steward in ethosengine's session.

Comms while it happens:
- Berth notes cut at about 700 characters lost half of two instructions; a note that arrives
  between a session's reads is lost unless it re-reads its whole inbox.
- Cross-session messages report "delivery not confirmed".
- A sleeping laptop reads the same as a gone one; three session-owned watchers died without a
  word in one night. The inventory of what each liveness check can and cannot tell apart is
  `genesis/docs/superpowers/specs/evidence/2026-11-24-device-liveness/inventory.md`.
- Codex with a workspace-write sandbox cannot commit in a git worktree (the worktree's git
  metadata is outside its writable root).

Rollback was not exercised. `epr bin land --revert` exists; nothing here shows it across two
devices.

Also found: the device notes publisher's refspec is unfiltered, so harvested `ci:*` notes reach
the forge though the code that writes them says they stay on the device (shem has held
`just dev notes publish` since 17:50Z pending the steward's word); and a manifest with no
rustflags inherits ambient `RUSTFLAGS` outside the gate key.

## Row 93 — two daemons stay connected to the relay and exchange nothing, and nothing notices

Second occurrence. On 2026-11-24 from about 17:05Z to 02:12Z the next day, shem and the laptop
exchanged nothing in either direction for about nine hours while both peer daemons were up and
connected to the relay; neither log carried an error. Shem last folded the laptop's head at seq
21503; the laptop's newest head of shem stayed at seq 91662 while shem passed 92,800. Shem's
daemon was restarted twice in that time (its epr head moved) without effect, and ethosengine
exchanged with shem within seconds throughout. Restarting the laptop's daemon (same build, same
relay) healed it at once: within about 30 s it folded shem's seq 92883, and it folded
ethosengine for the first time. The first occurrence was 2026-11-05, about twelve hours, healed
by restarting one side. Reported by the laptop's session, which kept its pre-restart log.

What made it invisible: `connected` and `exchanging` are not told apart anywhere, and the SILENT
line reads the same for a sleeping device, a gone one and this (the liveness inventory,
`genesis/docs/superpowers/specs/evidence/2026-11-24-device-liveness/inventory.md`). Landings were
not blocked, because a silent sibling is covered by the lease override; a session that needed
the laptop's acknowledgement would have waited without knowing why.

Not known: the cause, and why only one side's restart heals it. Proposed by the laptop's
session, not built: a watch that fires when the newest head from a sibling is older than N
minutes while that sibling is seen exchanging with a third device. Ethosengine has such a watch
for its own exchange (`genesis/local-dev/peer/daemon-liveness.sh`); shem and the laptop do not.

## Row 94 — the Edge storage step watches all of `elohim/sdk/**`, so an agent hook test selects a fleet roll

Observed on shem, 2026-11-25. A landing that changed a hook, its package projection, two
documents and one test file, `elohim/sdk/domains/elohim-agent/scripts/brit-first.test.py`, was
refused by the preflight's `fleet-roll` check: "CI would select an Edge build and its deploy step
for this push to dev: elohim-edge stale steps cargo-build-storage, build-edge-image,
deploy-manifests". The cause is one line: `elohim/holochain/build-manifest.json` lists
`elohim/sdk/**` among the sources of the `cargo-build-storage` step. The same path is in the
`elohim-storage` gate's read set, so the push also ran the storage gate on the lander (about an
hour) for a closure that never landed, because a stop request does not interrupt a gate that has
started.

What was done: the test cases moved to `.claude/hooks/tests/test_brit_first_forms.py` and the
sdk file went back to dev's bytes. No `[edge:roll]` was added.

Not cured: the source list. Nothing has measured which paths under `elohim/sdk/` the storage
build reads (schemas and generated types are likely; agent scripts and package fixtures are
not). Narrowing it edits the Edge manifest, which is itself an Edge input, so it belongs with the
next deliberate roll. A stop request that cannot interrupt a running gate is a second cost here
and has no row of its own yet.

## Row 95 — a requested stop now ends the running gate; what six reviews left open

Landed 2026-11-25 from shem (`gate-live.mjs stop --run <file> --now`). On the same day a refused
push had spent about 100 minutes finishing an `elohim-storage` gate for a closure that could not
land, because a stop only prevented gates that had not started. The change went through six
independent read-only reviews (receipts `review-gate-stop*-receipt.md` in the f4-ask campaign
directory on shem). What they settled: a gate is "stopped" only when a signal sent to its own
verified leader precedes the leader's death by that same signal or its 128+N exit code; every
other exit records what it recorded before; the stop command exits 0 only on a run record naming
its own request.

Rulings made by the integrator, not by the steward, and open to being overruled:
- An independent kill of the gate by the same signal inside the window after a requested stop
  cannot be told apart from the stop and is treated as stopped. No green can result; a real red
  in that window is not recorded.
- The last review marked one item blocking under the criterion "a push with no stop request must
  not be delayed": the stop point adds one more append to the run log, and an append can wait up
  to 10 s on a live lock holder. It was landed as it is, because every event the hook already
  writes takes the same lock with the same bound, so this is one more of an existing bounded
  wait and not a new kind.

Not cured:
- Any local process that can write a run log can stop another session's landing, or forge the
  record that acknowledges a stop. There is no authentication.
- A stop after the last stop point does not stop the landing; the command says so.
- An untapped gate (`GATE_TAP=off`) cannot be ended; a process that leaves the gate's tree is
  named, not ended.
- Nothing was exercised against a real cargo gate dying mid-build, nor on the `ark task run`
  path. Every gate now runs in its own session; no recipe is known to need a controlling
  terminal, and none was tested for it.
- No second device has run it. Under the standing rule a gate change is not met until one has.

## Row 96 — first install of epr on a third device from a sibling's artifact, and three things it showed

Reported by the ThinkPad session, 2026-11-25; not re-run on shem. The laptop (95% full disk, no
cargo pool) fetched epr and bin-launch from shem over the peer runtime (101 and 9 chunks),
ingested them with `--source-commit 4498b9394f16687a53ec05589c915ba726f4fa68`, got the release
CID shem's gate published (`bafyreif47wpi6iakdxocxkaitbial4iwe6ndo5geyukytraukq2xict43y`), landed
it (`"ring":"landed"`, `"treeClean":true`, `landedAt 2026-11-25T12:34:52Z`), restarted its peer
daemon, and the serve log read `payload store … (blob-store-v1)`. It then folded both siblings.
It removed about 8.6 GB of local builds afterwards.

Not cured:
- `just status device` on that laptop still reads `degraded peer daemon: running bafkr4igtg4… ;
  current bafyreif47w… — restart epr peer serve to follow current` after a correct restart. It
  compares the running binary's output CID with the head's release CID. On a device that built
  its own epr the line reads ok; on a device that fetched it, it can never clear.
- `just status device` has no payload-store line, so the format a daemon opened is read from its
  serve log.
- ethosengine's and shem's builds of the same epr sources (c27114cb9 and 4498b9394) have
  different output CIDs. Two producers cannot agree on one output by rebuilding.
- A release still does not state the payload formats it opens. The cure is written on
  ethosengine and not landed; until it is, a device with a `blob-store-v1` store that follows a
  release built before 4498b9394 lands an epr that refuses its store.

## Row 97 — a gate verdict is dated by an unrelated checkout's head

Read by ethosengine, 2026-11-25 (`elohim/eprfs/epr-cli/src/flow/note.rs:24`; not re-read on
shem). `epr flow note` dates every note by the author date of the HEAD commit at its `--root`,
under the ruled choice that dates are history-derived and never the wall clock. Since verdicts
are written from the device's main checkout, a `gate-verdict@1` carries the date of whatever
commit that checkout sits on: unrelated to the commit judged and to the run. A verdict recorded
on ethosengine on 2026-11-25 at about 05:19Z read `2026-11-24T20:56:01Z` on shem.

Corrected the same day: this row first said that freshness windows, a carry's valid-for and
"newest green" read that field. They do not. ethosengine's evidence note
(`genesis/docs/superpowers/specs/evidence/2026-11-25-gate-verdict-time-readers/README.md`) traces
every reader: no decision reads a verdict's time (reuse, the carry's poll, miss settling, live
carries, enrichment, crossing, retention), and "newest green" orders by the attestation's own
build time. What is wrong today is display and ordering: `epr peer verdicts`, note and context
ordering, the ledger, participation. It becomes a decision defect only if a freshness bound on
gate verdicts is ever declared. Not cured; which commit should date a verdict, and whether a
separate observed-at is added, is for the steward to rule.

## Row 98 — brit's quieter capture notice will fail epr's qualification probe

Reported by the ThinkPad session from brit sprint 5 item 7, 2026-11-25; the probe was read on
shem. `elohim/eprfs/epr-cli/src/feedback/qualify.rs` runs brit twice in one session
(`recorder-success`, then `recorder-refusal`) and requires `feedback capture gap` on stderr both
times. Item 7 prints that notice once per session. When the sprint's release moves the brit pin,
the second check fails with "Recorder failure was not visible." Not cured: the probe must change
before the pin moves, in the same landing or an earlier one.

## Row 99 — the git landing exception is retired on brit 0.3.4; what was proven and what was not

The "Temporary landing exception" (dev landings made with `git push` instead of `brit push`) is retired in the root guidance and in the `brit-first` hook's message. A dev landing is now `GATE_EVIDENCE=reuse brit -c core.hooksPath=<worktree>/.husky/_ push origin HEAD:dev` from a prepared worktree; `git push` for a landing is a fallback like any other, stated with its reason and recorded in the sprint journal. This was the integrator's decision (shem, 2026-11-25), taken under the steward's direction of the same day that decisions inside the sprint not wait on him.

Evidence: four dev landings were made with brit 0.3.4 and no git. c1035eebc ran its hook in about 65 s. 40418e53b and d6bf967da followed (d6bf967da about 8 min), and a3632f54 took 1543 s (about 26 min) for hook and push, with the eprfs gate run in the hook. `.git/landing/settle.log` reads `landed … tool=brit … released=ok` for the last three; the first read did-not-land because the landing agent then looked only for a git ancestor, which 40418e53b fixed. The brit team's CI and these landings confirm on 0.3.4 that the `-c core.hooksPath=<worktree>/.husky/_` form runs the worktree's own hook; that the environment (`GATE_EVIDENCE` and the rest) reaches the hook and its output streams; that HEAD is resolved once before the hook and that commit is what is sent, so a commit made during the hook does not go out in that push (a later separate push is not prevented); and that brit connects for the transfer after the hook returns.

Not proven: a hook that runs a full hour (the longest so far is 26 minutes), and a missing hooks directory under brit on this repository. The brit team's journey test says brit refuses with nothing sent there; it was not re-run here, so the `test -x <worktree>/.husky/_/pre-push` precaution stays.

## Row 100 — a latent red on dev for about four hours

On 2026-11-25, 97d92014e (16:46Z) made `.claude/scripts/_lib/session.py` and `epr_client.py` import `_lib/deadline.py`. Two fixtures in `genesis/agentic/shift_lease_test.py` copy a subset of `_lib` and did not copy it. The agentic-shift gate was not selected by that landing. It went red (5 failures, 2 errors) on the next two landings that selected it. Fixed in 668a11d22.

Open: a landing that adds an import to a shared helper is not checked against fixtures that stage a subset of that helper's directory.

## Row 101 — reuse measured on one landing

On 2026-11-25 the refusal half's candidate was pushed twice. First push: 6623 s, refused only by row 100's red. Re-push of the same content plus one test file: 1648 s. steward-node took 1211 s then 43 s; eprfs 1271 s then 44 s; elohim-storage 47 s (its own earlier verdict, which had cost 7134 s on a loaded device) then 53 s. Gates keyed on the whole tree ran both times (orchestrator-tree 522 s then 201 s).

A sibling's eprfs verdict for the same closure did not stand in, correctly: the landing changes `elohim/eprfs/epr-cli/src/peer.rs`, which is in `SIBLING_REUSE_GOVERNANCE_PATHS`, and the log said so (`[gate] eprfs not reused: governance-class change: …`). The integrator misread the log and reported that a declined reuse prints no reason; that was wrong.

Open: the reason prints thousands of lines after the `evidence: miss` line; put it beside it.

## Row 102 — shared cargo slot collisions

On 2026-11-25 four agent lanes on branches named `agentic/*` shared one target slot (`slot_path(family, workspace, profile)`). Three lost their orchestrator-tree verdicts to "file vanished" / "failed to write … invoked.timestamp".

Cure in progress: an exclusive slot lock (ethosengine, pool-slot lane), reviewed five rounds and accepted at v6 with three stated limits: a forged token; a holder's failure after survivors outlive KILL is not propagated; a holder killed after verification leaves the recipe unlocked. It is not landed: shem's gate on it found that `runtime-config-render.test.mjs:438` calls `run-local-gate.sh` directly without the lock. v7 pending.

## Row 103 — a run can be closed twice, and a failed close is silent

`closeRun` (`genesis/orchestrator/gate-live.mjs:306`) treats a run as closed only when its last line is `run-close`. A sibling-delivery line after a close lets the exit trap append a second `run-close` and send a second `gate-final`; `readRun` takes the first close, `finalBody` the last. `gate_live_close_on_exit` (`.husky/pre-push.bash`) discards the close's output and status.

Found while diagnosing a test that failed twice inside the gate and never outside it. The mechanism was reproduced only by forcing a delay, and why the gate failed only that one test is not explained. With ethosengine.

## Row 104 — the device-head resolver looked only at ~/.local/bin/epr

The laptop's launcher is `~/.cargo/bin/epr`, so its landed head (4498b9394) was read as absent and its first thin hand-over (f7fcd7b27) posted no misses, with one line. Every resolver test set `BIN_STORE`, so the launcher lookup had no test. Fixed in 12fd5cc16 (three tests). Found and traced by the ThinkPad session.

## Row 105 — "posted" is not "crossed", and a stalled exchange prints ok

After 12fd5cc16 the laptop's hook said `hand-over: posted 1 misses`, and nothing reached shem. Shem had last folded the laptop's head 8540 s earlier (SILENT printed about 21:15Z on 2026-11-25), while the laptop's own `just status device` printed `ok sibling exchange` with nothing folded for 8667 s. Both daemons were up, on the relay, with no error. Restarting shem's daemon alone healed it within 20 s (about 23:55Z). The same silent stall was seen on 2026-11-05.

Open: the sibling-exchange line reads acknowledgement, not head age in both directions; a hand-over does not learn that its misses did not cross; shem has no liveness watchdog (ethosengine has `genesis/local-dev/peer/daemon-liveness.sh`).

Not proven: the laptop's status line says its daemon runs `bafkr4igtg4…` against current `bafyreif47…`, which may be two identifiers of one build.

## Row 106 — thin-client stations after the first two thin landings

The laptop's two thin landings took 6 s for the worktree, 13 s for prep and 18 s for the push, about 670 MB, nothing built. Open, in order:
- Prep still requires every attested submodule when no project is selected (hidden on the laptop by local checkouts).
- Preflight's cargo-lock check runs `cargo metadata --locked` and tried the network on a docs-only push.
- A cargo-deny cure ran on a push with no Cargo change.
- Prep's hooks-path step points a worktree at the main checkout's hook, which on the laptop is an older branch.
- The pre-push reading leg uses the session's pinned epr and drops readings when the pin lacks `flow reading` (the head has it).
- `[gate] inputs: --inputs unavailable` on brit 0.3.0, waiting on the brit pin.

## Row 107 — unused capacity

For about four hours on 2026-11-25 shem ran every cargo gate serially while ethosengine had 0 of 3 Codex seats and 0 of 2 cargo seats in use and the laptop had readers free; Jenkins had no work from 16:46Z to 21:04Z. The steward asked twice before the integrator asked the other devices. Three reviews queued behind shem came back from the laptop in minutes.

Practice from here: ask each device its capacity when the landing lane blocks; reviews go to a device that did not write the change; a gate-only pull request (`[edge:gate-only]`, live on dev: `genesis/orchestrator/Jenkinsfile:944`) is available for native candidates.

## Row 108 — hook budget overruns under load

PreToolUse hooks timed out several times on 2026-11-25 (for example brit-first at 2 s and dialogue-stasis-sense at 3 s, each over by under a second) while shem's load was 13 to 25. Once the load was an agent's 30 busy loops run for 19 minutes as an experiment.

## Row 77, delta 4 (2026-11-26): a second design depends on this cure, and a collective's case goes further than it

Recorded by shem session 96b46344 while attaching the governed-space design to the epic. Nothing about this row's state changes: it is open.

The governed-space design (`genesis/docs/superpowers/specs/2026-11-26-governed-space-share-boundary-collective-memory-design.md`) lets one device's memory tending be accepted by its siblings. Its acceptance rule begins with "the signer is another admitted device" (station GS-2 of `genesis/docs/superpowers/plans/2026-11-26-governed-space-collective-memory-plan.md`). A device whose checkout does not yet carry a sibling's roster row would refuse that sibling's tending for the same reason it admits nobody here, so that station inherits this gap and carries no station of its own for it.

The design also names a case this row does not cover. For a space held by a collective, the list of members cannot stay in a tracked file at all, because who is a member must not be readable from outside the collective (the design's section 12a, missing node 6). Reading the roster from a published ref, which is this row's candidate cure, does not meet that: a collective's roster and affiliations would have to be held inside its own space.

## Row 110 — the orchestrator's own result went unread for two days, and a fleet roll built nothing

From 2026-11-24 to 2026-11-26 `elohim-orchestrator/dev` had no successful build (#2434 was the last).
`elohim-eprfs/dev` failed on one test in every run: `flow::sense::tests::direct_observation_uses_the_declared_native_writer`
demanded `CARGO_TARGET_DIR`, which a device gate names and the pipeline's nextest run does not. Every device gate was green.
Because the orchestrator never succeeded, its trusted Rust-style baseline stayed at #2434. Once that commit was more than
200 commits behind, it fell outside the job's shallow clone, `rust-style-ci.sh` refused with "trusted baseline object
missing", and the orchestrator stopped before dispatching anything (#2474 to #2477). The roll landed at `0f23d2d49` was not
built until `1812e398c` cured both (the fixture falls back to its own binary's directory; the script fetches full history
when the baseline is outside the clone). #2479 and #2480 were the first green builds.

Owed: the landing job prints `elohim-orchestrator/dev`'s result for the pushed commit, and what it dispatched, as its last
line, so a landing cannot be reported done without it. Until then the read is the integrator's last step by hand.
Jenkins' log readers time out on a running build; its stage view (`…/<build>/wfapi/describe`) answers.

## Row 111 — the early head acknowledgement was dropped: it let two devices hold one lease

A lane proposed counting a sibling's acknowledgement of a lease claim as soon as its head passed the fold verdict, before
that head's content was folded (ref `self/ethosengine-early-ack`, `034b486cf`, withdrawn, kept on the forge as the record).
An independent read found an honest race. A claims; B claims before folding A's claim; B then folds A's claim and announces a
head that acknowledges it, whose content holds B's own earlier claim; A counts the acknowledgement, sees no rival because it
has not folded B's content, reads Effective and lands; A's fold completes and B's claim precedes A's. The rule was safe only
because the acknowledgement and the content it covers arrive in one fold. A head signs device, root, seq, at and folded, and
events sit flat in the channel tree, so no head can prove "nothing unfolded here concerns the lease"; proving it needs a
head-format change. Two tests on dev (`efaed7e67`, eprfs-peer `tests/lease_view.rs`) pin the folded-only rule and name the
race.

## Row 112 — a doorway restart can outlast the server-rendered landing's adoption window

Edge #1930 rebuilt and restarted the doorway. Its validation build #1931 passed fleet quiesce and the dht-fetch smoke and
then failed `verify-projected-head.sh`: after 393 of 400 seconds the alpha doorway's renderer still served the previous
landing head (the refresh nudge returned 401, which the script treats as non-gating). The same script passed by hand three
minutes later, and the validate-only build #1932 passed every line. Not established: whether 400 seconds is too short for
a cold doorway, or the adoption is slower than it should be; and whether the 401 is new. Any roll that rebuilds the doorway
will meet this again.

## Row 113 — the managed launcher adopts itself at every arrival, and a clean release gains a dirty source

`install_launcher` calls `adopt_plain`, which ingests each plain executable it finds at the launcher's destination as
`adopted-from-path`, source commit unknown, tree not clean. One of those files is the managed launcher itself. When the
store already held those bytes, the ingest appended that row to a release built clean at a known commit and overwrote the
record's `provenance.ingestedAt`. Seen on shem and ethosengine on 2026-11-26. Nothing decides on a release's sources today;
the lanes that record closure keys and later draw by closure will read these records. Fix in review:
`self/ethosengine-adopt-held` (held bytes add no source and leave the record untouched; the managed launcher is not adopted).
Records that already carry the row are left as they are.

## Row 114 — `epr peer serve` says "serving" before it can answer for what it holds

`PeerDaemon::start` returns with an empty artifact index and the CLI prints the serving line; the first index rebuild is a
background task. Until it finishes, a request for a release the device holds is answered `NoRecord`, which also means "not
held". Under load the gap passed five seconds. A device that does not build and follows in that window is told the release
is absent. In progress: a distinct "index warming" answer that follow treats as ask-again inside its existing bound. Not
taken: moving the serving line to after the first rebuild.

Correction, 2026-11-26 (the integrator's own): the first form of this row said `just mesh wait`, the device supervisor and
the session hooks wait on that line. Only one of the three was read. What a search of dev at `0b369e541` shows: the line is
`SERVING_LINE` ("epr peer: serving", `elohim/eprfs/epr-cli/src/peer.rs:61`, printed at `:1075`); tests read it
(`epr-cli/tests/peer.rs:1016` and others); the steward-space design names it as the peer daemon's readiness probe under the
device ark (`genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md:203`), and the supervisor
honours a declared stdout-line probe with its own patience (`epr-cli/src/device/supervise.rs:1556`), so a device that
declares the daemon that way waits on the line. `just mesh wait` concerns the household mesh, not this daemon, and no session
hook was found reading the line. Before the line moves, read each device's declared readiness for its peer daemon.
