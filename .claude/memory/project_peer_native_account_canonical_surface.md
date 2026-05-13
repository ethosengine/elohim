---
name: Account layer — graduation-based login supersession (shape, not committed design)
description: Initial shape for the account layer; hosted-doorway login for unsgraduated humans, peer-native login once graduated to peer-steward; peer-native supersedes hosted; doorway then facilitates browser VIEW only, not auth
type: project
originSessionId: 253292ea-69ea-4e76-86e3-6d87ebdac46c
---
The account layer is not yet designed, but the **current shape** (as of 2026-04-24, not yet committed) is graduation-based supersession:

1. **New / hosted human** — lives fully inside a doorway's hosted conductor. Doorway-hosted login is the auth path. Their cell runs on the doorway-steward's infrastructure. Browser-only UX.

2. **Graduation to peer-steward** — the human acquires/operates a peer-native device (tauri steward / Moss / Launcher). Their identity migrates to being peer-native. This is a stewardship lifecycle stage — graduated capability in the stewardship philosophy sense.

3. **Post-graduation browser access** — a peer-steward who opens a browser and visits a doorway **does not use doorway-hosted login**. Instead: doorway **facilitates the browser-based view**, but authentication flows through the peer-native login. Peer-native supersedes hosted.

**What this means architecturally (the OAuth analogy):**

Doorway's relationship to identity is analogous to an OAuth **relying party** that presents the login portal but does not own it. Websites (doorways) present OAuth logins; identity providers (peer-native infrastructure) own them. This analogy is the canonical mental model.

- **Pre-graduation (hosted):** the doorway serves as both relying party AND identity provider, because there is no peer-native identity yet. Doorway has agency over the human's identity in this transitional state.
- **Post-graduation (peer-steward):** the doorway is only the relying party. The login portal it presents is backed by the peer-native identity provider. Doorway **never owns** the peer-steward's login portal — it can only present it.
- **Doorway agency over identity is strictly bounded** to the hosted state. Once a human graduates, doorway permanently loses identity authority over them; it retains only the view/proxy role.

This is consistent with the three-layer truth model: doorway is web2 projection; peer-native (via DHT + libp2p mesh) is where identity and state authoritatively live post-graduation.

**Why this shape works:**

- Matches the stewardship graduation pattern (new → stewarded → graduated to stewarding others).
- Preserves the "doorway is optional" principle — a peer-steward can use their own peer-native client or someone else's doorway-as-view interchangeably.
- Avoids the design trap of two parallel-forever auth systems that have to be kept in sync.

**What's still open:**

- ~~The actual peer-native login mechanism (Moss / Launcher integration? bespoke?).~~ **Resolved 2026-04-25:** elohim-app itself fills the role holochain-launcher / moss-launcher would serve today. The p2p-native OAuth portal is rendered either (a) directly from the human's own steward device running elohim-app, or (b) by their trusted peer network rendering it on their behalf (mesh-rendered IdP — meaningful for recovery/availability when the steward device is unavailable).
- How graduation happens as a ceremony (when does a hosted human's identity migrate to peer-native?).
- How doorway detects "this browser session is a returning peer-steward, use peer-native auth handoff" vs. "this is a fresh hosted human, use doorway-hosted login." (Likely via `peer_identity_bindings` projection table from EPR 2B Batch A.)
- Whether the hosted-login path is ever upgraded to some richer form, or remains the transitional shim for unsgraduated users.
- The libp2p ↔ browser bridging mechanic for the actual session handoff (identity-handshake is libp2p; browser-side needs translation).

**How to apply:**

- M3 and earlier ship DNA + mesh + storage plumbing that is initiator-agnostic. No login-layer code.
- When the account layer is designed, use this graduation-based shape as the starting point — don't rediscover it.
- Don't design "hosted account" and "peer-native account" as two parallel forever-separate subsystems; they're stages of one trajectory.
- Flag any design in any phase that forecloses graduation (e.g., ties identity indissolubly to doorway-hosted infrastructure).
