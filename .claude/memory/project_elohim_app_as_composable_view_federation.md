---
name: elohim-app is a composable federation of view modules, not a monolith
description: elohim-app today is a bootstrap/catch-all client; pieces should decompose into surfaces the human experiences regardless of whether the rendering host is their own tauri, their own browser-steward, or a peer rendering on their behalf
type: project
originSessionId: 4d20bf7b-4639-43d8-ad10-fccb514a7f0a
---
**Framing (2026-04-25):** elohim-app today is in essence a **bootstrap/catch-all client** — everything is in one Angular app because that was the fastest path to a working stack. The architectural future is **decomposition**: each pillar's surfaces (account, profile, learning, wallet …) graduate into composable view modules that the human can experience through any rendering host:

1. **Own tauri** — native, the human's own steward device.
2. **Own browser-steward** — the human's steward serving the app over HTTP.
3. **Peer-rendered** — a trusted peer's elohim-app instance renders the surface on the human's behalf when their own devices are unreachable. Peer is BLIND to secrets (per `project_socially_derived_security`); secrets flow through socially-derived primitives, not through the rendering host.

**M5 graduates the FIRST module — the account/auth pillar — to this composability shape.** That's the canonical example. Other pillars follow in their own sprints.

**What composability means concretely (pragmatic, not microfrontend-architecture):**

- Pillar has a clean public API. Cross-pillar dependencies go through `storage-client-ts`, not through Angular service imports across pillars.
- Pillar's routes are self-contained and lazily loadable.
- Pillar's services have no hidden coupling to global app state beyond a documented interface.
- Pillar can be embedded inside another elohim-app instance OR rendered as a discrete URL surface.

**What graduating the auth portal to mesh-renderable solves:**

- Browser-side handoff isn't a new service — it's the same portal accessed from a different rendering host.
- "Manage from your steward" isn't a deep link — it's a portal-host discovery + redirect to the human's preferred portal-host.
- "Peer renders portal when your devices are gone" is the recovery flow AND the borrowed-device flow.
- The human's portal is keyed to THE HUMAN, not to a particular app instance. Rendering host is incidental.

**What stays out of M5 even with this graduation:**

- Full microfrontend build artifact splitting. M5 ships clean module boundary; physical bundle separation is later.
- Full peer-render UX with availability handoff. M5 ships the discovery primitive + bootstrap-mode (own steward renders); peer-render UX is an extension that the architecture doesn't preclude.
- Other pillars graduating (learning, wallet, etc.) — that's their own sprints.

**How to apply:**

- When designing any pillar surface for the next ~10 sprints, ask: does this support being rendered by a peer? If no, what would change? If yes, what's the discovery + auth-flow shape?
- Cross-pillar imports across services are a smell. Fix at the storage-client boundary.
- "Where does this render?" should be a runtime configuration, not a build-time assumption.
