# Workspace → fleet release: the five-command recipe

Ship a coordinator release from **this workspace** to the deployed fleet over p2p, with no
pipeline in the path, and have the fleet read it without applying anything.

Story: `genesis/a2o/features/delivery/workspace-to-fleet-release.feature`
(steps: `genesis/a2o/steps/delivery/workspace-to-fleet-release.steps.ts`).
Household half of the same ceremony: `features/delivery/runtime-upgrade-propagation.feature`.

**Reading convention (the story asserts this):** every line a developer types starts with `$ `.
There are five of them and nothing else in this file starts that way — Station 5 counts them.

---

## Before you start

- **The peer.** `just dev conductor alpha` starts a conductor joined to the fleet's network
  (app port 4485) plus this workspace's own `elohim-storage` on `:8090`. Command 1 below is that,
  with the follow-set env set.
- **Enrolment is the one step that is not a command here.** A peer follows a channel because its
  runtime config says so:
  - on the **fleet**, that is data — `runtimeConfig.ELOHIM_RELEASE_CHANNELS` per human in
    `genesis/orchestrator/data/deployments.json`, which reaches the peers on the next edge render.
    Every active alpha human is enrolled at `runtime:coordinators:elohim:workspace=observe`.
    Changing a mode there is a commit, a push, and a render — never a live edit.
  - on a **household mesh**, it is one API call: `POST /admin/runtime-config/follow` on the peer
    (`{"channel": "<id>", "mode": "observe"}`), which rewrites the watched runtime-config file and
    reloads it. Before that route lands, the mesh fixture rewrites the TOML by hand.
- **Nothing here applies anything.** Every fleet peer follows this channel in `observe`. Promotion
  to `canary`/`apply` is a separate, deliberate data change made after this ceremony has a receipt.
- **Do not run this while an edge deploy is in flight.**

---

## Mint

```
$ CONDUCTOR_RELEASE_CHANNELS=runtime:coordinators:elohim:workspace=observe just dev conductor alpha
```

```
$ cd genesis/a2o && pnpm exec tsx scripts/epr-release-package.ts --artifact <your.happ> --artifact-class coordinator-bundle --channel-id runtime:coordinators:elohim:workspace --applies-to-from http://127.0.0.1:8090 --peer http://127.0.0.1:8090 --soak-secs 30 --attestation-threshold 1 --out /tmp/workspace-release.json
```

`--applies-to-from` reads **this peer's own** `GET /version` passport, so the release binds to the
validation-rule identity the workspace peer actually runs — the fleet's, because the peer joined
it. On a peer whose role has crossed a lineage, the packager reads that role's *authoring* cell.
The artifact bytes are PUT to this peer's own content-addressed store; any peer can fetch them.

## Publish

```
$ pnpm exec tsx scripts/release-ceremony.ts publish /tmp/workspace-release.json --as workspace --conductors workspace=$(grep -o '[0-9]*' ../../elohim/holochain/local-dev/.hc_ports | head -1):4485 --adoption-url http://127.0.0.1:8090
```

One act, signed by this workspace peer's own key. It declares the release **staging** on the
channel. Nothing is built, pushed, or deployed to move that head.

## Observe

```
$ pnpm exec tsx scripts/release-ceremony.ts status runtime:coordinators:elohim:workspace --conductors workspace=<admin>:4485
```

Did it cross? The election resolves through this peer's own conductor on the fleet's network;
the winner should be the release you just minted, at tier `staging`.

```
$ pnpm exec tsx scripts/release-ceremony.ts attestations <releaseCid> --as workspace --conductors workspace=<admin>:4485
```

Did anyone apply it? A peer that applies a release authors a **soak attestation** anchored on it,
readable by anyone on the network. `linkedCount: 0` is the network's own record that nobody did —
which, with every peer in `observe`, is the expected outcome.

---

## Appendix — not part of the five

- **First use of a channel, ever** (the root record; already authored for the workspace channel):
  `pnpm exec tsx scripts/release-ceremony.ts channel create runtime:coordinators:elohim:workspace --as workspace --conductors workspace=<admin>:4485`
- **This peer's own verdict** (your workstation, not a deployed machine):
  `curl -s localhost:8090/admin/adoption | jq '.channels[] | select(.channelId=="runtime:coordinators:elohim:workspace")'`
  — `verdict.state: "ok"` with `appliedRelease: null` is *admissible, applied by nothing*.
- **The whole ceremony as one checked run** (writes to the fleet's DHT, so it is opt-in twice):
  `A2O_ALLOW_FLEET_WRITE=1 pnpm exec cucumber-js -p delivery features/delivery/workspace-to-fleet-release.feature`
- A deployed peer's *own* verify verdict is private to that peer; read it from the fleet's own
  reports, never by reaching into a pod.
