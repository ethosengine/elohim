/**
 * Seed Humans — register all 33 humans from data/humans/humans.json via doorway.
 *
 * The canonical source is genesis/data/humans/*.md (YAML frontmatter per human).
 * humans.json is a generated artifact. If missing, regenerate with:
 *   pnpm --filter holochain-seeder run build:data
 *
 * Credential derivation (MUST match genesis/a2o/src/framework/fixtures/humans.ts):
 *   Matthew (human-matthew-manager): matthew.dowell@alpha.elohim.host / TestAdmin2026!
 *   All others: {slugify(displayName)}@test.elohim.host / Test2026!
 *
 * Environment variables:
 *   DOORWAY_URL     Doorway URL (default: https://doorway-alpha.elohim.host)
 *   API_KEY_ADMIN   Admin bootstrap key for Matthew (optional, promotes to admin)
 *
 * Exit codes:
 *   0 — all humans registered or already exist with matching credentials
 *   1 — one or more humans failed to register
 */

import { existsSync, readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { CreateHumanInputView } from '@elohim/storage-client';

// =============================================================================
// Types (mirrors humans.json schema)
// =============================================================================

interface HumansJsonHuman {
  id: string;
  displayName: string;
  bio: string;
  category: string;
  profileReach: string;
  affinities?: string[];
  agencyPhase?: string;
  /**
   * Household collective id (kind:household in collectives.json). Optional —
   * humans outside a household grouping leave this null. Projects onto
   * humans.household_id in elohim-storage (the resilience-snapshot junction)
   * via the storage create surface — see seedProjectionRow().
   */
  householdId?: string | null;
}

interface HumansJson {
  humans: HumansJsonHuman[];
}

interface DeploymentRecord {
  name: string;
  suspended?: boolean;
}

interface DeploymentsJson {
  humans?: DeploymentRecord[];
}

/**
 * Load suspended-human names from deployments.json. Mirrors the a2o
 * framework's loadDeployments() (genesis/a2o/src/framework/fixtures/humans.ts).
 *
 * Suspended humans (e.g. shem-pinned adam/pete/frank when shem is offline)
 * have dead conductor pods — registration via doorway's node/device branches
 * fails 502/503/WebSocket-closed because the per-human conductor isn't
 * scheduled. Skip them at the seeder boundary so the suspension flag in
 * deployments.json governs both deploy *and* seed.
 *
 * Fail-open on read errors: if deployments.json is unreadable, treat
 * everyone as deployable (preserves prior behavior).
 */
function loadSuspendedNames(): Set<string> {
  try {
    const __dirname = dirname(fileURLToPath(import.meta.url));
    const jsonPath = resolve(__dirname, '../../orchestrator/data/deployments.json');
    const raw = readFileSync(jsonPath, 'utf-8');
    const data = JSON.parse(raw) as DeploymentsJson;
    const suspended = new Set<string>();
    for (const h of data.humans ?? []) {
      if (h.suspended) suspended.add(h.name.toLowerCase());
    }
    return suspended;
  } catch {
    return new Set();
  }
}

// =============================================================================
// Household-mesh hosted cast — the allow-list
// =============================================================================

/**
 * WHY THIS EXISTS (measured 2026-09-11, household run 20260911T0319Z).
 *
 * Registering a human at `POST /auth/register` now PROVISIONS: the doorway
 * installs and enables a 5-cell app for that identifier on a conductor
 * (doorway-service `AgentProvisioner::provision_agent`). Until 60fb28a39 a
 * `--dev-mode` doorway skipped provisioning entirely and every hosted
 * registration rode one shared singleton Human — free, and wrong. `should_provision`
 * removed that path ON PURPOSE, so the standing cast's cost became real:
 * ~157 MB of conductor heap per cell × 5 cells ≈ 786 MB per registered human.
 * Casting the 29 standing personas took matthew's conductor from 1.1 GB to
 * 22.8 GB and the workspace RAM guard shed the whole lane (`exit 143`).
 *
 * The cast was designed under a bug. This is the corrected declaration for a
 * HOUSEHOLD mesh: provision only the humans the a2o household lane actually
 * signs in as. Everyone else is not registered here at all — NOT a silent drop:
 * every skipped persona is printed with the reason. There is deliberately no
 * "register without a cell" middle path: both register branches that a
 * conductor-registry-configured doorway can take (`hosted`, and `node`/`device`)
 * call `provision_agent`, which INSTALLS the app when it finds none
 * (doorway-service/src/conductor/provisioner.rs — "Find (or create)"). A
 * credential with no cell is not something the doorway can currently mint, so
 * the honest household answer is absence, not a fake.
 *
 * DERIVATION — do not edit this list by intuition; re-derive it. A name belongs
 * here iff some scenario under genesis/a2o/features/ that the household lane can
 * run — `@e2e`, act I or untagged, per the `mesh` / `mesh-browser` cucumber
 * profiles — contains a step of the form `human "<Name>" is logged in` (or
 * registers / signs in). Names referenced only as DATA ("Jessica holds a valid
 * record") need a row, not a cell, and are not here. Names that appear only under
 * `@act:ii`/`@act:iii` (the deployed-fleet acts) are not here either — the fleet
 * has the conductors for them; one household does not.
 *
 * The three `prologue-hosted-*` registrants are cast separately by
 * seed-hosted-humans.ts and are part of the same heap budget (they are what
 * hc-mesh.sh's MESH_DOORWAY_MAX_AGENTS default is sized against, together with
 * this list).
 *
 * MESH_HOSTED_CAST=all restores the full standing cast for someone who has the
 * RAM; MESH_HOSTED_CAST=lane forces the allow-list even against a remote doorway.
 * Unset, the allow-list applies only when DOORWAY_URL is loopback — a doorway on
 * localhost IS this host's own household mesh, and a deployed fleet (alpha) keeps
 * the full cast byte-for-byte as before.
 */
const HOUSEHOLD_HOSTED_CAST: readonly string[] = [
  // Everywhere. agencyPhase=doorway, so the operator alone costs NO provisioned
  // cell — the doorway branch calls the singleton ZomeCaller on its own conductor.
  'Matthew',
  // features/auth/fixture-humans.feature "Core family — Matthew's household" (@act:i)
  'Susan',
  'James',
  'Gertrude',
  // features/auth/fixture-humans.feature "Newcomers" (@act:i)
  'Maria',
  'Ronald',
  // features/auth/fixture-humans.feature "Red-team humans can login" (@act:i)
  'Charlie',
  'Sam',
  'Dr. Dolittle',
  // features/content/stewardship-allocation.feature (@act:i)
  'Jessica',
  // features/lms/know-thyself-discovery.feature (@act:i)
  'Terrance',
  // features/elohim/content-reach-negotiation.feature (@act:i @wip) — the lane runs
  // these under A2O_RUN_WIP=1, which is the documented local red loop, so they are
  // cast. Drop them here first if a household ever needs the heap back.
  'Miriam',
  'Ezra',
  'Levi',
];

function hostedCastMode(doorwayUrl: string): 'all' | 'lane' {
  const declared = (process.env.MESH_HOSTED_CAST || '').trim().toLowerCase();
  if (declared === 'all') return 'all';
  if (declared === 'lane') return 'lane';
  let host = '';
  try {
    host = new URL(doorwayUrl).hostname;
  } catch {
    return 'all';
  }
  return host === 'localhost' || host === '127.0.0.1' || host === '::1' || host === '[::1]'
    ? 'lane'
    : 'all';
}

// =============================================================================
// Credential derivation — MUST stay in sync with a2o fixtures/humans.ts
// =============================================================================

const DEFAULT_PASSWORD = 'Test2026!';
const ADMIN_EMAIL = 'matthew.dowell@alpha.elohim.host';
const ADMIN_PASSWORD = 'TestAdmin2026!';
const ADMIN_HUMAN_ID = 'human-matthew-manager';

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
}

function deriveCredentials(human: HumansJsonHuman): {
  identifier: string;
  password: string;
  displayName: string;
} {
  if (human.id === ADMIN_HUMAN_ID) {
    return {
      identifier: ADMIN_EMAIL,
      password: ADMIN_PASSWORD,
      displayName: human.displayName,
    };
  }
  return {
    identifier: `${slugify(human.displayName)}@test.elohim.host`,
    password: DEFAULT_PASSWORD,
    displayName: human.displayName,
  };
}

// =============================================================================
// Registration
// =============================================================================

type Result = 'registered' | 'exists' | 'failed';

interface RegisterResult {
  displayName: string;
  identifier: string;
  result: Result;
  error?: string;
  /**
   * The conductor-minted agent key this account now holds, as returned by
   * `/auth/register` (or by the `/auth/login` verification of an existing
   * account). This is the ONLY truthful source of the key at seed time, and it
   * is what `seedProjectionRow` writes into `humans.agent_pub_key` so the
   * seeded persona and the hosted account are one identity rather than two.
   */
  agentPubKey?: string;
}

async function registerHuman(
  doorwayUrl: string,
  human: HumansJsonHuman,
  adminBootstrapKey?: string
): Promise<RegisterResult> {
  const creds = deriveCredentials(human);
  const isAdmin = human.id === ADMIN_HUMAN_ID;

  const body: Record<string, unknown> = {
    // THE CANONICAL ID. Without it the doorway mints a random UUID (hosted) or
    // a `uhCHk…` account id for this registrant's Human, and the account can
    // never be matched to the seeded persona of the same name — the two-unlinked-
    // identities gap measured on the household mesh 2026-09-13, where James's
    // session resolved to no human row at all. `/auth/register` already PREFERS
    // a caller-supplied `humanId` in both the `hosted` and `doorway` arms
    // (doorway-service/src/routes/auth_routes.rs) and warns loudly when one is
    // absent; the seeder is the caller that knows it.
    humanId: human.id,
    identifier: creds.identifier,
    password: creds.password,
    displayName: creds.displayName,
    bio: human.bio,
    affinities: human.affinities ?? [],
    profileReach: human.profileReach,
    agencyPhase: human.agencyPhase ?? 'hosted',
  };

  if (isAdmin && adminBootstrapKey) {
    body.adminBootstrapKey = adminBootstrapKey;
  }

  try {
    const res = await fetch(`${doorwayUrl}/auth/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (res.ok) {
      const auth = (await res.json().catch(() => ({}))) as { agentPubKey?: string };
      return {
        displayName: human.displayName,
        identifier: creds.identifier,
        result: 'registered',
        agentPubKey: auth.agentPubKey,
      };
    }

    // 409 Conflict — already registered, verify credentials match via login
    if (res.status === 409) {
      return await verifyExisting(doorwayUrl, creds, human.displayName);
    }

    // 503 with "Agent already has a Human profile" means the Holochain DHT has the identity
    // but doorway didn't return 409 (e.g. doorway DB was cleared but conductor wasn't reset).
    // Treat as "exists" and verify via login.
    const errorText = await res.text();
    if (res.status === 503 && errorText.includes('Agent already has a Human profile')) {
      return await verifyExisting(doorwayUrl, creds, human.displayName);
    }
    return {
      displayName: human.displayName,
      identifier: creds.identifier,
      result: 'failed',
      error: `HTTP ${res.status}: ${errorText}`,
    };
  } catch (err) {
    return {
      displayName: human.displayName,
      identifier: creds.identifier,
      result: 'failed',
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

async function verifyExisting(
  doorwayUrl: string,
  creds: { identifier: string; password: string; displayName: string },
  displayName: string
): Promise<RegisterResult> {
  try {
    const loginRes = await fetch(`${doorwayUrl}/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        identifier: creds.identifier,
        password: creds.password,
      }),
    });

    if (loginRes.ok) {
      const auth = (await loginRes.json().catch(() => ({}))) as { agentPubKey?: string };
      return {
        displayName,
        identifier: creds.identifier,
        result: 'exists',
        agentPubKey: auth.agentPubKey,
      };
    }

    return {
      displayName,
      identifier: creds.identifier,
      result: 'failed',
      error: `Credential drift: exists but login failed (HTTP ${loginRes.status})`,
    };
  } catch (err) {
    return {
      displayName,
      identifier: creds.identifier,
      result: 'failed',
      error: `Credential verification failed: ${err instanceof Error ? err.message : String(err)}`,
    };
  }
}

// =============================================================================
// Storage projection bridge — humans.household_id junction
// =============================================================================

type BridgeResult = 'created' | 'exists' | 'skipped' | 'failed';

/**
 * Seed the human's elohim-storage projection row through the create surface
 * (`POST /api/v1/identity/register`, `CreateHumanInputView` — transparently
 * proxied by doorway to elohim-storage). This is the short-term explicit
 * bridge that populates `humans.household_id`, the load-bearing
 * resilience-snapshot junction: the DHT humans-replayer is a stub and
 * `household_backfill` only fills already-NULL rows from an external mapping
 * (see elohim-storage/src/api/identity.rs and tests/human_household_create_bridge.rs).
 *
 * `agentPubKey` comes from the registration that just ran for this same human
 * (`RegisterResult.agentPubKey` — what `/auth/register` or the `/auth/login`
 * verification returned). The seed CORPUS still has no truthful key, and
 * inventing one would break the snapshot joins worse than NULL; but the
 * registration one line earlier has just minted the real one, so this row is
 * born bound instead of waiting to be healed. When registration returned no key
 * (an older doorway, a failed provision) this stays null and the NULL-only
 * substrate healers remain the path: the doorway's own registration
 * write-through (`POST /api/v1/identity/heal`, which fires on every subsequent
 * registration for a caller-supplied humanId), the DHT humans-replayer
 * (services/holochain_humans_replayer.rs, currently a stub), and the
 * authenticated in-app registration flow (identity-api.service.ts).
 *
 * Idempotency: the create surface INSERTs — a row that already exists (e.g.
 * a re-run against a non-wiped projection DB) is reported as 'exists' and
 * left untouched; its household_id is NOT healed here (that is the
 * household_backfill / replayer's job). A fresh seed pass gets the value.
 */
async function seedProjectionRow(
  doorwayUrl: string,
  human: HumansJsonHuman,
  agentPubKey?: string
): Promise<{ result: BridgeResult; error?: string }> {
  if (!human.householdId) {
    return { result: 'skipped' };
  }

  const body: CreateHumanInputView = {
    id: human.id,
    agentPubKey: agentPubKey && agentPubKey.length > 0 ? agentPubKey : null,
    displayName: human.displayName,
    bio: human.bio ?? null,
    affinities: human.affinities ?? [],
    profileReach: human.profileReach,
    location: null,
    profilePhotoUrl: null,
    householdId: human.householdId,
  };

  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (process.env.DOORWAY_API_KEY) {
    headers['Authorization'] = `Bearer ${process.env.DOORWAY_API_KEY}`;
  }

  try {
    const res = await fetch(`${doorwayUrl}/api/v1/identity/register`, {
      method: 'POST',
      headers,
      body: JSON.stringify(body),
    });

    if (res.ok) {
      return { result: 'created' };
    }

    const errorText = await res.text();
    // Row already present (re-run against a non-wiped projection DB).
    if (
      res.status === 409 ||
      errorText.includes('UNIQUE') ||
      errorText.includes('already')
    ) {
      return { result: 'exists' };
    }
    return { result: 'failed', error: `HTTP ${res.status}: ${errorText.slice(0, 200)}` };
  } catch (err) {
    return { result: 'failed', error: err instanceof Error ? err.message : String(err) };
  }
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const doorwayUrl = (process.env.DOORWAY_URL || 'https://doorway-alpha.elohim.host').replace(
    /\/$/,
    ''
  );
  const adminBootstrapKey = process.env.API_KEY_ADMIN || undefined;

  // Load humans.json (generated from genesis/data/humans/*.md by build-data.ts)
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const jsonPath = resolve(__dirname, '../../data/humans/humans.json');
  if (!existsSync(jsonPath)) {
    console.error(`humans.json not found at ${jsonPath}`);
    console.error('Run `pnpm run build:data` in genesis/seeder to generate it from the markdown sources.');
    process.exit(1);
  }
  const humansJson: HumansJson = JSON.parse(readFileSync(jsonPath, 'utf-8'));

  // Filter to active humans:
  //  - Skip the visitor-category persona (Traveler — intentionally unseeded).
  //  - Skip humans suspended in deployments.json (shem-pinned adam/pete/frank
  //    while shem is decommissioned — their dedicated conductor pods are
  //    Pending and registration via doorway's node/device branches would
  //    fail 502/503/WebSocket-closed).
  //  - Humans with no explicit agencyPhase default to "hosted" at
  //    registration — doorway-managed account, no dedicated infra archetype.
  const suspendedNames = loadSuspendedNames();
  // Household-mesh hosted cast (see HOUSEHOLD_HOSTED_CAST above): on a loopback
  // doorway, registering is PROVISIONING, so the cast is the lane's allow-list.
  const castMode = hostedCastMode(doorwayUrl);
  const castAllowed = new Set(HOUSEHOLD_HOSTED_CAST.map(n => n.toLowerCase()));
  const active = humansJson.humans.filter(h => {
    if (h.category === 'visitor' || h.agencyPhase === 'visitor') return false;
    if (suspendedNames.has(h.displayName.toLowerCase())) return false;
    if (castMode === 'lane' && !castAllowed.has(h.displayName.toLowerCase())) return false;
    return true;
  });
  const skippedSuspended = humansJson.humans.filter(h =>
    suspendedNames.has(h.displayName.toLowerCase())
  );
  const skippedOffCast =
    castMode === 'lane'
      ? humansJson.humans.filter(
          h =>
            h.category !== 'visitor' &&
            h.agencyPhase !== 'visitor' &&
            !suspendedNames.has(h.displayName.toLowerCase()) &&
            !castAllowed.has(h.displayName.toLowerCase())
        )
      : [];

  console.log('=== Seed Humans ===\n');
  console.log(`Doorway:  ${doorwayUrl}`);
  console.log(`Humans:   ${active.length} active of ${humansJson.humans.length} total`);
  if (skippedSuspended.length > 0) {
    console.log(
      `Skipping: ${skippedSuspended.length} suspended (${skippedSuspended.map(h => h.displayName).join(', ')}) — see deployments.json`
    );
  }
  if (castMode === 'lane') {
    console.log(
      `Cast:     household allow-list (${HOUSEHOLD_HOSTED_CAST.length} names) — registering IS provisioning (~786 MB of conductor heap per human, 5 cells at ~157 MB)`
    );
    // Only a registration that PROVISIONS costs heap. agencyPhase=doorway uses the
    // singleton ZomeCaller on the doorway's own conductor and installs nothing, so
    // counting it would overstate the projection by one human.
    const provisioning = active.filter(h => (h.agencyPhase ?? 'hosted') !== 'doorway');
    console.log(
      `          projected conductor heap: ${provisioning.length} provisioning registrations x 786 MB ~= ${((provisioning.length * 786) / 1024).toFixed(1)} GB` +
        ` (+3 for the prologue-hosted-* registrants; ${active.length - provisioning.length} agencyPhase=doorway registration(s) provision nothing)`
    );
    if (skippedOffCast.length > 0) {
      console.log(
        `Skipping: ${skippedOffCast.length} off-cast — no household-lane scenario signs in as them, and a registration here would install a 5-cell app for a persona nothing exercises:`
      );
      console.log(`          ${skippedOffCast.map(h => h.displayName).join(', ')}`);
      console.log(
        '          MESH_HOSTED_CAST=all registers the full standing cast (needs ~23 GB of conductor heap for 29).'
      );
    }
  }
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

  const results: RegisterResult[] = [];
  const bridgeFailures: Array<{ displayName: string; error: string }> = [];
  let bridgeCreated = 0;
  let bridgeExists = 0;

  for (const human of sorted) {
    const result = await registerHuman(doorwayUrl, human, adminBootstrapKey);
    results.push(result);

    // Storage projection bridge: carry householdId through the create
    // surface so the resilience-snapshot junction is populated. Runs only
    // when registration didn't hard-fail (the human is live/known).
    let bridgeSuffix = '';
    if (result.result !== 'failed') {
      const bridge = await seedProjectionRow(doorwayUrl, human, result.agentPubKey);
      if (bridge.result === 'created') {
        bridgeCreated++;
        bridgeSuffix = ` household=${human.householdId}`;
      } else if (bridge.result === 'exists') {
        bridgeExists++;
        // Not healed HERE — but the doorway's registration write-through
        // (`POST /api/v1/identity/heal`) has already run for this human on the
        // line above, and it fills a NULL agent_pub_key without clobbering a
        // set one. A re-run against a non-wiped projection DB therefore still
        // converges on the binding.
        bridgeSuffix = ` household=${human.householdId} (projection row exists — key bound at registration)`;
      } else if (bridge.result === 'failed') {
        bridgeFailures.push({ displayName: human.displayName, error: bridge.error ?? 'unknown' });
        bridgeSuffix = ` household=${human.householdId} BRIDGE-FAILED`;
      }
    }

    const icon =
      result.result === 'registered' ? '+' : result.result === 'exists' ? '=' : 'X';
    const phase = human.agencyPhase ?? 'hosted';
    const suffix = result.error ? ` (${result.error})` : '';
    console.log(
      `  [${icon}] ${result.displayName.padEnd(16)} ${result.identifier.padEnd(40)} ${phase}${suffix}${bridgeSuffix}`
    );
  }

  // Summary
  const registered = results.filter(r => r.result === 'registered').length;
  const exists = results.filter(r => r.result === 'exists').length;
  const failed = results.filter(r => r.result === 'failed').length;

  console.log('');
  console.log(`=== Results: ${registered} registered, ${exists} existing, ${failed} failed ===`);
  console.log(
    `=== Household bridge: ${bridgeCreated} projection rows created, ${bridgeExists} already present, ${bridgeFailures.length} failed ===`
  );

  if (bridgeFailures.length > 0) {
    // Loud but soft: registration remains the gate for the exit code; the
    // household junction bridge is best-effort against older deployed
    // doorway/storage that may not carry the create-surface fields yet.
    console.warn('\nHousehold-bridge failures (humans.household_id NOT populated):');
    for (const f of bridgeFailures) {
      console.warn(`  ! ${f.displayName}: ${f.error}`);
    }
  }

  if (failed > 0) {
    console.error('\nFailed humans:');
    for (const r of results.filter(r => r.result === 'failed')) {
      console.error(`  ${r.displayName}: ${r.error}`);
    }
    process.exit(1);
  }

  process.exit(0);
}

main();
