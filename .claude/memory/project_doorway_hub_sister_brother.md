---
name: Doorway and hub as symmetric projection edges (web2 / peer-native)
description: Doorway projects substrate truth outward to web2 (CDN, DNS, OAuth-relying-party for browsers). Hub projects substrate truth inward to nearby peers (school-laptop syncing a Khan library to student devices when they arrive). They are symmetric projection edges, not the same thing. Both project the same canonical DHT/libp2p truth; they differ in audience and reach contract.
type: project
originSessionId: 155036b0-387a-441c-91c5-7a1333fb2f07
---
The four-surface model:

- **DHT** — protocol notary (Category A entries, content-addressed).
- **libp2p** — data-ops swarm; `elohim-storage` is the canonical participant.
- **Doorway** — web2 projection edge: CDN/DNS/TLS/OAuth-relying-party. Browsers, federation peers (other doorways), AT Proto / ActivityPub interop. Doorway is NOT a P2P participant.
- **Hub** — peer-native projection edge: aggregates substrate truth for nearby peers (same household, school, village). The teacher-laptop hosting a library that student devices sync from when they walk in. Hub IS a P2P participant.

**Why:** Until this sprint, "doorway" was crowding all projection concerns. The hub framing splits them by audience: doorway-shaped projection serves browsers and other doorways; hub-shaped projection serves nearby peers. Same substrate truth, different projection contract.

**How to apply:**
- Doorway federates to other doorways (DNS bonding, federation registry). Hub federates to other hubs (peer-native, libp2p-shape).
- A village's hub MAY peer with a doorway when the village wants a public web2 face — but the hub stands alone without one.
- View contracts (cluster-view, peer-topology-view, reciprocity-view, distribution-view, doorway-dashboard-view) should be serveable from BOTH surfaces where it makes sense. Don't bake a doorway-only assumption into a view that's also valid hub-side.
- When designing a new projection feature, ask: "Is this serving browsers + other doorways, or nearby peers? Or both?" The answer locates the work.
- Pairs with `project_doorway_views_through_not_owned`, `project_substrate_scale_ceiling`, `project_three_layer_truth_model`, `project_elohim_hub_elevation`, `project_hub_archetype_abstraction`.
