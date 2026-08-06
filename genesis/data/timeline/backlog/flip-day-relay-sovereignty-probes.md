---
id: "backlog-flip-day-relay-sovereignty-probes"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Flip-day relay reachability and n0-contamination runtime probes"
slug: "flip-day-relay-sovereignty-probes"
written: "2026-08-06"
author: "codex"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [iroh, relay-sovereignty, dataplane-validation, seam-smoke, wave-2]
shift_objective: |
  Replace seam-smoke[signal-bus] with the Wave-2 relay-reachability smoke and add
  conductor-diagnostics checks that reject n0 relay hosts and lingering tx5 peer URLs.
---

# Flip-day relay sovereignty probes

Claimed by Codex on 2026-08-06 from Wave-2 relay-sovereignty design §5.3.

The relay-reachability smoke must prove both sovereign relays answer `/ping` with
200, answer the plain-HTTP `/generate_204` probe with 204 rather than the ingress
301 trap, and accept an HTTP/1.1 WebSocket upgrade with status 101 while echoing
`Sec-WebSocket-Protocol: iroh-relay-v1`.

The runtime sovereignty check must read only `agents[].url` from each doorway's
`/db/p2p/conductor-diagnostics` response. It strips iroh's canonical trailing DNS
dot before accepting only `relay.alpha.elohim.host` and `relay.elohim.host`, and it
detects lingering tx5 only from a `wss://` peer URL. A bare log grep for `wss://`
is forbidden because healthy iroh relay dialing itself uses WebSockets.

## Claim fence

- `scripts/ci/substrate-seam-smoke.sh`
- focused regression coverage under `scripts/ci/`
- the seam-name comment in `elohim/holochain/Jenkinsfile`
- this backlog claim/resolution entry

Doorway service sources, conductor sources/fork, deployment manifests, and the
operator's live shift write-set are outside this claim.

## Acceptance

- `seam-smoke[signal-bus]` is retired from the runtime smoke.
- Both relay hosts receive all three reachability checks, with `--http1.1` on the
  WebSocket request.
- A 301 `/generate_204`, a non-101 upgrade, or a missing/wrong echoed subprotocol
  is reported as a named relay-reachability failure.
- Sovereign peer URLs with a trailing-dot host pass.
- An n0/public relay host and a peer URL using `wss://` fail distinct named checks.
- Focused tests exercise the healthy path and every probe-authoring trap above.

## Claim resolution — 2026-08-06

Code-complete inside the claim fence. `seam-smoke[signal-bus]` is replaced by
two `seam-smoke[relay-reachability]` checks, one per sovereign relay. Each checks
`/ping`, plain-HTTP `/generate_204`, and an explicitly HTTP/1.1 WebSocket
handshake with the `iroh-relay-v1` response protocol. The two conductor views
are fetched once each and reused for `peer-store`, trailing-dot-normalized
`n0-contamination`, and peer-URL-scoped `no-lingering-tx5` verdicts.

Focused regression evidence: `bash scripts/ci/substrate-seam-smoke.test.sh`
passes the healthy trailing-dot pair and five negative classes (301 redirect,
bad upgrade status, missing/wrong subprotocol, n0 host, and `wss://` peer URL).
The manifest validator reports 12 manifests / 31 steps / 0 errors; the relevant
orchestrator gate passes its Jenkinsfile scope tests. `shellcheck` is not
installed in this workspace; `bash -n` passes for the smoke and its test.

Live public probe evidence: both relays pass `200 / 204 / 101` with the echoed
protocol. The gate then correctly remains red before the conductor flip: each
doorway reports 35/35 addressed peer URLs, all still `wss://signal.*`, so both
`n0-contamination` and `no-lingering-tx5` fail by name. Bootstrap sharing and
the landing canonical head remain converged. This is a measured pre-flip red,
not a claimed delivery-tier promotion; the backlog status stays `wip` until the
transport rollout makes the runtime half green.
