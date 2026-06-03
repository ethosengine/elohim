---
id: qahal-homepage-ux-design
project: elohim-protocol
type: ux-design-spec
status: design — checkpoint-A output
created: 2026-05-22
gates: Sprint 1 — Qahal homepage UX exploration
spec: 2026-05-21-qahal-architecture-vision.md            # companion vision this UX exploration realizes
related: 2026-05-21-qahal-mvp-roadmap.md                 # companion roadmap
---

# Qahal Homepage UX Design

> **Checkpoint A output.** This document is the result of the Sprint 0 → Sprint 1 brainstorming checkpoint. It captures the UX decisions Sprint 1 (graphos-designer + component-architect) will implement as Library A elohim-elements + Library B pattern stories. Sprint 1 deliverables and the writing-plans handoff sit at the end of this spec.

## How to read this spec

- **Sections 1-2** carry the framing and the chrome layout. Read these first.
- **Sections 3-5** detail the 9 elohim-core panels, the 5 deep-implementation MVP panels, and the configurable resource list.
- **Sections 6-7** carry the provenance categories with capability gating + the mock-data fixture structure.
- **Section 8** specifies the simple → power-user toggle mechanism.
- **Section 9** maps architectural connections to existing memory + spec sections.
- **Section 10** specifies Sprint 1 deliverables for the writing-plans handoff.
- **Section 11** carries open questions forward to Sprint 2 schema work.

## Section 1 — Frame: the Qahal homepage as social garden tending surface

The Qahal homepage is where primary stewards come to **tend their social garden**. Matthew tending the Dowell household. Sheila tending her household + her podcast audience + her compute-stewardship relationships. Gertrude tending care, presence, and her compute-share for the Dowells across the continent. Brother Cal tending the congregation. The Hardins tending the Tuesday-evening life-group at three years of cohesion.

Tending is the right verb. The homepage is not a control panel. It is not an admin console. It is not a dashboard in the metrics-and-KPIs sense. It is the surface where a primary steward sees **all the valuable contributions of people in care and compute in their very-real and meaningful network**, and can attend to what wants attending.

This framing is load-bearing for the design discriminators that follow:

- **"Very-real and meaningful"** — the homepage refuses abstract metrics like flat membership counts. The Hylo "1176 members" is replaced by stratified standing tiers (active stewards · contributor presences · compute-hosting stewards) that show *what people actually do for this network*. Aggregate numbers without participation texture are dishonest about the substrate's nature.
- **"In care AND compute"** — the homepage makes BOTH visible. Care-economy contributions (the household's commons stream; Sheila's recipe arriving from across the continent) AND compute-stewardship contributions (Gertrude's hub holding the household's recovery share). Both are first-class. Both are stewardship.
- **"Tending a garden"** — the operator does small, attentive, recurring acts of attention. The homepage doesn't push notifications, doesn't gamify, doesn't compete for time. It is what you open when you want to know how your garden is doing.

The household homepage is the **living core** (cf. vision spec Section 1.2) — the place where the protocol becomes embodied for ordinary people, where lived contrast forms common sense, where the substrate spreads by being lived rather than by being persuaded. Other Qahal types (faith community, life-group, wisdom commons, and the Tier 1-3 catalog items downstream) render the same garden-tending surface at different scales. The chrome and primitives are the same; the rubric, content, and tier-distribution shift with the collective.

## Section 2 — The Hybrid 4-column chrome

The full chrome at desktop width is four columns. On narrower viewports it collapses progressively: the far-left switcher becomes a hamburger; the right context-column becomes a toggleable drawer; the per-Qahal sidebar can be hidden via the simple-user toggle.

```
┌─────┬─────────────────────────┬──────────────────────────┬───────────┐
│ All │ Dowell Household        │ Stream                   │ Context   │
│Qhls │ ─────────────────────   │ ──────────────────────   │ ────────  │
│ 🏠  │ ▼ Protocol panels       │                          │ Rules     │
│ ⛪  │   Home (stream)         │  [Commons stream]        │ ──────── │
│ 🪨  │   Members               │                          │ Co-steward│
│ 🌳  │   Rules                 │  Yesterday: James sick   │ "Household│
│  +  │   Co-steward            │  Sheila's soup           │  is       │
│     │   Social-Compute        │  Gertrude checked in     │  steady"  │
│     │ ─────────               │                          │ ──────── │
│     │ ▼ Curated EPRs ◆        │  [Member ring · 5]       │ Discovery │
│     │   ◆ Family Recipes      │                          │           │
│     │   ◆ Birthday calendar   │                          │           │
│     │   ◆ Sick-day playlist   │                          │           │
│     │ ─────────               │                          │           │
│     │ ▼ External ⤤ (parent-   │                          │           │
│     │           visible only) │                          │           │
│     │   ⤤ Family Google Doc   │                          │           │
│     │     (offline grey)      │                          │           │
│     │ ─────────               │                          │           │
│     │ ▼ Power-user toggle ⓘ   │                          │           │
│     │   Standing inspector    │                          │           │
│     │   Shefa resources       │                          │           │
│     │   Attestations          │                          │           │
│     │   Graph discovery       │                          │           │
└─────┴─────────────────────────┴──────────────────────────┴───────────┘
```

### Column responsibilities

| Column | Width | Content | Behavior |
|---|---|---|---|
| **Collective switcher** | narrow (60-80px) | Icons for each Qahal the operator participates in (household, congregation, life-group, wisdom commons, etc.) + a `+` to join/create | Click switches active Qahal. Currently-active Qahal is visually elevated. The icon visually encodes the Qahal type (🏠 household, ⛪ faith community, 🪨 wisdom commons, 🌳 natural collective, etc.) |
| **Qahal sidebar** | 200-260px | Per-Qahal sidebar — protocol panels + curated EPRs + external links + power-user-toggle expandables. Configured by the Qahal's rubric. | Sections are collapsible. Active item is visually elevated. Simple-user toggle hides the power-user section. |
| **Main viewer** | flex-1 (largest) | The active panel's rendered content. Default: the stream panel ("Home"). | Switches when sidebar item is clicked. State persists per-Qahal. |
| **Context column** | 240-300px | Right-nav contextual panels: rules summary, co-steward's view ("the household is steady"), graph-discovery suggestions. | Persistent within the active Qahal. Co-steward view is always visible; rules + discovery are collapsible. On narrow viewports collapses to a drawer. |

### Convergent UX heritage

The chrome converges with Matrix-modern, Slack, hybrid Hylo, Discord (when their "in-server" view is open). The four-column shape is what these apps converge on when carrying the most axes natively — collective-switching + within-Qahal navigation + main content + persistent context. The protocol's discriminator is in the *content* of each column (provenance categories in the sidebar; co-steward voice in the right; tier-aware member-ring in the main panel), not in the layout shape.

## Section 3 — The 9 elohim-core panels

Every Qahal has access to all 9 protocol panels. The Qahal's rubric decides which panels surface at simple-user tier vs power-user tier. None of the 9 can be removed by Qahal configuration — they are the protocol's foundational expression at the UI layer.

| # | Panel | Tier | What it carries |
|---|---|---|---|
| 1 | **Stream** | Simple | Commons stream — value-scanner observations, care-economy REA events, household/collective happenings. The home page when you open the Qahal. |
| 2 | **Member-Ring** | Simple | Tier-aware drill-down of participants — network reach headline + stratified tiers (active stewards · governance · contributor presences · compute-hosting stewards) + imagodei-lensed individual views |
| 3 | **Rules** | Simple | Qahal rubric — the standing curve, attestation requirements, friction-gradient thresholds, commons-elohim configuration. Steward-tier capability can edit. |
| 4 | **Co-Steward** | Simple | Commons-elohim co-steward's reflective view — "the household is steady"; "the congregation's reach into the neighborhood is rising slightly"; ambient witness without pushiness. Lives in the right-context column. |
| 5 | **Social-Compute** | Simple | Compute-stewardship topology — who hosts our state; who we host for; replication health; recovery readiness; Shamir threshold status |
| 6 | **Standing inspector** | Power-user | Detailed breakdown of the viewer's standing in this Qahal — attestation chain walked, affinity signals weighted, feedback debits subtracted, current Bloom tier surfaced |
| 7 | **Shefa resources** | Power-user | REA flow visualization for this Qahal — inflows, outflows, commons-share balance, Agreement cascade rules |
| 8 | **Attestations** | Power-user | The viewer's attestation history scoped to this Qahal — quiz results, peer recognitions, contribution validations |
| 9 | **Graph discovery** | Power-user | Suggestion surface — adjacent Qahals worth knowing, federation candidates, related contributor presences |

(The right-context column holds Panel 4 (Co-Steward) + condensed Rules + condensed Graph Discovery, even when those panels aren't the main viewer's active panel. The center viewer cycles through any of the 9; the right is persistent context.)

## Section 4 — The 5 deep-implementation panels (MVP)

Sprint 1 implements all 9 panels visually in Library B (so the simple → power-user toggle demonstrates the full architectural surface). Sprint 2-4 deeply wire the **5 simple-tier panels** to real backend. The 4 power-user panels remain visual/stub at MVP — they prove the architecture is real but don't carry full backend depth yet.

### 4.1 Stream

The household's commons stream — what the value-scanner has observed + REA care events + the small accounting motions of the day. Rendered like a feed but **without engagement metrics**. No likes, no reaction counts, no algorithmic ranking. Reverse-chronological with a few elevated items the co-steward has surfaced.

Each stream item carries:
- Author (imagodei badge, lensed through this Qahal)
- Timestamp (relative; "17m ago", "yesterday", "Tuesday morning")
- Content (the act observed: "Sheila sent soup", "James's morning routine attestation", "Matthew acknowledged Gertrude's check-in")
- Optional: small care-economy REA marker (tokens earned, presence attested)
- Optional: thread thread if it's a multi-step exchange

Edge cases the storyteller's narrative implies:
- Acknowledgment-pending items appear in a subtle "to-acknowledge" treatment (Sheila's recipe, Gertrude's check-in, the neighbor's offer to bring dinner)
- The co-steward's quiet observations interleave with member-authored content but visually distinct

### 4.2 Member-Ring (tier-aware)

The stratified member view from the brainstorm. The headline number is **network reach** (not flat membership); drill-down reveals four tiers. Each tier item is imagodei-lensed for the current viewer + this Qahal's context.

```
┌──────────────────────────────────────────────────────────┐
│ Network reach                                       1176 │
│ ──────────────────────────────────────────────────────── │
│                                                          │
│ Active stewards · governance                         15  │
│   Brother Cal · Elder Thompson · Elder Davis · ...       │
│                                                          │
│ Active stewards · community participation            50  │
│   Matthew Dowell · Susan Hardin · Jin Lee · ...          │
│                                                          │
│ Contributor presences ◇ value-in-trust                75 │
│   non-protocol participants whose recognition           │
│   accrues to the Qahal commons; in trust until          │
│   direct participation resolves it                       │
│                                                          │
│ Compute-hosting stewards ⚙                          100  │
│   lending stewarded compute allocation for the          │
│   Qahal's resilience, edge distribution, discovery      │
└──────────────────────────────────────────────────────────┘
```

The household variant of this panel has different tiers (no "active stewards · governance" tier for a household of 4) — the rubric defines which tiers apply. For the Dowell household:
- Family members (4: Matthew, Jessica, James, and the dog if attested)
- Core-family extended (Sheila, Gertrude)
- Compute-hosting stewards (3)
- Community-reach members (whoever in the broader peer graph has touched the household commons)

Click any tier → drilled-down list of imagodei profiles, each lensed through this Qahal.

### 4.3 Rules

Renders the Qahal's rubric in human-readable form. The Dowell household's rubric:

```
Dowell Household — what we honor here

Standing in this household is:
  • care contributed
  • presence shown up
  • repair offered when something has broken between us

Mastery progression (Bloom curve):
  Remember:   know who lives here, what we hold, our rhythms
  Understand: explain our care-economy patterns; recognize when help is needed
  Apply:      do the daily work — meals, chores, attention given without prompt
  Analyze:    notice when something's off; see what's not being said
  Evaluate:   judge contributions against what the household actually needs
  Create:     propose new rhythms; design our family's rule of life

Cadence: gentle. Standing decays slowly. Old contributions are honored.

Friction-gradient: no household member can accumulate disproportionate authority
without commons-elohim flagging for discussion. Plural stewardship is structural.

Configured by: Matthew Dowell, Jessica Dowell
Last revised: 2026-04-15 (added "repair offered" line)
```

Steward-tier capability gates the "edit rubric" affordance.

### 4.4 Co-Steward

The commons-elohim co-steward's reflective view. Lives in the right-context column as a persistent presence. Always visible. Updates ambient.

```
Co-steward · Dowell household

The household is steady.

Three care contributions pending acknowledgment:
  · Sheila's recipe
  · Gertrude's check-in
  · The neighbor's offer to bring dinner

No urgency. You can acknowledge them when you next sit down.
```

This panel's tone is critical — quiet, observational, declarative. It does not advise. It does not nudge. It witnesses. Per the storyteller's narrative ("It is a good kind of morning"), the co-steward's voice is one of presence, not productivity.

For a Qahal in active tension (a congregation with a doctrinal concern surfaced; a life-group with a member departing), the co-steward's view shifts register accordingly — still observational, but naming what is being held. Never alarmist.

### 4.5 Social-Compute

The shefa↔qahal intersection. Lights up the topology — proves the substrate's resilience claim is real.

```
┌──────────────────────────────────────────────────────────┐
│ Social-Compute Topology                                  │
│ ──────────────────────────────────────────────────────── │
│                                                          │
│  Our hub · Dowell household                              │
│  ─────────                                               │
│  matthew · jessica · james                               │
│  status: ●  healthy · 4 GB allocated · last-seen: now   │
│                                                          │
│  Compute-stewards FOR us         (3-of-3 active ✓)       │
│  ───────────────────────                                 │
│  ◆ gertrude-grandma     ●  100% · last sync: 17m ago    │
│      replicating: household state + care-economy ledger │
│  ◆ sheila-household     ●  100% · last sync: 3m ago     │
│      replicating: household state                       │
│  ◆ ethan-dowell (uncle) ◐   85% · last sync: 4h ago     │
│      replicating: household state · degraded            │
│                                                          │
│  We compute-steward FOR          (reciprocal trust)      │
│  ──────────────────────                                  │
│  ◆ gertrude-grandma → her household state               │
│  ◆ susan-household   → sibling-household trust          │
│                                                          │
│  Recovery readiness: ✓ ready  ·  Shamir threshold: 2/3  │
│  ──────────────────────────────────────────────────────  │
│  · Last recovery drill: 14 days ago (passed)            │
│  · Next scheduled drill: 16 days                        │
└──────────────────────────────────────────────────────────┘
```

This panel reads from existing notarized DHT entries (`PeerStatus`, `NodeRegistration`, household-shape entries per D1-D5 canon). It is a Category C computed view — no new schema invented here, but it surfaces the existing ones in a Qahal-legible register.

The recovery drill line is the operator's connection back to the recovery-protocol work (recovery-m4-* specs in `genesis/docs/superpowers/specs/`). A click on "Last recovery drill" surfaces the drill log; a click on "Next scheduled drill" surfaces the upcoming exercise + option to initiate now.

## Section 5 — The configurable resource list (Tier 1, MVP)

The per-Qahal sidebar's content is configured by the Qahal's stewards (capability-gated by standing tier). It carries four sections:

### Section 1 — Protocol panels

The 9 elohim-core panels. Always present. The Qahal's rubric configures which appear in simple-user vs power-user view, but all 9 are accessible to any operator at appropriate capability.

### Section 2 — Curated EPRs ◆

Stewards pin EPRs (within the elohim network) that are useful for this Qahal:
- A family recipes EPR for the household
- A sermon archive EPR for the congregation
- A learning path EPR for a life-group studying Romans
- A federation directory EPR for the wisdom commons

Pinning is a small Sprint 2 schema concern — a `QahalPinnedResource` entry that couples the Qahal to the target EPR with display metadata (title, icon, position).

### Section 3 — External hyperlinks ⤤

Web2.0 links — capability-gated and visually distinct:
- Always visually announced as "leaving the elohim network" (⤤ icon, different color treatment)
- **Greyed out when device is offline** (substrate's honesty about the boundary)
- **Capability-gated by Qahal rubric** — stewards configure which capability tiers see external links; vulnerable members (grandma, IDD members, humans under legal stewardship, children) can be excluded by the household's rubric to protect from open-web exposure

The household stewardship rubric is what makes this work — Matthew + Jessica configure that James (capability tier "child") doesn't see external links; Gertrude (capability tier "elder under guardianship") sees them filtered through the co-steward.

### Section 4 — Power-user toggle ⓘ

The expandable section that reveals the 4 power-user panels (standing inspector, shefa resources, attestations, graph discovery). Toggle controlled by the operator's UI preference, not by their capability — anyone with simple-user tier can opt into power-user view; what they can DO inside those panels is still capability-gated by standing.

## Section 6 — Provenance categories and capability gating

Every item that can appear in the Qahal sidebar carries one of four provenance categories. The treatment is consistent across the design system:

| Provenance | Visual | Offline | Capability gating | Trust model |
|---|---|---|---|---|
| **Protocol panel** | Standard panel icon, primary text color | ●  Always available | Standing-tier-gated for ACTIONS; visible to all members | Substrate-baked; cannot be hidden |
| **Curated EPR** ◆ | Native badge, primary text color | ●  Always available (content-addressed) | Rubric-configured; capability-gated optional | Steward-curated, attestation-traceable |
| **Installed applet** ⬢ | Applet badge, primary text color | ●  Available (per-device install, P2P-synced) | Installation governed by steward council; runtime capability sandboxed | Tier 3 substrate, post-MVP |
| **External hyperlink** ⤤ | External marker, dimmer text, distinct border | ◐ Greyed when offline | Capability-gated by rubric; many stewarded humans don't see them | "You're leaving the elohim network" — substrate honesty |

### The capability-gating discipline for external links

This is one of the architecturally distinctive moves. The household rubric declares:

```yaml
external_link_visibility:
  visitor: filtered_via_co_steward
  engaged: full
  contributor: full
  steward: full
  # Special protected tiers (override capability):
  child: hidden
  elder_under_guardianship: filtered_via_co_steward
  idd_member: filtered_via_co_steward
  legal_steward_protected: hidden
```

This honors the **elohim-as-counsel** memory pattern + the Imago Dei discriminator: dignity-floor protection of those whose agency requires support. The household acts as the steward for digital exposure for members who can't safely navigate the open web themselves.

Sprint 2 schema work needs to model these protected tiers explicitly. They are not "Bloom tiers" in the standing curve sense — they are *protected tiers* that the rubric declares for individual members. A grandma whose capability is unconstrained in the household's care-economy might still be `elder_under_guardianship` for purposes of external-link exposure, configured by family discernment.

## Section 7 — Mock-data fixtures: canonical + variations

Library B pattern stories render against mock-data fixtures. Per the brainstorm:

### Canonical (ground-truth fidelity)

Four exact-match fixtures rendering the storyteller-canonical scenes from spec Section 4:

```
fixtures/canonical/
  dowell-household-tuesday-morning.ts
    Matthew + Jessica + James (sick) + Sheila's soup + Gertrude's
    check-in + 3 pending acknowledgments + household co-steward
    saying "the household is steady"

  cofc-congregation-sunday-morning.ts
    230 members + Brother Cal + 4 elders + Romans 12 sermon series
    + youth retreat needing 2 drivers + 3 prayer requests +
    co-steward noting reach is rising and 3 life-groups at threshold

  hardins-life-group-tuesday-evening.ts
    6 families gathered + Romans 12 verse 1 discussion + Sarah's
    father in hospital + cohesion threshold reached + John Hardin's
    hosting accumulation gently caught by friction-gradient

  wisdom-commons-thursday-afternoon.ts
    83 congregations + Brother Cal's concern surface submitted to
    Arkansas sister congregation + peer council convening + witness
    produced + REA reconciliation event recorded
```

### Variations (composable edge cases)

Per-archetype variations for testing breadth:

```
fixtures/variations/
  household-with-toddlers.ts
  household-with-teen.ts
  household-recovering-from-loss.ts
  household-multi-generation.ts
  household-single-parent.ts

  congregation-doctrinal-tension.ts
  congregation-at-peace.ts
  congregation-newly-formed.ts

  life-group-newly-formed.ts
  life-group-three-years-cohesive.ts
  life-group-departing-member.ts

  wisdom-commons-concern-surfaced.ts
  wisdom-commons-reconciliation-recorded.ts
  wisdom-commons-new-congregation-joining.ts
```

The variations exercise the design's edge cases. They prove the chrome + 9 panels work for any household, not just the Dowells. Sprint 5 will author more variations by mining the 1,681 value-scanner scenarios (per the value-scanner audit document at `genesis/docs/plans/2026-05-22-value-scanner-content-audit.md`).

### Namespace separation

Per the value-scanner audit: the Parker family (value-scanner namespace at `genesis/data/lamad/content/`) and the Dowell family (canonical narratives namespace at spec Section 4) inhabit deliberately separate namespaces. Library B uses the Dowell namespace for canonical fixtures; Library B can draw from the Parker namespace for variations.

## Section 8 — The settings palette (where the simple → power-user toggle actually lives)

The simple → power-user toggle is **not a prominent UI element** on the Qahal homepage. There is no toggle button in the sidebar header, no "show more" gesture in the chrome. The toggle is a **setting buried in imagodei preferences** — alongside an entire palette of user-experience controls that govern how each human encounters the protocol.

This framing is important: prominent toggles on the homepage would invite operators to think of "more vs less" as a UX choice they make in the moment. That misframes the relationship. *How the protocol presents itself to a person* is a deliberate stewardship decision — sometimes made by the person themselves, sometimes made by their steward on their behalf. It belongs to imagodei preferences, not to homepage chrome.

### 8.1 The settings palette as substrate concern — and as ambient experience

A whole database of user settings backs the imagodei layer. They shape the UX gradient — what each human encounters of the protocol's surface — but they are an **ambient concern** for most users. The settings palette is not front-and-center; it sits in imagodei preferences, discoverable but not promoted. For most operators, they configure once (or never, accepting defaults) and the settings recede into the background, shaping their experience without demanding attention. The settings become **prominent only when** the operator is a developer debugging or an elohim-support agent helping a human figure out *"what's visible / not visible and why."* See Section 8.6 for that introspection surface.

Most settings have two-axis governance:

- **Self-configurable** — the human configures their own settings (the common case for unprotected-tier humans)
- **Steward-configurable** — for stewardee relationships, a steward can configure the stewardee's settings, with the configuration itself attestation-witnessed and revisable

The steward-stewardee setting-configuration relationship is a substrate primitive. It bakes in the elohim-as-counsel pattern at the settings layer: the household acts as the steward for the digital experience of members who can't safely curate it themselves. The palette is the visible surface of that stewardship.

### 8.2 The MVP settings palette

A non-exhaustive enumeration of controls in the imagodei settings palette. Each setting names: who can configure it (self / steward / both), the default value, and the rubric-binding (which protected tiers receive special treatment).

| Setting | Configurable by | Default | Protected-tier behavior |
|---|---|---|---|
| **Power-user view** (the toggle that was misframed) | self only | simple | n/a — UX preference, not capability gate |
| **External link visibility** | self + steward | full | child = hidden; idd_member, elder_under_guardianship = filtered-via-co-steward; legal_steward_protected = hidden |
| **Notification volume/style** | self + steward | gentle | protected tiers = quieter; child = none |
| **Content reach gating** (who can reach you) | self + steward | qahal-bounded | protected tiers = household-bounded; child = parent-bounded |
| **Standing visibility** (do you see your own standing breakdown?) | self + steward | visible | child = hidden; idd_member = simplified; elder_under_guardianship = simplified |
| **Co-steward voice register** | self + steward | observational | protected tiers = warmer; child = friendlier |
| **Recovery authority delegation** | self + steward | self | protected tiers = household; child = parents explicit |
| **Compute-stewardship visibility** | self + steward | visible | child, idd_member = hidden by default |
| **Data export visibility** (full graph viewable?) | self + steward | stewarded | protected tiers = household-mediated |
| **Language / accessibility** (font, contrast, audio) | self + steward | system | per-need; child = age-appropriate; visual-impairment = high-contrast + audio |
| **Web2.0 link click confirmation** | self + steward | one-tap-warning | protected tiers = blocked-or-mediated |
| **Imagodei lens defaults** (which Qahal context renders first when viewing others) | self only | most-recent-shared-Qahal | n/a |
| **Onboarding pace** (how quickly do new capabilities surface?) | self + steward | natural | protected tiers = paced by steward |

This is the **palette of controls that help curate the (grandma, legal-steward, IDD, child) experience within the elohim network**. MVP doesn't need to wire every setting to the substrate, but the palette must be present and the steward-stewardee configuration relationship must be modeled.

### 8.3 The settings palette UI location

The settings palette lives in the imagodei surface — accessed via the user's own profile, not via the Qahal homepage chrome. Specifically:

```
[user avatar] → imagodei profile → Settings → Palette → category
```

For stewards: when viewing a stewardee's imagodei profile, the steward sees a `Configure settings for [stewardee]` affordance (gated by their stewardship-authority attestation). The configuration UI is the same palette; the active-configurator is the steward.

Every setting change is attestation-witnessed:
- Self-changes: signed by the self
- Steward-changes: signed by the steward + a witness attestation from the commons-elohim co-steward of the relevant Qahal (household for child/elder; legal jurisdiction for legal-steward; designated guardian Qahal for IDD member)

### 8.4 The orthogonality discipline

Three axes are orthogonal:
- **UX preference** (simple vs power-user view) — set in palette, never gates capability
- **Capability authority** — gated by standing in this Qahal, computed view, never set in palette
- **Stewardship status** — declared in palette as a protected-tier rubric, configured by stewards

These three must not be confused in implementation. A capability-gated affordance (e.g., "edit rubric") is only visible to humans with steward-tier standing — regardless of their power-user toggle. A protected-tier-gated affordance (e.g., "external link click") is only visible to humans not in a protected tier — regardless of their standing. The power-user toggle controls only visual density, not capability or stewardship.

### 8.5 Sprint 1 scope for the settings palette

For Sprint 1 UX exploration:
- Library A: `elohim-imagodei-settings-palette.ts` — the buried settings palette element (NOT a homepage-level toggle)
- Library B: stories rendering the palette in three modes — self-configuration, steward-configuring-stewardee, and the witness/attestation surface
- Mock data: fixtures showing how each protected tier renders differently (grandma's view; IDD member's view; child's view; unprotected adult's view)
- The Qahal homepage simply *honors* the settings — it does NOT expose the toggle. If the user has simple view configured, the sidebar's power-user section is hidden; if they have power view configured, it's shown. The toggle itself is invisible from the homepage.

Sprint 2 schema work: the settings database (Category B agent-scoped private with B2 attestation for steward-configured changes) and the steward-stewardee configuration relationship as a substrate primitive.

### 8.6 Developer / support-agent introspection surface

When the operator is a developer debugging a UX issue, or an elohim-support agent helping a human understand why their view is what it is, they need an **introspection surface** that exposes:

- The full settings palette state for the human in question (with capability-attestation to view it)
- Which settings are self-configured vs steward-configured (with attestation chain visible)
- For each rendered affordance on the homepage: a "why is this here / why is this not here" trace — surfaces the rubric clause, the standing tier, the settings palette entry, the protected-tier rubric, and any commons-elohim mediation that contributed to the decision
- For each suppressed/hidden affordance: the same trace, surfacing what would need to change for it to appear

This is the substrate's **interpretability requirement** (vision spec Section 1.5) applied at the UX layer. The substrate must be able to *explain* itself to humans who need to understand it. A grandmother whose external links are hidden should — if she or her steward asks — be able to receive a plain-language explanation: *"External links are hidden in your view because Matthew configured your protected-tier status as `elder_under_guardianship`, and the household rubric filters external links for that tier."*

For Sprint 1 scope:
- Library A: `elohim-imagodei-introspection-panel.ts` — the developer/support-agent surface. Buried alongside the settings palette in imagodei. Not visible to ordinary operators by default; surfaces only when the human (or their support agent, with appropriate attestation) requests it.
- Library B: stories showing the introspection panel in two modes — self-introspection (the human asks "why am I seeing this?") and support-agent introspection (a developer or elohim-support agent debugging another human's view)

Sprint 2 schema work: the introspection trace primitive — how does each rendered affordance generate its provenance trace? Likely an inline metadata attribute on the rendered DOM (`data-elohim-trace`) that the introspection panel reads. This is a small new substrate concern downstream of the settings palette schema.

## Section 9 — Architectural connections

This UX design grounds in existing memory + spec sections. Sprint 1 implementation must honor these connections.

| Vision spec section | UX surface that renders it |
|---|---|
| Section 1.2 — household as living core | The household homepage is the MVP discriminator; all other Tier-0 archetypes use the same chrome |
| Section 1.5 — Imago Dei discriminator | External link capability-gating; protected-tier rubric primitives |
| Section 2.1 — graduated capability surface | Standing-tier-gated affordances within panels |
| Section 2.5 — rubric as governable EPR | Rules panel surfaces the rubric; steward-tier can edit |
| Section 2.7 — commons-elohim co-steward | Co-Steward panel in right-context column; ambient observational voice |
| Section 2.8 — friction-gradient limitarianism | Friction-gradient flags surfaced in co-steward view (e.g., "the body is leaning toward Brother Cal's voice this season") |
| Section 2.10 — imagodei lens recursion | Member-ring tier drill-down uses imagodei views lensed through this Qahal's context |
| Section 4 — Tier-0 worked examples | Mock-data canonical fixtures render these exact scenes |
| Section 7.6a — common-sense formation as diffusion | The chrome's "social garden tending" register is what reforms common sense; lived contrast at the homepage drives diffusion |

| Memory entry | UX surface that renders it |
|---|---|
| `project_household_living_core_lived_contrast_diffusion` | The entire framing of Section 1 |
| `project_commons_elohim_co_steward` | Co-Steward panel + ambient voice register |
| `project_resilience_epic_landed_2026_05_18` | Social-Compute Topology panel |
| `project_dissolution_principle_sensemaking_collectives` | Provenance categories for external links (substrate honesty about web2.0 boundary) |
| `project_p2p_is_hosting` + `project_doorway_peer_registration` | Compute-hosting stewards tier in member-ring |
| `project_elohim_as_counsel` | Capability-gated external link rubric for protected tiers |
| `project_recovery_grandma_standard` | Recovery readiness line in Social-Compute panel |

## Section 10 — Sprint 1 deliverables (for writing-plans handoff)

The writing-plans skill takes this UX design and produces a task-by-task plan for Sprint 1. Sprint 1 is purely Storybook + Library A + Library B — no backend touched.

### Library A — blank-slate elohim-elements

Each is a Lit web component with capability profile JSDoc, three precondition gates (a11y + i18n + ua-prefs), and default stories (Unstyled + CustomTheme + every claimed lens).

All Library A elements follow the codebase's existing per-pillar package structure. Qahal-pillar elements go in `app/elohim-elements/elohim-qahal/`; imagodei-pillar elements (settings palette + introspection) go in `app/elohim-elements/elohim-imagodei/`. Each package mirrors the elohim-core canonical pattern (`elohim-button.ts` + `.spec.ts` + `.manifest.spec.ts`, with `register.ts` + `index.ts`, `vite.config.ts` + `custom-elements-manifest.config.mjs`).

```
app/elohim-elements/elohim-qahal/src/
  # primitives — used by all panels + chrome
  elohim-qahal-imagodei-badge.ts            # contextual imagodei avatar+name+ring
  elohim-qahal-standing-ring.ts             # small ring indicator (Bloom-tier dots)
  elohim-qahal-capability-tier-chip.ts      # capability gating affordance label
  elohim-qahal-provenance-marker.ts         # ●/◆/⬢/⤤ markers per provenance category
  elohim-qahal-care-economy-marker.ts       # stream item REA event markers

  # chrome (4-column layout)
  elohim-qahal-collective-switcher.ts       # far-left column
  elohim-qahal-sidebar.ts                   # second column (resource list)
  elohim-qahal-main-viewer.ts               # third column (active panel)
  elohim-qahal-context-column.ts            # fourth column (right-nav)

  # 9 elohim-core panels
  elohim-qahal-stream-panel.ts              # panel 1 (deep-impl)
  elohim-qahal-member-ring-panel.ts         # panel 2 (deep-impl, tier-aware)
  elohim-qahal-rules-panel.ts               # panel 3 (deep-impl)
  elohim-qahal-co-steward-panel.ts          # panel 4 (deep-impl, right-context)
  elohim-qahal-social-compute-panel.ts      # panel 5 (deep-impl)
  elohim-qahal-standing-inspector-panel.ts  # panel 6 (visual-stub)
  elohim-qahal-shefa-resources-panel.ts     # panel 7 (visual-stub)
  elohim-qahal-attestations-panel.ts        # panel 8 (visual-stub)
  elohim-qahal-graph-discovery-panel.ts     # panel 9 (visual-stub)

  # resource list sidebar sections
  elohim-qahal-protocol-panel-list.ts       # sidebar section 1
  elohim-qahal-curated-epr-list.ts          # sidebar section 2 (◆ badge)
  elohim-qahal-external-link-list.ts        # sidebar section 3 (⤤ badge + offline-grey)
  elohim-qahal-power-user-expandable.ts     # sidebar section 4 — appears ONLY if
                                            # imagodei settings palette has power-user
                                            # view enabled; no toggle surfaced here

  register.ts
  index.ts

app/elohim-elements/elohim-imagodei/src/
  elohim-imagodei-settings-palette.ts       # the full settings palette surface
  elohim-imagodei-setting-control.ts        # individual setting control primitive
  elohim-imagodei-protected-tier-marker.ts  # rendering protected-tier status
  elohim-imagodei-steward-configure-banner.ts  # banner when steward edits stewardee
  elohim-imagodei-introspection-panel.ts    # the developer/support-agent surface

  register.ts
  index.ts
```

### Library B — designed graphos pattern stories

```
app/elohim-library/projects/graphos/stories/
  qahal-homepage/
    canonical/
      qahal-homepage-dowell-household.stories.ts
      qahal-homepage-congregation.stories.ts
      qahal-homepage-life-group.stories.ts
      qahal-homepage-wisdom-commons.stories.ts

    variations/
      qahal-homepage-household-with-toddlers.stories.ts
      qahal-homepage-household-multi-generation.stories.ts
      qahal-homepage-congregation-doctrinal-tension.stories.ts
      qahal-homepage-life-group-newly-formed.stories.ts
      ...

    user-toggles/
      qahal-homepage-simple-user-view.stories.ts
      qahal-homepage-power-user-view.stories.ts

    capability-gating/
      qahal-homepage-visitor-view.stories.ts
      qahal-homepage-contributor-view.stories.ts
      qahal-homepage-steward-view.stories.ts
      qahal-homepage-protected-tier-view.stories.ts  # grandma / IDD / child
```

### Mock-data fixtures

```
app/elohim-library/projects/graphos/src/fixtures/qahal/
  canonical/
    dowell-household-tuesday-morning.ts
    cofc-congregation-sunday-morning.ts
    hardins-life-group-tuesday-evening.ts
    wisdom-commons-thursday-afternoon.ts

  variations/
    household-with-toddlers.ts
    household-multi-generation.ts
    congregation-doctrinal-tension.ts
    life-group-newly-formed.ts
    ...

  primitives/
    mock-imagodei-profiles.ts        # the Dowells, Brother Cal, the Hardins, etc.
    mock-rubrics.ts                  # household + congregation + life-group + wisdom-commons rubrics
    mock-care-economy-events.ts      # stream items
    mock-social-compute-topology.ts  # replication graphs, peer status, recovery state
```

### Storybook configuration

- Storybook dev server runs on port `6006` per devfile.yaml (already exposed)
- Bind to `0.0.0.0` so Eclipse Che recognizes and surfaces the endpoint
- Theme: Elohim brand tokens applied via graphos design system
- Story controls: Qahal type selector, capability tier selector, simple/power toggle, provenance category toggle

### Documentation

- Each elohim-element gets a JSDoc capability profile + usage example
- Each Library B pattern story has a documentation note explaining what it demonstrates
- A top-level README at `app/elohim-elements/qahal-chrome/README.md` explaining how to compose the chrome

## Section 11 — Open questions forward to Sprint 2 schema work

These bear on Sprint 2 (substrate spine schema design) and beyond. Captured here so they're not lost.

1. **QahalPinnedResource schema** — the substrate primitive for the curated EPR sidebar section. Anchored as link from the Qahal entry. Sprint 2.
2. **ContributorPresence with value-in-trust** — the substrate primitive surfaced by the brainstorm. Extends `BeneficiaryRef` enum with `ContributorPresence { external_identity_attestation, in_trust_share }`. Sprint 2 + downstream when shefa cascade lands (post-MVP Sprint 6+).
3. **Protected-tier rubric primitives** — `child`, `elder_under_guardianship`, `idd_member`, `legal_steward_protected` — these are not Bloom tiers; they are rubric-declared protected statuses for individual members. Schema work needed: how are they declared? How are they revised? How does the substrate guarantee they cannot be circumvented? Sprint 2 design pass.
4. **External link provenance + capability gating enforcement** — at the substrate level, not just the UI level. The doorway shouldn't proxy a web2.0 link request from a protected-tier human even if the UI client tries to. Sprint 3 substrate wire-up.
5. **Recovery drill UI flow** — clicking "Last recovery drill" surfaces the drill log; clicking "Next scheduled drill" surfaces the upcoming exercise. The drill operation itself already exists (recovery-m4-* specs) — the Qahal homepage wires the human-readable surface to it. Sprint 3.
6. **Tier-aware member-ring rendering with rubric variance** — the rubric declares which tiers apply for this Qahal archetype (a household doesn't have "active stewards · governance"). Schema work: how does the rubric declare tier set? Sprint 2.
7. **Compute-hosting steward as steward-tier capability path** — how does a human earn compute-hosting steward standing? Sprint 2 rubric template design.
8. **Settings palette database schema** — Category B agent-scoped private (the settings themselves) with B2 attestation for steward-configured changes. Schema work: how is the palette structured? How are settings versioned? How does the attestation-witness primitive bind a steward's configuration change to the commons-elohim co-steward's witness? Sprint 2 substrate spine design.
9. **Steward-stewardee configuration relationship as substrate primitive** — explicit modeling of stewardship-authority attestation that gates steward-configurable settings. Connects to the elohim-as-counsel pattern and the graduated-recovery-authority pattern. Sprint 2 design pass + downstream substrate wire-up.
10. **Settings palette expansion** — Section 8.2 names 13 settings as a non-exhaustive starting palette. Sprint 2 should produce the canonical palette specification (which settings are MVP, which are post-MVP, which are reserved for archetype-specific extension). The palette is the surface where the elohim-as-counsel pattern becomes operable for everyday users — getting it complete is foundational, not cosmetic.

## Sprint 1 entry criteria

Once this UX design spec is reviewed and approved by the operator, Sprint 1 kicks off with:
- The writing-plans skill produces a task-by-task plan
- graphos-designer + component-architect agents lead the implementation
- Storybook is the deliverable surface — operator can review at `http://devspace:6006`
- MVP exit for Sprint 1: all canonical + variation Library B stories render; all 9 panels visible in power-user view; all 4 (5 simple-tier) deep-impl panels carry their full visual content; capability-gating + provenance + offline-grey treatments work in mock-data flow
