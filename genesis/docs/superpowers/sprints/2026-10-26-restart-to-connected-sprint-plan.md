---
title: "Sprint plan — a restarted workspace returns to latest and to its peers in one step"
id: restart-to-connected-sprint-plan-20261026
date: 2026-10-26
status: active
author: "claude-opus-5-5 (Che workspace shem), for the operator (steward of record) and the device-runtime session on ethosengine"
habits: [workspace-device-arrival, devices-share-one-berth]
cites:
  - .epr-meta/workspace-device-arrival.habit.md
  - genesis/agentic/.epr-meta/devices-share-one-berth.habit.md
  - genesis/docs/superpowers/sprints/2026-10-25-gate-merge-sprint-plan.md
---

# Sprint plan — a restarted workspace returns to latest and to its peers in one step

A Che restart wipes `$HOME` and `/tmp` and leaves the PVC. On 2026-10-25 at 22:19:51Z, shem
restarted in the middle of the T8 run. It came back with stale tools, no peer daemon, and a
session that could not claim anything. Recovery took six manual steps. Until shem's daemon was
running again, the other device had not heard from it for over two hours, and ethosengine's
push to dev was refused as `unconfirmed`.

This sprint makes one command, `epr device arrive`, do that recovery. The SessionStart hook
already runs it. "Latest" means the release the store has landed from dev. "Connected" means
both devices fold each other's fresh heads and a steward-scoped claim settles as `claimed`.

## What the restart broke, and what it took (measured 2026-10-26)

| Broken after restart | Manual cure | Owner of the real fix |
|---|---|---|
| git: dubious ownership | `git config --global --add safe.directory '*'` | devfile `setup-git` (exists; ran late) |
| `~/.local/bin/epr` linked to `/opt/rust/cargo/bin/epr`, which the restart removed; the arrival hook fell through to a cargo rebuild | linked the store's `current` epr | R1 |
| `brit`, `brit-build-ref`, `brit-verify` not on PATH | linked the store's `current` brit | R1 |
| The native launcher was never installed: `epr device arrive` refuses `pass --launcher PATH`, and its `bin-launch` was built only into `/tmp/eprfs-gate-target` | not cured; temporary links stand in, and `just status device` reads them as PATH shadows | R1 |
| A resumed conversation keeps its session id under a new process: `epr session open` refuses `envelope-owner-mismatch` even after `epr session gc` closed the dead owner's envelope, so every berth claim refuses | declared `ELOHIM_SESSION_ID=<id>-r2` with this process as owner | R3 |
| `epr peer serve` died; ethosengine last heard shem 8004 s earlier, and its dev claim read `unconfirmed` | restarted the daemon on the store's `current` with the alpha relay | R4 |

The store itself survived: it had landed the dev build `bafkreigdu4ga…` at 22:06:51Z. So the
channel to "latest" exists. What failed is getting from the store back onto PATH and back onto
the peer link.

## Step 0 — record the red (evidence, not intention)

`workspace-device-arrival` is green but has never been measured after a restart. On 2026-10-26,
`just status device` on shem exits 1 with `REFUSED epr binary` and `REFUSED brit binary`, and
`epr device arrive` refuses `pass --launcher PATH`. Add a DELTA line with that reading and set
the habit to `red`. Its invariant already covers this ("the native tool is reachable … every
REFUSED line names the command that cures it"). The cure `epr device arrive` names cannot run
without a build, so the habit is broken today.

## Lanes

Order: R3 first, because it blocks every resumed session's claims, the worst failure. Then R1
and R2. R4 is ruled (2026-10-26) and can run alongside. R5 measures throughout.

### R1. The launcher travels in the store (device-runtime)

`bin-launch` is ingested and landed with `epr`, as one record or as a sibling tool in
`genesis/agentic/bin-tools.json`. `epr device arrive` then installs the launchers for every
registered tool (`epr`, `brit`, `brit-build-ref`, `brit-verify`) from the store's `current`,
with no `--launcher` and no cargo. It installs them into a persistent directory on PATH and
replaces any named shadow it finds. (`~/.local/bin` survived this restart, but `/opt/rust/cargo/bin`
did not. The install directory should be chosen and declared, not assumed.)

Probe: `arrive_installs_launchers_from_the_store_without_building`. Delete the launchers and the
`/tmp` target, run `epr device arrive`, and assert no cargo process ran and that
`just status device` reads `ok epr binary` and `ok brit binary`.

### R2. Arrival follows dev's landed release (device-runtime)

When the store's head is behind the newest release published from `refs/remotes/origin/dev`
(the registry's `publishedRef`), arrival lands it. When no candidate for that commit exists, it
queues the bootstrap build (today's behaviour) and says so in one line. A session pinned to an
older release keeps its pin (`state: behind`). Only new sessions and the daemon move.

Probe: `arrive_lands_the_published_head_or_names_the_queued_build`. Use a store whose head is
one release behind a published candidate, and another with no candidate.

### R3. A resumed session reclaims its closed envelope (device-runtime)

Design item: `genesis/data/timeline/backlog/session-envelope-resume-after-restart.md` (device-runtime,
landing with slice M). Today an envelope is "bound once and never reopened after closure", so no
process can pick up a dead session's id. Replacing a closed envelope outright would let any
process that declares the id inherit its session-keyed records (covenant clauses, leases,
moorings), so the cure is a **successor generation**. It is admitted only when all three hold:
the old envelope is closed, its owner is proven dead (`owner_dead()`: the boot id changed or the
pid is gone), and the runtime itself names the id (identity source `runtimeAncestor`; a bare
`ELOHIM_SESSION_ID` declaration is not enough). The predecessor's record and seal are kept as a
generation, never overwritten, and covenant clauses are re-offered rather than inherited. A live
owner still refuses `envelope-owner-mismatch` (`device-runtime/src/envelope.rs:80`).

Until R3 lands, the workaround is to declare `ELOHIM_SESSION_ID=<id>-r2` with the current process
as owner. Hooks keyed by the payload's session id (the head notice) won't see the `-r2` pin.

Probe: `resumed_session_opens_a_successor_generation_after_its_owner_died`. Open, kill the owner,
`gc`, then resume under the same runtime-named id. A successor generation opens, the predecessor's
seal is unchanged, and a berth claim from it exits 0. Two refusals hold: the same flow with only
a declared id, and the same flow while the owner is still alive.

### R4. The daemon starts with one command (device-runtime, then the devfile)

Steward ruling, 2026-10-26: "peer daemon should be a single command to startup.. it could be
automated from the devfile or whoever configures it.. we don't have to be dogmatic about it."
Today `devfile.base.yaml` `peer-serve` says serving is "on demand only … an operator action,
never workspace arrival". That comment is not a governed rule with an ID, so no `variesRule`
clause applies. This lane rewrites the comment to match the ruling.

The mechanism: one idempotent command, `epr peer up`, with the name to be settled in R4. It
starts `epr peer serve` on the store's `current`, with the relay and listen address declared
for the device (the devfile's `EPR_PEER_RELAY_URL` and `EPR_PEER_PORT`). It returns once the
daemon is serving. When a daemon is already at `current`, it does nothing. When the running
daemon is behind `current`, it restarts it, because `daemon-not-current` already refuses a stale
one. On a device with no PeerEndpoint row on its steward's roster, it refuses and names
`epr actor device endpoint`.

Who calls it is configuration, not doctrine. The devfile's `postStart` may call it, so may
`epr device arrive` when the device declares so, and so may an operator by hand. The arriving
agent session never supervises the daemon.

Probe: `a_restarted_device_is_heard_within_its_freshness`. After restart and arrival, the
other device's `epr peer status` shows a head from this device within
`berth.channel.freshness_s`, and a `dev-integration` claim settles `claimed`.

### R5. The restart probe (this side)

A receipt that times restart → connected: from `postStart` to the moment `just status device` is
all ok, both devices hold fresh heads of each other, and a lane claim settles `claimed`.
Today's baseline is 2 h 14 min (22:19:51Z → about 00:33Z), mostly unattended wait, with six
manual steps. The target is under 5 minutes with zero manual steps. The receipt is the evidence
that returns `workspace-device-arrival` to green and keeps `devices-share-one-berth` green
across restarts.

## Done means

- After a restart, `epr device arrive` with no arguments returns the device to latest and
  connected. `just status device` exits 0, and the other device folds a fresh head.
- `workspace-device-arrival` is green again on an R5 receipt, with one DELTA line per lane.
- After a restart, `epr peer up` (or whatever the device configures to call it) is the only step
  needed to be heard by the other device again.

## Split between the two sessions

The device-runtime session on ethosengine takes R1, R2 and R3, and R4's mechanism once ruled;
these are in `elohim/eprfs/{epr-cli,device-runtime}`. This side takes step 0 and R5, and runs
the restart test on shem. Each lane lands by its own gated push.
