# EPR Protocol Layer — Startup Publication, Authorization, Recognition

**Date:** 2026-03-22
**Status:** Approved
**Scope:** Startup EPR Head publication for existing content, server-side reach authorization, recognition on delivery

## Problem

The EPR data plane (header + body sprints) can resolve and deliver content between peers. But three protocol-layer features are missing:

1. Content seeded before auto-publish has no EPR Heads in the DHT — it's invisible to the network
2. All content resolves regardless of reach — private content serves as freely as commons
3. No economic signal flows on delivery — stewards earn no recognition for content they maintain

## Design

### 1. Startup EPR Head Publication

On P2P node startup, after `db_pool` is wired and the swarm is listening, a one-time task publishes EPR Heads for all existing content.

**Trigger:** New arm in `run()` event loop, guarded by a `bool` flag (`initial_publish_done`). Runs once on first tick after startup.

**Flow:**
1. Query all content IDs from DB (`SELECT id FROM content WHERE app_id = 'lamad'`)
2. For each, call `resolve_epr_head_locally(id)` to construct the EPR Head
3. Send `P2PCommand::PublishEprHead` for each

**Steward population:** `resolve_epr_head_locally` currently sets `stewards: vec![]`. Change it to query `stewardship_allocations` via `get_content_stewardship()` and populate the shefa context with actual steward presence IDs and ratios. This feeds both DHT discovery and authorization.

**Adaptive rate limiting:** Start publishing at full speed. Monitor Kademlia put success/failure rate. If puts start failing or queuing, back off exponentially. Resume when success rate recovers. No fixed sleep intervals.

### 2. Server-Side Authorization (Reach Gates)

**Protocol changes:**

Add `agent_pubkey` to `EprRequest::Resolve`:
```rust
EprRequest::Resolve { id: String, agent_pubkey: Option<String> }
```

Add `AccessDenied` to `EprResponse`:
```rust
EprResponse::AccessDenied {
    required_reach: String,
    reason: String,
}
```

**Enforcement in `handle_epr_request(Resolve)`:**

1. Look up content's `reach` from DB
2. `"commons"` or `"public"` -> serve unconditionally
3. `"community"` or below -> require `agent_pubkey` in request
4. Map agent_pubkey -> human via `humans` table
5. Check `human_relationships` between requesting human and content's stewards
6. No qualifying relationship -> return `AccessDenied`
7. Relationship exists -> serve the EPR Head

**Requesting side:** `P2PHandle` needs access to the local agent pubkey (from `NodeIdentity`). Pass it through `P2PCommand::ResolveEpr` so the event loop includes it in `EprRequest::Resolve`. `resolve_and_fetch` sends it automatically.

**Reach hierarchy for authorization:**
- `commons`, `public` -> no check
- `community` -> any relationship with any steward
- `familiar` -> relationship with intimacy >= familiar
- `trusted` -> relationship with custody_enabled
- `intimate` -> mutual consent relationship
- `self`, `private` -> only the content creator

### 3. Recognition on Delivery

**When:** In the HTTP content GET handler, after `resolve_and_fetch` succeeds and content is persisted to SQLite.

**What:** One `EconomicEvent` per delivery:
- `action`: `"deliver"`
- `lamad_event_type`: `"CONTENT_DELIVERY"` (new, alongside existing CONTENT_VIEW)
- `provider`: primary steward's presence ID (from EPR Head shefa context)
- `receiver`: local agent's pubkey
- `content_id`: resolved content ID
- `resource_quantity_value`: `1.0`
- `note`: `"P2P EPR resolution"`

**Steward attribution:** EPR Head carries steward presence IDs and ratios (populated by Section 1). Event records primary steward as provider. Future: distribute recognition proportionally via existing `accumulate_recognition()` on `stewardship_allocations`.

**Failure handling:** Fire-and-forget. Recognition logging failure does not block content delivery or the HTTP response.

## Files Changed

| Action | File | What |
|--------|------|------|
| Modify | `src/p2p/epr_protocol.rs` | Add `agent_pubkey` to Resolve, add AccessDenied response |
| Modify | `src/p2p/mod.rs` | Startup publish task, authorization in handle_epr_request, steward population in resolve_epr_head_locally, agent_pubkey in commands |
| Modify | `src/http.rs` | Recognition event after P2P persist, handle AccessDenied |
| Modify | `src/db/economic_events.rs` | Add CONTENT_DELIVERY event type constant |

No new files. No new tables. No frontend changes.

## Risks

- **Startup publish floods Kademlia**: Adaptive rate limiting mitigates. 3,525 content nodes at ~500B each is ~1.7MB total — manageable.
- **Authorization blocks legitimate access**: Reach gates only apply to "community" and below. "commons" and "public" (the vast majority of genesis content) are unaffected.
- **Agent-to-human mapping fails**: If the requesting peer's agent_pubkey isn't in the serving peer's humans table, authorization defaults to deny for restricted content. This is correct — unknown agents shouldn't access private content.
