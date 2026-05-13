---
name: Imagodei has three distinct surfaces — social profile, self-knowledge, account management
description: Imagodei UX decomposes into three surfaces; M5 lands the third (account management) with Security & sign-in as its first pane
type: project
originSessionId: 4d20bf7b-4639-43d8-ad10-fccb514a7f0a
---
Imagodei is not a single page. It has three architecturally distinct surfaces, each with its own audience and access pattern:

1. **Social profile (public/private)** — intimacy-gradient view of a human shown to OTHERS. The default lens is computed from reach + qualitative peer/graph relationship (work/professional, family/personal, neighbor, collective member, collective participant). What surfaces in each lens is dynamic.

   **Profile is composite.** Beyond the imagodei core, **app-manifest-driven domains** (lamad, shefa, qahal, avodah, mishpat …) contribute their own profile surfaces — analogous to how YouTube and Google Maps Contributor compose facets onto your Google identity. Each app declares its profile surface via its manifest; the human's overall profile aggregates these composites and organizes them by domain.

   **Exists today** as `app/elohim-app/src/app/imagodei/components/profile/` with nine sections (agency, attestations, data, discovery, doorways, header, hosting, identity, network) — the imagodei core. Domain composites are scaffolded but not yet fully populated.

2. **Self-knowledge surface** — what the human aggregates ABOUT themselves through participation. Two kinds of preference data:
   - **Revealed preferences (behavioral)** — things the network has observed about you that you might not even know yourself.
   - **Stated preferences (declared)** — psephos psychometric results, personality quizzes, journals, system/governance preferences, "who I am" + "who I want to be."

   Together they form the **foundation of the human's contextual representation on the network**. The elohim loads this rich context to **represent the human's complex interests** in fully-automated sociocratic/holacratic governance and in value negotiations — anywhere, with any brought context, anytime. This is what makes elohim-as-counsel possible at machine speed.

   **Does not exist yet.** Not in M5 scope (filled in incrementally — psephos integration, journal capture, behavioral telemetry are separate sprints).

3. **Account management surface** — analogue to "Manage my Google Account." This is what the human sees when MANAGING their own presence:
   - Home
   - Personal info
   - Security & sign-in ← revocation UX, key management, recovery flows live here
   - Third-party apps & services (doorways registered with)
   - Data & privacy
   - People & sharing (relationships, emergency contacts)
   - Wallet & subscriptions (shefa)
   - Storage

   **Reachable from two surfaces:**
   - Through a doorway (when the human has lost their stewarded devices — recovery analog)
   - From their own steward devices (full peer-native management)

   **Does not exist yet in elohim-app.** doorway-app has `components/account/doorway-account.component.ts` which is the hosted-only view (account info, agency pipeline, graduation CTA). The peer-native account-management shell is M5's deliverable.

**Why this matters architecturally:**

- The third surface is where the OAuth-pattern handoff between hosted-doorway login and peer-native login becomes visible to the human (memory `project_peer_native_account_canonical_surface`).
- Recovery UX (M4 was invisible to humans) lands in the third surface's "Security & sign-in" pane.
- The defender (M5 backend) surfaces attestations into this pane — where the human can see "your elohim acted on your behalf" or "an anomaly is being investigated."

**How to apply:**

- Don't conflate the three surfaces. A "profile page" question is ambiguous — clarify which surface.
- M5's UX deliverable is Surface 3 (account management), starting with Security & sign-in. The other panes can be stubbed/placeholder for M5 and filled in later.
- Surface 3 must be reachable from BOTH a doorway (recovery context) AND a peer-native steward device. The route should be the same; only the auth flow differs.
