---
title: "Upstream comments — tx5/go-pion zombie-PeerConnection leak (2026-06-17)"
id: tx5-zombie-peerconnection-upstream-contribution
type: history-gotcha
status: noted
tier: history
created: 2026-06-17
topic: [conductor-leak, tx5, upstream, contribution, ethosengine-bot]
---

# Upstream comments — tx5/go-pion zombie-PeerConnection leak (2026-06-17)

**Status: POSTED as `EthosengineBot` (2026-06-17), after building + empirically verifying the fix.**
- holochain/holochain #5664 → https://github.com/holochain/holochain/issues/5664#issuecomment-4732914987
- holochain/tx5 #196 → https://github.com/holochain/tx5/issues/196#issuecomment-4732919972
- holochain/tx5 #207 → https://github.com/holochain/tx5/pull/207#issuecomment-4732920114
- #542 (kitsune2 scope note) — HELD, not posted (lowest value).

Each comment carries the empirical confirmation (built tx5 with #194+#199, teardown tests pass with the fix / time out without it) and the credibility guardrail (framed as *the tx5-transport contribution*, not "the cause of #5664" — recent #5664 profiles are iroh builds with a separate `magicsock` VecDeque grower). Drafts below are the posted text.

Survey + RCA sources: `2026-06-17-conductor-leak-tx5-zombie-hypothesis-falsified.md`, `HANDOFF-2026-06-17-upstream-tx5-transport-pin.md`.

---

## 1 — holochain/holochain #5664 ([BUG] memory leak in Holochain 0.6) — PRIMARY

> We think we've reproduced and source-root-caused the **tx5/go-pion transport contribution** to this on production Holochain 0.6.x nodes — sharing in case it helps the "awaiting clarification / can't reproduce" state. Caveat up front: this explains the **tx5-pinned 0.6.x** OOM shape; it does **not** account for the iroh-build profiles in the later comments here (the `iroh::magicsock` unbounded `VecDeque` looks like a separate mechanism).
>
> **Symptom** (14-node alpha cluster, conductor 0.6.0 / tx5 0.8.1): monotonic off-heap anonymous-mmap growth in the conductor child — Rust `[heap]` dead-flat (e.g. ~32 MB steady across samples) while `other` anon climbs GBs, ~0.2 GB/h on quiet nodes up to ~2 GB/h on busy anchors, ending in OOM-restart cycles. Off-heap + flat Rust heap ⇒ the growth is in the **Go/CGo (go-pion)** runtime, not Rust — which is consistent with the `BytesMut`/networking-buffer + large mmap'd VmSize shape you profiled, but localizes it to the tx5 transport.
>
> **Root cause (source-traced):** dead-peer **zombie PeerConnections**. On peer death, pion's `updateConnectionState` fires the state-change callback but does not auto-close (standard WebRTC contract); `tx5-go-pion-sys` forwards the state to Rust; and `tx5-connection/src/webrtc/go_pion.rs` at v0.8.1 discarded it (`Evt::State(_) => ()`). So the `PeerConnection` (ICE/DTLS/SCTP Go memory) is never closed/dropped/`peer_con_free`'d. The only teardown path needs an *active send* to the dead peer, so **idle** dead peers are never reaped → permanent zombies → monotonic growth on every node (matches our quietest node's clean, drop-free climb). Busy anchors churn more connections ⇒ more zombies/hour ⇒ steeper slope.
>
> **Already fixed upstream, but not released:** holochain/tx5#194 (drop connection on disconnected/closed/failed) + #199 landed on tx5 `main` 2025-11-18/20 — both **after** the v0.8.1 tag (2025-11-14). Conductor 0.6.x pins tx5 0.8.1, so it ships without the fix. A tx5 patch release bundling #194/#199 would resolve the tx5-transport portion for everyone on 0.6.x. Related teardown/liveness surface: holochain/tx5#196 (spike) and #207 (broader hardening, currently paused). Happy to share the smaps time-series / per-process attribution.

---

## 2 — holochain/tx5 #196 ([SPIKE] Connections held beyond their liveliness) — RELEASE-REQUEST HOME

> Confirming **Behavior 1** ("connection stays open indefinitely after a webrtc disconnected/failed/closed event") is a real, production-impacting **off-heap memory leak**, not just a stale liveliness metric.
>
> On Holochain 0.6.x / tx5 0.8.1 the conductor's go-pion child grows anonymous (Go-runtime) memory monotonically — Rust `[heap]` flat, `other` anon climbing GBs to OOM. Source trace: pion fires the state-change callback on peer death but doesn't auto-close, and `go_pion.rs` at v0.8.1 ignored it (`Evt::State(_) => ()`), so dead-peer `PeerConnection`s (ICE/DTLS/SCTP) are never `peer_con_free`'d. Idle dead peers are never reaped (the only teardown path needs an active send), so it's monotonic, not a plateau — which is exactly Behavior 1.
>
> #194 + #199 fix that arm, but they merged after v0.8.1 and aren't in any release, so anyone pinned to tx5 0.8.1 (Holochain 0.6.x) is still affected. Given this spike's AC ("create issues to resolve"), would it make sense to (a) cut a tx5 patch release bundling #194/#199, and (b) track the remaining liveness hardening (the `peer_map` cleanup in #207)? We're carrying #194/#199 as a downstream patch in the meantime.

---

## 3 — holochain/tx5 #207 (WIP: Harden Connection Management…)

> Even though this is paused in favor of iroh — flagging that conductors still pinned to tx5 (Holochain 0.6.x via kitsune2 0.3.x) are hitting the zombie-connection problem in production as an **off-heap OOM**, not just test flakiness.
>
> We root-caused our prod leak to the `Evt::State(_) => ()` gap in `go_pion.rs` (dead-peer PeerConnections never freed → monotonic Go-runtime mmap growth → OOM). #194/#199 fix that specific arm and we're carrying them downstream; the broader `peer_map` cleanup + `wait_for_ready` timeout + the 32-vs-1024 channel-capacity bug in this PR are complementary on the same surface. If a minimal #194/#199 tx5 patch release is feasible before iroh fully lands, it would unblock everyone on 0.6.x. Thanks for documenting the mechanics here — it matched our independent trace closely.

---

## 4 — holochain/kitsune2 #542 (Remove tx5 support) — scope/release one-liner (optional)

> Scope note for the tx5→iroh sunset: since tx5 is removed at kitsune2 0.5.x / Holochain 0.7.x, the only remedy for nodes still on 0.6.x (tx5 0.8.1) hitting the zombie-PeerConnection off-heap leak is a **tx5 patch release** bundling holochain/tx5#194 + #199 — there's no kitsune2/0.7.x-side fix that reaches them. Flagging so the tx5-side release isn't lost in the migration.

---

### Operator decisions before posting
1. **Identity** — post as `EthosengineBot` (the GH_TOKEN account) or do you want to post personally?
2. **Detail level** — drafts name our "14-node alpha cluster, conductor 0.6.0"; trim if you'd rather not disclose deployment specifics.
3. **Which to post** — recommend #1 (#5664) + #2 (#196) at minimum (#196 is the natural home for the release request); #3/#4 optional.
