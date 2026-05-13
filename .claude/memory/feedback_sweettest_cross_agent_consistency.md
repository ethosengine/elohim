---
name: Sweettest two_agent_conductors needs explicit DHT consistency wait
description: Cross-agent must_get_valid_record calls in sweettest fail with "Failed to get Record" unless tests explicitly exchange_peer_info + await_consistency between conductors
type: feedback
originSessionId: c423684a-b162-42c6-b5cf-177683da9ed0
---
The `two_agent_conductors()` helper in `elohim/holochain/tests/sweettest/src/common/conductors.rs` spins up two SweetConductors but does NOT exchange peer info or wait for DHT consistency. Tests that perform cross-agent reads (e.g. Agent B calling `must_get_valid_record(action_authored_by_A)` via a coordinator) will fail with `Host("Failed to get Record …")` because gossip hasn't propagated.

**Why:** Two failures in feedback_signal sweettests during shift `2026-05-03T18-19-orchestrator-805` had this exact root cause: `retraction_by_non_author_rejected` and `create_vouch_succeeds_when_signer_differs_from_target`. Both Alice/Bob cross-agent calls timed out before reaching the actual coordinator gate, asserting on the wrong error message.

**How to apply:** Whenever a sweettest uses `two_agent_conductors()` AND involves a coordinator that calls `must_get_valid_record` / `must_get_action` against an action authored by the OTHER agent, add this between the create and the cross-agent call:

```rust
use holochain::sweettest::{await_consistency, SweetConductor};

tokio::time::timeout(std::time::Duration::from_secs(10), async {
    while !SweetConductor::exchange_peer_info([&ca, &cb]).await {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
})
.await
.map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

await_consistency(10, [&cell_a, &cell_b])
    .await
    .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;
```

Reference patterns: `imagodei_peer_binding.rs`, `epr_phase_2b_batch_a_e2e.rs`, and now `feedback_signal.rs` tests #4 + #8. If you're authoring a new cross-agent test, copy this scaffold up front.
