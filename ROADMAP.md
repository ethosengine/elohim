# Elohim Protocol Sprint Roadmap

Last updated: 2026-02-12

## Vision Statement

The Elohim Protocol is a distributed learning platform where a person can register an account, navigate curated learning paths, complete assessments that reveal something about themselves, and track their mastery over time -- all through a gateway (doorway) that custodially manages their Holochain identity. The second person can join, and both can eventually run offline desktop nodes that sync peer-to-peer. The economics layer (shefa) enables cooperative resource stewardship, built on REA patterns, but only becomes relevant once the learning experience works end-to-end for real people.

"Done" means: two humans can independently log in, learn through paths with assessments, see their progress persist, and eventually sync through an offline-capable node. The economic and governance layers activate as the community forms.

---

## Current State (Validated)

This assessment is based on reading the actual source code, not documentation alone.

### Solid (80-100% functional)
- **Auth/Identity**: Password auth, JWT, Holochain agent provisioning, 4-stage agency model, session migration, doorway-registry, stewardship UI components (appeal wizard, policy console, intervention). Doorway auth module is 860 lines of production Rust.
- **Doorway Gateway**: Conductor proxy (1,386 lines), tiered caching (3,785 lines), projection engine (2,634 lines), bootstrap/signal. This is real infrastructure.
- **Content Pipeline**: 3,526 content files in genesis, 7 learning paths defined, seeder tooling with 30+ scripts. Content is there.
- **Lamad Navigation**: Path overview, path navigator, content viewer, learner dashboard, graph explorer, meaning map, search -- all routed and lazily loaded. The learning UI scaffold exists.
- **Content Rendering**: Markdown, Gherkin, iframe (HTML5 apps), quiz renderer all wired through registry pattern.
- **CI/CD**: Orchestrator pattern, multi-project quality gates, pre-push hooks.

### Partially Built (40-70%)
- **Mastery Tracking**: MasteryService talks to storage API, ContentMasteryService has dual-backend (localStorage + Holochain). Bloom's levels defined. But path-navigator has `isCompleted: false // TODO: Load from progress` -- the wiring is not connected.
- **Assessment Engine**: QuizSessionService (93.5% coverage), QuestionPoolService, DiscoveryAttestationService all exist. 5 discovery assessments defined in genesis (values hierarchy, attachment style, strengths finder, constitutional reasoning, personal values). But these are metadata records -- the actual question JSON (Sophia moments) needs to exist or be loaded.
- **Quiz Engine**: quiz-session, question-pool, streak-tracker, path-adaptation, attempt-cooldown services all built. The plumbing exists. PracticeService at 37.9% coverage -- less complete.
- **Offline Queue**: OfflineOperationQueueService exists with memory queue, but IndexedDB integration is at 20.3% coverage. Framework present, wiring incomplete.
- **elohim-node P2P**: This is further along than the assessment suggested. libp2p swarm with QUIC+TCP, mDNS, Kademlia, request-response protocol. Automerge-based CRDT sync engine with SQLite persistence. SyncCoordinator with peer tracking and periodic sync. The code compiles and runs. What it lacks: integration testing, actual deployment, and connecting it to real content sync.
- **doorway-app**: Admin UI with dashboard, login, register, doorway browser. Services for admin and federation. Functional scaffold.


### Modeled Only (0-30% functional)
- **REA Economics (shefa)**: Massive model files (stewarded-resources: 47KB, shefa-compute: 70KB, insurance-mutual: 56KB, requests-and-offers: 41KB). Services have elaborate method signatures but most are stub implementations -- `requests-and-offers.service.ts` has 15+ methods that `return await Promise.reject(new Error('Not yet implemented'))`. Insurance mutual has 30+ TODO comments for "In production, query Holochain DHT." The ElohimStubService explicitly exists as a development mock for AI agent calls.
- **Governance (qahal)**: Models defined (governance-deliberation, governance-feedback, collective-research, place). Community home shows "Coming Soon." Services re-export from elohim core (affinity-tracking, human-consent, governance). The governance service reads from DataLoaderService but there is no write path.
- **Token Circulation**: shefa-compute references token types, exchange rates, allocation -- all hardcoded placeholders (`value: totalTokens * 0.5, // TODO: Get actual exchange rate`).
- **Attribution Workflow**: No implementation found.
- **E2E Tests**: One Cypress feature file (learning journey, 29 lines). No step definitions implementing it.
- **Tauri Desktop**: No `src-tauri/` directory exists. TauriAuthService exists in imagodei but no desktop shell.

---

## Milestone Map

### M1: "Show Someone" (Weeks 1-4)
One person can register, log in, navigate a learning path, read content, and see their progress tracked. You can demo this to another human being.

### M2: "Two Learners" (Weeks 5-8)
Two separate accounts on the same doorway, each with independent progress. Assessment flow works end-to-end for at least one discovery instrument. Learner dashboard shows real data.

### M3: "Know Thyself" (Weeks 9-14)
All five discovery assessments completable. Results appear on profile as attestations. Path adaptation responds to assessment outcomes. A learner can take the "Know Thyself" path from start to finish.

### M4: "Take It With You" (Weeks 15-20)
Tauri desktop app packages the experience. elohim-node runs locally and syncs content. A learner can work offline and rejoin. Doorway-app provides operator visibility.

### M5: "Community" (Weeks 21-28)
Economic events recorded through REA patterns. Stewardship commitments tracked. Basic request/offer matching. Governance proposals readable (read path before write path).

### M6: "Cooperative" (Weeks 29-36)
Token generation from economic events. Insurance mutual claim filing. P2P sync between two family nodes. Governance deliberation write path.

---

## Sprint Breakdown

### Sprint 1 (Weeks 1-2): Progress Persistence
**Goal**: A logged-in learner's path progress survives page refresh.

**Stories**:

1. **As a learner, I want my completed steps to be remembered, so that I do not lose my place in a learning path.**
   - Acceptance: After completing step 3 of a path, refreshing the browser shows step 3 as completed
   - Wire path-navigator's `isCompleted: false` TODO to ContentMasteryService
   - Read mastery records from storage API on path load
   - Write mastery record (level: "aware") when content is viewed
   - Size: L (3 points)

2. **As a learner, I want to resume where I left off, so that I can continue learning without finding my place again.**
   - Acceptance: Returning to a path jumps to the last-viewed step
   - Store last-viewed step index per path in localStorage (visitor) or storage API (hosted)
   - Path overview shows "Continue" button pointing to last step
   - Size: M (2 points)

3. **As a learner, I want to see my overall path progress, so that I know how far I have come.**
   - Acceptance: Path overview shows "12 of 45 steps viewed" with a progress bar
   - Learner dashboard shows all active paths with completion percentage
   - Size: M (2 points)

**Dependencies**: None (builds on existing services)
**Sprint Total**: 7 points

---

### Sprint 2 (Weeks 3-4): Content Seeding Validation
**Goal**: Confirm the full content pipeline works end-to-end and fix what is broken.

**Stories**:

1. **As a developer, I want to run `npm run hc:start:seed` and get a working app with real content, so that I have a reliable demo environment.**
   - Acceptance: Fresh seed completes without errors. All 7 paths load. Content renders. Blobs resolve.
   - Run full seed pipeline. Fix any broken references.
   - Validate blob content resolution for sparse/blob pattern entries
   - Size: L (3 points)

2. **As a learner, I want content thumbnails and images to load reliably, so that the experience feels complete.**
   - Acceptance: No broken images on path overview or content viewer
   - Verify blob fallback chain (WASM hash -> server -> SubtleCrypto -> JS fallback)
   - Fix any broken blob references in genesis data
   - Size: M (2 points)

3. **As a developer, I want a smoke test that validates the seeded content, so that I know when the seed is broken.**
   - Acceptance: `npm run seed:verify` checks all paths resolve, all content loads, all blob references are valid
   - Implement using existing `verify-seed.ts` in genesis/seeder
   - Size: M (2 points)

**Dependencies**: Sprint 1 (progress tracking needed to see path completion)
**Sprint Total**: 7 points

**Milestone M1 complete** -- the app is demoable.

---

### Sprint 3 (Weeks 5-6): Multi-User Validation
**Goal**: Two separate accounts coexist on one doorway without data bleeding.

**Stories**:

1. **As a second learner, I want to register and see fresh (empty) progress, so that my learning is independent from the first learner.**
   - Acceptance: User A has progress on "Elohim Protocol" path. User B registers. User B sees no progress.
   - Verify session-scoped localStorage keys (AffinityTrackingService already uses session-specific keys)
   - Verify Holochain agent isolation through doorway conductor provisioning
   - Size: M (2 points)

2. **As a learner, I want to log out and back in and see my progress intact, so that my account is persistent.**
   - Acceptance: Log out, log back in, all progress and mastery records still present
   - Test the session migration path (localStorage -> backend on login)
   - Verify ContentMasteryService dual-backend handoff
   - Size: M (2 points)

3. **As a learner switching devices, I want my progress to follow me, so that I can learn from any browser.**
   - Acceptance: Log in from a second browser, see same progress as first
   - This works automatically if mastery is stored in Holochain via storage API
   - Verify storage-api mastery endpoint returns correct per-human data
   - Size: S (1 point)

**Dependencies**: Sprint 1 (progress must be stored, not just viewed)
**Sprint Total**: 5 points

---

### Sprint 4 (Weeks 7-8): Assessment Flow
**Goal**: A learner can complete one discovery assessment and see the result on their profile.

**Stories**:

1. **As a learner, I want to complete the "Personal Values Reflection" assessment within a learning path, so that I discover something about myself.**
   - Acceptance: Assessment content renders. Questions are answerable. Score is computed. Result is shown.
   - Create or validate Sophia moment JSON for the "Personal Values Reflection" instrument
   - Wire quiz-renderer to QuizSessionService for the discovery assessment type
   - Size: XL (5 points)

2. **As a learner, I want my assessment result to appear as an attestation on my profile, so that I carry what I have learned about myself.**
   - Acceptance: After completing an assessment, profile page shows the result (e.g., "Values: Family, Justice, Knowledge")
   - Wire DiscoveryAttestationService.createAttestation() on assessment completion
   - Display attestations on ProfilePageComponent
   - Size: L (3 points)

3. **As a learner, I want to see a summary of what the assessment revealed, so that I understand the result.**
   - Acceptance: Assessment completion shows an interpretation page before returning to the path
   - Use existing interpretation model in AssessmentService (dimensions, outcomes)
   - Size: M (2 points)

**Dependencies**: Sprint 2 (content must be seeded and renderable)
**Sprint Total**: 10 points

**Milestone M2 complete** -- two users learning and being assessed.

---

### Sprint 5 (Weeks 9-10): Remaining Discovery Instruments
**Goal**: All five discovery assessments are completable.

**Stories**:

1. **As a learner, I want to discover my attachment style, so that I understand how I relate to others.**
   - Create Sophia moment JSON for attachment assessment (25-35 min, multi-section)
   - Validate scoring logic against assessment content shape
   - Size: L (3 points)

2. **As a learner, I want to discover my character strengths, so that I understand what I naturally excel at.**
   - Create Sophia moment JSON for strengths finder (20-30 min)
   - Size: L (3 points)

3. **As a learner, I want to complete the values hierarchy assessment, so that I understand what I prioritize.**
   - Create Sophia moment JSON for values hierarchy (20-30 min, ranking interactions)
   - Size: L (3 points)

4. **As a learner, I want to assess my constitutional reasoning, so that I can participate in governance.**
   - Create Sophia moment JSON for constitutional reasoning (30-45 min, competency-based)
   - Size: L (3 points)

**Dependencies**: Sprint 4 (assessment pipeline proven with one instrument)
**Sprint Total**: 12 points

---

### Sprint 6 (Weeks 11-12): Path Adaptation and Mastery Levels
**Goal**: The learning experience responds to what the learner has demonstrated.

**Stories**:

1. **As a learner, I want path content to adapt based on my assessment results, so that my learning is personalized.**
   - Acceptance: After completing values assessment, the "Know Thyself" path shows relevant next steps
   - Wire PathAdaptationService to assessment results
   - Use discovery attestations to filter or reorder path steps
   - Size: L (3 points)

2. **As a learner, I want to see my Bloom's mastery level per concept, so that I know how deeply I understand each topic.**
   - Acceptance: Content viewer shows mastery badge (Aware, Understanding, Applying, etc.)
   - Path navigator shows color-coded mastery indicators per step
   - Wire existing MasteryService read methods to UI components
   - Size: M (2 points)

3. **As a learner, I want the quiz engine to advance my mastery level when I demonstrate understanding, so that my growth is tracked.**
   - Acceptance: Completing an inline quiz successfully advances mastery from "aware" to "understanding"
   - Wire quiz completion callback to ContentMasteryService.recordLevelChange()
   - Size: L (3 points)

**Dependencies**: Sprint 5 (assessment data available for adaptation)
**Sprint Total**: 8 points

---

### Sprint 7 (Weeks 13-14): Know Thyself Path and E2E Validation
**Goal**: A learner can complete the "Know Thyself" path start-to-finish and we can prove it works.

**Stories**:

1. **As a learner, I want to complete the Know Thyself path from beginning to end, so that I experience the full learning journey.**
   - Acceptance: Path starts, content loads at each step, assessments work, mastery advances, path completes
   - End-to-end walkthrough validation (manual and automated)
   - Fix any discovered issues in the path definition or content
   - Size: L (3 points)

2. **As a developer, I want Cypress E2E tests for the learning journey, so that regressions are caught automatically.**
   - Acceptance: `npm run cypress:run` passes. Covers: login, start path, view content, complete assessment, check progress.
   - Implement step definitions for existing `learning_journey.feature`
   - Add assessment completion scenario
   - Size: XL (5 points)

3. **As a developer, I want the learner dashboard to show real aggregated data, so that the dashboard is not empty.**
   - Acceptance: Dashboard shows: paths in progress, mastery distribution, recent activity, attestations earned
   - Wire existing LearnerDashboardComponent to mastery and attestation services
   - Size: M (2 points)

**Dependencies**: Sprint 6 (mastery progression must work)
**Sprint Total**: 10 points

**Milestone M3 complete** -- the Know Thyself journey works end-to-end.

---

### Sprint 8 (Weeks 15-16): Tauri Desktop Shell
**Goal**: The app runs as a desktop application with local storage.

**Stories**:

1. **As a learner, I want to install the app on my computer, so that I do not need a browser tab open.**
   - Acceptance: `cargo tauri build` produces an installable binary. App launches and shows the login screen.
   - Create `src-tauri/` scaffold (Cargo.toml, tauri.conf.json, main.rs)
   - Configure Tauri to load elohim-app build
   - Size: L (3 points)

2. **As a learner, I want the desktop app to connect to my doorway, so that I can log in and access my data.**
   - Acceptance: Login works. Content loads. Progress is tracked.
   - Wire TauriAuthService (already exists) to Tauri IPC
   - Configure HTTP client to connect to doorway (not localhost sidecar initially)
   - Size: L (3 points)

3. **As a learner, I want the desktop app to cache content locally, so that pages load instantly.**
   - Acceptance: After first load, content renders without network delay
   - Wire IndexedDBCacheService for Tauri context
   - Implement content pre-fetch on path start
   - Size: M (2 points)

**Dependencies**: Sprint 7 (app must work end-to-end first)
**Sprint Total**: 8 points

---

### Sprint 9 (Weeks 17-18): Local elohim-node Integration
**Goal**: An elohim-node runs alongside the Tauri app and provides local data.

**Stories**:

1. **As a node steward, I want to run elohim-node on my hardware, so that I have local sovereignty over my data.**
   - Acceptance: `elohim-node` starts, creates SQLite database, listens on configured port, dashboard accessible
   - Integration test the existing main.rs startup path
   - Package with Tauri as a sidecar process
   - Size: L (3 points)

2. **As a learner, I want the Tauri app to talk to my local node, so that my data stays on my hardware.**
   - Acceptance: Tauri app configured to point to localhost:8090 (elohim-storage sidecar mode)
   - Content reads from local node. Writes go to local node.
   - Size: L (3 points)

3. **As a node steward, I want the pod management system to handle basic operations, so that the node is self-maintaining.**
   - Acceptance: Pod starts in background, runs periodic health checks, dashboard shows status
   - Validate existing Pod implementation with real config
   - Size: M (2 points)

**Dependencies**: Sprint 8 (Tauri shell must exist)
**Sprint Total**: 8 points

---

### Sprint 10 (Weeks 19-20): P2P Sync and Offline
**Goal**: Two nodes discover each other and sync content.

**Stories**:

1. **As a learner, I want my desktop node to sync with my doorway, so that my progress follows me between devices.**
   - Acceptance: Progress created via browser (doorway) appears on desktop node after sync
   - Connect SyncCoordinator to actual content data (mastery records, attestations)
   - Test sync between elohim-node and doorway using Automerge documents
   - Size: XL (5 points)

2. **As a learner, I want to work offline and have changes sync when I reconnect, so that I do not need constant internet.**
   - Acceptance: Create progress offline. Reconnect. Progress appears on doorway.
   - Wire OfflineOperationQueueService to Automerge documents
   - Implement conflict-free merge for mastery records (last-write-wins by timestamp is acceptable for mastery)
   - Size: XL (5 points)

3. **As a family, I want two nodes on the same network to discover each other, so that we can share content.**
   - Acceptance: Two elohim-nodes on same LAN discover via mDNS and complete initial sync
   - Integration test the existing mDNS + request-response protocol chain
   - Size: L (3 points)

**Dependencies**: Sprint 9 (local node must be running)
**Sprint Total**: 13 points

**Milestone M4 complete** -- offline-capable desktop learning.

---

### Sprint 11 (Weeks 21-24): REA Economic Foundation
**Goal**: Economic events are recorded when real things happen.

**Stories**:

1. **As a node steward, I want compute contribution to be automatically recorded, so that my stewardship has value.**
   - Acceptance: Running a node generates periodic economic events (compute resource contribution)
   - Wire shefa-compute service to real node metrics (uptime, storage, bandwidth)
   - Replace hardcoded values with actual measurements
   - Size: L (3 points)

2. **As a learner, I want my learning engagement to generate recognition, so that learning has economic meaning.**
   - Acceptance: Completing a path step creates a "learning" economic event linked to my agent
   - Wire path completion to economic event factory
   - Size: M (2 points)

3. **As a content steward, I want my content contributions to be tracked, so that my creative work is recognized.**
   - Acceptance: Content edits generate "contribute" economic events with attribution
   - Wire content-editor submission to economic event creation
   - Size: M (2 points)

4. **As a steward, I want to see an economic activity feed, so that I understand the value flowing through the system.**
   - Acceptance: Shefa dashboard shows chronological economic events with agents, resources, and quantities
   - Wire ShefaDashboardComponent to economic event service
   - Size: L (3 points)

**Dependencies**: Sprint 7 (learning flow must work to generate events)
**Sprint Total**: 10 points

---

### Sprint 12 (Weeks 25-28): Requests, Offers, and Stewardship
**Goal**: Basic cooperative economics are functional.

**Stories**:

1. **As a community member, I want to make a request for help, so that the community can respond.**
   - Acceptance: Create request form works. Request appears in community feed. Another member can see it.
   - Implement core create/read methods in requests-and-offers service (replace Promise.reject stubs)
   - Create request form component
   - Size: L (3 points)

2. **As a community member, I want to offer a resource or skill, so that others know what I can contribute.**
   - Acceptance: Create offer form works. Offer appears in community feed. Matching suggestions shown.
   - Implement offer creation and basic matching (replace PLACEHOLDER IMPLEMENTATION)
   - Size: L (3 points)

3. **As a steward, I want to see my stewardship commitments, so that I know what I have promised.**
   - Acceptance: Stewardship dashboard shows active commitments, their status, and fulfillment progress
   - Wire existing stewardship.service.ts to real data
   - Size: M (2 points)

4. **As a node operator, I want the doorway-app to show connected users and resource usage, so that I can manage my doorway.**
   - Acceptance: Doorway dashboard shows active sessions, storage used, bandwidth consumed
   - Wire doorway-admin.service.ts to doorway API endpoints
   - Size: L (3 points)

**Dependencies**: Sprint 11 (economic events must be flowing)
**Sprint Total**: 11 points

**Milestone M5 complete** -- community economics functional.

---

### Sprint 13 (Weeks 29-32): Governance Read Path and Insurance
**Goal**: Governance is visible and insurance claims can be filed.

**Stories**:

1. **As a community member, I want to read governance proposals and their status, so that I understand community decisions.**
   - Acceptance: Governance page shows proposals, challenges, and precedents from seed data
   - Wire qahal community routes to governance service read methods
   - Replace "Coming Soon" with actual governance feed
   - Size: L (3 points)

2. **As a community member, I want to file an insurance claim, so that the mutual can help me.**
   - Acceptance: Claim form works. Claim appears in system. Status trackable.
   - Implement insurance-mutual create/read methods (replace TODO stubs)
   - Create claim filing UI component
   - Size: XL (5 points)

3. **As a community member, I want to vote on a governance proposal, so that I participate in community decisions.**
   - Acceptance: Vote button on proposals. Vote recorded. Tally updated.
   - Wire existing proposal-vote component to governance service write path
   - Size: L (3 points)

**Dependencies**: Sprint 12 (community context must exist)
**Sprint Total**: 11 points

---

### Sprint 14 (Weeks 33-36): Token Generation and Inter-Node Sync
**Goal**: Economic value circulates and nodes form a real network.

**Stories**:

1. **As a steward, I want economic events to generate tokens, so that contribution is rewarded.**
   - Acceptance: Contribution events produce tokens in the commons pool. Token balance visible.
   - Implement token generation rules (from compute events, learning events, content events)
   - Replace hardcoded exchange rates with configurable rates
   - Size: XL (5 points)

2. **As a family node, I want to sync with another family's node, so that we form a cooperative network.**
   - Acceptance: Two nodes on different networks connect via bootstrap peers and sync shared content
   - Implement Kademlia bootstrap peer discovery (beyond mDNS local network)
   - Test cross-network sync of economic events
   - Size: XL (5 points)

3. **As a community member, I want insurance claim adjudication to work, so that claims are processed fairly.**
   - Acceptance: Filed claims enter adjudication queue. Community members can review. Outcome recorded.
   - Replace ElohimStubService AI adjudication with rule-based adjudication
   - Size: L (3 points)

**Dependencies**: Sprint 13 (governance and insurance must be readable)
**Sprint Total**: 13 points

**Milestone M6 complete** -- cooperative economics with P2P sync.

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Content seed is stale or broken** | High | Blocks M1 | Sprint 2 explicitly validates the seed pipeline. Last known seed was Dec 2024. |
| **Sophia moment JSON does not exist for assessments** | High | Blocks M2 | Assessment index references 5 instruments but the rendering content (questions, answer formats) may need authoring. Sprint 4 is sized XL to account for this. |
| **Solo developer velocity** | High | Affects all milestones | Sprints are sized for 1 developer at ~7-10 points per sprint. Timelines assume 60-70% capacity (life happens). |
| **Holochain API changes** | Medium | Could break doorway/storage | Pin Holochain versions. Monitor release notes. The conductor proxy pattern insulates the frontend. |
| **libp2p integration gaps** | Medium | Blocks M4 | elohim-node P2P code exists but has not been integration-tested. Sprint 10 includes explicit integration testing time. |
| **Tauri packaging complexity** | Medium | Delays M4 | Start the Tauri scaffold early (Sprint 8). The app is already a standard Angular SPA, which Tauri handles well. |
| **Scope creep on shefa/qahal** | High | Delays M5-M6 | The REA models are enormous (300KB+ of TypeScript models). Implement only the minimum write path for each feature. Read-only first. |
| **Test coverage gaps** | Medium | Regression risk | Several key services are at low coverage (PracticeService 37.9%, IndexedDBCacheService 20.3%, DiscoveryAttestationService 17.4%). Sprint 7 includes E2E test investment. |
| **RUSTFLAGS build environment** | Low | CI failures | Documented in CLAUDE.md. Pre-push hooks catch this. |

---

## Definition of Done

### M1: "Show Someone"
- [ ] A new user can register an account through the browser
- [ ] The user can navigate to a learning path and view content at each step
- [ ] Step completion is tracked and persists across page refresh
- [ ] Path overview shows progress percentage
- [ ] Learner dashboard shows active paths
- [ ] Content seeding completes without errors
- [ ] All images and blob content load correctly
- [ ] Seed verification script passes

### M2: "Two Learners"
- [ ] Two accounts on one doorway have independent, isolated progress
- [ ] Logging out and back in preserves all progress
- [ ] At least one discovery assessment is completable end-to-end
- [ ] Assessment result appears as an attestation on the user's profile
- [ ] Cross-device login shows consistent progress

### M3: "Know Thyself"
- [ ] All 5 discovery assessments are completable
- [ ] Assessment results drive path adaptation
- [ ] Bloom's mastery levels advance through quiz completion
- [ ] The "Know Thyself" path is completable start-to-finish
- [ ] Cypress E2E test suite covers the core learning journey
- [ ] Learner dashboard shows real mastery, activity, and attestation data

### M4: "Take It With You"
- [ ] Tauri desktop app installs and launches on macOS/Linux
- [ ] Desktop app connects to doorway and functions identically to browser
- [ ] elohim-node runs as a local sidecar
- [ ] Content is cached locally for offline access
- [ ] Offline progress syncs when connectivity is restored
- [ ] Two nodes on the same LAN discover each other via mDNS

### M5: "Community"
- [ ] Learning engagement, compute contribution, and content creation generate economic events
- [ ] Shefa dashboard shows real economic activity
- [ ] Requests and offers can be created and viewed
- [ ] Stewardship commitments are tracked
- [ ] Doorway-app shows operator metrics

### M6: "Cooperative"
- [ ] Token generation from economic events
- [ ] Insurance claims can be filed and adjudicated
- [ ] Governance proposals can be read and voted on
- [ ] Two family nodes on different networks can sync via bootstrap peers
- [ ] Token balances are visible and accurate

---

## Velocity Assumptions

- **Solo developer**: ~7-10 story points per 2-week sprint
- **Calendar duration**: 36 weeks (9 months) for all 6 milestones
- **M1 target**: 4 weeks (February-March 2026)
- **M2 target**: 8 weeks (April 2026)
- **M3 target**: 14 weeks (May 2026)
- **M4 target**: 20 weeks (July 2026)
- **M5-M6**: Detailed planning deferred until M3 is complete. Estimates here are rough.
- **Buffer**: 20% schedule buffer built into sprint point estimates (7 points achievable, 10 is a stretch goal)

---

## What This Roadmap Does Not Cover

These are real capabilities mentioned in project documents but intentionally deferred:

- **Service Worker / PWA offline**: Tauri desktop path chosen instead. Browser PWA is a future consideration.
- **Full governance deliberation write path**: Read-only governance in M5. Write path in M6.
- **Demurrage / token decay**: Requires token circulation to be established first. Post-M6.
- **Content creation workflow**: Editor exists but authoring pipeline (genesis -> review -> publish) is manual. Post-M3.
- **WebRTC signaling for real-time presence**: Doorway signal module exists. Real-time presence is post-M4.
- **AI agent integration**: ElohimStubService placeholder pattern is correct. Real AI integration is post-M6.
- **Multi-doorway federation**: doorway-federation.service.ts exists. Federation is post-M6.
