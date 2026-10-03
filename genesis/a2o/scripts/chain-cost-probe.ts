/**
 * chain-cost-probe — does a zome call's cost grow with the caller's source-chain length?
 *
 * The measuring half of `zome-call-cost-bounded` on a DISPOSABLE mesh. It grows one
 * peer's lamad chain with the same three-action write the doorway health attestation
 * makes (`content_store::issue_attestation`), and at each checkpoint times a call that
 * does no host read at all (`export_schema_version`). Whatever that call costs is the
 * conductor's own per-call overhead: there is nothing in the zome function to blame.
 *
 * Never point it at a household whose identities matter: it writes thousands of
 * device-health attestations to the target agent's chain.
 *
 *   node --import tsx scripts/chain-cost-probe.ts time  --peer matthew:4444:4445 --calls 50
 *   node --import tsx scripts/chain-cost-probe.ts install-control --peer matthew:4444:4445 \
 *        --app-id perf-control --happ /abs/elohim.happ     # short-chain agent, same conductor
 *   node --import tsx scripts/chain-cost-probe.ts time  --peer matthew:4444:4445 --app-id perf-control
 *   node --import tsx scripts/chain-cost-probe.ts grow  --peer matthew:4444:4445 \
 *        --writes 15000 --every 500 --calls 30 --out /abs/dir/growth.jsonl
 *
 * `--control role` also times the same no-op-shaped call on another cell of the same
 * conductor (default `infrastructure`, zome/function from --control-fn), so a slow
 * conductor can be told apart from a slow chain.
 */
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

import {
  AdminWebsocket,
  AppWebsocket,
  encodeHashToBase64,
  getSigningCredentials,
  setSigningCredentials,
} from '@holochain/client';

/**
 * Where minted signing credentials are kept between runs. Authorising on every run
 * mints a new capability grant each time, which grows the grant table under
 * measurement (the per-connect pattern the grant backlog entry warns about).
 */
const CREDS_DIR =
  process.env.CHAIN_COST_PROBE_CREDS ??
  '/projects/elohim/genesis/local-dev/perf-deep-dive/probe-creds';

const b64 = (u: Uint8Array) => Buffer.from(u).toString('base64');
const unb64 = (s: string) => new Uint8Array(Buffer.from(s, 'base64'));

async function authorizeOnce(admin: AdminWebsocket, cellId: any, key: string) {
  const file = `${CREDS_DIR}/${key}.json`;
  if (existsSync(file)) {
    const c = JSON.parse(readFileSync(file, 'utf8'));
    setSigningCredentials(cellId, {
      capSecret: unb64(c.capSecret),
      keyPair: {
        ...c.keyPair,
        publicKey: unb64(c.keyPair.publicKey),
        privateKey: unb64(c.keyPair.privateKey),
      },
      signingKey: unb64(c.signingKey),
    } as any);
    return;
  }
  await admin.authorizeSigningCredentials(cellId);
  const c: any = getSigningCredentials(cellId);
  mkdirSync(CREDS_DIR, { recursive: true });
  writeFileSync(
    file,
    JSON.stringify({
      capSecret: b64(c.capSecret),
      keyPair: {
        ...c.keyPair,
        publicKey: b64(c.keyPair.publicKey),
        privateKey: b64(c.keyPair.privateKey),
      },
      signingKey: b64(c.signingKey),
    }),
    { mode: 0o600 }
  );
}

const APP_ID = 'elohim';
const RIDDEN_KIND = 'attestation:device-health';

function arg(name: string, fallback?: string): string | undefined {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : fallback;
}
const num = (name: string, fallback: number) => Number(arg(name, String(fallback)));

async function connect(spec: string, appId = APP_ID) {
  const [name, adminPort, appPort] = spec.split(':');
  const wsOpts: any = { origin: APP_ID };
  const admin = await AdminWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${adminPort}`),
    wsClientOptions: wsOpts,
  });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === appId);
  if (!app) throw new Error(`${name}: no app "${appId}" on :${adminPort}`);
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info))
    for (const info of infos as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
  const lamad = cells.get('lamad');
  if (!lamad) throw new Error(`${name}: no provisioned lamad cell on :${adminPort}`);
  // One credential per cell, kept across runs: never one per call or per run.
  for (const [role, cell] of cells) await authorizeOnce(admin, cell, `${name}-${appId}-${role}`);
  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${appPort}`),
    token: (await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id }))
      .token,
    wsClientOptions: wsOpts,
    defaultTimeout: 120_000,
  });
  const call = async (role: string, zome_name: string, fn_name: string, payload: any) =>
    appWs.callZome({ cell_id: cells.get(role), zome_name, fn_name, payload }, 120_000);
  return { name, call, cells, agent: encodeHashToBase64(lamad[1]) };
}

type Conn = Awaited<ReturnType<typeof connect>>;

function stats(ms: number[]) {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return {
    n: s.length,
    min: s[0],
    p50: q(0.5),
    p90: q(0.9),
    max: s.at(-1),
    mean: Math.round((s.reduce((a, b) => a + b, 0) / s.length) * 100) / 100,
  };
}

async function timeCalls(c: Conn, role: string, zome: string, fn: string, n: number) {
  const ms: number[] = [];
  for (let i = 0; i < n; i++) {
    const t = performance.now();
    await c.call(role, zome, fn, null);
    ms.push(Math.round((performance.now() - t) * 100) / 100);
  }
  return stats(ms);
}

const rfc3339 = (d: Date) => d.toISOString().replace(/\.\d+Z$/, 'Z');

/** Same wire shape as release-attestation-probe.ts `soakInput`, one subject per write. */
function attestation(agent: string, i: number, run: string) {
  const now = new Date();
  return {
    attestation_kind: RIDDEN_KIND,
    subject_cid: `chain-cost-probe-${run}-${i}`,
    subject_kind: 'content',
    title: `chain-cost-probe ${run} #${i}`,
    description: 'fixture write to grow a disposable source chain',
    reach: 'community',
    metadata: {
      device_id: agent,
      health_metric: 'availability',
      period_start: rfc3339(new Date(now.getTime() - 600_000)),
      period_end: rfc3339(now),
      sample_count: 1,
      summary_value: 'chain-cost-probe pass 1/1',
    },
    parent_governance_action_cid: null,
    vote_value: null,
    proof_class: 'witness',
    proof_evidence: {
      class: 'witness',
      kind: 'release-soak',
      releaseCid: `chain-cost-probe-${run}-${i}`,
      channelId: 'chain-cost-probe',
      deviceArchetype: 'workstation',
      capabilityLevel: 4,
      region: 'household',
      outcome: 'pass',
      probeResults: [{ name: 'noop', ok: true }],
      buildInfo: { probe: 'chain-cost-probe' },
      soakWindow: { start: rfc3339(new Date(now.getTime() - 600_000)), end: rfc3339(now) },
    },
    expires_at: null,
  };
}

async function main() {
  const mode = process.argv[2];
  const peerSpec = arg('peer', 'matthew:4444:4445')!;
  const calls = num('calls', 30);
  const controlRole = arg('control', 'infrastructure')!;
  const [controlZome, controlFn] = arg('control-fn', '')!.split('::');
  const out = arg('out');
  const appId = arg('app-id', APP_ID)!;

  if (mode === 'install-control') {
    // A second agent in the SAME conductor with a fresh, short chain: the control
    // that separates "this chain is long" from "this conductor or store is slow".
    const [, adminPort] = peerSpec.split(':');
    const admin = await AdminWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${adminPort}`),
      wsClientOptions: { origin: APP_ID } as any,
    });
    const agent_key = await admin.generateAgentPubKey();
    const info = await admin.installApp({
      source: { type: 'path', value: arg('happ')! },
      installed_app_id: appId,
      agent_key,
    } as any);
    await admin.enableApp({ installed_app_id: appId });
    console.log(
      JSON.stringify({
        installed: appId,
        agent: encodeHashToBase64(agent_key),
        roles: Object.keys(info.cell_info),
      })
    );
    process.exit(0);
  }

  const c = await connect(peerSpec, appId);

  const emit = (row: Record<string, unknown>) => {
    const line = JSON.stringify({ at: new Date().toISOString(), peer: c.name, app: appId, ...row });
    console.log(line);
    if (out) appendFileSync(out, line + '\n');
  };
  if (out) mkdirSync(dirname(out), { recursive: true });

  const checkpoint = async (writesDone: number) => {
    const noop = await timeCalls(c, 'lamad', 'content_store', 'export_schema_version', calls);
    const control =
      controlZome && controlFn && c.cells.has(controlRole)
        ? await timeCalls(c, controlRole, controlZome, controlFn, calls)
        : null;
    emit({
      kind: 'checkpoint',
      writesDone,
      actionsAdded: writesDone * 3,
      noopMs: noop,
      controlMs: control,
    });
  };

  if (mode === 'time') {
    await checkpoint(0);
  } else if (mode === 'grow') {
    const writes = num('writes', 15000);
    const every = num('every', 500);
    const run = arg('run', String(Date.now()))!;
    await checkpoint(0);
    let window: number[] = [];
    let headMoved = 0;
    for (let i = 1; i <= writes; i++) {
      const t = performance.now();
      // A background writer on the same cell (the doorway's health attestation, a
      // storage sweep) can move the chain head under a foreground commit. Count it
      // and retry: how often it happens is itself a reading.
      for (let attempt = 0; ; attempt++) {
        try {
          await c.call('lamad', 'content_store', 'issue_attestation', attestation(c.agent, i, run));
          break;
        } catch (e: any) {
          if (attempt >= 5 || !String(e?.message ?? e).includes('source chain head has moved'))
            throw e;
          headMoved++;
        }
      }
      window.push(Math.round((performance.now() - t) * 100) / 100);
      if (i % every === 0 || i === writes) {
        emit({
          kind: 'writes',
          writesDone: i,
          actionsAdded: i * 3,
          writeMs: stats(window),
          headMoved,
        });
        headMoved = 0;
        window = [];
        await checkpoint(i);
      }
    }
  } else {
    throw new Error('usage: chain-cost-probe.ts time|grow --peer name:admin:app [--out file]');
  }
  process.exit(0);
}

main().catch(e => {
  console.error(e);
  process.exit(1);
});
