---
name: Doorway routes are declared in app manifests, not coded in doorway
description: App manifests declare HTTP routes; doorway reads manifests and acts as a registry-driven proxy to the right elohim-storage instance. elohim-storage CID/DAG routing is the network source of truth.
type: project
originSessionId: 6ec4bfae-b3f0-4040-8a90-6ae504910fe7
---
Doorway is not a per-route Rust coder. It is a **registry-driven proxy/load-balancer**.

**The architecture:**

1. Each app manifest (e.g., `elohim/sdk/domains/lamad/manifest.json`) declares the HTTP routes the app exposes.
2. Doorway reads app manifests at startup (or reload) and builds a route table.
3. When a request arrives, doorway looks up the manifest-declared route, identifies which elohim-storage instance serves it (via CID/DAG routing on the network), and proxies the request.
4. elohim-storage handles the actual query — it is the source of truth for what content addresses map to which data.

**Why:** This keeps elohim-storage CID/DAG routing as the network's source of truth. Doorway's job is to translate web2 requests (HTTP URL paths, query parameters) into the protocol's content-addressed routing, then proxy/load-balance to the right peer. Adding a new domain route (e.g., `GET /api/gate-decisions/{cid}`) should be a MANIFEST change, not a doorway code change.

**How to apply:**
- When adding a new HTTP route to the protocol: declare it in the relevant app manifest (`elohim/sdk/domains/{domain}/manifest.json`); do NOT add a doorway handler.
- The elohim-storage side must expose the underlying query function (typically already present for internal storage projections).
- Doorway's manifest-reading/route-registration mechanism is the plumbing — verify it exists before declaring the route.

**Direct doorway Rust code changes are reserved for:**
- Federation (peer discovery, cross-community routing)
- CDN (caching layer)
- DNS (DNS-over-HTTPS, human-readable names → CIDs)
- Bootstrap (agent discovery)
- Signal (WebRTC signaling)

Anything that serves app-domain data (gate decisions, content nodes, attestations, economic events) should flow through manifest-declared routes.

Flagged 2026-04-19 during Phase 4. Corrects the default assumption that "new API endpoint = new doorway Rust handler."
