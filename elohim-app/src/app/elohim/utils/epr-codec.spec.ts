import {
  encodeEprHead,
  decodeEprHead,
  isDagCborCid,
  isDagCborCidString,
  DAG_CBOR_CODEC,
  EPR_HEAD_CODEC,
  EPR_DOCUMENT_CODEC,
  EPR_RELATIONSHIP_CODEC,
} from './epr-codec';
import type { EprHead } from '../models/epr-head.model';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

function sampleHead(): EprHead {
  return {
    version: 1,
    id: 'test-concept',
    content: 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku',
    lamad: {
      title: 'Test Concept',
      contentType: 'concept',
      description: 'A test concept',
      contentFormat: 'markdown',
      tags: ['test', 'ipld'],
    },
    shefa: {
      stewards: ['did:web:alice.example.com'],
      allocations: [100],
    },
    qahal: {
      reach: 'commons',
      layer: 'communal',
    },
    relationships: [
      {
        type: 'TEACHES',
        target: 'related-concept',
      },
    ],
    author: 'did:web:alice.example.com',
    updated: '2026-02-27T12:00:00Z',
  };
}

describe('epr-codec', () => {
  describe('encodeEprHead / decodeEprHead roundtrip', () => {
    it('should roundtrip through DAG-CBOR', async () => {
      const head = sampleHead();
      const { bytes, cid } = await encodeEprHead(head);

      // CID should use DAG-CBOR codec
      expect(cid.code).toBe(DAG_CBOR_CODEC);

      // Bytes should not start with '{' (not JSON)
      expect(bytes[0]).not.toBe(0x7b);

      // Decode back
      const decoded = decodeEprHead(bytes);
      expect(decoded).toEqual(head);
    });

    it('should auto-detect JSON input', () => {
      const head = sampleHead();
      const jsonBytes = new TextEncoder().encode(JSON.stringify(head));

      // First byte should be '{'
      expect(jsonBytes[0]).toBe(0x7b);

      const decoded = decodeEprHead(jsonBytes);
      expect(decoded.id).toBe('test-concept');
      expect(decoded.lamad.title).toBe('Test Concept');
    });
  });

  describe('CID properties', () => {
    it('should produce a CID with codec 0x71 (dag-cbor)', async () => {
      const { cid } = await encodeEprHead(sampleHead());
      expect(cid.code).toBe(0x71);
      expect(cid.code).not.toBe(0x55); // not raw
    });

    it('should produce a CID string starting with bafyr', async () => {
      const { cid } = await encodeEprHead(sampleHead());
      const cidStr = cid.toString();
      expect(cidStr.startsWith('bafyr')).toBe(true);
    });
  });

  describe('isDagCborCid', () => {
    it('should return true for DAG-CBOR CID', async () => {
      const { cid } = await encodeEprHead(sampleHead());
      expect(isDagCborCid(cid)).toBe(true);
    });

    it('should return false for raw CID', async () => {
      const data = new TextEncoder().encode('test');
      const hash = await sha256.digest(data);
      const rawCid = CID.create(1, 0x55, hash);
      expect(isDagCborCid(rawCid)).toBe(false);
    });
  });

  describe('isDagCborCidString', () => {
    it('should detect bafyr prefix', () => {
      expect(isDagCborCidString('bafyreigdp...')).toBe(true);
    });

    it('should reject bafkrei prefix (raw codec)', () => {
      expect(isDagCborCidString('bafkreihdw...')).toBe(false);
    });
  });

  describe('error handling', () => {
    it('should throw on empty input', () => {
      expect(() => decodeEprHead(new Uint8Array(0))).toThrowError('Empty input');
    });

    it('should throw on invalid JSON', () => {
      const bytes = new TextEncoder().encode('{invalid json}');
      expect(() => decodeEprHead(bytes)).toThrow();
    });

    it('should throw on invalid CBOR', () => {
      expect(() => decodeEprHead(new Uint8Array([0xa0, 0x01, 0x02]))).toThrow();
    });
  });

  describe('multicodec constants', () => {
    it('should be in the private-use range', () => {
      expect(EPR_HEAD_CODEC).toBeGreaterThanOrEqual(0x300000);
      expect(EPR_HEAD_CODEC).toBeLessThanOrEqual(0x3fffff);
      expect(EPR_DOCUMENT_CODEC).toBeGreaterThanOrEqual(0x300000);
      expect(EPR_RELATIONSHIP_CODEC).toBeGreaterThanOrEqual(0x300000);
    });

    it('should be distinct', () => {
      expect(EPR_HEAD_CODEC).not.toBe(EPR_DOCUMENT_CODEC);
      expect(EPR_HEAD_CODEC).not.toBe(EPR_RELATIONSHIP_CODEC);
      expect(EPR_DOCUMENT_CODEC).not.toBe(EPR_RELATIONSHIP_CODEC);
    });
  });

  describe('minimal head', () => {
    it('should encode and decode a minimal head', async () => {
      const head: EprHead = {
        version: 1,
        id: 'minimal',
        content: 'bafkrei...',
        lamad: { title: 'Minimal', contentType: 'concept' },
        shefa: {},
        qahal: {},
        relationships: [],
      };

      const { bytes, cid } = await encodeEprHead(head);
      expect(cid.code).toBe(DAG_CBOR_CODEC);

      const decoded = decodeEprHead(bytes);
      expect(decoded.id).toBe('minimal');
      expect(decoded.lamad.title).toBe('Minimal');
    });
  });
});
