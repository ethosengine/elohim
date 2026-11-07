# Brief E3 — `serve-blob` mount with scope enforcement (Opus 5.5; integrity class)

Worktree `as-e3`, branch `artifact-store/e3`. Read `common.md`, then `design.md` §0 (F0.1, F0.2, F0.6–F0.9) and §1 in full. **§1 is your specification**; this brief only frames it.

**Write-set:** `elohim/eprfs/eprfs-peer/src/{gate.rs, tier.rs, daemon.rs, rules.rs, lib.rs, error.rs, local_blob.rs, serves.rs (new), blob_mount.rs (new)}`, `elohim/eprfs/eprfs-peer/tests/{serve_blob.rs (new), daemon.rs (the assertion at :968), rules.rs}`, `elohim/eprfs/eprfs-storage/src/tier_blob.rs` (append `pub async fn read_tier_manifest` only; E2 owns `decode_manifest`'s body), `elohim/eprfs/epr-cli/src/peer.rs:649-684` (the `serve` call site), `genesis/agentic/pool-policy.json` (`berth.artifacts.serves` only).

**Deliverables:** design §1.1–§1.5 exactly: `PeerConfig.payload_store`; `TierRequest::{Manifest, Blob}` and `TierAnswer::Blob` on the existing ALPN; `ServeScope` and the gate rules; `ArtifactServeList` inside `Rules::digest`; `BlobMount` with its flow-store index and the `serve-blob` `Use` writer (one event per requester-artifact session, no `measure:` slot); `custody_use_event` factored out of `stream_local_blob`; `LocalBlobContext.crossing`.

**Rulings you must not reopen:** strangers get a byte-identical uniform refusal for granted, ungranted and unknown roots; admitted devices get typed refusals; the grant is per method name; address resolution is not authorization; `admit_object` is called by the blob answer path itself because `eprfs-serve` never calls it.

**Tests:** every test named in design §1.6, plus the amended `tests/daemon.rs:968` and `tests/rules.rs::serves_enters_the_rules_digest_and_relays_still_do_not`. Receipt for the sprint: `an_admitted_sibling_asking_outside_its_grant_is_refused_with_a_typed_reason` green, then repeated across two live devices by the orchestrator.

**Gate:** `just gate eprfs` and `just gate orchestrator` (pool-policy). Expect sibling reuse to be vetoed on this branch by `siblingGovernanceChange`; that is correct. **Reviewer:** Codex astra read leg **and** the orchestrator's hunk review of `gate.rs`, `serves.rs` and the `blob_mount.rs` answer path.
