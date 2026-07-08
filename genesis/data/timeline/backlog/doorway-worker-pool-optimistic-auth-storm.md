---
id: "backlog-doorway-worker-pool-optimistic-auth-storm"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway worker-pool conductor auth is optimistic (hand-rolled rmpv + sleep(50ms) + Ok, no ack read) — it logs 'Authenticated' without confirmation, so a rejected auth becomes a self-sustaining ~10s re-mint/reconnect storm"
slug: "doorway-worker-pool-optimistic-auth-storm"
written: "2026-06-30"
author: "pipeline-shakeout shift"
status: "open"
priority: "medium"
ci_status: backlog
jobs: [elohim-edge]
tags: [doorway, conductor, websocket, authenticate, optimistic-log, remint-storm, worker-pool, holochain-client, wire-message, hygiene]
cites:
  - doorway/doorway-service/src/worker/conductor.rs
  - doorway/doorway-service/src/projection/subscriber.rs
  - doorway/doorway-service/src/services/zome_caller.rs
  - genesis/data/timeline/backlog/conductor-websocket-flap-breaks-deploy-write-path.md
---

# Doorway worker-pool auth is optimistic → self-sustaining reconnect/re-mint storm

## What this is (and is NOT)

This is **Defect A** from the corrected elohim.host RCA
([[conductor-websocket-flap-breaks-deploy-write-path]]). It is a real, code-fixable doorway
bug — but it is a SEPARATE socket from the notarize write path, so **fixing it does NOT recover
elohim.host writes** (that is Defect B, a conductor-side DHT-arc-coherence problem,
operator/infra-owned). This item is hygiene: kill a self-sustaining storm + a misleading log.

## Observed (live Loki, alpha doorway, 2026-06-30)

The doorway worker pool loops, ~every 6-10s, forever (still ~30-36 events / 5min hours in):
`Minted app auth token for worker pool` → `Authenticated with conductor` →
`Re-minted app auth token after unstable conductor session` → repeat. Conductor side:
`websocket.rs:294 "…app port 4445 timed out while awaiting authentication. Dropping connection"`.

Decisive contrast (same conductor, same pod): the doorway's OFFICIAL `ZomeCaller`
(`services/zome_caller.rs`, holochain_client) logs `zome call succeeded` every ~60s — so the
conductor is NOT too overloaded to authenticate; the hand-rolled worker-pool auth path is the
discriminator.

## Root cause (code-verified)

`doorway/doorway-service/src/worker/conductor.rs`:
- `send_authenticate` (`:378-412`) hand-rolls the auth envelope via `rmpv`
  (`{type:"authenticate", data: msgpack{token}}`), `ws_sink.send(Binary)`, then
  `tokio::time::sleep(50ms)` and `Ok(())` — it **never reads the conductor's auth response**.
- So `debug!("Authenticated with conductor")` (`:249`) fires on a successful *write*, not a
  confirmation — it is optimistic and, during this incident, a lie.
- `run_session` (`:301`) treats any session `< STABLE_SESSION_THRESHOLD` (10s) as unstable and
  calls `remint_if_due` (`:345-347`). When the conductor silently drops the never-confirmed
  auth at its ~10s timeout, the loop re-mints (token was never the problem) and reconnects →
  self-sustaining metronome that adds conductor connection churn + log noise and masks the real
  auth state.

## The fix (honor design intent — keep the hostname-capable wrapper)

The doorway deliberately wraps its own `tokio-tungstenite` (hostname support; `AppWebsocket::connect`
needs `SocketAddr`) — do NOT switch to `holochain_client::AppWebsocket::connect`. Instead reuse
the subscriber's already-correct path over the existing socket:

1. **Official encoding.** Replace the hand-rolled rmpv in `send_authenticate` with the encoding
   the subscriber uses (`projection/subscriber.rs:665-704`): `WireMessage::Authenticate { data:
   AppAuthenticationRequest{ token }.try_into()? }` via `holochain_serialized_bytes` (the doorway
   already depends on these crates for the subscriber).
2. **Read the ack.** Replace `sleep(50ms); Ok(())` with the subscriber's bounded
   `wait_for_auth_response` pattern (`subscriber.rs:706-755`): treat a `Close` frame in the
   window as auth FAILURE (return `Err`) instead of optimistic success.
3. **Honest logging + no storm amplification.** Log `"Sent authentication"` before confirmation
   and `"Authenticated"` only on a confirmed round-trip; gate the unstable/re-mint decision
   (`:345-347`) on a real auth failure, not session-length alone, so a transient conductor stall
   does not amplify into a mint/reconnect storm.

## Local verification (Che — no docker/hc/k8s)

```
cd doorway/doorway-service
RUSTFLAGS="" cargo build --release
RUSTFLAGS="" cargo test --lib --bins conductor
RUSTFLAGS="" cargo clippy -- -D warnings && cargo fmt --check
```
- **Byte-parity unit test (deterministic, no conductor):** assert the new `send_authenticate`
  produces bytes identical to the subscriber's `send_auth_request` for the same token.
- **No-spurious-remint test:** assert `run_session` does not re-mint on a clean
  `ChannelClosed`/shutdown vs a true auth-failure close.

## Open uncertainty + falsifier (the first two RCAs were wrong — stay calibrated)

One lens argued the hand-rolled `bin`-token and the official `array-of-ints` token decode to the
**same** `Vec<u8>` under `rmp_serde 1.3.0` — i.e. the encoding is *tolerated*, in which case the
encoding swap alone is cosmetic and only the ack-read + storm-hygiene change behavior. The
official-`ZomeCaller`-succeeds-on-the-same-conductor contrast proves the worker-pool auth path
is the storm source, but does NOT prove whether the proximate trigger is the encoding or the
no-ack handling.

**Falsifier (run post-fix on a HEALTHY conductor):** deploy the official-encoding fix; if
`awaiting authentication` persists at ~30/5min on a healthy conductor afterward, the storm was
no-ack/handling-driven (and the byte-equivalence lens was right). This is why the real
acceptance gate is post-deploy on a conductor that is NOT also suffering Defect B — verify
Defect B is cleared first, else the storm signal is confounded.
