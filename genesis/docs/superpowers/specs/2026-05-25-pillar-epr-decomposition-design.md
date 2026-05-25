# Pillar EPR Decomposition — Design

**Status:** Design (pending implementation plan)
**Date:** 2026-05-25
**Predecessor:** `genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md` established the three-tier Custodian → Steward → Operator chain via REA Commitments + Attestations. This spec adds the next ring of the same wheel — projection contracts between EPR stewards and doorway operators.
**P2P-design-gate:** Run inline. No new DHT entry types. Reuses existing `Commitment` (REA, content_store_integrity) with a new `action="project-epr"` discriminator.
**Operator of record (MVP):** Matthew (alpha.elohim.host + elohim.host doorway pair).

---

## 0. The picture in one paragraph

The elohim-app monolith decomposes into independently-deliverable pillar EPR apps (lamad, shefa, qahal, avodah, imagodei, account, doorway), each its own bundle, each projected by a notarized contract between the EPR steward and a doorway operator. The doorway becomes a transport-agnostic projection router: it consults active project-epr commitments scoped to itself, dispatches incoming URL paths by longest-prefix to the right EPR bundle, and serves the bytes from storage (or proxies to a steward's peer when the projection is in steward-direct mode). The protocol is peer-native first; doorway is one of several delivery contexts (alongside tauri-direct, browser-cached-offline, and future native P2P). EPR-links flip cards in place (HyperCard semantics) rather than triggering browser navigation, preserving session and scroll state across pillar boundaries. Gates and offline edges are not errors but designed experiences — every EPR can carry its outward face (a preview EPR + ordered EPR-typed hints) so reaching the boundary always points the user to a path forward, never a wall. The MVP splits lamad out of elohim-app as the dogfood proof; the rest of the pillars follow the pattern in subsequent shifts.

---

## 1. Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  CLIENTS                                                         │
│  Tauri ←─direct─→ Storage           Browser ←via doorway─→ Storage│
│  Browser ←─cache─→ ServiceWorker    Browser ←─P2P (future)─→ Peer│
└──────────────────────────────────────────────────────────────────┘
                              ▲
                              │ resolves via Loader (in elohim-core,
                              │ transport-agnostic; tries local → tauri →
                              │ doorway → peer in priority order)
                              │
┌──────────────────────────────────────────────────────────────────┐
│  DELIVERY CONTRACT — REA Commitment, action='project-epr'        │
│  • Notarized in elohim DNA (content_store_integrity zome)        │
│  • Carries: eprId, urlPath, mode, reach, baseHref, entryFile,    │
│    redirectsFrom, previewEprRef, gateHints, deadEnd, optional    │
│    stewardDirectEndpoint                                         │
│  • Per (EPR, doorway) registration — one EPR can be projected at │
│    multiple doorways; each is its own commitment                 │
│  • Operator-curated (first-wins on path conflict per doorway)    │
│  • Substrate-validated: gated projections require previewEprRef, │
│    non-empty gateHints, or explicit deadEnd=true                 │
└──────────────────────────────────────────────────────────────────┘
                              ▲
                              │ resolved at doorway boot +
                              │ refreshed on event bus
                              │
┌──────────────────────────────────────────────────────────────────┐
│  EPR LAYER — two content shapes:                                 │
│  1. Bundle EPR (e.g., lamad-spa, elohim-host-landing)            │
│     • Full Angular/Lit app, base href = its urlPath              │
│     • Declares element dependencies in its bundle manifest       │
│  2. Element-Registry EPR (e.g., lamad-elements)                  │
│     • Manifest of custom elements this pillar exposes            │
│     • Each element entry: tagName, cid, version, view-deps       │
│     • Loaded on demand by other bundles                          │
└──────────────────────────────────────────────────────────────────┘
                              ▲
                              │ both bundle + element bytes
                              │ resolved by CID through Loader
                              │
┌──────────────────────────────────────────────────────────────────┐
│  ELOHIM-CORE — the cross-cutting library every bundle imports    │
│  • Session primitive (current-user, capabilities, reach-check)   │
│  • EPR-link with HyperCard semantics + progressive loading       │
│  • Loader (transport-agnostic CID resolution)                    │
│  • Context-menu primitive (Google Drive-style fold-down)         │
│  • Page-chrome with slotted omnibar contract                     │
│  • Default omnibar (used when bundle doesn't BYO)                │
│  • Base atoms (button, card, badge — already there)              │
│  • Brand chrome + tokens                                         │
└──────────────────────────────────────────────────────────────────┘
                              ▲
                              │ ships as elohim-elements/elohim-core
                              │ NPM-published, every bundle dep
                              │
┌──────────────────────────────────────────────────────────────────┐
│  PILLAR BUNDLES (independently deliverable EPRs)                 │
│  lamad │ shefa │ qahal │ avodah │ imagodei │ account │ doorway   │
│  Each bundle:                                                    │
│  • base href = its urlPath                                       │
│  • Imports elohim-core                                           │
│  • Renders its own lens components (shefa-profile, lamad-       │
│    profile…) — each pillar's projection of imagodei identity     │
│  • Embeds OTHER pillars' chips via dynamic element loading       │
│  • BYO omnibar (e.g., lamad's existing toolbar) or default       │
│  • Publishes element-registry EPR for cross-pillar consumption   │
└──────────────────────────────────────────────────────────────────┘
```

### Key architectural properties

- **Substrate is transport-agnostic.** Bundles and elements are content-addressed EPRs. Any source serving the right CID is valid. Doorway is one source.
- **Imagodei is doubled.** As bedrock — session, identity, capability primitive lives in elohim-core, every bundle uses it. As pillar — the imagodei bundle handles enrollment, recovery, key management, the canonical self-view.
- **Lenses are per-pillar.** shefa-profile lives in shefa bundle; lamad-profile in lamad; etc. No pillar embeds another pillar's full lens — it embeds a tiny mention/chip from the other pillar's element-registry when needed.
- **Reach gates ride the projection.** A reach-gated EPR routes through doorway-to-conductor handoff (substrate exists per the stewardship-chain design).
- **Steward-direct mode is opt-in and isolated.** Projection commitment carries an explicit `stewardDirectEndpoint` (peer-id + alt-host + TLS pinning); the steward's peer must publish a matching acceptance attestation. No traffic spillover to peers that didn't sign up.
- **The home page is a pillar.** `elohim-host-landing` (and any future landing variants) are pillar-shaped EPRs projected at `/`. There is no privileged "shell pillar."
- **Designed boundaries.** Gates and offline edges are first-class EPR experiences via `previewEprRef` + `gateHints`. The substrate validator prevents accidental dead-ends.

---

## 2. Substrate primitive — project-epr REA Commitment

### 2.1 Why REA Commitment (not a new entry type)

The existing `operate-doorway` Commitment pattern (Phase 2 L3, landed) demonstrates that a (steward, doorway) projection contract maps cleanly onto REA Commitment with an action discriminator. project-epr is the next-tier-down version of the same shape:

- operate-doorway: human commits to operating a specific doorway with specific capabilities
- **project-epr**: steward commits to publishing a specific EPR via a specific doorway at a specific URL path

Both reuse:
- The same `Commitment` entry type in `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`
- The same content-addressed deterministic id (`sha256(provider_peer_id|action|scope)` → idempotent on re-seed, 409 on duplicate)
- The same projection store in `elohim/elohim-storage/src/db/rea_commitments.rs`
- The same seeder pattern (`genesis/seeder/src/seed-operator-bindings.ts` → mirror as `seed-projections.ts`)
- The same JWT capability-snapshot resolver pattern (`find_active_operator_binding` → mirror as `find_active_projections` keyed by doorway_id)

**No new DHT entry type, no new zome coordinator function, no new substrate primitive.** project-epr is operationally a new action value on the existing Commitment infrastructure.

### 2.2 EprProjectionView wire shape

```rust
// elohim/elohim-views/src/projection.rs
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprProjectionView {
    pub commitment_id: String,            // sha256(...) deterministic
    pub epr_id: String,                   // e.g. "lamad-spa"
    pub doorway_id: String,               // e.g. "doorway:alpha-elohim-host"
    pub url_path: String,                 // e.g. "/lamad" or "/"
    pub mode: ProjectionMode,             // "cached" | "steward-direct"
    pub reach: String,                    // "commons" | "qahal:xyz" | etc.
    pub base_href: String,                // e.g. "/lamad/" — for static asset resolution
    pub entry_file: String,               // "index.html" default
    pub redirects_from: Vec<String>,      // old urlPaths that should 302 to current
    pub preview_epr_ref: Option<String>,  // EPR id of the outward-face preview
    pub gate_hints: Vec<GateHintRef>,     // ordered, see §6
    pub dead_end: bool,                   // explicit "no path through" opt-in
    pub steward_direct_endpoint: Option<StewardDirectEndpoint>,  // see §2.4
    pub seeded_at: String,                // ISO timestamp
    pub seeded_by: String,                // steward peer_id
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
pub enum ProjectionMode {
    Cached,
    StewardDirect,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
pub struct GateHintRef {
    pub epr_ref: String,                  // e.g. "epr:susan-aleph-elder"
    pub label: Option<String>,            // e.g. "Susan, who stewards aleph household"
    pub relation: GateHintRelation,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
pub enum GateHintRelation {
    PersonWhoCanGrant,
    MembershipPrerequisite,
    ContentToSync,
    PlaceToVisit,
    CapabilityToEarn,
    PaymentToOffer,
    WitnessToInvolve,
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
pub struct StewardDirectEndpoint {
    pub peer_id: String,                  // libp2p peer id
    pub alt_host: Option<String>,         // optional DNS pointing at the peer
    pub tls_cert_san: String,             // peer-id-as-SAN OR PEM-encoded leaf pin
    pub accepts_projection_for: Vec<String>,  // EPR ids this endpoint serves
}
```

### 2.3 Schema contract

JSON Schema at `elohim/sdk/schemas/v1/views/epr-projection-view.schema.json`. Schema-first per the established pattern: write the schema, the Rust struct conforms, the validation harness at `elohim/elohim-storage/tests/schema_contract.rs` catches drift, codegen generates TS interfaces.

### 2.4 Substrate validator

Runs at Commitment-create time in the storage handler when `action=="project-epr"`:

```
fn validate_project_epr_commitment(c: &EprProjectionView) -> Result<()> {
    // Rule 1: non-commons reach requires a path forward
    if c.reach != "commons"
        && c.preview_epr_ref.is_none()
        && c.gate_hints.is_empty()
        && !c.dead_end
    {
        return Err("Gated projection must declare at least one of: \
                    previewEprRef, gateHints (non-empty), or deadEnd=true");
    }

    // Rule 2: steward-direct mode requires endpoint declaration
    if c.mode == ProjectionMode::StewardDirect && c.steward_direct_endpoint.is_none() {
        return Err("steward-direct mode requires stewardDirectEndpoint");
    }

    // Rule 3: urlPath must be well-formed (starts with /, no trailing slash unless "/")
    validate_url_path(&c.url_path)?;

    // Rule 4: preview EPR (if declared) must itself resolve to commons-reach
    //         (validated lazily — preview unreachability is a runtime issue
    //          but commons-validation is checked on commit if the preview
    //          EPR is currently resolvable; else a soft warning)
    Ok(())
}
```

The validator runs in MVP from day one even though MVP only creates commons-reach projections. Opinionated substrate constraints from the start prevent whole classes of future-author footguns.

### 2.5 Resolver + events

```
elohim/elohim-storage/src/db/rea_commitments.rs:
  find_active_projections(doorway_id: &str) -> Vec<EprProjectionView>
  find_projection_by_url_path(doorway_id, url_path) -> Option<EprProjectionView>
  // longest-prefix match implementation

elohim/elohim-storage/src/services/events.rs:
  StorageEvent::ProjectionRegistered { commitment_id }
  StorageEvent::ProjectionRevoked { commitment_id }
  // Emitted from REA Commitment create/cancel when action == 'project-epr'
```

The events flow through the existing SSE bus that the doorway's `storage_events_subscriber` already consumes (Pattern Z bridge). New event kinds are added to the subscriber's handled set; on receipt, the doorway's `epr_router` refreshes its path-prefix table.

---

## 3. Element-Registry EPR pattern

### 3.1 Concept

Custom elements are themselves EPRs. A pillar publishes an "element-registry EPR" — a small manifest declaring "these are the custom elements I expose for other pillars to embed." Cross-pillar embedding fetches elements from those registries via the same Loader machinery that fetches bundles. CIDs identify exact versions; the doorway projection system delivers them like any other EPR.

### 3.2 Wire shape

```rust
// elohim/elohim-views/src/element_registry.rs
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
pub struct ElementRegistryView {
    pub epr_id: String,                   // e.g. "elohim-core-elements" or "lamad-elements"
    pub pillar: String,                   // e.g. "elohim-core", "lamad", "qahal"
    pub elements: Vec<ElementEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[serde(rename_all = "camelCase")]
pub struct ElementEntry {
    pub tag_name: String,                 // e.g. "qahal-attestation-chip"
    pub cid: String,                      // exact blob hash of the element's JS bundle
    pub version: String,                  // semver
    pub view_deps: Vec<String>,           // ts-rs view types this element consumes
}
```

Stored as content with `contentFormat: "element-registry-manifest"` — a new content format added to the lamad manifest.

### 3.3 MVP scope

ONE element-registry EPR ships in MVP: `elohim-core-elements`, declaring the elements elohim-core itself publishes (elohim-button, elohim-card, elohim-epr-link, elohim-page-chrome, elohim-default-omnibar, elohim-context-menu, elohim-skeleton, elohim-mention-base, etc.). Per-pillar registries (lamad-elements, qahal-elements, etc.) are deferred — the pattern is designed and the substrate is ready, but no other pillar publishes one until its own bundle split.

### 3.4 Loader contract for elements

In MVP, the Loader resolves elements only via the elohim-core registry (every bundle imports them at build time). The dynamic-loader-from-other-pillars code path is implemented but exercised only in tests, not in any production-rendered surface, until the second pillar splits.

---

## 4. Elohim-core library

The cross-cutting library every bundle imports. Already exists as `app/elohim-elements/elohim-core/` with button, compute-tile, capability primitives. This spec adds:

### 4.1 New primitives

| Primitive | Path | Responsibility |
|---|---|---|
| `<elohim-epr-link>` | `src/elohim-epr-link.ts` | HyperCard navigation primitive; renders chip/inline/card/popover; progressive loading; default-click navigation; right-click context menu |
| `<elohim-context-menu>` | `src/elohim-context-menu.ts` | Google Drive-style fold-down menu; accessibility (Shift+F10, keyboard nav, ARIA); composable for use beyond EPR-links |
| `<elohim-page-chrome>` | `src/elohim-page-chrome.ts` | Bundle root wrapper; slotted omnibar contract |
| `<elohim-default-omnibar>` | `src/elohim-default-omnibar.ts` | Default omnibar implementation (brand mark + auth + location); used when bundle doesn't BYO |
| `<elohim-skeleton>` | `src/elohim-skeleton.ts` | Sized shimmer placeholder; used by progressive-loading states |
| `<elohim-mention-base>` | `src/elohim-mention-base.ts` | Generic fallback chip element for cross-pillar EPR references when the specific pillar's mention element isn't loaded |
| `Loader` class | `src/loader/loader.ts` | Transport-agnostic CID resolution; cache → tauri-direct → doorway → peer; preview-aware fallback; CID verification |
| `Session` service | `src/session/session.ts` | Reactive current-user, capabilities, reach context; reads doorway-set cookie/JWT |
| Omnibar contract | `src/contracts/omnibar.contract.ts` | TS interface that any BYO omnibar consumes |

### 4.2 Bundle integration

Every pillar bundle imports `@elohim/elements-core` (or whatever the resolved package name becomes). The bundle's root template wraps content in `<elohim-page-chrome>`. Bundles that have their own toolbar (like lamad) slot it into `slot="omnibar"`; bundles that don't get the default automatically.

### 4.3 Angular bridge (transitional)

The remaining elohim-app (post-lamad-split) and any other Angular surface uses thin Angular wrappers around the Lit elements so existing Angular call sites keep working. Path: `app/elohim-app/src/app/elohim/components/epr-link/` becomes a wrapper around `<elohim-epr-link>`.

---

## 5. Data flow scenarios

### 5.1 Scenario A — Hard browser navigation to a pillar URL

User types `https://alpha.elohim.host/lamad/concept/fair-exchange` in a fresh tab.

```
Browser → ingress → Doorway A
  1. epr_router consults path-prefix table (built from active
     project-epr commitments scoped to doorway:alpha-elohim-host)
  2. Longest match: /lamad → eprId=lamad-spa
  3. Sub-path: /concept/fair-exchange
  4. Reach gate check: commitment.reach=commons → serve.
  5. Bundle resolution: find lamad-spa's current blob_hash.
  6. Sub-path resolution:
       - asset file in bundle? → serve from cache, 200
       - mode=spa, not in bundle? → serve index.html, 200
         (Angular router handles /concept/fair-exchange)

Browser receives index.html:
  - <elohim-page-chrome> mounts
  - lamad-toolbar slots into omnibar
  - Lamad's Angular app mounts
  - Angular router resolves <concept-view epr="fair-exchange">
```

### 5.2 Scenario B — EPR-link click (HyperCard card-flip)

User is viewing `/lamad/concept/fair-exchange`. The concept contains `<elohim-epr-link epr="epr:qahal-attestation-abc123" display="chip">`.

```
User clicks the chip
  → <elohim-epr-link> handler
  → resolveInContext(epr:qahal-..., currentPath=lamad, currentSteps=[concept/...])
  → Cross-pillar; display=chip → resolve content only

Loader.resolveContent(epr:qahal-attestation-abc123)
  Resolution order:
    1. localCache hit? → return bytes
    2. tauri-direct (if window.__TAURI__) → fetch from :8090
    3. doorway projection → GET /api/v1/epr/qahal-attestation-abc123
    4. peer P2P (future)

<elohim-epr-link> updates DOM with resolved chip
  - No browser navigation
  - Lamad's Angular app stays mounted
  - Scroll position + state preserved
```

### 5.3 Scenario C — Bundle redeploy + cache eviction

```
Jenkinsfile:
  1. Build new lamad bundle → dist/lamad-spa/
  2. Zip; SHA: sha256-xyz...
  3. PUT /admin/seed/blob (uploads bytes)
  4. PATCH /db/content/lamad-spa { blobHash: "sha256-xyz..." }

Storage:
  1. UpdateContent in SQLite
  2. emit StorageEvent::ContentUpdated { id: "lamad-spa" }
  3. load_slug_index() — refresh fast-path

SSE bus → storage_events_subscriber in each doorway:
  1. Receives content.updated for "lamad-spa"
  2. AppFileCacheService::clear_slug("lamad-spa")

Next browser request to /lamad/anything:
  1. epr_router still has the projection
  2. resolve_blob_hash → cache miss
  3. Fall through to storage → returns sha256-xyz...
  4. Extract from new ZIP, serve, repopulate cache.
```

### 5.4 Scenario D — Reach-gated EPR (substrate-ready, MVP doesn't exercise)

```
Browser → Doorway → epr_router matches /community/aleph → eprId=qahal-aleph-home
  projection.reach = "qahal:aleph-members"
  request has no auth → reach gate fails

Doorway responds:
  - Hard nav: 302 → /auth/signin?return=/community/aleph
    (auth EPR — itself commons-reach — handles login)
  - Card-flip: { error: "auth-required", retryAfterAuth: true }
    Calling bundle renders inline auth-wall card, retries on success
```

---

## 6. Edge behavior — designed boundaries

### 6.1 Principle

Gates and offline edges are not errors. Every EPR can present its outward face when its interior is unreachable. The substrate enforces this via the gate-hint validator (§2.4).

### 6.2 Case 1 — Reach gate fails

```
Doorway resolves projection → reach check fails
  Lookup projection.previewEprRef
    - If present: resolve THAT EPR (commons-reach by design)
    - If absent: generate default preview from public metadata

Response:
  - Hard nav: serve preview bundle with pinned header chip
  - Card-flip: { kind: "preview", preview: {...}, gateHints: [...] }
```

### 6.3 Case 2 — Offline + uncached

```
Loader tries each source, all fail (no connectivity)
  - Check local cache for previously-fetched preview
  - If preview cached: return preview with offline marker
  - If nothing cached: return "unknown EPR" stub with EPR id

UI: "This content isn't on your device. Syncs when back online."
For cached previews, render the rich preview as-is.
```

### 6.4 Case 3 — Projection exists, underlying EPR unreachable

```
Doorway resolves /lamad/... → projection → lamad-spa content row 404 or blobHash=""
Response: "this surface is being provisioned" preview with refresh affordance
          and projection.seededAt timestamp ("registered 2 days ago, awaiting first deploy")
```

### 6.5 Case 4 — Cross-pillar element fails to load

```
Bundle declares <qahal-mention epr="...">
Loader tries:
  - elohim-core registry (no match)
  - qahal element-registry EPR (unreachable / not cached)

Fallback: <elohim-mention-base epr="..."> renders minimal chip
          from whatever Loader CAN resolve.
Production: silent substitution + warning log.
```

### 6.6 Gate hints as recursive EPR graph

Each `GateHintRef` is itself an EPR. The user navigating a gate sees the hint EPRs rendered as chips; clicking one resolves that EPR (which may itself have its own preview + hints if also gated). No dead ends unless explicitly authored.

```
Locked EPR: qahal-aleph-proposal-xyz
  previewEprRef: epr:qahal-aleph-proposal-xyz-preview (commons, designed)
  gateHints:
    - { epr: epr:susan-aleph-elder, label: "Talk to Susan",
        relation: PersonWhoCanGrant }
    - { epr: epr:aleph-qahal-enrollment, label: "Join Aleph household",
        relation: MembershipPrerequisite }
  deadEnd: false

User experience: rich preview card + two clickable chips. Each chip
resolves another EPR. Susan's EPR might be commons-reach (always
visible). The enrollment EPR might itself be reach-gated to people
introduced by an existing member, with its own preview + hints
chain pointing further back.
```

---

## 7. EPR-link interaction model

### 7.1 Progressive loading

```
L1 (instant, EPR id only): pillar icon, EPR id, sized skeleton
L2 (first byte, cache or network): title, reach badge
L3 (full resolve): description, attestation summary, pillar metadata
L4 (preview if primary failed): preview content + gate hints inline
```

### 7.2 Default click

Single click navigates contextually via `resolveInContext()`:
- In-path → stay in path
- Cross-path → flip card or hard-nav depending on display variant
- Standalone → standalone resource view

### 7.3 Right-click context menu (Google Drive style)

```
┌─────────────────────────────────────────┐
│  Open                            ▸      │
│  ─────────────────────────────          │
│  View as...                      ▸      │  ← pillar lenses (per-registry)
│  Where this leads...             ▸      │  ← preview + gateHints chain
│  ─────────────────────────────          │
│  About this EPR                  ▸      │  ← reach, attestation chain, hash
│  Save to device                         │  ← explicit sync to local cache
│  Copy EPR link                          │  ← epr:xxx clipboard
└─────────────────────────────────────────┘
```

Accessibility: Shift+F10, Context menu key, full keyboard navigation, ARIA roles. Designed as Library A blank-slate + Library B brand-bound (graphos-designer).

### 7.4 MVP scope of the menu

Foundation in MVP: progressive loading (all 4 layers), default-click navigation, context-menu primitive scaffolded with three items only (**Open**, **About this EPR**, **Copy EPR link**). The "View as..." lens drill-down and "Where this leads..." hint chain land in a fast-follow shift, paired with the first per-pillar element-registry or the first gated projection (whichever ships first).

---

## 8. MVP scope

### 8.1 What ships

| Layer | Item |
|---|---|
| Substrate | `project-epr` REA action constant in `rea_commitments.rs` |
| Substrate | `EprProjectionView` + `GateHintRef` + `StewardDirectEndpoint` types |
| Substrate | JSON schema + schema contract test |
| Substrate | Validator enforcing non-dead-end + steward-direct-requires-endpoint rules |
| Substrate | `find_active_projections` + `find_projection_by_url_path` resolvers |
| Substrate | `ProjectionRegistered`/`ProjectionRevoked` events on the SSE bus |
| Seeder | `seed-projections.ts` mirroring `seed-operator-bindings.ts` |
| Seeder | 4 default projections: (landing@/, lamad@/lamad) × (alpha, elohim.host) |
| Seeder | `elohim-core-elements` element-registry seed |
| Doorway | `epr_router` module: load on boot, longest-prefix dispatch, SSE-refreshed |
| Doorway | EPR resolution endpoint `/api/v1/epr/{id}` (new route — confirmed missing today; only referenced in Pattern Z comments as future work) |
| Doorway | Drop `ROOT_APP_SLUG` env var |
| Doorway | Cache eviction on `ProjectionRegistered`/`ProjectionRevoked` |
| Lamad bundle | New Angular project at `app/lamad/` with `<base href="/lamad/">` |
| Lamad bundle | Pillar code moves from `app/elohim-app/src/app/lamad/` → `app/lamad/src/app/` |
| Lamad bundle | Existing toolbar wrapped to slot into `<elohim-page-chrome slot="omnibar">` |
| Elohim-app | `/lamad` route subtree removed; rest of monolith stays |
| Elohim-core | Loader, Session primitive, `<elohim-epr-link>`, `<elohim-page-chrome>`, `<elohim-default-omnibar>`, `<elohim-skeleton>`, `<elohim-mention-base>`, `<elohim-context-menu>` (minimal menu) |
| Elohim-core | Library A default stories + Library B designed stories for each |
| Pipeline | Jenkinsfile builds + uploads + PATCHes both bundles in one deploy run |
| Pipeline | Prebuild ordering: elohim-core builds before both Angular bundles |
| Tests | All unit + integration tests in §10 |
| Tests | Two a2o feature files: `native-epr-projection.feature` + `epr-link-hypercard.feature` |
| Dogfood | alpha.elohim.host serves both URL surfaces; operator-of-record signoff |

### 8.2 What's deferred (with rationale)

| Item | Why deferred | Likely shift |
|---|---|---|
| elohim.host TLS + ingress | Operator handles in parallel; not blocking lamad work | Operator-owned |
| Per-pillar element registries (shefa, qahal, etc.) | Wait for second pillar split to dogfood the pattern | Shift after MVP |
| Reach-gate enforcement at projection | No gated projections exist yet in MVP | Paired with first gated EPR |
| Steward-direct mode implementation | Schema present; no peer accepts traffic yet | Future |
| Per-EPR compute metering and back-charging | Foundation is the projection commitment; metering is a separate concern | Future |
| Rich context menu items ("View as...", "Where this leads...") | Foundation in MVP; richness in fast-follow | Fast-follow shift |
| Remaining pillar splits (shefa, qahal, avodah, imagodei, account, doorway) | Lamad dogfood informs order + pattern refinements | One shift per pillar |
| EPR-preview as a content format | Schema field exists; format implementation deferred until first preview is authored | Paired with first gated EPR |

---

## 9. Testing surfaces

### 9.1 Substrate (Rust)

| Test | Type | Path |
|---|---|---|
| EprProjectionView schema contract | integration | `elohim/elohim-storage/tests/schema_contract.rs` |
| project-epr create happy path | unit | `elohim/elohim-storage/src/db/rea_commitments.rs` `#[cfg(test)]` |
| Commons reach: no validator failure | unit | same |
| Non-commons + missing preview/hints/deadEnd → reject | unit | same |
| Non-commons + deadEnd=true → accept | unit | same |
| Non-commons + hints provided → accept | unit | same |
| Steward-direct mode requires endpoint → reject without | unit | same |
| `find_active_projections(doorway_id)` filters correctly | unit | same |
| `find_projection_by_url_path` returns longest-prefix | unit | same |
| `ProjectionRegistered`/`Revoked` events on create/cancel | unit | `elohim/elohim-storage/src/services/events.rs` |

### 9.2 Doorway (Rust)

| Test | Type | Path |
|---|---|---|
| epr_router builds path-prefix table | unit | `doorway/doorway-service/src/projection/epr_router.rs` |
| epr_router longest-prefix dispatch (table-driven) | unit | same |
| epr_router refreshes on `ProjectionRegistered` SSE | integration | `doorway/doorway-service/src/projection/storage_events_subscriber.rs` |
| epr_router refreshes on `ProjectionRevoked` SSE | integration | same |
| epr_router falls through when no projection matches | integration | `doorway/doorway-service/src/server/http.rs` |
| `/` driven by projection (no `ROOT_APP_SLUG`) | integration | same |
| reach=commons projection serves anonymously | integration | same |
| reach != commons → gate response (`#[ignore]` in MVP, doc'd) | integration | same |

### 9.3 Elohim-core (TypeScript)

| Test | Type | Path |
|---|---|---|
| Loader resolves from localCache | vitest unit | `app/elohim-elements/elohim-core/src/loader/loader.spec.ts` |
| Loader falls through to tauri-direct when localCache misses + `window.__TAURI__` | vitest unit | same |
| Loader falls through to doorway projection | vitest unit | same |
| Loader returns preview on primary fetch failure | vitest unit | same |
| Loader returns minimal stub when all sources fail | vitest unit | same |
| Loader verifies CID before caching/returning | vitest unit | same |
| `<elohim-epr-link>` renders inline/chip/card/popover | wtr unit | `app/elohim-elements/elohim-core/src/elohim-epr-link.spec.ts` |
| `<elohim-epr-link>` progressive loading state transitions L1→L4 | wtr unit | same |
| `<elohim-epr-link>` default click navigates | wtr unit | same |
| `<elohim-epr-link>` right-click opens context menu | wtr unit | same |
| `<elohim-epr-link>` renders preview chip when target unreachable | wtr unit | same |
| `<elohim-context-menu>` minimal items (Open/About/Copy) render | wtr unit | `app/elohim-elements/elohim-core/src/elohim-context-menu.spec.ts` |
| `<elohim-context-menu>` keyboard navigation (Shift+F10, arrows, Enter, Escape) | wtr unit | same |
| `<elohim-page-chrome>` default omnibar fallback | wtr unit | `app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts` |
| `<elohim-page-chrome>` slotted omnibar suppresses default | wtr unit | same |
| Session primitive reactive state | vitest unit | `app/elohim-elements/elohim-core/src/session/session.spec.ts` |

### 9.4 Library A & B stories

| Owner | Path |
|---|---|
| component-architect | `app/elohim-library/projects/graphos/src/default/core/__docs__/*.default.stories.ts` for each new primitive |
| graphos-designer | `app/elohim-library/projects/graphos/src/designed/core/__docs__/*.designed.stories.ts` for same |

### 9.5 Lamad bundle (migration + new)

- All existing lamad tests in `app/elohim-app/src/app/lamad/**/*.spec.ts` move to `app/lamad/src/app/**/*.spec.ts` with import path updates.
- New: `<base href>` assertion test (`app/lamad/test/base-href.spec.ts`)
- New: toolbar-as-omnibar Cypress test (`app/lamad/cypress/e2e/toolbar-as-omnibar.cy.ts`)
- New: elohim-core integration smoke test (`app/lamad/src/app/elohim-core-integration.spec.ts`)

### 9.6 a2o scenarios (executable specifications)

#### `genesis/a2o/features/doorway/native-epr-projection.feature`

```gherkin
Feature: Doorway natively projects EPRs at author-declared URL paths
  As a steward of a doorway, I declare which EPRs my doorway hosts and at
  what URL paths, so visitors reach the protocol-native experience via clean
  web2.0 URLs without doorway-side hardcoding.

  Scenario: Bare hostname serves the landing EPR
    Given the alpha.elohim.host doorway has an active project-epr commitment
      for "elohim-host-landing" at urlPath "/"
    When an anonymous browser GETs "https://alpha.elohim.host/"
    Then the response serves the landing EPR's bundle entry file
    And the response is HTTP 200

  Scenario: Pillar path serves the pillar EPR
    Given the alpha.elohim.host doorway has an active project-epr commitment
      for "lamad-spa" at urlPath "/lamad"
    When an anonymous browser GETs "https://alpha.elohim.host/lamad/concept/fair-exchange"
    Then the response serves the lamad bundle's index.html (SPA fallback)
    And the lamad bundle's <base href> is "/lamad/"
    And Angular client-side router handles "/concept/fair-exchange"

  Scenario: Bundle redeploy evicts doorway cache
    Given the lamad-spa EPR's blob is sha256-OLD
    When a deploy PATCHes the lamad-spa EPR with blobHash sha256-NEW
    Then within 5 seconds, the doorway's cache for "/lamad/index.html" is evicted
    And the next browser request to "/lamad/index.html" serves bytes from sha256-NEW

  Scenario: Federation — same EPR projected on second doorway serves same content
    Given the elohim.host doorway also has an active project-epr commitment
      for "lamad-spa" at urlPath "/lamad"
    When an anonymous browser GETs "https://elohim.host/lamad/"
    Then the response serves the same lamad bundle as alpha.elohim.host
    And both doorways' projections reference the same blob_hash
```

#### `genesis/a2o/features/elohim-core/epr-link-hypercard.feature`

```gherkin
Feature: EPR-links flip cards in place, preserving context
  Inside a mounted pillar bundle, clicking an EPR-link to another EPR resolves
  content inline (HyperCard flip) rather than triggering a browser navigation.
  The user keeps their session, scroll, and state.

  Scenario: EPR-link with display=chip resolves content inline
    Given the user is viewing "/lamad/concept/fair-exchange" in a mounted lamad bundle
    And the concept view contains <elohim-epr-link epr="epr:elohim-host-landing" display="chip">
    When the chip resolves
    Then the chip renders with the landing EPR's title and metadata
    And no browser navigation occurs
    And the lamad Angular app remains mounted

  Scenario: EPR-link to unreachable target renders the preview, not an error
    Given an EPR-link points to an EPR that is currently unreachable
    And the EPR has a previewEprRef declared
    When the link resolves
    Then the chip renders the preview EPR's content
    And the chip displays the offline/unreachable marker

  Scenario: EPR-link right-click opens the context menu
    Given the user is viewing a page containing an <elohim-epr-link>
    When the user right-clicks the link
    Then a context menu opens with Open, About this EPR, and Copy EPR link
    And the menu can be navigated by keyboard (arrows, Enter, Escape)
```

### 9.7 Manual dogfood (operator-of-record signoff)

1. `alpha.elohim.host/` loads the landing page, looks identical to today.
2. `alpha.elohim.host/lamad` loads the lamad app (separate bundle, base href `/lamad/`, lamad's own toolbar in slotted position).
3. Angular routes inside lamad work (`/lamad/path/something`, `/lamad/concept/xyz`, etc.).
4. Clicking an EPR-link from a lamad concept to a landing reference flips a card inline — no shell reload.
5. Right-clicking an EPR-link opens the (minimal) context menu with Open/About/Copy.
6. After operator's TLS work on elohim.host, the same two URLs work there too.
7. Bundle redeploy via CI lands new bytes within seconds of PATCH completing — no manual cache clear.

---

## 10. Definition of done

MVP complete when:

- All Rust + TypeScript unit tests above pass.
- Both a2o feature files pass via Cypress + Cucumber in CI.
- Library A + B stories for all new elohim-core elements render in Storybook.
- A clean reseed of a dev environment produces 4 project-epr commitments (2 EPRs × 2 doorways) plus the elohim-core-elements registry.
- alpha.elohim.host serves both URL surfaces, manually verified by the operator-of-record.
- `ROOT_APP_SLUG` env var is removed from doorway deployment manifests.
- The Jenkinsfile produces two bundle artifacts (elohim-app + lamad) and PATCHes both content rows in a single deploy run.

Deferred items that do NOT block MVP done are listed in §8.2.

---

## 11. Open questions for the implementation plan

These are decisions the writing-plans skill will handle when sizing the shifts, but worth flagging here:

1. **Shift boundary** — is MVP one shift or two? Substrate + seeds is naturally self-contained; doorway router + bundle split + elohim-core extract is a larger lump. Recommend two shifts (A: substrate + seeds, B: doorway router + bundle split + elohim-core), but the writing-plans skill should evaluate based on actual estimated complexity.

2. **Element library extraction order** — Loader and Session primitive can be developed independently of EPR-link/page-chrome/context-menu. Could parallelize between component-architect (UI primitives) and angular-architect (Loader integration), but adds coordination overhead. Worth considering whether to do them serially or in parallel.

3. **Angular wrapper strategy** — for transitional period, the remaining elohim-app monolith needs Angular components wrapping the Lit elements. Should we extract ALL `<elohim-*>` Angular wrappers in one pass, or only the ones the post-lamad-split monolith actually uses?

4. **Lamad pillar service dependencies** — lamad has 37 services, some of which may depend on cross-pillar services in the elohim pillar (60 services). The bundle split needs to either (a) duplicate those services in lamad, (b) extract them to elohim-core, or (c) consume them via the elohim-app's HTTP API. Audit needed during planning.

---

## Appendix A: Glossary

- **EPR**: Elohim Protocol Resource — a content-addressed unit of protocol-native content (apps, documents, concepts, attestations, humans, places). Notarized in Holochain DHT, projected to web2.0 via doorway commitments.
- **EPR-link**: HyperCard-style navigation primitive that resolves content inline rather than triggering browser navigation. Preserves session/scroll/state across pillar boundaries.
- **Projection**: A notarized commitment between an EPR steward and a doorway operator that the doorway will serve the EPR at a declared URL path under specified terms (reach, mode, gate hints, etc.).
- **Pillar EPR**: A protocol pillar (lamad, shefa, qahal, etc.) packaged as its own independently-deliverable bundle EPR.
- **Element-Registry EPR**: A manifest EPR declaring which custom elements a pillar exposes for cross-pillar embedding.
- **Lens**: A pillar's projection of a primitive concept — e.g., shefa-profile is a lens over imagodei identity, contextualized for shefa's economic vocabulary.
- **Steward-direct mode**: Projection mode where the doorway tunnels traffic directly to the steward's peer instead of caching+serving. Requires explicit acceptance attestation on the receiving peer to prevent traffic spillover to uninvolved peers.
- **Designed boundary**: An author-designed experience at a reach gate or offline edge, rendered via preview EPR + gate-hint EPR chain rather than a generic error.
- **Gate hint**: An EPR reference (person, qahal, lesson, place, capability, etc.) that, if reached, would unlock or progress access to the gated EPR. Recursive — hint EPRs may themselves have gates with hints.

---

## Appendix B: P2P-design-gate decisions

Per `.claude/skills/p2p-design-gate/SKILL.md`:

1. **Entity classification**: project-epr commitment is **notarized (A)** — REA Commitment in elohim DNA's content_store_integrity zome. Two-party attestation conceptually (steward + operator); MVP allows single-key signing when both are operationally the same agent, per §11.1 of the predecessor spec.

2. **Existing DHT entry type**: YES — `Commitment` exists. New `action="project-epr"` value reuses the existing entry type.

3. **Identity**: Content-derived via `sha256(steward_peer_id|action|scope)` where scope is `doorway:{doorway_id}|epr:{epr_id}`. Deterministic, idempotent, re-runs collapse to 409.

4. **Coordinator function**: `create_rea_commitment` (existing) with new action discriminator. New validator (§2.4) enforces project-epr-specific constraints. Signal: `ProjectionRegistered`/`ProjectionRevoked` events on the existing SSE bus.

The new entity (`ElementRegistryView`) is **operational (C)** — projected from content rows with `contentFormat="element-registry-manifest"`. Not separately notarized; consistency comes from the underlying content row's existing flow.

---

*End of design.*
