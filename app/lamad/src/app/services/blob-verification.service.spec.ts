import { TestBed } from '@angular/core/testing';
import { firstValueFrom } from 'rxjs';

import { BlobVerificationService } from './blob-verification.service';

// Byte CIDs minted at authoring time by an independent implementation (multiformats +
// @noble/hashes blake3, CIDv1 raw 0x55 + BLAKE3-256 0x1e) from the exact bytes named in each key.
const VECTORS: Record<string, string> = {
  'hello, byte profile': 'bafkr4igopxt2dtvvszg3fecuc4xmjgtllyj6o6axuwhrpksauwp2b7ap4i',
  'x*5000': 'bafkr4icsw4jm2xvvxkj6ffp6if5any6dbznrgjyikcsdukh6jvuvobleve',
  'chunk a': 'bafkr4ibjqxbkkeh2s4kajb5e7dguzrqeijdtsrjnnvj7bqn5vgpuspfpqm',
  'chunk b': 'bafkr4ihziip5jfmsrramlq2kv2k3c7gmb7nra5xqel4lzpc22bkhku37ue',
};
const byteCidOf = (text: string): { toString(): string } => ({ toString: () => VECTORS[text] });

function blobOf(text: string): { blob: Blob; bytes: Uint8Array } {
  const bytes = new Uint8Array(new TextEncoder().encode(text));
  return { blob: new Blob([bytes]), bytes };
}

describe('BlobVerificationService (byte profile)', () => {
  let service: BlobVerificationService;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [BlobVerificationService] });
    service = TestBed.inject(BlobVerificationService);
  });

  it('is a singleton', () => {
    expect(TestBed.inject(BlobVerificationService)).toBe(service);
  });

  it('verifies a blob against the byte CID of its bytes', async () => {
    const { blob } = blobOf('hello, byte profile');
    const expected = byteCidOf('hello, byte profile').toString();
    expect(expected.startsWith('bafkr4i')).toBe(true);

    const result = await firstValueFrom(service.verifyBlob(blob, expected));
    expect(result.isValid).toBe(true);
    expect(result.computedHash).toBe(expected);
    expect(result.method).toBe('blake3-js');
    expect(result.durationMs).toBeGreaterThanOrEqual(0);
  });

  it('rejects bytes that do not match the address', async () => {
        const { blob: other } = blobOf('something else');
    const result = await firstValueFrom(service.verifyBlob(other, byteCidOf('hello, byte profile').toString()));
    expect(result.isValid).toBe(false);
    expect(result.computedHash).not.toBe(result.expectedHash);
  });

  it('refuses a legacy sha256-<hex> marker even for matching bytes (not an address)', async () => {
    const { blob } = blobOf('abc');
    const sha256OfAbc = 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad';
    for (const expected of [`sha256-${sha256OfAbc}`, `sha256:${sha256OfAbc}`, sha256OfAbc]) {
      const result = await firstValueFrom(service.verifyBlob(blob, expected));
      expect(result.isValid).toBe(false);
      expect(result.error).toContain('not a byte CID');
    }
  });

  it('refuses a truncated or upper-cased byte CID (full decode, not a prefix check)', async () => {
    const { blob } = blobOf('hello, byte profile');
    const good = byteCidOf('hello, byte profile').toString();
    for (const expected of [good.slice(0, 20), good.slice(0, -1), good.toUpperCase(), '']) {
      const result = await firstValueFrom(service.verifyBlob(blob, expected));
      expect(result.isValid).toBe(false);
    }
  });

  it('streamComputeHash agrees with the kernel-style mint across chunk boundaries', async () => {
    const { blob } = blobOf('x'.repeat(5000));
    const progress: number[] = [];
    const cid = await service.streamComputeHash(blob, 1024, processed => progress.push(processed));
    expect(cid).toBe(VECTORS['x*5000']);
    expect(progress.at(-1)).toBe(5000);
  });

  it('verifyChunk and verifyMultiple delegate to the same check', async () => {
    const a = blobOf('chunk a');
    const b = blobOf('chunk b');
    const cidA = VECTORS['chunk a'];
    const cidB = VECTORS['chunk b'];
    expect((await firstValueFrom(service.verifyChunk(a.blob, cidA))).isValid).toBe(true);
    const many = await firstValueFrom(
      service.verifyMultiple([
        [a.blob, cidA],
        [b.blob, cidA],
      ])
    );
    expect(many.map(r => r.isValid)).toEqual([true, false]);
    expect(cidB).not.toBe(cidA);
  });

  it('blake3-js is always available', async () => {
    expect(await service.checkAvailableMethods()).toEqual({ blake3Js: true });
  });
});
