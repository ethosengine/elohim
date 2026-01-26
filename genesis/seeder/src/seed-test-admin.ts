/**
 * Seed Test Admin - Create Matthew Dowell as test user and doorway admin.
 *
 * This script bootstraps the primary test user with admin privileges.
 *
 * Usage:
 *   npx tsx src/seed-test-admin.ts [options]
 *
 * Environment variables:
 *   ADMIN_PROXY_URL     Doorway URL (default: https://doorway-dev.elohim.host)
 *   HOLOCHAIN_APP_URL   Holochain app URL (default: ws://localhost:4445)
 *   TEST_ADMIN_PASSWORD Override default test password
 *
 * The script will:
 *   1. Register Matthew Dowell human profile in Holochain
 *   2. Create auth credentials (email/password)
 *   3. Create an Admin-level API key
 *
 * Default credentials (for testing only):
 *   Email: matthew.dowell@elohim.host
 *   Password: TestAdmin2026!
 */

import { AdminWebsocket, AppWebsocket } from '@holochain/client';

// =============================================================================
// Configuration
// =============================================================================

const TEST_USER = {
  email: 'matthew.dowell@elohim.host',
  // IMPORTANT: Change in production! This is for dev/alpha testing only.
  password: process.env.TEST_ADMIN_PASSWORD || 'TestAdmin2026!',
  name: 'Matthew Dowell',
  bio: 'Doorway operator and admin at Ethos Engine. Oversees infrastructure, deployment, and network operations for the Elohim Protocol.',
  affinities: ['infrastructure', 'holochain', 'devops', 'distributed-systems'],
};

const API_KEY_NAME = 'matthew-dowell-admin-key';

// =============================================================================
// Types
// =============================================================================

interface RegisterHumanInput {
  display_name: string;
  bio: string | null;
  affinities: string[];
  profile_reach: string;
  location: string | null;
  email_hash: string | null;
  passkey_credential_id: string | null;
  external_identifiers_json: string;
}

interface HumanOutput {
  id: string;
  display_name: string;
  bio: string | null;
  affinities: string[];
  profile_reach: string;
  location: string | null;
  created_at: string;
  updated_at: string;
}

interface HumanSessionResult {
  agent_pubkey: string;
  human: HumanOutput;
  attestations: Array<{ attestation: { attestation_type: string } }>;
}

interface AuthRegisterResponse {
  token: string;
  humanId: string;
  agentPubKey: string;
  expiresAt: string;
  identifier: string;
}

interface CreateApiKeyResponse {
  key: string;
  name: string;
  permissionLevel: string;
  expiresAt?: string;
}

// =============================================================================
// Helpers
// =============================================================================

function uint8ArrayToBase64(arr: Uint8Array): string {
  return Buffer.from(arr).toString('base64');
}

function getAdminProxyUrl(): string {
  return process.env.ADMIN_PROXY_URL || process.env.DOORWAY_URL || 'https://doorway-dev.elohim.host';
}

function getAdminWsUrl(): string {
  const baseUrl = getAdminProxyUrl();
  // Convert HTTP(S) URL to WebSocket URL for admin connection
  if (baseUrl.startsWith('https://')) {
    return baseUrl.replace('https://', 'wss://') + '/admin';
  } else if (baseUrl.startsWith('http://')) {
    return baseUrl.replace('http://', 'ws://') + '/admin';
  }
  return process.env.HOLOCHAIN_ADMIN_URL || 'ws://localhost:8888/admin';
}

// =============================================================================
// Holochain Connection
// =============================================================================

async function connectToHolochain(): Promise<{
  appWs: AppWebsocket;
  agentPubKey: string;
  cellId: [Uint8Array, Uint8Array];
}> {
  const adminUrl = getAdminWsUrl();
  console.log(`Connecting to Holochain admin at ${adminUrl}...`);

  try {
    const adminWs = await AdminWebsocket.connect({
      url: new URL(adminUrl),
      wsClientOptions: { origin: 'http://localhost' },
    });

    const apps = await adminWs.listApps({});
    const elohimApp = apps.find(a => a.installed_app_id === 'elohim');

    if (!elohimApp) {
      throw new Error('elohim app not found - ensure conductor is running with app installed');
    }

    console.log('Found elohim app');

    // Get app interface port
    const interfaces = await adminWs.listAppInterfaces();
    const appPort = interfaces.length > 0 ? interfaces[0].port : 4445;
    console.log(`Using app interface on port ${appPort}`);

    // Get cell ID
    const cellInfo = Object.values(elohimApp.cell_info)[0];
    if (!cellInfo || cellInfo[0]?.type !== 'provisioned') {
      throw new Error('elohim app cell not provisioned');
    }

    const cellId = cellInfo[0].value.cell_id as [Uint8Array, Uint8Array];
    const agentPubKey = uint8ArrayToBase64(cellId[1]);

    // Authorize signing
    console.log('Authorizing signing credentials...');
    await adminWs.authorizeSigningCredentials(cellId);

    // Issue app auth token
    const token = await adminWs.issueAppAuthenticationToken({
      installed_app_id: 'elohim',
      single_use: false,
      expiry_seconds: 3600,
    });

    // Connect to app interface
    const useProxy = adminUrl.includes('/admin');
    const appUrl = useProxy
      ? adminUrl.replace('/admin', `/app/${appPort}`)
      : `ws://localhost:${appPort}`;

    console.log(`Connecting to app at ${appUrl}...`);
    const appWs = await AppWebsocket.connect({
      url: new URL(appUrl),
      token: token.token,
      wsClientOptions: { origin: 'http://localhost' },
    });

    await adminWs.client.close();
    return { appWs, agentPubKey, cellId };
  } catch (err) {
    throw new Error(`Failed to connect to Holochain: ${err instanceof Error ? err.message : err}`);
  }
}

// =============================================================================
// Registration Functions
// =============================================================================

async function registerHumanInHolochain(
  appWs: AppWebsocket,
  cellId: [Uint8Array, Uint8Array],
  input: RegisterHumanInput
): Promise<HumanSessionResult> {
  console.log('Registering human in Holochain...');

  const result = await appWs.callZome({
    cell_id: cellId,
    zome_name: 'content_store',
    fn_name: 'register_human',
    payload: input,
  });

  return result as HumanSessionResult;
}

async function getExistingHuman(
  appWs: AppWebsocket,
  cellId: [Uint8Array, Uint8Array]
): Promise<HumanSessionResult | null> {
  console.log('Checking for existing human...');

  try {
    const result = await appWs.callZome({
      cell_id: cellId,
      zome_name: 'content_store',
      fn_name: 'get_current_human',
      payload: null,
    });
    return result as HumanSessionResult | null;
  } catch {
    return null;
  }
}

async function registerAuthCredentials(
  adminProxyUrl: string,
  humanId: string,
  agentPubKey: string,
  email: string,
  password: string
): Promise<AuthRegisterResponse> {
  console.log('Registering auth credentials...');

  const response = await fetch(`${adminProxyUrl}/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      humanId,
      agentPubKey,
      identifier: email.toLowerCase(),
      identifierType: 'email',
      password,
    }),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(`Auth registration failed: ${error.error || error.message || response.statusText}`);
  }

  return response.json() as Promise<AuthRegisterResponse>;
}

async function createAdminApiKey(
  adminProxyUrl: string,
  authToken: string,
  keyName: string
): Promise<CreateApiKeyResponse> {
  console.log('Creating admin API key...');

  const response = await fetch(`${adminProxyUrl}/auth/api-keys`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${authToken}`,
    },
    body: JSON.stringify({
      name: keyName,
      permissionLevel: 'ADMIN',
      description: 'Admin API key for Matthew Dowell - doorway operator',
    }),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));
    // API key endpoint might not exist yet - this is optional
    if (response.status === 404) {
      console.log('  API key endpoint not available (404) - skipping');
      return { key: '', name: keyName, permissionLevel: 'ADMIN' };
    }
    throw new Error(`API key creation failed: ${error.error || error.message || response.statusText}`);
  }

  return response.json() as Promise<CreateApiKeyResponse>;
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  console.log('=== Seed Test Admin: Matthew Dowell ===\n');

  const adminProxyUrl = getAdminProxyUrl();
  console.log(`Doorway URL: ${adminProxyUrl}`);
  console.log(`Email:       ${TEST_USER.email}`);
  console.log(`Name:        ${TEST_USER.name}`);
  console.log('');

  try {
    // Step 1: Connect to Holochain
    const { appWs, agentPubKey, cellId } = await connectToHolochain();
    console.log(`Connected. Agent: ${agentPubKey.substring(0, 20)}...`);

    // Step 2: Register human in Holochain
    let holochainResult: HumanSessionResult;

    try {
      const humanInput: RegisterHumanInput = {
        display_name: TEST_USER.name,
        bio: TEST_USER.bio,
        affinities: TEST_USER.affinities,
        profile_reach: 'public',
        location: null,
        email_hash: null,
        passkey_credential_id: null,
        external_identifiers_json: '[]',
      };

      holochainResult = await registerHumanInHolochain(appWs, cellId, humanInput);
      console.log('  Human registered in Holochain');
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      if (errMsg.includes('already registered')) {
        console.log('  Human already exists, retrieving...');
        const existing = await getExistingHuman(appWs, cellId);
        if (!existing) {
          throw new Error('Human exists but could not be retrieved');
        }
        holochainResult = existing;
      } else {
        throw err;
      }
    }

    console.log(`  Human ID: ${holochainResult.human.id}`);

    // Step 3: Register auth credentials
    let authResult: AuthRegisterResponse | null = null;
    try {
      authResult = await registerAuthCredentials(
        adminProxyUrl,
        holochainResult.human.id,
        holochainResult.agent_pubkey,
        TEST_USER.email,
        TEST_USER.password
      );
      console.log('  Auth credentials registered');
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      if (errMsg.includes('already exists') || errMsg.includes('already has')) {
        console.log('  Auth credentials already exist - attempting login...');
        // Try to login to get a token for API key creation
        const loginResponse = await fetch(`${adminProxyUrl}/auth/login`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            identifier: TEST_USER.email.toLowerCase(),
            password: TEST_USER.password,
          }),
        });

        if (loginResponse.ok) {
          authResult = await loginResponse.json();
          console.log('  Logged in successfully');
        } else {
          console.log('  Could not login - password may have changed');
        }
      } else {
        throw err;
      }
    }

    // Step 4: Create admin API key (if we have a token)
    let apiKey: CreateApiKeyResponse | null = null;
    if (authResult?.token) {
      try {
        apiKey = await createAdminApiKey(adminProxyUrl, authResult.token, API_KEY_NAME);
        if (apiKey.key) {
          console.log('  Admin API key created');
        }
      } catch (err) {
        console.log(`  API key creation skipped: ${err instanceof Error ? err.message : err}`);
      }
    }

    // Output summary
    console.log('\n=== Test Admin Seeded Successfully ===\n');
    console.log('Credentials:');
    console.log(`  Email:    ${TEST_USER.email}`);
    console.log(`  Password: ${TEST_USER.password}`);
    console.log('');
    console.log('Identity:');
    console.log(`  Human ID:     ${holochainResult.human.id}`);
    console.log(`  Agent PubKey: ${holochainResult.agent_pubkey.substring(0, 30)}...`);
    console.log('');

    if (apiKey?.key) {
      console.log('Admin API Key:');
      console.log(`  Name: ${apiKey.name}`);
      console.log(`  Key:  ${apiKey.key}`);
      console.log('');
      console.log('Use this API key in headers: X-API-Key: <key>');
    }

    console.log('\nLogin command:');
    console.log(`  curl -X POST ${adminProxyUrl}/auth/login \\`);
    console.log(`    -H "Content-Type: application/json" \\`);
    console.log(`    -d '{"identifier":"${TEST_USER.email}","password":"${TEST_USER.password}"}'`);

    await appWs.client.close();
    process.exit(0);
  } catch (err) {
    console.error('\nSeed failed:', err instanceof Error ? err.message : err);
    process.exit(1);
  }
}

main();
