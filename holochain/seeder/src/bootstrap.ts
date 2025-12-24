/**
 * Bootstrap CLI - Create initial node steward account.
 *
 * This script runs inside the development environment where localhost is accessible.
 * It creates a hosted human profile in Holochain and registers auth credentials.
 *
 * Usage:
 *   npx tsx src/bootstrap.ts --email you@example.com --password yourpass --name "Your Name"
 *
 * Options:
 *   --email      Email address for login
 *   --password   Password (min 8 chars)
 *   --name       Display name
 *   --bio        Optional bio
 *   --affinities Optional comma-separated affinities
 *   --admin-url  Admin proxy URL (default: http://localhost:8888)
 *   --app-url    Holochain app URL (default: ws://localhost:4445)
 */

import { AdminWebsocket, AppWebsocket } from '@holochain/client';

// =============================================================================
// Types
// =============================================================================

interface BootstrapArgs {
  email: string;
  password: string;
  name: string;
  bio?: string;
  affinities?: string[];
  adminProxyUrl: string;
  holochainAppUrl: string;
}

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

// =============================================================================
// Argument Parsing
// =============================================================================

function parseArgs(): BootstrapArgs {
  const args = process.argv.slice(2);
  const parsed: Partial<BootstrapArgs> = {
    adminProxyUrl: process.env.ADMIN_PROXY_URL || 'http://localhost:8888',
    holochainAppUrl: process.env.HOLOCHAIN_APP_URL || 'ws://localhost:4445',
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    const next = args[i + 1];

    switch (arg) {
      case '--email':
      case '-e':
        parsed.email = next;
        i++;
        break;
      case '--password':
      case '-p':
        parsed.password = next;
        i++;
        break;
      case '--name':
      case '-n':
        parsed.name = next;
        i++;
        break;
      case '--bio':
      case '-b':
        parsed.bio = next;
        i++;
        break;
      case '--affinities':
      case '-a':
        parsed.affinities = next?.split(',').map(s => s.trim().toLowerCase());
        i++;
        break;
      case '--admin-url':
        parsed.adminProxyUrl = next;
        i++;
        break;
      case '--app-url':
        parsed.holochainAppUrl = next;
        i++;
        break;
      case '--help':
      case '-h':
        printHelp();
        process.exit(0);
    }
  }

  // Validate required args
  if (!parsed.email) {
    console.error('Error: --email is required');
    printHelp();
    process.exit(1);
  }
  if (!parsed.password) {
    console.error('Error: --password is required');
    printHelp();
    process.exit(1);
  }
  if (parsed.password.length < 8) {
    console.error('Error: password must be at least 8 characters');
    process.exit(1);
  }
  if (!parsed.name) {
    console.error('Error: --name is required');
    printHelp();
    process.exit(1);
  }

  return parsed as BootstrapArgs;
}

function printHelp(): void {
  console.log(`
Bootstrap CLI - Create initial node steward account

Usage:
  npx tsx src/bootstrap.ts --email <email> --password <pass> --name <name> [options]

Required:
  --email, -e      Email address for login
  --password, -p   Password (min 8 chars)
  --name, -n       Display name

Optional:
  --bio, -b        Bio/description
  --affinities, -a Comma-separated interests (e.g., "governance,technology")
  --admin-url      Admin proxy URL (default: http://localhost:8888)
  --app-url        Holochain app URL (default: ws://localhost:4445)
  --help, -h       Show this help

Environment variables:
  ADMIN_PROXY_URL     Override default admin proxy URL
  HOLOCHAIN_APP_URL   Override default Holochain app URL
  HOLOCHAIN_ADMIN_URL Override default Holochain admin URL (default: ws://localhost:8888/admin via proxy)

Examples:
  npx tsx src/bootstrap.ts -e steward@elohim.host -p securepass123 -n "Node Steward"
  npx tsx src/bootstrap.ts --email you@example.com --password mypass123 --name "Your Name" --bio "Running the first node"
`);
}

// =============================================================================
// Holochain Connection
// =============================================================================

async function connectToHolochain(appUrl: string): Promise<{
  appWs: AppWebsocket;
  agentPubKey: string;
  cellId: [Uint8Array, Uint8Array];
}> {
  // Connect to admin via proxy (default) or direct to conductor
  // Proxy route: ws://localhost:8888/admin (recommended - auto-detects conductor port)
  // Direct route: ws://localhost:<admin_port> (requires knowing dynamic port)
  const adminUrl = process.env.HOLOCHAIN_ADMIN_URL || 'ws://localhost:8888/admin';

  console.log(`Connecting to Holochain admin at ${adminUrl}...`);

  // Try to connect to admin to get the app port
  let appPort = 4445;
  try {
    const adminWs = await AdminWebsocket.connect({
      url: new URL(adminUrl),
      wsClientOptions: { origin: 'http://localhost' },
    });
    const apps = await adminWs.listApps({});
    const elohimApp = apps.find(a => a.installed_app_id === 'elohim');

    if (elohimApp) {
      console.log('Found elohim app');

      // Get app interfaces to find the right port
      const interfaces = await adminWs.listAppInterfaces();
      if (interfaces.length > 0) {
        appPort = interfaces[0].port;
        console.log(`Using app interface on port ${appPort}`);
      }

      // Get cell ID and agent pubkey from app info
      const cellInfo = Object.values(elohimApp.cell_info)[0];
      if (cellInfo && cellInfo[0]?.type === 'provisioned') {
        const cellId = cellInfo[0].value.cell_id as [Uint8Array, Uint8Array];
        const agentPubKey = uint8ArrayToBase64(cellId[1]);

        // Authorize signing credentials for the cell
        console.log('Authorizing signing credentials...');
        await adminWs.authorizeSigningCredentials(cellId);

        // Issue app auth token
        const token = await adminWs.issueAppAuthenticationToken({
          installed_app_id: 'elohim',
          single_use: false,
          expiry_seconds: 3600,
        });

        // Connect to app interface via proxy
        // Proxy route: ws://localhost:8888/app/<port> (recommended)
        // Direct route: ws://localhost:<port> (if not using proxy)
        // Token is passed via the connect options, library handles URL encoding
        const useProxy = adminUrl.includes(':8888');
        const appUrl = useProxy
          ? `ws://localhost:8888/app/${appPort}`
          : `ws://localhost:${appPort}`;
        console.log(`Connecting to app at ${appUrl}...`);
        const appWs = await AppWebsocket.connect({
          url: new URL(appUrl),
          token: token.token,
          wsClientOptions: { origin: 'http://localhost' },
        });

        await adminWs.client.close();

        return { appWs, agentPubKey, cellId };
      }
    }

    await adminWs.client.close();
    throw new Error('elohim app not found or not properly installed');
  } catch (err) {
    throw new Error(`Failed to connect to Holochain: ${err instanceof Error ? err.message : err}`);
  }
}

function uint8ArrayToBase64(arr: Uint8Array): string {
  return Buffer.from(arr).toString('base64');
}

// =============================================================================
// Registration
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

  const result = await appWs.callZome({
    cell_id: cellId,
    zome_name: 'content_store',
    fn_name: 'get_current_human',
    payload: null,
  });

  return result as HumanSessionResult | null;
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
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      humanId,
      agentPubKey,
      identifier: email.toLowerCase(),
      identifierType: 'email',
      password,
    }),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(`Auth registration failed: ${error.error || response.statusText}`);
  }

  return response.json() as Promise<AuthRegisterResponse>;
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  console.log('=== Elohim Node Steward Bootstrap ===\n');

  const args = parseArgs();

  console.log(`Email:    ${args.email}`);
  console.log(`Name:     ${args.name}`);
  console.log(`Admin:    ${args.adminProxyUrl}`);
  console.log(`Holochain: ${args.holochainAppUrl}`);
  console.log('');

  try {
    // Step 1: Connect to Holochain
    const { appWs, agentPubKey, cellId } = await connectToHolochain(args.holochainAppUrl);
    console.log(`Connected. Agent: ${agentPubKey.substring(0, 20)}...`);

    // Step 2: Register or get existing human in Holochain
    let holochainResult: HumanSessionResult;
    let isExistingHuman = false;

    try {
      const humanInput: RegisterHumanInput = {
        display_name: args.name,
        bio: args.bio || null,
        affinities: args.affinities || [],
        profile_reach: 'community',
        location: null,
        email_hash: null,  // We store email in admin-proxy, not Holochain
        passkey_credential_id: null,
        external_identifiers_json: '[]',
      };

      holochainResult = await registerHumanInHolochain(appWs, cellId, humanInput);
      console.log(`Holochain registration successful`);
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      if (errMsg.includes('already registered')) {
        console.log('Human already registered, retrieving existing...');
        const existing = await getExistingHuman(appWs, cellId);
        if (!existing) {
          throw new Error('Human exists but could not be retrieved');
        }
        holochainResult = existing;
        isExistingHuman = true;
        console.log('Using existing human profile');
      } else {
        throw err;
      }
    }

    console.log(`  Human ID: ${holochainResult.human.id}`);

    // Step 3: Register auth credentials
    let authResult: AuthRegisterResponse;
    try {
      authResult = await registerAuthCredentials(
        args.adminProxyUrl,
        holochainResult.human.id,
        holochainResult.agent_pubkey,
        args.email,
        args.password
      );
      console.log(`\nAuth registration successful`);
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      if (errMsg.includes('already exists') || errMsg.includes('already has credentials')) {
        console.log('\nAuth credentials already exist for this identity.');
        console.log('If you need to update credentials, reset the auth store first.');

        // Still show summary with existing info
        console.log('\n=== Bootstrap Complete (Existing Account) ===\n');
        console.log(`  Human ID:     ${holochainResult.human.id}`);
        console.log(`  Agent PubKey: ${holochainResult.agent_pubkey.substring(0, 30)}...`);
        console.log(`  Email:        ${args.email}`);
        console.log(`  Display Name: ${holochainResult.human.display_name}`);
        console.log('');
        console.log('You can login via the UI or CLI:');
        console.log(`  curl -X POST ${args.adminProxyUrl}/auth/login \\`);
        console.log(`    -H "Content-Type: application/json" \\`);
        console.log(`    -d '{"identifier":"${args.email}","password":"<your-password>"}'`);

        await appWs.client.close();
        process.exit(0);
      }
      throw err;
    }

    console.log(`  Identifier: ${authResult.identifier}`);
    console.log(`  Expires: ${authResult.expiresAt}`);

    // Step 4: Output summary
    const status = isExistingHuman ? 'Restored' : 'Created';
    console.log(`\n=== Bootstrap Complete (${status}) ===\n`);
    console.log('Your node steward account:');
    console.log(`  Human ID:     ${holochainResult.human.id}`);
    console.log(`  Agent PubKey: ${holochainResult.agent_pubkey.substring(0, 30)}...`);
    console.log(`  Email:        ${args.email}`);
    console.log(`  Display Name: ${holochainResult.human.display_name}`);
    console.log('');
    console.log('You can now login via the UI or CLI:');
    console.log(`  curl -X POST ${args.adminProxyUrl}/auth/login \\`);
    console.log(`    -H "Content-Type: application/json" \\`);
    console.log(`    -d '{"identifier":"${args.email}","password":"<your-password>"}'`);
    console.log('');
    console.log('JWT Token (valid for 24h):');
    console.log(`  ${authResult.token.substring(0, 50)}...`);

    await appWs.client.close();
    process.exit(0);
  } catch (err) {
    console.error('\nBootstrap failed:', err instanceof Error ? err.message : err);
    process.exit(1);
  }
}

main();
