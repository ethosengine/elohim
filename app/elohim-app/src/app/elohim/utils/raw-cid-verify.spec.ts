import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

import { expectedSha256Hex, verifyRawSha256 } from './raw-cid-verify';

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

describe('raw-cid-verify', () => {
  it('verifies bytes against a sha256-<hex> address', async () => {
    const address = `sha256-${await hexOf(bytes)}`;
    expect(await verifyRawSha256(bytes, address)).toBe(true);
    expect(await verifyRawSha256(bytes.buffer, address)).toBe(true);
  });

  it('verifies bytes against a CIDv1 raw sha2-256 address (bafkrei…)', async () => {
    const cid = CID.create(1, RAW, await sha256.digest(bytes)).toString();
    expect(cid.startsWith('bafkrei')).toBe(true);
    expect(await verifyRawSha256(bytes, cid)).toBe(true);
    expect(expectedSha256Hex(cid)).toBe(await hexOf(bytes));
  });

  it('rejects bytes that do not match the address', async () => {
    const other = new Uint8Array(new TextEncoder().encode('bytes that lie'));
    const cid = CID.create(1, RAW, await sha256.digest(bytes)).toString();
    expect(await verifyRawSha256(other, cid)).toBe(false);
    expect(await verifyRawSha256(other, `sha256-${await hexOf(bytes)}`)).toBe(false);
  });

  it('fails closed on a CID whose codec is not raw (dag-cbor bafyrei…)', async () => {
    const cid = CID.create(1, DAG_CBOR, await sha256.digest(bytes)).toString();
    expect(cid.startsWith('bafyrei')).toBe(true);
    expect(expectedSha256Hex(cid)).toBeNull();
    expect(await verifyRawSha256(bytes, cid)).toBe(false);
  });

  it('fails closed on malformed addresses', async () => {
    for (const address of [
      '',
      'bafkreinotacid',
      'sha256-deadbeef',
      `sha256-${'A'.repeat(64)}`,
      'my-app-slug',
    ]) {
      expect(expectedSha256Hex(address)).toBeNull();
      expect(await verifyRawSha256(bytes, address)).toBe(false);
    }
  });
});
