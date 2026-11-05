// The shared vectors, reproduced in TypeScript: tree bytes and CIDs, the walk and diff results,
// the TreeView normalization and CID, and the wire shapes Rust's serializer wrote.
import { encodeCanonical, verifyAddress as verifiesAddress } from '@elohim/epr';
import { blake3 } from '@noble/hashes/blake3';
import { CID } from 'multiformats/cid';
import { create as createDigest } from 'multiformats/hashes/digest';
import { describe, expect, it } from 'vitest';

import {
  ChunkerError,
  chunkBytes,
  FASTCDC_2020_EPOCH_0,
  FASTCDC_2020_TABLE_DIGEST,
  fastcdc2020TableDigest,
} from '../src/chunker.js';

import { MemoryTier, proximityOrder, standingPolicy } from '../src/tier.js';
import {
  BLAKE3_256,
  byteCidOf,
  cidOf,
  DAG_CBOR_CODEC,
  decodeTree,
  encodeTree,
  parseByteCid,
  RAW_CODEC,
  SHA2_256,
  verifiesCid,
} from '../src/tree.js';
import { decodeTreeView, encodeTreeView, parseTreeViewJson, treeViewCid, treeViewJson } from '../src/view.js';
import { collectWalk, diffTrees, View, type WalkedView } from '../src/walk.js';
import { isByteProfile, isItemAbsence, isTierRecord, isUnanswered } from '../src/wire.js';
import { build, chunkerInput, hex, kindJson, unhex, vectors, vpath, vtext } from './fixture.js';

function walkedJson(w: WalkedView): unknown {
  return {
    absent: w.absent.map(({ viewPath, reason }) => {
      switch (reason.kind) {
        case 'not-found':
          return { kind: 'not-found', path: vtext(reason.path), viewPath: vtext(viewPath) };
        case 'through-leaf':
          return { kind: 'through-leaf', leaf: vtext(reason.leaf), viewPath: vtext(viewPath) };
        case 'absent':
          return {
            cid: reason.cid,
            kind: 'absent',
            path: vtext(reason.path),
            unanswered: reason.unanswered,
            viewPath: vtext(viewPath),
          };
        default:
          throw new Error(`unexpected ${reason.kind}`);
      }
    }),
    boundaries: w.boundaries.map((b) => ({ coverage: b.coverage, kind: b.kind, path: vtext(b.path) })),
    nodes: w.nodes.map((n) => ({ coverage: n.coverage, path: vtext(n.path) })),
  };
}

describe('the shared tree vectors', () => {
  it('encodes the frozen tag-42 link', () => {
    const cid = CID.parse(vectors.link.cid);
    expect(hex(encodeCanonical(cid))).toBe(vectors.link.dagCborHex);
  });

  it('reproduces every fixture tree byte for byte and decodes it back', async () => {
    for (const t of vectors.trees) {
      const tier = new MemoryTier();
      const { root } = await build(tier, t.tree);
      expect(root.toString()).toBe(t.cid);
      const bytes = tier.blobs.get(root.toString())!;
      expect(hex(bytes)).toBe(t.bytesHex);
      const node = decodeTree(unhex(t.bytesHex));
      expect(hex((await encodeTree(node)).bytes)).toBe(t.bytesHex);
      expect(await verifiesAddress(root, bytes)).toBe(true);
    }
  });

  it('refuses noncanonical and unknown-field nodes, as decode_tree does', () => {
    // The empty tree's map with its keys reversed, and with an extra field.
    const reversed = unhex('a26776657273696f6e0167656e747269657380');
    expect(() => decodeTree(reversed)).toThrow(/noncanonical/);
    const extra = unhex('a367656e7472696573806565787472610067' + '76657273696f6e01');
    expect(() => decodeTree(extra)).toThrow();
  });

  it('walks every case to the result Rust wrote', async () => {
    const tier = new MemoryTier();
    const { root, paths } = await build(tier, vectors.walk.tree);
    expect(root.toString()).toBe(vectors.walk.root);
    for (const c of vectors.walk.cases) {
      const held = new MemoryTier();
      for (const [k, v] of tier.blobs) held.blobs.set(k, v);
      for (const m of c.missing) held.blobs.delete(paths.get(m)!.toString());
      const view = View.of(c.view.map(vpath));
      const walked = await collectWalk(held, root, view);
      expect(walkedJson(walked), c.name).toEqual(c.expect);
    }
  });

  it('diffs every case to the result Rust wrote', async () => {
    for (const c of vectors.diff) {
      const tier = new MemoryTier();
      const old = await build(tier, c.old);
      const neu = await build(tier, c.new);
      const changes = await diffTrees(tier, old.root, neu.root, View.of(c.view.map(vpath)));
      expect(
        changes.map((ch) => ({ new: kindJson(ch.new), old: kindJson(ch.old), path: vtext(ch.path) })),
        c.name,
      ).toEqual(c.expect);
    }
  });

  it('normalizes a tree view, refuses the covered list, and agrees on its CID', async () => {
    const tv = vectors.treeView;
    const root = CID.parse(tv.root);
    const view = View.of(tv.declared.map(vpath));
    expect(treeViewJson(root, view)).toEqual(tv.normalized);
    expect(hex(encodeTreeView(root, view))).toBe(tv.dagCborHex);
    expect((await treeViewCid(root, view)).toString()).toBe(tv.cid);
    expect(() => parseTreeViewJson({ root: tv.root, paths: tv.declared })).toThrow(/covered/);
    const decoded = decodeTreeView(unhex(tv.dagCborHex));
    expect(decoded.root.toString()).toBe(tv.root);
    expect(decoded.view.paths.map(vtext)).toEqual(tv.normalized.paths);
  });

  it('asks the tiers eprfs-core\'s OpennessAsk asks, case by case', () => {
    const policy = standingPolicy(vectors.ask.levels);
    const root = CID.parse(vectors.walk.root);
    expect(vectors.ask.cases.length).toBeGreaterThanOrEqual(10);
    for (const [i, c] of vectors.ask.cases.entries()) {
      const record = {
        id: `agent:uhCAkask-${i}`,
        class: c.class,
        sources: [{ source: 'held-record' as const, record: 'vectors' }],
        endpoint: null,
        standing: c.standing,
        proximity: null,
      };
      expect(policy.mayAsk(record, root, c.rootReach), JSON.stringify(c)).toBe(c.expect);
    }
  });

  it('orders tiers as eprfs-core\'s ProximityOrder does, for each of two roots', () => {
    const { roots, records, expect: expected } = vectors.order;
    expect(roots).toHaveLength(2);
    for (const [i, root] of roots.entries()) {
      const ordered = proximityOrder(records, CID.parse(root)).map((r) => r.id);
      expect(ordered, root).toEqual(expected[i]);
    }
    expect(expected[0]).not.toEqual(expected[1]);
  });

  it('reads the wire shapes Rust wrote', () => {
    expect(isTierRecord(vectors.wire.tierRecord)).toBe(true);
    expect(vectors.wire.itemAbsences).toHaveLength(6);
    for (const a of vectors.wire.itemAbsences) expect(isItemAbsence(a), JSON.stringify(a)).toBe(true);
    expect(new Set(vectors.wire.itemAbsences.map((a: { kind: string }) => a.kind)).size).toBe(6);
    expect(isUnanswered(vectors.wire.unanswered, isItemAbsence)).toBe(true);
    expect(vectors.wire.unanswered.escalation.asked).toBe(2);
    // `own` never crosses the wire.
    expect(isItemAbsence({ kind: 'unreachable', tier: 'own' })).toBe(false);
  });
});

describe('the byte profile (re-genesis 1, amended R1)', () => {
  const encoder = new TextEncoder();

  it('mints and verifies the byte-CID positives under each declared profile', async () => {
    for (const row of vectors.address.positive) {
      const bytes = encoder.encode(row.input);
      const byte = await byteCidOf(bytes);
      expect(byte.toString()).toBe(row.byteCid);
      expect((await cidOf(bytes, RAW_CODEC)).toString()).toBe(row.citeCid);
      expect((await cidOf(bytes, DAG_CBOR_CODEC)).toString()).toBe(row.recordCid);
      for (const text of [row.byteCid, row.citeCid, row.recordCid]) {
        expect(await verifiesCid(CID.parse(text), bytes)).toBe(true);
        expect(await verifiesCid(CID.parse(text), encoder.encode(`${row.input}!`))).toBe(false);
      }
      expect(parseByteCid(row.byteCid)).toBeInstanceOf(CID);
    }
  });

  it('refuses every byte-surface negative by full decode', () => {
    expect(vectors.address.negative.length).toBeGreaterThan(0);
    for (const row of vectors.address.negative) {
      expect(parseByteCid(row.input), row.name).toHaveProperty('refused');
    }
  });

  it('never verifies a sha2 CID carrying a BLAKE3 digest, nor the reverse', async () => {
    const bytes = encoder.encode('hello world');
    const blake3Digest = (await byteCidOf(bytes)).multihash.digest;
    const sha2Digest = (await cidOf(bytes, RAW_CODEC)).multihash.digest;
    const forged = (code: number, digest: Uint8Array) =>
      CID.create(1, RAW_CODEC, createDigest(code, digest));
    expect(await verifiesCid(forged(SHA2_256, blake3Digest), bytes)).toBe(false);
    expect(await verifiesCid(forged(BLAKE3_256, sha2Digest), bytes)).toBe(false);
  });

  it('carries the byte profile wire shape Rust wrote', () => {
    expect(isByteProfile(vectors.wire.byteProfile)).toBe(true);
    expect(vectors.wire.byteProfile.chunker).toEqual(FASTCDC_2020_EPOCH_0);
    expect(isByteProfile({ ...vectors.wire.byteProfile, chunker: { id: 'fastcdc-2016' } })).toBe(false);
  });

  it('rebuilds the fastcdc-2020 tables from the pinned digest', () => {
    expect(vectors.chunker.tableDigest).toBe(FASTCDC_2020_TABLE_DIGEST);
    expect(fastcdc2020TableDigest()).toBe(FASTCDC_2020_TABLE_DIGEST);
    expect(vectors.chunker.chunker).toEqual(FASTCDC_2020_EPOCH_0);
  });

  it('reproduces every chunker vector exactly', { timeout: 600_000 }, async () => {
    const section = vectors.chunker;
    expect(section.vectors.length).toBeGreaterThan(0);
    for (const vector of section.vectors) {
      const bytes = chunkerInput(section, vector);
      expect(hex(blake3(bytes)), vector.name).toBe(vector.inputBlake3);
      expect((await byteCidOf(bytes)).toString(), vector.name).toBe(vector.wholeCid);
      const chunks = chunkBytes(bytes, section.chunker);
      const named = await Promise.all(
        chunks.map(async ([offset, len]) => [
          offset,
          len,
          (await byteCidOf(bytes.subarray(offset, offset + len))).toString(),
        ]),
      );
      expect(named, vector.name).toEqual(vector.chunks);
    }
  });

  it('refuses chunker parameters outside the specification', () => {
    expect(() => chunkBytes(new Uint8Array(1), { ...FASTCDC_2020_EPOCH_0, min: 3 })).toThrow(ChunkerError);
    expect(() => chunkBytes(new Uint8Array(1), { ...FASTCDC_2020_EPOCH_0, level: 4 })).toThrow(ChunkerError);
    expect(chunkBytes(new Uint8Array(0))).toEqual([]);
  });
});
