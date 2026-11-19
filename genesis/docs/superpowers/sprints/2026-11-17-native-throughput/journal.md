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

### The day's landings (2026-11-18, after 10:50Z)

| Time (Z) | Head on `dev` | Lander | Gate before | Push | Reused in the push |
|---|---|---|---|---|---|
| 11:00 | `542e5304c` stand-in red for a failed step with no recognised line | ethosengine | 158 s | 54 s | orchestrator |
| 13:12 | `618597e5f` two tests clear `GATE_LIVE_REF` | ethosengine | 157 s | 53 s | orchestrator |
| 13:24 | `d082572b8` worktree prep (handed over on `self/nt-prep` in 22.9 s) | shem | 224 s on ethosengine | 274 s | orchestrator, from the sibling |
| 14:01 | `7f2710614` E5 (six review items, schema `ref`, gospel sync) | ethosengine | 21 min | 425 s | elohim-storage |
| 14:13 | `6a8a19767` E6 item 1, prep never forces over a staged deletion | ethosengine | 3 min | 50 s | orchestrator |
| 15:31 | `59cbe20cd` shem's batch, 14 commits | shem | 83 min, green at 14:27 | 3360 s | nothing |

**Line 3 of the definition of done, the receipt.** Shem landed ethosengine's hand-over from a
fresh worktree after `just dev prep` (16 s install, preflight clear):
`[gate] orchestrator reused: green in 210 s on the sibling at d082572b8 (signer sibling
did:key:z6MkpZRh…VMZB, independence: sibling)`. `address-profile-lint` is not enrolled and ran
again. Logs `push-nt-prep.log`, `land-nt-prep.log`.

**Shem's batch paid for its gates three times.** (1) A first gate was stopped at 17 minutes to
take review fixes. (2) The second failed in eprfs and memory-ceremony with `failed to write
…/family/claude/elohim__eprfs/dev/debug/build/…/invoked.timestamp: No such file or directory`:
a delegated builder's worktree, also on a `claude/` branch, ran `just gate orchestrator` in the
same pool family and the two runs removed each other's build tree (`.gate-owner` named the
other worktree). Thirty minutes lost; shem's worktree now has its own family record. (3) The
third ran green in 83 minutes, cold. Then the rebase onto four small landings from ethosengine
moved the orchestrator, eprfs and eprfs-live-corpus closures, and the push ran all of them
again: 56 minutes with nothing reused, five minutes after the same commits were green. Which
read those landings touched is not yet established. Every `claude/*` worktree shares one pool
family by prefix; that is a new part of backlog row 18.

The red ethosengine met while gating its hand-over came from shem's own earlier commit
`994f10630`: `just gate` exports `GATE_LIVE_REF`, and two orchestrator tests inherited it. It
bites only when the orchestrator gate runs direct, since a Task runs in a constructed
environment. The run-open event's schema also refused `ref`, which production records carry.

**Independent reviews, and what they stopped.**

| Work | Verdicts | Outcome |
|---|---|---|
| Peer-note injection and pool family record | FIX FIRST | fixed, landed in `59cbe20cd` |
| First-`epr` verifier (R5) | FIX FIRST, FIX FIRST | second round fixed at `22e0c2931`; the hand-off command the note documented did not run, and now ingests a real staged release into a scratch store; third review not run; not landed |
| Gate enrolment (rows 20, 21) | FIX FIRST, FIX FIRST | **held by ruling**, branch `claude/nt-gate-verdicts` `fbbbe6234` |
| E5 | FOLLOW-UP REQUIRED | lane E6; item 1 landed, items 2 to 6 built on ethosengine |

The enrolment ruling: widening the key to hold the ambient environment does not converge. The
second review found a variable read by a dependency (`ESLINT_USE_FLAT_CONFIG`) that changes
the outcome and not the key, a clean checkout at one commit certifying another, a Task child
whose constructed environment omits a keyed variable, and environment values written in
plaintext to a task file. A gate should be reusable when it ran in the declared envelope, so
the envelope is the input. Receipts: `codex-enrol-review.receipt.md`,
`codex-enrol-rereview.receipt.md`.

**The sysroot review (steward's request, 15:00Z).** The two devices run different images by an
earlier ruling: shem AlmaLinux 10.2 x86-64-v2 from `:v2-latest`, ethosengine the UBI 10
x86-64-v3 chain from `:latest`. Every tag in both chains floats, `dnf install` is unpinned,
the registry keeps three dated builds, no package list is recorded, and nothing compares a
running workspace with a declared image. A gate verdict's key holds `rustc -vV` and repository
inputs and nothing of gcc, glibc or the image, so a verdict stands in across the two systems
and across an image update. `epr`, `brit`, `elohim-storage` and `doorway` need only libc, libm,
libgcc_s and the loader, at a glibc floor of 2.39; the fleet's images are Debian bookworm
(2.36), so no workspace-built binary can be deployed, and admission says so. The floor counted
only libc's symbol versions. `sysroot pack|put` exists and a peer can serve a floor; nothing
links against one. Built, not yet reviewed (`claude/nt-sysroot-visible`): `epr device status`
names the sysroot and says when it changed, and the floor counts every glibc library. Backlog
row 26.

Not done: `just dev notes publish` (the dry run lists 47 refs including seven `ci:*` notes,
and whether device mirrors carry those is the steward's open decision); `epr bin follow --tool
brit` from the published head ran 300 s with no output and was stopped, so shem's brit is
still 0.2.0.

### Evening: the live-red loop closes, and the read audit is found wanting (2026-11-18)

| Time (Z) | Head on `dev` | Lander | Push | Reused |
|---|---|---|---|---|
| 15:56 | `40ad0c3cf` E6 items 2 to 6 | ethosengine | 268 s | schema-dna, genesis, orchestrator, elohim-storage |
| 16:22 | `5449615ce` E8, three items | ethosengine | 51 s | orchestrator |

The E6 review again returned FOLLOW-UP REQUIRED. Ruling: the gate-live review loop closes
after E8 (a follower that marked notes seen before printing them, a prep that could still
force over a deliberately emptied submodule, a final one byte over its cap). Each round fixed
the last findings and surfaced smaller ones; the eight that remain are recorded as known
limits in backlog row 27 and are not a reason for another round.

**Why shem's rebase reused nothing (E7, ethosengine).** One read moved the eprfs key: the
directory `elohim/sdk/schemas/v1`, where a property had been added to the gate-live event
schema. The read comes from `elohim/epr/tests/schema_contract.rs` and
`elohim/placement/core/tests/schema_contract.rs`. Those crates are dependencies of the
`elohim/eprfs` workspace and not members, and `cargo test --workspace` never compiles a
dependency's tests, so neither file runs in the gate. Skipping such files makes the eprfs key
equal at both bases (`38cf5829…` at `542e5304c` and at `6a8a19767`; 217 reads against 230).

**That narrowing is held, because the phantom reads carry real ones.** It was reviewed before
landing, twice. The first review found that doorway's own schema-contract test reads
`views/auth-response.schema.json` and that only the dependency's test had put the schema in
doorway's key. The second found the same shape in the two projects the narrowing was for: a
member test of eprfs reads `.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json`
and asserts its dimensions, the live corpus fails if a cited path disappears, and the
orchestrator's own `gate-live-schema.test.mjs` loads the event schema. The audit finds none of
these reads and reports none unresolved. Today's keys hold them only because an unrelated test
file in a dependency happens to read the same place. Lane E9 declares the real reads
(widening only); the narrowing waits on `self/nt-audit` (`d9d7933c4`).

Taken with the enrolment ruling, the finding is one statement: a static scan and an ambient
shell are not authorities on what a gate reads. The envelope that runs the gate can observe
its reads and its environment; a verdict keyed on that observation needs no scan to be
complete. Backlog row 28.

**Glibc floor.** Reviewed twice and split from the sysroot status line: the floor counts every
glibc library's version needs, `GLIBC_ABI_DT_RELR` on libc is a 2.36 floor and no longer a
refusal (reproduced on `/usr/bin/getconf` at `dev`), loader names match exactly, and a needed
libc with no numeric need still refuses. The reviewer checked the binaries the devices
exchange: each keeps its floor. The sysroot status line is in a third round (arrival could
hang on a pipe held by a descendant; status trusted paths in its own record).

**E9 and the sysroot line (18:30Z).** Ethosengine landed E9 at `1bb6d1c90` (799 s push): the
embedding model manifest, 26 cite targets (an undeclared one fails the leg) and doorway's
schema path are keyed where the code lives, and the orchestrator is keyed on the whole tree
because its Node, Python and shell suite was never audited and reads 171 paths outside its
sources. The cost showed in that push: the orchestrator ran 182 s where earlier landings
reused it, and memory-ceremony, not enrolled, ran 456 s. `epr device status` now carries a
`sysroot` line: `unknown sysroot: not yet observed` until an arrival records the identity,
`ok` while the recorded link inputs still hash the same, `degraded sysroot: changed since …`
for seven days after they change. Three independent reviews; the last round (bounded
`os-release` read, a refused reader thread, a retargeted library symlink) landed without a
fourth. Run read-only on shem before landing: exit 0 in 1.2 s. Not verified: a real arrival
writing the record, and ethosengine. Receipts `codex-sysroot-review{,2,3}` and
`codex-sysroot-rereview` under `genesis/local-dev/native-throughput/`.

**The sysroot line lands on the second push (20:27Z, `1a0879d0a`).** The first push was
refused by the gate: `elohim-epr-cli` did not compile with `default-features = false`, which
is how `bridges/cargo` and `bridges/jenkins` take it, because the new code called into the
`peer`-gated `artifact` module. Three reviews and every local check had used default
features. The probe moved to an ungated module; both feature sets and both bridges were proven
before the second push (2718 s).

**A drawn binary crashed on shem, and line 1's receipt is corrected.** Verifying the landing,
the orchestrator ran `epr device arrive` on shem. The arrival restarted the device supervisor,
which started the peer daemon from the installed `epr`: the release shem drew from ethosengine
at landing 3 (`bafyreiebwmixsfw…`). It died at once with SIGILL (exit 132; kernel `trap invalid
opcode … in epr`), in `selectExpander`, bundled SQLite. Shem's CPU is x86-64-v2. Shem's gcc
defaults to `-march=x86-64-v2` (AlmaLinux's v2 flavour); ethosengine runs RHEL 10, whose gcc
defaults to v3 (to be confirmed there), so C code its gate compiles carries v3 instructions
with no CPU check. The admission fix `7f48672f8` judges the level the Rust build declared and
named a C dependency's flags as a residual; the residual is every C dependency. The refusal it
removed was true for the C part. Shem's daemon had kept running its own older build since
before the draw, which is why nothing failed for sixteen hours. Recovered at 20:29Z: shem
followed its own release `bafyreih54myd2…` at `1a0879d0a`, the supervisor restarted, the peer
serves and ethosengine's head is seconds old. Until admission counts the C driver's level,
shem does not draw `epr` from ethosengine; the other direction is sound. Lane E10
(ethosengine), reviewed before it lands. Backlog row 29.

### Definition of done, as of 2026-11-18 20:40Z

| Line | State |
|---|---|
| 1 release from the gate, drawn by the sibling | drawn in 11.6 s at `78cd0c0e5`, but the drawn binary cannot run its peer daemon on shem (SIGILL); **not met** until admission counts the C compiler level (row 29) |
| 2 requirements judged at admission | landed; not re-run by the orchestrator |
| 3 hand-over in seconds, landed by the sibling | met: 22.9 s hand-over, `independence: sibling` on the landing |
| 4 a sibling's red arrives live | met: 4.9 s on a scratch ref; the fix is on `dev`; E6 items 2 to 6 open |
| 5 deployment path read, first step ruled | met earlier in the sprint; the sysroot review adds that the fleet cannot run a workspace build |
| 6 backlog and habit deltas | current to row 28 |

## 2026-11-19: row 29.2 answered — C compiles at the Rust target's baseline

**The record of the crash is corrected.** The trap was not in SQLite's `selectExpander`. All
four `invalid opcode` lines in shem's kernel log share one offset, which decodes in the drawn
binary to `vpbroadcastd ymm0,xmm0` (AVX2) in ring's C `curve25519.c`. The compiler wrote its own
flags into that file's debug record: `-march=x86-64-v3` in ethosengine's build, `-march=x86-64-v2`
in shem's. About 211 C functions in the drawn binary carry v3 instructions with no CPU check
(mostly SQLite, plus ring's plain-C fallbacks); shem's own build has none.

**Why only `epr`.** The cc crate decides. `elohim-storage`, `doorway` and `steward-node` lock cc
1.11.1, which passes `-march=x86-64` itself for this target. `elohim/eprfs` locks cc 1.2.65,
which passes nothing, so `epr` took each device's compiler default (v3 on RHEL 10, v2 on
AlmaLinux 10).

**The admission branch does not land.** `claude/nt-isa` (four rounds, head `1ac49d13e`)
predicted the C level from a compiler probe and judged releases in the bin store. Three review
passes and an eight-lens review each found new ways a predicted level differs from the compiled
one (unrecorded variables, two environments in the mesh script, a second cc version, level
rounding) or the judge is bypassed (a workspace restart, session pins, a missing sibling
statement). Its store hook also lives in `elohim/eprfs/device-runtime`, a storage build input,
so landing it would dispatch Edge and roll the alpha fleet. The branch stays as a reference.

**The decision** (steward asked the orchestrator to answer it, 2026-11-19; a panel of six fact
finders, five advocated options, fifteen adversarial critiques and three judges, unanimous on
the level). The gate builds C at a declared, fixed level on every device, and the level is a
rule, not a number chosen for two machines: C compiles at the baseline the Rust target already
has. For `x86_64-unknown-linux-gnu` that is `-march=x86-64`.

- Plain x86-64 over v2: it equals the Rust level, so a release has one declared level and
  nothing to predict; it is what cc 1.11.1 and the fleet's bookworm images already produce; the
  fleet runs four alpha peers on shem's v2 CPU today; and on SQLite v2 differs from v1 in 0.11%
  of instructions at `-O2` and none at `-O0`.
- Scope: the gate projects on the `elohim/eprfs` workspace (eprfs, eprfs-live-corpus,
  memory-ceremony, orchestrator). Widening is one list entry and hours of one-off rebuild.
- A triple with no record (aarch64 today) builds for itself and offers nothing until its record
  exists.

**What enforces it**, each a small landing (lane E12, ethosengine):

| Step | What | Where |
|---|---|---|
| set | a per-triple record `declared_targets` (isa, cflags, workspaces); the gate exports `CFLAGS_<triple>` and `CXXFLAGS_<triple>` for listed workspaces, inside the gate key | `genesis/agentic/pool-policy.json`, `gate-runner.mjs` |
| lock | cc >= 1.11.1 in `elohim/eprfs/Cargo.lock`, so manual cargo compiles the same flag | same landing as the record, one C rebuild per device |
| check, seconds | the compiler is probed with the record's flags before any build; a mismatch or a stray C variable warns, and the release is not offered (the device's own gate still runs) | new `c-baseline.mjs` in preflight and in `eprfs-produce.sh` |
| check, bytes | at put, the recorded flag must equal the declared one and ring's compiler-written flags in the binary must say the same; only then is `env:isa-c` written | `epr-cli`, producer side (L2a) |
| refuse | a sibling release without the marker value this device knows is refused before fetch, as unmeasured | `epr-cli`, `sibling::admissible` (L2b) |
| run | staged sibling bytes run one x25519 agreement and one SQLite round trip before ingest, and again inside `peer serve --check` before they become current; death by signal refuses | `epr-cli` (L2b) |
| watch | a pin test holds the record, the producer script, the cc locks and the Dockerfiles together | new `c-baseline-pin.test.mjs` |

**What it does not promise.** It sets and cross-checks flags; it does not prove every byte.
SQLite has no debug record, so ring's thirteen files stand in for it, and the exercise runs two
paths out of many. A dependency that adds its own flag, or compiles outside the cc crate, passes
until it traps. Objects linked from the distribution are compiled at the distribution's level;
only a linked sysroot closes that (row 26.4, whose ISA half this answers and whose glibc half
stays open). Neither device is a v1 CPU, so the level proven on hardware is v2, on shem. Until
the store hook lands on a deliberate Edge roll, the crashed release on shem stays reachable by
`EPR_PIN_CID`, by `epr bin pin --cid`, by `epr bin land --revert`, and by bootstrap if
`current.json` is lost; nobody runs those on shem meanwhile.

**The experiment comes first.** Nobody has yet built `epr` on ethosengine at the baseline and
run its daemon on shem. E12 starts there, before any landing. If it fails, the answer falls back
to "declare, refuse, build locally" until a sysroot is linked.

**Two fan-outs alongside it.** A read audit of the reuse-enrolled Rust gates (row 28) confirmed
53 reads a gate performs outside its key (34 claims refuted): eprfs is the leakiest, through a
Python fixture suite and `berth` reading `.claude/hooks`, `.claude/scripts/_lib` and the lens
files; `pool-lib.sh` is unkeyed for every cargo gate; storage reads a schema inside the rakia
submodule that neither its key nor its dirty check sees. The audit's critic: a static scan
cannot close this, only an observed trace of a real gate run can. Ethosengine's E11 hand-over
(`self/nt-e11`, an observed read fence for the orchestrator suite, 160 s to 107 s on a docs-only
landing) is under review as that primitive.

**Cost, stated.** The first two fan-outs ran every agent on the top model and hit its usage
limit: 148 agents, 19.6M tokens, 55 failed. The rerun and the three later fan-outs completed.
Also found: `berth say` fails silently-to-the-peer when the note write exceeds a fixed 5 s on a
loaded device (backlog row 30).
