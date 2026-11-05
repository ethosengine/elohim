import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

import { byteCidOf } from '@app/testing/byte-cid';

import { isByteCid, verifyByteCid } from './raw-cid-verify';

const RAW = 0x55;
const DAG_CBOR = 0x71;

// Node-realm Uint8Array: jsdom's TextEncoder output fails multiformats' instanceof check.
const bytes = new Uint8Array(new TextEncoder().encode('bytes prove themselves'));

async function hexOf(data: Uint8Array): Promise<string> {
  const digest = await sha256.digest(data);
  return Array.from(digest.digest)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

describe('raw-cid-verify (byte plane)', () => {
  it('verifies bytes against their byte CID (bafkr4i…, raw + BLAKE3-256)', async () => {
    const cid = byteCidOf(bytes).toString();
    expect(cid.startsWith('bafkr4i')).toBe(true);
    expect(cid).toHaveLength(59);
    expect(isByteCid(cid)).toBe(true);
    expect(await verifyByteCid(bytes, cid)).toBe(true);
    expect(await verifyByteCid(bytes.buffer.slice(0) as ArrayBuffer, cid)).toBe(true);
  });

  it('rejects bytes that do not match the address', async () => {
    const other = new Uint8Array(new TextEncoder().encode('bytes that lie'));
    expect(await verifyByteCid(other, byteCidOf(bytes).toString())).toBe(false);
  });

  it('refuses a legacy sha256-<hex> / sha256:<hex> spelling even when the bytes match', async () => {
    const hex = await hexOf(bytes);
    for (const address of [`sha256-${hex}`, `sha256:${hex}`, hex]) {
      expect(isByteCid(address)).toBe(false);
      expect(await verifyByteCid(bytes, address)).toBe(false);
    }
  });

  it('refuses a real CID that is not the byte profile (sha2-256 raw bafkrei…, dag-cbor bafyrei…)', async () => {
    const digest = await sha256.digest(bytes);
    const citation = CID.create(1, RAW, digest).toString();
    const record = CID.create(1, DAG_CBOR, digest).toString();
    expect(citation.startsWith('bafkrei')).toBe(true);
    expect(record.startsWith('bafyrei')).toBe(true);
    for (const address of [citation, record]) {
      expect(isByteCid(address)).toBe(false);
      expect(await verifyByteCid(bytes, address)).toBe(false);
    }
  });

  it('fails closed on malformed addresses (full decode, never a prefix check)', async () => {
    const good = byteCidOf(bytes).toString();
    for (const address of [
      '',
      'bafkr4inotacid',
      good.slice(0, -1),
      `${good}a`,
      good.toUpperCase(),
      'my-app-slug',
    ]) {
      expect(isByteCid(address)).toBe(false);
      expect(await verifyByteCid(bytes, address)).toBe(false);
    }
  });
});
