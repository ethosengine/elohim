---
title: "Re-genesis 1 R1 report: custody announcements carry an authenticated author over iroh"
id: task-rg1-custody-iroh-report
status: claimed-not-verified
landed_commit: "afadfb5c9 + 33bf2b742 (ruling follow-up) on regenesis/integrate (local, unpushed)"
verified_by:
actor: agent:rust-architect@claude-opus-5-5
cites: []
concerns:
  - "The push wire gained a trailing optional `h_app_id` on `ShardRequest::Push`. iroh frames are map-keyed (old builds ignore the key); libp2p frames are positional, so a new distributor pushing to a pre-R1 holder over libp2p is refused by the old decoder and the push router falls back once to iroh. A mixed fleet over libp2p only would lose those pushes until every holder runs R1. A scope-less push stays byte-identical (the historical-bytes tests still pass)."
  - "Dual nodes now sign custody claims with the iroh key (author = iroh NodeId), not the libp2p identity key the brief named. Reason measured on the running household (read-only sqlite on each content.db, 2026-11-06 ~15:30Z): every peer_transport_manifest row binds iroh_node_id and has libp2p_peer_id NULL, so a libp2p-key-signed claim could only ever be refused as UnboundAuthor there. The inventory envelope still signs with the libp2p key and meets the same unbound author on that household — not touched here; it is the next likely silent refusal on the same plane."
  - "Relay-source refusal now reads peer_identity_bindings as well as the manifest, and lookup_active falls back to UNVERIFIED binding rows. An unverified row naming a relay's PeerId as another agent can therefore refuse a libp2p delivery of a genuine claim (the iroh copy still applies). That is the ruling's 'must not contradict' taken literally; restricting the contradiction to cross-signed rows would close it."
  - "Custody rows seeded BEFORE this fix are not re-announced (the catch-up arm is dormant), so the chaos ladder's pre-existing content still counts one copy on the running household; the orchestrator validates by recasting the household on the fixed head."
  - "Pure-iroh mode (no libp2p P2PNode) still publishes no custody claims: distribute_shards and the held-shard sink live on the libp2p node. Unchanged by R1."
  - "The custody catch-up arm (projection_reconcile.rs reconcile_shard_locations_from_peers) is dormant, so a claim missed on gossip is never repaired; its inventory entries are also unsigned and would bypass C1 if it were woken. Out of scope; named only."
  - "A holder announces only shards it is pushed WITH a scope (new distributors) and only when its own agent_cid resolves (active session or boot cell key); a holder whose self agent_cid differs from what peers' manifests bind will be refused_binding — now visible as a zero-or-more counter."
  - "Verification ran on a cold pool slot (/projects/.cargo-target-pool/family/sprint/elohim/dev) waiting for other agents' cargo between steps; the first focused run used rustup stable (1.99.1) before the toolchain pin was added — the later runs pin 1.96.1 as the gate does."
  - "The signed body is encoded with serde_ipld_dagcbor directly, as InventoryEnvelope::signed_body does; the interface-first-reuse hook pointed at elohim/epr's canonical codec (src/cbor.rs). Kept parallel to the inventory precedent rather than diverge from it; a shared helper for both envelopes would be the follow-up."
  - "brit commit refused in this worktree (\"refusing to ignore an untrusted core.hooksPath\"); the commit used the Git fallback by pathspec."
  - "Commit attribution UNMEASURED (the prepare-commit-msg hook reported no epr session model); Co-Authored-By and Claude-Session trailers were written by hand."
---

# R1 report — custody over iroh

## What was wrong (three stacked defects, not one)

1. **No authenticated author over iroh.** `handle_custody_announce` treated `GossipSource::Iroh` as authorless, so `check_binding` refused every iroh-delivered claim (`NoAuthenticatedAuthor`).
2. **The producer did not reach iroh.** `P2PHandle::announce_custody` sent `P2PCommand::GossipPublish`, which publishes straight to gossipsub, bypassing the fan-out publisher. In the household libp2p does not form a mesh (`InsufficientPeers`), so the claims did not leave the node (`received` stayed at zero per the brief).
3. **Push-ack claims named a remote holder.** The distributor announced "peer X holds shard S" after a push-ack; under C1 (holder must be the signer's bound agent) that claim cannot bind on either plane (enforced: elohim/elohim-storage/src/p2p/custody_announce.rs `check_binding`).

## The fix

### Signed wire shape (`elohim/elohim-storage/src/p2p/custody_announce.rs`)

```rust
pub struct SignedCustodyAnnouncement {
    pub sig: SignatureEnvelope,      // elohim_epr {scheme, key, sig}
    pub body: CustodyAnnouncement,   // {shard_cid, holder_agent_cid, h_app_id, status, announced_at_ms}
    pub epoch: u16,                  // 0
    pub author_peer: String,         // libp2p PeerId or iroh NodeId text of the signing key
}
```

Wire: MessagePack, map-keyed (`rmp_serde::to_vec_named`), the framing the v2 announcement already used. Topic unchanged: `elohim/storage/custody/2`. `CustodyAnnouncement` itself is unchanged and remains the unit the projection and the (dormant) catch-up inventory carry.

- Domain: `elohim:custody-announce:v2:` (`CUSTODY_ANNOUNCE_DOMAIN`), epoch 0 (`CUSTODY_ANNOUNCE_EPOCH`), transcript v1 (`SignatureEnvelope::sign_ed25519` / `check`).
- Signed body: dag-cbor of `{epoch, status, h_app_id, shard_cid, author_peer, announced_at_ms, holder_agent_cid}` in canonical key order — every claim field, the author and the epoch.
- Size cap: 4096 bytes (`CUSTODY_ANNOUNCE_LIMIT`, the iroh-gossip message cap).

### Verification order (`custody_announce::admit`)

1. Oversize → refused (`refused_signature`).
2. Header `{epoch, sig.scheme}` must decode; an unsigned announcement has none → refused on **both** planes (`refused_signature`).
3. Unknown epoch or scheme → **Held**: retained opaque in `held_envelopes` (topic `elohim/storage/custody/2`) and not projected (`held`; enforced: `announcement_with_unknown_scheme_is_held`).
4. Full decode (`refused_signature` on failure), then structural check of the claim body (`refused_structural`).
5. Signature verifies under the envelope's own key (`refused_signature`; a `Held` check result is held).
6. The author is derived from the signing key: `author_peer` must be one of the key's transport spellings (the libp2p PeerId it makes, or the iroh NodeId hex it is) — else `refused_signature`.
7. Binding lookup: the agent `peer_transport_manifest` binds to that transport identity (`bound_agent_for_author` over `libp2p_peer_id OR iroh_node_id`, claimed spelling first, then the key's other spelling).
8. C1: `holder_agent_cid` must equal the signer's bound agent (`check_binding`) — else `refused_binding`.
9. libp2p only (coordinator ruling, 2026-11-06 after the household run): the gossipsub `message.source` is a relay identity. It need not match the signer, but it must not contradict the claim: if `relay_agent` (manifest, else an active `peer_identity_bindings` row) binds the source to a DIFFERENT agent than the signer's → `refused_binding`. An unbound or absent source does not refuse. The first cut required the source to be the signer or the signer's own bound transport; on the household that refused james's iroh-signed claims relayed under his unbound libp2p PeerId (refused_binding 3 on matthew and jessica, 17:05:08Z).
10. Then, unchanged: dedup → self-drop → `apply_peer_announced` (`applied` / `dropped_weaker` / `dropped_self`).

### Producer

- `P2PHandle::announce_custody` (self-held after placement) now sends `P2PCommand::AnnounceCustody(body)`; the node signs and publishes through its configured gossip publisher — the `FanoutGossipPublisher` in dual mode, whose libp2p leg is still `GossipPublish`, so nothing recurses.
- Custody signer: defaults to the libp2p identity key (author = PeerId); `main.rs` dual wiring switches it to the iroh endpoint key (author = NodeId) via `P2PNode::set_custody_signer`.
- Push-ack: the distributor no longer announces a remote holder. `ShardRequest::Push` carries `h_app_id`; the holder's `ShardService::handle_push`, after storing CID-verified bytes, reports `(cid, h_app_id)` to a `HeldShardSink` (installed by `P2PNode::with_shard_service`), which queues `P2PCommand::ClaimHeldShard`; the node probes the pantry, records its own `self-held` row (`custody_announce::holder_claim_after_store`) and publishes the holder-signed claim.

### Metrics

`CUSTODY_ANNOUNCE_TOTAL` labels are pre-touched at zero inside `register_all` (`CUSTODY_ANNOUNCE_DIRECTIONS`: sent, received, refused_signature, refused_binding, refused_structural, held, applied, dropped_weaker, dropped_self). `received` now counts every arrival on the topic before any verdict.

### Rust-style paydown

Two reasonless `#[allow(dead_code)]` in `p2p/mod.rs` (`P2PCommand::PublishEprHead`, `P2PHandle::publish_epr_head`) now carry RS-GOV-04 reasons.

## Tests

Named by the brief:
- `p2p::gossip_dispatch::tests::signed_announcement_over_iroh_is_applied` (signer = iroh key, manifest binds only the NodeId, as on the household)
- `p2p::gossip_dispatch::tests::unsigned_announcement_is_refused_on_both_planes`
- `p2p::custody_announce::tests::announcement_with_unknown_scheme_is_held` (scheme and epoch)
- `p2p::custody_announce::tests::holder_not_bound_to_signing_key_is_refused` (mismatch, unbound, author not the signer)
- `p2p::gossip_dispatch::tests::libp2p_author_mismatch_is_refused` (now: a source bound to another agent refuses)

Named by the ruling:
- `p2p::custody_announce::tests::libp2p_source_unbound_does_not_refuse_a_signed_bound_claim`
- `p2p::custody_announce::tests::libp2p_source_bound_to_another_agent_refuses`
- `p2p::gossip_dispatch::tests::an_iroh_signed_claim_relayed_over_gossipsub_projects_unless_the_relay_is_another_agent` (the household shape end to end, with the `peer_identity_bindings` path)

Added for the push-ack correction and the rest:
- `p2p::gossip_dispatch::tests::push_ack_custody_is_claimed_by_the_holder_and_projected_by_peers` (push with scope → holder stores → self-held row → holder-signed claim published → peer projects the holder over iroh)
- `shard_service::tests::a_scoped_push_reports_the_stored_shard_to_the_holder`
- `p2p::custody_announce::tests::an_iroh_signed_claim_binds_through_its_node_id` (now: same-agent, unbound and absent relay sources are admitted)
- `p2p::custody_announce::tests::signed_announcement_verifies_and_a_tampered_field_does_not`
- `p2p::custody_announce::tests::signed_announcement_bound_to_its_signer_is_admitted_end_to_end`
- `p2p::custody_announce::tests::an_unsigned_announcement_is_refused_before_any_binding`
- `metrics::tests::custody_announce_directions_render_before_any_announcement`
- `p2p::gossip_dispatch::tests::custody_announce_projects_a_peer_announced_row_and_dedups_replay` (rewritten for the signed wire)

## Evidence (logs under `genesis/local-dev/regenesis-1/`)

Logs are local evidence (the directory is excluded from Git); every runner line pins `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim/dev`, `CARGO_BUILD_JOBS=1`, `RUSTFLAGS=--cfg getrandom_backend="custom"`.

| Step | Log | Result |
|---|---|---|
| focused tests (`--lib` filters custody_announce, gossip_dispatch, shard_service, metrics custody), rustup stable 1.99.1 | `r1-test-focused.log` | 45 passed, `EXIT=0` |
| clippy `--all-targets --features "p2p p2p-iroh" -D warnings`, toolchain 1.96.1 (first attempt) | `r1-clippy-iroh-1-red.log` | `redundant_closure` in `custody_announce::admit`, `EXIT=101` — fixed |
| clippy, same args, after the fix | `r1-clippy-iroh.log` | `EXIT=0` (lib, bins incl. `main.rs` dual wiring, tests, iroh integration tests) |
| `cargo test --lib` (default features), toolchain 1.96.1 | `r1-test-lib.log` | 4927 passed, 0 failed, 3 ignored, `EXIT=0` (4914 before + 13 new) |
| focused tests again, toolchain 1.96.1 | `r1-test-focused-1961.log` | 45 passed, `EXIT=0` |
| `cargo fmt --check` | `r1-fmt-check.log` | `EXIT=0` |
| RS-GOV-04 checker (`epr-rust-style --root`, binary from the rust-style gate slot) | `r1-rust-style-worktree.log` | elohim-storage debt 116 → 114, PASS, `EXIT=0` |

Ruling follow-up (relay source must not contradict), same runner and pins:

| Step | Log | Result |
|---|---|---|
| focused tests | `r1b-test-focused.log` | 48 passed, `EXIT=0` |
| clippy `--all-targets --features "p2p p2p-iroh" -D warnings` | `r1b-clippy-iroh.log` | `EXIT=0` |
| `cargo test --lib` | `r1b-test-lib.log` | 4930 passed, 0 failed, 3 ignored, `EXIT=0` |
| `cargo fmt --check` | `r1b-fmt-check.log` | `EXIT=0` |
| RS-GOV-04 checker | `r1b-rust-style-worktree.log` | debt 116 → 114, PASS, `EXIT=0` |

Two runs were aborted by R1 itself and kept for honesty: `r1-clippy-iroh-aborted.log` (started while another agent's cargo ran; restarted serially) and `r1-test-lib-aborted.log` (stopped to apply the clippy fix). The runner and queue are `r1-run.sh` / `r1-queue.sh` beside the logs.

## Not done here

- The orchestrator's household validation of afadfb5c9 (logs `genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/household-r1-*.log`): matthew received 7 / applied 4, jessica received 6 / applied 3, james sent 6, deliverability 7/7; the one refusal class (refused_binding 3, relay mismatch) is what the ruling follow-up removes. The follow-up itself has not been run on the household.
- `just gate elohim-storage` was not run as a unit (cold slot, other agents' cargo on the host). Its integration-test targets were compiled and linted by the iroh clippy run but not executed; `cargo test --lib` was.
- `just gate rust-style` was not run as a unit (its `--verify` re-tests the checker itself); the checker binary it builds was run directly.
