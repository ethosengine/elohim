---
title: "History/ADR: Conductor agent-info substrate gossip (step zero)"
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [conductor, agent-info, gossip, doorway, kitsune2, federation, peer-cache]
# DISTILLS a landed-and-merged substrate-gossip step (c88febe93). Code merged + 14 tests
# behind ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP (default false); operator soak HELD — runtime
# stability NOT asserted. Raw design+plan retire to git.
distills:
  - genesis/docs/superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md
  - genesis/docs/superpowers/plans/2026-05-28-conductor-agent-info-substrate-gossip.md
canonical:
  - ../../../../../.claude/memory/project_multi_doorway_human_registration.md   # conductor peer-cache now substrate-warmed
memory_anchors:
  - project_multi_doorway_human_registration
  - project_doorway_single_target_no_fanout
  - project_three_layer_truth_model
  - project_inventory_exchange_not_byte_replication
---

# Conductor agent-info substrate gossip ("step zero")

> **One-sentence lesson:** Step-zero is *necessary but not sufficient* for cross-doorway EPR delivery —
> it propagates DHT entries cross-mesh but does not project them on remote pods. Warming the conductor
> peer cache lets gossip reach across two signal servers without the signal servers federating.

Phase-1 per-human doorway routing split the alpha cluster's kitsune2 signaling mesh along the doorway-A /
doorway-B boundary (each Holochain conductor registers at exactly one `signal_url`; kitsune2 has no
multi-bootstrap schema). To keep the two halves discoverable, every `elohim-storage` pod publishes its
embedded conductor's own `AgentInfoSigned` JSON strings (`admin_ws.agent_info(None)`, filtered to self
via `list_cell_ids`) on the gossip topic `elohim/conductor/agent-info/v1` over the existing
`DualGossipPublisher`; every other pod subscribes (libp2p edge → bounded mpsc → rate-limited worker →
batched `admin_ws.add_agent_info`) and injects into its conductor's peer cache. The substrate is pure
transport — the conductor stays authoritative for signature-verify + dedup; the kitsune2 signal URL is
registration-only, so a warmed peer cache lets outbound gossip reach across both signal servers without
the signal servers federating. The payload (`ConductorAgentInfo`) is Category-C operational: never
stored, reconstructed by the next 60s heartbeat; no DHT entry type, no diesel table, no HTTP route.

**Landed-by-evidence (soak HELD).** Merged to dev (`c88febe93`; module
`elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs`, 14 unit tests, dual-publish catalog row
#13, byte-parity test, a2o feature), behind `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` (default false) pending
operator matthew+adam soak. *Code is merged; the 24h runtime soak is outstanding — this record does NOT
assert runtime stability.*

**Watch-out.** Step-zero is necessary but not sufficient for cross-doorway EPR delivery — it propagates
DHT entries cross-mesh but does not project them on remote pods (Holochain `post_commit` fires
local-only), so the sibling gaps F1 (remote-receive projection visibility) + F2 (Jenkins seeds one
doorway) + F4 (blob reachability) must close too; those are tracked in
`genesis/docs/superpowers/plans/2026-05-29-substrate-shakeout-epr-delivery-sprint.md`. Explicitly
deferred to Phase 12+: cold-start with signal_url down (substrate WebRTC signaling), multi-URL
agent_info fallback, session.doorway_url single-pin refactor, substrate-native signaler, iroh
subscriber-side wiring (behind Plan 4 Task 8). The publisher own-key filter uses a substring match
against kitsune2 v2 agent_info JSON — re-confirm against a live `agent_info()` return on any
`holochain_client` major bump.

## Bidirectional links

- **This record → canonical:** the [`project_multi_doorway_human_registration`](../../../../../.claude/memory/project_multi_doorway_human_registration.md) memory entry (conductor peer-cache layer now substrate-warmed — first of three federation-wiring-audit layers closed) + the dual-publish catalog at `elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md` row #13.
- **Distilled-from (raw design+plan in git history):** conductor-agent-info-substrate-gossip design + plan (linked in frontmatter).
