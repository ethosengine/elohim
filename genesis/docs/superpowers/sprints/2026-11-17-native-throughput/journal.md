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
