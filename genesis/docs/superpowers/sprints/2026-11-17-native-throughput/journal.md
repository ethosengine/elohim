---
id: native-throughput-journal
title: Native throughput — sprint journal
date: 2026-11-17
status: OPEN
author: shem session b49f7c46 (Claude Fable 5.2), orchestrating; ethosengine session 6fa5c22f (Claude Opus 5.5)
habits: [workspace-device-arrival, push-gate-pays-once, gate-failures-arrive-live]
---

# Native throughput — sprint journal

## Why

The steward's definition (2026-11-17): harvest wall clock by increasing native throughput and
usefulness in dev, integration, release and deployment with native primitives. Patches to the
legacy shell gate are not the harvest. This sprint follows the steward-device-arrival sprint
(`../2026-11-16-steward-device-arrival/journal.md`) and works the rows it left open
(`genesis/data/timeline/backlog/steward-device-arrival-open-items.md`) against the spec
`../../specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md` (joins F1r, F2,
F4 and §9).

Authority: the steward affirmed steward of record and declared the purpose for session b49f7c46
(covenant clauses `bafyreidj6l…w3vq`, `bafyreigrmu…rsqsy`), and delegated the sprint's decisions
to the orchestrator. Sibling sessions do not stop for a per-item operator go.

## Definition of done

The sprint closes when each line has its receipt in this journal.

1. **Every landing offers.** Two consecutive `dev` landings, with no manual step, each leave a
   crossed Produce for `epr` and `bin-launch` at the new head; the other device, at that head,
   runs `epr bin follow --tool epr` and lands it with no local build.
2. **The draw is honest before it is paid.** A device that cannot run an offer prints the failed
   requirement and fetches nothing (row 12). A device whose `origin/dev` is stale fetches nothing
   and names the ref and its cure (row 14).
3. **A hand-over costs seconds and is gated once.** A `self/<name>` push from ethosengine takes
   under two minutes, and shem's landing of that head prints `independence: sibling` for every
   gate the sender ran.
4. **A sibling's red arrives live.** The first red of a gate run on ethosengine is in the shem
   orchestrator's context within a minute, and one thin final report follows at exit.
5. **The deployment path is read and its first step ruled.** A finding says what the fleet build
   compiles and what stops it drawing a recorded artifact; the first step has a receipt defined,
   and is built if it fits one landing.
6. Habit deltas and backlog rows are updated where the work landed.

## Rulings (orchestrator, under the steward's delegation)

| # | Question | Ruling | Basis |
|---|---|---|---|
| R1 | Hand-over lanes (row 9) | Adopt the row 9 table. Build step 1 this sprint: `self/<name>` runs preflight and seconds checks, records the hand-over head, and leaves what a `dev` landing demands unchanged. `carry/` stays the existing defer rule under its current names until a carrier folds records back | Spec §4: a name states reach and rung; a verdict is paid once per closure |
| R2 | Stop on a cheap red (rows 4, 10) | No stop. One run still reports every red. Cheap gates are ordered before long ones, and the first red is an event the other device hears | The steward's direction recorded in row 10 |
| R3 | The three generated-output scan skips (row 2) | Drop them. The build-input view makes them unnecessary; the scanner hardening stays | Row 2 finding, 2026-11-17 |
| R4 | Per-device build notes on the forge (row 3) | Each device publishes its build notes to its own destination, `refs/notes/brit/devices/<device-id>/build/*`, and keeps `refs/notes/brit/build/*` locally. Each device then only ever fast-forwards its own mirror; the shared legacy refs stay frozen. No merge over signed records | No forge-side reader or fetch refspec for these notes was found (Jenkins fetches heads and calls `verify` and `plan`; `delivery-series.mjs` and brit read local refs). Sibling verdicts cross on the peer channel |
| R5 | First `epr` on a device (row 15) | A forge release asset carrying the producer's exact `epr` and `bin-launch` for the target. Before either runs, a verifier on the fresh device checks both files and recomputes the release CID against a pin the steward authenticated. Every later `epr` is a native draw. A launcher-carried draw and a standalone fetcher are deferred: each still needs its own trusted first delivery | Spec §5: forge hosting adds no principal and no earned reach; the release CID already covers both file CIDs (`device-runtime/src/bin_store/mod.rs:426`); brit's release workflow is the packaging precedent |

## Lanes

| Lane | Device | Work | Receipt |
|---|---|---|---|
| S1 | shem | Producer in `_eprfs-ingest-around`: describe, key, the gate's own bytes, `put --release --release-bin --cross`, with a test. Batched in the same justfile landing: the pool library's tests enter a gate (row 6) and `family_busy` gets a test (row 13) | DoD 1 |
| S2 | shem | The eprfs live-corpus legs become their own seconds-scale gate project (row 7) | A docs-only change no longer moves the eprfs workspace key |
| S3 | shem | `self/` lane step 1 (R1) and the spec section for lanes | DoD 3 |
| S4 | shem | Deployment path read, ruling, first step | DoD 5 |
| E1 | ethosengine | Requirements cross with the Produce and `select` skips an offer this device cannot run (row 12); the published check runs before the draw (row 14); R3 | DoD 2 |
| E2 | ethosengine | A sibling's reds over the peer channel: first red and thin final report by default, intent stated at subscription, the subscription itself recorded (row 10 a) | DoD 4 |

Landing: shem integrates. Ethosengine hands over on `claude/<name>` until S3 lands, after running
`just gate` for the trees it touched so its verdicts cross.

## Log

### S4 — the deployment path, read (2026-11-17, read-only, dev `b97893259`)

**Finding: the fleet build cannot draw a device's recorded artifact today, and should not try.**

- What CI compiles. Storage and doorway are compiled inside the image build on BuildKit
  (`scripts/ci/build-storage-image.sh:83-95`): storage as `cargo build --release --features
  p2p-iroh…` on `rust:1.100.0-slim-bookworm` (`elohim/elohim-storage/Dockerfile:49,629-634`),
  doorway as `cargo build --release` on `rust:1.93-slim-bookworm`
  (`doorway/doorway-service/Dockerfile:9,155-173`). Recorded cost on a miss: storage 21.9 min,
  doorway 9.4 min; quality gates plus both builds 50.6 and 73.2 min
  (`genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md`).
- Reuse already exists there, and it is the registry's: each image has a closure key
  (`scripts/ci/closure-input-digest.sh`) checked against Harbor, and a hit skips gate, build and
  push in 0.1 min (`elohim/holochain/Jenkinsfile:460-494`).
- Why device bytes do not fit. A device records a dev-profile build
  (`epr-cli/src/artifact/describe.rs:92` fixes `--profile=dev`); the image wants release. Devices
  are el10 with glibc 2.39; the image builds and runs on bookworm, glibc 2.36. The Dockerfile
  compiles a rewritten source layout under `/app`. The method CIDs cannot be equal.
- Why CI cannot fetch at all. `epr artifact fetch` needs a running peer daemon, a device key and
  an admitted roster standing on the steward's devices (`epr-cli/src/peer.rs:1811-1822`); the
  transport is iroh only. Jenkins is a delegated service agent under a compute grant
  (`genesis/agentic/compute/operations.md:5-9`), not an enrolled device, and `carrier:<did>` is
  reserved in `pool-policy.json` and grants nothing yet.

**Ruling (first step):** the direction is the fleet build *recording* what it built, as a carrier
(spec §4 rung R2, join F4), so a device can draw the fleet's release bytes and a deferred gate
ends as a reuse hit. Drawing device bytes into the image is not pursued. The step has three
prerequisites that are not agent work: a device key and roster row for the carrier, outbound
iroh reach from the build pod, and the carrier grant. Until then the registry closure key stays
the fleet's reuse path, and the harvest in deployment is keeping that key from moving for
changes the image does not read. Receipt for the first step when it is built: a local reuse hit
whose signer is the carrier.

Not established: whether a build pod can reach the steward's relays; whether any fleet node is
below x86-64-v3.

### Shem's first batch (2026-11-17)

| Commit | Lane | What |
|---|---|---|
| `bd6a23772` | S3 | A push whose every destination is `refs/heads/self/*` keeps the seconds checks, skips the project gate recipes and writes one advisory record per pushed head (`genesis/orchestrator/handover-record.mjs`). A Codex review (gpt-6.2-astra) returned five findings, all fixed before the commit: the stale-hook note could lead a sender to run no hook; a hand-over ignored a stop request; a ref name with U+2028 escaped the no-defer rule; the negative control failed for the wrong reason; the record append was not atomic |
| `72e84ae8b` | S1, S2 | The eprfs gate's ingest is followed by a producer (`genesis/orchestrator/eprfs-produce.sh`) that puts the gate's own `epr` and `bin-launch` with `--release --release-bin --cross`. The `live_corpus_*` tests move to the `eprfs-live-corpus` gate project. The cargo-pool library's tests run in the agentic gate, with a `family_busy` test |
| `1602399ee` | — | brit 0.2.6 pin, carried from session 93459a2c |

Findings on the way:

- The manual producer of 2026-11-17 did not rebuild: its "scrubbed" cargo run finished in 0.58 s,
  so the bytes put that day were the gate's bytes while the method recorded an environment they
  were not built under. The producer now records the environment the gate builds in, and `put`
  re-checks it.
- `just gate eprfs-live-corpus` went red on its first run: the Rust reader of `// gate-reads:`
  lines did not know the scoped form. Fixed, with one grammar fixture read by both the Rust and
  the JavaScript reader. Warm wall time 58 s (compile about 5 s, tests about 23 s, the rest fixed
  gate cost); the first run in a cold pool family compiled for 9 min 18 s. It is a one-minute
  gate on a warm slot, not a seconds one.
- The terminal that launched the session exported a stale `EPR_BIN`, which shadowed the device's
  head `epr`; commits and pushes here run with it unset.

### Landing 1 (shem, 2026-11-17 22:33Z): `b97893259..bea497110`

The producer ran inside the eprfs gate with no manual step:

```
[gate] epr producer put: release bafyreic4kq…5kzoa at bea497110028, epr bafkr4iedokx…, bin-launch bafkr4ig6pyq… (crosses)
```

The release CID equals the one put by hand at `a51dacfc8`: `epr`'s source closure did not change
between the two heads, so the bytes are the same and only the offer's head is new.

Gate cost of this landing (78 min, the justfile being in every root gate's key): eprfs 1596 s,
memory-ceremony 1152 s, orchestrator 573 s, agentic-shift 280 s (now with the cargo-pool
library's tests), rust-style 253 s, cargo-bridge 238 s, jenkins-bridge 237 s,
eprfs-live-corpus 60 s. The first attempt was stopped by the orchestrator on a misreading: the
security-paydown leg prints `REFUSED` lines while being warn-only by default.

Ethosengine's E1 found a real consequence on the way: `epr` now depends on
`elohim/ark/supervisor` for in-process host admission, so that a device with no `ark` binary can
still be admitted for a draw. Four build manifests gain a watch line for that crate. Ruled: keep
the dependency. It adds two packages to the eprfs lock (the supervisor and `nix`).

### What the 78 minutes were, and the first cut (2026-11-17)

The landing's gate times sum to the hook's 4607 s. Reading the log against the keying code:

- memory-ceremony (1152 s), agentic-shift (280 s), rust-style (253 s), dialogue-stasis (110 s)
  and epr-app-package (70 s) declare no steps, so they have no evidence key and run on every
  push that selects them.
- memory-ceremony shares the eprfs slot but built with `-p elohim-epr-cli`, which resolves
  features differently from the eprfs gate's `--workspace`. It recompiled the workspace and its
  test binaries straight after the eprfs gate built them. With the same package set as the
  ingest and `--workspace` on its two test lines, the same gate ran in **365 s** (compiles 0.55
  to 0.62 s; the same 14 test targets, 210 hook tests and 11 scenarios pass). Saving about
  787 s on every landing that runs both gates, with no change to any key.
- rust-style's cost is one self-test (176 s) that builds the checker in a fresh temporary
  directory. Not changed yet: splitting the checker's own fmt, lint and tests into a keyed cargo
  project would save about 180 to 230 s on landings that do not touch the checker.
- A sound key for agentic-shift is the whole tree, so it would help a retry or a sibling and
  not a first landing. Not changed.

Held, not landed: keying a root gate on the closure of recipes it runs in place of the whole
justfile (lane S5). A Codex review confirmed six ways to change what a gate runs without moving
its key (a recipe reached through `xargs` or a split command word; a `just` call inside a
script a recipe runs; an imported justfile; the `just` binary not pinned between dump and run).
Measured on this landing it would have kept six small gates' keys. The patch is kept at
`genesis/local-dev/native-throughput/s5-recipe-closure.patch`.

### Hand-overs on `self/` (ethosengine, 2026-11-17 and 18)

| Head | Lane | Push wall | What |
|---|---|---|---|
| `6616ce547` | `self/nt-e1` | 38 s | E1: requirements cross with the Produce and `select` admits only an offer this device can run; the published check runs before the draw |
| `76cacc2a6` | `self/nt-e2` | 39.7 s | E2: a sibling's first red and thin final report cross as berth notes |
| `faa225e05` | `self/nt-e3` | 39.5 s | E3: the six review findings on E1, each confirmed against the code and fixed with a test |

The first half of line 3 is met: a hand-over costs under a minute. The hand-over record lists
only the verdicts minted at the pushed head; a verdict reused from an older commit with the same
closure is not listed, and five gates (`epr-storage`, `rust-style`, `rakia-validate`,
`pipeline-list-fresh`, `cargo-coverage`) have no step fingerprint and write no note at all.

A Codex review (gpt-6.2-astra) of E1 returned FIX FIRST with six findings: a disclosed decode gap
was dropped; the size cap ran after decoding and a duplicate commit field took the last value;
the newest offer masked an older compatible one; the build fallback ran before the published
check; "published" accepted a local-only `dev`; an oversized Produce could block the ack of a
sibling's head. E1 landed anyway, because it adds a check where `dev` had none, and the fixes
became lane E3. Residual, recorded and not blocked on: the requirements are the producing
device's own word.

### Landing 2 (shem, 2026-11-18 00:46Z to 03:29Z): `bea497110..6616ce547`

162 minutes, exit 0, no red. The producer ran in the gate: release
`bafyreibp2zww66ca4esr4tw4lcvv3dnomflk5ut7lf53zuhjf7c5wralci` at `6616ce547`.

**No verdict was reused from ethosengine.** Every reuse-enrolled gate printed `not reused:
governance-class change: elohim/eprfs/epr-cli/src/peer.rs`. `SIBLING_REUSE_GOVERNANCE_PATHS`
(`genesis/orchestrator/gate-evidence.mjs:442`) refuses a sibling's verdict for the whole push
when the push changes `peer.rs`, `eprfs-peer/**`, any `gate-*.mjs`, `pool-policy.json`,
`measures.yaml` or a compiler gate input. The rule is one diff over the push, not per project.

| Gate | shem, landing 2 | ethosengine, same crates warm |
|---|---|---|
| `elohim-storage` | 3403 s | 876 s |
| `eprfs` | 2392 s | 1187 s |
| `epr-storage` | 1272 s | 150 s |
| `steward-node` | 1128 s | 117 s |
| `orchestrator` | 474 s | 199 s |

The ethosengine column is its gate of `faa225e05`, one commit later. The integrating device
compiled cold what the sending device had already compiled and judged.

| # | Question | Ruling | Basis |
|---|---|---|---|
| R6 | Who lands a head the governance fence covers | The device that gated it. A device's own verdict is not fenced, so its landing reuses what it paid. The orchestrator orders the lane and holds it for one lander at a time. A head outside the fence is handed over on `self/` and landed by the other device, where the sibling's verdict stands in | Landing 2: 162 min on shem against 48 min of gate on ethosengine for the same closure. The fence itself (decision D5, "the space never rides what it upgrades") is not changed here |

Open, for the steward: every head in this sprint touched a fenced path, because the sprint's
subject is the gate and the peer channel. Whether the fence should be per project (a `peer.rs`
change does not alter what `elohim-storage`'s tests judge) is a trust decision, recorded in the
backlog and not taken.

### Ethosengine lands its own heads (2026-11-18)

| Landing | Head | Gate on ethosengine | Push to `dev` | Reused in the push |
|---|---|---|---|---|
| 2b (E3) | `faa225e05` | 48 min, before hand-over | 321 s | genesis, jenkins-bridge, elohim-ark, eprfs-live-corpus, orchestrator, steward-node, eprfs, elohim-storage |
| 3 (requirements fix + shem's five commits) | `78cd0c0e5` | 33 min (eprfs 941 s, memory-ceremony 463 s) | 631 s | rust-style-checker, jenkins-bridge, eprfs-live-corpus, orchestrator, eprfs |

Ruling R6 has its receipt: the same kind of head cost shem 162 minutes and cost the device
that had gated it 5 to 11 minutes. About 10 minutes of landing 3's push were eleven gates that
are not enrolled for reuse and ran twice (agentic-shift and memory-ceremony are most of it).

**The draw check on `dev` refused every cross-device draw between landing 2 and landing 3.**
At `6616ce547` ethosengine's draw of shem's `epr` printed `requirement-unmet … isa.x86 unmet
(requires subset [… fma4 … vaes vpclmulqdq])` and fetched nothing in 1.2 s. The refusal was
false: the extractor counted CPUID-guarded code paths in dependencies as requirements, and
ethosengine's own build of the same commit lists the same set and runs. Fix `7f48672f8`:
admission judges the level the build declared (target ISA and `-C target-cpu` or
`target-feature` in the recorded flags, baseline x86-64 when none); what the bytes merely
contain is kept as evidence. A build declared above the host, or `target-cpu=native`, still
refuses. Residual: a C dependency compiled with its own machine flags is outside the
declaration.

**Line 1 of the definition of done.** At `78cd0c0e5` shem ran `EPR_FOLLOW_SIBLING=1 epr bin
follow --tool epr` and printed `drew epr release bafyreiebwmixsfw… from did:key:z6MkpZRh… in
11.6 s`, 21.9 s wall, no local build. Landing 1 (`bea497110`) was drawn by ethosengine in
3.7 s, with the caveat that its store already held that release. Landings 1, 2 and 3 each put
a release from the gate with no manual step; the draw at landing 2 was the false refusal above.

A Codex review of E2 (`76cacc2a6`) returned FIX FIRST with eight findings, so E2 did not ride
landing 3. Required before it lands: raw failure text and the absolute run path cross devices;
sibling text enters an agent's context with no untrusted frame; a send has no timeout; the
recipient is read from the note body. Lane E4, ethosengine.

### E4 lands; the live-red test finds a silent red (2026-11-18)

E4 (`94e768892`, the eight E2 findings) landed from ethosengine: 21 min gate, 70 s push. It
landed before the independent review the orchestrator had asked for. The review, run after the
fact, returned FOLLOW-UP REQUIRED: findings 2 and 8 fixed, six partial (consent is not bound to
the sending device; `GATE_LIVE_DEVICE` crosses verbatim; some sends are unbounded; the follower
drops or replays notes around its cursor and crashes on `--timeout-ms 0`; a tap's ninth red is
dropped; a two-branch push tracks one destination). Receipt:
`genesis/local-dev/native-throughput/codex-e4-review.receipt.md`. Lane E5, ethosengine.

**Line 4 of the definition of done is not met, and the first live attempt says why.** Shem
subscribed (`every`, `refs/heads/scratch/*`) at 05:09Z and followed. Ethosengine did not read
that note for five and a half hours; two followers expired with nothing. At 10:43:04Z it fired
the red anyway: `gherkin-prepush-lint` failed in 2 s with exit 1, and no `gate-red@1` was sent.
The tap recognises no failure line in a Gherkin parse error, and the emitter fires only on a
recognised line, so a gate can go red and a sibling hears nothing until the final. The fix is
the first E5 item: a failed step with no recognised line records one synthetic failure event,
and a delivery that finds no device, note or subscriber prints that instead of returning zero.
Shem's follower is re-armed (10:48Z, three hours); the rerun follows the fix's landing.

The five-hour silence is its own finding: a peer note reaches a session only when that session
next reads its inbox. Nothing wakes an agent that is waiting on the wrong signal.

### Shem's batch, built by delegated agents (2026-11-18)

| Work | Where | State |
|---|---|---|
| R4 device note mirrors (`just dev notes [publish]`) | `5317022ca`, `2768f142c` | on the sprint branch, not landed; first real publish not run |
| `epr artifact inputs diff <key-a> <key-b>` | `ce337cf02`, `398224121` | Codex review FIX FIRST (7), all fixed; not landed |
| R5 first-`epr` verifier and packager | `claude/nt-r5-first-epr` | Codex review FIX FIRST (6); fixes in progress |
| Gate verdict enrolment (backlog rows 20, 21) | `claude/nt-gate-verdicts` | in progress |
| `brit log <ref>`, `brit diff tree --name-only` | brit `nt-gaps` `f35d1b753` | local to the submodule; needs a brit release lane |

The inputs diff answered backlog row 22 by measurement: shem and ethosengine never share an
artifact key because their glibc and gcc runtime builds differ (the gcc predefined-macro digest
and ten link inputs). A shared key needs a shared sysroot, not a change to the key.

The installed `brit` on shem is 0.2.0, not the pinned 0.2.6. `brit commit` refuses after a
rebase leaves `AUTO_MERGE`, and `epr bin follow --tool brit` refuses from an unpublished head,
so commits on this branch went through the Git fallback. The follow is retried from the
published head once the batch lands.

### Definition of done, as of 2026-11-18 10:50Z

| Line | State |
|---|---|
| 1 release from the gate, drawn by the sibling | met (11.6 s draw at `78cd0c0e5`); landing 2's draw was the false ISA refusal, since fixed |
| 2 requirements judged at admission | landed (`6616ce547`, `faa225e05`, `7f48672f8`); not re-run by the orchestrator |
| 3 hand-over in seconds, landed by the sibling | hand-overs 38 to 77 s; no `independence: sibling` landing receipt yet (waits on `self/nt-prep`) |
| 4 a sibling's red arrives live | met on a scratch ref at 10:50Z (4.9 s); the fix that made it fire is not on `dev` yet |
| 5 | met earlier in the sprint |
| 6 backlog and habit deltas | backlog current to row 22; habit deltas for landings 2, 3 and E4 owed |

**Line 4, the receipt.** With the fix on a scratch head (`5804e04b3`), ethosengine failed
`gherkin-prepush-lint` at 10:50:03.608Z. Its run log records the stand-in failure 19 ms later
and the note handed to berth in 1.2 s. Shem's follower printed
`SIBLING RED z6MkpZRhMw refs/heads/scratch/dod4b run 4a45f28895574a8d gherkin-prepush-lint step
_gate-gherkin-prepush-lint red #1 +7s; 5 gate(s) still to run, ~9 min` at 10:50:08.474Z:
4.9 s from the failed step to the other device, and it was in the orchestrator's context 21 s
after that. The thin final has its own receipt from the earlier run: recorded on ethosengine at
10:49:43.361Z, printed on shem at 10:49:48.926Z (`SIBLING FINAL … failed in 397s — 0 red(s);
ran 4, reused 0`). Log: `genesis/local-dev/native-throughput/dod4-follow-2.log`. The first
attempt would have passed a test that only checked the final; the red itself was silent.
