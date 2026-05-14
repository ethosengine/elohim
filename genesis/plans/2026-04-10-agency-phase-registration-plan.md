# Agency-Phase Registration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make doorway's `/auth/register` branch on the human's agency phase so each registration targets the correct conductor, fixing the seeder's 503/401 failures.

**Architecture:** Add `agencyPhase` to humans.json and the register payload. Doorway branches: `doorway` uses the existing ZomeCaller, `hosted` provisions on the operator's conductor then creates identity there, `node`/`device` skips `create_human` (identity exists on their conductor) and only creates the DB record. The seeder creates identities directly on node/device conductors before registering with doorway.

**Tech Stack:** Rust (doorway-service), TypeScript (genesis seeder), JSON (humans.json)

**Spec:** `genesis/plans/2026-04-10-agency-phase-registration-design.md`

---

### Task 1: Add `agencyPhase` to humans.json

**Files:**
- Modify: `genesis/docs/humans/humans.json`

- [ ] **Step 1: Add agencyPhase to the 7 active humans**

Add `"agencyPhase"` field to each human. Place it after `"id"` for readability:

```json
{"id": "human-matthew-manager", "agencyPhase": "doorway", ...}
{"id": "human-adam-firstman", "agencyPhase": "node", ...}
{"id": "human-eve-firstwoman", "agencyPhase": "device", ...}
{"id": "human-jessica-spouse", "agencyPhase": "device", ...}
{"id": "human-pete-pastor", "agencyPhase": "device", ...}
{"id": "human-terrance-tutor", "agencyPhase": "device", ...}
{"id": "human-nancy-neighbor", "agencyPhase": "hosted", ...}
```

All other humans: do NOT add the field. Absent means `visitor` (inactive persona).

- [ ] **Step 2: Verify JSON is valid**

Run: `node -e "JSON.parse(require('fs').readFileSync('genesis/docs/humans/humans.json', 'utf-8')); console.log('OK')"`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add genesis/docs/humans/humans.json
git commit -m "feat(genesis): add agencyPhase to active humans in humans.json"
```

---

### Task 2: Add `agencyPhase` to doorway's RegisterRequest

**Files:**
- Modify: `doorway/doorway-service/src/routes/auth_routes.rs:103-136` (RegisterRequest struct)

- [ ] **Step 1: Add agency_phase field to RegisterRequest**

In `doorway/doorway-service/src/routes/auth_routes.rs`, add the field to `RegisterRequest` (after `admin_bootstrap_key`, around line 135):

```rust
    /// Agency phase for graduated stewardship: doorway, node, device, hosted, visitor.
    /// Determines registration flow — whether doorway creates identity or just DB record.
    #[serde(default)]
    pub agency_phase: Option<String>,
```

- [ ] **Step 2: Verify it compiles**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/routes/auth_routes.rs
git commit -m "feat(doorway): add agency_phase field to RegisterRequest"
```

---

### Task 3: Refactor handle_register to branch on agency phase

This is the core change. The current `handle_register` (line 626) has a single flow that always calls `create_human` via ZomeCaller then provisions. We refactor into phase-specific branches.

**Files:**
- Modify: `doorway/doorway-service/src/routes/auth_routes.rs:626-1052` (handle_register function)
- Modify: `doorway/doorway-service/src/routes/zome_helpers.rs` (add `call_create_human_on_conductor`)

- [ ] **Step 1: Add `call_create_human_on_conductor` to zome_helpers.rs**

This creates a temporary ZomeCaller targeting a specific conductor (not the singleton). Add after `call_get_my_human` in `doorway/doorway-service/src/routes/zome_helpers.rs`:

```rust
/// Call imagodei::create_human on a specific conductor (not the singleton ZomeCaller).
///
/// Used for `hosted` registrations where the human's identity is created on
/// the operator's conductor (identified during provisioning), not the doorway's
/// default ZomeCaller target.
pub async fn call_create_human_on_conductor(
    conductor_url: &str,
    installed_app_id: &str,
    input: CreateHumanInput,
) -> Result<HumanOutput> {
    // Derive admin URL: port - 1 (socat convention: 4444=admin, 4445=app)
    let admin_url = crate::derive_admin_url_from_app(conductor_url);

    debug!(
        conductor_url = %conductor_url,
        admin_url = %admin_url,
        installed_app_id = %installed_app_id,
        human_id = %input.id,
        "Creating temporary ZomeCaller for hosted registration"
    );

    let caller = crate::services::ZomeCaller::new(&admin_url, conductor_url, installed_app_id);

    let result: HumanOutput = caller
        .call("imagodei", "imagodei", "create_human", &input)
        .await
        .map_err(|e| DoorwayError::Holochain(format!("create_human on conductor failed: {e}")))?;

    Ok(result)
}
```

- [ ] **Step 2: Make `derive_admin_url_from_app` public**

In `doorway/doorway-service/src/main.rs`, change:
```rust
fn derive_admin_url_from_app(app_url: &str) -> String {
```
to:
```rust
pub fn derive_admin_url_from_app(app_url: &str) -> String {
```

- [ ] **Step 3: Verify it compiles**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished` with no errors (warning about unused function is OK for now)

- [ ] **Step 4: Refactor handle_register to branch on agency_phase**

Replace the body of `handle_register` in `doorway/doorway-service/src/routes/auth_routes.rs` (from after body parsing/validation through to the provisioning block, approximately lines 653-815) with phase-branched logic.

The new flow after body parsing and validation:

```rust
    // Determine display name for registration
    let display_name = if body.display_name.is_empty() {
        body.identifier
            .split('@')
            .next()
            .unwrap_or("User")
            .to_string()
    } else {
        body.display_name.clone()
    };

    // Parse agency phase (default to "hosted" for backwards compatibility)
    let agency_phase = body.agency_phase.as_deref().unwrap_or("hosted");

    // Reject visitors — they don't register
    if agency_phase == "visitor" {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                error: "Visitors do not register — use the app without an account".into(),
                code: Some("VISITOR_NO_REGISTER".into()),
            },
        );
    }

    // Branch registration flow by agency phase
    let (human_id, agent_pub_key, profile, provisioned) = match agency_phase {
        // =====================================================================
        // DOORWAY: Operator bootstrap — existing ZomeCaller flow
        // =====================================================================
        "doorway" => {
            let generated_human_id = uuid::Uuid::new_v4().to_string();
            let zome_result = call_create_human(
                &state,
                CreateHumanInput {
                    id: generated_human_id.clone(),
                    display_name: display_name.clone(),
                    bio: body.bio.clone(),
                    affinities: body.affinities.clone(),
                    profile_reach: body.profile_reach.clone(),
                    location: body.location.clone(),
                },
            )
            .await;

            match zome_result {
                Ok(human_output) => {
                    let agent_key = match get_agent_pub_key(&state) {
                        Ok(k) => k,
                        Err(e) => {
                            warn!("Failed to get agent_pub_key: {}", e);
                            return json_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                &ErrorResponse {
                                    error: "Failed to get agent identity".into(),
                                    code: Some("AGENT_KEY_ERROR".into()),
                                },
                            );
                        }
                    };
                    let profile = HumanProfileResponse {
                        id: human_output.human.id.clone(),
                        display_name: human_output.human.display_name,
                        bio: human_output.human.bio,
                        affinities: human_output.human.affinities,
                        profile_reach: human_output.human.profile_reach,
                        location: human_output.human.location,
                        created_at: human_output.human.created_at,
                        updated_at: human_output.human.updated_at,
                    };
                    (human_output.human.id, agent_key, Some(profile), None)
                }
                Err(e) => {
                    let err_str = e.to_string();
                    // Recovery: operator profile already exists (DB cleared, conductor intact)
                    if err_str.contains("Agent already has a Human profile") {
                        warn!("Operator already has Human profile — recovering for DB re-registration");
                        match call_get_my_human(&state).await {
                            Ok(Some(existing)) => {
                                let agent_key = match get_agent_pub_key(&state) {
                                    Ok(k) => k,
                                    Err(e2) => {
                                        warn!("Failed to get agent_pub_key during recovery: {}", e2);
                                        return json_response(
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                            &ErrorResponse {
                                                error: "Failed to get agent identity during recovery".into(),
                                                code: Some("AGENT_KEY_ERROR".into()),
                                            },
                                        );
                                    }
                                };
                                let profile = HumanProfileResponse {
                                    id: existing.human.id.clone(),
                                    display_name: existing.human.display_name,
                                    bio: existing.human.bio,
                                    affinities: existing.human.affinities,
                                    profile_reach: existing.human.profile_reach,
                                    location: existing.human.location,
                                    created_at: existing.human.created_at,
                                    updated_at: existing.human.updated_at,
                                };
                                (existing.human.id, agent_key, Some(profile), None)
                            }
                            _ => {
                                warn!("Failed to recover operator profile: {}", e);
                                return json_response(
                                    StatusCode::SERVICE_UNAVAILABLE,
                                    &ErrorResponse {
                                        error: format!("Failed to create Holochain identity: {e}"),
                                        code: Some("IDENTITY_CREATION_FAILED".into()),
                                    },
                                );
                            }
                        }
                    } else {
                        warn!("Failed to create operator identity: {}", e);
                        return json_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            &ErrorResponse {
                                error: format!("Failed to create Holochain identity: {e}"),
                                code: Some("IDENTITY_CREATION_FAILED".into()),
                            },
                        );
                    }
                }
            }
        }

        // =====================================================================
        // HOSTED: Provision first, then create_human on provisioned conductor
        // =====================================================================
        "hosted" => {
            // Step 1: Provision agent on operator's conductor
            let provisioned = if let Some(registry) = &state.conductor_registry {
                let provisioner = AgentProvisioner::new(Arc::clone(registry))
                    .with_app_id(state.args.installed_app_id.clone())
                    .with_bundle_path(state.args.happ_bundle_path.clone());
                match provisioner.provision_agent(&body.identifier).await {
                    Ok(p) => {
                        info!(
                            conductor = %p.conductor_id,
                            agent = %p.agent_pub_key,
                            "Hosted agent provisioned on operator's conductor"
                        );
                        p
                    }
                    Err(e) => {
                        error!("Hosted agent provisioning failed: {}", e);
                        return json_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            &ErrorResponse {
                                error: format!("Agent provisioning failed: {e}"),
                                code: Some("PROVISIONING_FAILED".into()),
                            },
                        );
                    }
                }
            } else {
                return json_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &ErrorResponse {
                        error: "Conductor registry not available".into(),
                        code: Some("NO_REGISTRY".into()),
                    },
                );
            };

            // Step 2: Create Human on the provisioned conductor
            let generated_human_id = uuid::Uuid::new_v4().to_string();
            let zome_result = call_create_human_on_conductor(
                &provisioned.conductor_url,
                &provisioned.installed_app_id,
                CreateHumanInput {
                    id: generated_human_id.clone(),
                    display_name: display_name.clone(),
                    bio: body.bio.clone(),
                    affinities: body.affinities.clone(),
                    profile_reach: body.profile_reach.clone(),
                    location: body.location.clone(),
                },
            )
            .await;

            match zome_result {
                Ok(human_output) => {
                    let profile = HumanProfileResponse {
                        id: human_output.human.id.clone(),
                        display_name: human_output.human.display_name,
                        bio: human_output.human.bio,
                        affinities: human_output.human.affinities,
                        profile_reach: human_output.human.profile_reach,
                        location: human_output.human.location,
                        created_at: human_output.human.created_at,
                        updated_at: human_output.human.updated_at,
                    };
                    (
                        human_output.human.id,
                        provisioned.agent_pub_key.clone(),
                        Some(profile),
                        Some(provisioned),
                    )
                }
                Err(e) => {
                    let err_str = e.to_string();
                    // Recovery: hosted identity already exists (re-seeding)
                    if err_str.contains("Agent already has a Human profile") {
                        warn!(
                            identifier = %body.identifier,
                            "Hosted agent already has Human profile — recovering"
                        );
                        // Create a temporary ZomeCaller to call get_my_human on the provisioned conductor
                        let admin_url = crate::derive_admin_url_from_app(&provisioned.conductor_url);
                        let caller = crate::services::ZomeCaller::new(
                            &admin_url,
                            &provisioned.conductor_url,
                            &provisioned.installed_app_id,
                        );
                        match caller.call::<(), Option<HumanOutput>>("imagodei", "imagodei", "get_my_human", &()).await {
                            Ok(Some(existing)) => {
                                let profile = HumanProfileResponse {
                                    id: existing.human.id.clone(),
                                    display_name: existing.human.display_name,
                                    bio: existing.human.bio,
                                    affinities: existing.human.affinities,
                                    profile_reach: existing.human.profile_reach,
                                    location: existing.human.location,
                                    created_at: existing.human.created_at,
                                    updated_at: existing.human.updated_at,
                                };
                                (
                                    existing.human.id,
                                    provisioned.agent_pub_key.clone(),
                                    Some(profile),
                                    Some(provisioned),
                                )
                            }
                            _ => {
                                return json_response(
                                    StatusCode::SERVICE_UNAVAILABLE,
                                    &ErrorResponse {
                                        error: format!("Failed to create hosted identity: {e}"),
                                        code: Some("IDENTITY_CREATION_FAILED".into()),
                                    },
                                );
                            }
                        }
                    } else {
                        warn!("Failed to create hosted identity: {}", e);
                        return json_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            &ErrorResponse {
                                error: format!("Failed to create hosted identity: {e}"),
                                code: Some("IDENTITY_CREATION_FAILED".into()),
                            },
                        );
                    }
                }
            }
        }

        // =====================================================================
        // NODE / DEVICE: DB record only — identity exists on their conductor
        // =====================================================================
        "node" | "device" => {
            // Find their existing app across all conductors
            let provisioned = if let Some(registry) = &state.conductor_registry {
                let provisioner = AgentProvisioner::new(Arc::clone(registry))
                    .with_app_id(state.args.installed_app_id.clone())
                    .with_bundle_path(state.args.happ_bundle_path.clone());
                match provisioner.provision_agent(&body.identifier).await {
                    Ok(p) => {
                        info!(
                            conductor = %p.conductor_id,
                            agent = %p.agent_pub_key,
                            phase = %agency_phase,
                            "Located existing {} conductor for ingress registration",
                            agency_phase
                        );
                        p
                    }
                    Err(e) => {
                        error!(
                            phase = %agency_phase,
                            "Failed to locate conductor for {} registration: {}",
                            agency_phase, e
                        );
                        return json_response(
                            StatusCode::SERVICE_UNAVAILABLE,
                            &ErrorResponse {
                                error: format!("Conductor not found for {agency_phase} registration: {e}"),
                                code: Some("CONDUCTOR_NOT_FOUND".into()),
                            },
                        );
                    }
                }
            } else {
                return json_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &ErrorResponse {
                        error: "Conductor registry not available".into(),
                        code: Some("NO_REGISTRY".into()),
                    },
                );
            };

            // No create_human call — identity already exists on their conductor.
            // Use human_id and agent_pub_key provided in the request body,
            // or fall back to the provisioned agent key with a placeholder human_id.
            let human_id = if !body.human_id.is_empty() {
                body.human_id.clone()
            } else {
                // The seeder should provide human_id for node/device humans.
                // If not provided, generate a deterministic one from identifier.
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(body.identifier.as_bytes());
                hasher.update(b"agency_phase_human_id");
                let hash = hasher.finalize();
                format!("uhCHk{}", hex::encode(&hash[..20]))
            };

            (
                human_id,
                provisioned.agent_pub_key.clone(),
                None, // No profile — doorway doesn't own their identity
                Some(provisioned),
            )
        }

        // Unknown phase
        _ => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &ErrorResponse {
                    error: format!("Unknown agency phase: '{agency_phase}'. Valid: doorway, node, device, hosted"),
                    code: Some("INVALID_AGENCY_PHASE".into()),
                },
            );
        }
    };
```

The rest of `handle_register` (password validation, JWT, MongoDB insert, etc.) remains unchanged — it already works with the `(human_id, agent_pub_key, profile, provisioned)` tuple. Remove the old separate provisioning block (lines ~817-851) since provisioning is now inside each branch.

- [ ] **Step 5: Update the import for call_create_human_on_conductor**

In `doorway/doorway-service/src/routes/auth_routes.rs`, update the import:

```rust
use crate::routes::zome_helpers::{
    call_create_human, call_create_human_on_conductor, call_get_my_human,
    get_agent_pub_key, CreateHumanInput,
};
```

Also add the `HumanOutput` import (needed for the hosted recovery path):

```rust
use crate::routes::zome_helpers::{
    call_create_human, call_create_human_on_conductor, call_get_my_human,
    get_agent_pub_key, CreateHumanInput, HumanOutput,
};
```

- [ ] **Step 6: Run cargo fmt**

Run: `cd doorway/doorway-service && cargo fmt`

- [ ] **Step 7: Verify it compiles**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 8: Run existing tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: All 396+ tests pass

- [ ] **Step 9: Commit**

```bash
git add doorway/doorway-service/src/routes/auth_routes.rs \
        doorway/doorway-service/src/routes/zome_helpers.rs \
        doorway/doorway-service/src/main.rs
git commit -m "feat(doorway): branch handle_register on agency phase

Doorway phase: existing ZomeCaller flow (operator's conductor).
Hosted phase: provision first, then create_human on provisioned conductor.
Node/device phase: DB record only — identity exists on their conductor."
```

---

### Task 4: Update seeder to send agencyPhase and skip visitors

**Files:**
- Modify: `genesis/seeder/src/seed-humans.ts`

- [ ] **Step 1: Add agencyPhase to HumansJsonHuman interface**

In `genesis/seeder/src/seed-humans.ts`, update the interface (around line 25):

```typescript
interface HumansJsonHuman {
  id: string;
  displayName: string;
  bio: string;
  category: string;
  profileReach: string;
  affinities?: string[];
  agencyPhase?: string;
}
```

- [ ] **Step 2: Add agencyPhase to the register payload**

In `registerHuman` (around line 94), add the field to the body:

```typescript
  const body: Record<string, unknown> = {
    identifier: creds.identifier,
    password: creds.password,
    displayName: creds.displayName,
    bio: human.bio,
    affinities: human.affinities ?? [],
    profileReach: human.profileReach,
    agencyPhase: human.agencyPhase ?? 'visitor',
  };
```

- [ ] **Step 3: Skip visitors in main loop**

In `main()`, filter out visitors before registration (replace the sorting block around line 198-208):

```typescript
  // Filter to active humans (those with a non-visitor agencyPhase)
  const active = humansJson.humans.filter(
    h => h.agencyPhase && h.agencyPhase !== 'visitor'
  );

  console.log('=== Seed Humans ===\n');
  console.log(`Doorway:  ${doorwayUrl}`);
  console.log(`Humans:   ${active.length} active of ${humansJson.humans.length} total`);
  console.log(`Admin key: ${adminBootstrapKey ? 'provided' : 'not set'}`);
  console.log('');

  // Sort: doorway operator first, then node, device, hosted
  const phaseOrder: Record<string, number> = {
    doorway: 0,
    node: 1,
    device: 2,
    hosted: 3,
  };
  const sorted = [...active].sort((a, b) => {
    const aOrder = phaseOrder[a.agencyPhase ?? 'hosted'] ?? 99;
    const bOrder = phaseOrder[b.agencyPhase ?? 'hosted'] ?? 99;
    return aOrder - bOrder;
  });
```

- [ ] **Step 4: Add phase label to output**

Update the logging in the `for` loop (around line 216-219):

```typescript
    const icon =
      result.result === 'registered' ? '+' : result.result === 'exists' ? '=' : 'X';
    const phase = human.agencyPhase ?? 'visitor';
    const suffix = result.error ? ` (${result.error})` : '';
    console.log(
      `  [${icon}] ${result.displayName.padEnd(16)} ${result.identifier.padEnd(40)} ${phase}${suffix}`
    );
```

- [ ] **Step 5: Verify TypeScript compiles**

Run: `cd genesis/seeder && npx tsx --eval "import './src/seed-humans.ts'" 2>&1 | head -5`
(This will fail at runtime since it can't reach doorway, but should compile)

- [ ] **Step 6: Commit**

```bash
git add genesis/seeder/src/seed-humans.ts
git commit -m "feat(seeder): send agencyPhase in register payload, skip visitors

Only active humans (with agencyPhase set and != visitor) are seeded.
Registration order: doorway first, then node, device, hosted."
```

---

### Task 5: Add direct conductor seeding for node/device humans

Node and device humans need their Human profiles created directly on their conductors, *before* registering with doorway. The seeder calls each conductor's app interface directly.

**Files:**
- Create: `genesis/seeder/src/seed-conductor-identities.ts`
- Modify: `genesis/seeder/package.json` (add script)

- [ ] **Step 1: Create seed-conductor-identities.ts**

This script reads humans.json, finds node/device humans, and creates their Human profiles by calling `create_human` directly on each conductor via the Holochain admin/app WebSocket APIs.

```typescript
/**
 * Seed Conductor Identities — create Human profiles directly on node/device conductors.
 *
 * For node and device humans, identity is created on THEIR conductor, not through doorway.
 * This models the real-world flow: the human sets up identity on their own device/node,
 * then registers with a doorway for ingress/recovery.
 *
 * Environment variables:
 *   CONDUCTOR_URLS    Comma-separated conductor app URLs (same as doorway's CONDUCTOR_URLS)
 *                     e.g. "ws://elohim-matthew-alpha:4445,ws://elohim-adam-alpha:4445,..."
 *   INSTALLED_APP_ID  Holochain app ID (default: "elohim")
 *
 * Must run BEFORE seed-humans.ts (which registers with doorway for ingress).
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

interface HumansJsonHuman {
  id: string;
  displayName: string;
  bio: string;
  category: string;
  profileReach: string;
  affinities?: string[];
  agencyPhase?: string;
}

interface HumansJson {
  humans: HumansJsonHuman[];
}

/**
 * Derive admin URL from app URL (port - 1, socat convention).
 */
function deriveAdminUrl(appUrl: string): string {
  const match = appUrl.match(/^(wss?:\/\/[^:]+):(\d+)$/);
  if (!match) return appUrl.replace(/:\d+$/, ':4444');
  const port = parseInt(match[2], 10);
  return `${match[1]}:${port - 1}`;
}

/**
 * Call create_human on a specific conductor using holochain-client.
 *
 * NOTE: This uses the conductor's admin and app WebSocket interfaces directly.
 * The holochain-client package must be available. If not, this falls back to
 * a raw WebSocket approach.
 */
async function createHumanOnConductor(
  conductorAppUrl: string,
  installedAppId: string,
  human: HumansJsonHuman
): Promise<{ humanId: string; agentPubKey: string } | { error: string }> {
  const adminUrl = deriveAdminUrl(conductorAppUrl);

  try {
    // Dynamic import — holochain-client may not be installed in all environments
    const { AdminWebsocket, AppWebsocket } = await import('@holochain/client');

    // Connect to admin interface
    const adminWs = await AdminWebsocket.connect(adminUrl);

    // Find the installed app
    const apps = await adminWs.listApps({});
    const app = apps.find(a => a.installed_app_id.startsWith(installedAppId));
    if (!app) {
      return { error: `No app starting with '${installedAppId}' found on ${conductorAppUrl}` };
    }

    // Get agent pub key
    const agentPubKey = Buffer.from(app.agent_pub_key).toString('base64');

    // Authorize signing credentials and connect app interface
    const { ClientAgentSigner } = await import('@holochain/client');
    const signer = new ClientAgentSigner();

    for (const [_roleName, cells] of Object.entries(app.cell_info)) {
      for (const cell of cells as any[]) {
        if (cell.provisioned) {
          const creds = await adminWs.authorizeSigningCredentials(cell.provisioned.cell_id);
          signer.addCredentials(cell.provisioned.cell_id, creds);
        }
      }
    }

    const token = await adminWs.issueAppAuthToken({
      installed_app_id: app.installed_app_id,
      expiry_seconds: 60,
      single_use: false,
    });

    const appWs = await AppWebsocket.connect(conductorAppUrl, token.token, signer);

    // Check if Human already exists
    const existing = await appWs.callZome({
      role_name: 'imagodei',
      zome_name: 'imagodei',
      fn_name: 'get_my_human',
      payload: null,
    });

    if (existing) {
      console.log(`  [=] ${human.displayName.padEnd(16)} already has Human on ${conductorAppUrl}`);
      return { humanId: existing.human.id, agentPubKey };
    }

    // Create the Human
    const humanId = crypto.randomUUID();
    const result = await appWs.callZome({
      role_name: 'imagodei',
      zome_name: 'imagodei',
      fn_name: 'create_human',
      payload: {
        id: humanId,
        display_name: human.displayName,
        bio: human.bio ?? null,
        affinities: human.affinities ?? [],
        profile_reach: human.profileReach,
        location: null,
      },
    });

    console.log(`  [+] ${human.displayName.padEnd(16)} Human created on ${conductorAppUrl}`);
    return { humanId: result.human.id, agentPubKey };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { error: `${conductorAppUrl}: ${msg}` };
  }
}

async function main(): Promise<void> {
  const conductorUrlsRaw = process.env.CONDUCTOR_URLS;
  if (!conductorUrlsRaw) {
    console.error('CONDUCTOR_URLS env var required (comma-separated conductor app URLs)');
    process.exit(1);
  }
  const conductorUrls = conductorUrlsRaw.split(',').map(s => s.trim()).filter(Boolean);
  const installedAppId = process.env.INSTALLED_APP_ID || 'elohim';

  // Load humans.json
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const jsonPath = resolve(__dirname, '../../docs/humans/humans.json');
  const humansJson: HumansJson = JSON.parse(readFileSync(jsonPath, 'utf-8'));

  // Filter to node/device humans only
  const peerHumans = humansJson.humans.filter(
    h => h.agencyPhase === 'node' || h.agencyPhase === 'device'
  );

  console.log('=== Seed Conductor Identities ===\n');
  console.log(`Conductors: ${conductorUrls.length}`);
  console.log(`Node/device humans: ${peerHumans.length}`);
  console.log('');

  // For each peer human, try to find their conductor and create identity
  // Convention: conductor URLs are ordered to match the humans they serve.
  // The doorway operator's conductor is first (conductor-0), then node/device conductors.
  // We try each conductor to find the one with a matching app for this human's identifier.
  let failures = 0;
  for (const human of peerHumans) {
    let created = false;
    for (const url of conductorUrls) {
      const result = await createHumanOnConductor(url, installedAppId, human);
      if ('humanId' in result) {
        created = true;
        break;
      }
      // If error is "no app found", try next conductor
      if (result.error.includes('No app starting with')) {
        continue;
      }
      // Other errors: log and try next
      console.error(`  [!] ${human.displayName}: ${result.error}`);
    }
    if (!created) {
      console.error(`  [X] ${human.displayName}: no conductor found with matching app`);
      failures++;
    }
  }

  console.log('');
  if (failures > 0) {
    console.error(`${failures} human(s) failed to create identity on their conductor`);
    process.exit(1);
  }
  console.log('All node/device identities created successfully');
}

main();
```

- [ ] **Step 2: Add script to package.json**

In `genesis/seeder/package.json`, add to the `"scripts"` section:

```json
"seed:conductors": "npx tsx src/seed-conductor-identities.ts"
```

- [ ] **Step 3: Verify it compiles**

Run: `cd genesis/seeder && npx tsx --eval "import './src/seed-conductor-identities.ts'" 2>&1 | head -5`

- [ ] **Step 4: Commit**

```bash
git add genesis/seeder/src/seed-conductor-identities.ts genesis/seeder/package.json
git commit -m "feat(seeder): add direct conductor identity seeding for node/device humans

Node and device humans get their Human profiles created directly on their
conductor before registering with doorway for ingress. Models the real-world
flow: set up identity on own device/node, then register with doorway."
```

---

### Task 6: Update seeding order in Genesis pipeline

The genesis seeder Justfile/scripts need to run conductor identity seeding before human registration.

**Files:**
- Modify: `genesis/seeder/justfile` (or equivalent pipeline script)

- [ ] **Step 1: Find the seeding orchestration file**

Check: `ls genesis/seeder/justfile genesis/seeder/Justfile genesis/Justfile 2>/dev/null`
Also check: `grep -r "seed:humans\|seed-humans" genesis/seeder/package.json genesis/Justfile genesis/seeder/justfile 2>/dev/null`

- [ ] **Step 2: Add seed:conductors before seed:humans**

In the seeding pipeline, add `seed:conductors` as a step that runs before `seed:humans`. The exact location depends on the orchestration file found in step 1. Example for a justfile:

```just
seed-identities:
    cd seeder && CONDUCTOR_URLS={{CONDUCTOR_URLS}} pnpm run seed:conductors

seed-humans: seed-identities
    cd seeder && DOORWAY_URL={{DOORWAY_URL}} pnpm run seed:humans
```

- [ ] **Step 3: Commit**

```bash
git add genesis/seeder/justfile  # or whichever file was modified
git commit -m "feat(genesis): run conductor identity seeding before doorway registration"
```

---

### Task 7: Integration test — full seeding flow

Run the complete seeding pipeline against alpha to verify all 7 humans register correctly.

**Prerequisites:** Eve needs a StatefulSet (`elohim-eve-alpha`) created in K8s before conductor identity seeding. This replaces Frank's active slot. Frank's StatefulSet can remain but won't be seeded. K8s manifest changes are outside this plan's scope.

**Files:** None (manual verification)

- [ ] **Step 1: Verify humans.json has agencyPhase**

Run: `node -e "const h=JSON.parse(require('fs').readFileSync('genesis/docs/humans/humans.json','utf-8')); h.humans.filter(x=>x.agencyPhase).forEach(x=>console.log(x.agencyPhase.padEnd(10), x.displayName))"`

Expected:
```
doorway   Matthew
node      Adam
device    Eve
device    Jessica
device    Pete
device    Terrance
hosted    Nancy
```

- [ ] **Step 2: Build and deploy doorway**

Push to dev, wait for doorway pipeline to build and deploy the new image to alpha.

- [ ] **Step 3: Run conductor identity seeding**

Run: `cd genesis/seeder && CONDUCTOR_URLS="ws://elohim-adam-alpha:4445,ws://elohim-jessica-alpha:4445,ws://elohim-frank-alpha:4445,ws://elohim-pete-alpha:4445,ws://elohim-terrance-alpha:4445" pnpm run seed:conductors`

Expected: All node/device humans show `[+]` or `[=]`

- [ ] **Step 4: Run doorway human seeding**

Run: `cd genesis/seeder && DOORWAY_URL=https://doorway-alpha.elohim.host pnpm run seed:humans`

Expected:
```
  [+] Matthew          matthew.dowell@alpha.elohim.host        doorway
  [+] Adam             adam@test.elohim.host                   node
  [+] Eve              eve@test.elohim.host                    device
  [+] Jessica          jessica@test.elohim.host                device
  [+] Pete             pastor-pete@test.elohim.host            device
  [+] Terrance          terrance@test.elohim.host                device
  [+] Nancy            nancy@test.elohim.host                  hosted
```

All `[+]` (registered) or `[=]` (exists). Zero `[X]` (failed).

- [ ] **Step 5: Verify login works for all 7**

```bash
for id in matthew.dowell@alpha.elohim.host adam@test.elohim.host nancy@test.elohim.host; do
  echo -n "$id: "
  curl -s -o /dev/null -w "%{http_code}" -X POST https://doorway-alpha.elohim.host/auth/login \
    -H "Content-Type: application/json" \
    -d "{\"identifier\":\"$id\",\"password\":\"Test2026!\"}"
  echo
done
```

Expected: All return `200`.

- [ ] **Step 6: Final commit — push all changes**

```bash
git push
```
