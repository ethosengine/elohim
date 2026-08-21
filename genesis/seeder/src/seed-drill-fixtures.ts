/**
 * Seed the Household Drill Fixtures (Act I Prologue leg)
 *
 * Two resilience scenarios name content this household's seed chain never
 * created, and both say so in their own words when it is missing:
 *
 *   features/resilience/app-blob-heal-on-read.feature
 *     Given content "heal-target" is an "html5-app" with its ZIP blob held by
 *     at least one peer
 *   → steps/mesh/household-chaos.steps.ts: '"<id>" is not seeded on this mesh,
 *     so the heal-on-read drill has nothing to break and nothing to repair.'
 *
 *   features/resilience/chaos-peer-churn.feature
 *     Given content "chaos-ladder" is under custody on all 3 household peers
 *   → steps/mesh/household-chaos.steps.ts: '"<id>" has no custody footprint on
 *     this mesh, so there is no promise for a peer death to break — the drill
 *     would be killing peers to watch nothing.'
 *
 * Those are FIXTURE reds, deliberately worded so they cannot be mistaken for
 * resilience defects. This seeder closes them.
 *
 * ## What "under custody" costs, exactly
 *
 * `assertUnderCustody` (household-chaos.steps.ts) demands BOTH halves for every
 * household peer, or the mesh's belief is already corrupt at t=0:
 *
 *   1. a `custody-blob` commitment naming that peer's agent key as `provider`
 *      for the content's blob hash — the promise, on record; and
 *   2. the bytes actually present in that peer's local blob store — the promise,
 *      kept.
 *
 * Plus `commitmentBacked > 0` on the household card, which the separate
 * `commitment_backed_collectives` fold answers from any active `provide` /
 * `replicates-*` commitment whose `resource_classified_as` list contains
 * `content:<reach>` (seed-provide-rows.ts already writes those).
 *
 * So each fixture needs: bytes on every peer, a content row naming that blob,
 * and one self-custody commitment per peer. The custody commitments are
 * authored by `seed-commitments.ts` via `CUSTODY_PAIRS_JSON` — this seeder
 * writes that pair file rather than re-deriving the commitment shape, so there
 * is exactly ONE place the `custody-blob` wire contract lives.
 *
 * ## Why the ZIP is built byte-deterministically here
 *
 * The fixture's identity IS its hash: an idempotent re-seed must produce the
 * same `blobHash`, or every run mints a new blob, a new content row mismatch,
 * and a new custody commitment id (the drift guard seed-commitments.ts already
 * documents for re-uploaded bundles). A shell `zip` embeds the file mtime, so
 * two runs a second apart hash differently — measured while building this. The
 * archive is therefore assembled in-process with STORE (method 0) entries and a
 * fixed DOS timestamp, which makes the bytes a pure function of the payload.
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 DOORWAY_B_URL=http://localhost:8889 \
 *     PEER_STORAGE_URLS=matthew=localhost:8090,jessica=localhost:8091,james=localhost:8092 \
 *     npx tsx src/seed-drill-fixtures.ts
 *
 * Exit codes mirror the Jenkinsfile `runProbedSeeder` contract: 0 = clean,
 * 2 = partial, 1 = total failure.
 */

import { createHash } from 'node:crypto';
import { writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { CommitmentClient } from './seed-commitments.js';
import { computeContentAddresses } from './doorway-client.js';
import { parseNamedCsv } from './peer-id.js';

// =============================================================================
// Deterministic ZIP (STORE only) — see the module docs for why
// =============================================================================

const CRC_TABLE: number[] = (() => {
  const table: number[] = [];
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

export function crc32(buf: Buffer): number {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/**
 * Build a ZIP archive with STORE (no compression) entries and a FIXED DOS
 * timestamp (1980-01-01 00:00, the format's own epoch — the only value with no
 * clock in it). Same entries in, same bytes out, forever.
 */
export function buildDeterministicZip(entries: { name: string; data: Buffer }[]): Buffer {
  const DOS_TIME = 0; // 00:00:00
  const DOS_DATE = 0x0021; // 1980-01-01
  const locals: Buffer[] = [];
  const centrals: Buffer[] = [];
  let offset = 0;

  for (const entry of entries) {
    const nameBuf = Buffer.from(entry.name, 'utf8');
    const crc = crc32(entry.data);

    const local = Buffer.alloc(30 + nameBuf.length);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(10, 4); // version needed
    local.writeUInt16LE(0, 6); // flags
    local.writeUInt16LE(0, 8); // method: store
    local.writeUInt16LE(DOS_TIME, 10);
    local.writeUInt16LE(DOS_DATE, 12);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(entry.data.length, 18);
    local.writeUInt32LE(entry.data.length, 22);
    local.writeUInt16LE(nameBuf.length, 26);
    local.writeUInt16LE(0, 28);
    nameBuf.copy(local, 30);
    locals.push(local, entry.data);

    const central = Buffer.alloc(46 + nameBuf.length);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4); // version made by
    central.writeUInt16LE(10, 6); // version needed
    central.writeUInt16LE(0, 8);
    central.writeUInt16LE(0, 10);
    central.writeUInt16LE(DOS_TIME, 12);
    central.writeUInt16LE(DOS_DATE, 14);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(entry.data.length, 20);
    central.writeUInt32LE(entry.data.length, 24);
    central.writeUInt16LE(nameBuf.length, 28);
    central.writeUInt16LE(0, 30); // extra len
    central.writeUInt16LE(0, 32); // comment len
    central.writeUInt16LE(0, 34); // disk number
    central.writeUInt16LE(0, 36); // internal attrs
    central.writeUInt32LE(0, 38); // external attrs
    central.writeUInt32LE(offset, 42);
    nameBuf.copy(central, 46);
    centrals.push(central);

    offset += local.length + entry.data.length;
  }

  const centralBuf = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(0, 4);
  end.writeUInt16LE(0, 6);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(centralBuf.length, 12);
  end.writeUInt32LE(offset, 16);
  end.writeUInt16LE(0, 20);

  return Buffer.concat([...locals, centralBuf, end]);
}

// =============================================================================
// Fixture declarations
// =============================================================================

export interface DrillFixture {
  id: string;
  title: string;
  description: string;
  /** The single page the ZIP carries; its text names the drill it serves. */
  page: string;
}

/**
 * The two drill fixtures, declared once.
 *
 * Both are `html5-app` because that is the format the heal-on-read step asserts
 * on (`"<id>" is a <actual>, not an html5-app`) and because the /apps resolver
 * — the surface the heal drill actually exercises — only reads ZIP-backed app
 * bundles. `chaos-ladder` shares the shape so one code path seeds both.
 *
 * Reach is `commons`: the chaos scenarios read the household card without an
 * identity, and any restricted tier would make the read a reach-gate question
 * instead of a resilience one.
 */
export const DRILL_FIXTURES: DrillFixture[] = [
  {
    id: 'heal-target',
    title: 'Heal target (drill fixture)',
    description:
      'A dedicated drill fixture for features/resilience/app-blob-heal-on-read.feature: ' +
      'its ZIP is deliberately removed from the serving peer so the first read must ' +
      'race-fetch the bytes back from a peer that still holds them.',
    page: 'heal-target — this page exists so a peer can lose it and get it back.',
  },
  {
    id: 'chaos-ladder',
    title: 'Chaos ladder (drill fixture)',
    description:
      'A dedicated drill fixture for features/resilience/chaos-peer-churn.feature: ' +
      'held under custody by every household peer so a kill can walk the protection ' +
      'ladder protected → partial → at-risk without gambling with anyone’s real custody.',
    page: 'chaos-ladder — three copies, so the ladder has rungs to descend.',
  },
];

/** The blob bytes + hash for one fixture — a pure function of its declaration. */
export function fixtureBlob(fixture: DrillFixture): { data: Buffer; hash: string } {
  const html =
    `<!doctype html>\n<html lang="en">\n<head><meta charset="utf-8">` +
    `<title>${fixture.title}</title></head>\n` +
    `<body><h1>${fixture.title}</h1><p>${fixture.page}</p></body>\n</html>\n`;
  const data = buildDeterministicZip([{ name: 'index.html', data: Buffer.from(html, 'utf8') }]);
  return { data, hash: `sha256-${createHash('sha256').update(data).digest('hex')}` };
}

// =============================================================================
// Live-mesh legs
// =============================================================================

/**
 * Put the bytes on EVERY household peer.
 *
 * Both surfaces, deliberately: the doorway's `PUT /admin/seed/blob` (which
 * forwards to the doorway's own primary storage and is the path
 * seed-commitments.ts already uses) AND each peer's own `PUT /blob/{hash}`.
 * The doorways front only their primaries — on the Act I mesh that is matthew
 * and jessica — so a doorway-only push leaves the third peer without the bytes,
 * and `assertUnderCustody` checks EVERY peer's local store. Waiting for
 * inventory gossip to close that gap would make the drill's precondition a race.
 *
 * Content-addressed, so every one of these is idempotent.
 */
async function pushToAllHolders(
  doorwayUrls: string[],
  storageUrls: string[],
  blob: { data: Buffer; hash: string },
  mimeType: string,
  apiKey: string | undefined,
  entryPoint?: string,
): Promise<{ placed: number; failures: string[] }> {
  const failures: string[] = [];
  let placed = 0;

  const headers = (): Record<string, string> => ({
    'Content-Type': mimeType,
    'X-Blob-Hash': blob.hash,
    'X-Blob-Size': String(blob.data.length),
    ...(entryPoint ? { 'X-Entry-Point': entryPoint } : {}),
    ...(apiKey ? { 'X-Api-Key': apiKey } : {}),
  });

  for (const url of doorwayUrls) {
    try {
      const response = await fetch(`${url.replace(/\/+$/, '')}/admin/seed/blob`, {
        method: 'PUT',
        headers: headers(),
        body: blob.data,
        signal: AbortSignal.timeout(60_000),
      });
      if (response.ok) placed += 1;
      else failures.push(`${url}: HTTP ${response.status} ${(await response.text()).slice(0, 120)}`);
    } catch (error) {
      failures.push(`${url}: ${String(error)}`);
    }
  }

  for (const url of storageUrls) {
    try {
      const base = url.replace(/\/+$/, '');
      // Recognize bytes already held before re-PUTting them: a peer that the
      // doorway push already reached does not need a second copy pushed at it.
      const existing = await fetch(`${base}/blob/${blob.hash}`, {
        signal: AbortSignal.timeout(30_000),
      });
      if (existing.ok) {
        placed += 1;
        continue;
      }
      const response = await fetch(`${base}/blob/${blob.hash}`, {
        method: 'PUT',
        headers: headers(),
        body: blob.data,
        signal: AbortSignal.timeout(60_000),
      });
      if (response.ok) placed += 1;
      else failures.push(`${url}: HTTP ${response.status} ${(await response.text()).slice(0, 120)}`);
    } catch (error) {
      failures.push(`${url}: ${String(error)}`);
    }
  }

  return { placed, failures };
}

/**
 * Declare the content row.
 *
 * `POST /db/content` is INSERT-only (it answers 500 "UNIQUE constraint failed"
 * for an existing row — verified live), so an existing fixture is recognized by
 * a GET first. The insert path is also the one that fires `distribute_shards`,
 * which is what gives the fixture a real shard manifest and self-held evidence
 * — the same measured distribution the chaos ladder reads.
 */
async function declareContent(
  storageUrl: string,
  fixture: DrillFixture,
  blob: { data: Buffer; hash: string },
  apiKey: string | undefined,
): Promise<'created' | 'exists' | string> {
  const base = storageUrl.replace(/\/+$/, '');
  const existing = await fetch(`${base}/db/content/${fixture.id}`, {
    signal: AbortSignal.timeout(10_000),
  });
  if (existing.ok) return 'exists';

  const response = await fetch(`${base}/db/content`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Schema-Version': '1',
      ...(apiKey ? { 'X-Api-Key': apiKey } : {}),
    },
    body: JSON.stringify({
      id: fixture.id,
      title: fixture.title,
      description: fixture.description,
      contentType: 'collective',
      contentFormat: 'html5-app',
      blobHash: blob.hash,
      contentSizeBytes: blob.data.length,
      reach: 'commons',
      // Content-derived provenance anchor set at INGEST so the row satisfies
      // the `require_provenance` read gate on a mesh whose libp2p publish drain
      // may not have run yet — the honest alternative to stamping
      // p2pPublishedAt, which would assert a DHT publication that never happened.
      dhtAnchorHash: blob.hash,
      metadata: {
        seedGeneration: 'genesis',
        fixture: 'household-drill',
        drillFor: fixture.id === 'heal-target' ? 'app-blob-heal-on-read' : 'chaos-peer-churn',
      },
    }),
    signal: AbortSignal.timeout(30_000),
  });
  if (response.ok) return 'created';
  return `HTTP ${response.status} ${(await response.text()).slice(0, 200)}`;
}

/** One self-custody pair per (peer, fixture) — the shape seed-commitments.ts reads. */
export interface DrillCustodyPair {
  providerHumanId: string;
  providerArchetype: 'desktop';
  receiverHumanId: string;
  receiverArchetype: 'desktop';
  blobHash: string;
  blobSizeBytes: number;
  fixture: string;
}

/** One fixture's identity on the mesh: what to hold, and how much of it. */
export interface CustodySubject {
  contentId: string;
  hash: string;
  sizeBytes: number;
}

/**
 * The self-custody pair set: every household peer promises to hold every
 * subject. Self-pairs (provider === receiver) are the honest shape here — each
 * peer is pledging its OWN custody of the bytes, not a favour to a named
 * counterparty, and `assertUnderCustody` only reads the provider side.
 *
 * Emitted as `blobHash` + `blobSizeBytes` pairs (never `contentId` +
 * `artifactRole`): the fixture's exact artifact is already resolved here, and
 * the contentId form would re-resolve it against the provider's own pod and
 * could select a different artifact than the one just pushed.
 */
export function buildCustodyPairs(
  humanIds: string[],
  subjects: CustodySubject[],
): Record<string, unknown>[] {
  const pairs: Record<string, unknown>[] = [];
  for (const humanId of humanIds) {
    for (const subject of subjects) {
      pairs.push({
        providerHumanId: humanId,
        providerArchetype: 'desktop',
        receiverHumanId: humanId,
        receiverArchetype: 'desktop',
        blobHash: subject.hash,
        blobSizeBytes: subject.sizeBytes,
        fixture: 'household-drill',
      });
    }
  }
  return pairs;
}

/**
 * The CANONICAL human id a peer answers with at `GET /auth/me`.
 *
 * Custody pairs are declared in human-id terms because `seed-commitments.ts`
 * re-resolves each side's agent key from that human's own pod — so the id must
 * round-trip through `storageUrlForHuman` back to THIS peer. Reading it from
 * the pod (rather than assuming `human-<peer name>`) is what keeps the pair set
 * honest on a mesh whose peer names and human slugs have drifted apart.
 *
 * Returns `undefined` for a pod that answers with a minted UUID identity rather
 * than a canonical slug — an unresolvable peer is skipped loudly upstream, not
 * guessed at.
 */
async function canonicalHumanIdFor(storageUrl: string): Promise<string | undefined> {
  try {
    const response = await fetch(`${storageUrl.replace(/\/+$/, '')}/auth/me`, {
      signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) return undefined;
    const body = (await response.json()) as { humanId?: unknown };
    return typeof body.humanId === 'string' && body.humanId.startsWith('human-')
      ? body.humanId
      : undefined;
  } catch {
    return undefined;
  }
}

/**
 * Bring an ALREADY-SEEDED commons EPR (the chaos feature names `manifesto`)
 * under household custody, without inventing any bytes for it.
 *
 * The honesty rule that shapes this whole function: a custody promise is a
 * claim that a peer HOLDS specific bytes. This seeder may only make that claim
 * for bytes it can prove are the row's declared artifact. So it walks two
 * sources, in order, and refuses rather than substitutes:
 *
 *   1. a peer that already holds the blob — the bytes are on the mesh, so
 *      pushing them to the rest is replication, not fabrication;
 *   2. the row's own `contentBody`, but ONLY when its CIDv1/raw/sha256 digest
 *      EXACTLY equals the declared `blobHash`. That equality is a proof, not a
 *      heuristic: content addressing means matching bytes ARE the artifact.
 *
 * Neither available → return undefined and say so. A markdown row whose blob
 * was ingested elsewhere and garbage-collected everywhere simply has no bytes
 * to be under custody of, and a promise to hold what nobody has is exactly the
 * corrupt belief the chaos drill exists to detect.
 */
async function resolveExistingSubject(
  doorways: string[],
  storageUrls: string[],
  contentId: string,
  apiKey: string | undefined,
): Promise<{ subject: CustodySubject; pushed: number } | { reason: string }> {
  let row: Record<string, unknown>;
  try {
    const response = await fetch(`${doorways[0]}/db/content/${encodeURIComponent(contentId)}`, {
      signal: AbortSignal.timeout(10_000),
    });
    if (!response.ok) return { reason: `GET /db/content/${contentId} → HTTP ${response.status}` };
    row = (await response.json()) as Record<string, unknown>;
  } catch (error) {
    return { reason: `GET /db/content/${contentId} failed: ${String(error)}` };
  }

  const declaredHash = typeof row['blobHash'] === 'string' ? row['blobHash'] : '';
  if (!declaredHash) return { reason: `"${contentId}" declares no blobHash` };

  // Source 1 — a peer that already holds it.
  let bytes: Buffer | undefined;
  for (const url of storageUrls) {
    try {
      const response = await fetch(`${url}/blob/${encodeURIComponent(declaredHash)}`, {
        signal: AbortSignal.timeout(30_000),
      });
      if (response.ok) {
        bytes = Buffer.from(await response.arrayBuffer());
        break;
      }
    } catch {
      // Next peer. An unreachable peer is not evidence about the bytes.
    }
  }

  // Source 2 — the row's own body, admitted ONLY on an exact content-address match.
  if (!bytes && typeof row['contentBody'] === 'string' && row['contentBody'].length > 0) {
    const candidate = Buffer.from(row['contentBody'], 'utf8');
    const { cid, hash } = await computeContentAddresses(new Uint8Array(candidate));
    if (declaredHash === cid || declaredHash === hash || declaredHash === `sha256-${hash}`) {
      bytes = candidate;
    } else {
      return {
        reason:
          `"${contentId}" declares blobHash ${declaredHash} but its contentBody addresses to ` +
          `${cid} — the body is NOT the declared artifact, so seeding custody for it would be ` +
          'a promise to hold bytes nobody has. Refusing.',
      };
    }
  }

  if (!bytes) {
    return {
      reason:
        `no household peer holds ${declaredHash} for "${contentId}" and the row carries no ` +
        'contentBody that addresses to it — there are no bytes on this mesh to be under custody of.',
    };
  }

  // Replicate to every holder so all three peers really hold it — the same
  // both-surfaces push the minted fixtures use, for the same reason.
  const push = await pushToAllHolders(
    doorways,
    storageUrls,
    { data: bytes, hash: declaredHash },
    String(row['mimeType'] ?? 'application/octet-stream'),
    apiKey,
  );

  return {
    subject: { contentId, hash: declaredHash, sizeBytes: bytes.length },
    pushed: push.placed,
  };
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayA = (process.env.DOORWAY_URL || 'http://localhost:8888').replace(/\/+$/, '');
  const doorwayB = (process.env.DOORWAY_B_URL || '').replace(/\/+$/, '');
  const doorways = doorwayB && doorwayB !== doorwayA ? [doorwayA, doorwayB] : [doorwayA];
  const apiKey = process.env.DOORWAY_API_KEY || process.env.STORAGE_API_KEY_ADMIN;
  const storageUrl = (process.env.STORAGE_URL || 'http://localhost:8090').replace(/\/+$/, '');

  console.log('='.repeat(64));
  console.log('Household Drill Fixture Seeder (Act I Prologue)');
  console.log(`  doorways : ${doorways.join(', ')}`);
  console.log(`  storage  : ${storageUrl}`);
  console.log('='.repeat(64));

  const client = new CommitmentClient({ baseUrl: doorwayA, apiKey });
  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: doorway A not healthy — ${health.error}`);
    process.exit(1);
  }

  let failed = 0;
  const seeded: CustodySubject[] = [];
  const peerUrls = parseNamedCsv(process.env.PEER_STORAGE_URLS ?? '').map(entry =>
    entry.value.includes('://') ? entry.value : `http://${entry.value}`,
  );

  for (const fixture of DRILL_FIXTURES) {
    const blob = fixtureBlob(fixture);
    console.log(`\n-- ${fixture.id} (${blob.hash.slice(0, 22)}..., ${blob.data.length} bytes) --`);

    const push = await pushToAllHolders(
      doorways,
      peerUrls,
      blob,
      'application/zip',
      apiKey,
      'index.html',
    );
    if (push.placed === 0) {
      console.error(`  [X] blob push failed everywhere: ${push.failures.join('; ')}`);
      failed += 1;
      continue;
    }
    console.log(`  [+] bytes placed on ${push.placed} holder(s)`);
    if (push.failures.length > 0) {
      // Partial placement is reported, never swallowed: `assertUnderCustody`
      // checks EVERY peer, so a peer that missed the bytes turns into a custody
      // red later, and that red must be traceable to here.
      console.warn(`  [?] some holders refused the bytes: ${push.failures.join('; ')}`);
    }

    const declared = await declareContent(storageUrl, fixture, blob, apiKey);
    if (declared === 'created' || declared === 'exists') {
      console.log(`  [${declared === 'created' ? '+' : '='}] content row ${declared}`);
      seeded.push({ contentId: fixture.id, hash: blob.hash, sizeBytes: blob.data.length });
    } else {
      console.error(`  [X] content declare failed: ${declared}`);
      failed += 1;
    }
  }

  // The commons EPR the un-parked chaos scenarios name alongside the drill
  // fixtures. Unlike heal-target/chaos-ladder this row already exists with its
  // own artifact, so the bytes are RECOVERED (never minted) — see
  // resolveExistingSubject for the two admissible sources and the refusal.
  const commonsEprId = process.env.COMMONS_CUSTODY_EPR_ID || 'manifesto';
  console.log(`\n-- ${commonsEprId} (existing commons EPR) --`);
  const existing = await resolveExistingSubject(doorways, peerUrls, commonsEprId, apiKey);
  if ('subject' in existing) {
    console.log(
      `  [+] recovered ${existing.subject.sizeBytes} bytes for ${existing.subject.hash.slice(0, 22)}... ` +
        `and placed them on ${existing.pushed} holder(s)`,
    );
    seeded.push(existing.subject);
  } else {
    // NOT counted as a failure: an absent artifact is a fact about this mesh's
    // corpus, not a defect in this seeder. Reported in full so a chaos red on
    // "manifesto" is attributable to the missing bytes, not to the drill.
    console.warn(`  [?] ${commonsEprId} left without custody — ${existing.reason}`);
  }

  const humanIds = (await Promise.all(peerUrls.map(canonicalHumanIdFor))).filter(
    (id): id is string => Boolean(id),
  );

  if (seeded.length > 0 && humanIds.length > 0) {
    const pairs = buildCustodyPairs(humanIds, seeded);
    // A DECLARED output path (not a random temp dir) is what lets the Prologue
    // chain the next leg without scraping this one's stdout — the pair file is
    // an artifact handed to seed-commitments.ts, not a log line.
    const path =
      process.env.DRILL_CUSTODY_PAIRS_OUT ||
      join(mkdtempSync(join(tmpdir(), 'drill-custody-')), 'custody-pairs.json');
    writeFileSync(path, JSON.stringify(pairs, null, 2));
    console.log('\n-- custody pairs --');
    console.log(`  [+] ${pairs.length} self-custody pair(s) across ${humanIds.length} peer(s)`);
    console.log(`      CUSTODY_PAIRS_JSON=${path}`);
    console.log('      (seed-commitments.ts owns the custody-blob wire contract)');
  } else if (seeded.length > 0) {
    console.warn(
      '  [?] no household peer answered /auth/me with a canonical human id — custody pairs ' +
        'not written. Without them the fixtures exist but nothing has PROMISED to hold them, ' +
        'and the chaos drill would be killing peers to watch nothing.',
    );
    failed += 1;
  }

  if (failed > 0) {
    console.error(`=== Results: ${failed} fixture(s) failed — partial ===`);
    process.exit(2);
  }
  console.log('\n=== Results: drill fixtures seeded ===');
  process.exit(0);
}
