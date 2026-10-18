#!/usr/bin/env node
// Bounded experiment only. Never starts/stops a conductor or alters a deployed bundle.
// Run against ALREADY RUNNING, empty test conductors; keep storage disconnected.
// Direct calls avoid the seeder, but an attached storage can still project/re-author content.
// Example (F=genesis/a2o/scripts/fixtures-clone.mjs; R=genesis/a2o/reports/clone-probe):
// node $F prepare --out $R/bundle --hc /path/to/hc --bundle /path/to/elohim.happ
// S=$(node -p "require('./$R/bundle/prepare.json').seed")
// for P in 4444 4454 4464 4474; do
//   SEED=$S; [ "$P" = 4474 ] && SEED=$S-outsider
//   node $F install --admin ws://localhost:$P --app probe --seed $SEED --clones 1 \
//     --bundle $R/bundle/workdir/elohim.happ --out $R/peer-$P
// done
// node $F authorize --admin ws://localhost:4444 --app probe --out $R/auth \
//   --credentials $R/credentials.json  # before the baseline; writes CapGrants
// node $F measure --admin ws://localhost:4444 --app probe --clones 1 --pid $PID \
//   --data $DATA --out $R/cost
// node $F write --admin ws://localhost:4444 --app probe --clone fixtures \
//   --credentials $R/credentials.json --out $R/probe
// node $F poll --admin 4444,4454,4464 --expect-absent 4474 --app probe \
//   --write $R/probe/write.json --timeout 600 --out $R/probe
// for P in 4444 4454 4464 4474; do
//   SEED=$S; [ "$P" = 4474 ] && SEED=$S-outsider
//   node $F grow --admin ws://localhost:$P --app probe --seed $SEED --clones 5 \
//     --out $R/peer-$P
// done
// node $F measure --admin ws://localhost:4444 --app probe --clones 5 --pid $PID \
//   --data $DATA --out $R/cost
// --clones counts lamad clones; deprecated --cells counts all five roles plus clones.
// Repeat grow/measure at 15 clones; measure each peer with its own PID/data/output if desired.
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
      'clones',
      'cells',
      'pid',
      'data',
      'storage',
      'db',
      'credentials',
      'clone',
      'write',
      'timeout',
      'expect-absent',
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
function targetCells() {
  const clones = o.clones === undefined ? undefined : Number(required('clones'));
  const cells = o.cells === undefined ? undefined : Number(required('cells'));
  if (clones !== undefined && cells !== undefined && clones + 5 !== cells)
    throw Error('--clones and deprecated --cells disagree (cells must equal clones + 5)');
  if (clones === undefined && cells === undefined && mode === 'measure') required('clones');
  const target = clones === undefined ? (cells ?? 6) : clones + 5;
  if (![5, 6, 10, 20].includes(target))
    throw Error('--clones must be 0, 1, 5 or 15 (--cells: 5, 6, 10 or 20)');
  return target;
}
const target = ['install', 'grow', 'measure'].includes(mode) ? targetCells() : undefined;

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
// New modes share the same exact-name, enabled experiment-clone guard.
function experimentClone(info, name) {
  if (!/^fixtures(?:-[1-9]\d*)?$/.test(name)) throw Error('Expected a fixtures clone name');
  const matches = (info.cell_info.lamad ?? []).filter(
    c => c.type === 'cloned' && c.value.name === name && c.value.enabled
  );
  if (matches.length !== 1) throw Error(`Expected one enabled clone named ${name}`);
  return matches[0].value;
}
if (mode === 'poll') {
  await poll();
  process.exit(0);
}
async function poll() {
  const receipt = JSON.parse(fs.readFileSync(required('write'), 'utf8'));
  const timeout = Number(o.timeout ?? 600);
  if (!Number.isFinite(timeout) || timeout <= 0) throw Error('Invalid --timeout');
  if (!receipt.action_hash || !receipt.signature || !receipt.cell_id?.[0])
    throw Error('Expected write.json with action hash, signature and cell id');
  const peers = [];
  for (const [list, absent] of [
    [required('admin'), false],
    [o['expect-absent'] ?? '', true],
  ]) {
    for (const address of list.split(',').filter(Boolean)) {
      const url = new URL(/^\d+$/.test(address) ? `ws://localhost:${address}` : address);
      if (!['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname))
        throw Error('Harness requires loopback admin');
      if (peers.some(p => p.url === url.href)) throw Error('Duplicate peer');
      peers.push({ url: url.href, expect_absent: absent, first_seen: 'never', samples: 0 });
    }
  }
  if (!peers.some(p => !p.expect_absent)) throw Error('--admin needs at least one member');
  const started = performance.now();
  const deadline = started + timeout * 1000;
  const result = { action_hash: receipt.action_hash, started_at: new Date().toISOString(), peers };
  const connections = [];
  const save = () =>
    fs.writeFileSync(
      path.join(out, 'poll.json'),
      JSON.stringify(result, (key, value) => (key === 'id' ? undefined : value), 2) + '\n'
    );
  // Bound each request by the remaining observation window, including connection setup.
  async function bounded(operation) {
    let timer;
    try {
      return await Promise.race([
        operation(),
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(Error('Observation deadline elapsed')),
            Math.max(1, deadline - performance.now())
          );
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  }
  try {
    for (const peer of peers) {
      const connection = await bounded(() =>
        AdminWebsocket.connect({
          url: new URL(peer.url),
          defaultTimeout: Math.max(1, deadline - performance.now()),
          wsClientOptions: {
            origin: 'http://localhost',
            handshakeTimeout: Math.max(1, deadline - performance.now()),
          },
        }).then(connection => {
          if (performance.now() >= deadline) {
            connection.client.close();
            throw Error('Observation deadline elapsed during connection');
          }
          return connection;
        })
      );
      connections.push(connection);
      const apps = await bounded(() => connection.listApps({}));
      if (apps.length !== 1 || apps[0].installed_app_id !== required('app'))
        throw Error('Poll requires exactly one matching test app per peer');
      const clone = experimentClone(apps[0], receipt.clone);
      peer.cell_id = clone.cell_id.map(encodeHashToBase64);
      if ((peer.cell_id[0] === receipt.cell_id[0]) === peer.expect_absent)
        throw Error('Member DNA must match write; outsider DNA must differ');
      peer.id = clone.cell_id;
    }
    do {
      await Promise.all(
        peers.map(async (peer, index) => {
          let cursor;
          for (let page = 0; ; page++) {
            if (page >= 10000) throw Error('Poll exceeded DHT page budget');
            // @holochain/client/lib/api/admin/types.d.ts:201-205,801-829:
            // cell_id + optional dht_ops_cursor; only integrated ops establish holding.
            const dump = await bounded(() =>
              connections[index].dumpFullState({
                cell_id: peer.id,
                dht_ops_cursor: cursor,
              })
            );
            const ops = dump.integration_dump;
            // DHT dumps carry signed actions, not necessarily action hashes. Match the
            // exact 64-byte signature returned WITH this action hash by the authoring call;
            // never match a referenced prev_action/entry hash or a limbo operation.
            if (ops.integrated.some(op => holdsSignedAction(op, receipt.signature))) {
              if (peer.first_seen === 'never') peer.first_seen = new Date().toISOString();
              break;
            }
            if (
              !ops.integrated.length &&
              !ops.integration_limbo.length &&
              !ops.validation_limbo.length
            )
              break;
            const next = ops.dht_ops_cursor;
            if (next == null || next === cursor) throw Error('DHT cursor did not advance');
            cursor = next;
          }
          peer.samples++;
          peer.last_checked = new Date().toISOString();
        })
      );
      save();
      if (peers.some(p => p.expect_absent && p.first_seen !== 'never'))
        throw Error('Isolation failed: outsider holds probe');
      if (!peers.some(p => p.expect_absent) && peers.every(p => p.first_seen !== 'never')) break;
      const remaining = deadline - performance.now();
      if (remaining <= 0) break;
      await new Promise(resolve => setTimeout(resolve, Math.min(15000, remaining)));
    } while (performance.now() < deadline);
    if (peers.some(p => !p.expect_absent && p.first_seen === 'never'))
      throw Error('Propagation timeout: see per-peer first_seen in poll.json');
    result.passed = true;
  } catch (error) {
    result.passed = false;
    result.error = error.message;
    throw error;
  } finally {
    result.finished_at = new Date().toISOString();
    for (const peer of peers) delete peer.id;
    save();
    await Promise.all(connections.map(c => c.client.close()));
  }
}
function holdsSignedAction(op, signature) {
  // lib/hdk/dht-ops.d.ts:67-84: ChainOp tuples start with a SignedAction
  // (or a Signature for CreateEntry). Older 0.7 dumps use signature-first tuples.
  // Inspect only the operation's signer, never arbitrary entry bytes/references.
  const tuple = op.ChainOp && Object.values(op.ChainOp)[0];
  const signer = tuple?.[0];
  const bytes = signer?.signature ?? signer;
  if (typeof bytes === 'string') return bytes === signature;
  if (!(bytes instanceof Uint8Array) && !Array.isArray(bytes)) return false;
  return bytes.length === 64 && Buffer.from(bytes).toString('base64') === signature;
}
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
    if (cells !== target) throw Error(`Unexpected cell count ${cells}`);
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
    const membership = [];
    for (const cell of info.cell_info.lamad ?? []) {
      if (
        cell.type !== 'provisioned' &&
        !(cell.type === 'cloned' && /^fixtures(?:-[1-9]\d*)?$/.test(cell.value.name ?? ''))
      )
        continue;
      const id = cell.value.cell_id;
      // @holochain/client/lib/api/admin/websocket.d.ts:93 and types.d.ts:520-526:
      // agentInfo({ dna_hashes: DnaHash[] | null }) returns signed JSON strings.
      const raw = await admin.agentInfo({ dna_hashes: [id[0]] });
      const agents = raw.map(encoded => {
        const signed = JSON.parse(encoded);
        const value =
          typeof signed.agentInfo === 'string'
            ? JSON.parse(signed.agentInfo)
            : (signed.agentInfo ?? signed);
        if (typeof value.agent !== 'string') throw Error('Unrecognized agent-info identity');
        // Kitsune2 advertises the raw 32-byte Ed25519 key as base64url.
        // Normalize to base64; this is not a 39-byte HoloHash.
        const pubkey = Buffer.from(value.agent, 'base64url');
        if (pubkey.length !== 32) throw Error('Unexpected agent public-key length');
        return {
          agent_pubkey: pubkey.toString('base64'),
          agent_pubkey_encoding: 'raw-ed25519-base64',
          ...(Object.hasOwn(value, 'storageArc') ? { storage_arc: value.storageArc } : {}),
          ...(Object.hasOwn(value, 'storage_arc') ? { storage_arc: value.storage_arc } : {}),
        };
      });
      membership.push({
        name: cell.type === 'provisioned' ? 'base' : cell.value.name,
        cell_id: id.map(encodeHashToBase64),
        agents,
        raw,
      });
    }
    write(`agents-${cells}.json`, membership);
    fs.appendFileSync(path.join(out, 'measurements.jsonl'), JSON.stringify(row) + '\n');
    console.log(JSON.stringify(row));
  } else if (mode === 'write') {
    if (fs.existsSync(path.join(out, 'write.json'))) throw Error('write.json already exists');
    const clone = experimentClone(info, required('clone'));
    const id = clone.cell_id;
    const credentials = JSON.parse(fs.readFileSync(required('credentials'), 'utf8'), (_, value) =>
      value?.bytes ? Uint8Array.from(value.bytes) : value
    );
    const credential = credentials[encodeHashToBase64(id[0])];
    if (!credential) throw Error('No pre-baseline signing credential for clone');
    setSigningCredentials(id, credential);
    // content_store/src/lib.rs:15994-16001: one public FlowMilestone create_entry,
    // then get; no anchors, links or content projection signal. Payload is defined
    // in content_store_integrity/src/lib.rs:2580-2598 (public EntryTypes:3773).
    const time = new Date().toISOString();
    const record = await app.callZome({
      cell_id: id,
      zome_name: 'content_store',
      fn_name: 'create_flow_milestone',
      payload: {
        id: `clone-probe-${randomUUID()}`,
        milestone_number: 'probe',
        plan_id: 'probe',
        name: 'Clone probe',
        description: null,
        target_date: Date.now() * 1000,
        actual_date: null,
        success_criteria_json: '[]',
        all_criteria_met: false,
        depends_on_goals_json: '[]',
        depends_on_milestones_json: '[]',
        blocks_goals_json: '[]',
        status: 'pending',
        achieved_at: null,
        milestone_event_ids_json: '[]',
        schema_version: 1,
        validation_status: 'valid',
      },
    });
    write('write.json', {
      time,
      returned_at: new Date().toISOString(),
      clone: clone.name,
      cell_id: id.map(encodeHashToBase64),
      action_hash: encodeHashToBase64(record.signed_action.hashed.hash),
      signature: Buffer.from(record.signed_action.signature).toString('base64'),
    });
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
    if (o.db === undefined && o.storage === undefined) {
      console.log('Projection/storage inventory skipped: neither --db nor --storage was given');
    } else {
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
    }
  } else throw Error('Modes: prepare, install, grow, measure, write, poll, authorize, inventory');
} finally {
  if (app) await app.client.close();
  await admin.client.close();
}
