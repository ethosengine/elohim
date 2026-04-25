# Recovery Protocol Phase 2 — M5: Auth Portal Convergence + Revocation UX + Stub Defender — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the existing recovery primitives + EPR 2B Batch A identity substrate into a clearly-defined SDK/API surface, exposed through a new graduated `account/` pillar in elohim-app — making M4 revocation primitives human-visible and connecting hosted-doorway with peer-native-steward auth portals.

**Architecture:** Schema-first JSON contracts → imagodei DNA additions (1 new entry type, 4 new coordinators) → elohim-storage views/projections/HTTP routes → doorway routing extensions → elohim-agent defender stub → new elohim-app `account/` pillar with composability boundary → a2o feature verification.

**Tech Stack:** Rust (Holochain HDI/HDK 0.6, Diesel, Hyper, libp2p 0.54), Angular 19 (standalone components, signals, lazy-loaded routes), TypeScript (storage-client-ts auto-generated), Cucumber/Gherkin (a2o), Cypress (e2e), Vitest (unit), Sweettest (zome integration).

**Spec:** `genesis/docs/superpowers/specs/2026-04-25-recovery-protocol-phase-2-m5-auth-portal-convergence-revocation-ux-and-stub-defender-design.md`

**Working branch:** `feature/recovery-m5-auth-portal-and-revocation-ux` (cut from dev at session start).

---

## Pre-flight conventions (apply to every task)

### Scope guardrails for subagent dispatch (per memory `feedback_subagent_scope_guardrails`)

Every subagent dispatch prompt MUST include verbatim:

> **Scope guardrails:**
> - You may ONLY modify files listed in this task's "Files" section.
> - You may NOT run `git revert`, `git reset`, `git checkout --`, or any destructive operation on commits authored by anyone other than yourself in THIS task.
> - You may NOT modify files outside this task's listed paths even to "fix" perceived issues — if you find a problem in out-of-scope code, write it in your BLOCKED report and stop.
> - If a build/test failure appears unrelated to your task's changes, file a BLOCKED report. Do NOT attempt to fix unrelated code.
> - Report the SHA range you authored at end (orchestrator verifies).

Orchestrator post-dispatch action: scan `git log <pre-dispatch-SHA>..HEAD` and verify no out-of-scope commits.

### Build commands (per memory `feedback_shift_measure_jenkins` — Eclipse Che lacks Holochain env, so sweettest + e2e run in Jenkins; local builds are still possible for fmt/clippy/build but tests escalate to CI)

```bash
# imagodei DNA (integrity + coordinator zomes)
cd /projects/elohim/elohim/holochain/dna/imagodei && just check && just pack

# elohim-storage (Rust, includes views, projection, HTTP routes, ReconcileController)
cd /projects/elohim/elohim/elohim-storage \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release

# elohim-storage tests (unit + schema contract + ts-rs export)
cd /projects/elohim/elohim/elohim-storage \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
cd /projects/elohim/elohim/elohim-storage \
  && cargo test export_bindings    # Regenerate TS types

# Schema validation
pnpm run schema:test
pnpm run schema:validate
pnpm run schema:codegen:ts        # Verifies codegen freshness

# Doorway service
cd /projects/elohim/doorway/doorway-service \
  && RUSTFLAGS="" cargo build --release \
  && RUSTFLAGS="" cargo test --lib --bins \
  && RUSTFLAGS="" cargo clippy -- -D warnings \
  && cargo fmt --check

# Doorway app
cd /projects/elohim/doorway/doorway-app && pnpm install && pnpm run build && pnpm exec eslint src --ext .ts,.html

# elohim-agent service
cd /projects/elohim/elohim/elohim-agent/elohim-agent-service \
  && RUSTFLAGS="" cargo build \
  && RUSTFLAGS="" cargo test

# elohim-app
cd /projects/elohim/app/elohim-app && pnpm install && pnpm run build && pnpm test && pnpm run lint

# Sweettest (CI only — Eclipse Che lacks the nix env; local subagents skip and let CI verify)
cd /projects/elohim/elohim/holochain/tests/sweettest \
  && CARGO_TARGET_DIR=target/native-tests cargo test
```

### Fresh-tree mandate (memory `feedback_swarm_composition_fresh_tree_build`)

The `ReconcileController` extension (Task 9) and post-commit signal additions (Task 5) are swarm-adjacent. Before committing those tasks: run `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release` from a clean checkout in the elohim-storage crate to verify the swarm composition still resolves.

### Husky pre-push (memory `feedback_session_orchestrate_vs_implement` + general principle)

Pre-push runs `cargo fmt + clippy + tests`. Run locally BEFORE `git push`. Pre-push gate currently takes ~25–30 minutes on cold cache. Plan around it.

### Branch hygiene

All commits land on `feature/recovery-m5-auth-portal-and-revocation-ux`. Local merge to `dev` happens only at end of the sprint per memory `feedback_dev_branch_no_pr`. No PR cycle for batch landings.

---

## File structure overview

### New files (created by M5)

```
elohim/sdk/schemas/v1/
  views/
    portal-host-view.schema.json
    agent-peer-binding-view.schema.json
    key-rotation-view.schema.json
    key-revocation-view.schema.json
    revocation-vote-view.schema.json
    recovery-request-view.schema.json
    account-view.schema.json
  zome-inputs/
    add-portal-host.schema.json
    submit-specialist-revocation.schema.json
  agent/
    defender-manifest.schema.json
    anomaly-attestation.schema.json

elohim/holochain/dna/imagodei/zomes/
  imagodei-integrity/src/
    portal_host.rs                              (NEW)
  imagodei-coordinator/src/
    portal_host.rs                              (NEW)
    submit_specialist_revocation.rs             (NEW)

elohim/elohim-storage/src/
  db/portal_hosts.rs                            (NEW)
  migrations/<timestamp>_create_portal_hosts/
    up.sql                                      (NEW)
    down.sql                                    (NEW)
  reconcile/portal_host_handlers.rs             (NEW)

elohim/elohim-agent/specialists/defender/      (NEW directory)
  Cargo.toml
  src/
    lib.rs
    manifest.rs
    role_marker.rs
    detection.rs
    attestation.rs

doorway/doorway-service/src/auth/
  portal_host.rs                                (NEW — handler for /auth/portal-host)

app/elohim-app/src/app/account/                 (NEW pillar directory)
  account.routes.ts
  index.ts
  models/account.model.ts
  models/portal-host.model.ts
  services/account.service.ts
  services/portal-host.service.ts
  services/portal-host-discovery.service.ts
  services/revocation.service.ts
  services/handoff.service.ts
  components/account-shell/
  components/security-signin-pane/
  components/security-signin-pane/key-list/
  components/security-signin-pane/self-revoke/
  components/security-signin-pane/vote-as-ec/
  components/security-signin-pane/lost-key-entry/
  components/personal-info-pane/                (placeholder)
  components/data-privacy-pane/                 (placeholder)
  components/people-sharing-pane/               (placeholder)
  components/third-party-apps-pane/             (placeholder)
  guards/account-guard.ts

genesis/a2o/features/auth/recovery/
  recovery-m5-list-my-keys.feature              (NEW)
  recovery-m5-self-revoke.feature               (NEW)
  recovery-m5-vote-as-emergency-contact.feature (NEW)
  recovery-m5-lost-key-entry.feature            (NEW)
  recovery-m5-doorway-handoff-to-steward.feature (NEW)
  recovery-m5-portal-host-discovery.feature     (NEW)
  recovery-m5-defender-role-gate.feature        (NEW)

app/elohim-app/cypress/e2e/account-m5/          (NEW)
  security-pane-renders.cy.ts
  self-revoke-flow.cy.ts
  vote-as-ec-flow.cy.ts
  lost-key-entry-flow.cy.ts
  handoff-from-doorway.cy.ts
```

### Modified files

```
elohim/holochain/dna/imagodei/zomes/imagodei-integrity/src/
  lib.rs                              (register PortalHost entry type + link type)

elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/
  lib.rs                              (export portal_host coordinator + submit_specialist_revocation)
  signal.rs                           (add PortalHostCreated / PortalHostRemoved variants)

elohim/elohim-storage/src/
  views.rs                            (add 7 new View types + 2 InputView types)
  http.rs                             (add 9 /api/v1/account/* route handlers + register in build_manifest)
  reconcile/controller.rs             (dispatch on_portal_host_created / on_portal_host_removed)
  reconcile/signal_stream.rs          (add PortalHostCreatedSignal / PortalHostRemovedSignal)
  db/mod.rs                           (re-export portal_hosts module)
  db/diesel_schema.rs                 (add portal_hosts table generated)
  db/models.rs                        (add NewPortalHostRow / PortalHostRow)

elohim/elohim-storage/tests/
  schema_contract.rs                  (add verify_*_view tests for new view types)

elohim/sdk/schemas/scripts/
  codegen-ts.mjs                      (add new view interfaces to INTERFACE_FILES)

doorway/doorway-service/src/
  routes/auth_routes.rs               (add handle_portal_host + extend ExchangeSessionResponse)
  server/http.rs                      (route /auth/portal-host)

doorway/doorway-app/src/app/
  components/account/doorway-account.component.ts
                                      (add Manage-from-steward section)
  services/doorway-admin.service.ts   (add getPortalHostUrl + mintSessionToken methods)
  models/doorway.model.ts             (add PortalHostResponse type)

elohim/elohim-agent/elohim-agent-service/src/
  lib.rs                              (export specialists::defender)
  service.rs                          (wire defender role-marker check into gate-client handler)

app/elohim-app/src/app/
  app.routes.ts                       (add /account lazy-loaded route)

app/elohim-app/
  eslint.config.mjs OR .eslintrc      (add boundaries plugin config for account pillar)
```

---

## Task 1: Author all M5 JSON schemas (schema-first IoC)

**Why first:** memory `feedback_schema_first_ioc` mandates schemas BEFORE Rust/TS implementation. Every wire contract has a JSON schema first. The Rust structs in later tasks must match these schemas; `tests/schema_contract.rs` validates at `cargo test` time.

**Files:**
- Create: `elohim/sdk/schemas/v1/views/portal-host-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/agent-peer-binding-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/key-rotation-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/key-revocation-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/revocation-vote-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/recovery-request-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/account-view.schema.json`
- Create: `elohim/sdk/schemas/v1/zome-inputs/add-portal-host.schema.json`
- Create: `elohim/sdk/schemas/v1/zome-inputs/submit-specialist-revocation.schema.json`
- Create: `elohim/sdk/schemas/v1/agent/defender-manifest.schema.json`
- Create: `elohim/sdk/schemas/v1/agent/anomaly-attestation.schema.json`

- [ ] **Step 1: Write `portal-host-view.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/views/portal-host-view.json",
  "title": "PortalHostView",
  "type": "object",
  "additionalProperties": false,
  "required": ["humanId", "hostUrl", "addedAt", "reach", "dhtAnchorHash"],
  "properties": {
    "humanId":          { "type": "string", "description": "ActionHash (base64url) of the Human entry this PortalHost anchors on" },
    "hostUrl":          { "type": "string", "format": "uri", "maxLength": 2048 },
    "label":            { "type": ["string", "null"], "maxLength": 256 },
    "addedAt":          { "type": "string", "format": "date-time" },
    "lastReachableAt":  { "type": ["string", "null"], "format": "date-time", "description": "Operational enrichment from libp2p; not part of notarized entry" },
    "reach":            { "type": "string", "enum": ["trusted", "intimate", "public"], "description": "M5 ships only 'trusted'" },
    "dhtAnchorHash":    { "type": "string", "description": "ActionHash (base64url) of the PortalHost entry; canonical PK" }
  }
}
```

- [ ] **Step 2: Write `agent-peer-binding-view.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/views/agent-peer-binding-view.json",
  "title": "AgentPeerBindingView",
  "type": "object",
  "additionalProperties": false,
  "required": ["agentCid", "peerId", "validFrom", "signature", "dhtAnchorHash"],
  "properties": {
    "agentCid":      { "type": "string" },
    "peerId":        { "type": "string", "description": "libp2p PeerId" },
    "validFrom":     { "type": "string", "format": "date-time" },
    "validUntil":    { "type": ["string", "null"], "format": "date-time" },
    "signature":     { "type": "string", "description": "Ed25519 signature over canonical bytes" },
    "dhtAnchorHash": { "type": "string" }
  }
}
```

- [ ] **Step 3: Write `key-rotation-view.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/views/key-rotation-view.json",
  "title": "KeyRotationView",
  "type": "object",
  "additionalProperties": false,
  "required": ["humanId", "newPubKey", "authority", "rotatedAt", "dhtAnchorHash"],
  "properties": {
    "humanId":       { "type": "string" },
    "oldPubKey":     { "type": ["string", "null"] },
    "newPubKey":     { "type": "string" },
    "authority":     { "type": "string", "enum": ["IntimateQuorum", "CryptographicQuorum", "CommunityConsensus", "GovernanceAct", "NetworkWitness"] },
    "rotatedAt":     { "type": "string", "format": "date-time" },
    "dhtAnchorHash": { "type": "string" }
  }
}
```

- [ ] **Step 4: Write `key-revocation-view.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/views/key-revocation-view.json",
  "title": "KeyRevocationView",
  "type": "object",
  "additionalProperties": false,
  "required": ["humanId", "revokedPubKey", "triggerType", "revokedAt", "dhtAnchorHash"],
  "properties": {
    "humanId":       { "type": "string" },
    "revokedPubKey": { "type": "string" },
    "triggerType":   { "type": "string", "enum": ["self", "emergency_contact_quorum", "specialist_attestation"] },
    "attestation":   { "type": ["object", "null"], "description": "anomaly_attestation payload when triggerType is specialist_attestation" },
    "revokedAt":     { "type": "string", "format": "date-time" },
    "dhtAnchorHash": { "type": "string" }
  }
}
```

- [ ] **Step 5: Write `revocation-vote-view.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/views/revocation-vote-view.json",
  "title": "RevocationVoteView",
  "type": "object",
  "additionalProperties": false,
  "required": ["revocationRequestHash", "voterAgentKey", "decision", "votedAt", "dhtAnchorHash"],
  "properties": {
    "revocationRequestHash": { "type": "string" },
    "voterAgentKey":         { "type": "string" },
    "decision":              { "type": "string", "enum": ["approve", "reject"] },
    "votedAt":               { "type": "string", "format": "date-time" },
    "dhtAnchorHash":         { "type": "string" }
  }
}
```

- [ ] **Step 6: Write `recovery-request-view.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/views/recovery-request-view.json",
  "title": "RecoveryRequestView",
  "type": "object",
  "additionalProperties": false,
  "required": ["humanId", "proposedAuthority", "requestedAt", "status", "dhtAnchorHash"],
  "properties": {
    "humanId":           { "type": "string" },
    "proposedAuthority": { "type": "string", "enum": ["IntimateQuorum", "CryptographicQuorum", "CommunityConsensus", "GovernanceAct", "NetworkWitness"] },
    "requestedAt":       { "type": "string", "format": "date-time" },
    "status":            { "type": "string", "enum": ["pending", "approved", "rejected", "expired"] },
    "dhtAnchorHash":     { "type": "string" }
  }
}
```

- [ ] **Step 7: Write `account-view.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/views/account-view.json",
  "title": "AccountView",
  "type": "object",
  "additionalProperties": false,
  "required": ["human", "isSteward", "hasLocalConductor", "portalHosts", "emergencyContacts", "recentRevocations", "pendingRecoveryRequests"],
  "properties": {
    "human":                   { "$ref": "https://schemas.elohim.protocol/v1/views/human-view.json" },
    "activeKeyRotation":       { "anyOf": [{ "$ref": "https://schemas.elohim.protocol/v1/views/key-rotation-view.json" }, { "type": "null" }] },
    "recentRevocations":       { "type": "array", "items": { "$ref": "https://schemas.elohim.protocol/v1/views/key-revocation-view.json" } },
    "pendingRecoveryRequests": { "type": "array", "items": { "$ref": "https://schemas.elohim.protocol/v1/views/recovery-request-view.json" } },
    "emergencyContacts":       { "type": "array", "items": { "$ref": "https://schemas.elohim.protocol/v1/views/human-relationship-view.json" } },
    "portalHosts":             { "type": "array", "items": { "$ref": "https://schemas.elohim.protocol/v1/views/portal-host-view.json" } },
    "isSteward":               { "type": "boolean", "description": "Derived from peer_identity_bindings projection" },
    "hasLocalConductor":       { "type": "boolean", "description": "Derived" }
  }
}
```

- [ ] **Step 8: Write `add-portal-host.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/zome-inputs/add-portal-host.json",
  "title": "AddPortalHostInput",
  "type": "object",
  "additionalProperties": false,
  "required": ["hostUrl"],
  "properties": {
    "hostUrl": { "type": "string", "format": "uri", "maxLength": 2048 },
    "label":   { "type": ["string", "null"], "maxLength": 256 },
    "reach":   { "type": "string", "enum": ["trusted", "intimate", "public"], "default": "trusted", "description": "M5 only accepts 'trusted'" }
  }
}
```

- [ ] **Step 9: Write `submit-specialist-revocation.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/zome-inputs/submit-specialist-revocation.json",
  "title": "SubmitSpecialistRevocationInput",
  "type": "object",
  "additionalProperties": false,
  "required": ["humanActionHash", "revokedPubKey", "anomalyAttestation"],
  "properties": {
    "humanActionHash":    { "type": "string", "description": "ActionHash (base64url) of the target Human" },
    "revokedPubKey":      { "type": "string" },
    "anomalyAttestation": { "$ref": "https://schemas.elohim.protocol/v1/agent/anomaly-attestation.json" }
  }
}
```

- [ ] **Step 10: Write `defender-manifest.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/agent/defender-manifest.json",
  "title": "DefenderManifest",
  "type": "object",
  "additionalProperties": false,
  "required": ["specialistKind", "forHumans", "disclosureTier", "outputs", "systemPromptTemplate"],
  "properties": {
    "specialistKind":       { "type": "string", "const": "defender" },
    "forHumans":            { "type": "array", "items": { "type": "string", "description": "Human ActionHash (base64url)" }, "minItems": 1 },
    "disclosureTier":       { "type": "string", "enum": ["intimate", "trusted", "public"] },
    "outputs":              { "type": "array", "items": { "type": "string" }, "description": "DHT entry types this specialist may author (e.g., 'KeyRevocation::specialist_attestation')" },
    "systemPromptTemplate": { "type": "string" }
  }
}
```

- [ ] **Step 11: Write `anomaly-attestation.schema.json`**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.elohim.protocol/v1/agent/anomaly-attestation.json",
  "title": "AnomalyAttestation",
  "type": "object",
  "additionalProperties": false,
  "required": ["observedAt", "anomalyKind", "evidence", "confidence"],
  "properties": {
    "observedAt":  { "type": "string", "format": "date-time" },
    "anomalyKind": { "type": "string", "enum": ["none", "rapid_rotation", "quorum_evasion", "geographic_anomaly", "vulnerability_heuristic"] },
    "evidence":    { "type": "array", "items": { "type": "object" } },
    "confidence":  { "type": "number", "minimum": 0.0, "maximum": 1.0 }
  }
}
```

- [ ] **Step 12: Run schema self-tests**

```bash
cd /projects/elohim && pnpm run schema:test
```

Expected: PASS (24 existing assertions remain green; new schemas don't break existing).

- [ ] **Step 13: Commit**

```bash
git add elohim/sdk/schemas/v1/views/portal-host-view.schema.json \
        elohim/sdk/schemas/v1/views/agent-peer-binding-view.schema.json \
        elohim/sdk/schemas/v1/views/key-rotation-view.schema.json \
        elohim/sdk/schemas/v1/views/key-revocation-view.schema.json \
        elohim/sdk/schemas/v1/views/revocation-vote-view.schema.json \
        elohim/sdk/schemas/v1/views/recovery-request-view.schema.json \
        elohim/sdk/schemas/v1/views/account-view.schema.json \
        elohim/sdk/schemas/v1/zome-inputs/add-portal-host.schema.json \
        elohim/sdk/schemas/v1/zome-inputs/submit-specialist-revocation.schema.json \
        elohim/sdk/schemas/v1/agent/defender-manifest.schema.json \
        elohim/sdk/schemas/v1/agent/anomaly-attestation.schema.json
git commit -m "schemas(m5): wire contracts for PortalHost, account view, defender stub"
```

---

## Task 2: imagodei integrity zome — PortalHost entry type + link type

**Why second:** DHT primitives must exist before coordinators can call them.

**Files:**
- Create: `elohim/holochain/dna/imagodei/zomes/imagodei-integrity/src/portal_host.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei-integrity/src/lib.rs`

- [ ] **Step 1: Write the PortalHost integrity entry**

Create `elohim/holochain/dna/imagodei/zomes/imagodei-integrity/src/portal_host.rs`:

```rust
//! PortalHost — declares URLs authorized to render this human's auth portal.
//!
//! Category A (Notarized). Anchored on the Human entry's ActionHash so portal
//! hosts survive KeyRotation. M5 ships only `Trusted` reach.
//!
//! See: genesis/docs/superpowers/specs/2026-04-25-recovery-protocol-phase-2-m5-...md §6, §8

use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PortalHost {
    pub human_action_hash: ActionHash,
    pub host_url: String,
    pub label: Option<String>,
    pub added_at: Timestamp,
    pub reach: PortalHostReach,
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq, Eq)]
pub enum PortalHostReach {
    Trusted,
    Intimate,
    Public,
}

/// Validate a PortalHost entry. Per `project_hdi_no_get_links_in_validators`,
/// this validator only does deterministic shape checks; cross-entity link
/// traversal (e.g. confirming the link from Human exists) lives in the
/// coordinator pre-commit gate.
pub fn validate_create_portal_host(
    action: EntryCreationAction,
    portal_host: PortalHost,
) -> ExternResult<ValidateCallbackResult> {
    // Non-empty URL
    if portal_host.host_url.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("host_url must not be empty".into()));
    }
    // Length cap
    if portal_host.host_url.len() > 2048 {
        return Ok(ValidateCallbackResult::Invalid("host_url too long (>2048)".into()));
    }
    // HTTPS-only structural check (full URL parsing is host-side work)
    if !portal_host.host_url.starts_with("https://") {
        return Ok(ValidateCallbackResult::Invalid("host_url must be https://".into()));
    }
    // Label length cap
    if let Some(ref label) = portal_host.label {
        if label.len() > 256 {
            return Ok(ValidateCallbackResult::Invalid("label too long (>256)".into()));
        }
    }
    // M5: only Trusted reach
    if !matches!(portal_host.reach, PortalHostReach::Trusted) {
        return Ok(ValidateCallbackResult::Invalid("M5 ships reach=Trusted only".into()));
    }
    // added_at within ±5 minutes of action timestamp
    let action_ts = action.timestamp().as_seconds_and_nanos().0;
    let entry_ts = portal_host.added_at.as_seconds_and_nanos().0;
    if (action_ts - entry_ts).abs() > 300 {
        return Ok(ValidateCallbackResult::Invalid("added_at skew >5min from action timestamp".into()));
    }
    // human_action_hash must reference an existing entry; type check via must_get_entry
    must_get_entry(EntryHash::from(portal_host.human_action_hash.clone()))?;
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_portal_host(
    _action: Update,
    _new: PortalHost,
    _old_action: EntryCreationAction,
    _old: PortalHost,
) -> ExternResult<ValidateCallbackResult> {
    // PortalHost is immutable — to change a host, delete + re-create
    Ok(ValidateCallbackResult::Invalid("PortalHost is immutable; delete + re-create instead".into()))
}

pub fn validate_delete_portal_host(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original: PortalHost,
) -> ExternResult<ValidateCallbackResult> {
    // Only the original author can delete (enforced by Holochain core)
    Ok(ValidateCallbackResult::Valid)
}
```

- [ ] **Step 2: Register the entry type and link type in `lib.rs`**

Modify `elohim/holochain/dna/imagodei/zomes/imagodei-integrity/src/lib.rs`. Locate the existing `EntryTypes` enum (it has 28 variants today after EPR 2B Batch A added `AgentPeerBinding`); add:

```rust
// In EntryTypes:
#[entry_def(visibility = "public")]
PortalHost(PortalHost),

// In LinkTypes:
PortalHosts,                            // Human → PortalHost
```

In the `validate` function's match block, add:

```rust
EntryTypes::PortalHost(portal_host) => {
    portal_host::validate_create_portal_host(action, portal_host)
}
```

For Update + Delete branches, route to the corresponding validate functions.

For the new link type validation:

```rust
LinkTypes::PortalHosts => {
    // base must be a Human entry; target must be a PortalHost
    // Per project_hdi_no_get_links_in_validators, do shape check only:
    // detailed cross-entity is in coordinator pre-commit
    Ok(ValidateCallbackResult::Valid)
}
```

Add `mod portal_host;` at the top.

- [ ] **Step 3: Build the integrity zome to verify compilation**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```

Expected: PASS (no clippy/fmt errors).

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei-integrity/src/portal_host.rs \
        elohim/holochain/dna/imagodei/zomes/imagodei-integrity/src/lib.rs
git commit -m "imagodei(integrity): PortalHost entry type + PortalHosts link type"
```

---

## Task 3: imagodei coordinator — portal-host CRUD (add/remove/get_my)

**Files:**
- Create: `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/portal_host.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/lib.rs`

- [ ] **Step 1: Write the coordinator module**

Create `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/portal_host.rs`:

```rust
//! Coordinator functions for PortalHost CRUD.
//!
//! Pre-commit gate (Category A2 derived discipline per
//! `project_hdi_no_get_links_in_validators`): coordinator verifies the
//! human_action_hash references a Human entry authored by the calling agent
//! before committing. Validator only checks deterministic shape.

use hdk::prelude::*;
use imagodei_integrity::{
    EntryTypes, Human, LinkTypes, PortalHost, PortalHostReach,
};

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddPortalHostInput {
    pub host_url: String,
    pub label: Option<String>,
    pub reach: Option<PortalHostReach>,    // default Trusted
}

/// Add a portal host for the calling agent's Human.
#[hdk_extern]
pub fn add_portal_host(input: AddPortalHostInput) -> ExternResult<ActionHash> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;

    // Locate this agent's Human entry. Convention: each agent links from their
    // pubkey to their Human via LinkTypes::AgentToHuman (existing).
    let human_links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToHuman)?.build(),
    )?;
    let human_link = human_links.first().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest("No Human entry for this agent".into()))
    })?;
    let human_action_hash = ActionHash::from(human_link.target.clone());

    // Confirm the linked entry is actually a Human authored by this agent
    let record = must_get_valid_record(human_action_hash.clone())?;
    let _human: Human = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("decode Human: {e}"))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Linked entry is not a Human".into())))?;
    if record.action().author() != &agent_info()?.agent_initial_pubkey {
        return Err(wasm_error!(WasmErrorInner::Guest("Human is not authored by caller".into())));
    }

    // Build and commit the PortalHost entry
    let entry = PortalHost {
        human_action_hash: human_action_hash.clone(),
        host_url: input.host_url,
        label: input.label,
        added_at: sys_time()?,
        reach: input.reach.unwrap_or(PortalHostReach::Trusted),
    };
    let action_hash = create_entry(EntryTypes::PortalHost(entry.clone()))?;

    // Create the link from Human → PortalHost
    create_link(
        human_action_hash,
        action_hash.clone(),
        LinkTypes::PortalHosts,
        (),
    )?;

    Ok(action_hash)
}

/// Remove a portal host by URL match. Removes ALL matching entries (multi-add semantics).
#[hdk_extern]
pub fn remove_portal_host(host_url: String) -> ExternResult<()> {
    let portal_hosts = get_my_portal_hosts(())?;
    for (action_hash, ph) in portal_hosts {
        if ph.host_url == host_url {
            delete_entry(action_hash)?;
        }
    }
    Ok(())
}

/// List portal hosts for the calling agent's Human.
#[hdk_extern]
pub fn get_my_portal_hosts(_: ()) -> ExternResult<Vec<(ActionHash, PortalHost)>> {
    let agent_pub_key = agent_info()?.agent_initial_pubkey;
    let human_links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key, LinkTypes::AgentToHuman)?.build(),
    )?;
    let human_link = human_links.first().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest("No Human entry for this agent".into()))
    })?;
    let human_action_hash = ActionHash::from(human_link.target.clone());

    let portal_links = get_links(
        GetLinksInputBuilder::try_new(human_action_hash, LinkTypes::PortalHosts)?.build(),
    )?;
    let mut out = Vec::with_capacity(portal_links.len());
    for link in portal_links {
        let action_hash = ActionHash::from(link.target);
        if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
            if let Ok(Some(ph)) = record.entry().to_app_option::<PortalHost>() {
                out.push((action_hash, ph));
            }
        }
    }
    Ok(out)
}
```

- [ ] **Step 2: Register the coordinator module**

Modify `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/lib.rs`:

```rust
pub mod portal_host;
pub use portal_host::*;
```

- [ ] **Step 3: Build the coordinator zome**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/portal_host.rs \
        elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/lib.rs
git commit -m "imagodei(coordinator): add/remove/get_my portal_host functions"
```

---

## Task 4: imagodei coordinator — submit_specialist_revocation

**Files:**
- Create: `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/submit_specialist_revocation.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/lib.rs`

- [ ] **Step 1: Write the coordinator function**

Create `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/submit_specialist_revocation.rs`:

```rust
//! submit_specialist_revocation — defender producer for the
//! `trigger_type = "specialist_attestation"` revocation path.
//!
//! Stage 1 (M5) gate: defender role marker is local elohim-agent state, queried
//! via the existing gate-client mechanism. Stage 3 will reuse the existing
//! Attestation primitive (no new entry type) — see spec §17.1.

use hdk::prelude::*;
use imagodei_integrity::{
    EntryTypes, KeyRevocation, KeyRevocationTrigger, Human,
};

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSpecialistRevocationInput {
    pub human_action_hash: ActionHash,
    pub revoked_pub_key: AgentPubKey,
    pub anomaly_attestation: serde_json::Value,    // matches anomaly-attestation.schema.json
}

/// Stage 1 gate: ask the local elohim-agent (via gate-client) whether the
/// calling agent is configured as a defender for this human. The gate-client
/// abstraction is in the elohim-agent crate; for M5, the gate returns from
/// in-process state hydrated from manifest at startup.
fn caller_is_defender_for(human_action_hash: &ActionHash) -> ExternResult<bool> {
    // gate-client call: kind = "is_defender_for", payload = human_action_hash
    // Falls through to elohim-agent which reads its DefenderManifest.
    let gate_input = serde_json::json!({
        "kind": "is_defender_for",
        "humanActionHash": ActionHashB64::from(human_action_hash.clone()).to_string(),
    });
    let result: serde_json::Value = call_remote(
        agent_info()?.agent_initial_pubkey,
        ZomeName::from("gate_client"),
        FunctionName::from("ask_gate"),
        None,
        gate_input,
    )
    .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("gate call failed: {e}"))))?
    .decode()
    .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("gate decode: {e}"))))?;

    Ok(result.get("isDefender").and_then(|v| v.as_bool()).unwrap_or(false))
}

#[hdk_extern]
pub fn submit_specialist_revocation(
    input: SubmitSpecialistRevocationInput,
) -> ExternResult<ActionHash> {
    // Confirm the target is a Human
    let _record = must_get_valid_record(input.human_action_hash.clone())?;

    // Stage 1 gate
    if !caller_is_defender_for(&input.human_action_hash)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "caller is not a configured defender for this human".into()
        )));
    }

    // Build KeyRevocation with trigger_type = SpecialistAttestation
    let entry = KeyRevocation {
        human_action_hash: input.human_action_hash,
        revoked_pub_key: input.revoked_pub_key,
        trigger_type: KeyRevocationTrigger::SpecialistAttestation {
            anomaly_attestation: input.anomaly_attestation,
        },
        revoked_at: sys_time()?,
    };
    create_entry(EntryTypes::KeyRevocation(entry))
}
```

- [ ] **Step 2: Register the coordinator module**

Append to `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/lib.rs`:

```rust
pub mod submit_specialist_revocation;
pub use submit_specialist_revocation::*;
```

- [ ] **Step 3: Build the coordinator zome**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/submit_specialist_revocation.rs \
        elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/lib.rs
git commit -m "imagodei(coordinator): submit_specialist_revocation defender producer"
```

---

## Task 5: imagodei post-commit signals + DNA pack

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/signal.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/lib.rs` (post_commit dispatch)

- [ ] **Step 1: Add signal variants**

In `signal.rs`, locate the existing `RecoveryV2Signal` enum (six variants from M3+M4) and add:

```rust
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RecoveryV2Signal {
    // ... existing variants (KeyRotationCommitted, RecoveryRequestCreated, ...)

    PortalHostCreated {
        action_hash: ActionHashB64,
        human_id: ActionHashB64,
        host_url: String,
        label: Option<String>,
        added_at: Timestamp,
    },
    PortalHostRemoved {
        action_hash: ActionHashB64,
        human_id: ActionHashB64,
        host_url: String,
    },
}
```

- [ ] **Step 2: Wire post-commit dispatch**

In `lib.rs`'s existing `post_commit` block, add match arms for `EntryTypes::PortalHost` (create) and the corresponding delete actions, emitting the new signal variants.

- [ ] **Step 3: Pack the DNA**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check && just pack
```

Expected: PASS; new `imagodei.dna` artifact produced.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/signal.rs \
        elohim/holochain/dna/imagodei/zomes/imagodei-coordinator/src/lib.rs
git commit -m "imagodei(signal): PortalHostCreated/Removed RecoveryV2Signal variants"
```

---

## Task 6: Sweettest scenarios for PortalHost CRUD + submit_specialist_revocation

**Files:**
- Create: `elohim/holochain/tests/sweettest/tests/portal_host_crud.rs`
- Create: `elohim/holochain/tests/sweettest/tests/submit_specialist_revocation.rs`

> **Note:** Sweettest runs in CI per memory `feedback_shift_measure_jenkins`. Local Eclipse Che lacks the nix env. Subagents writing this task should commit the code; CI runs verify.

- [ ] **Step 1: Write portal-host happy-path test**

Create `elohim/holochain/tests/sweettest/tests/portal_host_crud.rs`:

```rust
use holochain::sweettest::*;
use imagodei_coordinator::portal_host::AddPortalHostInput;
use imagodei_integrity::{PortalHost, PortalHostReach};

#[tokio::test(flavor = "multi_thread")]
async fn portal_host_add_remove_roundtrip() {
    let mut conductor = SweetConductor::from_standard_config().await;
    let agent = SweetAgents::one(conductor.keystore()).await;
    let dna = SweetDnaFile::from_bundle(std::path::Path::new(
        "../../dna/imagodei/imagodei.dna",
    ))
    .await
    .unwrap();
    let app = conductor
        .setup_app_for_agent("imagodei-test", agent.clone(), &[&dna])
        .await
        .unwrap();
    let (cell,) = app.into_tuple();

    // Pre-req: create a Human (existing coordinator)
    let _human_hash: holo_hash::ActionHash = conductor
        .call(&cell.zome("imagodei_coordinator"), "create_human", ())
        .await;

    // Add portal host
    let add_input = AddPortalHostInput {
        host_url: "https://matthew.steward.example/account".into(),
        label: Some("Main steward".into()),
        reach: Some(PortalHostReach::Trusted),
    };
    let action_hash: holo_hash::ActionHash = conductor
        .call(
            &cell.zome("imagodei_coordinator"),
            "add_portal_host",
            add_input,
        )
        .await;
    assert!(!action_hash.get_raw_39().is_empty());

    // Get my portal hosts → should contain the one we just added
    let hosts: Vec<(holo_hash::ActionHash, PortalHost)> = conductor
        .call(&cell.zome("imagodei_coordinator"), "get_my_portal_hosts", ())
        .await;
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].1.host_url, "https://matthew.steward.example/account");

    // Remove
    conductor
        .call::<_, ()>(
            &cell.zome("imagodei_coordinator"),
            "remove_portal_host",
            "https://matthew.steward.example/account".to_string(),
        )
        .await;

    // Empty after remove
    let hosts_after: Vec<(holo_hash::ActionHash, PortalHost)> = conductor
        .call(&cell.zome("imagodei_coordinator"), "get_my_portal_hosts", ())
        .await;
    assert!(hosts_after.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn portal_host_rejects_non_https() {
    let mut conductor = SweetConductor::from_standard_config().await;
    let agent = SweetAgents::one(conductor.keystore()).await;
    let dna = SweetDnaFile::from_bundle(std::path::Path::new(
        "../../dna/imagodei/imagodei.dna",
    ))
    .await
    .unwrap();
    let app = conductor
        .setup_app_for_agent("imagodei-test-nonhttps", agent, &[&dna])
        .await
        .unwrap();
    let (cell,) = app.into_tuple();

    let _human_hash: holo_hash::ActionHash = conductor
        .call(&cell.zome("imagodei_coordinator"), "create_human", ())
        .await;

    let add_input = AddPortalHostInput {
        host_url: "http://insecure.example/account".into(),
        label: None,
        reach: Some(PortalHostReach::Trusted),
    };
    let result: Result<holo_hash::ActionHash, _> = conductor
        .call_fallible(
            &cell.zome("imagodei_coordinator"),
            "add_portal_host",
            add_input,
        )
        .await;
    assert!(result.is_err(), "expected validator rejection of http://");
}
```

- [ ] **Step 2: Write submit_specialist_revocation tests**

Create `elohim/holochain/tests/sweettest/tests/submit_specialist_revocation.rs`:

```rust
use holochain::sweettest::*;
use imagodei_coordinator::submit_specialist_revocation::SubmitSpecialistRevocationInput;

#[tokio::test(flavor = "multi_thread")]
async fn submit_specialist_revocation_rejects_without_role_marker() {
    // No defender manifest configured → gate returns false → coordinator rejects.
    let mut conductor = SweetConductor::from_standard_config().await;
    let agent = SweetAgents::one(conductor.keystore()).await;
    let dna = SweetDnaFile::from_bundle(std::path::Path::new(
        "../../dna/imagodei/imagodei.dna",
    ))
    .await
    .unwrap();
    let app = conductor
        .setup_app_for_agent("imagodei-test-noxrole", agent, &[&dna])
        .await
        .unwrap();
    let (cell,) = app.into_tuple();

    let human_hash: holo_hash::ActionHash = conductor
        .call(&cell.zome("imagodei_coordinator"), "create_human", ())
        .await;

    let input = SubmitSpecialistRevocationInput {
        human_action_hash: human_hash,
        revoked_pub_key: agent_info_for_test(&conductor, &cell).await,
        anomaly_attestation: serde_json::json!({
            "observedAt": "2026-04-25T12:00:00Z",
            "anomalyKind": "rapid_rotation",
            "evidence": [],
            "confidence": 0.7
        }),
    };
    let result: Result<holo_hash::ActionHash, _> = conductor
        .call_fallible(
            &cell.zome("imagodei_coordinator"),
            "submit_specialist_revocation",
            input,
        )
        .await;
    assert!(result.is_err(), "expected role-marker rejection");
}

// Happy-path test requires a configured defender manifest. M5 ships the test
// scaffolding; full happy-path verification lands when the elohim-agent fixture
// is wired (see Task 14).

async fn agent_info_for_test(
    _conductor: &SweetConductor,
    cell: &SweetCell,
) -> holochain::prelude::AgentPubKey {
    cell.agent_pubkey().clone()
}
```

- [ ] **Step 3: Defer `cargo test` to CI**

Sweettest does not run on Eclipse Che. The dispatch reports "test code authored; CI runs verification."

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/tests/sweettest/tests/portal_host_crud.rs \
        elohim/holochain/tests/sweettest/tests/submit_specialist_revocation.rs
git commit -m "tests(sweettest): M5 portal-host CRUD + specialist-revocation gate"
```

---

## Task 7: elohim-storage view types (camelCase boundary)

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

> **Plan amendment (Task 1 corrected schemas):** `KeyRotationView`, `KeyRevocationView`, `RevocationVoteView`, `RecoveryRequestView`, `HumanView`, `HumanRelationshipView`, `RecoveryWitnessView` **already exist** in `views.rs` and have matching schemas at `elohim/sdk/schemas/v1/views/key-rotation.schema.json` etc. Task 1 deleted the duplicate `*-view.schema.json` files it had created. M5 only adds the **3 truly new view types**: `PortalHostView`, `AgentPeerBindingView`, `AccountView`. Plus 2 input views: `AddPortalHostInputView`, `SubmitSpecialistRevocationInputView`. Skip the old types in step 1 below — they're already in the file.

- [ ] **Step 1: Add 3 NEW output view structs**

Append to `views.rs` (preserve existing types — including the existing KeyRotationView/KeyRevocationView/RevocationVoteView/RecoveryRequestView — untouched):

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PortalHostView {
    pub human_id: String,
    pub host_url: String,
    pub label: Option<String>,
    pub added_at: String,
    pub last_reachable_at: Option<String>,    // operational, NOT in notarized entry
    pub reach: String,
    pub dht_anchor_hash: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AgentPeerBindingView {
    pub agent_cid: String,
    pub peer_id: String,
    pub valid_from: String,
    pub valid_until: Option<String>,
    pub signature: String,
    pub dht_anchor_hash: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct KeyRotationView {
    pub human_id: String,
    pub old_pub_key: Option<String>,
    pub new_pub_key: String,
    pub authority: String,
    pub rotated_at: String,
    pub dht_anchor_hash: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct KeyRevocationView {
    pub human_id: String,
    pub revoked_pub_key: String,
    pub trigger_type: String,
    pub attestation: Option<Value>,
    pub revoked_at: String,
    pub dht_anchor_hash: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RevocationVoteView {
    pub revocation_request_hash: String,
    pub voter_agent_key: String,
    pub decision: String,
    pub voted_at: String,
    pub dht_anchor_hash: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecoveryRequestView {
    pub human_id: String,
    pub proposed_authority: String,
    pub requested_at: String,
    pub status: String,
    pub dht_anchor_hash: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccountView {
    pub human: HumanView,
    pub active_key_rotation: Option<KeyRotationView>,
    pub recent_revocations: Vec<KeyRevocationView>,
    pub pending_recovery_requests: Vec<RecoveryRequestView>,
    pub emergency_contacts: Vec<HumanRelationshipView>,
    pub portal_hosts: Vec<PortalHostView>,
    pub is_steward: bool,
    pub has_local_conductor: bool,
}
```

- [ ] **Step 2: Add 2 input view structs**

```rust
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AddPortalHostInputView {
    pub host_url: String,
    pub label: Option<String>,
    pub reach: Option<String>,    // default "trusted"
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SubmitSpecialistRevocationInputView {
    pub human_action_hash: String,
    pub revoked_pub_key: String,
    pub anomaly_attestation: Value,
}
```

- [ ] **Step 3: Build to verify type-checks**

```bash
cd /projects/elohim/elohim/elohim-storage \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "storage(views): M5 view + input types — PortalHost/AccountView/KeyRotation/KeyRevocation/RevocationVote/RecoveryRequest/AgentPeerBinding"
```

---

## Task 8: elohim-storage SQLite migration + diesel CRUD for portal_hosts

**Files:**
- Create: `elohim/elohim-storage/migrations/<TIMESTAMP>_create_portal_hosts/up.sql`
- Create: `elohim/elohim-storage/migrations/<TIMESTAMP>_create_portal_hosts/down.sql`
- Create: `elohim/elohim-storage/src/db/portal_hosts.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`

> Replace `<TIMESTAMP>` with `diesel migration generate create_portal_hosts` output (ISO format like `2026-04-25-123456`).

- [ ] **Step 1: Generate the migration**

```bash
cd /projects/elohim/elohim/elohim-storage && diesel migration generate create_portal_hosts
```

This creates `migrations/<TIMESTAMP>_create_portal_hosts/{up,down}.sql`.

- [ ] **Step 2: Write up.sql**

```sql
-- Source of truth: Holochain DHT (PortalHost entry in imagodei DNA, Category A).
-- This table is a Category A projection rebuildable from signal replay.

CREATE TABLE portal_hosts (
    rowid               INTEGER PRIMARY KEY AUTOINCREMENT,
    human_id            TEXT NOT NULL,
    host_url            TEXT NOT NULL,
    label               TEXT,
    added_at            TEXT NOT NULL,
    last_reachable_at   TEXT,
    reach               TEXT NOT NULL,
    dht_anchor_hash     TEXT NOT NULL UNIQUE
);

CREATE INDEX idx_portal_hosts_human_id ON portal_hosts(human_id);
CREATE INDEX idx_portal_hosts_dht_anchor ON portal_hosts(dht_anchor_hash);
```

- [ ] **Step 3: Write down.sql**

```sql
DROP INDEX IF EXISTS idx_portal_hosts_dht_anchor;
DROP INDEX IF EXISTS idx_portal_hosts_human_id;
DROP TABLE IF EXISTS portal_hosts;
```

- [ ] **Step 4: Run the migration to update diesel_schema.rs**

```bash
cd /projects/elohim/elohim/elohim-storage && diesel migration run && diesel print-schema > src/db/diesel_schema.rs
```

This regenerates `diesel_schema.rs` with the new `portal_hosts` table macro.

- [ ] **Step 5: Add diesel models in models.rs**

Append to `elohim/elohim-storage/src/db/models.rs`:

```rust
#[derive(Debug, Clone, Queryable, Identifiable)]
#[diesel(table_name = portal_hosts, primary_key(rowid))]
pub struct PortalHostRow {
    pub rowid: i32,
    pub human_id: String,
    pub host_url: String,
    pub label: Option<String>,
    pub added_at: String,
    pub last_reachable_at: Option<String>,
    pub reach: String,
    pub dht_anchor_hash: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = portal_hosts)]
pub struct NewPortalHostRow {
    pub human_id: String,
    pub host_url: String,
    pub label: Option<String>,
    pub added_at: String,
    pub last_reachable_at: Option<String>,
    pub reach: String,
    pub dht_anchor_hash: String,
}
```

- [ ] **Step 6: Write CRUD module**

Create `elohim/elohim-storage/src/db/portal_hosts.rs`:

```rust
//! CRUD for the `portal_hosts` table.
//!
//! Source of truth: Holochain DHT (imagodei PortalHost entry — M5).
//! This table is a Category A projection rebuildable from signal replay.

use crate::db::diesel_schema::portal_hosts;
use crate::db::models::{NewPortalHostRow, PortalHostRow};
use crate::error::StorageError;
use diesel::prelude::*;

pub fn upsert(
    conn: &mut SqliteConnection,
    row: &NewPortalHostRow,
) -> Result<(), StorageError> {
    diesel::replace_into(portal_hosts::table)
        .values(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("portal_hosts upsert: {e}")))
}

pub fn delete_by_anchor_hash(
    conn: &mut SqliteConnection,
    anchor: &str,
) -> Result<usize, StorageError> {
    use portal_hosts::dsl;
    diesel::delete(dsl::portal_hosts.filter(dsl::dht_anchor_hash.eq(anchor)))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("portal_hosts delete: {e}")))
}

pub fn list_for_human(
    conn: &mut SqliteConnection,
    human_id: &str,
) -> Result<Vec<PortalHostRow>, StorageError> {
    use portal_hosts::dsl;
    dsl::portal_hosts
        .filter(dsl::human_id.eq(human_id))
        .order(dsl::added_at.desc())
        .load::<PortalHostRow>(conn)
        .map_err(|e| StorageError::Database(format!("portal_hosts list: {e}")))
}

pub fn update_last_reachable(
    conn: &mut SqliteConnection,
    anchor: &str,
    timestamp_iso: &str,
) -> Result<(), StorageError> {
    use portal_hosts::dsl;
    diesel::update(dsl::portal_hosts.filter(dsl::dht_anchor_hash.eq(anchor)))
        .set(dsl::last_reachable_at.eq(timestamp_iso))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("portal_hosts update_last_reachable: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder().build(manager).unwrap();
        run_migrations(&pool).unwrap();
        pool
    }

    #[test]
    fn upsert_and_list_roundtrip() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let row = NewPortalHostRow {
            human_id: "uhCEkAbC".into(),
            host_url: "https://m.example/account".into(),
            label: Some("main".into()),
            added_at: "2026-04-25T12:00:00Z".into(),
            last_reachable_at: None,
            reach: "trusted".into(),
            dht_anchor_hash: "uhCkAbC123".into(),
        };
        upsert(&mut conn, &row).unwrap();
        let rows = list_for_human(&mut conn, "uhCEkAbC").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].host_url, "https://m.example/account");
    }
}
```

- [ ] **Step 7: Re-export from db/mod.rs**

Append to `elohim/elohim-storage/src/db/mod.rs`:

```rust
pub mod portal_hosts;
```

- [ ] **Step 8: Run unit tests**

```bash
cd /projects/elohim/elohim/elohim-storage \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib portal_hosts
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-storage/migrations/*_create_portal_hosts \
        elohim/elohim-storage/src/db/portal_hosts.rs \
        elohim/elohim-storage/src/db/mod.rs \
        elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/src/db/models.rs
git commit -m "storage(db): portal_hosts table + CRUD module"
```

---

## Task 9: elohim-storage ReconcileController extension (swarm-adjacent — fresh-tree mandate)

**Files:**
- Create: `elohim/elohim-storage/src/reconcile/portal_host_handlers.rs`
- Modify: `elohim/elohim-storage/src/reconcile/controller.rs`
- Modify: `elohim/elohim-storage/src/reconcile/signal_stream.rs`
- Modify: `elohim/elohim-storage/src/reconcile/mod.rs`

> **Fresh-tree mandate (memory `feedback_swarm_composition_fresh_tree_build`):** The ReconcileController is swarm-adjacent. Before committing this task, run a fresh full build to verify swarm composition still resolves.

- [ ] **Step 1: Add signal types**

In `signal_stream.rs`, add (after existing `KeyRotationSignal`, `KeyRevocationSignal`, etc.):

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalHostCreatedSignal {
    pub action_hash: String,
    pub human_id: String,
    pub host_url: String,
    pub label: Option<String>,
    pub added_at: String,
    pub reach: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalHostRemovedSignal {
    pub action_hash: String,
    pub human_id: String,
    pub host_url: String,
}
```

Add the variants to the `DnaSignal` enum:

```rust
#[serde(rename_all = "camelCase", tag = "type")]
pub enum DnaSignal {
    // ... existing variants
    PortalHostCreated(PortalHostCreatedSignal),
    PortalHostRemoved(PortalHostRemovedSignal),
}
```

- [ ] **Step 2: Write the handler module**

Create `elohim/elohim-storage/src/reconcile/portal_host_handlers.rs`:

```rust
//! Handlers for PortalHost signals — upsert/delete the projection.

use crate::db::{portal_hosts, DbPool};
use crate::db::models::NewPortalHostRow;
use crate::reconcile::signal_stream::{PortalHostCreatedSignal, PortalHostRemovedSignal};
use crate::reconcile::ReconcileError;
use std::sync::Arc;

pub async fn on_portal_host_created(
    db_pool: Arc<DbPool>,
    sig: PortalHostCreatedSignal,
) -> Result<(), ReconcileError> {
    let mut conn = db_pool
        .get()
        .map_err(|e| ReconcileError::Pool(format!("{e}")))?;
    let row = NewPortalHostRow {
        human_id: sig.human_id,
        host_url: sig.host_url,
        label: sig.label,
        added_at: sig.added_at,
        last_reachable_at: None,
        reach: sig.reach,
        dht_anchor_hash: sig.action_hash,
    };
    portal_hosts::upsert(&mut conn, &row).map_err(|e| ReconcileError::Sweep(e.to_string()))
}

pub async fn on_portal_host_removed(
    db_pool: Arc<DbPool>,
    sig: PortalHostRemovedSignal,
) -> Result<(), ReconcileError> {
    let mut conn = db_pool
        .get()
        .map_err(|e| ReconcileError::Pool(format!("{e}")))?;
    portal_hosts::delete_by_anchor_hash(&mut conn, &sig.action_hash)
        .map_err(|e| ReconcileError::Sweep(e.to_string()))?;
    Ok(())
}
```

- [ ] **Step 3: Wire dispatch in controller.rs**

In `reconcile/controller.rs`'s `dispatch` method, add match arms:

```rust
DnaSignal::PortalHostCreated(sig) => {
    portal_host_handlers::on_portal_host_created(self.db_pool.clone(), sig).await
}
DnaSignal::PortalHostRemoved(sig) => {
    portal_host_handlers::on_portal_host_removed(self.db_pool.clone(), sig).await
}
```

- [ ] **Step 4: Re-export from reconcile/mod.rs**

```rust
pub mod portal_host_handlers;
```

- [ ] **Step 5: Run a fresh-tree full build (mandatory for swarm-adjacent edits)**

```bash
cd /projects/elohim/elohim/elohim-storage && cargo clean && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: PASS. (Slow — first build is cold cache; budget ~10 min.)

- [ ] **Step 6: Run tests**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/reconcile/portal_host_handlers.rs \
        elohim/elohim-storage/src/reconcile/controller.rs \
        elohim/elohim-storage/src/reconcile/signal_stream.rs \
        elohim/elohim-storage/src/reconcile/mod.rs
git commit -m "storage(reconcile): PortalHost signal dispatch — fresh-tree verified"
```

---

## Task 10: elohim-storage HTTP routes (`/api/v1/account/*`)

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Add 9 route handlers**

Append to `http.rs`:

```rust
async fn handle_get_account(state: AppState, _req: Request<Body>) -> Response<Body> {
    // 1) Resolve calling agent's Human via existing zome call
    // 2) Aggregate AccountView from projections:
    //    - human → existing humans table
    //    - active_key_rotation → key_rotations table (latest)
    //    - recent_revocations → key_revocations table (last 10)
    //    - pending_recovery_requests → recovery_requests table (where I am EC)
    //    - emergency_contacts → human_relationships (emergency_access_enabled = true)
    //    - portal_hosts → portal_hosts table
    //    - is_steward → peer_identity_bindings table (any binding with my agent_pub_key)
    //    - has_local_conductor → from request header / state config
    // 3) Return AccountView as JSON
    todo!("M5 — see spec §9.3")
}

async fn handle_get_account_keys(state: AppState, _req: Request<Body>) -> Response<Body> {
    // SELECT * FROM key_rotations WHERE human_id = ? ORDER BY rotated_at DESC
    todo!()
}

async fn handle_get_account_revocations(state: AppState, _req: Request<Body>) -> Response<Body> {
    // SELECT * FROM key_revocations WHERE human_id = ? ORDER BY revoked_at DESC LIMIT 50
    todo!()
}

async fn handle_post_self_revocation(state: AppState, req: Request<Body>) -> Response<Body> {
    // Forwards to imagodei coordinator create_self_revocation (existing M4 primitive)
    // via the conductor proxy.
    todo!()
}

async fn handle_get_pending_recovery(state: AppState, _req: Request<Body>) -> Response<Body> {
    // SELECT * FROM recovery_requests WHERE my_agent IN emergency_contacts AND status = 'pending'
    todo!()
}

async fn handle_post_recovery_vote(state: AppState, req: Request<Body>) -> Response<Body> {
    // Forwards to imagodei coordinator submit_revocation_vote (existing M4 primitive)
    todo!()
}

async fn handle_get_portal_hosts(state: AppState, _req: Request<Body>) -> Response<Body> {
    // SELECT * FROM portal_hosts WHERE human_id = ? ORDER BY added_at DESC
    todo!()
}

async fn handle_post_portal_host(state: AppState, req: Request<Body>) -> Response<Body> {
    // Deserialize AddPortalHostInputView
    // Forward to imagodei coordinator add_portal_host
    // Wait for signal-driven projection update; return PortalHostView
    todo!()
}

async fn handle_delete_portal_host(state: AppState, req: Request<Body>) -> Response<Body> {
    // Forward to imagodei coordinator remove_portal_host
    todo!()
}
```

> **Note:** Each `todo!` placeholder is one focused implementation. Subagent dispatch should expand each in turn. The pattern is well-established in existing handlers (`handle_get_content`, `handle_create_content` etc.) — follow them.

- [ ] **Step 2: Register routes in `build_manifest()` and the HTTP match block**

In `build_manifest()`, add:

```rust
// In the manifest builder (existing pattern):
DoorwayRouteSpec {
    method: "GET", path: "/api/v1/account",
    description: "Aggregate account view".into(),
},
DoorwayRouteSpec {
    method: "GET", path: "/api/v1/account/keys",
    description: "Active key + rotation history".into(),
},
DoorwayRouteSpec {
    method: "GET", path: "/api/v1/account/revocations",
    description: "Recent KeyRevocation events".into(),
},
DoorwayRouteSpec {
    method: "POST", path: "/api/v1/account/self-revocation",
    description: "Initiate self-revocation".into(),
},
DoorwayRouteSpec {
    method: "GET", path: "/api/v1/account/pending-recovery",
    description: "Recovery requests where I am an EC".into(),
},
DoorwayRouteSpec {
    method: "POST", path: "/api/v1/account/recovery/:id/vote",
    description: "Vote on a recovery request".into(),
},
DoorwayRouteSpec {
    method: "GET", path: "/api/v1/account/portal-hosts",
    description: "List my portal hosts".into(),
},
DoorwayRouteSpec {
    method: "POST", path: "/api/v1/account/portal-hosts",
    description: "Add a portal host".into(),
},
DoorwayRouteSpec {
    method: "DELETE", path: "/api/v1/account/portal-hosts/:url_b64",
    description: "Remove a portal host".into(),
},
```

In the main HTTP match block, route each path to its handler.

- [ ] **Step 3: Implement each `todo!` body following the AccountView aggregation in spec §9.3**

(One sub-task per handler; expand into TDD cycles using existing route handler patterns.)

- [ ] **Step 4: Build + test**

```bash
cd /projects/elohim/elohim/elohim-storage \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "storage(http): /api/v1/account/* routes — M5"
```

---

## Task 11: elohim-storage schema contract tests + ts-rs codegen

**Files:**
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (add new view interfaces to `INTERFACE_FILES`)

> **Plan amendment (Task 1 corrected schemas):**
> - `KeyRotationView`, `KeyRevocationView`, `RevocationVoteView`, `RecoveryRequestView` already have schemas (`key-rotation.schema.json` etc., NO `-view` suffix). Their schema-contract tests likely already exist in `tests/schema_contract.rs` — verify and skip.
> - **Forward-reference resolution:** `account-view.schema.json` `$ref`s `epr:schema:view:human` and `epr:schema:view:human-relationship` — neither `human.schema.json` nor `human-relationship.schema.json` exists yet. Their corresponding Rust `HumanView` and `HumanRelationshipView` already exist in `views.rs` (lines 328 and 2288). Task 11 must create these two view schemas (not -view suffixed, per existing convention) so the AccountView contract test resolves cleanly.
> - New schema-contract tests needed: `verify_portal_host_view`, `verify_agent_peer_binding_view`, `verify_account_view`, `verify_add_portal_host_input`, `verify_submit_specialist_revocation_input`, `verify_human_view`, `verify_human_relationship_view`.

- [ ] **Step 1: Add schema contract tests**

In `tests/schema_contract.rs`, add `verify_*_view` tests for each new view (pattern matches existing `verify_content_view` etc.):

```rust
#[test]
fn verify_portal_host_view_matches_schema() {
    let schema_path = "../../sdk/schemas/v1/views/portal-host-view.schema.json";
    let example = PortalHostView {
        human_id: "uhCEkAbc".into(),
        host_url: "https://m.example/account".into(),
        label: Some("main".into()),
        added_at: "2026-04-25T12:00:00Z".into(),
        last_reachable_at: None,
        reach: "trusted".into(),
        dht_anchor_hash: "uhCkBcd".into(),
    };
    let json = serde_json::to_value(&example).unwrap();
    validate_against_schema(schema_path, &json);
}

// Similar test for each: AgentPeerBindingView, KeyRotationView, KeyRevocationView,
// RevocationVoteView, RecoveryRequestView, AccountView, AddPortalHostInputView,
// SubmitSpecialistRevocationInputView
```

- [ ] **Step 2: Add new types to codegen-ts.mjs**

In `elohim/sdk/schemas/scripts/codegen-ts.mjs`'s `INTERFACE_FILES` array, add:

```js
'portal-host-view',
'agent-peer-binding-view',
'key-rotation-view',
'key-revocation-view',
'revocation-vote-view',
'recovery-request-view',
'account-view',
```

- [ ] **Step 3: Run schema contract tests**

```bash
cd /projects/elohim/elohim/elohim-storage \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract
```

Expected: PASS — all new types match their schemas.

- [ ] **Step 4: Regenerate TS bindings**

```bash
cd /projects/elohim/elohim/elohim-storage \
  && cargo test export_bindings
cd /projects/elohim && pnpm run schema:codegen:ts
```

Expected: TS files appear in `elohim/sdk/storage-client-ts/src/generated/` for each new view.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "storage(schemas): contract tests + TS codegen for M5 view types"
```

---

## Task 12: doorway — `/auth/portal-host` route + `/auth/exchange-session` extension

**Files:**
- Create: `doorway/doorway-service/src/auth/portal_host.rs`
- Modify: `doorway/doorway-service/src/routes/auth_routes.rs`
- Modify: `doorway/doorway-service/src/server/http.rs`
- Modify: `doorway/doorway-service/src/auth/mod.rs`

- [ ] **Step 1: Write the portal-host handler**

Create `doorway/doorway-service/src/auth/portal_host.rs`:

```rust
//! GET /auth/portal-host — returns the authenticated human's preferred reachable
//! portal host URL.
//!
//! Behavior:
//! - Validate JWT
//! - Query elohim-storage GET /api/v1/account/portal-hosts (forwarded via http client)
//! - Probe each host with HEAD /healthz timeout 1s
//! - Return first reachable, or 200 with { reachable: false } when none

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;

use crate::auth::{extract_token_from_header, JwtValidator};
use crate::server::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalHostResponse {
    pub reachable: bool,
    pub host_url: Option<String>,
    pub all_hosts: Vec<String>,
}

pub async fn handle_portal_host(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Response<crate::routes::BoxBody> {
    let token = match extract_token_from_header(req.headers().get("Authorization")) {
        Some(t) => t,
        None => return crate::routes::json_response(
            StatusCode::UNAUTHORIZED,
            &serde_json::json!({"error": "no token"}),
        ),
    };
    let jwt = match crate::routes::auth_routes::get_jwt_validator(&state) {
        Ok(j) => j,
        Err(resp) => return resp,
    };
    let result = jwt.verify_token(token);
    if !result.valid {
        return crate::routes::json_response(
            StatusCode::UNAUTHORIZED,
            &serde_json::json!({"error": "invalid token"}),
        );
    }

    // Query storage for this human's portal hosts
    let claims = result.claims.unwrap();
    let storage_url = format!(
        "{}/api/v1/account/portal-hosts",
        state.args.storage_url.as_deref().unwrap_or("http://127.0.0.1:8090")
    );
    let client = reqwest::Client::new();
    let hosts: Vec<crate::types::PortalHostView> = match client
        .get(&storage_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => return crate::routes::json_response(
            StatusCode::OK,
            &PortalHostResponse { reachable: false, host_url: None, all_hosts: vec![] },
        ),
    };

    // Probe each for reachability
    let mut all_urls = Vec::with_capacity(hosts.len());
    let mut chosen: Option<String> = None;
    for h in &hosts {
        all_urls.push(h.host_url.clone());
        if chosen.is_none() {
            let probe = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                client.head(&format!("{}/healthz", h.host_url)).send(),
            )
            .await;
            if let Ok(Ok(resp)) = probe {
                if resp.status().is_success() {
                    chosen = Some(h.host_url.clone());
                }
            }
        }
    }

    crate::routes::json_response(
        StatusCode::OK,
        &PortalHostResponse {
            reachable: chosen.is_some(),
            host_url: chosen,
            all_hosts: all_urls,
        },
    )
}
```

- [ ] **Step 2: Register the module**

Modify `doorway/doorway-service/src/auth/mod.rs`:

```rust
pub mod portal_host;
```

- [ ] **Step 3: Route in http.rs**

In the HTTP match block, add an arm for `(Method::GET, "/auth/portal-host")`:

```rust
(Method::GET, "/auth/portal-host") => {
    crate::auth::portal_host::handle_portal_host(req, state).await
}
```

- [ ] **Step 4: Extend ExchangeSessionResponse**

In `auth_routes.rs`'s `ExchangeSessionResponse`, add:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub portal_host_url: Option<String>,
```

In `handle_exchange_session`, after determining `is_steward`, optionally call the portal-host handler internally and populate `portal_host_url`.

- [ ] **Step 5: Build + test**

```bash
cd /projects/elohim/doorway/doorway-service \
  && RUSTFLAGS="" cargo build --release \
  && RUSTFLAGS="" cargo test --lib --bins
```

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/auth/portal_host.rs \
        doorway/doorway-service/src/auth/mod.rs \
        doorway/doorway-service/src/routes/auth_routes.rs \
        doorway/doorway-service/src/server/http.rs
git commit -m "doorway(auth): /auth/portal-host route + ExchangeSessionResponse.portalHostUrl"
```

---

## Task 13: doorway-app — "Manage from your steward" section

**Files:**
- Modify: `doorway/doorway-app/src/app/components/account/doorway-account.component.ts`
- Modify: `doorway/doorway-app/src/app/services/doorway-admin.service.ts`
- Modify: `doorway/doorway-app/src/app/models/doorway.model.ts`

- [ ] **Step 1: Add type to model**

In `doorway.model.ts`:

```ts
export interface PortalHostResponse {
  reachable: boolean;
  hostUrl?: string;
  allHosts: string[];
}
```

- [ ] **Step 2: Add admin-service methods**

In `doorway-admin.service.ts`:

```ts
async getPortalHostUrl(): Promise<PortalHostResponse> {
  const resp = await firstValueFrom(
    this.http.get<PortalHostResponse>('/auth/portal-host')
  );
  return resp;
}

async mintSessionToken(): Promise<{ sessionToken: string; expiresAt: number }> {
  const resp = await firstValueFrom(
    this.http.get<{ sessionToken: string; expiresAt: number }>('/auth/session-token')
  );
  return resp;
}
```

- [ ] **Step 3: Add the section to doorway-account.component**

In `doorway-account.component.ts`, add a signal + method:

```ts
private readonly portalHostSignal = signal<PortalHostResponse | null>(null);
readonly portalHostUrl = computed(() => this.portalHostSignal()?.hostUrl ?? null);

async loadPortalHost() {
  try {
    const resp = await this.adminService.getPortalHostUrl();
    this.portalHostSignal.set(resp);
  } catch { this.portalHostSignal.set(null); }
}

async openSteward() {
  const url = this.portalHostUrl();
  if (!url) return;
  const { sessionToken } = await this.adminService.mintSessionToken();
  window.location.href = `${url}?session_token=${encodeURIComponent(sessionToken)}`;
}
```

In the template, after the existing `account-info` section:

```html
@if (portalHostUrl(); as hostUrl) {
  <section class="card portal-host">
    <h2>Manage from your steward</h2>
    <p>Your account is also reachable from your peer-native client.</p>
    <button class="btn-primary" (click)="openSteward()" data-testid="portal-host-redirect">
      Manage from your steward →
    </button>
  </section>
}
```

Call `loadPortalHost()` from `ngOnInit`.

- [ ] **Step 4: Build + lint**

```bash
cd /projects/elohim/doorway/doorway-app \
  && pnpm run build \
  && pnpm exec eslint src --ext .ts,.html
```

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-app/src/app/components/account/doorway-account.component.ts \
        doorway/doorway-app/src/app/services/doorway-admin.service.ts \
        doorway/doorway-app/src/app/models/doorway.model.ts
git commit -m "doorway-app(account): Manage-from-steward section + portal-host signal"
```

---

## Task 14: elohim-agent specialists/defender stub

**Files:**
- Create: `elohim/elohim-agent/specialists/defender/Cargo.toml`
- Create: `elohim/elohim-agent/specialists/defender/src/lib.rs`
- Create: `elohim/elohim-agent/specialists/defender/src/manifest.rs`
- Create: `elohim/elohim-agent/specialists/defender/src/role_marker.rs`
- Create: `elohim/elohim-agent/specialists/defender/src/detection.rs`
- Create: `elohim/elohim-agent/specialists/defender/src/attestation.rs`
- Modify: `elohim/elohim-agent/Cargo.toml` (add specialists/defender to workspace)
- Modify: `elohim/elohim-agent/elohim-agent-service/Cargo.toml` (add dep)
- Modify: `elohim/elohim-agent/elohim-agent-service/src/lib.rs`

- [ ] **Step 1: Workspace member**

Append to `elohim/elohim-agent/Cargo.toml` `[workspace] members`:

```toml
"specialists/defender",
```

- [ ] **Step 2: Defender Cargo.toml**

Create `elohim/elohim-agent/specialists/defender/Cargo.toml`:

```toml
[package]
name = "elohim-agent-defender-specialist"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
schemars = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
```

- [ ] **Step 3: Manifest**

Create `elohim/elohim-agent/specialists/defender/src/manifest.rs`:

```rust
//! DefenderManifest — declares the specialist's role/inputs/outputs.
//! Schema: elohim/sdk/schemas/v1/agent/defender-manifest.schema.json

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefenderManifest {
    pub specialist_kind: String,        // const "defender"
    pub for_humans: Vec<String>,        // Human ActionHashes (b64url)
    pub disclosure_tier: String,
    pub outputs: Vec<String>,
    pub system_prompt_template: String,
}

impl DefenderManifest {
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path).map_err(ManifestError::Io)?;
        let manifest: DefenderManifest = serde_json::from_str(&content).map_err(ManifestError::Parse)?;
        if manifest.specialist_kind != "defender" {
            return Err(ManifestError::WrongKind(manifest.specialist_kind));
        }
        Ok(manifest)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("wrong specialist kind: {0}")]
    WrongKind(String),
}
```

- [ ] **Step 4: Role marker**

Create `elohim/elohim-agent/specialists/defender/src/role_marker.rs`:

```rust
//! Local role marker — answers "is this elohim-agent configured as a defender
//! for the given human?" Hydrated from the DefenderManifest at startup.

use crate::manifest::DefenderManifest;

pub struct DefenderRoleMarker {
    manifest: DefenderManifest,
}

impl DefenderRoleMarker {
    pub fn new(manifest: DefenderManifest) -> Self {
        Self { manifest }
    }

    pub fn is_defender_for(&self, human_action_hash_b64: &str) -> bool {
        self.manifest
            .for_humans
            .iter()
            .any(|h| h == human_action_hash_b64)
    }
}
```

- [ ] **Step 5: Detection stub**

Create `elohim/elohim-agent/specialists/defender/src/detection.rs`:

```rust
//! Detection stub — subscribes to ReconcileController events and logs.
//!
//! M5 emits ZERO attestations. M6+ replaces this with real detection logic.

use tokio::sync::broadcast;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum ObservedEvent {
    KeyRotation(serde_json::Value),
    KeyRevocation(serde_json::Value),
    AgentPeerBinding(serde_json::Value),
    RevocationAttestation(serde_json::Value),
    PortalHostCreated(serde_json::Value),
    PortalHostRemoved(serde_json::Value),
}

pub async fn run_detection_loop(mut events: broadcast::Receiver<ObservedEvent>) {
    while let Ok(event) = events.recv().await {
        // STUB: M5 logs only.
        debug!("defender observed: {:?}", event);
    }
}
```

- [ ] **Step 6: Attestation stub**

Create `elohim/elohim-agent/specialists/defender/src/attestation.rs`:

```rust
//! Attestation stub — produces canned "no anomaly" responses.
//! Schema: elohim/sdk/schemas/v1/agent/anomaly-attestation.schema.json

use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyAttestation {
    pub observed_at: String,
    pub anomaly_kind: String,
    pub evidence: Vec<serde_json::Value>,
    pub confidence: f64,
}

pub fn build_no_anomaly_attestation() -> AnomalyAttestation {
    AnomalyAttestation {
        observed_at: Utc::now().to_rfc3339(),
        anomaly_kind: "none".into(),
        evidence: vec![],
        confidence: 0.0,
    }
}
```

- [ ] **Step 7: lib.rs**

Create `elohim/elohim-agent/specialists/defender/src/lib.rs`:

```rust
//! elohim-agent defender specialist — M5 stub.
//!
//! See: genesis/docs/superpowers/specs/2026-04-25-recovery-protocol-phase-2-m5-...md §11
//!
//! Stage 3 evolution: defender role attestation reuses the existing imagodei
//! `Attestation` entry type. NO new entry type ever needed for defender role.

pub mod attestation;
pub mod detection;
pub mod manifest;
pub mod role_marker;

pub use attestation::*;
pub use detection::*;
pub use manifest::*;
pub use role_marker::*;
```

- [ ] **Step 8: Build the specialist crate**

```bash
cd /projects/elohim/elohim/elohim-agent && RUSTFLAGS="" cargo build -p elohim-agent-defender-specialist
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-agent/Cargo.toml \
        elohim/elohim-agent/specialists/
git commit -m "elohim-agent(specialists): defender stub crate — manifest, role marker, detection+attestation stubs"
```

---

## Task 15: elohim-agent gate-client wiring for `is_defender_for`

**Files:**
- Modify: `elohim/elohim-agent/elohim-agent-service/src/service.rs`
- Modify: `elohim/elohim-agent/elohim-agent-service/Cargo.toml`

- [ ] **Step 1: Add dependency**

In `elohim-agent-service/Cargo.toml`:

```toml
elohim-agent-defender-specialist = { path = "../specialists/defender" }
```

- [ ] **Step 2: Wire gate handler**

In `service.rs` (or wherever `gate-client` `ask_gate` requests are routed), add a handler for `kind = "is_defender_for"`:

```rust
async fn handle_gate_request(
    &self,
    kind: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, ServiceError> {
    match kind {
        // ... existing kinds
        "is_defender_for" => {
            let human_hash = payload
                .get("humanActionHash")
                .and_then(|v| v.as_str())
                .ok_or(ServiceError::BadRequest("missing humanActionHash".into()))?;
            let is_defender = self.defender_role_marker
                .as_ref()
                .map(|m| m.is_defender_for(human_hash))
                .unwrap_or(false);
            Ok(serde_json::json!({ "isDefender": is_defender }))
        }
        _ => Err(ServiceError::UnknownGateKind(kind.into())),
    }
}
```

- [ ] **Step 3: Hydrate the role marker at startup**

In service initialization, load the defender manifest from a configured path (env var or config file). If absent, leave `defender_role_marker = None` (gate returns `false` — Stage 1 default-deny).

```rust
let defender_role_marker = std::env::var("ELOHIM_DEFENDER_MANIFEST")
    .ok()
    .and_then(|p| DefenderManifest::from_path(&p).ok())
    .map(DefenderRoleMarker::new);
```

- [ ] **Step 4: Build + test**

```bash
cd /projects/elohim/elohim/elohim-agent/elohim-agent-service \
  && RUSTFLAGS="" cargo build \
  && RUSTFLAGS="" cargo test
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-agent/elohim-agent-service/src/service.rs \
        elohim/elohim-agent/elohim-agent-service/Cargo.toml
git commit -m "elohim-agent(service): is_defender_for gate handler — hydrates role marker from manifest"
```

---

## Task 16: elohim-app pre-flight cross-pillar import audit + ESLint config

**Files:**
- Create: `app/elohim-app/scripts/audit-pillar-imports.mjs` (one-shot audit script)
- Modify: `app/elohim-app/eslint.config.mjs` (add boundaries config)
- Modify: any imagodei files violating the boundary (fix as part of this task)

- [ ] **Step 1: Write the audit script**

Create `app/elohim-app/scripts/audit-pillar-imports.mjs`:

```js
import { readFileSync } from 'fs';
import { globSync } from 'glob';

const PILLAR_BOUNDARIES = {
  imagodei: { allow: ['elohim', 'storage-client', 'common'] },
  account:  { allow: ['imagodei', 'elohim', 'storage-client', 'common'] },
  lamad:    { allow: ['elohim', 'storage-client', 'common'] },
  shefa:    { allow: ['elohim', 'storage-client', 'common'] },
  qahal:    { allow: ['elohim', 'storage-client', 'common'] },
};

const violations = [];
for (const pillar of Object.keys(PILLAR_BOUNDARIES)) {
  const files = globSync(`src/app/${pillar}/**/*.ts`);
  for (const file of files) {
    const src = readFileSync(file, 'utf-8');
    const importMatches = src.matchAll(/from ['"]@app\/(\w+)/g);
    for (const m of importMatches) {
      const importedPillar = m[1];
      if (importedPillar === pillar) continue;
      if (!PILLAR_BOUNDARIES[pillar].allow.includes(importedPillar)) {
        violations.push({ file, pillar, imports: importedPillar });
      }
    }
  }
}

if (violations.length === 0) {
  console.log('No cross-pillar boundary violations.');
  process.exit(0);
}

console.log('Cross-pillar boundary violations:');
for (const v of violations) {
  console.log(`  ${v.file}: ${v.pillar} → ${v.imports} (forbidden)`);
}
process.exit(1);
```

- [ ] **Step 2: Run the audit**

```bash
cd /projects/elohim/app/elohim-app && node scripts/audit-pillar-imports.mjs
```

Expected: prints violations (likely some in imagodei → lamad). Each violation must be resolved before account/ pillar can ship — either route the dependency through `storage-client-ts` or refactor.

- [ ] **Step 3: Fix any violations found**

For each `pillar/path/file.ts: imagodei → lamad`-style violation:
- Option A: refactor the dependency to come from `storage-client-ts`.
- Option B: relocate the dependency to a shared `common/` module.
- Option C: invert the dependency so the lower-level pillar exposes a service.

(Subagent dispatching this task should LIST violations first and confirm fix approach with orchestrator before refactoring across files.)

- [ ] **Step 4: Add ESLint boundaries config**

Append to `app/elohim-app/eslint.config.mjs`:

```js
import boundaries from 'eslint-plugin-boundaries';

export default [
  // ... existing config
  {
    plugins: { boundaries },
    settings: {
      'boundaries/elements': [
        { type: 'imagodei',       pattern: 'src/app/imagodei' },
        { type: 'account',        pattern: 'src/app/account' },
        { type: 'lamad',          pattern: 'src/app/lamad' },
        { type: 'shefa',          pattern: 'src/app/shefa' },
        { type: 'qahal',          pattern: 'src/app/qahal' },
        { type: 'elohim',         pattern: 'src/app/elohim' },
        { type: 'storage-client', pattern: 'node_modules/@elohim/storage-client' },
        { type: 'common',         pattern: 'src/app/common' },
      ],
    },
    rules: {
      'boundaries/element-types': ['error', {
        default: 'disallow',
        rules: [
          { from: 'imagodei', allow: ['elohim', 'storage-client', 'common'] },
          { from: 'account',  allow: ['imagodei', 'elohim', 'storage-client', 'common'] },
          { from: 'lamad',    allow: ['elohim', 'storage-client', 'common'] },
          { from: 'shefa',    allow: ['elohim', 'storage-client', 'common'] },
          { from: 'qahal',    allow: ['elohim', 'storage-client', 'common'] },
          { from: 'elohim',   allow: ['storage-client', 'common'] },
        ],
      }],
    },
  },
];
```

- [ ] **Step 5: Verify lint passes**

```bash
cd /projects/elohim/app/elohim-app && pnpm install && pnpm run lint
```

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/scripts/audit-pillar-imports.mjs \
        app/elohim-app/eslint.config.mjs \
        app/elohim-app/package.json \
        # any files refactored to fix violations
git commit -m "elohim-app(boundaries): cross-pillar import audit + ESLint boundaries plugin"
```

---

## Task 17: elohim-app account pillar skeleton + services

**Files:**
- Create: `app/elohim-app/src/app/account/account.routes.ts`
- Create: `app/elohim-app/src/app/account/index.ts`
- Create: `app/elohim-app/src/app/account/models/account.model.ts`
- Create: `app/elohim-app/src/app/account/models/portal-host.model.ts`
- Create: `app/elohim-app/src/app/account/services/account.service.ts`
- Create: `app/elohim-app/src/app/account/services/portal-host.service.ts`
- Create: `app/elohim-app/src/app/account/services/portal-host-discovery.service.ts`
- Create: `app/elohim-app/src/app/account/services/revocation.service.ts`
- Create: `app/elohim-app/src/app/account/services/handoff.service.ts`
- Create: `app/elohim-app/src/app/account/guards/account-guard.ts`
- Modify: `app/elohim-app/src/app/app.routes.ts`

- [ ] **Step 1: Models barrel**

Create `models/account.model.ts`:

```ts
export type {
  AccountView,
  KeyRotationView,
  KeyRevocationView,
  RecoveryRequestView,
  AgentPeerBindingView,
} from '@elohim/storage-client';

export interface AccountServiceState {
  account: AccountView | null;
  loading: boolean;
  error: string | null;
}
```

Create `models/portal-host.model.ts`:

```ts
export type { PortalHostView, AddPortalHostInputView } from '@elohim/storage-client';

export interface PortalHostServiceState {
  hosts: PortalHostView[];
  loading: boolean;
  error: string | null;
}
```

- [ ] **Step 2: account.service.ts**

```ts
import { Injectable, inject, signal, computed } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import type { AccountView } from '@elohim/storage-client';
import type { AccountServiceState } from '../models/account.model';

@Injectable({ providedIn: 'root' })
export class AccountService {
  private readonly http = inject(HttpClient);
  private readonly state = signal<AccountServiceState>({ account: null, loading: false, error: null });

  readonly account = computed(() => this.state().account);
  readonly loading = computed(() => this.state().loading);
  readonly error = computed(() => this.state().error);

  async refresh(): Promise<void> {
    this.state.update(s => ({ ...s, loading: true, error: null }));
    try {
      const account = await firstValueFrom(this.http.get<AccountView>('/api/v1/account'));
      this.state.set({ account, loading: false, error: null });
    } catch (e) {
      this.state.update(s => ({ ...s, loading: false, error: String(e) }));
    }
  }
}
```

- [ ] **Step 3: portal-host.service.ts**

```ts
import { Injectable, inject, signal, computed } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import type { PortalHostView, AddPortalHostInputView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class PortalHostService {
  private readonly http = inject(HttpClient);
  private readonly hostsSignal = signal<PortalHostView[]>([]);
  readonly hosts = this.hostsSignal.asReadonly();

  async list(): Promise<void> {
    const hosts = await firstValueFrom(this.http.get<PortalHostView[]>('/api/v1/account/portal-hosts'));
    this.hostsSignal.set(hosts);
  }

  async add(input: AddPortalHostInputView): Promise<PortalHostView> {
    const created = await firstValueFrom(this.http.post<PortalHostView>('/api/v1/account/portal-hosts', input));
    this.hostsSignal.update(hs => [created, ...hs]);
    return created;
  }

  async remove(hostUrl: string): Promise<void> {
    const b64 = btoa(hostUrl);
    await firstValueFrom(this.http.delete(`/api/v1/account/portal-hosts/${b64}`));
    this.hostsSignal.update(hs => hs.filter(h => h.hostUrl !== hostUrl));
  }
}
```

- [ ] **Step 4: portal-host-discovery.service.ts**

```ts
import { Injectable, inject, computed } from '@angular/core';
import { AccountService } from './account.service';

@Injectable({ providedIn: 'root' })
export class PortalHostDiscoveryService {
  private readonly account = inject(AccountService);
  readonly isSteward          = computed(() => this.account.account()?.isSteward ?? false);
  readonly hasLocalConductor  = computed(() => this.account.account()?.hasLocalConductor ?? false);
  readonly portalHosts        = computed(() => this.account.account()?.portalHosts ?? []);
  readonly preferredHost      = computed(() => this.portalHosts()[0]?.hostUrl ?? null);
}
```

- [ ] **Step 5: revocation.service.ts**

```ts
import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class RevocationService {
  private readonly http = inject(HttpClient);

  async selfRevoke(revokedPubKey: string): Promise<void> {
    await firstValueFrom(this.http.post('/api/v1/account/self-revocation', { revokedPubKey }));
  }

  async voteOnRecovery(recoveryRequestId: string, decision: 'approve' | 'reject'): Promise<void> {
    await firstValueFrom(this.http.post(`/api/v1/account/recovery/${recoveryRequestId}/vote`, { decision }));
  }
}
```

- [ ] **Step 6: handoff.service.ts**

```ts
import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { AuthService } from '@app/imagodei';

interface ExchangeSessionResponse {
  token: string;
  humanId: string;
  agentPubKey: string;
  identifier: string;
  expiresAt: number;
  doorwayUrl?: string;
  portalHostUrl?: string;
}

@Injectable({ providedIn: 'root' })
export class HandoffService {
  private readonly http = inject(HttpClient);
  private readonly auth = inject(AuthService);

  async consumeHandoffToken(token: string, doorwayUrl: string): Promise<boolean> {
    try {
      const resp = await firstValueFrom(
        this.http.get<ExchangeSessionResponse>(
          `${doorwayUrl}/auth/exchange-session?session_token=${encodeURIComponent(token)}`
        )
      );
      this.auth.setHostedAuth({
        token: resp.token,
        humanId: resp.humanId,
        agentPubKey: resp.agentPubKey,
        identifier: resp.identifier,
        expiresAt: resp.expiresAt,
      });
      return true;
    } catch {
      return false;
    }
  }
}
```

- [ ] **Step 7: account-guard.ts**

```ts
import { CanActivateFn, Router } from '@angular/router';
import { inject } from '@angular/core';
import { AuthService } from '@app/imagodei';
import { HandoffService } from '../services/handoff.service';
import { AccountService } from '../services/account.service';

export const accountGuard: CanActivateFn = async (route) => {
  const auth = inject(AuthService);
  const handoff = inject(HandoffService);
  const account = inject(AccountService);
  const router = inject(Router);

  // 1. Consume handoff token if present in query
  const sessionToken = route.queryParamMap.get('session_token');
  if (sessionToken) {
    const doorwayUrl = route.queryParamMap.get('doorway_url') || ''; // populated by doorway redirect
    const ok = await handoff.consumeHandoffToken(sessionToken, doorwayUrl);
    if (!ok) {
      router.navigate(['/identity/login']);
      return false;
    }
  }

  // 2. Confirm authenticated
  if (!auth.isAuthenticated()) {
    router.navigate(['/identity/login']);
    return false;
  }

  // 3. Refresh account view
  await account.refresh();
  return true;
};
```

- [ ] **Step 8: account.routes.ts**

```ts
import { Routes } from '@angular/router';
import { accountGuard } from './guards/account-guard';

export const ACCOUNT_ROUTES: Routes = [
  {
    path: '',
    canActivate: [accountGuard],
    loadComponent: () => import('./components/account-shell/account-shell.component')
      .then(m => m.AccountShellComponent),
    children: [
      { path: '', redirectTo: 'security', pathMatch: 'full' },
      { path: 'security',         loadComponent: () => import('./components/security-signin-pane/security-signin-pane.component').then(m => m.SecuritySigninPaneComponent) },
      { path: 'personal-info',    loadComponent: () => import('./components/personal-info-pane/personal-info-pane.component').then(m => m.PersonalInfoPaneComponent) },
      { path: 'data-privacy',     loadComponent: () => import('./components/data-privacy-pane/data-privacy-pane.component').then(m => m.DataPrivacyPaneComponent) },
      { path: 'people-sharing',   loadComponent: () => import('./components/people-sharing-pane/people-sharing-pane.component').then(m => m.PeopleSharingPaneComponent) },
      { path: 'third-party-apps', loadComponent: () => import('./components/third-party-apps-pane/third-party-apps-pane.component').then(m => m.ThirdPartyAppsPaneComponent) },
    ],
  },
];
```

- [ ] **Step 9: index.ts barrel**

```ts
export { AccountService } from './services/account.service';
export { PortalHostService } from './services/portal-host.service';
export { PortalHostDiscoveryService } from './services/portal-host-discovery.service';
export { RevocationService } from './services/revocation.service';
export { HandoffService } from './services/handoff.service';
export { ACCOUNT_ROUTES } from './account.routes';
```

- [ ] **Step 10: Mount in app.routes.ts**

In `app/elohim-app/src/app/app.routes.ts`:

```ts
{
  path: 'account',
  loadChildren: () => import('./account/account.routes').then(m => m.ACCOUNT_ROUTES),
},
```

- [ ] **Step 11: Build + lint**

```bash
cd /projects/elohim/app/elohim-app && pnpm run build && pnpm run lint
```

- [ ] **Step 12: Commit**

```bash
git add app/elohim-app/src/app/account/ \
        app/elohim-app/src/app/app.routes.ts
git commit -m "elohim-app(account): pillar skeleton + services + routes + handoff guard"
```

---

## Task 18: elohim-app account-shell + placeholder panes

**Files:**
- Create: `app/elohim-app/src/app/account/components/account-shell/account-shell.component.{ts,html,css}`
- Create: `app/elohim-app/src/app/account/components/personal-info-pane/personal-info-pane.component.{ts,html,css}`
- Create: `app/elohim-app/src/app/account/components/data-privacy-pane/data-privacy-pane.component.{ts,html,css}`
- Create: `app/elohim-app/src/app/account/components/people-sharing-pane/people-sharing-pane.component.{ts,html,css}`
- Create: `app/elohim-app/src/app/account/components/third-party-apps-pane/third-party-apps-pane.component.{ts,html,css}`

- [ ] **Step 1: Account-shell component**

Create `account-shell.component.ts`:

```ts
import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { AccountService } from '../../services/account.service';

@Component({
  selector: 'app-account-shell',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, RouterLinkActive],
  templateUrl: './account-shell.component.html',
  styleUrl: './account-shell.component.css',
})
export class AccountShellComponent {
  readonly account = inject(AccountService);
}
```

`account-shell.component.html`:

```html
<div class="account-shell">
  <nav class="pane-nav" data-testid="account-pane-nav">
    <a routerLink="security"          routerLinkActive="active" data-testid="nav-security">Security & sign-in</a>
    <a routerLink="personal-info"     routerLinkActive="active" data-testid="nav-personal-info">Personal info</a>
    <a routerLink="data-privacy"      routerLinkActive="active" data-testid="nav-data-privacy">Data & privacy</a>
    <a routerLink="people-sharing"    routerLinkActive="active" data-testid="nav-people-sharing">People & sharing</a>
    <a routerLink="third-party-apps"  routerLinkActive="active" data-testid="nav-third-party-apps">Third-party apps</a>
  </nav>
  <main class="pane-content"><router-outlet /></main>
</div>
<!-- [M5 scaffold — Playwright sprint will polish layout, a11y, responsive] -->
```

`account-shell.component.css` (minimal):

```css
.account-shell { display: flex; gap: 1rem; }
.pane-nav { display: flex; flex-direction: column; min-width: 220px; padding: 1rem; }
.pane-nav a { padding: 0.5rem; color: inherit; text-decoration: none; border-left: 3px solid transparent; }
.pane-nav a.active { border-left-color: currentColor; font-weight: 600; }
.pane-content { flex: 1; padding: 1rem; }
```

- [ ] **Step 2: Placeholder panes (4 of them — same shape)**

Each placeholder follows this template. Example `personal-info-pane.component.ts`:

```ts
import { Component } from '@angular/core';
@Component({
  selector: 'app-personal-info-pane',
  standalone: true,
  template: `
    <h1 data-testid="pane-title-personal-info">Personal info</h1>
    <p>[M5 scaffold — content lands in a follow-on sprint]</p>
  `,
})
export class PersonalInfoPaneComponent {}
```

Repeat for `data-privacy-pane`, `people-sharing-pane`, `third-party-apps-pane`.

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/app/elohim-app && pnpm run build
```

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/account/components/account-shell/ \
        app/elohim-app/src/app/account/components/personal-info-pane/ \
        app/elohim-app/src/app/account/components/data-privacy-pane/ \
        app/elohim-app/src/app/account/components/people-sharing-pane/ \
        app/elohim-app/src/app/account/components/third-party-apps-pane/
git commit -m "elohim-app(account): account-shell + placeholder panes"
```

---

## Task 19: elohim-app security-signin-pane + 4 sub-flow components

**Files:**
- Create: `app/elohim-app/src/app/account/components/security-signin-pane/security-signin-pane.component.{ts,html,css}`
- Create: `app/elohim-app/src/app/account/components/security-signin-pane/key-list/key-list.component.{ts,html,css}`
- Create: `app/elohim-app/src/app/account/components/security-signin-pane/self-revoke/self-revoke.component.{ts,html,css}`
- Create: `app/elohim-app/src/app/account/components/security-signin-pane/vote-as-ec/vote-as-ec.component.{ts,html,css}`
- Create: `app/elohim-app/src/app/account/components/security-signin-pane/lost-key-entry/lost-key-entry.component.{ts,html,css}`

- [ ] **Step 1: security-signin-pane**

`security-signin-pane.component.ts`:

```ts
import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { AccountService } from '../../services/account.service';
import { KeyListComponent } from './key-list/key-list.component';
import { SelfRevokeComponent } from './self-revoke/self-revoke.component';
import { VoteAsEcComponent } from './vote-as-ec/vote-as-ec.component';
import { LostKeyEntryComponent } from './lost-key-entry/lost-key-entry.component';

@Component({
  selector: 'app-security-signin-pane',
  standalone: true,
  imports: [CommonModule, KeyListComponent, SelfRevokeComponent, VoteAsEcComponent, LostKeyEntryComponent],
  templateUrl: './security-signin-pane.component.html',
})
export class SecuritySigninPaneComponent {
  readonly account = inject(AccountService);
}
```

`security-signin-pane.component.html`:

```html
<h1 data-testid="pane-title-security">Security & sign-in</h1>

<section class="card" data-testid="my-keys-section">
  <h2>My keys</h2>
  <app-key-list />
</section>

<section class="card" data-testid="self-revoke-section">
  <h2>Revoke this key</h2>
  <app-self-revoke />
</section>

<section class="card" data-testid="vote-as-ec-section">
  <h2>Help someone recover</h2>
  <app-vote-as-ec />
</section>

<section class="card" data-testid="lost-key-section">
  <h2>I lost my key</h2>
  <app-lost-key-entry />
</section>
```

- [ ] **Step 2: key-list (read-only display)**

```ts
import { Component, inject } from '@angular/core';
import { CommonModule, DatePipe } from '@angular/common';
import { AccountService } from '../../../services/account.service';

@Component({
  selector: 'app-key-list',
  standalone: true,
  imports: [CommonModule, DatePipe],
  template: `
    @let acct = account.account();
    @if (acct?.activeKeyRotation; as active) {
      <div class="key-row" data-testid="active-key">
        <strong>Active</strong> · {{ active.newPubKey | slice:0:16 }}…
        <span class="muted">since {{ active.rotatedAt | date:'mediumDate' }}</span>
      </div>
    }
    @for (rev of acct?.recentRevocations ?? []; track rev.dhtAnchorHash) {
      <div class="key-row revoked" [attr.data-testid]="'revoked-' + rev.dhtAnchorHash">
        <strong>Revoked</strong> · {{ rev.revokedPubKey | slice:0:16 }}…
        <span class="muted">{{ rev.triggerType }} · {{ rev.revokedAt | date:'mediumDate' }}</span>
      </div>
    }
  `,
})
export class KeyListComponent {
  readonly account = inject(AccountService);
}
```

- [ ] **Step 3: self-revoke**

```ts
import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { AccountService } from '../../../services/account.service';
import { RevocationService } from '../../../services/revocation.service';

@Component({
  selector: 'app-self-revoke',
  standalone: true,
  imports: [CommonModule],
  template: `
    @let active = account.account()?.activeKeyRotation;
    @if (active) {
      <p>You're about to revoke the key currently in use. This is irreversible.</p>
      @if (!confirming()) {
        <button data-testid="self-revoke-start" (click)="confirming.set(true)">Revoke this key</button>
      } @else {
        <p><strong>Are you sure?</strong> Your peers will be notified to stop trusting this key.</p>
        <button data-testid="self-revoke-confirm" (click)="confirm()" [disabled]="working()">Yes, revoke</button>
        <button data-testid="self-revoke-cancel" (click)="confirming.set(false)">Cancel</button>
      }
    }
  `,
})
export class SelfRevokeComponent {
  readonly account = inject(AccountService);
  private readonly rev = inject(RevocationService);
  readonly confirming = signal(false);
  readonly working = signal(false);

  async confirm() {
    const pubKey = this.account.account()?.activeKeyRotation?.newPubKey;
    if (!pubKey) return;
    this.working.set(true);
    await this.rev.selfRevoke(pubKey);
    await this.account.refresh();
    this.working.set(false);
    this.confirming.set(false);
  }
}
```

- [ ] **Step 4: vote-as-ec**

```ts
import { Component, inject } from '@angular/core';
import { CommonModule, DatePipe } from '@angular/common';
import { AccountService } from '../../../services/account.service';
import { RevocationService } from '../../../services/revocation.service';

@Component({
  selector: 'app-vote-as-ec',
  standalone: true,
  imports: [CommonModule, DatePipe],
  template: `
    @let pending = account.account()?.pendingRecoveryRequests ?? [];
    @if (pending.length === 0) {
      <p data-testid="vote-empty">No pending recovery requests where you are an emergency contact.</p>
    } @else {
      @for (req of pending; track req.dhtAnchorHash) {
        <div class="card" [attr.data-testid]="'pending-' + req.dhtAnchorHash">
          <p>Recovery for human <code>{{ req.humanId | slice:0:16 }}…</code></p>
          <p class="muted">Authority: {{ req.proposedAuthority }} · {{ req.requestedAt | date:'medium' }}</p>
          <button [attr.data-testid]="'approve-' + req.dhtAnchorHash" (click)="vote(req.dhtAnchorHash, 'approve')">Approve</button>
          <button [attr.data-testid]="'reject-'  + req.dhtAnchorHash" (click)="vote(req.dhtAnchorHash, 'reject')">Reject</button>
        </div>
      }
    }
  `,
})
export class VoteAsEcComponent {
  readonly account = inject(AccountService);
  private readonly rev = inject(RevocationService);

  async vote(id: string, decision: 'approve' | 'reject') {
    await this.rev.voteOnRecovery(id, decision);
    await this.account.refresh();
  }
}
```

- [ ] **Step 5: lost-key-entry**

```ts
import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router } from '@angular/router';
import { AccountService } from '../../../services/account.service';

@Component({
  selector: 'app-lost-key-entry',
  standalone: true,
  imports: [CommonModule],
  template: `
    @let acct = account.account();
    @if (acct) {
      @if (acct.activeKeyRotation) {
        <p>You can revoke your current key (above) or initiate recovery if you can't sign in.</p>
        <button data-testid="lost-key-recovery" (click)="goRecovery()">Start recovery</button>
      } @else {
        <p>You don't have an active key. Use the recovery flow.</p>
        <button data-testid="lost-key-recovery" (click)="goRecovery()">Start recovery</button>
      }
    }
  `,
})
export class LostKeyEntryComponent {
  readonly account = inject(AccountService);
  private readonly router = inject(Router);

  goRecovery() {
    // Redirect to existing /identity recovery flow
    this.router.navigate(['/identity/recover']);
  }
}
```

- [ ] **Step 6: Build + lint + unit-tests**

```bash
cd /projects/elohim/app/elohim-app \
  && pnpm run build \
  && pnpm run lint \
  && pnpm test --run
```

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/src/app/account/components/security-signin-pane/
git commit -m "elohim-app(account): security-signin-pane + key-list/self-revoke/vote-as-ec/lost-key-entry"
```

---

## Task 20: a2o features (`@recovery-m5`)

**Files:**
- Create: `genesis/a2o/features/auth/recovery/recovery-m5-list-my-keys.feature`
- Create: `genesis/a2o/features/auth/recovery/recovery-m5-self-revoke.feature`
- Create: `genesis/a2o/features/auth/recovery/recovery-m5-vote-as-emergency-contact.feature`
- Create: `genesis/a2o/features/auth/recovery/recovery-m5-lost-key-entry.feature`
- Create: `genesis/a2o/features/auth/recovery/recovery-m5-doorway-handoff-to-steward.feature`
- Create: `genesis/a2o/features/auth/recovery/recovery-m5-portal-host-discovery.feature`
- Create: `genesis/a2o/features/auth/recovery/recovery-m5-defender-role-gate.feature`

- [ ] **Step 1: list-my-keys.feature**

```gherkin
@recovery-m5 @account-pillar
Feature: Listing my keys in the account-management surface
  As a steward
  I want to see my active key and revocation history
  So that I can verify my account's security posture

  Background:
    Given I am authenticated as a hosted human with a graduated steward presence
    And my AccountView includes one active KeyRotation and zero recent KeyRevocations

  Scenario: Active key is visible on the Security & sign-in pane
    When I navigate to /account/security
    Then I see the "Security & sign-in" pane title
    And I see my active key listed under "My keys"
    And the active key shows its rotation date

  Scenario: Revocation history is visible
    Given my AccountView has one revoked key with triggerType "self"
    When I navigate to /account/security
    Then I see the revoked key under "My keys"
    And the revoked key is labelled with triggerType "self"
```

- [ ] **Step 2: self-revoke.feature**

```gherkin
@recovery-m5 @account-pillar @revocation
Feature: Self-revocation through the account-management surface
  As a steward concerned my key may be compromised
  I want to revoke my current key
  So that peers stop trusting it immediately

  Scenario: Successful self-revocation
    Given I am authenticated with an active key
    And I navigate to /account/security
    When I click "Revoke this key"
    And I click "Yes, revoke"
    Then a KeyRevocation entry is committed with triggerType "self"
    And the AccountView refreshes to show the key as revoked
    And my Security & sign-in pane reflects the revocation

  Scenario: Cancel before confirming does not revoke
    Given I am authenticated with an active key
    When I click "Revoke this key"
    And I click "Cancel"
    Then no KeyRevocation entry is committed
```

- [ ] **Step 3: vote-as-emergency-contact.feature**

```gherkin
@recovery-m5 @account-pillar @recovery-vote
Feature: Voting on recovery as an emergency contact
  As an emergency contact
  I want to approve or reject pending recovery requests
  So that the human's recovery can proceed under graduated authority

  Background:
    Given I am authenticated
    And I am an emergency contact for a human with a pending RecoveryRequest

  Scenario: Approve a pending recovery
    When I navigate to /account/security
    And I click "Approve" on the pending recovery card
    Then a RevocationVote entry is committed with decision "approve"
    And the pending recovery card disappears from my view

  Scenario: Reject a pending recovery
    When I navigate to /account/security
    And I click "Reject" on the pending recovery card
    Then a RevocationVote entry is committed with decision "reject"
```

- [ ] **Step 4: lost-key-entry.feature**

```gherkin
@recovery-m5 @account-pillar @recovery-entry
Feature: Lost-key entry point routes to the right flow
  As a steward who cannot sign in
  I want a single entry point for "I lost my key"
  So that I am routed to recovery or revocation based on my state

  Scenario: Active key holder is routed to recovery
    Given I am authenticated and have an active key
    When I navigate to /account/security
    And I click "Start recovery" under "I lost my key"
    Then I am redirected to /identity/recover

  Scenario: No active key — routed to recovery anyway
    Given I am authenticated but my account has no active key
    When I navigate to /account/security
    And I click "Start recovery" under "I lost my key"
    Then I am redirected to /identity/recover
```

- [ ] **Step 5: doorway-handoff-to-steward.feature**

```gherkin
@recovery-m5 @auth-portal-convergence
Feature: Doorway redirects steward humans to their portal host
  As a steward who has graduated from hosted to peer-native
  I want doorway/account to point me at my own steward
  So that I manage my account from peer-native infrastructure

  Background:
    Given I am authenticated at a doorway as a graduated steward
    And my AccountView includes a portal host at "https://matthew.steward.example/account"
    And the portal host responds to /healthz with 200

  Scenario: Doorway shows the redirect link
    When I navigate to doorway/account
    Then I see a "Manage from your steward →" button

  Scenario: Click redirects with a session token
    When I click "Manage from your steward →"
    Then I am redirected to the portal host URL with a session_token query parameter
    And elohim-app's account-guard consumes the session_token
    And I land on /account/security authenticated as the same identity

  Scenario: No reachable portal host — fall through to hosted view
    Given my portal host does not respond to /healthz
    When I navigate to doorway/account
    Then I do NOT see the "Manage from your steward →" button
    And I see the existing hosted account view
```

- [ ] **Step 6: portal-host-discovery.feature**

```gherkin
@recovery-m5 @account-pillar @portal-host
Feature: Adding and listing portal hosts
  As a steward
  I want to declare which URLs may render my auth portal
  So that doorway and trusted peers know where to redirect

  Scenario: Add a portal host
    Given I am authenticated as a steward
    When I POST {"hostUrl": "https://matthew.steward.example/account", "label": "main"} to /api/v1/account/portal-hosts
    Then the response is 200 with the new PortalHostView
    And /api/v1/account/portal-hosts returns the host in the list

  Scenario: Validator rejects http URL
    When I POST {"hostUrl": "http://insecure.example/account"} to /api/v1/account/portal-hosts
    Then the response is 400 with an error mentioning https
```

- [ ] **Step 7: defender-role-gate.feature**

```gherkin
@recovery-m5 @defender-stub
Feature: submit_specialist_revocation gated by local defender role marker
  As an elohim-agent acting on a human's behalf
  I want submit_specialist_revocation to verify my defender role
  So that the structural quorum gate from M4 retains its meaning

  Scenario: Without role marker — coordinator rejects
    Given the calling elohim-agent has no DefenderManifest configured
    When I call submit_specialist_revocation with a valid SubmitSpecialistRevocationInput
    Then the call returns an error mentioning "not a configured defender"

  Scenario: With role marker — coordinator accepts
    Given the calling elohim-agent has a DefenderManifest listing the target human
    When I call submit_specialist_revocation with a valid SubmitSpecialistRevocationInput
    Then the call returns an ActionHash
    And a KeyRevocation entry is committed with triggerType "specialist_attestation"
```

- [ ] **Step 8: Commit**

```bash
git add genesis/a2o/features/auth/recovery/recovery-m5-*.feature
git commit -m "a2o(recovery): M5 acceptance scenarios — 7 features tagged @recovery-m5"
```

---

## Task 21: Cypress wiring tests for the four flows + handoff

**Files:**
- Create: `app/elohim-app/cypress/e2e/account-m5/security-pane-renders.cy.ts`
- Create: `app/elohim-app/cypress/e2e/account-m5/self-revoke-flow.cy.ts`
- Create: `app/elohim-app/cypress/e2e/account-m5/vote-as-ec-flow.cy.ts`
- Create: `app/elohim-app/cypress/e2e/account-m5/lost-key-entry-flow.cy.ts`
- Create: `app/elohim-app/cypress/e2e/account-m5/handoff-from-doorway.cy.ts`

- [ ] **Step 1: security-pane-renders.cy.ts**

```ts
import { Given, When, Then } from '@badeball/cypress-cucumber-preprocessor';

Given('I am authenticated with an active key on the account pillar', () => {
  cy.intercept('GET', '/api/v1/account', { fixture: 'account-with-active-key.json' });
  cy.window().then(win => win.localStorage.setItem('auth_token', 'fixture-token'));
});

When('I navigate to {string}', (path: string) => {
  cy.visit(path);
});

Then('I see the {string} pane title', (title: string) => {
  cy.get('[data-testid^=pane-title-]').should('contain.text', title);
});

Then('I see my active key listed', () => {
  cy.get('[data-testid=active-key]').should('be.visible');
});
```

- [ ] **Step 2: self-revoke-flow.cy.ts**

```ts
import { Given, When, Then } from '@badeball/cypress-cucumber-preprocessor';

When('I click {string} on the security pane', (label: string) => {
  if (label === 'Revoke this key') cy.get('[data-testid=self-revoke-start]').click();
  else if (label === 'Yes, revoke') cy.get('[data-testid=self-revoke-confirm]').click();
  else if (label === 'Cancel')      cy.get('[data-testid=self-revoke-cancel]').click();
});

Then('a KeyRevocation entry is committed with triggerType {string}', (triggerType: string) => {
  cy.intercept('POST', '/api/v1/account/self-revocation').as('selfRevoke');
  cy.wait('@selfRevoke').its('request.body').should('have.property', 'revokedPubKey');
});
```

- [ ] **Step 3: vote-as-ec-flow.cy.ts** (similar pattern)

```ts
import { Given, When, Then } from '@badeball/cypress-cucumber-preprocessor';

When('I click {string} on the pending recovery card', (action: string) => {
  cy.get(`[data-testid^=${action.toLowerCase()}-]`).first().click();
});

Then('a RevocationVote entry is committed with decision {string}', (decision: string) => {
  cy.intercept('POST', '/api/v1/account/recovery/*/vote').as('vote');
  cy.wait('@vote').its('request.body.decision').should('eq', decision);
});
```

- [ ] **Step 4: lost-key-entry-flow.cy.ts**

```ts
import { Then } from '@badeball/cypress-cucumber-preprocessor';

Then('I am redirected to {string}', (path: string) => {
  cy.url().should('include', path);
});
```

- [ ] **Step 5: handoff-from-doorway.cy.ts**

```ts
import { Given, When, Then } from '@badeball/cypress-cucumber-preprocessor';

Given('I am authenticated at a doorway as a graduated steward', () => {
  cy.intercept('GET', '/auth/portal-host', {
    body: { reachable: true, hostUrl: 'http://localhost:4200/account', allHosts: ['http://localhost:4200/account'] },
  });
  cy.intercept('GET', '/auth/session-token', { body: { sessionToken: 'fixture-token', expiresAt: 9999999999 } });
  cy.intercept('GET', '/auth/exchange-session*', { fixture: 'exchange-session-response.json' });
});

When('I navigate to doorway/account', () => {
  cy.visit('/');  // doorway-app fixture mode
});

Then('I see a {string} button', (label: string) => {
  cy.get('[data-testid=portal-host-redirect]').should('contain.text', label);
});

When('I click {string}', (label: string) => {
  cy.get('[data-testid=portal-host-redirect]').click();
});

Then('I land on {string} authenticated as the same identity', (path: string) => {
  cy.url().should('include', path);
});
```

- [ ] **Step 6: Run cypress headless**

```bash
cd /projects/elohim/app/elohim-app && pnpm run cypress:run --spec "cypress/e2e/account-m5/**/*.cy.ts"
```

> **Note:** Cypress runs in CI per memory `feedback_shift_measure_jenkins` — Eclipse Che can't run a full e2e stack. Subagents commit the code and let CI verify.

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/cypress/e2e/account-m5/
git commit -m "tests(cypress): M5 wiring tests — Security pane + flows + handoff"
```

---

## Task 22: Final integration validation + DNA pack release

**Files:**
- Run: pre-push gate locally
- Run: full storage build + tests
- Run: schema validate + codegen verify

- [ ] **Step 1: Pack the imagodei DNA**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei && just check && just pack
```

Expected: PASS, fresh `imagodei.dna` artifact.

- [ ] **Step 2: Full elohim-storage build**

```bash
cd /projects/elohim/elohim/elohim-storage && cargo clean && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```

- [ ] **Step 3: Doorway full build**

```bash
cd /projects/elohim/doorway/doorway-service && cargo clean && RUSTFLAGS="" cargo build --release && RUSTFLAGS="" cargo test --lib --bins && RUSTFLAGS="" cargo clippy -- -D warnings && cargo fmt --check
cd /projects/elohim/doorway/doorway-app && pnpm run build && pnpm exec eslint src --ext .ts,.html
```

- [ ] **Step 4: elohim-agent full build**

```bash
cd /projects/elohim/elohim/elohim-agent && RUSTFLAGS="" cargo build && RUSTFLAGS="" cargo test
```

- [ ] **Step 5: elohim-app full check**

```bash
cd /projects/elohim/app/elohim-app && pnpm run build && pnpm test --run && pnpm run lint && pnpm run format:check
```

- [ ] **Step 6: Schema verifications**

```bash
cd /projects/elohim && pnpm run schema:test && pnpm run schema:validate && pnpm run schema:codegen:ts && pnpm run schema:check-dna
```

- [ ] **Step 7: Audit cross-pillar boundaries**

```bash
cd /projects/elohim/app/elohim-app && node scripts/audit-pillar-imports.mjs
```

Expected: "No cross-pillar boundary violations."

- [ ] **Step 8: Push (triggers Jenkins for sweettest + Cypress)**

```bash
cd /projects/elohim && git push origin feature/recovery-m5-auth-portal-and-revocation-ux
```

> Pre-push gate runs `cargo fmt + clippy + tests`. Budget ~25–30 minutes on cold cache.

- [ ] **Step 9: Verify Jenkins pipelines**

Use `pipeline-diagnostics` skill or `mcp__jenkins__getBuild` to confirm:
- `holochain/dna-imagodei` pipeline: PASS
- `holochain/edge` pipeline (storage): PASS
- `app` pipeline (elohim-app): PASS
- a2o pipeline (Cucumber scenarios): PASS — all `@recovery-m5` scenarios green

- [ ] **Step 10: Merge to dev (per memory `feedback_dev_branch_no_pr`)**

```bash
git checkout dev && git merge --no-ff feature/recovery-m5-auth-portal-and-revocation-ux
git push origin dev
```

---

## Self-review (executed before saving)

**Spec coverage check:**

| Spec section | Plan task |
|---|---|
| §2.1 imagodei DNA additions | Tasks 2, 3, 4, 5 |
| §2.1 elohim-storage views/projection/HTTP | Tasks 7, 8, 9, 10, 11 |
| §2.1 doorway routes | Task 12 |
| §2.1 doorway-app section | Task 13 |
| §2.1 elohim-agent specialists | Tasks 14, 15 |
| §2.1 elohim-app account pillar | Tasks 16, 17, 18, 19 |
| §2.1 schemas (schema-first IoC) | Task 1 |
| §2.1 verification (a2o, sweettest, vitest, cypress) | Tasks 6, 19 (vitest runs with build), 20, 21 |
| §6 P2P design gate compliance | Encoded in Task 2 (PortalHost on Human ActionHash), Task 14 (defender Operational), Task 16 (boundary lint) |
| §15 backward compat | Tasks are additive — no existing entry/route/component modified destructively |

**Placeholder scan:**
- Task 10 has nine `todo!()` markers per handler — those are explicit work items for the subagent dispatch and follow the existing `handle_*` pattern in storage HTTP. Acceptable as labelled work surface.
- Task 8 step 1 references `<TIMESTAMP>` for migration filename — generated by `diesel migration generate` command (real placeholder).

No "TBD," "implement later," or untyped function references.

**Type consistency:**
- `AddPortalHostInput` (zome input) is shaped identically across Tasks 1 (schema), 3 (Rust struct), 7 (storage InputView), 17 (TS model). Field names match.
- `SubmitSpecialistRevocationInput` consistent across Tasks 1, 4, 7.
- `PortalHostView` consistent across schema (Task 1), Rust view (Task 7), TS model (Task 17), Cypress fixture (Task 21).
- `AccountView` consistent across schema (Task 1), Rust view (Task 7), service (Task 17).

No mismatches found.

---

## Execution handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-04-25-recovery-protocol-phase-2-m5-auth-portal-convergence-revocation-ux-and-stub-defender.md`.

**Two execution options:**

**1. Subagent-Driven (recommended)** — orchestrator dispatches one fresh subagent per task, reviews between tasks, fast iteration. Each subagent prompt carries the scope guardrails verbatim. Orchestrator scans SHA range post-dispatch to verify no out-of-scope commits.

**2. Inline Execution** — execute tasks in the current session via `superpowers:executing-plans`, batch execution with checkpoints for review.

**Which approach?**
