---
name: Views are served THROUGH a doorway, never owned BY one (CDN-shape)
description: The EPR pattern says any view should be servable from any doorway projecting the same canonical content. Doorways are CDN edges, not authorities. DNS bonding records federate two doorway addresses into a CDN layer for the same canonical content. Implementers are tempted to write doorway-specific route handlers with business logic in doorway-service; this is the anti-pattern.
type: feedback
originSessionId: 155036b0-387a-441c-91c5-7a1333fb2f07
---
**Rule:** When adding a new view or route, the design must answer "which doorway address could a client swap to and get the same content?" If the answer is "none, this doorway authored the response," it is a doorway anti-pattern.

**Why:** Doorway's mandate is web2 projection, not authority. The protocol's capture-resistance depends on doorways being interchangeable CDN edges over canonical substrate content. DNS bonding lets two doorway hostnames federate as a CDN layer for the same content; if a doorway authors content, the bonding becomes a lie. See `project_doorway_manifest_driven_routes`, `project_doorway_single_target_no_fanout`, `project_three_layer_truth_model`.

**How to apply:**
- New endpoint surfacing substrate state → declare it in elohim-storage's `build_manifest()`. Doorway proxies it via the route registry. ZERO doorway code change. This is the default.
- Doorway-specific Operational/Category-C state (cache stats, federation peer list, public-surface DNS/TLS health) IS legitimate doorway-resident state — but the view CONTRACT is shared (schema lives at `elohim/sdk/schemas/v1/views/`) so a sibling doorway can serve its own equivalent. The contract is canonical; the values are local.
- Anti-pattern smells in plans / PRs:
  - "Add `routes/<thing>.rs` to doorway with hand-rolled aggregation logic" → wrong unless it's doorway-local Operational state
  - "Doorway iterates peers / fans out / decides which storage holds the bytes" → forbidden per `project_doorway_single_target_no_fanout`
  - "Federation peer A asks doorway B for canonical content B authored" → no doorway authors canonical content
  - "Per-DNA proxy file" → forbidden per doorway/CLAUDE.md (we deleted 13 of these)
- Audit any plan with new doorway routes: every route should either (a) proxy to elohim-storage via manifest, or (b) surface explicitly doorway-local Operational state with a canonical schema sibling doorways could implement.
- Federation pattern is DNS bonding + content-addressed cache convergence, not "this doorway is authoritative for these CIDs."
