---
id: project-storage-signal-subscribers-filter-by-envelope
name: storage-signal-subscribers-filter-by-envelope
description: "elohim-storage signal subscribers filter by envelope type — new ProjectionSignal variants from the DNA need explicit subscribe_* methods wired in main.rs, not just receiver function impls"
metadata: 
  node_type: memory
  type: project
  originSessionId: de3918e5-34fe-4832-bc41-5a3ca933e9fc
cites:
  - elohim/elohim-storage/src/hc_client.rs
---

elohim-storage's conductor signal subscribers (`HcClient::subscribe_*` methods + matching `tokio::spawn` blocks in `main.rs`) each filter incoming signals by attempting to deserialize as a specific envelope type. Signals that don't match are logged at debug and dropped.

As of 2026-05-26, the wired subscribers are:
- `subscribe_infrastructure_signals` → filters to `InfrastructureSignal` (PeerStatus projection)
- `subscribe_elohim_content_signals` → filters to `ElohimContentSignal` (attestation:* + governance-action:* via elohim_content_dispatcher)
- `subscribe_rea_projection_signals` → filters to `ReaProjectionSignal` (REA commitments/agreements/economic events; ADDED 2026-05-26 in commit fcfc6069c, prior to that the rea_projection::handle_rea_signal was dead code)

**Why:** Without an explicit subscribe_* method matching a variant, a fully-implemented receiver function in elohim-storage (e.g. `rea_projection::handle_rea_signal`) is dead code. The function compiles, greps find it, but no signal ever reaches it. Symptom: conductor write path succeeds at DHT level but storage's SQL projection never lands; service-layer bounded polls time out.

**How to apply:** Whenever adding a new ProjectionSignal variant from the elohim DNA's content_store post_commit handler (e.g. ContentCommitted, PathCommitted), the migration includes a new (or extended) subscribe_* method. Either add a brand-new subscriber, or extend an existing one's envelope enum + handler arm. The pattern at `subscribe_rea_projection_signals` is the template (see `elohim/elohim-storage/src/hc_client.rs:440`-ish + `main.rs:651`-ish).

See [[project_three_layer_truth_model]] for the broader DHT=notary / libp2p=data-ops / doorway=projection scoping.
