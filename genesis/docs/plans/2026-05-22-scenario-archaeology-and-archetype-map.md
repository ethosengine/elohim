---
project: elohim-protocol
type: archaeology + reclassification proposal
status: draft (operator review)
created: 2026-05-22
scope: a2o feature corpus, qahal pillar Sprint 0.5
authors:
  - Claude Opus 4.7 (archaeology + synthesis)
governs:
  - Sprint 1 (UX exploration — scenario base for graphos pattern stories)
  - Sprint 5 (genesis + canonical-template authoring — new scenarios)
companion-to:
  - genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md
  - genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md
  - genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md
---

# Scenario Archaeology and Archetype-Aligned Reclassification

> **Purpose.** The 76 .feature files at `genesis/a2o/features/` predate the gospel-tier qahal architecture vision. They were authored under a mixed-axis taxonomy (pillar + implementation-shape + content-surface) at a moment when the protocol's collective-archetype catalog had not yet been articulated. This document inventories the corpus, maps each file to the archetype catalog established in the qahal architecture vision spec, and proposes a new directory taxonomy that puts the **collective-archetype axis primary** so that Sprint 1 UX work and Sprint 5 scenario authoring resolve cleanly to the gospel-tier reference.
>
> **What this document is not.** It is not a file-movement execution plan. It is a proposal Matthew reviews end-to-end. Migration is operator-driven in a later sprint.

## How to read

- **Section 1** is the full file inventory (table; 76 rows) — path, feature title, one-sentence summary.
- **Section 2** maps each file to one or more archetypes (table; primary + secondary + confidence). This is the central archaeology pass.
- **Section 3** lists orphan scenarios with proposed dispositions (graduate / memorialize / hold / keep+reclassify / rewrite).
- **Section 4** is the gap analysis — archetypes with zero matched scenarios, with proposed new scenarios (titles + one-sentence summaries). Tier 0 archetypes are highlighted because they drive Sprint 5 MVP authoring.
- **Section 5** proposes the new directory taxonomy.
- **Section 6** is the concrete migration plan (source → destination, splits, merges, renames). No moves executed.
- **Section 7** is the connection map — each proposed directory linked back to its gospel-tier source section.

## Archetype catalog quick reference

For brevity in the tables below, archetypes are abbreviated. Full definitions live in the gospel-tier spec (`2026-05-21-qahal-architecture-vision.md`).

**Tier 0 — Worked examples (Section 4, MVP-critical):**
- `T0:household` — Dowell household (Section 4.1) — intimate care economy
- `T0:faith-community` — Restoration Movement congregation (Section 4.2) — plural-stewardship
- `T0:life-group` — sub-Qahal nested in congregation (Section 4.3) — holonic nesting
- `T0:wisdom-commons` — autonomous Churches of Christ federation (Section 4.4) — horizontal peer federation

**Tier 1+2 — Everyday + civic (Section 5, post-MVP stubs):**
- `T1:eae` — ChickenMax → EAE franchise-to-collective conversion (5.1)
- `T1:grocery-coop` — member-owner food retail (5.2)
- `T1:farm-csa` — community-supported agriculture (5.3)
- `T1:distribution-center` — worker-stewarded logistics hub (5.4)
- `T1:factory-intimate` — small-scale craft production (5.5)
- `T1:industry-association` — peer federation of firm-Qahals (5.6)
- `T1:library` — knowledge commons (5.7)
- `T1:neighborhood-association` — residence-based civic coordination (5.8)
- `T2:city-hall` — municipal civic coordination (5.9)

**Tier 3 — Abstract sensemaking collectives (Section 6, far-horizon):**
- `T3:research` (6.1), `T3:university` (6.2), `T3:nuclear` (6.3), `T3:military` (6.4), `T3:rnd` (6.5), `T3:venture-coop` (6.6), `T3:justice-reconciliation` (6.7), `T3:mutual-aid-insurance` (6.8), `T3:health-services` (6.9), `T3:platform-coop` (6.10), `T3:arts-patronage` (6.11), `T3:transportation` (6.12), `T3:mineral-industrial` (6.13), `T3:logistics-freight` (6.14), `T3:education-k12` (6.15), `T3:childcare` (6.16), `T3:natural-collective` (6.17), `T3:identity-collective` (6.18)

**Cross-cutting (substrate concerns that span archetypes):**
- `CC:reach` — outward content visibility / nervous system
- `CC:standing` — inward Qahal capability surface
- `CC:imago-dei` — inherent-dignity discriminator
- `CC:friction-gradient` — anti-concentration substrate
- `CC:attestation` — recognition + provenance primitives
- `CC:commons-elohim` — per-Qahal co-steward
- `CC:rea-flow` — REA Commitment/Fulfillment/Event mechanics
- `CC:reach-x-standing` — composition of both axes

**Infrastructure (substrate plumbing — not archetype-bound):**
- `INF:ssr` — server-side rendering capability
- `INF:p2p` — peer-to-peer mesh / federation transport
- `INF:doorway` — web2 projection gateway
- `INF:recovery` — key recovery / identity continuity
- `INF:browser` — browser-shape smoke tests
- `INF:delivery` — content delivery (CIDs, omnibar, projection cache)
- `INF:deployment` — operational topology / device modeling
- `INF:resilience` — placement, gap detection, contract-aware distribution

---

## Section 1 — Inventory (76 files)

| # | Path | Feature title | Summary |
|---|---|---|---|
| 1 | auth/auth-lifecycle.feature | Authentication Lifecycle | Hosted-human register/login/logout cycle on the doorway; identity, sessions, password gates. |
| 2 | auth/conductor-pool-recovery.feature | Conductor Pool Recovery for Hosted Users | Hosted humans reconnect after the conductor pool composition changes across deploys. |
| 3 | auth/fixture-humans.feature | Fixture Human Categories | All categories in humans.json (core family, community, affinity, local economy, newcomers, red-team) can log in. |
| 4 | auth/operator-onboarding.feature | Operator Onboarding | First admin (Matthew) bootstraps the alpha doorway with a bootstrap key and federation peer config. |
| 5 | auth/recovery/cross-stack/recovery-cross-stack-transport.feature | Recovery completes across mixed iroh/libp2p share-holder transports | Intimate-quorum recovery works whether share-holders speak iroh, libp2p, or a mix. |
| 6 | auth/recovery/freeze-floor-blocks-intimate-rotation.feature | Freeze-Floor Gate Blocks Intimate-Layer Rotation | Active intimate freeze blocks IntimateQuorum rotation; CryptographicQuorum is exempt. |
| 7 | auth/recovery/intimate-quorum-happy-path.feature | Intimate Quorum Happy Path | Abby's three required witnesses approve and her lost pubkey is rotated. |
| 8 | auth/recovery/recovery-m5-defender-role-gate.feature | submit_specialist_revocation gated by local defender role marker | Coordinator only accepts the specialist revocation call when the local defender role marker is present. |
| 9 | auth/recovery/recovery-m5-doorway-handoff-to-steward.feature | Doorway redirects steward humans to their portal host | Graduated stewards are routed from hosted doorway to their peer-native portal with a session token. |
| 10 | auth/recovery/recovery-m5-list-my-keys.feature | Listing my keys in the account-management surface | Account-management UI shows active keys + revocation history. |
| 11 | auth/recovery/recovery-m5-lost-key-entry.feature | Lost-key entry point routes to the right flow | Lost-key entry routes whether or not an active key still exists. |
| 12 | auth/recovery/recovery-m5-portal-host-discovery.feature | Adding and listing portal hosts | Stewards add portal hosts (https only) via account management. |
| 13 | auth/recovery/recovery-m5-self-revoke.feature | Self-revocation through the account-management surface | Steward self-revokes a key with a confirm step. |
| 14 | auth/recovery/recovery-m5-vote-as-emergency-contact.feature | Voting on recovery as an emergency contact | Emergency contact approves or rejects a pending recovery vote. |
| 15 | auth/recovery/recovery-shamir-optional.feature | Recovery succeeds with or without Shamir share custody | Path A (intimate quorum) is sufficient; Path B (Shamir) is an optional add. |
| 16 | auth/recovery/revocation-emergency-quorum.feature | Emergency Contacts Kill a Captured Key by Quorum | Jessica + intimate contacts revoke Matthew's captured key by quorum; attacker locked out. |
| 17 | auth/recovery/revocation-self.feature | Self-Revocation of a Stolen Device Key | Matthew kills his stolen phone's key from his laptop; other devices unaffected; safety floor against self-lockout. |
| 18 | auth/session-handoff.feature | Cross-App Session Handoff | Single-use session transfer token allows handoff from elohim-app to doorway-app. |
| 19 | auth/user-management.feature | Hosted User Management | Matthew (admin) lists, views, suspends, quotas hosted users; non-admin denied. |
| 20 | auth/visitor-boundaries.feature | Visitor Boundaries | Anonymous visitor sees commons content, blocked from network-reach + private; no JWT. |
| 21 | browser/auth-browser.feature | Browser Authentication | Login/logout via the real browser UI; clean console. |
| 22 | browser/doorway-dashboard-health.feature | Doorway Dashboard Health | Doorway admin dashboard tabs load cleanly; capabilities + auth state shown. |
| 23 | browser/navigation-browser.feature | Browser Navigation Health | Home, learning hub, profile pages load without console errors. |
| 24 | browser/spatial-map.feature | Spatial Map Renders | /map route mounts SpatialMapComponent and paints OSM tiles. |
| 25 | content/content-lifecycle.feature | Content Lifecycle | Hosted human creates content, reads own, discovers by tag; Susan and Terrance discover Matthew's content. |
| 26 | content/epr-content-addressing.feature | EPR Content Addressing | EPR links in markdown resolve cross-path and standalone; popover surfaces three-pillar metadata. |
| 27 | content/relationship-idempotency.feature | Relationship import converges under bidirectional authorship | Spouse relationship authored by both Adam and Eve creates one record; re-import idempotent. |
| 28 | content/ssr_capability.feature | SSR capability is advertised, honored, and accountable | doorway publishes derived SSR capability; storage projects it; honest degradation when missing. |
| 29 | content/stewardship-allocation.feature | Content Stewardship Allocation | Content gets multiple stewards reflecting human affinities; no exclusive ownership; allocations sum ~1.0. |
| 30 | delivery/client-resilience.feature | Client Resilience — Service Worker and Capability Negotiation | SW registers in browser + Tauri; cached app survives offline + storage restart; capability probe. |
| 31 | delivery/content-addressing.feature | Content-addressed delivery | Slug URL + CID URL serve same content; SW caches by content address; re-seed invalidates. |
| 32 | delivery/delivery-diagnostics.feature | Delivery Diagnostics — Observability and Controlled Degradation | Projection cache absorbs load; cache layer headers; operator can disable cache for diagnostics. |
| 33 | delivery/landing-page.feature | Protocol Landing Page as SPA ContentNode | Landing page loads with hero/manifesto/pillars/stats; is itself a ContentNode with SEO meta. |
| 34 | delivery/peer-mesh.feature | Peer Mesh — P2P App Delivery | Tauri household nodes serve app files to peers over LAN; EPR-resolved fallback chain; doorway optional. |
| 35 | delivery/protocol-omnibar.feature | Full-Browser Content Delivery with Protocol Omnibar | Markdown + HTML5 content render full-page; omnibar pill shows provenance + copyable EPR. |
| 36 | delivery/spa-bundle-delivery.feature | SPA Bundle Delivery — Root App from Blob Storage | Root app serves from CID blob storage with correct cache headers and SPA fallback. |
| 37 | delivery/transport-perf.feature | Transport perf parity — dual-stack promise | Iroh/libp2p parity for co-edits, UDP-blocked networks, video downloads, connection-pool, signature cap. |
| 38 | delivery/web2-absorption.feature | Web2 Absorption — Doorway Projection Cache | First load proxies + populates cache; second load all-cache; concurrent requests coalesce; replica-shared. |
| 39 | deployment/compute-commitment-bounds.feature | Compute commitments are bounded and breach without contagion | Compute breach is class-scoped; substrate verdict deterministic without elohim; elohim adds discernment. |
| 40 | deployment/conductor-admin-reachability.feature | Conductor admin WebSocket is reachable through elohim-storage | Registration succeeds without surfacing connection-refused errors. |
| 41 | deployment/conductor-visibility.feature | Conductor Pool Visibility | Matthew lists conductors + agents in the pool; checks user→conductor mapping; admin-only. |
| 42 | deployment/doorway-self-registration.feature | Doorway Self-Registration | Matthew's doorway reports orchestrator section + real hardware capacity in admin dashboard. |
| 43 | deployment/human-device-mapping.feature | Human × Device × Deployment mapping is internally consistent | Every deployed human resolves in registry; archetype matches pod resources + cluster vocabulary. |
| 44 | deployment/ingress-body-size-budget.feature | Ingress body-size budget — diversity of peers, diversity of payloads | Without chunking, 1MB default rejects bulk content; chunked POST succeeds; budget per archetype. |
| 45 | deployment/p2p-validation.feature | P2P Peer Validation | Doorway reports peers; sync paused during bulk import; sync resumes after failures. |
| 46 | deployment/peer-diversity.feature | Peer Diversity — Operations Adapt to Device Constraints | Device portfolio covers full gradient (phone, Pi, family node); phones pause sync; Pi stewards modestly. |
| 47 | deployment/persona-testnet-validation.feature | Persona Testnet — 20 Humans on One Box | Five-conductor topology with Matthew's household + faith community (Pete) + local economy peers. |
| 48 | deployment/seeder-registry-coherence.feature | Seeder respects the deployment registry | Seeder imports only deployed humans; idempotent reruns; --deployed-humans CLI override. |
| 49 | deployment/staging-validation.feature | Staging Site Validation | Staging site loads with essential page elements and git-hash validation. |
| 50 | deployment/sync-control.feature | User-controlled sync — mobile, wifi, operator's data plan | Operator pauses/resumes sync; wifi-only mode toggles on cellular; archetype defaults; status visible. |
| 51 | elohim/compute-allocation.feature | Community compute allocation | Matthew requests compute from 5 community peers; settled as REA records; budget-exhaustion degrades. |
| 52 | elohim/compute-coordination.feature | Elohim Compute Coordination | Authenticated learner receives insight within budget; deferred when exhausted; recorded as economic event. |
| 53 | elohim/content-reach-negotiation.feature | Content Reach Negotiation | Author's trust shapes reach pipeline (lightweight/standard/full); author can re-negotiate with explanation. |
| 54 | elohim/elohim-presence.feature | Elohim Presence | Learner sees elohim insight after discovery completion; constitutional reasoning transparency; cost shown. |
| 55 | elohim/network-health-posture.feature | Network Health Posture — Aggregate Awareness and Attestation-Gated Introspection | Operator sees aggregate posture from neighbor table; attestation-gated debug introspection. |
| 56 | federation/cross-doorway-content.feature | Cross-Doorway Content Discovery | Content created on alpha is discoverable from staging and vice versa. |
| 57 | federation/epr-cross-peer-resolution.feature | EPR Cross-Peer Content Resolution | Cross-peer EPR resolution with reach-gated access (community-reach + trusted-reach checks). |
| 58 | federation/peer-advertisement.feature | Peer Advertisement — The Network's Self-Awareness Heartbeat | Peer archetype profiles advertised every 30s; neighbor table aggregates + evicts stale entries. |
| 59 | federation/shard-tracking.feature | Shard Tracking for Content Auto-Recovery | Publishing content creates traceable shard assignments; invalid shard index rejected; custodian query. |
| 60 | lamad/assessment-completion-feedback.feature | Assessment Completion Feedback | Learner sees personalized discovery result (subscales) or mastery feedback after assessment. |
| 61 | lamad/attention-analytics.feature | Protocol-Native Attention Analytics | Dwell threshold creates economic event; session start/end events; learner sees own flow; steward sees engagement. |
| 62 | lamad/epr-link-navigation.feature | EPR relationship navigation boxes | Typed relationship cards beneath a concept; click navigates to target. |
| 63 | lamad/know-thyself-discovery.feature | Know Thyself Discovery Path | Learners complete Values Hierarchy + Attachment Style assessments; first discovery earns milestone attestation. |
| 64 | lamad/learning-journey.feature | Lamad Learning Journey | Starting a journey, earning affinity through navigation, attestation-gated restricted access. |
| 65 | lamad/love-map-negotiation.feature | Love Map Path Negotiation | Adam + Eve negotiate intimate-consent love-map path; mutual attestation; invisible to non-participants. |
| 66 | lamad/path-adaptation.feature | Adaptive Path Progression | Prior mastery unlocks steps; attestation gates respected; pre-assessment skip-ahead; failed quiz surfaces prerequisites. |
| 67 | protocol/landing-page-dogfood.feature | Elohim Protocol landing page is dogfooded as protocol content | Landing is itself a ContentNode with in-kind REA Commitment + protocol-signal badge. |
| 68 | protocol/protocol-omni.feature | ProtocolOmniComponent makes protocol context legible at the top of the viewport | EPR nav-context endpoint serves projection; chip + expanded toolbar with EPR identifier on protocol routes. |
| 69 | qahal/collective-governance.feature | Collective governance | Create + vote on proposals; block with justification; anonymous voting; ranked-choice curriculum; steward scoring. |
| 70 | qahal/feedback-dialogue-panel.feature | Feedback dialogue panel — accountable peer surface | Four governance feedback categories with spec's grievance vocabulary; close/Escape/click-outside. |
| 71 | resilience/observable-distribution.feature | Observable + contract-aware auto-distribute | Full placement across households; placement gaps detected; resilience tooltips + signals card + cluster page. |
| 72 | shefa/human-resilience.feature | Human Resilience Profile | Matthew alone (at-risk), with Susan (household), with Pete (community); cold-start Maria; degradation cascade. |
| 73 | shefa/m1-matthew-terrance-delivery.feature | Matthew sees real topology data after M1 substrate completion | Cluster + topology + reciprocity pages show real data; content viewer shows distribution + resilience snapshots. |
| 74 | ssr/browser-hydrates-without-flash.feature | Browser hydrates SSR'd content without a re-render flash | Concept page hydrates seamlessly without flash. |
| 75 | ssr/external-webfetch-renders-content.feature | External WebFetch renders concept HTML readable without JS | HTTP-only client (AI design tool, search engine) gets readable HTML. |
| 76 | ssr/social-card-crawler-gets-rich-preview.feature | Social card crawlers receive rich link previews | Crawlers (Twitter/Slack/Mastodon/Discord) preview a learning path step. |

---

## Section 2 — Archetype mapping (76 files)

The mapping rule: every file gets a primary archetype (or cross-cutting / infrastructure tag). Secondary archetypes are listed where the file substantively touches another archetype. Confidence reflects how cleanly the scenario in the file fits the archetype as defined in the gospel-tier spec.

| # | Path | Primary | Secondary | Confidence | Notes |
|---|---|---|---|---|---|
| 1 | auth/auth-lifecycle.feature | INF:recovery | T0:household | high | Foundational hosted-doorway identity; household is the named context for Matthew/Susan/James login. |
| 2 | auth/conductor-pool-recovery.feature | INF:recovery | INF:deployment | high | Pure operational continuity; not archetype-bound. |
| 3 | auth/fixture-humans.feature | INF:deployment | T0:household, T0:faith-community | med | Fixture sweep across categories — the categories themselves are archetype-aligned (core family = T0:household; affinity = T0:life-group; faith = T0:faith-community). |
| 4 | auth/operator-onboarding.feature | INF:deployment | INF:doorway | high | Operator bootstrap; substrate ops, not archetype. |
| 5 | auth/recovery/cross-stack/recovery-cross-stack-transport.feature | INF:recovery | INF:p2p | high | Transport-layer continuity of recovery; substrate-only. |
| 6 | auth/recovery/freeze-floor-blocks-intimate-rotation.feature | INF:recovery | CC:imago-dei | med | Freeze-floor is the substrate's expression of inherent-dignity protection — refuses operations a user under duress cannot consent to. Tag secondary. |
| 7 | auth/recovery/intimate-quorum-happy-path.feature | INF:recovery | T0:household | high | "Intimate quorum" = the household's care-economy expression of socially derived security; canonical-household-relevant. |
| 8 | auth/recovery/recovery-m5-defender-role-gate.feature | INF:recovery | CC:standing | med | Defender role is a standing-gate; the marker is a standing-claim primitive. |
| 9 | auth/recovery/recovery-m5-doorway-handoff-to-steward.feature | INF:doorway | INF:recovery | high | Graduated handoff to peer-native portal; auth-pillar convergence. |
| 10 | auth/recovery/recovery-m5-list-my-keys.feature | INF:recovery | — | high | Account-mgmt surface; substrate. |
| 11 | auth/recovery/recovery-m5-lost-key-entry.feature | INF:recovery | — | high | Routing UX in account-mgmt. |
| 12 | auth/recovery/recovery-m5-portal-host-discovery.feature | INF:recovery | INF:p2p | high | Portal-host registration; substrate. |
| 13 | auth/recovery/recovery-m5-self-revoke.feature | INF:recovery | — | high | Self-revocation flow. |
| 14 | auth/recovery/recovery-m5-vote-as-emergency-contact.feature | INF:recovery | T0:household | high | Emergency contacts are household-scale intimate-quorum members. |
| 15 | auth/recovery/recovery-shamir-optional.feature | INF:recovery | T0:household | high | Same intimate-quorum-as-household pattern. |
| 16 | auth/recovery/revocation-emergency-quorum.feature | INF:recovery | T0:household | high | Jessica + Susan are the canonical household intimate quorum. |
| 17 | auth/recovery/revocation-self.feature | INF:recovery | T0:household | high | Multi-device household pattern. |
| 18 | auth/session-handoff.feature | INF:doorway | INF:recovery | high | Substrate session continuity across apps. |
| 19 | auth/user-management.feature | INF:doorway | INF:deployment | high | Hosted-user mgmt — substrate ops. **Operator-decision note**: the underlying function (hosting humans on a doorway) is *itself* a dissolution target — at substrate maturity, doorway recedes to a projection. Flag for rewrite in Sprint 5+ to frame as transient-bridge, not enduring institution. |
| 20 | auth/visitor-boundaries.feature | CC:reach | INF:doorway | high | Commons reach vs network reach vs private — the reach axis made operational. |
| 21 | browser/auth-browser.feature | INF:browser | INF:recovery | high | Browser smoke for auth. |
| 22 | browser/doorway-dashboard-health.feature | INF:browser | INF:doorway | high | Dashboard smoke. |
| 23 | browser/navigation-browser.feature | INF:browser | — | high | Page-load smoke. |
| 24 | browser/spatial-map.feature | INF:browser | T2:city-hall, T3:natural-collective | low | The map *is* a candidate surface for spatial commons (neighborhood-association, bioregion); right now it's a smoke test. Secondary archetype is aspirational. |
| 25 | content/content-lifecycle.feature | CC:rea-flow | T0:household | high | The "create/discover" lifecycle is the household member authoring content for community-reach distribution. |
| 26 | content/epr-content-addressing.feature | CC:attestation | CC:rea-flow | high | Three-pillar metadata on the EPR Head is the attestation-substrate surface. |
| 27 | content/relationship-idempotency.feature | T0:household | CC:attestation | high | Adam-Eve marriage as bidirectional household relationship; idempotency invariant. |
| 28 | content/ssr_capability.feature | INF:ssr | CC:rea-flow | high | SSR capability advertised, honored — Section "SSR is a compute-shape capability claim" in memory. |
| 29 | content/stewardship-allocation.feature | CC:rea-flow | T0:wisdom-commons | high | Multi-steward content allocation; "no exclusive ownership" is the no-sovereignty substrate principle. |
| 30 | delivery/client-resilience.feature | INF:delivery | INF:resilience | high | SW + capability negotiation — substrate. |
| 31 | delivery/content-addressing.feature | INF:delivery | CC:attestation | high | CID = content-addressed identity; attestation-substrate-adjacent. |
| 32 | delivery/delivery-diagnostics.feature | INF:delivery | INF:doorway | high | Projection-cache observability — substrate. |
| 33 | delivery/landing-page.feature | INF:delivery | T0:wisdom-commons | med | The protocol landing is *the* federation-shape commons entry; at MVP it's a SPA, but archetypally it is the public face of the protocol-as-wisdom-commons. |
| 34 | delivery/peer-mesh.feature | INF:p2p | T0:household | high | Tauri household nodes deliver to LAN peers — household-as-resilience-unit operationalized. |
| 35 | delivery/protocol-omnibar.feature | INF:delivery | CC:attestation | high | Provenance pill + EPR-copyable — the legible-protocol-context surface. |
| 36 | delivery/spa-bundle-delivery.feature | INF:delivery | — | high | Substrate-only. |
| 37 | delivery/transport-perf.feature | INF:p2p | INF:delivery | high | Iroh/libp2p dual-stack perf parity — substrate. |
| 38 | delivery/web2-absorption.feature | INF:doorway | INF:delivery | high | Projection cache = "doorway absorbs web2 mass-read" memory. |
| 39 | deployment/compute-commitment-bounds.feature | CC:friction-gradient | CC:rea-flow | high | Bounded compute commitments, breach without contagion = the friction-gradient mechanic at compute scale; canonical to compute-commitments-are-bounded-REA-primitives memory. |
| 40 | deployment/conductor-admin-reachability.feature | INF:deployment | — | high | Operational. |
| 41 | deployment/conductor-visibility.feature | INF:deployment | INF:doorway | high | Operator visibility. |
| 42 | deployment/doorway-self-registration.feature | INF:deployment | INF:doorway | high | Doorway as peer-registration point (memory). |
| 43 | deployment/human-device-mapping.feature | INF:deployment | T0:household | med | The mapping registers household members to devices — the substrate's expression of "household horizontal scaling" memory. |
| 44 | deployment/ingress-body-size-budget.feature | INF:deployment | INF:resilience | high | Per-archetype body-size budgets — peer-diversity surface. |
| 45 | deployment/p2p-validation.feature | INF:p2p | INF:deployment | high | P2P validation smoke. |
| 46 | deployment/peer-diversity.feature | INF:deployment | INF:resilience | high | Device-archetype operational adaptation; canonical to "compute and model independent diversity surfaces" memory. |
| 47 | deployment/persona-testnet-validation.feature | INF:deployment | T0:household, T0:faith-community | high | Pete's faith-community + Matthew's household + local-economy peers — this *is* the worked-example operational seed. |
| 48 | deployment/seeder-registry-coherence.feature | INF:deployment | — | high | Operational. |
| 49 | deployment/staging-validation.feature | INF:deployment | INF:browser | high | Staging smoke. |
| 50 | deployment/sync-control.feature | INF:deployment | INF:resilience | high | Sync control + archetype-defaults; canonical to cadence-archetype-tunable memory. |
| 51 | elohim/compute-allocation.feature | T0:household | CC:rea-flow, CC:commons-elohim | high | "Compute from 5 community peers" — the household reaching into its peer neighborhood for mutual aid in compute form. |
| 52 | elohim/compute-coordination.feature | CC:commons-elohim | CC:rea-flow | high | Elohim mediates compute budget; insight delivery within budget. |
| 53 | elohim/content-reach-negotiation.feature | CC:reach | CC:standing | high | The reach engine as gospel-tier-described; trust shapes pipeline. |
| 54 | elohim/elohim-presence.feature | CC:commons-elohim | CC:imago-dei | high | Constitutional reasoning transparency = the interpretability-requirement of Section 1.5. |
| 55 | elohim/network-health-posture.feature | INF:resilience | CC:commons-elohim | high | Aggregate posture from neighbor table — the elohim's substrate-level eyes on the mesh. |
| 56 | federation/cross-doorway-content.feature | INF:doorway | T0:wisdom-commons | med | Cross-doorway = the substrate-level expression of peer federation; archetypally adjacent to wisdom-commons-federation but presently a thin doorway test. |
| 57 | federation/epr-cross-peer-resolution.feature | CC:reach | CC:standing | high | Reach-gated cross-peer resolution (community-reach + trusted-reach) — reach engine canonical. |
| 58 | federation/peer-advertisement.feature | INF:p2p | INF:resilience | high | Heartbeat + neighbor table. |
| 59 | federation/shard-tracking.feature | INF:resilience | INF:p2p | high | Shard-tracking for auto-recovery; substrate. |
| 60 | lamad/assessment-completion-feedback.feature | CC:attestation | CC:standing | high | Discovery → milestone attestation; mastery → score feedback — the standing-function inputs in motion. |
| 61 | lamad/attention-analytics.feature | CC:rea-flow | CC:standing | high | Dwell → economic event; canonical to "trust as efficiency signal" + reach memory. |
| 62 | lamad/epr-link-navigation.feature | CC:reach | CC:attestation | high | Typed relationship cards with trust signals (reach + stewardship resilience) — the EPR-content-addressing-skill memory. |
| 63 | lamad/know-thyself-discovery.feature | T0:household | CC:attestation | med | Values + Attachment discovery is intimate-scale self-knowledge — sits closest to household, though it's pre-Qahal. Could read as Imagodei surface #2 (self-knowledge); operator-decision. |
| 64 | lamad/learning-journey.feature | CC:standing | CC:attestation | high | Earning affinity + restricted access via attestations — the standing function rendered in lamad. |
| 65 | lamad/love-map-negotiation.feature | T0:household | CC:reach, CC:standing | high | Adam + Eve intimate-consent path is the archetypal household-scale love-map — "intimate" reach + "core-family" standing. |
| 66 | lamad/path-adaptation.feature | CC:standing | CC:attestation | high | Bloom-graded mastery curve (apply/analyze/evaluate/create) — canonical to Section 2.4 of the spec. |
| 67 | protocol/landing-page-dogfood.feature | T0:wisdom-commons | CC:rea-flow | high | Landing as ContentNode + in-kind REA Commitment — the protocol-substrate dogfooding itself; archetypally the federation's public surface. |
| 68 | protocol/protocol-omni.feature | CC:attestation | CC:reach | high | EPR nav-context + omni chip surfaces protocol-context — the legibility-substrate gospel-tier requirement. |
| 69 | qahal/collective-governance.feature | T0:wisdom-commons | T0:faith-community, T0:life-group | high | Proposals, votes, blocks, ranked-choice, steward scoring — generic governance applicable across plural-stewardship archetypes. **Operator-decision note**: spec says "voting is replaced by witness" (Section 7.5) — this file needs *rewrite* to align with witness-as-consensus, not vote-as-decision. See orphan Section 3. |
| 70 | qahal/feedback-dialogue-panel.feature | CC:standing | CC:imago-dei | high | Feedback dialogue with grievance vocabulary — the FeedbackSignal substrate, canonical to standing-composes-evidence-streams + reach-as-nervous-system. |
| 71 | resilience/observable-distribution.feature | INF:resilience | CC:rea-flow | high | Placement + gap detection — canonical to "placement signals are shefa inputs" memory. |
| 72 | shefa/human-resilience.feature | T0:household | T0:faith-community | high | Matthew/Susan/Pete = household → faith community → community-depth — the canonical resilience-through-relationships shape, with Pete carrying T0:faith-community. |
| 73 | shefa/m1-matthew-terrance-delivery.feature | T0:household | CC:rea-flow | high | Real-data topology + reciprocity for Matthew + Terrance's household relationship. |
| 74 | ssr/browser-hydrates-without-flash.feature | INF:ssr | — | high | SSR hydration smoke. |
| 75 | ssr/external-webfetch-renders-content.feature | INF:ssr | INF:doorway | high | Doorway as projection for non-JS clients. |
| 76 | ssr/social-card-crawler-gets-rich-preview.feature | INF:ssr | T0:wisdom-commons | med | Social crawlers preview path steps — the doorway-as-federation-projection surface. |

### Mapping summary

- **Archetype-mapped (primary is T0/T1/T2/T3):** 11 of 76
  - T0:household: 7 (auth/auth-lifecycle, content/relationship-idempotency, elohim/compute-allocation, lamad/know-thyself, lamad/love-map, shefa/human-resilience, shefa/m1)
  - T0:wisdom-commons: 3 (protocol/landing-page-dogfood, qahal/collective-governance, plus secondary on others)
  - T0:faith-community: 0 primary (all secondary — needs Sprint 5 authoring)
  - T0:life-group: 0 primary (needs Sprint 5 authoring)
  - T1/T2/T3: 0 primary (intentional — these are post-MVP)
- **Cross-cutting primary:** 16 of 76 (reach, standing, attestation, friction-gradient, commons-elohim, rea-flow, imago-dei across many files)
- **Infrastructure primary:** 49 of 76 (recovery, deployment, p2p, doorway, ssr, browser, delivery, resilience)
- **Mapping confidence:** high on 65, med on 9, low on 2 (browser/spatial-map secondary; lamad/know-thyself primary placement — both flagged operator-decision)

The asymmetry is real and informative: **the existing corpus is overwhelmingly substrate-and-cross-cutting**, with the household carrying most of the explicit archetype weight. The faith-community and life-group archetypes — central to MVP — have **zero primary scenarios**, only secondary tags. Sprint 5 has work to do.

---

## Section 3 — Orphan scenarios + disposition proposals

A scenario is "orphan" when its archetype mapping is low-confidence, when the scenario carries manifesto-era framing that contradicts the dissolution principle, or when the scenario is so substrate-specific that no archetype touches it meaningfully.

### 3.1 Rewrite candidates (manifesto-era language → functional-frame)

These scenarios carry institutional-shape framing that the gospel-tier spec's Section 2.11 dissolution principle would correct. They should not be deleted; they should be rewritten to match the architecture.

| # | File | Issue | Proposed disposition |
|---|---|---|---|
| 69 | qahal/collective-governance.feature | "Create a proposal", "Vote on a proposal" — vote-as-decision; but Section 7.5 says councils *witness*, they do not vote. | **Rewrite.** Reframe scenarios around witness-publication-and-affected-Qahal-response, not majority-vote-binding. Keep the proposal-authoring surface; replace vote → witness; preserve "Block a proposal with justification" as "Steward refuses commons participation in the proposed action" (witness with refusal). Anonymous-voting scenario is most at risk — anonymous witness defeats the substrate's interpretability requirement; mark for **operator decision** whether to keep or graduate. |
| 19 | auth/user-management.feature | "Hosted User Management" — perpetuates the hosting-platform shape that doorway-as-projection ultimately dissolves. | **Keep + reclassify** as `INF:doorway` (already done) but flag in Section 6 migration for *rewrite* in Sprint 5 to frame as "doorway operator transient steward of hosted humans during graduation to peer-native" — explicit dissolution arc. |
| 67 | protocol/landing-page-dogfood.feature | "Matthew's hosting agreement" reads as commercial-hosting — but the substrate's framing is in-kind REA Commitment to wisdom commons. | **Keep + reclassify** as T0:wisdom-commons (already done). The in-kind REA Commitment + protocol-signal badge are *correctly* substrate-aligned; the title alone reads ambiguous. Minor scenario rewording suggested, not a full rewrite. |

### 3.2 Graduate candidates (lesson canonical → archive)

Scenarios whose lesson is already absorbed into the substrate and which no longer need to live as active a2o coverage.

| # | File | Why graduate | Memory anchor (if applicable) |
|---|---|---|---|
| 40 | deployment/conductor-admin-reachability.feature | Single-scenario regression for a bug already absorbed; the substrate now reaches conductor admin reliably; this is anti-regression rather than ongoing learner experience. | feedback_a2o_is_human_experience_not_dev_bugs — should never have been a feature file. Graduate to a unit/integration test. |
| 49 | deployment/staging-validation.feature | Generic SPA smoke + git-hash check; not human-experience-bearing. | Graduate to deployment-pipeline assertion. |
| 23 | browser/navigation-browser.feature | Page-load smoke for known routes; not human-experience-bearing. | Graduate to Cypress smoke suite (still maintained, but outside .feature corpus). |
| 27 | content/relationship-idempotency.feature | "UNIQUE constraint does not fail the seed" — this is anti-regression seed hygiene, not learner experience. Adam-Eve marriage as bidirectional household relationship is a *real* T0:household scenario; the idempotency-protection should move to unit-test land. | **Operator decision** whether to graduate the whole file or split: keep "spouse relationship authored by both parties is created once" as T0:household; graduate the import/idempotency scenarios. |
| 28 | content/ssr_capability.feature | SSR capability negotiation is substrate plumbing; the human experience is "I see content on a slow connection" — covered by ssr/browser-hydrates-without-flash + ssr/external-webfetch-renders-content. | **Operator decision**: graduate the capability-negotiation scenarios to substrate-level integration tests, keep the human-facing SSR-as-experience scenarios. |

### 3.3 Memorialize candidates (deep archive with story pointer)

Scenarios that captured a specific moment in the substrate's evolution that should be preserved but is no longer actively maintained.

| # | File | Why memorialize | Story pointer |
|---|---|---|---|
| 47 | deployment/persona-testnet-validation.feature | The "20 humans on one box" persona testnet is a milestone in P2P validation; its specific 5-conductor topology shape is operationally superseded by the alpha-cluster topology (6-peer; adam+matthew bootstrap pair; shem multi-tenant). The lesson — that the household + faith-community + local-economy topology can run on commodity hardware — is canonical and worth memorializing. | Sprint 0.5 archaeology: this file becomes the canonical reference for "what 20 humans look like when seeded" — preserved but no longer actively run. |
| 73 | shefa/m1-matthew-terrance-delivery.feature | The M1 substrate completion is a historical milestone; the specific "after M1" framing is bound to that moment. Matthew + Terrance is canonical to the household-archetype story but the scenario framing is M1-bound. | Sprint 0.5: split — extract the canonical Matthew + Terrance topology scenarios into a T0:household-relevant file with present-tense framing; memorialize the M1-completion-specific framing. |

### 3.4 Keep + reclassify (lower-obvious archetype fit)

Scenarios that fit an archetype the original taxonomy didn't recognize.

| # | File | New archetype mapping | Rationale |
|---|---|---|---|
| 7 | auth/recovery/intimate-quorum-happy-path.feature | T0:household + INF:recovery (already mapped) | The intimate quorum *is* the household care-economy expressed as cryptographic recovery; the household-archetype assignment is the gospel-tier alignment. |
| 65 | lamad/love-map-negotiation.feature | T0:household primary (already mapped) | Adam + Eve love-map is the canonical intimate-scale Qahal path; the archetypal household. |
| 24 | browser/spatial-map.feature | INF:browser primary, T3:natural-collective + T2:city-hall secondary | The map *will become* the spatial commons surface for bioregional + neighborhood-association Qahals; currently a smoke test. Operator should decide whether to author Sprint 5 scenarios extending the spatial map to a real T3:natural-collective surface, or leave as smoke. |
| 39 | deployment/compute-commitment-bounds.feature | CC:friction-gradient primary (already mapped) | This scenario *is* the friction-gradient at compute scale — bounded commitments, breach without contagion. Promote from "deployment" to friction-gradient cross-cutting. |
| 70 | qahal/feedback-dialogue-panel.feature | CC:standing primary, CC:imago-dei secondary (already mapped) | The four-category grievance vocabulary is the FeedbackSignal substrate; promote from "qahal" pillar bucket to the standing-cross-cutting bucket. |

### 3.5 Hold (no story yet; defer to next cycle)

Scenarios that don't fit the current archetype catalog but also don't justify graduate/memorialize. The librarian holds for the next memory ceremony cycle.

| # | File | Hold rationale |
|---|---|---|
| 63 | lamad/know-thyself-discovery.feature | The know-thyself discovery surface is genuinely cross-cutting — it precedes Qahal participation (it builds the imagodei profile that Qahals then render through their lenses). It fits "Imagodei three surfaces" memory (social profile / self-knowledge / account mgmt — self-knowledge surface #2). Hold until a Sprint 1 brainstorm clarifies whether self-knowledge gets its own archetype-adjacent bucket or stays cross-cutting. **Operator decision** suggested. |
| 33 | delivery/landing-page.feature | The landing page sits between INF:delivery (it is rendered by the delivery pipeline) and T0:wisdom-commons (it is the federation's public face). The protocol/landing-page-dogfood.feature already covers the wisdom-commons-archetype framing. Hold the basic-rendering-smoke scenarios in INF:delivery; consider whether to consolidate with landing-page-dogfood. |

---

## Section 4 — Gap analysis: archetypes with no scenarios

The mapping in Section 2 shows that **T0:faith-community** and **T0:life-group** have zero primary scenarios — the two archetypes most critical to MVP after the household. The wisdom-commons federation has three (landing-page-dogfood, collective-governance, stewardship-allocation as secondary). Tier 1+2 and Tier 3 are intentionally empty (post-MVP).

Sprint 5 must author against these gaps. The proposed scenarios below are seeds — titles + one-sentence summaries — not full Gherkin. Storyteller in Sprint 5 elaborates each.

### 4.1 T0:household — has 7 scenarios, but gaps remain

The household has decent coverage but is missing the **commons-elohim's quiet witness** scenarios — the right-nav panel sentence, the care-economy ledger, the ambient-not-pushy notification pattern. These are *the* Section 4.1 canonical narrative beats.

Proposed Sprint 5 scenarios:
1. **Commons-elohim witnesses care without notification** — Sheila drops off soup; the household stream notes "care given" in the smallest font; no chirp, no badge.
2. **Member ring shows household standing as ring thickness** — Jessica's ring is full; Matthew's is nearly full; James's is small (eleven, sick week); Susan's is muted (visibility-only-when-active).
3. **Care-economy ledger accumulates over years without becoming a debt** — Sheila's pattern of sending recipes when someone is sick is the slow truth the family already knows.
4. **Gertrude's check-in propagates through the household commons-elohim** — without anyone telling it, the elohim knows Gertrude considers the kids her people; quiet cross-household witness.
5. **Household reach into wider commons stays steady by default** — no one outside core-family knows James was sick unless the household chose to extend reach.

### 4.2 T0:faith-community — ZERO primary scenarios (MVP-critical gap)

The Restoration Movement congregation is the central plural-stewardship example in the spec and has no scenarios. This is the largest authoring task for Sprint 5.

Proposed Sprint 5 scenarios:
1. **Plural elders share stewardship without hierarchy** — Brother Cal is one of four elders; no one outranks; the substrate shows them slightly larger in the member ring but carries no ranking-above relation.
2. **Congregation rubric is a versioned EPR authored by elders + ratified by congregation** — rubric carries Bloom-graded mastery curve (recall, application, evaluation, creation) and is revised in seasons.
3. **New family appears in member ring with welcomed-into-fellowship ring** — standing grows through showing up, confession, baptism, communion service; mastery-attested gate; no exclusion-by-newness.
4. **Friction-gradient flags one elder accruing disproportionate attestation weight** — commons-elohim writes a small line to the elders: "the body is leaning toward Brother Cal's voice this season; consider whether this reflects gifting or accumulation."
5. **Congregation commons-elohim writes one-paragraph witness in right-nav** — does not preach; witnesses (reach into neighborhood rising; three life-groups at cohesion threshold; sermon series received).
6. **Sunday morning: substrate goes quiet** — the protocol respects that screens are not for Sunday mornings; no notifications during gathering.
7. **Prayer request reach extends from household to congregation by member choice** — Matthew chooses to ask the congregation to pray for James; the rubric handles reach extension; prayer arrives at right faces in right life-group with right detail and no more.

### 4.3 T0:life-group — ZERO primary scenarios (MVP-critical gap)

The life-group as sub-Qahal nesting (Section 4.3) is the architectural test of holonic composition. Also zero coverage.

Proposed Sprint 5 scenarios:
1. **Life-group rubric inherits from congregation + adds local customization** — John Hardin's "presence kept, prayer attested, vulnerability offered, care shown across the week" customization, ratified by the eight households, anchored to congregation rubric.
2. **Standing in life-group is partially derived from standing in parent congregation** — substrate carries this as a hard rule; visitors welcomed with "visiting" ring; full life-group standing requires congregational fellowship threshold.
3. **Prayer attestations are cryptographically signed but encrypted to the group** — not published, not searched, not analytics; commons memory of the life-group, available to group when group wants to remember.
4. **Commons-elohim notes life-group has reached cohesion threshold for mission-engagement** — does not assign a project; witnesses the moment the rubric points to and lets the life-group decide.
5. **Friction-gradient prompts host rotation** — substrate notices John Hardin has hosted 23 of last 24 Tuesdays; commons-elohim writes a small line; John (who hoped for the prompt) accepts; Lees offer; substrate updates rotation.
6. **Life-group's commons stream visible only to families inside** — substrate honors the choice without making the choice into a wall.
7. **Reach extension to broader congregation by life-group steward consent** — life-group can propagate prayer or care request upward through the holonic nesting with the appropriate detail-elision.

### 4.4 T0:wisdom-commons — 3 scenarios; significant gaps remain

Landing-page-dogfood + collective-governance + stewardship-allocation give partial coverage. The Section 4.4 federation-without-hierarchy narrative is mostly unexpressed.

Proposed Sprint 5 scenarios:
1. **Brother Cal opens a concern surface as peer to peer** — names sister congregation by name, cites apostolic passages, attaches rubric reference, signs and sends; not denouncement, not silence, peer accountability.
2. **Peer council convenes voluntarily on every side** — sister congregation's elder is invited, not summoned; convening has no authority to bind, only to listen and to speak; commons-elohim witnesses the convening with a short factual public line.
3. **Peer council produces a witness, not a verdict** — written jointly, offered to affected congregations; each congregation reads it; each chooses what to do with it; no congregation lost autonomy.
4. **Friction-gradient verifies witness was not authored by any one elder** — substrate refuses councils that produce one-voice-dominant witnesses; canonical anti-concentration at federation scale.
5. **Wisdom flows horizontally as REA gift events** — sermon series outline, neighborhood-meals ministry detail, theological essay, prayer request, grief note; the stream is horizontal; all items are offered, none commanded.
6. **Federation Qahal has no authority over any participating congregation** — no upward link to institutional parent; rubric is a *template*, each congregation's local rubric anchors to template only by choice.
7. **Reconciliation recorded as REA event** — Arkansas congregation sits with witness two months, writes back that it has revised; Brother Cal's congregation writes back grateful, fellowship continues; substrate records the resolution without anyone losing autonomy.

### 4.5 Cross-cutting gaps

A few cross-cutting concerns are visible in the corpus but under-articulated:

- **CC:imago-dei** — present only as a secondary tag on three files (freeze-floor, elohim-presence, feedback-dialogue). Section 1.5 of the spec is gospel-tier; the discriminator should have its own substrate-floor scenarios. Proposed: "Substrate refuses a Qahal rubric configuration that denies dignity to any being" (the most fundamental substrate guarantee); "Substrate carries witness-of-harm + attestation-of-repair + ongoing-acknowledgment as distinct REA primitives" (the Foster reconciliation frame).
- **CC:commons-elohim** — present in elohim-pillar files but the *per-Qahal* commons-elohim co-steward (not the per-human elohim) has no dedicated scenarios. Proposed: "Household commons-elohim quietly writes one sentence in right-nav, no chirp, no badge"; "Congregation commons-elohim witnesses but does not preach"; "Wisdom commons commons-elohim is itself a peer-elohim, not magisterial".
- **CC:friction-gradient** — well-covered at compute scale (compute-commitment-bounds) but missing at *Qahal scale*. Proposed: "Substrate notices elder accruing disproportionate attestation weight"; "Substrate prompts host rotation when one household hosts 23 of 24 Tuesdays"; "Substrate refuses rubric updates that would centralize authority without sibling-council validation".

### 4.6 Tier 1+2 — zero scenarios, intentional

The catalog stubs are post-MVP. Authoring is deferred to Sprint 6+ once the Tier 0 four are landed. **Do not author Tier 1+2 scenarios in Sprint 5 unless explicitly scoped.** Listed here for awareness:
- T1:eae (ChickenMax → EAE — anchor work exists in genesis/docs/content/elohim-protocol/autonomous_entity/epic.md)
- T1:grocery-coop, T1:farm-csa, T1:distribution-center, T1:factory-intimate, T1:industry-association, T1:library, T1:neighborhood-association, T2:city-hall — all stubs in Section 5.

### 4.7 Tier 3 — zero scenarios, intentional + far horizon

All 18 Tier 3 archetypes are post-MVP per Section 6 of the spec. The most architecturally consequential — T3:natural-collective (6.17) and T3:identity-collective (6.18, the Imago Dei red-team test) — should be tracked as substrate-extension requirements, not as Sprint 5 authoring targets.

---

## Section 5 — Proposed new directory taxonomy

The existing taxonomy is mixed-axis (pillar + implementation-shape + content-surface). The proposed taxonomy puts **collective-archetype** as the primary axis, **infrastructure** as the secondary axis, and **cross-cutting** as the tertiary axis. This aligns the corpus with the gospel-tier spec without requiring Tier 1+2 / Tier 3 directories that would sit empty.

```
genesis/a2o/features/
├── archetypes/
│   ├── household/                # T0 — Section 4.1
│   ├── faith-community/          # T0 — Section 4.2 (created empty; Sprint 5 populates)
│   ├── life-group/               # T0 — Section 4.3 (created empty; Sprint 5 populates)
│   └── wisdom-commons/           # T0 — Section 4.4
│
├── cross-cutting/
│   ├── reach/                    # Outward visibility / nervous system
│   ├── standing/                 # Inward Qahal capability surface (incl. FeedbackSignal)
│   ├── attestation/              # Recognition + provenance primitives (EPRs, mastery, lamad)
│   ├── friction-gradient/        # Anti-concentration substrate
│   ├── commons-elohim/           # Per-Qahal co-steward (elohim-pillar files)
│   ├── rea-flow/                 # REA Commitment/Fulfillment/Event mechanics
│   └── imago-dei/                # Inherent-dignity discriminator (Section 1.5)
│
└── infrastructure/
    ├── recovery/                 # Key recovery / identity continuity
    ├── doorway/                  # Web2 projection gateway
    ├── p2p/                      # Peer mesh / federation transport
    ├── delivery/                 # Content delivery (CIDs, omnibar, projection cache)
    ├── ssr/                      # Server-side rendering capability
    ├── browser/                  # Browser smoke / hosted-human shape
    ├── deployment/               # Operational topology / device modeling
    └── resilience/               # Placement, gap detection, contract-aware distribution
```

### Why no Tier 1+2 / Tier 3 directories yet

Per the directive in the task brief: "Don't include archetypes with zero scenarios unless they're Tier 0." T1:eae through T2:city-hall and all T3 archetypes are intentionally absent from the directory tree until Sprint 6+ authors them. When that happens, the structure extends naturally:

```
├── archetypes/
│   ├── tier-0/                   # MVP — Section 4
│   │   ├── household/
│   │   ├── faith-community/
│   │   ├── life-group/
│   │   └── wisdom-commons/
│   ├── tier-1-2/                 # Everyday + civic — Section 5
│   │   ├── eae/                  # Per-archetype subdirs added as scenarios author
│   │   └── ...
│   └── tier-3/                   # Stafford Beer endgame — Section 6
│       └── ...
```

**Operator decision**: keep the flat `archetypes/{household,faith-community,life-group,wisdom-commons}/` for MVP, or pre-emptively bucket as `archetypes/tier-0/{...}/` to set up the tier extension? My recommendation: **flat for MVP** to keep paths short; introduce tier subdirs when Tier 1+2 authoring begins.

### Why `cross-cutting/` separated from `archetypes/`

The cross-cutting concerns (reach, standing, attestation, friction-gradient, commons-elohim, rea-flow, imago-dei) are substrate-level invariants that apply to every archetype. Folding them into individual archetype directories would force duplication and obscure the recursive substrate property. Keeping them as a top-level peer of `archetypes/` mirrors the spec's structure (Section 2 architecture is cross-archetype; Sections 4–6 are per-archetype).

### Why `infrastructure/` separated

Infrastructure files (recovery, doorway, p2p, etc.) are substrate plumbing that enables archetypes but is not itself archetype-bound. The current taxonomy already separates these (the existing `auth/`, `browser/`, `delivery/`, `deployment/`, `federation/`, `protocol/`, `ssr/` directories are infrastructure-shape). The proposal regularizes the names and collapses duplicates (e.g., `federation/` and parts of `delivery/` both touch P2P; consolidating to `p2p/` is cleaner).

### Tag taxonomy alignment (recommended secondary)

Every .feature file already carries Cucumber tags (`@e2e`, `@lamad`, `@qahal`, etc.). The migration should standardize tags to mirror the directory taxonomy:
- Primary archetype: `@archetype:household`, `@archetype:faith-community`, `@archetype:wisdom-commons`, etc.
- Cross-cutting: `@cc:reach`, `@cc:standing`, `@cc:friction-gradient`, etc.
- Infrastructure: `@inf:recovery`, `@inf:doorway`, etc.
- Tier: `@tier:0`, `@tier:1`, `@tier:3` for archetype scenarios.

The existing pillar tags (`@lamad`, `@qahal`, `@shefa`) can be retained as legacy aliases but should be deprecated in favor of archetype + cross-cutting tags. This is a tag-rename pass for Sprint 5+ to land.

---

## Section 6 — Migration plan (file-by-file)

This section names the proposed source-to-destination move for each of the 76 files. **No file moves are executed here.** Operator drives the actual move in a later sprint. Files marked SPLIT need to be divided across multiple destinations; MERGE-CANDIDATE notes when a file's scenarios could fold into another file; RENAME notes where the file's name should change to match the new taxonomy.

### 6.1 Files moving to `archetypes/household/`

| # | Source | Destination | Action |
|---|---|---|---|
| 1 | auth/auth-lifecycle.feature | infrastructure/recovery/auth-lifecycle.feature | move + RENAME-OPTIONAL (the file is auth-flow not archetype) |
| 17 | auth/recovery/revocation-self.feature | infrastructure/recovery/revocation-self.feature | move (household secondary preserved via tag) |
| 7 | auth/recovery/intimate-quorum-happy-path.feature | infrastructure/recovery/intimate-quorum-happy-path.feature | move (household-flavored intimate quorum) |
| 16 | auth/recovery/revocation-emergency-quorum.feature | infrastructure/recovery/revocation-emergency-quorum.feature | move |
| 14 | auth/recovery/recovery-m5-vote-as-emergency-contact.feature | infrastructure/recovery/recovery-m5-vote-as-emergency-contact.feature | move |
| 15 | auth/recovery/recovery-shamir-optional.feature | infrastructure/recovery/recovery-shamir-optional.feature | move |
| 27 | content/relationship-idempotency.feature | archetypes/household/spouse-bidirectional-authorship.feature | move + SPLIT (graduate idempotency scenarios out; keep "spouse relationship authored by both parties is created once" as T0:household) + RENAME |
| 51 | elohim/compute-allocation.feature | archetypes/household/compute-mutual-aid.feature | move + RENAME (Matthew requests compute from 5 community peers = household mutual-aid at compute scale) |
| 63 | lamad/know-thyself-discovery.feature | HOLD — operator decision (cross-cutting/attestation/ or archetypes/household/) | hold |
| 65 | lamad/love-map-negotiation.feature | archetypes/household/love-map-negotiation.feature | move (Adam + Eve canonical household intimate-scale path) |
| 72 | shefa/human-resilience.feature | archetypes/household/human-resilience-profile.feature | move (Matthew alone → +Susan → +Pete is the canonical household-extending-into-community resilience shape) |
| 73 | shefa/m1-matthew-terrance-delivery.feature | archetypes/household/matthew-terrance-real-topology.feature | move + SPLIT (extract Matthew+Terrance canonical T0:household scenarios; memorialize the M1-completion framing) + RENAME |

### 6.2 Files moving to `archetypes/faith-community/`

(All files in this section need to be authored in Sprint 5 — see Section 4.2. No existing files map directly as primary.)

| # | Source | Destination | Action |
|---|---|---|---|
| — | Sprint 5 new | archetypes/faith-community/plural-elder-stewardship.feature | author |
| — | Sprint 5 new | archetypes/faith-community/congregation-rubric.feature | author |
| — | Sprint 5 new | archetypes/faith-community/new-family-welcomed-into-fellowship.feature | author |
| — | Sprint 5 new | archetypes/faith-community/elder-attestation-weight-flagged.feature | author |
| — | Sprint 5 new | archetypes/faith-community/commons-elohim-witness.feature | author |
| — | Sprint 5 new | archetypes/faith-community/sunday-morning-substrate-quiet.feature | author |
| — | Sprint 5 new | archetypes/faith-community/prayer-request-reach-extension.feature | author |

### 6.3 Files moving to `archetypes/life-group/`

(All Sprint 5 — see Section 4.3.)

| # | Source | Destination | Action |
|---|---|---|---|
| — | Sprint 5 new | archetypes/life-group/rubric-inheritance-with-customization.feature | author |
| — | Sprint 5 new | archetypes/life-group/standing-partially-derived-from-parent.feature | author |
| — | Sprint 5 new | archetypes/life-group/prayer-attestation-encrypted-to-group.feature | author |
| — | Sprint 5 new | archetypes/life-group/cohesion-threshold-mission-engagement.feature | author |
| — | Sprint 5 new | archetypes/life-group/host-rotation-friction-gradient.feature | author |
| — | Sprint 5 new | archetypes/life-group/commons-stream-visible-only-inside.feature | author |
| — | Sprint 5 new | archetypes/life-group/upward-reach-by-steward-consent.feature | author |

### 6.4 Files moving to `archetypes/wisdom-commons/`

| # | Source | Destination | Action |
|---|---|---|---|
| 67 | protocol/landing-page-dogfood.feature | archetypes/wisdom-commons/landing-page-as-content.feature | move + RENAME |
| 69 | qahal/collective-governance.feature | archetypes/wisdom-commons/proposal-and-witness.feature | move + **REWRITE** (vote-as-decision → witness-publication; see Section 3.1) + SPLIT (the curriculum-ranked-choice scenario may fit T0:life-group better — operator decision) |
| 29 | content/stewardship-allocation.feature | archetypes/wisdom-commons/multi-steward-content-allocation.feature | move (content stewarded by multiple humans = federation peer-stewardship pattern) + RENAME |
| — | Sprint 5 new | archetypes/wisdom-commons/peer-concern-surface.feature | author |
| — | Sprint 5 new | archetypes/wisdom-commons/peer-council-convenes-voluntarily.feature | author |
| — | Sprint 5 new | archetypes/wisdom-commons/witness-not-verdict.feature | author |
| — | Sprint 5 new | archetypes/wisdom-commons/friction-gradient-witness-authorship.feature | author |
| — | Sprint 5 new | archetypes/wisdom-commons/horizontal-wisdom-rea-events.feature | author |
| — | Sprint 5 new | archetypes/wisdom-commons/no-upward-institutional-link.feature | author |
| — | Sprint 5 new | archetypes/wisdom-commons/reconciliation-rea-event.feature | author |

### 6.5 Files moving to `cross-cutting/reach/`

| # | Source | Destination | Action |
|---|---|---|---|
| 20 | auth/visitor-boundaries.feature | cross-cutting/reach/visitor-boundaries.feature | move |
| 53 | elohim/content-reach-negotiation.feature | cross-cutting/reach/content-reach-negotiation.feature | move |
| 57 | federation/epr-cross-peer-resolution.feature | cross-cutting/reach/epr-cross-peer-resolution.feature | move (reach-gated cross-peer is the reach engine at federation scale) |
| 62 | lamad/epr-link-navigation.feature | cross-cutting/reach/epr-link-navigation.feature | move (reach + stewardship resilience trust signals on link cards) |

### 6.6 Files moving to `cross-cutting/standing/`

| # | Source | Destination | Action |
|---|---|---|---|
| 64 | lamad/learning-journey.feature | cross-cutting/standing/learning-journey-affinity.feature | move + RENAME |
| 66 | lamad/path-adaptation.feature | cross-cutting/standing/bloom-graded-path-adaptation.feature | move + RENAME (canonical to Section 2.4 Bloom curve) |
| 70 | qahal/feedback-dialogue-panel.feature | cross-cutting/standing/feedback-dialogue-panel.feature | move (FeedbackSignal substrate = standing input) |

### 6.7 Files moving to `cross-cutting/attestation/`

| # | Source | Destination | Action |
|---|---|---|---|
| 26 | content/epr-content-addressing.feature | cross-cutting/attestation/epr-content-addressing.feature | move |
| 60 | lamad/assessment-completion-feedback.feature | cross-cutting/attestation/assessment-completion-feedback.feature | move |
| 68 | protocol/protocol-omni.feature | cross-cutting/attestation/protocol-omni-nav-context.feature | move + RENAME |

### 6.8 Files moving to `cross-cutting/friction-gradient/`

| # | Source | Destination | Action |
|---|---|---|---|
| 39 | deployment/compute-commitment-bounds.feature | cross-cutting/friction-gradient/compute-commitment-bounds.feature | move |
| — | Sprint 5 new | cross-cutting/friction-gradient/elder-attestation-flattening.feature | author |
| — | Sprint 5 new | cross-cutting/friction-gradient/host-rotation-prompt.feature | author |
| — | Sprint 5 new | cross-cutting/friction-gradient/rubric-centralization-refused.feature | author |

### 6.9 Files moving to `cross-cutting/commons-elohim/`

| # | Source | Destination | Action |
|---|---|---|---|
| 52 | elohim/compute-coordination.feature | cross-cutting/commons-elohim/compute-coordination.feature | move |
| 54 | elohim/elohim-presence.feature | cross-cutting/commons-elohim/elohim-presence.feature | move (interpretability + constitutional reasoning transparency) |

### 6.10 Files moving to `cross-cutting/rea-flow/`

| # | Source | Destination | Action |
|---|---|---|---|
| 25 | content/content-lifecycle.feature | cross-cutting/rea-flow/content-lifecycle.feature | move (create/discover lifecycle = REA author/event/witness) |
| 61 | lamad/attention-analytics.feature | cross-cutting/rea-flow/attention-analytics.feature | move (dwell → economic event) |

### 6.11 Files moving to `cross-cutting/imago-dei/`

Currently no primary files; needs Sprint 5 authoring:

| # | Source | Destination | Action |
|---|---|---|---|
| — | Sprint 5 new | cross-cutting/imago-dei/substrate-refuses-dignity-denial.feature | author |
| — | Sprint 5 new | cross-cutting/imago-dei/witness-harm-attestation-repair.feature | author |

### 6.12 Files moving to `infrastructure/recovery/`

| # | Source | Destination | Action |
|---|---|---|---|
| 1 | auth/auth-lifecycle.feature | infrastructure/recovery/auth-lifecycle.feature | (already listed) |
| 2 | auth/conductor-pool-recovery.feature | infrastructure/recovery/conductor-pool-recovery.feature | move |
| 5 | auth/recovery/cross-stack/recovery-cross-stack-transport.feature | infrastructure/recovery/cross-stack-transport.feature | move + flatten subdir |
| 6 | auth/recovery/freeze-floor-blocks-intimate-rotation.feature | infrastructure/recovery/freeze-floor-blocks-intimate-rotation.feature | move |
| 7 | auth/recovery/intimate-quorum-happy-path.feature | infrastructure/recovery/intimate-quorum-happy-path.feature | (already listed) |
| 8 | auth/recovery/recovery-m5-defender-role-gate.feature | infrastructure/recovery/m5-defender-role-gate.feature | move + RENAME |
| 10 | auth/recovery/recovery-m5-list-my-keys.feature | infrastructure/recovery/m5-list-my-keys.feature | move + RENAME |
| 11 | auth/recovery/recovery-m5-lost-key-entry.feature | infrastructure/recovery/m5-lost-key-entry.feature | move + RENAME |
| 12 | auth/recovery/recovery-m5-portal-host-discovery.feature | infrastructure/recovery/m5-portal-host-discovery.feature | move + RENAME |
| 13 | auth/recovery/recovery-m5-self-revoke.feature | infrastructure/recovery/m5-self-revoke.feature | move + RENAME |
| 14 | auth/recovery/recovery-m5-vote-as-emergency-contact.feature | infrastructure/recovery/m5-vote-as-emergency-contact.feature | (already listed) |
| 15 | auth/recovery/recovery-shamir-optional.feature | infrastructure/recovery/shamir-optional.feature | (already listed) + RENAME |
| 16 | auth/recovery/revocation-emergency-quorum.feature | infrastructure/recovery/revocation-emergency-quorum.feature | (already listed) |
| 17 | auth/recovery/revocation-self.feature | infrastructure/recovery/revocation-self.feature | (already listed) |

### 6.13 Files moving to `infrastructure/doorway/`

| # | Source | Destination | Action |
|---|---|---|---|
| 4 | auth/operator-onboarding.feature | infrastructure/doorway/operator-onboarding.feature | move |
| 9 | auth/recovery/recovery-m5-doorway-handoff-to-steward.feature | infrastructure/doorway/handoff-to-steward.feature | move + RENAME |
| 18 | auth/session-handoff.feature | infrastructure/doorway/session-handoff.feature | move |
| 19 | auth/user-management.feature | infrastructure/doorway/user-management.feature | move + flag for **REWRITE** (Section 3.1: frame as transient-bridge) |
| 22 | browser/doorway-dashboard-health.feature | infrastructure/doorway/dashboard-health.feature | move + RENAME |
| 38 | delivery/web2-absorption.feature | infrastructure/doorway/projection-cache.feature | move + RENAME |
| 42 | deployment/doorway-self-registration.feature | infrastructure/doorway/self-registration.feature | move + RENAME |
| 56 | federation/cross-doorway-content.feature | infrastructure/doorway/cross-doorway-content.feature | move |

### 6.14 Files moving to `infrastructure/p2p/`

| # | Source | Destination | Action |
|---|---|---|---|
| 34 | delivery/peer-mesh.feature | infrastructure/p2p/peer-mesh.feature | move |
| 37 | delivery/transport-perf.feature | infrastructure/p2p/transport-perf-dual-stack.feature | move + RENAME |
| 45 | deployment/p2p-validation.feature | infrastructure/p2p/peer-validation.feature | move + RENAME |
| 58 | federation/peer-advertisement.feature | infrastructure/p2p/peer-advertisement.feature | move |

### 6.15 Files moving to `infrastructure/delivery/`

| # | Source | Destination | Action |
|---|---|---|---|
| 30 | delivery/client-resilience.feature | infrastructure/delivery/client-resilience.feature | move |
| 31 | delivery/content-addressing.feature | infrastructure/delivery/content-addressing.feature | move |
| 32 | delivery/delivery-diagnostics.feature | infrastructure/delivery/delivery-diagnostics.feature | move |
| 33 | delivery/landing-page.feature | infrastructure/delivery/landing-page.feature | move (consider MERGE with archetypes/wisdom-commons/landing-page-as-content — operator decision) |
| 35 | delivery/protocol-omnibar.feature | infrastructure/delivery/protocol-omnibar.feature | move |
| 36 | delivery/spa-bundle-delivery.feature | infrastructure/delivery/spa-bundle-delivery.feature | move |

### 6.16 Files moving to `infrastructure/ssr/`

| # | Source | Destination | Action |
|---|---|---|---|
| 28 | content/ssr_capability.feature | infrastructure/ssr/ssr-capability.feature | move + RENAME (snake → kebab) — **operator decision** whether to graduate per Section 3.2 |
| 74 | ssr/browser-hydrates-without-flash.feature | infrastructure/ssr/browser-hydrates-without-flash.feature | move |
| 75 | ssr/external-webfetch-renders-content.feature | infrastructure/ssr/external-webfetch-renders-content.feature | move |
| 76 | ssr/social-card-crawler-gets-rich-preview.feature | infrastructure/ssr/social-card-crawler-gets-rich-preview.feature | move |

### 6.17 Files moving to `infrastructure/browser/`

| # | Source | Destination | Action |
|---|---|---|---|
| 21 | browser/auth-browser.feature | infrastructure/browser/auth-browser.feature | move |
| 23 | browser/navigation-browser.feature | infrastructure/browser/navigation-browser.feature | move (consider graduate per Section 3.2 — operator decision) |
| 24 | browser/spatial-map.feature | infrastructure/browser/spatial-map.feature | move (T3:natural-collective secondary preserved via tag) |

### 6.18 Files moving to `infrastructure/deployment/`

| # | Source | Destination | Action |
|---|---|---|---|
| 3 | auth/fixture-humans.feature | infrastructure/deployment/fixture-humans.feature | move |
| 40 | deployment/conductor-admin-reachability.feature | infrastructure/deployment/conductor-admin-reachability.feature | move (consider graduate per Section 3.2) |
| 41 | deployment/conductor-visibility.feature | infrastructure/deployment/conductor-visibility.feature | move |
| 43 | deployment/human-device-mapping.feature | infrastructure/deployment/human-device-mapping.feature | move |
| 44 | deployment/ingress-body-size-budget.feature | infrastructure/deployment/ingress-body-size-budget.feature | move |
| 46 | deployment/peer-diversity.feature | infrastructure/deployment/peer-diversity.feature | move |
| 47 | deployment/persona-testnet-validation.feature | infrastructure/deployment/persona-testnet-validation.feature | move + flag for **MEMORIALIZE** per Section 3.3 |
| 48 | deployment/seeder-registry-coherence.feature | infrastructure/deployment/seeder-registry-coherence.feature | move |
| 49 | deployment/staging-validation.feature | infrastructure/deployment/staging-validation.feature | move (consider graduate per Section 3.2) |
| 50 | deployment/sync-control.feature | infrastructure/deployment/sync-control.feature | move |

### 6.19 Files moving to `infrastructure/resilience/`

| # | Source | Destination | Action |
|---|---|---|---|
| 55 | elohim/network-health-posture.feature | infrastructure/resilience/network-health-posture.feature | move (commons-elohim secondary preserved via tag) |
| 59 | federation/shard-tracking.feature | infrastructure/resilience/shard-tracking.feature | move |
| 71 | resilience/observable-distribution.feature | infrastructure/resilience/observable-distribution.feature | move |

### 6.20 Migration plan summary

- **Files moving with simple path change:** 53
- **Files needing RENAME:** ~22 (kebab-case normalization; `recovery-m5-*` prefix dropped; `_` → `-`)
- **Files needing SPLIT:** 3 (content/relationship-idempotency, shefa/m1-matthew-terrance-delivery, qahal/collective-governance)
- **Files flagged for REWRITE:** 3 (qahal/collective-governance, auth/user-management, protocol/landing-page-dogfood — last is minor)
- **Files flagged for GRADUATE consideration:** 5 (conductor-admin-reachability, staging-validation, navigation-browser, content-lifecycle [partial], ssr_capability [partial])
- **Files flagged for MEMORIALIZE consideration:** 2 (persona-testnet-validation, shefa/m1-matthew-terrance-delivery [partial])
- **Files in HOLD:** 2 (lamad/know-thyself, delivery/landing-page)
- **New files Sprint 5 must author:** ~25 (7 faith-community, 7 life-group, 7 wisdom-commons, ~3 friction-gradient, ~2 imago-dei, ~5 household witness/care)

---

## Section 7 — Connection map

Each proposed directory links back to its gospel-tier source. This is how Sprint 1 UX brainstorming finds its scenario base, and how Sprint 5 authors against the canonical reference.

### archetypes/

| Directory | Canonical reference | What's there | What's missing |
|---|---|---|---|
| `household/` | `2026-05-21-qahal-section-4-canonical-narratives.md` Section 4.1; `2026-05-21-qahal-architecture-vision.md` Section 2.10 (imagodei lens recursion) | 7 scenarios; love-map, intimate quorum, compute mutual aid, resilience profile, real topology | Care-economy ambient witness; member-ring standing; commons-elohim quiet-witness pattern; reach-stays-steady-by-default — see Section 4.1 above |
| `faith-community/` | `2026-05-21-qahal-section-4-canonical-narratives.md` Section 4.2; `2026-05-21-qahal-architecture-vision.md` Section 2.5 (rubric as governable EPR), 2.7 (commons-elohim co-steward), 2.8 (friction-gradient at plural-elder scale) | 0 scenarios | All 7 proposed in Section 4.2 above |
| `life-group/` | `2026-05-21-qahal-section-4-canonical-narratives.md` Section 4.3; `2026-05-21-qahal-architecture-vision.md` Section 3.1 (Qahal-as-sub-Qahal coupling via `QahalToSubQahal`) | 0 scenarios | All 7 proposed in Section 4.3 above |
| `wisdom-commons/` | `2026-05-21-qahal-section-4-canonical-narratives.md` Section 4.4; `2026-05-21-qahal-architecture-vision.md` Section 3.1 (federation coupling pattern, "shared commons-elohim entity rather than parent-child links"); Section 7.5 (council convening as coordination substrate) | 3 scenarios (landing-page-dogfood, collective-governance, stewardship-allocation) | All 7 proposed in Section 4.4 above; plus rewrite of collective-governance per Section 3.1 |

### cross-cutting/

| Directory | Canonical reference | What's there | What's missing |
|---|---|---|---|
| `reach/` | `2026-05-21-qahal-architecture-vision.md` Section 2.2 (two axes), Section 2.10 (imagodei lens); `project_social_reach_nervous_system` memory | 4 scenarios | (Operator decision whether more needed beyond what archetype-scoped reach tests cover) |
| `standing/` | `2026-05-21-qahal-architecture-vision.md` Section 2.3 (standing function), 2.4 (Bloom curve); `project_standing_composes_multiple_evidence_streams` memory | 3 scenarios | Direct standing-decay scenarios (Section 2.9); imagodei-lens-rendered-differently-per-Qahal-context scenarios (Section 2.10) |
| `attestation/` | `2026-05-21-qahal-architecture-vision.md` Section 3.5 (five truths); `project_epr_substrate_vs_vf_graphql` memory | 3 scenarios | Lamad mastery-EPR attestation chain (Section 3.6 composition); commons-elohim witness-as-first-class-EPR |
| `friction-gradient/` | `2026-05-21-qahal-architecture-vision.md` Section 2.8; `project_friction_gradient_limitarianism` memory | 1 scenario (compute-commitment-bounds) | Qahal-scale flattening; rubric-update-requires-sibling-council-validation; recursive application to wisdom layer (Section 1.6) |
| `commons-elohim/` | `2026-05-21-qahal-architecture-vision.md` Section 2.7; `project_commons_elohim_shadow_agent` memory (note: spec renames "shadow" → "co-steward") | 2 scenarios | Per-archetype commons-elohim configuration scenarios; mediation between stewards; layered elohim arbitration councils |
| `rea-flow/` | `2026-05-21-qahal-architecture-vision.md` Section 3.6 (shefa composition); Section 7 (fractal-circular REA pattern); `project_principle_p1_reconciliation_controller` memory | 2 scenarios | Commitment/Fulfillment/Event triad; Agreement cascade clauses; restitution flows (Section 6.7); patronage flows (Section 6.11) |
| `imago-dei/` | `2026-05-21-qahal-architecture-vision.md` Section 1.5 (Foster's reconciliation frame); Section 6.18 (red-team test case) | 0 primary scenarios | Substrate-refuses-dignity-denial; witness-of-harm + attestation-of-repair + ongoing-acknowledgment as REA primitives; anti-tribalism friction-gradient at cross-collective edge |

### infrastructure/

| Directory | Canonical reference | What's there | What's missing |
|---|---|---|---|
| `recovery/` | `project_graduated_recovery_authority`, `project_socially_derived_security`, `project_recovery_grandma_standard` memories | 14 scenarios | Well-covered for MVP |
| `doorway/` | `project_doorway_full_facilitator_sprint`, `project_doorway_manifest_driven_routes`, `project_doorway_single_target_no_fanout` memories | 8 scenarios | Manifest-driven panel composition (Sprint 1 will surface gaps) |
| `p2p/` | `project_p2p_is_hosting`, libp2p-transport skill, libp2p-discovery skill | 4 scenarios | Iroh/libp2p version-bridge scenarios (Phase 11 prereq #2-#10 in memory) |
| `delivery/` | `project_storage_vocabulary_quilt`, `project_quilt_as_native_s3_surface` memories | 6 scenarios | Quilt vocabulary scenarios; pantry/draw flows |
| `ssr/` | `project_ssr_is_compute_capability_claim`, `project_ssr_anonymous_auth_context`, `project_doorway_ssr_pod_resource_floor` memories | 4 scenarios | Authenticated SSR with higher-reach content (currently anonymous-only context) |
| `browser/` | (smoke / hosted-human shape) | 3 scenarios | (Operator decision whether to keep or graduate per Section 3.2) |
| `deployment/` | `project_household_horizontal_scaling`, `project_alpha_topology_bootstrap_pair`, `project_elohim_node_role` memories | 10 scenarios | Operator-preset / cadence-archetype scenarios (memory: cadences are archetype-tunable) |
| `resilience/` | `project_placement_signals_are_shefa_inputs`, `project_inventory_exchange_not_byte_replication`, `project_seed_whoever_is_ready` memories | 3 scenarios | Inventory-vs-byte-replication regression; per-peer seeding |

---

## Closing notes

### What this archaeology surfaced

Three observations worth flagging to the operator beyond the per-file mappings:

1. **The MVP-critical archetypes are under-served.** Faith-community and life-group have zero primary scenarios. The household has 7 but is missing the Section 4.1 ambient-witness beats (Sheila's recipe, Gertrude's check-in, the right-nav one-sentence panel). Sprint 5 has ~21 new scenarios to author in the Tier 0 directories alone.

2. **The qahal pillar bucket is misleading.** Two files live in `qahal/` (collective-governance, feedback-dialogue-panel). Neither is archetype-specific; collective-governance needs a witness-not-vote rewrite per Section 7.5; feedback-dialogue-panel is canonical to cross-cutting/standing. The `qahal/` pillar bucket should disappear in the new taxonomy — the qahal pillar's substrate concerns live in archetypes/* and cross-cutting/* (which is exactly what gospel-tier prescribes: Qahal is the coordination surface that binds the other pillars).

3. **The dissolution principle changes how we read several existing scenarios.** `auth/user-management.feature` is the clearest case — hosted-user management is the substrate's bridge during graduation; it should not be framed as the substrate's enduring institution. The same dissolution lens applies (subtly) to several deployment/ files: human-device-mapping currently reads as "managing a fleet"; in the dissolution frame, it reads as "humans bring their devices into a household-scale resilience unit." Rewriting these is not urgent, but Sprint 5's storyteller pass should make the dissolution arc legible in scenario language wherever it touches existing institutional shapes.

### Recommended Sprint 1 and Sprint 5 entry points

- **Sprint 1 (Qahal homepage UX exploration)** should ground its graphos pattern stories in `archetypes/household/`, `archetypes/faith-community/`, and `archetypes/wisdom-commons/`. Section 4 of the canonical narratives is the storyboard; the proposed migrations of Sections 6.1, 6.2, 6.4 give the existing scenario base. The household has the most existing material; faith-community needs Sprint 5 to land first or the UX has nothing concrete to render.
- **Sprint 5 (genesis content + canonical templates + a2o scenarios)** should author the 21 Tier 0 scenarios proposed in Section 4 above, in the order: household-witness gap-fills (Section 4.1, 5 scenarios), faith-community (Section 4.2, 7 scenarios), life-group (Section 4.3, 7 scenarios), wisdom-commons gap-fills (Section 4.4, 7 scenarios). Plus ~5 cross-cutting scenarios (3 friction-gradient, 2 imago-dei).

### Operator decisions flagged

The following decisions need operator sign-off before Sprint 5 can author cleanly:

1. **Tier-subdir vs flat archetype directories** (Section 5) — recommendation: flat for MVP.
2. **lamad/know-thyself disposition** (Section 3.5) — hold, or assign to archetypes/household/, or to a new imagodei-self-knowledge bucket?
3. **delivery/landing-page disposition** (Section 3.5) — merge with archetypes/wisdom-commons/landing-page-as-content, or keep as INF:delivery smoke?
4. **content/relationship-idempotency split** (Section 6.1) — graduate the idempotency-protection scenarios, or keep the file whole?
5. **content/ssr_capability disposition** (Section 6.16) — graduate the capability-negotiation scenarios, or keep?
6. **qahal/collective-governance rewrite scope** (Section 3.1) — full vote→witness rewrite, or surface partial preservation?
7. **Anonymous voting in collective-governance** (Section 3.1) — anonymous witness defeats interpretability requirement; keep or graduate?
8. **persona-testnet-validation memorialize vs maintain** (Section 3.3) — confirm or revise?
9. **conductor-admin-reachability / staging-validation / navigation-browser graduate** (Section 3.2) — confirm graduate-to-unit-test, or keep as smoke?
10. **Tag-rename pass scope** (Section 5) — adopt `@archetype:*`, `@cc:*`, `@inf:*` tags now, or defer to Sprint 5+?

These are listed here so the operator can review and respond per-item; no decision is needed for the document to land — they shape downstream execution.

---

*Document end. Status: ready for operator review. Next: Matthew reviews; sign-off or specific revisions; once signed off, Sprint 1 UX exploration kicks off using the archetypes/* mapping above as its scenario base; Sprint 5 authors the proposed new scenarios in the order recommended.*
