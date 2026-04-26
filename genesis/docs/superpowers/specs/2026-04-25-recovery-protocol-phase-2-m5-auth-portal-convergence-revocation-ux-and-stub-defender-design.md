# Recovery Protocol Phase 2 — M5: Auth Portal Convergence + Revocation UX + Stub Defender Specialist

**Status:** Draft
**Date:** 2026-04-25
**Owner:** Matthew Dowell
**Supersedes:** None (extends Phase 2 spec)
**Builds on:**
- `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` (Phase 2 spec)
- `genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation-design.md` (M4 spec)
- EPR Phase 2B Batch A (merged at `79181f8e` — `AgentPeerBinding`, `ReconcileController`, identity handshake protocol, `peer_identity_bindings` projection)

**Kickoff prompt:** `genesis/docs/plans/2026-04-24-recovery-m5-elohim-defender-and-revocation-ux-kickoff-prompt.md` (reframed during brainstorm — see §1.2)

---

## 1. Vision Alignment

### 1.1 Phase 2 commitments (unchanged)

This milestone honors the four architectural commitments from the Phase 2 spec:
- **Graduated authority** — community can always make it right.
- **Elohim as counsel** — first-class defense of imagodei (M5 ships the *stub* shape; real defense lives behind the same coordinator gate when detection wires up later).
- **Ungrudging service** — gifts flow without recognition.
- **Cradle-to-grave care** — dissolution is part of recovery.

### 1.2 M5-specific framing

The kickoff prompt scoped M5 as "elohim defender backend + revocation UX." Brainstorm reframed it. M5 ships:

1. **Auth portal convergence** — connecting the existing hosted-doorway portal (web2 IdP) with the peer-native steward portal (peer-rendered IdP). The elohim-app today is a bootstrap/catch-all client; M5 graduates its first pillar to a composability shape that supports rendering on own-tauri, own-browser-steward, or peer-on-your-behalf.
2. **Revocation UX as visible deliverable** — M4's `KeyRevocation` + `RevocationVote` primitives become human-visible inside the new account-management pillar.
3. **Defender specialist as stub** — the elohim-defender role marker, manifest, and coordinator gate ship; real detection is deferred to a later sprint when elohim integration is concrete.

The deliverable is a **clearly-defined SDK/API/graph surface**. UI is minimum-viable scaffold to prove wiring works. Polished presentation is a separate Playwright-driven sprint.

This reframe aligns with three durable framings recorded in memory:
- `project_imagodei_three_surfaces` — social profile (Surface 1, exists), self-knowledge (Surface 2, separate sprint), account management (Surface 3, M5 ships).
- `project_peer_native_account_canonical_surface` — OAuth-pattern graduation: doorway is relying party post-graduation; elohim-app fills the holochain-launcher / moss-launcher provider role.
- `project_elohim_app_as_composable_view_federation` — pillars graduate to surfaces renderable by own-tauri / own-browser-steward / peer-on-your-behalf; M5 graduates the auth pillar first.

---

## 2. Scope

### 2.1 In scope

**imagodei DNA:**
- New entry type: `PortalHost` (Category A — see §6 P2P Design Gate).
- New coordinator functions:
  - `add_portal_host(input)`
  - `remove_portal_host(host_url)`
  - `get_my_portal_hosts() -> Vec<Link>`
  - `submit_specialist_revocation(input)` — stub producer for the M4-validator-accepted `trigger_type = "specialist_attestation"` path.
- Validators for the above.
- Post-commit signals: `PortalHostCreated`, `PortalHostRemoved`, plus the existing `KeyRevocationCreated` signal already used by the controller.

**elohim-storage:**
- New view types in `views.rs`:
  - `PortalHostView` (new)
  - `AgentPeerBindingView` (project-out from existing DHT entry — never reached HTTP before)
  - `KeyRotationView`, `KeyRevocationView`, `RevocationVoteView`, `RecoveryRequestView` (project-out from existing DHT entries)
  - `AccountView` (aggregate over the human's account state)
- New SQLite projection: `portal_hosts` table (`dht_anchor_hash` NOT NULL).
- New HTTP routes (registry-driven; doorway gets them free):
  - `GET /api/v1/account` → `AccountView`
  - `GET /api/v1/account/keys` → `KeyRotationView[]`
  - `GET /api/v1/account/revocations` → `KeyRevocationView[]`
  - `POST /api/v1/account/self-revocation` (M4 primitive)
  - `GET /api/v1/account/pending-recovery` → recovery requests where I am EC
  - `POST /api/v1/account/recovery/:id/vote`
  - `GET /api/v1/account/portal-hosts`
  - `POST /api/v1/account/portal-hosts`
  - `DELETE /api/v1/account/portal-hosts/:host_url_b64`
- `ReconcileController` extension: `on_portal_host` handler upserts the projection.

**doorway:**
- New custom-logic route: `GET /auth/portal-host` — returns the authenticated human's preferred reachable portal host URL.
- `/auth/exchange-session` response gains optional `portalHostUrl` field.
- `doorway-app/components/account/doorway-account.component.ts` — new "Manage from your steward →" section that fetches `/auth/portal-host` and renders a redirect link with a session token.

**elohim-agent:**
- New crate or module path: `elohim/elohim-agent/specialists/defender/`.
- `manifest.rs` — `DefenderManifest` declaration.
- `role_marker.rs` — local role marker check (in-process state, hydrated from manifest at startup).
- `detection.rs` — STUB: subscribes to `ReconcileController` events; logs only; no detection logic.
- `attestation.rs` — STUB: builds `anomaly_attestation` payload shape; returns canned "no anomaly" responses.
- Schemas under `specialists/defender/schemas/`.

**elohim-app:**
- **New top-level pillar `account/`** at `app/elohim-app/src/app/account/` (peer to imagodei, lamad, shefa, qahal):
  - `account.routes.ts` — top-level `/account/*`, lazy-loaded.
  - `index.ts` — public API barrel.
  - `services/` — `account.service.ts`, `portal-host.service.ts`, `portal-host-discovery.service.ts`, `revocation.service.ts`, `handoff.service.ts`.
  - `components/` — `account-shell/`, `security-signin-pane/`, plus placeholders for `personal-info-pane/`, `data-privacy-pane/`, `people-sharing-pane/`, `third-party-apps-pane/`.
  - `guards/account-guard.ts`.
- Cross-pillar import audit pre-flight (one-shot pass) — list any imports from imagodei into lamad/shefa/qahal that the graduation will need to resolve through `storage-client-ts`.

**Schemas (schema-first per `feedback_schema_first_ioc`):**
- `elohim/sdk/schemas/v1/views/portal-host-view.schema.json`
- `elohim/sdk/schemas/v1/views/agent-peer-binding-view.schema.json`
- `elohim/sdk/schemas/v1/views/key-rotation-view.schema.json`
- `elohim/sdk/schemas/v1/views/key-revocation-view.schema.json`
- `elohim/sdk/schemas/v1/views/revocation-vote-view.schema.json`
- `elohim/sdk/schemas/v1/views/recovery-request-view.schema.json`
- `elohim/sdk/schemas/v1/views/account-view.schema.json`
- `elohim/sdk/schemas/v1/zome-inputs/add-portal-host.schema.json`
- `elohim/sdk/schemas/v1/zome-inputs/submit-specialist-revocation.schema.json`
- `elohim/sdk/schemas/v1/agent/defender-manifest.schema.json`
- `elohim/sdk/schemas/v1/agent/anomaly-attestation.schema.json`

**Verification:**
- a2o features tagged `@recovery-m5` (see §14).
- Sweettest scenarios for `submit_specialist_revocation` and `add_portal_host`.
- vitest unit tests for new Angular services.
- Cypress wiring tests for the four Security & sign-in flows.

### 2.2 Out of scope (deferred)

- **Real defender detection** — no detection signals wired in M5. The defender's `detection.rs` stub subscribes and logs but emits no attestations. Deferred until elohim integration is concrete (M6+).
- **DHT-notarized `DefenderRole`** — Stage 3 evolution will reuse the existing `Attestation` entry type. No new entry type ever needed for defender role.
- **Mesh-rendered portal UX** — the architecture supports it (PortalHost discovery + composability boundary) but the actual peer-rendered UX flow is not implemented in M5.
- **Real session-handoff cryptography for browser path** — M5 uses doorway's existing `/auth/exchange-session` 60s tokens. Full libp2p-to-browser bridging is later.
- **Other pillars graduating to composability** — only `account/` pillar in M5; learning, wallet, profile-as-Surface-1 graduations are their own sprints.
- **Surface 2 (self-knowledge)** — psephos integration, journals, behavioral telemetry — separate sprint(s).
- **UI design polish, accessibility audit, design system** — separate Playwright-driven sprint.
- **Hosted-cell migration / browser session full lifecycle** — M6.
- **Hashcash / rate limiting on portal-host adds** — M6+.
- **Anti-lockout audit suite** — M6.
- **PortalHost reach semantics** (public vs. trusted vs. intimate) — entry shape carries the attribute, but M5 ships everything as `reach = trusted`.

---

## 3. Design Principles

1. **Schema-first IoC.** Every wire contract has a JSON schema before any Rust or TS implementation. Per `feedback_schema_first_ioc`.
2. **P2P-native classification.** Every data entity passed through `p2p-design-gate`. See §6.
3. **Composability boundary.** The new `account/` pillar imports only `storage-client-ts` and `@app/imagodei`. No imports from `lamad`, `shefa`, `qahal`. Enforced via ESLint config.
4. **Plumbing-first.** UI is minimum-viable scaffold. Per `project_m5_is_plumbing_sprint`.
5. **Stage 1 bootstrap social.** Coordinator gates use local checks; structural quorum from M4 is the meaningful enforcement. Stage 3 elohim-enforcement is later. Per `project_bootstrap_to_elohim_security_gradient`.
6. **HDI-validator constraints.** Validation callbacks use only deterministic primitives (`must_get_*`); cross-entity enforcement lives in coordinator pre-commit gates. Per `project_hdi_no_get_links_in_validators`.
7. **Operational state on libp2p, not DHT.** Per `project_dht_vs_libp2p_scoping` — the controller pattern (`project_principle_p1_reconciliation_controller`) handles operational state derived from notarized facts.
8. **Reconciliation cohort cohesion.** PortalHost lives in imagodei alongside `AgentPeerBinding`, `KeyRotation`, `KeyRevocation` — its operational-presence cohort. If imagodei splits later, the cohort migrates together. See §17.

---

## 4. Existing Primitives Inventory

### 4.1 Doorway (already built)

```
HTTP surface:
  /auth/{register, login, logout, refresh, me, account}
  /auth/{authorize, token}                        — OAuth dance
  /auth/native-handoff                            — identity bundle for native session
  /auth/{session-token, exchange-session}         — 60s cross-app transfer with is_steward + has_local_conductor flags
  /auth/{recover-custody, check-recovery-status, activate-recovery}
  /auth/{elohim-verify/start, elohim-verify/answer}
  /auth/{export-key, confirm-stewardship}
  /identity                                       — DID Document for federation

Auth modules:
  auth/jwt.rs, auth/password.rs, auth/api_key.rs, auth/permissions.rs
  db/schemas/oauth_session.rs                     — OAuthSessionDoc

App components:
  components/account/doorway-account.component.ts — hosted view + agency pipeline + graduation CTA
  components/login/threshold-login.component.ts
```

### 4.2 elohim-storage (already built — EPR 2B Batch A)

```
p2p/identity_handshake.rs        — /elohim/identity/handshake/1.0.0 protocol (Stage 1 verified)
p2p/identity_binding_gossip.rs   — gossipsub propagation
p2p/identity_map.rs              — HolochainBackedPeerIdentityMap
db/peer_identity_bindings.rs     — Category C projection (upsert + lookup_active)
reconcile/controller.rs          — ReconcileController with on_key_rotation, on_key_revocation,
                                   on_agent_peer_binding, on_revocation_attestation
reconcile/holochain_app_signal.rs — DNA signal stream from imagodei conductor
reconcile/pubkey_timeline.rs     — pubkey timeline cache
reconcile/sweep.rs               — eager revocation sweep
```

### 4.3 elohim-app (already built)

```
imagodei/services:
  auth.service.ts                  — JWT-based hosted human auth
  tauri-auth.service.ts            — Tauri OAuth flow: /auth/token → /auth/native-handoff → /auth/confirm-stewardship
  hosting-account.service.ts       — calls GET /auth/account
  identity.service.ts              — three-mode identity (session / hosted / network)
  session-migration.service.ts     — session-to-network migration state machine
  session-human.service.ts

imagodei/components/profile/sections — 9 sections (Surface 1)
generated/identity-handshake.ts    — TS bindings for the libp2p identity-handshake protocol
```

### 4.4 imagodei DNA (M3 + M4)

```
Entries: 28 (well below the ~100 ceiling)
  Identity: Human, HumanRelationship, ContributorPresence, Attestation, ...
  Recovery: KeyRotation, RecoveryRequest, HumanityWitness, IdentityChallenge, ChallengeSupport,
            IdentityAnomaly, IdentityFreeze, KeyStewardship
  Revocation (M4): KeyRevocation, RevocationVote
  P2P (Batch A): AgentPeerBinding

Coordinators (M3 + M4): create_recovery_request, commit_key_rotation, submit_intimate_witness,
                        create_self_revocation, create_revocation_request, submit_revocation_vote,
                        count_approved_revocation_votes, create_agent_peer_binding
```

---

## 5. Gaps Resolved in M5

| # | Gap | M5 Resolution |
|---|---|---|
| 1 | No browser-side handoff service | Resolved by graduation: account pillar is host-agnostic; same code path renders on tauri or browser-steward |
| 2 | storage-client-ts has no identity types | New view types (§7) + ts-rs export → storage-client-ts |
| 3 | No /account route in elohim-app | New top-level `account/` pillar |
| 4 | M4 revocation primitives invisible | Security & sign-in pane wires the four flows |
| 5 | No "you are a steward" awareness in elohim-app | New `portal-host-discovery.service.ts` reads peer_identity_bindings via storage |
| 6 | No "Manage from your steward" CTA in doorway-app | New section in `doorway-account.component` |
| 7 | Defender specialist scaffolding doesn't exist | New `specialists/defender/` module |
| 8 | submit_specialist_revocation coordinator missing | New coordinator function |
| 9 | No M5 a2o features | New `recovery-m5-*.feature` files |
| 10 | Schema gaps | New schema files (§7) |
| 11 | Browser-redirect OAuth-pattern session payload undefined | `/auth/exchange-session` extended with `portalHostUrl` |
| 12 | How doorway discovers steward's elohim-app URL | New `PortalHost` entry + `/auth/portal-host` route |

---

## 6. P2P Design Gate Output

### Entity: `PortalHost`

- **Classification:** Notarized (A) — new entry type in imagodei DNA.
- **Justification:** Other peers (doorways, peer-renderers, recovery contacts) must discover the human's portal host URL. Cannot be agent-scoped private (peers need to read). Cannot be operational (must be canonical, not reconstructable). imagodei DNA at 28/~100 has ample headroom; one new entry type is the right shape — cleaner than extending `Human`, supports per-host reach control and individual add/remove.
- **Content Address Strategy:** Agent-Scoped Composite — `(human_action_hash, host_url, added_at)`.
- **Address Justification:** Anchor on the `Human` entry's ActionHash (NOT the agent pub key) so portal hosts survive `KeyRotation`. Two humans declaring the same URL produce two distinct facts. Re-add after remove produces a new fact (added_at differentiates).
- **Source of Truth:** Holochain DHT.
- **Coordinator Zome:** `imagodei::add_portal_host`, `imagodei::remove_portal_host`, `imagodei::get_my_portal_hosts`.
- **Storage Projection:** `portal_hosts` table — `(rowid, human_id, host_url, label, added_at, last_reachable_at NULL, reach, dht_anchor_hash NOT NULL)`. Source-of-truth comment in migration.
- **HTTP Route:** `GET/POST/DELETE /api/v1/account/portal-hosts[/:host_url_b64]`.
- **Anti-Pattern Check:** No UUID-on-notarized; no CID-as-FK; canonical PK is `dht_anchor_hash`; URL is display alias.

### Entity: `DefenderRoleMarker`

- **Classification:** Operational (C) for M5 Stage 1 → Notarized (A) at Stage 3 reusing existing `Attestation` entry.
- **Justification:** At Stage 1, structural quorum gates from M4 are the meaningful enforcement. The role marker is a local elohim-agent assertion, reconstructable from startup manifest. Stage 3 evolution: the existing `Attestation` primitive carries `attestation_kind = "defender_role"`. **No new entry type ever needed for defender role.**
- **Content Address Strategy:** Slug/UUID (operational entity, no content to hash). Stage 3 inherits `Attestation`'s addressing.
- **Source of Truth:** elohim-agent in-process state (M5); `Attestation` DHT entry (Stage 3).
- **Coordinator Zome:** None for M5. Stage 3: existing `imagodei::create_attestation` with `attestation_kind = "defender_role"`.
- **Storage Projection:** None for M5. Stage 3: existing `attestations` projection.
- **HTTP Route:** None for M5.
- **Anti-Pattern Check:** Avoids "create new entry type when one already exists."

### Entity: Cross-pillar import boundaries

- **Classification:** N/A — not a data entity.
- **Disposition:** ESLint `eslint-plugin-boundaries` (or `@nx/enforce-module-boundaries`) at build time. Pre-flight audit pass surfaces existing imagodei pillar imports into other pillars.

### Design constraints discovered

1. `PortalHost` MUST anchor on `Human` ActionHash, not agent pub key — survives key rotation. Critical: caught by gate.
2. `PortalHost` reach attribute deferred. M5 ships `reach = trusted` (visible to emergency contacts + collective members). Public-vs-intimate semantics later, no DNA migration needed since the field is in the entry.
3. `last_reachable_at` is operational enrichment in the projection (sourced from libp2p `peer_identity_bindings` connectivity hints) — not part of the notarized entry. Marked clearly in `views.rs`.
4. Stage 3 defender migration locked: reuse `Attestation`. Documented in elohim-agent code comment for forward reference.
5. `Human` entry unchanged by M5. No migration burden on existing humans.
6. DNA capacity check passes: imagodei 28 → 29.
7. Reversal logged: pre-gate recommendation was "extend `Human` entry"; gate caught it. Spec uses separate `PortalHost` entry.

---

## 7. Wire Types & Schemas (the SDK/API deliverable)

All schemas live in `elohim/sdk/schemas/v1/`. Rust structs in `elohim-storage/src/views.rs` match the schema (validated by `tests/schema_contract.rs`). TypeScript bindings auto-generated via `cargo test export_bindings`.

### 7.1 New view types

```rust
// views.rs — output views (camelCase via #[serde(rename_all = "camelCase")])
PortalHostView { humanId, hostUrl, label, addedAt, lastReachableAt?, reach, dhtAnchorHash }
AgentPeerBindingView { agentCid, peerId, validFrom, validUntil?, signature, dhtAnchorHash }
KeyRotationView { humanId, oldPubKey, newPubKey, authority, rotatedAt, dhtAnchorHash }
KeyRevocationView { humanId, revokedPubKey, triggerType, attestation?, revokedAt, dhtAnchorHash }
RevocationVoteView { revocationRequestHash, voterAgentKey, decision, votedAt, dhtAnchorHash }
RecoveryRequestView { humanId, proposedAuthority, requestedAt, status, dhtAnchorHash }
AccountView {
  human: HumanView,                                    — existing
  activeKeyRotation: Option<KeyRotationView>,
  recentRevocations: Vec<KeyRevocationView>,
  pendingRecoveryRequests: Vec<RecoveryRequestView>,    — where I am EC
  emergencyContacts: Vec<HumanRelationshipView>,        — existing
  portalHosts: Vec<PortalHostView>,
  isSteward: bool,                                      — derived from peer_identity_bindings
  hasLocalConductor: bool,                              — derived
}
```

### 7.2 New input views

```rust
AddPortalHostInputView { hostUrl, label?, reach? }      — reach default "trusted"
SubmitSpecialistRevocationInputView { revokedPubKey, anomalyAttestation }
```

### 7.3 New libp2p / agent schemas

- `agent/defender-manifest.schema.json` — declares the defender's role, inputs (none real for M5), outputs (KeyRevocation with trigger_type=specialist_attestation), disclosure tier.
- `agent/anomaly-attestation.schema.json` — payload shape; M5 stub returns canned "no anomaly."

### 7.4 Schema contract test additions

- `elohim/elohim-storage/tests/schema_contract.rs` adds `verify_*_view` tests for each new view type.

---

## 8. DNA Changes (imagodei)

### 8.1 New entry type: `PortalHost`

```rust
// integrity zome
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PortalHost {
    pub human_action_hash: ActionHash,    // anchor — survives KeyRotation
    pub host_url: String,                  // e.g., "https://matthew.steward.example/account"
    pub label: Option<String>,             // e.g., "My main steward"
    pub added_at: Timestamp,
    pub reach: PortalHostReach,            // M5: always Trusted
}

pub enum PortalHostReach { Trusted, Intimate, Public }
```

### 8.2 Validation rules

- `host_url`: non-empty, parseable as `https://*` URL, max length 2048.
- `human_action_hash`: must reference an existing `Human` entry authored by the current agent (or the agent's steward chain).
- `added_at`: within ±5 minutes of validation time.
- `reach`: must be one of the enum variants (M5: only `Trusted` accepted).

Cross-entity check (must reference `Human`): coordinator pre-commit gate uses `must_get_action` on the referenced ActionHash. Validator only checks the must-get returns a `Human` entry kind. (Per `project_hdi_no_get_links_in_validators` — coordinator does the link traversal, validator only does deterministic shape checks.)

### 8.3 Links

- `Human → PortalHost` (link type `PortalHosts`) — created in coordinator on each `add_portal_host` call.

### 8.4 Coordinator functions

```rust
// coordinator zome
pub fn add_portal_host(input: AddPortalHostInput) -> ExternResult<ActionHash>
pub fn remove_portal_host(host_url: String) -> ExternResult<()>
pub fn get_my_portal_hosts() -> ExternResult<Vec<PortalHost>>
pub fn submit_specialist_revocation(input: SubmitSpecialistRevocationInput) -> ExternResult<ActionHash>
```

### 8.5 `submit_specialist_revocation` gate

The coordinator checks whether the calling agent is configured as a defender for the human whose key is being revoked. M5 implementation:

- Coordinator queries elohim-agent (via existing `gate-client` mechanism) for `is_defender_for(human_action_hash)`.
- elohim-agent answers from its local manifest (in-process state).
- If `false`: return `ExternResult::Err("not a configured defender")`.
- If `true`: commit a `KeyRevocation` with `trigger_type = "specialist_attestation"` and the supplied `anomaly_attestation` payload. Validator already accepts this trigger_type from M4.

### 8.6 Post-commit signals

```rust
DnaSignal::PortalHostCreated { entry, action_hash }
DnaSignal::PortalHostRemoved { action_hash }
```

### 8.7 ReconcileController dispatch additions

```rust
// elohim-storage/src/reconcile/controller.rs
impl<S: DnaSignalStream> ReconcileController<S> {
    async fn on_portal_host_created(&self, sig: PortalHostCreatedSignal) -> Result<(), ReconcileError>;
    async fn on_portal_host_removed(&self, sig: PortalHostRemovedSignal) -> Result<(), ReconcileError>;
}
```

Each handler upserts/deletes from the `portal_hosts` SQLite projection.

---

## 9. Storage Layer Changes

### 9.1 New SQLite migration

```sql
-- migrations/<timestamp>_create_portal_hosts.sql
-- Source of truth: Holochain DHT (PortalHost entry in imagodei DNA, Category A).
-- This table is a Category A projection rebuildable from signal replay.

CREATE TABLE portal_hosts (
    rowid               INTEGER PRIMARY KEY AUTOINCREMENT,
    human_id            TEXT NOT NULL,
    host_url            TEXT NOT NULL,
    label               TEXT,
    added_at            TEXT NOT NULL,
    last_reachable_at   TEXT,                   -- operational; sourced from peer_identity_bindings; NOT notarized
    reach               TEXT NOT NULL,
    dht_anchor_hash     TEXT NOT NULL UNIQUE
);

CREATE INDEX idx_portal_hosts_human_id ON portal_hosts(human_id);
CREATE INDEX idx_portal_hosts_dht_anchor ON portal_hosts(dht_anchor_hash);
```

### 9.2 New diesel model + CRUD

`db/portal_hosts.rs` — mirrors `db/peer_identity_bindings.rs` pattern: `upsert`, `delete_by_anchor_hash`, `list_for_human`, `lookup_by_url`.

### 9.3 New HTTP routes

Per the route registry pattern (`doorway/CLAUDE.md` — no per-domain proxy files in doorway). Routes added to `build_manifest()` in elohim-storage's `http.rs`:

```
GET    /api/v1/account                          → handle_get_account
GET    /api/v1/account/keys                     → handle_get_account_keys
GET    /api/v1/account/revocations              → handle_get_account_revocations
POST   /api/v1/account/self-revocation          → handle_post_self_revocation (M4 primitive)
GET    /api/v1/account/pending-recovery         → handle_get_pending_recovery
POST   /api/v1/account/recovery/:id/vote        → handle_post_recovery_vote
GET    /api/v1/account/portal-hosts             → handle_get_portal_hosts
POST   /api/v1/account/portal-hosts             → handle_post_portal_host
DELETE /api/v1/account/portal-hosts/:url_b64    → handle_delete_portal_host
```

`AccountView` aggregation: `handle_get_account` reads multiple projections (humans, key_rotations, key_revocations, recovery_requests, peer_identity_bindings, portal_hosts) and assembles. Single round-trip for the account-mgmt shell.

---

## 10. Doorway Layer Changes

### 10.1 New custom-logic route

```rust
// doorway-service/src/routes/auth_routes.rs
async fn handle_portal_host(req, state) -> Response<BoxBody>
// Validates JWT; queries elohim-storage GET /api/v1/account/portal-hosts;
// pings each host_url with HEAD/timeout-1s; returns the first reachable.
// If none reachable: returns 404 with shape { reachable: false }.
// If JWT invalid: 401.
// If no portal_hosts configured: 200 with { reachable: false, hosts: [] }.
```

Route: `GET /auth/portal-host` (added to `http.rs` match block).

### 10.2 `/auth/exchange-session` extension

```rust
// auth_routes.rs — ExchangeSessionResponse
pub struct ExchangeSessionResponse {
    pub token: String,
    pub human_id: String,
    pub agent_pub_key: String,
    pub identifier: String,
    pub expires_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")] pub doorway_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub doorway_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub portal_host_url: Option<String>,  // NEW
}
```

If the human has a reachable portal host, `portal_host_url` is populated; otherwise omitted.

### 10.3 doorway-app changes

`doorway-app/src/app/components/account/doorway-account.component.ts` — add a section:

```html
@if (portalHostUrl(); as hostUrl) {
  <section class="card portal-host">
    <h2>Manage from your steward</h2>
    <p>Your account is also reachable from your peer-native client.</p>
    <a [href]="hostUrl + '?session_token=' + sessionToken()"
       data-testid="portal-host-redirect">
      Manage from your steward →
    </a>
  </section>
}
```

`doorway-admin.service.ts` — adds `getPortalHostUrl()` and `mintSessionToken()` calls.

---

## 11. elohim-agent Layer Changes

### 11.1 New module path

```
elohim/elohim-agent/specialists/
  defender/
    Cargo.toml
    src/
      lib.rs              — public API: DefenderSpecialist::new(manifest), DefenderSpecialist::is_defender_for(human_action_hash)
      manifest.rs         — DefenderManifest struct + parser; matches schemas/defender-manifest.schema.json
      role_marker.rs      — local in-process state; hydrated from manifest at startup
      detection.rs        — STUB: subscribes to ReconcileController events; logs only
      attestation.rs      — STUB: builds anomaly_attestation payload; canned "no anomaly"
    schemas/
      defender-manifest.schema.json
      anomaly-attestation.schema.json
```

### 11.2 Manifest shape

```json
{
  "$schema": "https://schemas.elohim.protocol/v1/agent/defender-manifest.json",
  "specialist_kind": "defender",
  "for_humans": ["<human_action_hash_b64>"],
  "disclosure_tier": "trusted",
  "outputs": ["KeyRevocation::specialist_attestation"],
  "system_prompt_template": "you are the defender specialist for {{human_label}}; ..."
}
```

Per `project_elohim_subagent_specialists`: a specialist is a snapshot/fork of imagodei context + system-prompt wrapper. The manifest declares the wrapper.

### 11.3 Role marker check

```rust
// role_marker.rs
pub fn is_defender_for(human_action_hash: &ActionHash, manifest: &DefenderManifest) -> bool {
    manifest.for_humans.iter().any(|h| h == &human_action_hash.into_b64())
}
```

Called by the imagodei coordinator's `submit_specialist_revocation` gate via the existing `gate-client` mechanism.

### 11.4 Detection stub

```rust
// detection.rs
pub async fn run_detection_loop(controller_events: broadcast::Receiver<ReconcileEvent>) {
    while let Ok(event) = controller_events.recv().await {
        // STUB: M5 logs and emits zero attestations.
        // M6+: real detection logic walks the signal types and judges anomaly.
        tracing::debug!("defender observed: {:?}", event);
    }
}
```

### 11.5 Attestation stub

```rust
// attestation.rs
pub fn build_anomaly_attestation(_observation: &Observation) -> AnomalyAttestation {
    AnomalyAttestation {
        observed_at: now_iso(),
        anomaly_kind: "none".into(),  // M5 stub
        evidence: vec![],
        confidence: 0.0,
    }
}
```

---

## 12. elohim-app Layer Changes

### 12.1 New top-level pillar

```
app/elohim-app/src/app/account/
  account.routes.ts                  — /account/*, lazy-loaded
  index.ts                           — public API barrel
  models/
    account.model.ts                 — TS types (re-exports from storage-client)
    portal-host.model.ts
  services/
    account.service.ts               — wraps GET /api/v1/account
    portal-host.service.ts           — wraps PortalHost CRUD
    portal-host-discovery.service.ts — resolves "who renders my portal right now"
    revocation.service.ts            — wraps M4 primitives
    handoff.service.ts               — browser-side equivalent of tauri-auth.service
  components/
    account-shell/                   — pane-navigation layout
      account-shell.component.{ts,html,css}
    security-signin-pane/            — M4 flows (My keys, Self-revoke, Vote-as-EC, Lost-key)
      security-signin-pane.component.{ts,html,css}
      key-list/key-list.component.{ts,html}
      self-revoke/self-revoke.component.{ts,html}
      vote-as-ec/vote-as-ec.component.{ts,html}
      lost-key-entry/lost-key-entry.component.{ts,html}
    personal-info-pane/              — placeholder
    data-privacy-pane/               — placeholder
    people-sharing-pane/             — placeholder
    third-party-apps-pane/           — placeholder (lists doorways registered with)
  guards/
    account-guard.ts                 — auth + portal-host-discovery resolution
```

### 12.2 Composability boundary (ESLint config)

`.eslintrc` or flat config at the workspace level:

```js
{
  rules: {
    'boundaries/element-types': ['error', {
      default: 'allow',
      rules: [
        { from: 'account', allow: ['storage-client', 'imagodei', 'elohim'] },
      ],
    }],
  },
}
```

Pre-flight audit task: scan for existing `imagodei → lamad/shefa/qahal` imports; list violations to fix as part of M5.

### 12.3 Routing

```ts
// app.routes.ts
{ path: 'account', loadChildren: () => import('./account/account.routes').then(m => m.ACCOUNT_ROUTES) },
// existing /identity/* untouched
```

```ts
// account.routes.ts
export const ACCOUNT_ROUTES: Routes = [
  {
    path: '',
    canActivate: [accountGuard],
    component: AccountShellComponent,
    children: [
      { path: '', redirectTo: 'security', pathMatch: 'full' },
      { path: 'security', component: SecuritySigninPaneComponent },
      { path: 'personal-info', component: PersonalInfoPaneComponent },
      { path: 'data-privacy', component: DataPrivacyPaneComponent },
      { path: 'people-sharing', component: PeopleSharingPaneComponent },
      { path: 'third-party-apps', component: ThirdPartyAppsPaneComponent },
    ],
  },
];
```

### 12.4 Handoff service

```ts
// services/handoff.service.ts
@Injectable({ providedIn: 'root' })
export class HandoffService {
  // Browser-redirect path:
  //   1. Reads ?session_token=xxx from URL
  //   2. Calls doorway's /auth/exchange-session?session_token=xxx
  //   3. On 200: stores returned JWT via AuthService; mounts session
  //   4. On 401/404: redirects to /identity/login
  async consumeHandoffToken(token: string): Promise<HandoffResult>;
}
```

### 12.5 Portal-host discovery service

```ts
// services/portal-host-discovery.service.ts
@Injectable({ providedIn: 'root' })
export class PortalHostDiscoveryService {
  // Reads peer_identity_bindings via storage-client to determine isSteward.
  // Reads portal_hosts via storage-client.
  readonly isSteward = computed(() => /* from peer_identity_bindings */);
  readonly portalHosts = computed(() => /* from portal_hosts */);
  readonly preferredHost = computed(() => /* first reachable */);
}
```

---

## 13. Data Flow — OAuth-Pattern with Portal Host Discovery

```
[Browser] visits doorway.elohim.host/account
    │
    ▼
[doorway] /admin or /auth/account loads doorway-account.component
[doorway-app] hits GET /auth/portal-host
    │
    ├─ Has JWT + portal_hosts non-empty + first host reachable
    │      → response: { reachable: true, hostUrl: "https://matthew.steward.example/account" }
    │      → doorway-app renders "Manage from your steward →" link with mintSessionToken()
    │      → user clicks → navigates to {hostUrl}?session_token=xxx
    │           │
    │           ▼
    │      [Browser] loads {hostUrl}/account?session_token=xxx
    │      [elohim-app] account-guard runs:
    │           - HandoffService.consumeHandoffToken(token)
    │             → POST doorway/auth/exchange-session?session_token=xxx
    │             → returns full JWT + identity claims
    │           - AuthService.setAuth(jwt, claims)
    │           - PortalHostDiscoveryService confirms isSteward = true
    │      [account-shell] renders; default redirects to /account/security
    │      [security-signin-pane] AccountService.refresh() → AccountView
    │      User clicks "Revoke this key"
    │           → RevocationService.selfRevoke(pubKey)
    │           → POST /api/v1/account/self-revocation
    │           → storage calls imagodei coordinator commit_key_revocation(trigger_type=self)
    │           → DHT validates; post-commit signal
    │           → ReconcileController.on_key_revocation upserts projection + sweep
    │           → AccountService receives broadcast event; AccountView recomputes
    │           → UI reflects revocation
    │
    └─ No reachable host
           → response: { reachable: false, hosts: [] }
           → doorway-app renders existing hosted view (agency pipeline + graduation CTA)
```

Tauri host: same code path, mounted from `localhost:8090` directly. No doorway redirect involved. Account pillar reads `AccountView` from local elohim-storage sidecar.

---

## 14. Verification Harness

### 14.1 a2o features (`@recovery-m5`)

Files in `genesis/a2o/features/auth/recovery/`:

- `recovery-m5-list-my-keys.feature` — given AccountView, when user opens Security & sign-in, then they see active key + revocation history.
- `recovery-m5-self-revoke.feature` — given user with active key, when they self-revoke, then KeyRevocation entry is committed and AccountView reflects.
- `recovery-m5-vote-as-emergency-contact.feature` — given user is EC for human X with pending RevocationRequest, when user votes, then RevocationVote entry is committed.
- `recovery-m5-lost-key-entry.feature` — given user enters "I lost my key" flow, then they are routed to recovery-vs-revocation branch based on their current state.
- `recovery-m5-doorway-handoff-to-steward.feature` — given user has steward presence + reachable portal host, when they visit doorway/account, then they see "Manage from your steward →" link; when they click, then they land at portal host URL with session bootstrapped.
- `recovery-m5-portal-host-discovery.feature` — given user adds a portal host, then storage projection updates and `/auth/portal-host` returns the host.
- `recovery-m5-defender-role-gate.feature` — given calling agent has no defender role marker, when they call `submit_specialist_revocation`, then coordinator rejects; when role marker present, accepts.

### 14.2 Sweettest scenarios

`elohim/holochain/tests/sweettest/`:
- `submit_specialist_revocation_happy_path.rs` — defender role marker present → KeyRevocation committed.
- `submit_specialist_revocation_no_role.rs` — defender role marker absent → coordinator returns Err.
- `add_portal_host_happy_path.rs` — entry committed, link from Human created.
- `remove_portal_host_happy_path.rs` — entry deletion, link removed.

### 14.3 vitest unit tests

`app/elohim-app/src/app/account/services/`:
- `account.service.spec.ts`
- `portal-host.service.spec.ts`
- `portal-host-discovery.service.spec.ts`
- `revocation.service.spec.ts`
- `handoff.service.spec.ts`

### 14.4 Cypress wiring tests

`app/elohim-app/cypress/e2e/account-m5/`:
- `security-pane-renders.cy.ts`
- `self-revoke-flow.cy.ts`
- `vote-as-ec-flow.cy.ts`
- `lost-key-entry-flow.cy.ts`
- `handoff-from-doorway.cy.ts`

### 14.5 UI scaffold expectations

- All new components have `data-testid` attributes per `page-model` skill.
- No design polish, no a11y review, no responsive breakpoint work.
- Bare-minimum Angular templates with explicit "[M5 scaffold — Playwright sprint will polish]" comments.

---

## 15. Backward Compatibility / Migration

- **DNA:** New `PortalHost` entry type — additive. Existing `Human` entries unchanged. No migration of existing data.
- **DNA coordinator:** `submit_specialist_revocation` is new. Existing M4 coordinators (`create_self_revocation`, `submit_revocation_vote`, etc.) unchanged.
- **Storage schema:** New `portal_hosts` table via migration. Existing tables unchanged.
- **Storage views:** New view types only; existing types unchanged.
- **Storage HTTP routes:** New routes under `/api/v1/account/*`. Existing routes unchanged.
- **Doorway:** New `/auth/portal-host` route. `/auth/exchange-session` response gains optional field (additive). Existing routes unchanged.
- **elohim-app:** New `/account` pillar alongside existing `/identity/*`. Routing additive; no displacement of `/identity/profile`, `/identity/login`, etc.
- **storage-client-ts:** New generated types; pre-push hook validates codegen freshness.

---

## 16. Out-of-Scope Explicit Punts (rationale)

| Item | Why deferred |
|---|---|
| Real defender detection | Elohim integration not concrete; M5 ships scaffolding so M6+ can flesh out without re-designing structure. |
| Mesh-rendered portal UX | Architecture supports it (PortalHost discovery + composability); UX flow has its own design questions deserving dedicated brainstorm. |
| DHT-notarized DefenderRole | Stage 3 work; reuses existing `Attestation`, locked in spec to prevent regression. |
| Surface 2 (self-knowledge) | Requires psephos integration, journal capture, behavioral telemetry — separate sprint(s) per pillar. |
| Other pillar graduations | Each pillar has its own composability work; M5 ships canonical example via `account/`. |
| UI polish + a11y + design system | Separate Playwright-driven sprint per `project_m5_is_plumbing_sprint`. |
| Hosted-cell migration / browser session full lifecycle | M6. |
| Hashcash on portal-host adds | M6+. |
| Anti-lockout audit suite | M6. |
| PortalHost reach semantics (Public/Intimate distinction) | Entry shape carries reach attribute; M5 ships `Trusted`-only; future work refines without DNA migration. |

---

## 17. Future Considerations

### 17.1 Stage 3 defender role evolution

When elohim integration is concrete and constitutional governance is wired, the defender role marker graduates from local-only (Operational, Category C) to DHT-notarized (Notarized, Category A) — but **without creating a new entry type**. The existing `Attestation` primitive carries `attestation_kind = "defender_role"`; qahal governance authors the attestation; the imagodei coordinator's gate switches from "ask elohim-agent local manifest" to "verify Attestation exists with valid disclosure tier."

This is a code-level change to the gate, not a DNA change. Recorded in elohim-agent code comment for forward reference.

### 17.2 Mesh-rendered portal UX

The PortalHost entry already supports multi-host. Future UX:
- Add discovery: "your steward is unreachable — these trusted peers can render your portal." Pulled from `HumanRelationship` with `emergency_access_enabled = true`.
- Peer-rendered portal is the same Angular `account/` pillar code; the rendering peer fetches AccountView via storage-client; secrets flow through socially-derived primitives (per `project_socially_derived_security`) — peer sees UI, not secrets.
- Recovery flow integrates: when a human has lost their devices, an emergency contact opens their elohim-app and renders the lost-key-entry flow on behalf of the human.

### 17.3 Future imagodei DNA split

If imagodei grows toward its 100-entry-type ceiling, a natural cleave is:
- **imagodei** keeps "who you are" — Human, profile, relationships, persona attestations.
- **account / infra** absorbs operational presence — `KeyRotation`, `KeyRevocation`, `RevocationVote`, `AgentPeerBinding`, `PortalHost`, sessions, doorway registrations.
- **mishpat** continues governance and constitutional disclosure.

`PortalHost` migrates with its operational-presence cohort. The cohort moves together because they share the `ReconcileController` reconciliation pattern. One refactor, one cohort.

### 17.4 Other pillars graduating

`profile/` (Surface 1), `learn/` (lamad), `wallet/` (shefa), `community/` (qahal) — each pillar follows the `account/` pattern:
- Clean public API in barrel.
- Cross-pillar deps through `storage-client-ts`.
- Lazy-loaded routes.
- No imports from sibling pillars.

The graduation is incremental. M5 ships the canonical example; future sprints replicate the pattern.

---

## 18. References

### 18.1 Memories (canonical)

- `project_principle_p1_reconciliation_controller`
- `project_epr2b_recovery_m4_convergence`
- `project_imagodei_three_surfaces`
- `project_peer_native_account_canonical_surface`
- `project_m5_reframe_auth_portal_convergence`
- `project_m5_is_plumbing_sprint`
- `project_elohim_app_as_composable_view_federation`
- `project_elohim_subagent_specialists`
- `project_elohim_as_counsel`
- `project_bootstrap_to_elohim_security_gradient`
- `project_socially_derived_security`
- `project_recovery_grandma_standard`
- `project_graduated_recovery_authority`
- `project_dht_vs_libp2p_scoping`
- `project_three_layer_truth_model`
- `project_hdi_no_get_links_in_validators`
- `project_household_is_resilience_unit`
- `feedback_schema_first_ioc`
- `feedback_subagent_scope_guardrails`
- `feedback_swarm_composition_fresh_tree_build`
- `feedback_session_orchestrate_vs_implement`
- `feedback_shift_measure_jenkins`
- `feedback_less_pushy_notifications`
- `feedback_dev_branch_no_pr`

### 18.2 Specs

- Phase 2 revised: `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md`
- M3: `genesis/docs/superpowers/plans/2026-04-22-recovery-protocol-phase-2-m3-graduated-key-rotation.md`
- M4: `genesis/docs/superpowers/specs/2026-04-24-recovery-protocol-phase-2-m4-fast-path-revocation-design.md`
- EPR Phase 2B Batch A: merged at `79181f8e` on dev

### 18.3 Plans

- M5 kickoff prompt: `genesis/docs/plans/2026-04-24-recovery-m5-elohim-defender-and-revocation-ux-kickoff-prompt.md`

### 18.4 Schemas (to be created in M5)

- `elohim/sdk/schemas/v1/views/portal-host-view.schema.json`
- `elohim/sdk/schemas/v1/views/agent-peer-binding-view.schema.json`
- `elohim/sdk/schemas/v1/views/key-rotation-view.schema.json`
- `elohim/sdk/schemas/v1/views/key-revocation-view.schema.json`
- `elohim/sdk/schemas/v1/views/revocation-vote-view.schema.json`
- `elohim/sdk/schemas/v1/views/recovery-request-view.schema.json`
- `elohim/sdk/schemas/v1/views/account-view.schema.json`
- `elohim/sdk/schemas/v1/zome-inputs/add-portal-host.schema.json`
- `elohim/sdk/schemas/v1/zome-inputs/submit-specialist-revocation.schema.json`
- `elohim/sdk/schemas/v1/agent/defender-manifest.schema.json`
- `elohim/sdk/schemas/v1/agent/anomaly-attestation.schema.json`
