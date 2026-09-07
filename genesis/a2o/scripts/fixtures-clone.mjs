#!/usr/bin/env node
// Bounded experiment only. Never starts/stops a conductor or alters a deployed bundle.
import {
  AdminWebsocket,
  AppWebsocket,
  encodeHashToBase64,
  getSigningCredentials,
  setSigningCredentials,
} from '@holochain/client';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
const requireSeeder = createRequire(new URL('../../seeder/package.json', import.meta.url));
const YAML = requireSeeder('yaml');
const {
  values: o,
  positionals: [mode],
} = parseArgs({
  allowPositionals: true,
  options: Object.fromEntries(
    [
      'out',
      'bundle',
      'hc',
      'admin',
      'app',
      'seed',
      'cells',
      'pid',
      'data',
      'storage',
      'db',
      'credentials',
    ].map(key => [key, { type: 'string' }])
  ),
});
const root = fileURLToPath(new URL('../../../', import.meta.url));
function required(key) {
  if (!o[key]) throw Error(`--${key} required`);
  return o[key];
}
function localOutput(value) {
  const result = path.resolve(value);
  if (!result.startsWith(root) || result === root)
    throw Error('Output must be inside this worktree');
  // Reject symlink escapes as well as lexical traversal.
  let parent = result;
  while (!fs.existsSync(parent)) parent = path.dirname(parent);
  if (!fs.realpathSync(parent).startsWith(root)) throw Error('Output resolves outside worktree');
  return result;
}
const out = localOutput(required('out'));
if (mode === 'prepare') {
  const hc = required('hc');
  const version = execFileSync(hc, ['--version'], { encoding: 'utf8' });
  if (!version.includes('0.7.')) throw Error(`Expected hc 0.7: ${version}`);
  fs.mkdirSync(out); // refuse reuse: preserve previous evidence and source bundle
  execFileSync(hc, [
    'app',
    'unpack',
    path.resolve(required('bundle')),
    '-o',
    path.join(out, 'workdir'),
  ]);
  const manifest = path.join(out, 'workdir/happ.yaml');
  const doc = YAML.parse(fs.readFileSync(manifest, 'utf8'));
  const role = doc.roles.find(role => role.name === 'lamad');
  if (!role) throw Error('No lamad role');
  // 0.7: clone_only panics assembling AppInfo; deferred is ignored on install.
  // Keep normal provisioning, change ONLY this copied role's clone_limit.
  if (role.provisioning?.strategy !== 'create') throw Error('Expected provisioned create strategy');
  role.dna.clone_limit = 15;
  fs.writeFileSync(manifest, YAML.stringify(doc));
  execFileSync(hc, ['app', 'pack', path.dirname(manifest)]);
  fs.writeFileSync(
    path.join(out, 'prepare.json'),
    JSON.stringify({ source: path.resolve(o.bundle), hc: version, seed: randomUUID() }, null, 2)
  );
  console.log(path.join(out, 'workdir/elohim.happ'));
  process.exit(0);
}
fs.mkdirSync(out, { recursive: true });
const adminUrl = new URL(required('admin'));
if (!['127.0.0.1', 'localhost', '[::1]'].includes(adminUrl.hostname))
  throw Error('Harness requires loopback admin');
const appId = required('app');
const admin = await AdminWebsocket.connect({
  url: adminUrl,
  defaultTimeout: 300000,
  wsClientOptions: { origin: 'http://localhost' },
});
let app;
const write = (name, data) =>
  fs.writeFileSync(path.join(out, name), JSON.stringify(data, null, 2) + '\n');
try {
  if (mode === 'install') {
    if ((await admin.listApps({})).length) throw Error('Install requires an empty test conductor');
    await admin.installApp({
      source: { type: 'path', value: path.resolve(required('bundle')) },
      installed_app_id: appId,
      network_seed: required('seed'),
    });
    await admin.enableApp({ installed_app_id: appId });
  }
  const apps = await admin.listApps({});
  if (apps.length !== 1) throw Error('Measurement requires exactly one test app');
  const info = apps.find(a => a.installed_app_id === appId);
  if (!info) throw Error(`App ${appId} missing`);
  const existing = (await admin.listAppInterfaces()).find(
    i => i.installed_app_id === appId || i.installed_app_id == null
  );
  const { port } =
    existing ??
    (await admin.attachAppInterface({
      port: 0,
      allowed_origins: '*',
      installed_app_id: appId,
    }));
  const { token } = await admin.issueAppAuthenticationToken({ installed_app_id: appId });
  app = await AppWebsocket.connect({
    url: new URL(`ws://${adminUrl.hostname}:${port}`),
    token,
    defaultTimeout: 300000,
    wsClientOptions: { origin: 'http://localhost' },
  });
  if (mode === 'install' || mode === 'grow') {
    const target = Number(o.cells ?? 6);
    if (![5, 6, 10, 20].includes(target)) throw Error('--cells must be 5, 6, 10 or 20');
    const installed = Object.values(info.cell_info).flat();
    if (installed.filter(c => c.type === 'provisioned').length !== 5)
      throw Error('Expected five provisioned cells');
    // appInfo does not guarantee clone ordering: derive the index from the name so the
    // guard stays order-independent across repeated grow passes.
    for (const cell of (info.cell_info.lamad ?? []).filter(c => c.type === 'cloned')) {
      const name = cell.value.name;
      const match = /^fixtures(?:-([1-9]\d*))?$/.exec(name ?? '');
      const index = match ? Number(match[1] ?? 0) : NaN;
      if (
        !cell.value.enabled ||
        !match ||
        cell.value.dna_modifiers.network_seed !== `${required('seed')}-fixtures-${index}`
      )
        throw Error('Existing clone does not belong to this experiment seed/sequence');
    }
    let actual = (await admin.listCellIds()).length;
    if (actual > target) throw Error(`Already ${actual} cells, cannot measure ${target}`);
    while (actual < target) {
      const index = actual - 5;
      if (index < 0) throw Error('Expected five provisioned roles');
      const started = performance.now();
      const clone = await app.createCloneCell({
        role_name: 'lamad',
        name: index === 0 ? 'fixtures' : `fixtures-${index}`,
        modifiers: { network_seed: `${required('seed')}-fixtures-${index}` },
      });
      await app.enableCloneCell({ clone_cell_id: { type: 'clone_id', value: clone.clone_id } });
      fs.appendFileSync(
        path.join(out, 'clones.jsonl'),
        JSON.stringify({ ...clone, create_enable_ms: performance.now() - started }) + '\n'
      );
      actual++;
    }
    write('app.json', await app.appInfo());
  } else if (mode === 'measure') {
    const pid = Number(required('pid'));
    if (!Number.isInteger(pid) || pid < 1) throw Error('Invalid pid');
    await new Promise(resolve => setTimeout(resolve, 60000)); // same settling interval as idle run
    const status = fs.readFileSync(`/proc/${pid}/status`, 'utf8');
    const cells = (await admin.listCellIds()).length;
    if (cells !== Number(required('cells'))) throw Error(`Unexpected cell count ${cells}`);
    const row = {
      time: new Date().toISOString(),
      cells,
      clones: cells - 5,
      rss_kib: Number(status.match(/VmRSS:\s+(\d+)/)[1]),
      disk_bytes: Number(
        execFileSync('du', ['-s', '-B1', required('data')], { encoding: 'utf8' }).split(/\s/)[0]
      ),
      fds: fs.readdirSync(`/proc/${pid}/fd`).length,
      threads: Number(status.match(/Threads:\s+(\d+)/)[1]),
    };
    write(`network-${cells}.json`, await admin.dumpNetworkStats());
    write(`app-${cells}.json`, await app.appInfo());
    fs.appendFileSync(path.join(out, 'measurements.jsonl'), JSON.stringify(row) + '\n');
    console.log(JSON.stringify(row));
  } else if (mode === 'authorize') {
    // This writes CapGrants: run once BEFORE the baseline, then reuse credentials.
    const file = localOutput(required('credentials'));
    if (fs.existsSync(file)) throw Error('Credentials already exist; reuse them');
    const credentials = {};
    for (const cell of info.cell_info.lamad) {
      if (!['provisioned', 'cloned'].includes(cell.type)) continue;
      await admin.authorizeSigningCredentials(cell.value.cell_id);
      credentials[encodeHashToBase64(cell.value.cell_id[0])] = getSigningCredentials(
        cell.value.cell_id
      );
    }
    fs.writeFileSync(
      file,
      JSON.stringify(credentials, (_, value) =>
        value instanceof Uint8Array ? { bytes: [...value] } : value
      ),
      { mode: 0o600, flag: 'wx' }
    );
  } else if (mode === 'inventory') {
    // Persist complete DHT dumps, not counts. Zome export is a source-chain view only;
    // dumpFullState additionally exposes integrated DHT operations on the peer.
    for (const cell of info.cell_info.lamad) {
      if (!['provisioned', 'cloned'].includes(cell.type)) continue;
      const id = cell.value.cell_id;
      const label = cell.type === 'provisioned' ? 'base' : cell.value.clone_id;
      const dumps = [];
      let cursor;
      for (let page = 0; ; page++) {
        if (page >= 10000) throw Error('DHT inventory exceeded page budget; incomplete');
        const dump = await admin.dumpFullState({
          cell_id: id,
          dht_ops_cursor: cursor,
          limit: 1000,
        });
        dumps.push(dump);
        const ops = dump.integration_dump;
        if (!ops.integrated.length && !ops.integration_limbo.length && !ops.validation_limbo.length)
          break;
        const next = ops.dht_ops_cursor;
        if (next == null || JSON.stringify(next) === JSON.stringify(cursor))
          throw Error('DHT cursor did not advance; incomplete');
        cursor = next;
      }
      write(`${label}-dht.json`, dumps);
      write(
        `${label}-ops.json`,
        [
          ...new Set(
            dumps.flatMap(d =>
              ['integrated', 'integration_limbo', 'validation_limbo'].flatMap(bucket =>
                d.integration_dump[bucket].map(op => JSON.stringify(op))
              )
            )
          ),
        ].sort()
      );
      write(
        `${label}-actions.json`,
        [
          ...new Set(
            dumps
              .flatMap(d => d.source_chain_dump.records)
              .map(r => encodeHashToBase64(r.action_address))
          ),
        ].sort()
      );
      const credentials = JSON.parse(
        fs.readFileSync(required('credentials'), 'utf8'),
        (_, value) => (value?.bytes ? Uint8Array.from(value.bytes) : value)
      );
      const credential = credentials[encodeHashToBase64(id[0])];
      if (!credential) throw Error(`No pre-baseline signing credential for ${label}`);
      setSigningCredentials(id, credential);
      const records = await app.callZome({
        cell_id: id,
        zome_name: 'content_store',
        fn_name: 'export_all_content',
        payload: null,
      });
      write(
        `${label}-content.json`,
        records
          .map(r => ({
            id: r.content.id,
            action: encodeHashToBase64(r.action_hash),
            entry: encodeHashToBase64(r.entry_hash),
          }))
          .sort((a, b) => a.action.localeCompare(b.action))
      );
    }
    write('targets.json', {
      app: appId,
      target: 'lamad.fixtures',
      cells: info.cell_info.lamad.map(c => ({
        type: c.type,
        name: c.value.name,
        cloneId: c.value.clone_id,
        cellId: c.value.cell_id.map(encodeHashToBase64),
      })),
    });
    const rows = execFileSync(
      'python3',
      [
        '-c',
        'import sqlite3,json,sys; c=sqlite3.connect("file:"+sys.argv[1]+"?mode=ro",uri=True); c.row_factory=sqlite3.Row; print(json.dumps([dict(r) for r in c.execute("SELECT * FROM content ORDER BY id")],sort_keys=True))',
        required('db'),
      ],
      { encoding: 'utf8' }
    );
    write('projection.json', JSON.parse(rows));
    const documents = [];
    for (let offset = 0; ; offset += 100) {
      const response = await fetch(
        `${required('storage')}/sync/v1/elohim/docs?offset=${offset}&limit=100`
      );
      if (!response.ok) throw Error(`Sync inventory HTTP ${response.status}`);
      const page = await response.json();
      if (!Array.isArray(page.documents)) throw Error('Unexpected sync page');
      documents.push(...page.documents);
      if (documents.length >= page.total) break;
      if (!page.documents.length) throw Error('Incomplete sync inventory');
    }
    write(
      'sync.json',
      documents.sort((a, b) => a.docId.localeCompare(b.docId))
    );
  } else throw Error('Modes: prepare, install, grow, measure, authorize, inventory');
} finally {
  if (app) await app.client.close();
  await admin.client.close();
}
