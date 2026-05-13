---
name: Storage as actor vs forwarder — two distinct HcClient patterns
description: elohim-storage's existing HcClient is service-bot pattern (storage signs as itself); Phase 11 introduces the first "storage forwarding on behalf of human" use case which has different auth/cell-routing requirements
type: project
originSessionId: e208c11f-36b4-47a5-a45e-0dff7060161d
---
elohim-storage has an existing `HcClient` (`elohim/elohim-storage/src/hc_client.rs`) that wraps `holochain_client::AppWebsocket` with admin-issued signing credentials. All current consumers use it as **service-bot pattern** — storage acting on its own behalf:

- `heartbeat.rs` — storage records its own infrastructure peer status
- `import_handler.rs` — storage publishes content as itself
- `node_registry_api.rs` — storage registers itself
- `content_server.rs` — storage publishes content as itself

These all sign with storage's own admin-issued credentials and the `agent_info()` in the zome correctly returns storage's identity (which is what those zomes want).

**Recovery M5 introduced the first "storage forwarding on behalf of a human" use case.** When a human submits self-revocation via `POST /api/v1/account/self-revocation`, the imagodei zome's `agent_info()?.agent_initial_pubkey` MUST return the human's pubkey, not storage's — otherwise self-revocation semantics break ("I revoked my own key" requires the zome to see the human as caller).

Two deployment modes constrain the answer:

| Mode | Conductor topology | Cells | Storage's existing HcClient sufficient? |
|------|--------------------|-------|----------------------------------------|
| Tauri-direct sidecar | Local | ONE (the human's) | Possibly — needs provenance probe |
| Browser via doorway | Hosted multi-tenant | MANY (one per human) | No — needs per-cell auth pool |

**Why:** This is a real architectural distinction that affects how Phase 11 is scoped (and any future storage→conductor write paths on behalf of humans). The candidate approaches are documented in the Phase 11 kickoff prompt: per-cell HcClient pool keyed on agent_pub_key, OR Tauri-only first then graduate, OR move write path off storage onto doorway entirely.

**How to apply:** When designing any new storage HTTP route that mutates DHT entries owned by a specific human (not storage itself), classify it as "forwarder" not "actor" and reach for the multi-tenant routing approach Phase 11 lands. When designing routes where storage IS the rightful actor (infrastructure heartbeats, content publication by storage, node registration), reuse the existing single-cell HcClient pattern. The question to ask: whose `agent_info()` should the zome see — storage's or the calling human's?

Cross-reference: `feedback_serde_json_value_breaks_zome_boundary` (pre-stringify convention for structured payloads on this same bridge), `project_three_layer_truth_model` (storage is libp2p data-ops layer, not P2P-write-canonical for human-owned entries), `sign_for_agent` zome function added in EPR Phase 2B Task C.1 (related but distinct pattern: caller signs themselves, not "service forwards on behalf of").
