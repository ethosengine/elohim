---
name: Doorway as peer registration point, not just proxy
description: Doorway serves as inside-out peer registration — peers register content/capabilities with doorway for discovery, inverting Holo's outside-in marketplace model.
type: project
originSessionId: 63499c63-1cde-41b5-a0b0-66503d4c008c
---
Doorway is more than a conductor proxy/gateway. It's a peer registration point where peers on the network announce "I have this content, I can serve it." This inverts Holo Host's model:

- **Holo (outside-in):** developer → marketplace → hosts opt-in → users consume
- **Elohim (inside-out):** peer stewards content → registers with doorway → doorway surfaces to browsers/peers

**Why:** The content IS the app. Registration is peers telling doorway what they've got. This is functionally what Holo's HHA (Hosting App) does, but from the peer's perspective, not a developer marketplace.

**How to apply:** When designing doorway APIs or storage-to-doorway communication, remember doorway needs to know each peer's content inventory, capability level, and availability — not just proxy conductor WebSocket calls. The registration flow is partially built but not complete. This also means the storage↔doorway API is richer than just conductor proxying.

**Status (2026-04-16):** Intent is clear, registration not fully implemented yet. Keep in mind when designing the elohim-node consolidation — if storage and conductor merge into one process, doorway's relationship to that process encompasses both conductor proxying AND content registration.
