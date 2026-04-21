# R&O Lessons → Elohim Roadmap — Handoff for Breakout Session

**Date:** 2026-04-21
**Authors:** Matthew Dowell + Opus 4.7
**Purpose:** Hand off a decomposed roadmap of R&O-inspired adoptions to a parallel breakout session so the main session can stay focused on the elohim-core graph substrate spec.
**Status:** Handoff (not a spec; not a plan; a decomposition + context dump)
**Parallel work:** `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md` — the graph substrate core is being designed separately and is NOT part of this handoff. It is the *foundation* that several of these sub-projects depend on, but it is not itself one of them.

---

## For the session picking this up cold

You have 9 loosely-coupled sub-projects below. This is a **decomposition problem, not a single-feature brainstorm.** Do not try to scope all of them into one spec.

**Your job:**
1. Read §1–§3 for context (~5 min)
2. Read the 9 sub-project briefs in §5 and pick one with the user
3. Take that sub-project through the standard brainstorming → spec → plan flow (`superpowers:brainstorming` → `superpowers:writing-plans`)
4. The other 8 are future work — leave them alone

**Expected output:** one design spec at `genesis/docs/superpowers/specs/YYYY-MM-DD-<sub-project>-design.md` and one implementation plan at `genesis/docs/superpowers/plans/YYYY-MM-DD-<sub-project>-plan.md`.

**Do not:**
- Re-analyze R&O from scratch (that work is captured in §2–§4 below)
- Try to do the graph substrate work (that's the parent session's scope)
- Pick more than one sub-project per session

---

## 1. Context: how this roadmap came to be

Matthew asked for an analysis of the drift between his fork (`origin/che-ethosengine`) of `happenings-community/requests-and-offers` and upstream. The fork is ~7 months behind — 186 upstream commits across 6 minor releases (v0.2.0 → v0.5.1). The analysis revealed R&O's upstream has shipped substantial architectural and process improvements that elohim has not adopted, some of which are worth borrowing.

From that analysis, Matthew asked specifically about:
- Disciplined releases, feature flags, Sweettest framework, hREA DNA alignment, multi-platform Tauri support, Apollo Client
- Whether elohim's Holochain usage is up-to-date, using latest features and batching
- Possibility of using the Holochain Launcher / joining Moss
- "Anything else worth adopting"

He also framed the strategic intent: *"my goal with R&O is to offer the performance of the libp2p backend for the app to scale, eventually giving R&O legacy in the elohim protocol... I want Sasha and the holochain team to really consider elohim protocol as a natural landing zone for the future they are wanting to build."*

The conversation then pivoted (correctly, per the brainstorming skill's decomposition rule) from "let's plan all of this" to "let's pick the foundational architecture first." That pivot spawned the parallel graph substrate spec. **This document preserves the rest of the roadmap for separate work.**

---

## 2. R&O drift analysis (what upstream shipped)

Brief version. Full analysis is in the session transcript; rough source: upstream `CHANGELOG.md` at `happenings-community/requests-and-offers@main`.

### Major architectural shifts

| Shift | Release | Verdict |
|---|---|---|
| **Progenitor pattern** in admin/integrity zomes — designated network creator, coordinator-enforced auth, bootstrap first-admin restricted to dev mode | v0.5.0 | Applicable to imagodei + mishpat |
| **Tryorama → Sweettest** migration — Rust-native in-process integration tests | v0.5.0 | Adopt (sub-project #3) |
| **Holochain 0.5.x → 0.6.0** — HDK 0.6.0, HDI 0.7.0, manifest schema change, Nix flake, new getrandom backend | v0.2.2 | Elohim already on 0.6.0 — parity confirmed |
| **hREA v0.600 alignment** — GraphQL v0.600 API contract, Phase 1 proposal mapping with CFN pattern, pending queue, archived status | v0.4.0 | Strategic — see sub-project #4 |
| **Weave/Moss integration** — R&O as a Weave Tool for Moss, hybrid Moss/standalone profile display | v0.4.0 | Dual-path — see sub-project #8 |
| **Effect-TS 7-layer architecture** codified as a Claude skill with templates + validation | v0.4.0+ | Pattern worth stealing |
| **Action hash type safety** — compile-time distinct hash types to prevent kind confusion | v0.5.0 | Apply to elohim DNAs |

### Notable features

- **Active/Archived listings** (v0.3.0 — BREAKING) — `ListingStatus` enum, new LinkTypes `ActiveRequests`/`ArchivedRequests`/`ActiveOffers`/`ArchivedOffers`, tab switchers, archived = read-only
- **Markdown support** for descriptions + bios
- **Organization contact person** designation
- **Search** across requests/offers/organizations/users (filter composables + FilterControls components)
- **Atomic dev features** (v0.2.3) — replaced env-mode switches with individual flags `VITE_MOCK_BUTTONS_ENABLED` / `VITE_PEERS_DISPLAY_ENABLED`
- **Post-MVP rewrite** — conversation-first exchange model, hREA-first reputation
- **ProfileGuard + ProfileStatusIndicator + ProgenitorBadge** UI
- **NetworkStatusPopup** extracted from NavBar
- **DHT propagation fix** — `GetStrategy::Local → Network` (chose correctness over latency, documented why)

### Infrastructure / DX

- `deployment/` submodules for kangaroo-electron + Homebrew tap
- Agent OS framework removed, replaced by `.claude/skills/*` (effect-ts-7layer, hrea-integration)
- Documentation restructure: `documentation/mvp/` → `documentation/requirements/mvp/`; new `MOSS_INTEGRATION.md`, `technical-specs/action-hash-type-safety.md`, `technical-specs/organization-contacts.md`
- Default UI port 8888 → 8880
- Dev mode: `start:progenitor` launcher, kitsune2-bootstrap-srv race-condition fix, 127.0.0.1 binding, admin-WS retry logic

### Persistence architecture (critical finding)

**R&O is pure Holochain DHT — no split persistence layer.** They stayed fully on the DHT and addressed performance with bounded content + UI-side memoization + federation:

- Request/Offer entries on DHT, description capped at **1000 chars**
- User entries include `picture: Option<SerializedBytes>` — avatars embedded as raw bytes in DHT entry
- No sqlite/diesel/sled/rocksdb anywhere in workspace
- Only "cache" is `ui/src/lib/utils/cache.svelte.ts` — Effect-TS in-memory cache, capacity 1000, 5-min TTL, not persistent
- Apollo Client's in-memory cache for hREA GraphQL — not persistent either

**This validates elohim's split architecture.** R&O's 1000-char cap is a tacit admission that you can't put real content on the DHT. Their federation-via-Moss-groups scales by multiplication of small DHTs, not by addressing the DHT-as-content-store problem. For elohim's content-heavy domain, the split (libp2p + sqlite/diesel for content, Holochain for notarization) is the right call.

### DNA upgrade strategy (critical finding)

**R&O has no DNA upgrade mechanism.** Every breaking integrity zome change is a network reset — they've had three in six months (v0.2.2, v0.3.0, v0.5.0). Their "migration guide" is manual export + reinstall, and `#[serde(default)]` forward-compat only works within the same DNA hash. `lineage:` field is not used anywhere. `clone_limit: 0` rules out clone-cell migration.

This is structurally unfixable in pure-Holochain, and it's another argument for elohim's split architecture. Your content survives DNA resets because it lives in elohim-storage; Holochain is free to be the "fingerprint of integrity" layer it's best at.

---

## 3. Strategic framing

The elohim positioning play vs. Holochain's current story:

**Holochain's current story:** "hApps are isolated coordination tools running inside Moss groups." This caps at medium-sized cooperatives. It does not explain how the protocol carries an ecosystem — cross-group economic coordination, shared content libraries, cross-network identity, reputation portability.

**What elohim offers that HC doesn't have:**
1. A durable-content layer (libp2p + storage) that respects the DHT's strengths and sidesteps its weakness
2. An economic layer (shefa) that can speak hREA/ValueFlows as its wire vocabulary, making R&O + any REA hApp a first-class peer
3. Identity/provenance notarization at the DHT layer (imagodei) that gives any hApp free portable identity
4. A content-addressed link model that is genuinely post-URL
5. (In progress — separate spec) A graph substrate that lets hApps compose across groups

**The pitch:** elohim isn't a competitor. It's the missing protocol substrate that lets `hApp` mean "specialized governance/coordination DNA" rather than "walled garden." R&O becomes the reference implementation of "coordination hApp that graduates its content to elohim."

Several sub-projects below are chosen because they are the concrete steps toward that positioning.

---

## 4. Current elohim state (verified 2026-04-21)

Established during the parent session:

- **Holochain stack:** HDK 0.6.0, HDI 0.7.0 — same as R&O v0.5.x upstream ✅
- **5 DNAs:** `infrastructure`, `mishpat`, `imagodei`, `elohim` (lamad), `node-registry`
- **None use `lineage:`** — zero hits across all DNA manifests
- **`lamad/dna.yaml` has `network_seed: ~`** — not even a stable seed yet
- **Tauri:** `steward/device` via darksoil's `tauri-plugin-holochain` (main-0.6 branch). `tauri.conf.json` bundle targets `["deb"]` — Linux-only.
- **No root CHANGELOG.md, no release script, no feature-flag system**
- **No Apollo / GraphQL / hREA / valueflows / `@theweave/api`** in package.json
- **No sweettest or tryorama** — backend test coverage is thin
- **Has `content_store` zome** with chunked encoding + RS-4-7 / RS-8-12 erasure codes + `batch_get_content_by_ids` — the split architecture is doing work R&O can't
- **True pillars:** imagodei, lamad, shefa, mishpat, qahal
- **Process demonstrator subgraphs:** avodah (protocol-as-process), doorway (web2 projection)

---

## 5. The 9 sub-projects

Ordered for readability, not priority. Matthew's strong preference from the parent session: **7 (DNA manifest hygiene)** and **9 (R&O graduation path doc)** are the leveraged/leading candidates; **4 (hREA alignment)** is the biggest strategic prize but is multi-week and should not be the first brainstorm.

Each sub-project below has: brief, current state, upstream R&O reference, effort (S/M/L/XL), strategic weight (L/M/H), dependencies, and key open questions to explore in brainstorming.

---

### 1. Release discipline (CHANGELOG, semver, `/release`)

**Brief:** Establish semantic versioning and release checklist for elohim's shippable artifacts (elohim-app, elohim-storage, steward). Add a `/release` slash command driven by a `RELEASE_CHECKLIST.md`. Migrate to `main = release-only, dev = default` branch policy (R&O v0.4.0+ pattern).

**Current state:** No root `CHANGELOG.md`. No semver. No release script. Ships from `dev` branch-as-default.

**Upstream R&O reference:**
- `.claude/commands/release.md`
- `documentation/RELEASE_CHECKLIST.md`
- Release ladder v0.1.x → v0.5.1 in ~6 months
- Keep-a-Changelog format with emoji section headers (Features / Bug Fixes / Refactor / Docs / Infra / Breaking)

**Effort:** M
**Strategic weight:** M
**Depends on:** nothing
**Blocks:** eventual external-facing positioning (credibility signal for HC team)

**Brainstorm questions:**
- Single repo-level CHANGELOG or per-component (elohim-app / storage / steward / DNAs)?
- Semver scope — do DNA hash changes force a MAJOR bump, or do we version envelope + DNA separately?
- Pre-release identifiers (alpha / beta / rc) — when do they apply?
- How does the `/release` command interact with Jenkins pipeline triggers?
- Branch policy migration — how to migrate in-flight branches without blocking current work?

---

### 2. Feature flag system (atomic, sense-and-respond aware)

**Brief:** Replace ad-hoc flag checks with an atomic feature-flag system that respects elohim's sense-and-respond architecture. Flags should derive from observed state where possible, not just build-time env vars.

**Current state:** No systematic feature flags. One file has ad-hoc flag checks (`app/elohim-app/src/app/lamad/quiz-engine/services/quiz-sound.service.ts`). No central registry.

**Upstream R&O reference:**
- v0.2.3 — replaced env-mode flags (`VITE_DEV_FEATURES_ENABLED`) with atomic flags (`VITE_MOCK_BUTTONS_ENABLED`, `VITE_PEERS_DISPLAY_ENABLED`)
- Each flag = one feature, no mode dependencies

**Effort:** S-M
**Strategic weight:** M
**Depends on:** nothing
**Blocks:** clean mock-data toggles, dev/prod behavior toggles, experimental feature rollouts

**Brainstorm questions:**
- Which flags derive from **observed state** (like `Phase::ElohimActive` per the memory rule) vs. **declared state** (build-time env var)?
- Where does the `DevFeaturesService` live — per-shell (web / tauri) or shared?
- How do flags interact with elohim-agent's gate declarations?
- Do we need a runtime toggle UI for stewards, or is build-time sufficient for v1?
- Memory rule to respect: `project_elohim_active_observed_not_flagged.md`

---

### 3. Sweettest adoption + first test suite per DNA

**Brief:** Adopt Rust-native `holochain::sweettest` for DNA integration tests. Establish a `tests/sweettest/` workspace pattern per the R&O v0.5.0 layout. Write a baseline suite per DNA (5 DNAs in elohim).

**Current state:** No sweettest, no tryorama. `elohim/holochain/dna/infrastructure/tests/peer_status.rs` has commented-out `#[tokio::test]` annotations — suggests an abandoned attempt.

**Upstream R&O reference:**
- v0.5.0 — full migration from Tryorama
- `tests/sweettest/` workspace member, excluded from default workspace build
- Invoked via `CARGO_TARGET_DIR=target/native-tests cargo test -p requests_and_offers_sweettest`
- `common/conductors.rs`, `common/fixtures.rs`, `common/mirrors.rs` scaffolding (mirrors = sync wait helpers)

**Effort:** M
**Strategic weight:** H
**Depends on:** nothing
**Blocks:** regression safety for DNA changes, progenitor-pattern adoption, any confidence in cross-agent behavior

**Brainstorm questions:**
- Is there one `tests/sweettest/` at repo root covering all DNAs, or per-DNA?
- How do sweettest runs fit into Jenkins pipeline (separate stage, parallel, gating)?
- What are the baseline scenarios per DNA (e.g., admin bootstrap, content publish + link, agent discovery)?
- Native-only deps (holochain, tokio) — how do we exclude from WASM builds cleanly?
- Memory rule: `feedback_shift_measure_jenkins.md` — measures live in Jenkins, not locally, because Che has no local HC.

---

### 4. hREA alignment — shefa speaks VF-GraphQL

**Brief:** Make shefa (economic pillar) speak ValueFlows / VF-GraphQL as its wire vocabulary. Two paths:
- **(a) Embed hREA.dna** as a second DNA in steward/device (R&O's pattern) — full fidelity, but inherits hREA's schema burden and DHT performance profile.
- **(b) Shefa exposes VF-GraphQL-shaped views** over elohim's own economic events in elohim-storage — cheaper, preserves your split architecture, R&O interop via vocabulary alignment rather than DNA co-location.

Matthew's lean from the parent session: **path (b)**. Reasons: preserves sovereignty over your economic data, avoids inheriting hREA's DHT performance constraints, and still gets you cross-app interop because the *vocabulary* is what matters for federation.

**Current state:** No hREA, no Apollo, no `@valueflows/vf-graphql`, no GraphQL. Shefa pillar exists at `app/elohim-app/src/app/shefa/` and `elohim/sdk/domains/shefa/` with REA-shaped types but is not wired to hREA/VF vocabulary.

**Upstream R&O reference:**
- v0.4.0 — Phase 1 hREA proposal mapping with CFN pattern, pending queue, archived status
- `@valueflows/vf-graphql-holochain 0.600.0-dev.0`, `@apollo/client 3.13.8`, `svelte-apollo`
- Added `ui/src/lib/services/mappers/offer-proposal.mapper.ts`, `request-proposal.mapper.ts`

**Effort:** L (path b) / XL (path a)
**Strategic weight:** H (biggest single strategic prize)
**Depends on:** elohim-core graph substrate spec (§parent session). hREA alignment is a natural *instance* of the manifest graph — VF-GraphQL is a published Manifest EPR, shefa's manifest extends it.
**Blocks:** R&O graduation path (sub-project #9), cross-ecosystem economic federation

**Brainstorm questions:**
- Path (a), path (b), or hybrid? (Strong recommendation: b. Confirm with user.)
- Which 3-5 VF types do we start with (Agent, EconomicEvent, Commitment, ResourceSpecification, Agreement)?
- How does the VF-GraphQL manifest get published — elohim project alone, or coordinate with Lynn Foster / Bob Haugen / VF team first?
- If path (b), how do we resolve the impedance between VF's Agent-centric model and elohim's Human-vs-Agent distinction?
- REA CFN (Commitment/Fulfillment/Notification) pattern — how does it map to elohim's Signal Harness?
- How does this interact with the existing `elohim/sdk/domains/shefa/manifest.json`?

**This is the sub-project that delivers the "R&O legacy in elohim" story.** But it's the biggest. Don't brainstorm it first.

---

### 5. Tauri multi-platform bundling

**Brief:** Expand `steward/device` from Linux-only (`deb`) to the full desktop matrix (AppImage, DMG, MSI, NSIS, app). Add a Jenkins matrix build stage. Optionally add mobile targets.

**Current state:** `steward/device/src-tauri/tauri.conf.json` bundle targets `["deb"]`. No macOS, Windows, AppImage, or mobile builds. Uses darksoil's `tauri-plugin-holochain` (main-0.6 branch).

**Upstream R&O reference:**
- kangaroo-electron submodule handles desktop packaging
- Full bundle matrix
- Homebrew tap auto-bumps on release

**Effort:** S (flag-flip) to M (CI matrix)
**Strategic weight:** M
**Depends on:** nothing (could run in parallel with release discipline #1)
**Blocks:** reaching non-Linux users, showing up in HC-community distribution channels

**Brainstorm questions:**
- Do we adopt kangaroo-electron (R&O's approach) or vanilla Tauri bundle targets?
- Jenkins matrix vs. GitHub Actions for cross-compile — what's the CI story?
- Do we include mobile (iOS / Android) in scope or defer?
- Code-signing certificates — who holds them, how are they rotated?
- Auto-update strategy — Tauri's built-in updater vs. something else?
- How does this interact with the steward/device identity handoff flow?

---

### 6. Holochain Launcher listing

**Brief:** Package elohim-app as a `.webhapp` and list it in the Holochain Launcher portal. This is a quick web-community distribution win.

**Current state:** No webhapp bundle. No Launcher listing. Steward/device packages Holochain directly via `tauri-plugin-holochain` instead of using the Launcher shell.

**Upstream R&O reference:**
- R&O produces a `.webhapp` via `hc web-app pack`
- Ships through kangaroo-electron + Homebrew; also listable on Launcher

**Effort:** S
**Strategic weight:** M
**Depends on:** working happ.yaml (may need #7 first)
**Blocks:** nothing, but enables HC-community visibility

**Brainstorm questions:**
- Is the Launcher listing for elohim-app the "web2 trial surface" while steward/device is the full protocol?
- What happens when a Launcher user wants to graduate to full steward — what's the identity handoff story?
- Do we need a separate `web-happ.yaml` or does the steward's manifest serve both?
- What does the Launcher user experience vs. the steward experience look like (feature differences, capability differences)?
- Does this come before or after #5 (multi-platform tauri)?

---

### 7. DNA manifest hygiene

**Brief:** Bring all 5 elohim DNA manifests up to Holochain 0.6 best practices: stable versioned `network_seed`, `lineage:` field for future upgradability, `progenitor_pubkey` pattern for admin bootstrapping, `modifiers` block in `happ.yaml`, `clone_limit` consideration.

**Current state (verified 2026-04-21):**
- `lamad/dna.yaml` has `network_seed: ~` (null — every dev build is a distinct network)
- **Zero `lineage:` usage** across any manifest
- No `progenitor_pubkey` — imagodei + mishpat have admin/governance concerns that would map perfectly
- No `modifiers` block in `happ.yaml`

**Upstream R&O reference:**
- v0.5.0 progenitor pattern (`a9b20ff5`, `7783bfd4`) — 173 lines added to admin integrity zome
- `workdir/happ.yaml` has `modifiers` block with `network_seed` + `progenitor_pubkey` property
- Alpha network seed naming convention (`requests_and_offers_alpha`)

**Effort:** S (flip flags + add properties + document)
**Strategic weight:** H (de-risks everything downstream; visible to HC team on first code read)
**Depends on:** nothing
**Blocks:** sweettest progenitor tests (#3), any deployment discipline (#1), any cross-peer testing

**Brainstorm questions:**
- Stable network seed naming — `{dna}_alpha` like R&O, or versioned (`lamad_v1`)?
- Progenitor pattern scope — does it apply to all 5 DNAs or just imagodei + mishpat?
- Lineage policy — when do we bump DNA hash vs. keep same hash with `#[serde(default)]`?
- `clone_limit` — is there a use case for cloned cells in any DNA (e.g., per-household infrastructure cell)?
- How does `progenitor_pubkey` interact with elohim's stewardship-instead-of-ownership rule? Who is the progenitor of a protocol? (This is a real question — don't gloss.)
- Memory rule: `project_no_sovereignty_stewardship_over_ownership.md` — reconcile progenitor with stewardship framing.

**Matthew's note from parent session:** "H de-risks everything downstream. You can't adopt progenitor, lineage, or stable network_seed piecemeal — they want to be thought through together."

---

### 8. Moss Weave Tool (lamad-as-applet)

**Brief:** Package lamad (or a sub-feature of lamad like the quiz engine or content viewer) as a Moss Weave Tool. Expose it via `@theweave/api`, wire it to Moss's `ProfilesClient` for identity, render via `isWeaveContext()` detection.

**Moss is the "narthex" — trial doorway. Steward is the full liturgy.** Someone discovers elohim through a Moss group, gets curious about where the content really lives, graduates into running a full steward.

**Current state:** Zero Moss/Weave integration. No `@theweave/api` in deps.

**Upstream R&O reference:**
- v0.4.0 — R&O ships as a Weave Tool via `@theweave/api 0.6.3`
- `ui/src/lib/services/weave.service.ts` (lazy context detection, render-info parsing)
- `ui/src/lib/stores/weave.store.svelte.ts`
- `ProfileDisplayService` for hybrid Moss/standalone profile display
- `weave/` directory with Moss 0.15 curations, tool-list, weave.dev.config.json, applet-dev.sh

**Effort:** L
**Strategic weight:** H
**Depends on:** elohim-core graph substrate spec (ProfileDisplayService parallel pattern), #7 DNA manifest hygiene, possibly #4 hREA alignment
**Blocks:** nothing; enables HC-community user acquisition

**Brainstorm questions:**
- Which lamad sub-feature is the MVP applet (quiz engine? content viewer? whole lamad?)
- What's the identity bridge — Moss `ProfilesClient` talks to imagodei how?
- How do Moss-group-scoped EPRs relate to elohim's reach enum (commons/community/collective/steward/private)?
- Graduation UX — how does a Moss user learn they can run a steward and import their data?
- Does the Moss Tool speak directly to elohim-storage (REST), or does it rely on the Moss group's DHT cell and sync via elohim-agent?
- **Tension:** Moss is built on the "many small DHTs" scaling model. Elohim's thesis is that peer-sharded libp2p beats DHTs for global content. This sub-project is about presenting elohim *inside* Moss's federation model — design accordingly.

---

### 9. R&O graduation path doc

**Brief:** Write a concrete recipe for how a Moss group running R&O can "graduate" its historical exchange events into elohim. What happens to the R&O agent identity? How do `Request` and `Offer` records become EPRs published to elohim's substrate? What does the user experience of graduation look like?

**This is the pitch, made concrete.** Before building, write the recipe. Doing this first reveals what's missing in #7, #8, and especially #4.

**Current state:** No doc. Only conceptual framing from the parent session.

**Upstream R&O reference:** none directly — this is a greenfield doc describing a path across the R&O / elohim boundary.

**Effort:** S
**Strategic weight:** H (the pitch for Sasha + VF team; writing it exposes what's missing elsewhere)
**Depends on:** conceptual alignment from elohim-core graph substrate spec; informs #4 and #8 by revealing requirements
**Blocks:** the Phase-7 HC-team demo in the graph substrate spec

**Brainstorm questions:**
- Is "graduation" a one-time data migration, an ongoing bridge, or an identity handoff?
- How does the R&O agent's signing key relate to their imagodei identity — same key, linked, or separate?
- What does VF-GraphQL look like as the wire between them (depends on #4)?
- What happens to R&O's DHT network after graduation — does it persist for the group, or does the group sunset it?
- What's the governance model — does the group vote to graduate, or is it per-member?
- How does Moss group identity compose with elohim collective identity (per the `project_social_compute_collective_is_stewardship_unit.md` memory)?
- What's the credibility story — "your R&O history is preserved, your group continues, you gain a protocol-wide graph presence"?

**Format suggestion:** write this as a *narrative* doc (a user story / walkthrough), not a technical spec. The audience is Sasha, VF team, and R&O's existing users — they need to see what graduation *feels* like before they care how it works.

---

## 6. Recommended first brainstorm

From the parent session:

> **My recommendation for the first brainstorm: H (DNA manifest hygiene) or I (R&O graduation path doc).**
>
> Both are small, both are leveraged, both are the right *sequencing* moves:
> - **H (#7)** de-risks everything downstream. You can't adopt progenitor, lineage, or stable network_seed piecemeal — they want to be thought through together. And it's the thing the HC team will notice first if they look at your code.
> - **I (#9)** is the pitch made concrete. Before building anything, write the recipe. Doing this first tells you what's missing in H + D + G.

Matthew did not finalize a pick before pivoting to the graph substrate work. The new session should confirm with Matthew which sub-project to take first.

---

## 7. Relevant memory rules

The following memories in `/projects/.claude-config/projects/-projects-elohim/memory/` bear on this work:

- `project_no_sovereignty_stewardship_over_ownership.md` — reject own/ownership/sovereign vocabulary; relevant to #7 (progenitor framing)
- `project_elohim_active_observed_not_flagged.md` — derive flags from observation; relevant to #2
- `feedback_shift_measure_jenkins.md` — measures live in Jenkins, not locally; relevant to #3
- `project_doorway_manifest_driven_routes.md` — doorway is registry-driven proxy; relevant to any endpoint-level change
- `project_social_compute_collective_is_stewardship_unit.md` — design collective-general, not household-specific; relevant to #8, #9
- `feedback_schema_first_ioc.md` — schemas drive implementation; relevant to #4, #7
- `project_avodah_pillar.md` — avodah is protocol-as-process, not a domain pillar; touches #9 framing
- `project_stewardship_philosophy.md` — graduated capability, accountable authority; relevant to progenitor design (#7)

## 8. Relevant files and directories

- `elohim/sdk/CLAUDE.md` — capture test as SDK boundary rule
- `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` — existing ThreeLegCoupling structure
- `elohim/sdk/domains/lamad/manifest.json` + `elohim/sdk/domains/shefa/manifest.json` — existing domain manifests
- `elohim/holochain/dna/*/dna.yaml` — current DNA manifests (all 5 — the targets for #7)
- `steward/device/src-tauri/tauri.conf.json` — current bundle targets (target for #5)
- `app/elohim-app/src/app/shefa/` — shefa pillar implementation (target for #4)
- `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` — prior EPR-shaped-atom spec; vocabulary source
- `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md` — **parent session's parallel work**; do not duplicate

## 9. Out of scope for this handoff

- The elohim-core graph substrate spec (parent session)
- Apollo Client adoption as a general-purpose choice (re-scoped during parent session — GraphQL now has a full substrate-level design; Apollo is just one possible client)
- Anything about Holochain 0.6 upgrade (already done)
- Effect-TS 7-layer architecture adoption (out of scope unless Matthew raises it explicitly)
- Deep analysis of R&O internals (done; see §2)

---

## Appendix — original decomposition table from parent session

Kept for traceability. This is how the sub-projects were first framed before renumbering:

| Letter | Candidate | Effort | Strategic weight | Renumbered to |
|---|-----------|:---:|:---:|:---:|
| A | Release discipline (CHANGELOG, semver, `/release`) | M | M | #1 |
| B | Feature flag system (atomic, sense-and-respond aware) | S-M | M | #2 |
| C | Sweettest adoption + first test suite per DNA | M | H | #3 |
| D | hREA alignment — shefa speaks VF-GraphQL | L / XL | **H** | #4 |
| E | Tauri multiplatform bundling | S | M | #5 |
| F | Holochain Launcher listing | S | M | #6 |
| H | DNA manifest hygiene | S | **H** | #7 |
| G | Moss Weave Tool | L | **H** | #8 |
| I | R&O graduation path doc | S | **H** | #9 |
