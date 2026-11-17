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
| R4 | Per-device build notes on the forge (row 3) | Pending one read: who on the forge side reads `refs/notes/brit/build/*`. Sibling verdicts already cross on the peer channel; the forge copy is a mirror and gets no merge strategy over signed records | Spec §2, §8 |
| R5 | First `epr` on a device (row 15) | Pending a design leg against spec §5 | — |

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
