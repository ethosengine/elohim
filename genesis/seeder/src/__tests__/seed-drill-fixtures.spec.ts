import { describe, it, expect } from 'vitest';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  buildCustodyPairs,
  buildDeterministicZip,
  crc32,
  fixtureBlob,
  DRILL_FIXTURES,
  type CustodySubject,
} from '../seed-drill-fixtures.js';

describe('crc32', () => {
  it('matches the known ZIP/PKZIP check value for "123456789"', () => {
    expect(crc32(Buffer.from('123456789', 'utf8'))).toBe(0xcbf43926);
  });

  it('is 0 for empty input', () => {
    expect(crc32(Buffer.alloc(0))).toBe(0);
  });
});

describe('buildDeterministicZip', () => {
  const entries = [{ name: 'index.html', data: Buffer.from('<h1>hi</h1>\n', 'utf8') }];

  it('produces byte-identical output across calls', () => {
    // The fixture's identity IS its hash. A shell `zip` embeds the file mtime,
    // so two runs a second apart hash differently and every re-seed would mint
    // a new blob, a stale content row, and a new custody commitment id.
    expect(buildDeterministicZip(entries)).toEqual(buildDeterministicZip(entries));
  });

  it('writes a real local-file header and end-of-central-directory record', () => {
    const zip = buildDeterministicZip(entries);
    expect(zip.readUInt32LE(0)).toBe(0x04034b50);
    expect(zip.readUInt32LE(zip.length - 22)).toBe(0x06054b50);
    expect(zip.readUInt16LE(zip.length - 22 + 10)).toBe(entries.length);
  });

  it('stores rather than compresses, so the payload is recoverable byte-for-byte', () => {
    const zip = buildDeterministicZip(entries);
    expect(zip.readUInt16LE(8)).toBe(0); // method 0 = store
    expect(zip.includes(entries[0].data)).toBe(true);
  });

  it('unzips with the system unzip to exactly the bytes put in', () => {
    // A hand-rolled archive is only worth anything if standard tooling — and
    // therefore the /apps ZIP resolver this fixture exists to exercise — reads it.
    const dir = mkdtempSync(join(tmpdir(), 'zip-roundtrip-'));
    const zipPath = join(dir, 'fixture.zip');
    writeFileSync(zipPath, buildDeterministicZip(entries));
    const listed = execFileSync('unzip', ['-l', zipPath], { encoding: 'utf8' });
    expect(listed).toContain('index.html');
    const extracted = execFileSync('unzip', ['-p', zipPath, 'index.html']);
    expect(Buffer.from(extracted)).toEqual(entries[0].data);
  });

  it('changes its bytes when the payload changes', () => {
    const other = buildDeterministicZip([{ name: 'index.html', data: Buffer.from('other') }]);
    expect(other).not.toEqual(buildDeterministicZip(entries));
  });
});

describe('fixtureBlob', () => {
  it('gives each declared fixture a stable, distinct hash', () => {
    const hashes = DRILL_FIXTURES.map(f => fixtureBlob(f).hash);
    expect(new Set(hashes).size).toBe(DRILL_FIXTURES.length);
    for (const hash of hashes) expect(hash).toMatch(/^sha256-[0-9a-f]{64}$/);
  });

  it('is a pure function of the declaration (idempotent re-seed)', () => {
    for (const fixture of DRILL_FIXTURES) {
      expect(fixtureBlob(fixture).hash).toBe(fixtureBlob(fixture).hash);
    }
  });

  it('declares the two ids the resilience features actually name', () => {
    expect(DRILL_FIXTURES.map(f => f.id).sort()).toEqual(['chaos-ladder', 'heal-target']);
  });
});

describe('buildCustodyPairs', () => {
  const subjects: CustodySubject[] = [
    { contentId: 'chaos-ladder', hash: 'sha256-aaa', sizeBytes: 158 },
    { contentId: 'manifesto', hash: 'sha256-bbb', sizeBytes: 4096 },
  ];
  const humans = ['human-matthew-manager', 'human-jessica-spouse', 'human-james-son'];

  it('promises every subject on every peer — the ladder needs all three rungs', () => {
    expect(buildCustodyPairs(humans, subjects)).toHaveLength(6);
  });

  it('emits SELF pairs: each peer pledges its own custody, not a favour', () => {
    for (const pair of buildCustodyPairs(humans, subjects)) {
      expect(pair['providerHumanId']).toBe(pair['receiverHumanId']);
    }
  });

  it('carries the resolved artifact, never a contentId to be re-resolved', () => {
    // A contentId + artifactRole pair re-resolves against the provider's own
    // pod and can select a DIFFERENT artifact than the one just pushed.
    const pair = buildCustodyPairs(humans, subjects)[0];
    expect(pair['blobHash']).toBe('sha256-aaa');
    expect(pair['blobSizeBytes']).toBe(158);
    expect(pair).not.toHaveProperty('contentId');
  });

  it('marks its provenance so the fixtures are legible as fixtures', () => {
    expect(buildCustodyPairs(humans, subjects)[0]['fixture']).toBe('household-drill');
  });

  it('is empty when there are no peers or no subjects', () => {
    expect(buildCustodyPairs([], subjects)).toEqual([]);
    expect(buildCustodyPairs(humans, [])).toEqual([]);
  });
});
