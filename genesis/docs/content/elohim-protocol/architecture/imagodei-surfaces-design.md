---
title: Imagodei — the three identity surfaces
id: imagodei-surfaces
tier: architecture
status: Design
created: 2026-06-03
authors: Matthew Dowell + Opus 4.8
pillar coupling: imagodei (identity core), doorway (web2 projection + recovery web-path), elohim (defender attestations into the management surface)
realizes:
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (intimacy-gradient identity)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md (the OAuth relying-party handoff the management surface makes visible)
  - genesis/docs/architecture/cradle-to-grave-capability-gradient.md (the capability gradient the surfaces sit on)
informs:
  - imagodei pillar UX work; recovery UX; defender attestation rendering
memory_anchors:
  - project_imagodei_three_surfaces
  - project_peer_native_account_canonical_surface
  - project_elohim_as_counsel
defers:
  - the self-knowledge surface's wiring (psephos integration, journal capture, behavioral telemetry are separate deliveries)
---

# Imagodei — the three identity surfaces

Imagodei is **not a single page.** It decomposes into three architecturally distinct surfaces, each
with its own audience, access pattern, and source of truth. Conflating them is the recurring design
error: a "profile page" question is ambiguous until you name which surface. They are:

1. **Social profile** — what OTHERS see of a human.
2. **Self-knowledge** — what the human aggregates ABOUT themselves through participation.
3. **Account management** — what the human sees when MANAGING their own presence.

---

## 1 · Social profile — the intimacy-gradient view

The social profile is an **intimacy-gradient view shown to others.** The default lens is *computed*,
not chosen: from the viewer's earned reach plus the qualitative peer/graph relationship
(work/professional, family/personal, neighbor, collective member, collective participant). What
surfaces in each lens is dynamic — a colleague and a sibling see different facets of the same human
without the human curating two profiles.

**The profile is composite.** Beyond the imagodei core, **app-manifest-driven domains** (lamad,
shefa, qahal, avodah, mishpat …) contribute their own profile surfaces — the same way YouTube and
Maps-Contributor compose facets onto a Google identity. Each app declares its profile surface via its
manifest; the human's overall profile aggregates these composites and organizes them by domain. The
imagodei core owns the identity sections (agency, attestations, data, discovery, doorways, header,
hosting, identity, network); the domain composites attach to it rather than the reverse.

## 2 · Self-knowledge — the human's contextual representation

The self-knowledge surface is what the human aggregates about themselves through participation. It
holds two kinds of preference data:

- **Revealed preferences (behavioral)** — what the network has observed about the human, possibly
  things they do not know about themselves.
- **Stated preferences (declared)** — psephos psychometric results, personality instruments, journals,
  system/governance preferences, "who I am" + "who I want to be."

Together these form the **foundation of the human's contextual representation on the network.** This
is the load-bearing reason the surface exists: the elohim loads this rich context to **represent the
human's complex interests** in fully-automated sociocratic/holacratic governance and in value
negotiations — anywhere, with any brought context, anytime. The self-knowledge surface is what makes
*elohim-as-counsel at machine speed* possible; without it the elohim has nothing to represent.

## 3 · Account management — "manage my presence"

The account-management surface is the analogue to "Manage my Google Account" — the panes through which
a human manages their own presence:

- Home
- Personal info
- **Security & sign-in** — revocation UX, key management, recovery flows live here
- Third-party apps & services (the doorways registered with)
- Data & privacy
- People & sharing (relationships, emergency contacts)
- Wallet & subscriptions (shefa)
- Storage

**This surface is reachable from two access paths, and the route is the same — only the auth flow
differs:**

- **Through a doorway** — when the human has lost their stewarded devices (the recovery analog). The
  doorway implements the *web access* path; it facilitates and validates against the steward's
  peer-native identity without owning it.
- **From their own steward devices** — full peer-native management.

This is **the canonical surface where the OAuth-pattern handoff becomes visible to the human.** The
doorway is an OAuth relying-party: pre-graduation it is both relying-party and identity-provider for a
transitional hosted identity; post-graduation it permanently loses identity authority and backs auth
with the peer-native IdP (see `2026-05-23-doorway-access-tier-patterns.md`, "Tier 2 → Tier 3 is one
identity trajectory"). The account-management surface is where that trajectory is rendered to the
person living it.

---

## Why the three-surface split is load-bearing

- **Recovery lands here, not as a separate app.** Recovery UX is not its own surface — it is the
  "Security & sign-in" pane of account management, reached via the doorway web-path when devices are
  lost. Building recovery as a standalone flow re-fragments what this architecture deliberately unifies.
- **The defender writes into account management.** The elohim defender (which reads the imagodei
  profile deeply) surfaces its attestations into this surface — "your elohim acted on your behalf,"
  "an anomaly is being investigated." Account management is the human-facing window onto the elohim's
  protective action, so attestation rendering targets this surface.
- **Each surface has a different source of truth.** Social profile is a *projection* gated by reach;
  self-knowledge is *aggregated* from behavioral + declared streams; account management is *authoritative
  control* over keys, doorways, and sharing. A change that treats them as one page will route a control
  action through a projection, or expose authoritative management state to a reach-gated viewer.

## How to apply this

- When a request says "the profile page," **clarify which of the three surfaces** before designing.
- The account-management surface (Surface 3) must be reachable from **both** a doorway (recovery
  context) and a peer-native steward device; design the route once, vary only the auth flow.
- Domain profile composites attach via app manifest — never hard-code a pillar's profile section into
  the imagodei core.
