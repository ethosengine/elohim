/**
 * Seed Humans — register all 27 humans from docs/humans/humans.json via doorway.
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

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

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
}

interface HumansJson {
  humans: HumansJsonHuman[];
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
}

async function registerHuman(
  doorwayUrl: string,
  human: HumansJsonHuman,
  adminBootstrapKey?: string
): Promise<RegisterResult> {
  const creds = deriveCredentials(human);
  const isAdmin = human.id === ADMIN_HUMAN_ID;

  const body: Record<string, unknown> = {
    identifier: creds.identifier,
    password: creds.password,
    displayName: creds.displayName,
    bio: human.bio,
    affinities: human.affinities ?? [],
    profileReach: human.profileReach,
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
      return { displayName: human.displayName, identifier: creds.identifier, result: 'registered' };
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
      return { displayName, identifier: creds.identifier, result: 'exists' };
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
// Main
// =============================================================================

async function main(): Promise<void> {
  const doorwayUrl = (process.env.DOORWAY_URL || 'https://doorway-alpha.elohim.host').replace(
    /\/$/,
    ''
  );
  const adminBootstrapKey = process.env.API_KEY_ADMIN || undefined;

  // Load humans.json
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const jsonPath = resolve(__dirname, '../../docs/humans/humans.json');
  const humansJson: HumansJson = JSON.parse(readFileSync(jsonPath, 'utf-8'));

  console.log('=== Seed Humans ===\n');
  console.log(`Doorway:  ${doorwayUrl}`);
  console.log(`Humans:   ${humansJson.humans.length}`);
  console.log(`Admin key: ${adminBootstrapKey ? 'provided' : 'not set'}`);
  console.log('');

  // Sort: Matthew first, then the rest
  const sorted = [...humansJson.humans].sort((a, b) => {
    if (a.id === ADMIN_HUMAN_ID) return -1;
    if (b.id === ADMIN_HUMAN_ID) return 1;
    return 0;
  });

  const results: RegisterResult[] = [];

  for (const human of sorted) {
    const result = await registerHuman(doorwayUrl, human, adminBootstrapKey);
    results.push(result);

    const icon =
      result.result === 'registered' ? '+' : result.result === 'exists' ? '=' : 'X';
    const suffix = result.error ? ` (${result.error})` : '';
    console.log(`  [${icon}] ${result.displayName.padEnd(16)} ${result.identifier}${suffix}`);
  }

  // Summary
  const registered = results.filter(r => r.result === 'registered').length;
  const exists = results.filter(r => r.result === 'exists').length;
  const failed = results.filter(r => r.result === 'failed').length;

  console.log('');
  console.log(`=== Results: ${registered} registered, ${exists} existing, ${failed} failed ===`);

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
