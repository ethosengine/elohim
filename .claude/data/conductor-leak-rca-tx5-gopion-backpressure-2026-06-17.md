# Conductor anon-leak — RCA progress (source-level), 2026-06-17

**Reached via the `ethosengine/{holochain,tx5,kitsune2}` forks.** Source read at the conductor's pinned versions: tx5 0.8.1 / `kitsune2_transport_tx5` 0.3.2 / `pion/webrtc v4.1.3` / `pion/sctp v1.8.39`. Goes past where #5664 stalled (its dumps were iroh; ours is tx5/go-pion, off-heap CGo).

> ## ❌ FALSIFIED IN PRODUCTION (2026-06-18) — this root cause was WRONG
> The fix below was built, deployed fleet-wide, and the conductor binary **confirmed patched (sha256 `36ddf7ab`) and running** — and the leak **PERSISTED at the same rate** (matthew/james kept climbing ~1.1–1.2 GB/h to OOM post-deploy). So the dead-peer zombie-PeerConnection root cause is **falsified**: the teardown fix (#194/#199) demonstrably works (unit tests pass), but teardown is **not** the leak. Everything below is the now-falsified hypothesis + (correct) source-mechanism detail — only the *conclusion* is wrong. **Fresh investigation: `HANDOFF-2026-06-18-conductor-leak-rca-reopened.md` (profile, don't reason from source).** The deploy pipeline + binary-extraction chain are all proven correct (ruled out as the failure).
>
> ## ~~✅ CONFIRMED · BUILT · VERIFIED · DELIVERED (2026-06-17)~~ (superseded by the production result above)
> **Root cause [thought] confirmed** (source chain below; independently corroborated by the maintainers' own fix tx5 **#194**, which rewrites the exact `Evt::State` arm). **Fix built + empirically verified** locally: built tx5 with #194+#199 (go-pion backend, real pion CGo stack) — the teardown tests `conn_dropped_on_peer_connection_state_{disconnected,closed,failed}` **PASS with the fix and TIME OUT without it** (reverted the one arm to demonstrate). **Delivered:** `ethosengine/tx5@elohim-0.8.1-zombie-fix` (#194+#199, floor) + `ethosengine/holochain@elohim-0.6` (#5719 amplifier brake + tx5 `[patch]`) pushed; upstream comments posted as EthosengineBot on holochain#5664, tx5#196, tx5#207 (see `upstream-comments-tx5-zombie-leak-2026-06-17.md`). Remaining: the custom-edgenode build + canary deploy (needs a Go+nix build host; the final cluster confirmation).
>
> The earlier "NOT a confirmed single root cause" framing below was the in-progress state — superseded by this banner. The two refuted hypotheses (no-backpressure, unbounded-recv) remain correct refutations en route.

> ⚠️ **Honesty correction (supersedes an earlier draft of this file).** An earlier draft asserted "VERDICT: no backpressure → unbounded pion buffer." That is **WRONG** — backpressure *does* exist (see Refutation 1). Do not cite the no-backpressure claim. Careful source reading refuted both of the simple explanations; the real cause is one layer deeper and needs the empirical repro to settle.

## CONFIRMED (primary-source, in the forks)
1. **Off-heap ⇒ Go/CGo, not Rust.** `tx5-go-pion-sys` CGo-compiles vendored `pion/webrtc v4.1.3` + `pion/sctp v1.8.39`; the Go runtime mmaps its own heap → matches the signature (Rust `[heap]` flat, anon grows GBs).
2. **Per-send `GoBuf` is freed.** `GoBuf: Drop → buffer_free` (`go_buf.rs:137`). `DataChannel::send`'s `r2id!` keeps the owned GoBuf alive through `spawn_blocking`, dropping→freeing on return; Go's `CallDataChanSend` (`datachannel.go:231`) does not free (relies on the Rust Drop). No per-send GoBuf leak.
3. **`PeerConnection` and `DataChannel` free on Drop** (`peer_con.rs:103 → peer_con_free`; `data_chan.rs:11 → data_chan_free`). A torn-down connection releases its Go memory.

## REFUTED (the two simple stories — both wrong)
- **R1 — "no backpressure."** WRONG. `tx5-connection/src/webrtc/go_pion.rs` implements send-side backpressure: `Cmd::SendMessage` does `d.send(msg).await`; if pion's returned `BufferedAmount > send_buffer` (`send_buffer_bytes_max` default **64 KB**, `config.rs:161`) it withholds the caller's completion oneshot in `pend_buffer` (caller blocks in `message()` at `r.await`) until `BufferedAmountLow` clears it. The `data_chan.rs:181` "TODO implement backpressure" is at the wrong layer — it's done one level up.
- **R2 — "unbounded recv GoBuf accumulation."** WRONG. The `data_recv` queue is unbounded, BUT `CloseSend::send_or_close` (`lib.rs:92`) is a non-blocking **`try_send`-then-close**: when the bounded (1024) `cmd`/`evt` channel fills, it closes the connection and errors. So a slow consumer kills the connection (the author's "error so the connection will close" intent at `go_pion.rs:245-248` IS wired) rather than buffering forever.

## LIVE CANDIDATES — resolved via pion/sctp v1.8.39 source read

- **R3 (was "leading candidate 1") — SCTP retransmit-to-dead-peer is REFUTED as an *unbounded single-channel* source.** `pion/sctp` `Stream.onBufferReleased` (`stream.go:446-447`) is *"called by association's readLoop to notify this stream that the specified amount of outgoing data has been **delivered to the peer**"* — i.e. `bufferedAmount` is decremented on **ACK**, so it **includes sent-but-unacked (inflight) bytes.** Backpressure therefore DOES see inflight → to a dead peer, `bufferedAmount` rises to the 64 KB threshold and the sender blocks (`pend_buffer`); per-channel un-acked is **capped at ~64 KB**. A single channel cannot grow to GBs.

- **CONFIRMED ROOT CAUSE — permanent zombie `PeerConnection`s: idle dead peers are NEVER reaped.** Per-channel buffer is bounded (R3), so the GBs are the accumulating population of go-pion `PeerConnection`s (each a heavy Go object: ICE agent + DTLS + SCTP) to peers that went dead and were **never torn down**. The full causal chain, read end-to-end:
  1. Peer dies → pion ICE → `updateConnectionState` (pion/webrtc v4.1.3 `peerconnection.go:759-802`) sets `PeerConnectionStateFailed`/`Disconnected` and **fires the `onConnectionStateChange` callback but does NOT call `Close()`** (standard WebRTC contract — the app must close).
  2. tx5-go-pion-sys CGo handler (`peerconnection.go:147-155`) just `EmitEvent(TyPeerConOnStateChange, …, state)` — **forwards the state, does not close.**
  3. Rust surfaces it as `PeerConnectionEvent::State(Failed)`.
  4. **`tx5-connection/src/webrtc/go_pion.rs:128` does `Evt::State(_) => ()` — it DISCARDS the state.** No close, no task-end.
  5. → PeerConnection never closed → Rust wrapper never drops → `peer_con_free` never called → the go-pion ICE/DTLS/SCTP Go memory **leaks permanently.**
  - The ONLY other reap path is `Cmd::SendMessage → d.send → Err → break` — which requires an **active send** to the dead peer. For an **idle** dead peer (nothing being sent), it never fires. **So idle-dead peers are permanent zombies, never reaped.**
  - **This uniquely satisfies the floor-monotonicity constraint.** A *slow-reaped* population reaches steady-state and plateaus; a *never-reaped* one climbs **monotonically** — exactly terrance's signature (clean monotonic, no drops, 0 receipt errors). No other candidate produces monotonic growth on a quiet node.
  - Corroborating: no idle/heartbeat reaper in the tx5 `Endpoint` either (`ep.rs peer_map` pruned only when a Peer task ends, `peer.rs:180-184`); `peer.rs:202-208` self-documents the exact failure mode — a task that doesn't end "*would leave a dead connection in state that the endpoint doesn't know about.*"

- **Go-runtime retention** (freed arenas not returned to OS) is moot here — the memory is never even logically freed (the objects are pinned-alive zombies, not GC'd-but-unreturned).

## ONE unified mechanism explains floor AND amplifier
- **FLOOR** (~0.2 GB/h, ALL 14 nodes, 0 receipt errors — terrance): every node has peers that come and go; those that die while idle become permanent zombies → slow monotonic climb everywhere, arc-independent. ✓
- **AMPLIFIER** (~2 GB/h, matthew/james/jessica): the high-fanout anchors churn the **most** connections/sec → the most zombies/hour. The receipt storm correlates because busy = high churn (not because the receipt path itself leaks; "could not find url for peer" fails at lookup upstream of pion). **#5719** (cherry-picked clean onto 0.6.0) cuts the receipt re-drive — useful brake on the busy nodes — but the zombie fix is what flattens the universal floor.

## Fix — THE UPSTREAM FIX ALREADY EXISTS (independent corroboration of this RCA)
**Do NOT author our own.** holochain/tx5 **#194** (`60c1b48e4`, merged 2025-11-18) — *"fix: drop connection on pion event that PeerConnectionState has changed to disconnected, closed, or failed"* — rewrites the exact arm (`go_pion.rs` +153/-5): `Evt::State(state) => match state { Disconnected|Closed => Cmd::Close; break; Failed => Cmd::Error; break }`, with `Cmd::Close → WebrtcEvt::Closed; break` (ends the task → `DropPeer` → `peer_con_free`) and unit tests. Plus follow-up **#199** (`31e22c7fa`) "don't send Disconnect until peer has been removed from peer_map." The maintainers found the same bug — strong validation.

**Why the conductor still leaks:** both merged AFTER the v0.8.1 tag (2025-11-14) and **no tx5 release bundles them** (latest release = v0.8.1; #194/#199 live on `main` only). tx5 0.8.1 = the conductor's version (Cargo.lock @ a6d4e805) → it has the bug.

**Delivery = the PATCH path with upstream commits (not hand-rolled):**
1. Base the `ethosengine/tx5` fork branch on **v0.8.1** (`3e3b71b`, the conductor's exact version) and **cherry-pick #194 (`60c1b48e4`) + #199 (`31e22c7fa`)** on top — minimal, fidelity-preserving, maintainer-vetted. (Our fork is currently at `27fbdd1` "Prepare next release", which is *below* #194 — so it does NOT yet carry the fix.)
2. `[patch.crates-io] tx5 = { path = "…/elohim/tx5/crates/tx5" }` (+ the sub-crates) in the holochain-conductor workspace so the custom edgenode build links the patched tx5.
3. Note: #194 hard-drops on `Disconnected` (no grace period — slightly aggressive since Disconnected can be transient, but it definitively kills zombies). Carry as-is; don't second-guess the upstream choice.
- (Belt-and-suspenders idle reaper in the tx5 `Endpoint` is now unnecessary — #194 is the targeted fix.)

## What remains for the empirical seal (magnitude only — the MECHANISM is source-confirmed)
The causal chain is read and verified; what the repro (`throughput.rs`/`tx5-demo`, no cluster) would quantify is **magnitude** — per-zombie Go memory × idle-dead-peer rate ⇒ does it match terrance's ~0.2 GB/h and matthew's ~2 GB/h, and does **handling `Evt::State` flatten it**. Open/idle/kill many peers, watch live-vs-zombie PeerConnection count + `/proc/self/smaps` anon. (Toolchain blocked here: no Go 1.24 / no `nix` on PATH; run on a nix+Go host/CI.)

## The empirical seal (cluster-independent, toolchain-blocked here)
Build tx5, run `crates/tx5/benches/throughput.rs` / `tx5-demo` (no conductor, no alpha cluster). Discriminating recipe: **open many peers, kill them mid-stream (churn), and watch go-pion `PeerConnection` count + `/proc/self/smaps` anon + Go `pprof`.** If memory tracks the count of *un-reaped* dead connections (not per-channel buffer), candidate 2 is confirmed; then verify that **handling `Evt::State` + a reaper flattens it.** Profile the **Go** side — Rust heaptrack is blind to pion. **BLOCKER:** no Go 1.24 / no `nix` on PATH here (tx5 ships `flake.nix` but nix isn't installed); run on a nix+Go host/CI/build-pod.

## Forks / pins (working trees)
- `elohim/holochain-conductor` (ethosengine/holochain) — branch `elohim-0.6` off `a6d4e805`; **#5719 amplifier brake** cherry-picks clean (CHANGELOG conflict only). [submodule registration pending — fresh-fork object lag; clone+targeted-fetch used]
- `elohim/tx5` (ethosengine/tx5) — origin=fork, upstream=holochain/tx5; ⚠ on main tip `27fbdd1` (stale local `v0.8.1` tag); buffer/send code identical to v0.8.1 — re-pin `3e3b71b` for a fidelity build.
- `elohim/kitsune2` (ethosengine/kitsune2) — at `22de6e4` = the conductor's flake.lock rev (exact fidelity).

Related: `HANDOFF-2026-06-17-upstream-tx5-transport-pin.md` §3.1, `conductor-leak-upstream-research-2026-06-17.md`, memory `project_storage_metrics_surface_and_leak_verdict`, plan `genesis/docs/superpowers/plans/2026-06-17-conductor-leak-fork-patch-debug-plan.md`.
