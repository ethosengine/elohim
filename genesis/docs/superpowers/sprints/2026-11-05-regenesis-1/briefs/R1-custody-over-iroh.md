---
id: rg1-brief-r1-custody-over-iroh
title: Brief R1 — custody announcements must carry an authenticated author over the iroh plane
date: 2026-11-06
status: active
---

# Brief R1 — custody over iroh (Opus 5.5, rust-architect)

Worktree `/projects/elohim/.worktrees/rg1-integrate` (branch `regenesis/integrate` = dev `97a956439`). Work ONLY there; commit by pathspec with trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; every git command's FIRST line is `# Brit fallback: <reason>` (brit panics in worktrees). Never push. Never `git stash`. A household (3 conductors, 3 storage peers, 3 doorways) is RUNNING from this worktree — do not stop, restart or rebuild it, do not run `just mesh *`; the orchestrator does the household validation. Builds: `CARGO_TARGET_DIR` from `cargo-pool key` for `elohim/elohim-storage`, `CARGO_BUILD_JOBS=1`, `RUSTFLAGS` as the gate sets it; one build at a time on this host (a PreToolUse hook refuses a second).

## The regression (reproduced on two households after the address crossing)

Household peers connect over iroh (`p2p/status` → `transportPaths[].transport = iroh`); libp2p gossipsub never forms a mesh (every publish logs `InsufficientPeers`). Custody announcements (`elohim/storage/custody/2`) do ride the iroh gossip plane (`iroh gossip: neighbor up` on that topic), but `handle_custody_announce` (`elohim/elohim-storage/src/p2p/gossip_dispatch.rs` ~775–840) treats `GossipSource::Iroh` as having no authenticated author → `check_binding` → `NoAuthenticatedAuthor` → refused. Result: remote custody never reaches `shard_locations`, `services/household_resilience.rs` counts one intra-household copy, and `features/resilience/chaos-peer-churn.feature:112` refuses at its baseline step. Before the crossing (commit `30e7dde43` introduced author-bound custody) iroh-received announcements were applied. Also: `CUSTODY_ANNOUNCE_TOTAL` never appears in `/metrics` (lazily registered), so the absence of receipts is invisible.

## The fix (keep C1: holder bound to an authenticated author)

1. `elohim/elohim-storage/src/p2p/custody_announce.rs`: `CustodyAnnouncement` becomes a signed object the way `InventoryEnvelope` (`p2p/inventory_gossip.rs`) is: body `{shard_cid, holder_agent_cid, h_app_id, status, announced_at_ms, author_peer}` + `sig: SignatureEnvelope` (elohim-epr `SignatureEnvelope{scheme,key,sig}`, transcript v1 under a new domain `elohim:custody-announce:v2:` with epoch 0), signed with the node's transport key exactly as the inventory delta is. Wire: dag-cbor or the existing msgpack framing — keep whatever `to_bytes/from_bytes` uses, bump nothing else (the topic is already `/2`).
2. Receiver (`gossip_dispatch.rs`): verify the envelope first (`SignatureCheck::{Verified, Invalid, Held}` — Held for an unknown scheme is kept, not dropped, under the existing HELD discipline); derive the author from the signing key (multikey → the transport identity `peer_transport_manifest` binds: libp2p peer id for a libp2p key, iroh node id for an iroh key — reuse `bound_agent_for_author` or add the iroh-key lookup beside it); then `check_binding(author, bound)` as today. A libp2p-sourced message must ALSO pass the signature (the gossipsub author is no longer the only authority). An unsigned announcement is refused on both planes. Do NOT accept unsigned iroh announcements — that would be a C1 regression.
3. Publisher: wherever the announcement is built (`custody_announce` producer; grep `CustodyAnnouncement {` outside tests), sign it with the node's transport keypair the inventory publisher already holds.
4. Register `CUSTODY_ANNOUNCE_TOTAL` eagerly (with the labels `received`, `refused_binding`, `refused_signature`, `held`, `applied`) so `/metrics` shows zeros.
5. Tests in `custody_announce.rs` / `gossip_dispatch.rs`: `signed_announcement_over_iroh_is_applied`, `unsigned_announcement_is_refused_on_both_planes`, `announcement_with_unknown_scheme_is_held`, `holder_not_bound_to_signing_key_is_refused`, `libp2p_author_mismatch_is_refused`; keep every existing test green. `cargo test -p elohim-storage --lib p2p::custody_announce p2p::gossip_dispatch` then `cargo test -p elohim-storage --lib` (4914 pass today).
6. `just gate rust-style` must PASS (elohim-storage carries debt 116: touching it means paying ≥1 finding down).

## Hand-off

Commit(s) on `regenesis/integrate` by pathspec. Report `genesis/docs/superpowers/plans/task-rg1-custody-iroh-report.md` (`status: claimed-not-verified`, `landed_commit`, `verified_by:` blank, `concerns:`, `actor`) with: the signed wire shape (struct text), the domain string, the verification order, the test names + `EXIT=` lines + log paths under `genesis/local-dev/regenesis-1/`, and anything you could not do. Hand back concisely. The orchestrator then runs `just mesh build storage`, restarts the three storage peers and re-runs `features/resilience/chaos-peer-churn.feature`.
