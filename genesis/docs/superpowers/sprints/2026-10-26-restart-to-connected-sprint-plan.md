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
  - gate-merge-sprint-plan-20261025 | Sprint plan | sha256:45b16a3ea7f20ce0 | path: genesis/docs/superpowers/sprints/2026-10-25-gate-merge-sprint-plan.md
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

Slice M already carries half of this. Each release carries `bin-launch` beside `epr`; `land`
refuses a release missing either; and `epr bin --tool epr install-launcher` installs the landed
launcher with no `--launcher` and never falls back to an unrelated one. What R1 adds:

- `epr device arrive` calls install-launcher for every registered tool (`epr`, `brit`,
  `brit-build-ref`, `brit-verify`) from `current`, with no cargo, and replaces any named shadow.
- brit needs a landed release in each device's store first. ethosengine has none yet ("brit
  binary: no executable candidate, pin, or committed head"), while shem's store has one.
- The install directory is declared device state, not a new environment knob. Slice M gives the
  RuntimeManifest a `tools` field, written through the device chain and projected to
  `bin-tools.json` beside the launcher, and that field names it. (`~/.local/bin` survived this
  restart; `/opt/rust/cargo/bin` did not.)

Probe: `arrive_installs_launchers_from_the_store_without_building`. Delete the launchers and the
`/tmp` target, run `epr device arrive`, and assert no cargo process ran and that
`just status device` reads `ok epr binary` and `ok brit binary`.

### R2. Arrival follows dev's landed release (device-runtime)

When the store's head is behind the newest release published from `refs/remotes/origin/dev`
(the registry's `publishedRef`), arrival lands it. Session-arrival `follow` already does this once a
candidate exists; that is how ethosengine landed 8c97287ff at 18:38Z on 2026-10-25. So the gap
is the path with no candidate yet. When no candidate for that commit exists, it
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
seal is unchanged, a berth claim from it exits 0, and the head-notice hook, which is keyed by
the runtime's session id, reads the successor's pin (the blind spot of the `-r2` workaround).
Two refusals hold: the same flow with only a declared id, and the same flow while the owner is
still alive.

### R4. The daemon starts with one command (device-runtime, then the devfile)

Steward ruling, 2026-10-26: "peer daemon should be a single command to startup.. it could be
automated from the devfile or whoever configures it.. we don't have to be dogmatic about it."
Today `devfile.base.yaml` `peer-serve` says serving is "on demand only … an operator action,
never workspace arrival". That comment is not a governed rule with an ID, so no `variesRule`
clause applies. This lane rewrites the comment to match the ruling.

The mechanism: one idempotent command, `epr peer up`. It starts `epr peer serve` on the store's
`current`. It reads the relay and listen port from the device's declaration (slice M's runtime
manifest), with `EPR_PEER_RELAY_URL` and `EPR_PEER_PORT` as overrides, so two devices cannot drift
on relay and the devfile only has to call it. It returns once the
daemon is serving. When a daemon is already at `current`, it does nothing. When the running
daemon is behind `current`, it restarts it, because `daemon-not-current` already refuses a stale
one. On a device with no PeerEndpoint row on its steward's roster, it refuses and names
`epr actor device endpoint`.

Who calls it is configuration, not doctrine. The devfile's `postStart` may call it, so may
`epr device arrive` when the device declares so, and so may an operator by hand. A failed
`postStart` command aborts the whole workspace start, so the devfile's call logs the refusal line
and exits 0. `epr peer up` itself keeps its exit codes for operators. The arriving
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

## Dependency: slice M

R1's remaining half and R4 build on slice M (the bin-launch release pairing and the RuntimeManifest
`tools` field). Slice M changes storage's `release_adoption` and `http.rs`, so it must pass the
NO-SERVING-RECEIPT rule: the household stories `epr-app-deliverability` and
`app-delivery-refuses-fast` must pass on its exact source before pre-push admits it. That chain
was running on 2026-10-26 and takes hours. R3 starts on top of slice M's commit and rebases when M
lands.

## Split between the two sessions

The device-runtime session on ethosengine took R1–R4 on 2026-10-26, with the changes above;
these are in `elohim/eprfs/{epr-cli,device-runtime}`. This side takes step 0 and R5, and runs
the restart test on shem. Each lane lands by its own gated push.
