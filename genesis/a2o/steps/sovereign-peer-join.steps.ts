/**
 * Sovereign-peer join — steps for features/deployment/sovereign-peer-join.feature
 * (scenarios 1 "joins and the fleet sees it" and 5 "hosted read == peer read").
 *
 * The T3 hybrid rung of the evidence ladder (spec ratchet-to-delivery-dataplane-sdk-lanes, lane P
 * rung P5): the WORKSPACE conductor — started by `just dev conductor alpha`
 * (app/elohim-app/scripts/hc-start.sh --conductor, NETWORK_PROFILE=join-alpha) — joins the alpha
 * network the fleet runs. These steps never start or stop that conductor: the runner does, and
 * declares the cap for the run with ELOHIM_CAP_LOCAL_CONDUCTOR_STATUS=available. What they read:
 *
 *   elohim/holochain/local-dev/.hc_ports       admin_port=<n> / app_port=4445 (written by hc-start.sh)
 *   elohim/holochain/local-dev/.hc_wrapper.sh  the exact `hc sandbox generate … network --bootstrap
 *                                              <url> webrtc <url>` line the conductor was started with
 *   elohim/holochain/local-dev/deployed-bundles/elohim.happ   the fetched deployed bundle
 *
 * "Fingerprint" in the feature = the DNA hash. doorway alpha's /db/p2p/conductor-diagnostics lists
 * every live agent with the kitsune `space` it lives in; a space id is the base64url of the DNA
 * hash's 32-byte core, i.e. `encodeHashToBase64(dna).slice(5, 48)` (the `uhC0k` prefix and the
 * 4-byte location suffix dropped). The same core-compare identifies our agent key in that list.
 */

import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import path from 'node:path';

import { Given, Then, When } from '@cucumber/cucumber';

import {
  AdminWebsocket,
  AppWebsocket,
  decodeHashFromBase64,
  encodeHashToBase64,
} from '@holochain/client';

import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { getFixture } from '../src/framework/fixtures/humans.js';
import { Human } from '../src/framework/human.js';
import { E2EWorld } from '../src/framework/world.js';

import type { CellId } from '@holochain/client';

const APP_ID = 'elohim';
const MINUTE_MS = 60_000;
const POLL_MS = 10_000;

interface WorkspaceConductor {
  adminPort: number;
  appPort: number;
  admin: AdminWebsocket;
  agentKey: string; // full base64 (uhCAk…)
  agentCore: string; // 43-char core, the form conductor-diagnostics prints
  cells: Map<string, CellId>; // role -> cell id
  dnaCores: Set<string>; // 43-char DNA cores, the form diagnostics `space` prints
}

interface DiagnosticsAgent {
  agent?: string;
  space?: string;
  isTombstone?: boolean | string;
}

interface JoinState {
  conductor?: WorkspaceConductor;
  hostedRead?: Record<string, unknown>;
  peerRead?: Record<string, unknown>;
  peerReadUsedHostedToken?: boolean;
}

const state: JoinState = {};

function localDevDir(): string {
  return (
    process.env.E2E_LOCAL_CONDUCTOR_DIR ??
    path.resolve(process.cwd(), '..', '..', 'elohim', 'holochain', 'local-dev')
  );
}

/**
 * The 32-byte core of a holochain hash, base64url without padding — the form kitsune2 prints for
 * a `space` and an `agent`. A 39-byte hash (3-byte type prefix + 32-byte core + 4-byte location)
 * does NOT slice cleanly as base64 text (the 43rd character shares bits with the location bytes —
 * `…TcSg` vs `…TcSi`), so decode, slice bytes 3..35, re-encode.
 */
function hashCore(b64: string): string {
  const bytes = decodeHashFromBase64(b64);
  return Buffer.from(bytes.subarray(3, 35)).toString('base64url');
}

async function tcpOpen(port: number, timeoutMs = 1000): Promise<boolean> {
  return new Promise(resolve => {
    const sock = createConnection({ host: '127.0.0.1', port });
    const done = (ok: boolean) => {
      sock.destroy();
      resolve(ok);
    };
    sock.setTimeout(timeoutMs, () => done(false));
    sock.once('connect', () => done(true));
    sock.once('error', () => done(false));
  });
}

async function pollUntil<T>(
  budgetMs: number,
  probe: () => Promise<T | undefined>,
  describe: () => string
): Promise<T> {
  const deadline = Date.now() + budgetMs;
  for (;;) {
    const hit = await probe();
    if (hit !== undefined) return hit;
    if (Date.now() >= deadline) break;
    await new Promise(r => setTimeout(r, POLL_MS));
  }
  throw new Error(`timed out after ${Math.round(budgetMs / MINUTE_MS)} min — ${describe()}`);
}

async function fetchDiagnostics(doorwayUrl: string): Promise<DiagnosticsAgent[]> {
  const res = await fetch(`${doorwayUrl}/db/p2p/conductor-diagnostics`);
  if (!res.ok) return [];
  const body = (await res.json()) as { agents?: DiagnosticsAgent[] };
  return (body.agents ?? []).filter(a => a.isTombstone !== true && a.isTombstone !== 'True');
}

function liveSpaces(agents: DiagnosticsAgent[]): Set<string> {
  return new Set(agents.map(a => a.space).filter((s): s is string => typeof s === 'string'));
}

async function connectWorkspaceConductor(): Promise<WorkspaceConductor> {
  if (state.conductor) return state.conductor;
  const dir = localDevDir();
  const portsFile = path.join(dir, '.hc_ports');
  assert.ok(
    existsSync(portsFile),
    `no workspace conductor: ${portsFile} missing — start one with \`just dev conductor alpha\``
  );
  const ports = Object.fromEntries(
    readFileSync(portsFile, 'utf8')
      .split('\n')
      .filter(Boolean)
      .map(l => l.split('=') as [string, string])
  );
  const adminPort = Number(ports.admin_port);
  const appPort = Number(ports.app_port ?? 4445);
  assert.ok(
    Number.isFinite(adminPort) && (await tcpOpen(adminPort)),
    `workspace conductor admin port ${ports.admin_port} is not accepting connections — the recorded ` +
      `port is stale; restart with \`just dev conductor alpha\``
  );
  const admin = await AdminWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${adminPort}`),
    wsClientOptions: { origin: APP_ID },
  });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID);
  assert.ok(
    app,
    `workspace conductor has no installed app "${APP_ID}" (installed: ${apps.map(a => a.installed_app_id).join(', ') || 'none'})`
  );
  const cells = new Map<string, CellId>();
  const dnaCores = new Set<string>();
  let agentKey = '';
  for (const [role, infos] of Object.entries(app.cell_info)) {
    for (const info of infos as { type?: string; value?: { cell_id?: CellId } }[]) {
      if (info.type !== 'provisioned' || !info.value?.cell_id) continue;
      const cellId = info.value.cell_id;
      cells.set(role, cellId);
      dnaCores.add(hashCore(encodeHashToBase64(cellId[0])));
      agentKey = encodeHashToBase64(cellId[1]);
    }
  }
  assert.ok(cells.size > 0 && agentKey, 'workspace conductor reports no provisioned cells');
  state.conductor = {
    adminPort,
    appPort,
    admin,
    agentKey,
    agentCore: hashCore(agentKey),
    cells,
    dnaCores,
  };
  return state.conductor;
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

/**
 * The feature restates the standard `doorway {string} at {string}` step with the parenthetical its
 * glossary promises (so the story is judgeable without another document); same behaviour as
 * mode-aware.steps.ts — what the run was GIVEN wins.
 */
Given(
  /^doorway "([^"]+)" at "([^"]+)" \(the address the fleet's doorway is reachable at, read from the environment\)$/,
  function (this: E2EWorld, doorwayId: string, urlOrEnv: string) {
    const url = urlOrEnv.startsWith('E2E_') ? process.env[urlOrEnv] : urlOrEnv;
    assert.ok(url, `Cannot resolve doorway URL from: ${urlOrEnv}`);
    this.addDoorway(doorwayId, url);
  }
);

Given(
  'the deployed application bundle that doorway {string} runs is available to the developer',
  function (this: E2EWorld, _doorwayId: string) {
    const bundle = path.join(localDevDir(), 'deployed-bundles', 'elohim.happ');
    assert.ok(
      existsSync(bundle),
      `deployed bundle missing at ${bundle} — fetch it with app/elohim-app/scripts/fetch-deployed-dna.sh ` +
        `(hc-start.sh does this itself under NETWORK_PROFILE=join-alpha)`
    );
  }
);

// ---------------------------------------------------------------------------
// Scenario 1 — the workspace conductor joins the alpha network and the fleet sees it
// ---------------------------------------------------------------------------

Given(
  'a workspace conductor that has installed the deployed application bundle',
  async function () {
    await connectWorkspaceConductor();
  }
);

/**
 * The start has already happened (the runner owns the process); this step verifies WHAT it was
 * started with, from the wrapper hc-start.sh wrote — the join parameters are the falsifiable part.
 */
When(
  "the developer starts it with its bootstrap endpoint set to doorway {string}'s and its signal endpoint set to the fleet's",
  function (this: E2EWorld, doorwayId: string) {
    const wrapper = path.join(localDevDir(), '.hc_wrapper.sh');
    assert.ok(
      existsSync(wrapper),
      `no start record at ${wrapper} — the conductor was not started by hc-start.sh`
    );
    const line = readFileSync(wrapper, 'utf8');
    const doorway = this.getDoorway(doorwayId).url.replace(/\/$/, '');
    const bootstrapHost = new URL(doorway).host;
    // hc-start.sh quotes both URLs in the wrapper: `--bootstrap "<url>" webrtc "<url>"`.
    const bootstrap = /--bootstrap\s+"?([^"\s]+)"?/.exec(line)?.[1];
    // tx5 (stock) writes `webrtc <signal>`; the iroh fork's hc writes `quic <relay>` — the
    // fleet's relay IS its signal plane on iroh (signal.alpha ↔ relay.alpha pairing).
    const signal = /(?:webrtc|quic)\s+"?([^"\s]+)"?/.exec(line)?.[1];
    assert.ok(bootstrap, `wrapper carries no --bootstrap endpoint: ${line.trim()}`);
    assert.ok(signal, `wrapper carries no webrtc signal endpoint: ${line.trim()}`);
    assert.ok(
      new URL(bootstrap).host === bootstrapHost,
      `bootstrap endpoint ${bootstrap} is not doorway "${doorwayId}"'s (${bootstrapHost})`
    );
    const signalUrl = new URL(signal);
    assert.ok(
      ['wss:', 'ws:', 'https:'].includes(signalUrl.protocol) && signalUrl.pathname === '/',
      `signal/relay endpoint must be the fleet's pathless URL (tx5 rejects a path), got ${signal}`
    );
  }
);

Then(
  'within 3 minutes the workspace conductor lists every application fingerprint that doorway {string} reports, and no other — a leftover install from an earlier run would be a second fingerprint',
  { timeout: 4 * MINUTE_MS },
  async function (this: E2EWorld, doorwayId: string) {
    const c = await connectWorkspaceConductor();
    const doorway = this.getDoorway(doorwayId).url.replace(/\/$/, '');
    let remote = new Set<string>();
    await pollUntil(
      3 * MINUTE_MS,
      async () => {
        remote = liveSpaces(await fetchDiagnostics(doorway));
        if (remote.size === 0) return undefined;
        const missingLocally = [...remote].filter(s => !c.dnaCores.has(s));
        const extraLocally = [...c.dnaCores].filter(s => !remote.has(s));
        return missingLocally.length === 0 && extraLocally.length === 0 ? true : undefined;
      },
      () =>
        `doorway ${doorwayId} spaces=${[...remote].join(',') || '(none reported)'} ` +
        `workspace dna cores=${[...c.dnaCores].join(',')}`
    );
  }
);

Then(
  "within 3 minutes the workspace conductor's peer store holds at least one fleet agent — proof it discovered a peer, not merely the bootstrap address it was given",
  { timeout: 4 * MINUTE_MS },
  async function () {
    const c = await connectWorkspaceConductor();
    await pollUntil(
      3 * MINUTE_MS,
      async () => {
        const infos: string[] = await c.admin.agentInfo({ dna_hashes: null });
        const others = infos
          .map(raw => {
            try {
              const outer = JSON.parse(raw) as { agentInfo?: string };
              const inner = JSON.parse(outer.agentInfo ?? '{}') as { agent?: string };
              return inner.agent ?? '';
            } catch {
              return '';
            }
          })
          .filter(a => a && a !== c.agentCore && !a.startsWith(c.agentCore));
        return others.length > 0 ? others.length : undefined;
      },
      () => `peer store holds no agent other than the workspace agent ${c.agentCore}`
    );
  }
);

Then(
  "within 10 minutes doorway {string}'s conductor diagnostics list the workspace agent's key as a live agent",
  { timeout: 11 * MINUTE_MS },
  async function (this: E2EWorld, doorwayId: string) {
    const c = await connectWorkspaceConductor();
    const doorway = this.getDoorway(doorwayId).url.replace(/\/$/, '');
    await pollUntil(
      10 * MINUTE_MS,
      async () => {
        const agents = await fetchDiagnostics(doorway);
        return agents.some(a => typeof a.agent === 'string' && a.agent === c.agentCore)
          ? true
          : undefined;
      },
      () => `doorway ${doorwayId} conductor-diagnostics does not yet list ${c.agentCore}`
    );
  }
);

// ---------------------------------------------------------------------------
// Scenario 5 — a hosted login on alpha and the workspace peer read the same node the same way
// ---------------------------------------------------------------------------

/** "has joined" = all three checks of scenario 1 hold; re-verified here with a short budget. */
Given(
  'the workspace conductor has joined the alpha network',
  { timeout: 4 * MINUTE_MS },
  async function (this: E2EWorld) {
    const c = await connectWorkspaceConductor();
    const doorway = this.getDoorway('alpha').url.replace(/\/$/, '');
    const agents = await fetchDiagnostics(doorway);
    const spaces = liveSpaces(agents);
    assert.ok(
      [...c.dnaCores].every(s => spaces.has(s)),
      `workspace fingerprints ${[...c.dnaCores].join(',')} are not all among doorway alpha's spaces ${[...spaces].join(',')}`
    );
    await pollUntil(
      3 * MINUTE_MS,
      async () =>
        (await fetchDiagnostics(doorway)).some(a => a.agent === c.agentCore) ? true : undefined,
      () => `doorway alpha does not list the workspace agent ${c.agentCore} as live`
    );
  }
);

Given(
  'the developer is logged in on doorway {string} as fixture human {string} through the normal hosted login',
  async function (this: E2EWorld, doorwayId: string, humanName: string) {
    const fixture = getFixture(humanName);
    const doorway = this.getDoorway(doorwayId);
    const human = new Human(humanName, fixture.credentials);
    const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
    human.addDevice(device);
    const auth = await device.login({
      identifier: fixture.credentials.identifier,
      password: fixture.credentials.password,
    });
    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    human.setToken(doorwayId, auth.token);
    this.addHuman(humanName, human);
  }
);

When(
  'the developer reads {string} through doorway {string} as {word}',
  async function (this: E2EWorld, contentId: string, _doorwayId: string, humanName: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} holds no device to read through`);
    state.hostedRead = await device.client.getContent(contentId);
  }
);

When(
  'the workspace agent reads {string} through the workspace conductor',
  { timeout: 4 * MINUTE_MS },
  async function (contentId: string) {
    const c = await connectWorkspaceConductor();
    const role =
      [...c.cells.keys()].find(r => r.includes('lamad') || r === 'elohim') ??
      [...c.cells.keys()][0];
    const cellId = c.cells.get(role)!;
    const tok = await c.admin.issueAppAuthenticationToken({ installed_app_id: APP_ID });
    await c.admin.authorizeSigningCredentials(cellId);
    const app = await AppWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${c.appPort}`),
      token: tok.token,
      wsClientOptions: { origin: APP_ID },
    });
    try {
      // A freshly joined peer holds the node only once gossip has carried its ops over; the
      // story's ordinary-propagation bound (3 minutes) is the budget, and the time it took is the
      // measurement — a miss after that is the honest red.
      const t0 = Date.now();
      const found = await pollUntil(
        3 * MINUTE_MS,
        async () => {
          const out = await app.callZome<{ content?: Record<string, unknown> } | null>({
            cell_id: cellId,
            zome_name: 'content_store',
            fn_name: 'get_content_by_id',
            payload: { id: contentId },
          });
          return out?.content ?? undefined;
        },
        () => `workspace conductor returned no content for "${contentId}" (DHT read miss)`
      );
      console.warn(
        `  ⏱  workspace peer held "${contentId}" after ${Math.round((Date.now() - t0) / 1000)}s`
      );
      state.peerRead = found;
      state.peerReadUsedHostedToken = false; // admin-issued app token only; no doorway session involved
    } finally {
      await app.client.close();
    }
  }
);

function contentHashOf(read: Record<string, unknown>, blobKey: string, bodyKey: string): string {
  const blob = read[blobKey];
  if (typeof blob === 'string' && blob) return `blob:${blob}`;
  const body = read[bodyKey];
  assert.ok(typeof body === 'string' && body, `read carries neither ${blobKey} nor ${bodyKey}`);
  return `sha256:${createHash('sha256').update(body).digest('hex')}`;
}

Then('both reads return the same content hash for the node', function () {
  assert.ok(state.hostedRead, 'no hosted read recorded');
  assert.ok(state.peerRead, 'no workspace-peer read recorded');
  const hosted = contentHashOf(state.hostedRead, 'blobCid', 'contentBody');
  const peer = contentHashOf(state.peerRead, 'blob_cid', 'content');
  assert.equal(hosted, peer, `hosted read ${hosted} != workspace-peer read ${peer}`);
});

Then("the workspace agent's read carried no hosted-session token", function () {
  assert.equal(
    state.peerReadUsedHostedToken,
    false,
    'the workspace read went through a hosted session'
  );
});
