/** Read-only fork admin dumps. Connect/authenticate elsewhere, to the receiver's
 * verified conductor. Capture clocks never replace the publication deadline.
 * The pooled /hc/admin route requires an explicit Admin conductor_id selector. */
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

import type { NativeReceiverProof } from './steward-native-proof.js';
import type { AdminWebsocket } from '@holochain/client';

type Admin = Pick<AdminWebsocket, 'client' | 'listApps'>;
interface Cursor {
  when_received: number;
  hash: Uint8Array;
}
interface State {
  integration_dump: {
    integrated: unknown[];
    validation_limbo: unknown[];
    integration_limbo: unknown[];
    dht_ops_cursor: Cursor | null;
  };
}
interface Timings {
  timings: unknown[];
  cursor: Cursor | null;
}
interface CaptureOptions {
  appId: string;
  role: string;
  directory: string;
  /** Separate bounded capture budget; may run after the publication deadline. */
  captureDeadlineMs: number;
  maxPages?: number;
}

/** Core serde accepts byte arrays. Preserve every byte, including entry payloads,
 * without guessing which binary values are hashes or rewriting protocol values. */
export function dumpJson(value: unknown): string {
  const plain = (v: unknown): unknown => {
    if (v instanceof Uint8Array) return Array.from(v);
    if (Array.isArray(v)) return v.map(plain);
    if (v !== null && typeof v === 'object')
      return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, plain(x)]));
    return v;
  };
  return JSON.stringify(plain(value), null, 2);
}

async function request<T>(
  admin: Admin,
  type: string,
  value: unknown,
  deadline: number
): Promise<T> {
  const remaining = Math.min(10000, deadline - Date.now());
  if (remaining <= 0) throw new Error('Operation capture budget elapsed');
  let timer: NodeJS.Timeout | undefined;
  try {
    const response = await Promise.race([
      admin.client.request<{ type: string; value: T }>({ type, value }),
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error(`Operation capture timeout: ${type}`)),
          remaining
        );
      }),
    ]);
    const expected = type === 'dump_full_state' ? 'full_state_dumped' : 'op_timings_dumped';
    if (response.type !== expected)
      throw new Error(`Unexpected capture response: ${response.type}`);
    return response.value;
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/** Verify the actual installed native cell before dumping. A hosted persona with
 * another signing key fails even if its display name is Adam. Never creates grants. */
export async function captureReceiverOps(
  admin: Admin,
  proof: NativeReceiverProof,
  options: CaptureOptions
): Promise<string> {
  if (
    !proof.accepted ||
    !proof.electionLink ||
    proof.dna !== proof.target.dna ||
    proof.acceptedAt === undefined ||
    proof.acceptedAt > proof.deadlineMs ||
    !Number.isSafeInteger(proof.deadlineMs * 1000)
  )
    throw new Error('Capture requires an accepted exact native observation');
  const adminUrl = admin.client.url;
  if (
    !adminUrl ||
    (adminUrl.pathname === '/hc/admin' && !adminUrl.searchParams.get('conductor_id'))
  ) {
    throw new Error('Capture requires a direct admin endpoint or explicit conductor_id route');
  }
  const maxPages = options.maxPages ?? 256;
  if (!Number.isSafeInteger(maxPages) || maxPages < 1 || maxPages > 4096)
    throw new Error('Invalid capture page budget');
  if (!Number.isFinite(options.captureDeadlineMs)) throw new Error('Invalid capture deadline');
  const remaining = Math.min(10000, options.captureDeadlineMs - Date.now());
  if (remaining <= 0) throw new Error('Operation capture budget elapsed');
  const apps = await admin.listApps({}, remaining);
  const app = apps.find(a => a.installed_app_id === options.appId);
  const matching = app?.cell_info[options.role]?.some(
    info =>
      info.type === 'provisioned' &&
      encodeHashToBase64(info.value.cell_id[0]) === proof.dna &&
      encodeHashToBase64(info.value.cell_id[1]) === proof.agent
  );
  if (!matching)
    throw new Error('Capture admin route does not host the exact expected native cell');
  await mkdir(options.directory, { recursive: true, mode: 0o700 });
  const statePages: string[] = [];
  const timingPages: string[] = [];
  for (const kind of ['state', 'timings'] as const) {
    let cursor: Cursor | null = null;
    const seen = new Set<string>();
    for (let page = 0; ; page++) {
      if (page >= maxPages) throw new Error(`Incomplete ${kind} capture: page budget exhausted`);
      const response: State | Timings =
        kind === 'state'
          ? await request<State>(
              admin,
              'dump_full_state',
              {
                cell_id: [decodeHashFromBase64(proof.dna), decodeHashFromBase64(proof.agent)],
                dht_ops_cursor: cursor,
                limit: 256,
              },
              options.captureDeadlineMs
            )
          : await request<Timings>(
              admin,
              'dump_op_timings',
              { dna_hash: decodeHashFromBase64(proof.dna), cursor, limit: 256 },
              options.captureDeadlineMs
            );
      const path = join(options.directory, `${kind}-${page}.json`);
      // Never persist source_chain_dump: it may contain private entries.
      const body =
        kind === 'state' ? { integration_dump: (response as State).integration_dump } : response;
      await writeFile(path, dumpJson(body), { mode: 0o600, flag: 'wx' });
      (kind === 'state' ? statePages : timingPages).push(path);
      const next: Cursor | null =
        kind === 'state'
          ? (response as State).integration_dump.dht_ops_cursor
          : (response as Timings).cursor;
      if (!next) break;
      const key = `${next.when_received}:${encodeHashToBase64(next.hash)}`;
      if (seen.has(key)) throw new Error('Capture cursor repeated; refusing incomplete evidence');
      seen.add(key);
      cursor = next;
    }
  }
  const path = join(options.directory, 'request.json');
  await writeFile(
    path,
    dumpJson({
      head: decodeHashFromBase64(proof.target.head),
      root: decodeHashFromBase64(proof.target.root),
      election_link: decodeHashFromBase64(proof.electionLink),
      issuance_action_hash: proof.target.issuanceActionHash
        ? decodeHashFromBase64(proof.target.issuanceActionHash)
        : undefined,
      acceptance_witness_hash: proof.target.acceptanceWitnessHash
        ? decodeHashFromBase64(proof.target.acceptanceWitnessHash)
        : undefined,
      deadline_micros: proof.deadlineMs * 1000,
      state_pages: statePages,
      timing_pages: timingPages,
    }),
    { mode: 0o600, flag: 'wx' }
  );
  await writeFile(
    join(options.directory, 'observation.json'),
    dumpJson({
      proof,
      capturedAt: new Date().toISOString(),
      appId: options.appId,
      role: options.role,
      conductorId: adminUrl.searchParams.get('conductor_id'),
    }),
    { mode: 0o600, flag: 'wx' }
  );
  return path;
}
