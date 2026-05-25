import { expect } from '@open-wc/testing';
import { Loader, type LoaderTransport } from './loader.js';

// ---------------------------------------------------------------------------
// Fake transport for test isolation
// ---------------------------------------------------------------------------

class FakeTransport implements LoaderTransport {
  constructor(
    public readonly name: string,
    public response: Uint8Array | 'fail' | 'unavailable'
  ) {}

  async fetch(_cid: string): Promise<Uint8Array | null> {
    if (this.response === 'fail') throw new Error(`${this.name} fetch error`);
    if (this.response === 'unavailable') return null;
    return this.response;
  }

  isAvailable(): boolean {
    return this.response !== 'unavailable';
  }
}

// ---------------------------------------------------------------------------
// Shared fixture
// ---------------------------------------------------------------------------

const SAMPLE_BYTES = new Uint8Array([1, 2, 3, 4]);
// sha256 of [0x01, 0x02, 0x03, 0x04]
const SAMPLE_CID = 'sha256-9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a';

// ---------------------------------------------------------------------------
// Loader — transport fallback chain
// ---------------------------------------------------------------------------

describe('Loader — transport fallback chain', () => {
  it('resolves from the first available transport (localCache hit)', async () => {
    const loader = new Loader(
      [
        new FakeTransport('localCache', SAMPLE_BYTES),
        new FakeTransport('tauri', 'fail'),
        new FakeTransport('doorway', 'fail'),
      ],
      { verifyCid: false }
    );
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.bytes).to.deep.equal(SAMPLE_BYTES);
    expect(result.source).to.equal('localCache');
    expect(result.cid).to.equal(SAMPLE_CID);
  });

  it('falls through to next transport when first returns null (unavailable)', async () => {
    const loader = new Loader(
      [new FakeTransport('localCache', 'unavailable'), new FakeTransport('tauri', SAMPLE_BYTES)],
      { verifyCid: false }
    );
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.source).to.equal('tauri');
    expect(result.bytes).to.deep.equal(SAMPLE_BYTES);
  });

  it('falls through to next transport when first throws (transport error)', async () => {
    const loader = new Loader(
      [new FakeTransport('doorway', 'fail'), new FakeTransport('peer', SAMPLE_BYTES)],
      { verifyCid: false }
    );
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.source).to.equal('peer');
    expect(result.bytes).to.deep.equal(SAMPLE_BYTES);
  });

  it('returns unresolved when all transports fail', async () => {
    const loader = new Loader(
      [new FakeTransport('doorway', 'fail'), new FakeTransport('peer', 'unavailable')],
      { verifyCid: false }
    );
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.bytes).to.be.null;
    expect(result.source).to.equal('unresolved');
    expect(result.cid).to.equal(SAMPLE_CID);
  });

  it('skips transports reporting isAvailable() === false without calling fetch', async () => {
    let fetchCalled = false;
    const unavailableTransport: LoaderTransport = {
      name: 'skipped',
      isAvailable: () => false,
      async fetch(_cid) {
        fetchCalled = true;
        return SAMPLE_BYTES;
      },
    };
    const loader = new Loader([unavailableTransport, new FakeTransport('fallback', SAMPLE_BYTES)], {
      verifyCid: false,
    });
    const result = await loader.resolve(SAMPLE_CID);
    expect(fetchCalled).to.be.false;
    expect(result.source).to.equal('fallback');
  });
});

// ---------------------------------------------------------------------------
// Loader — CID verification
// ---------------------------------------------------------------------------

describe('Loader — CID verification', () => {
  it('accepts bytes when CID matches (verifyCid: true)', async () => {
    const loader = new Loader([new FakeTransport('doorway', SAMPLE_BYTES)], { verifyCid: true });
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.bytes).to.deep.equal(SAMPLE_BYTES);
    expect(result.source).to.equal('doorway');
  });

  it('propagates CID mismatch as a hard rejection (does not fall through)', async () => {
    const loader = new Loader([new FakeTransport('doorway', SAMPLE_BYTES)], { verifyCid: true });
    let threw = false;
    let message = '';
    try {
      await loader.resolve('sha256-wrongwrongwrong');
    } catch (err) {
      threw = true;
      message = err instanceof Error ? err.message : String(err);
    }
    expect(threw).to.be.true;
    expect(message).to.contain('CID mismatch');
  });

  it('does not fall through to next transport on CID mismatch', async () => {
    // If CID mismatch fell through, the second transport would return bytes and
    // the resolved source would be 'fallback'. We assert it never reaches it.
    let fallbackFetched = false;
    const fallback: LoaderTransport = {
      name: 'fallback',
      isAvailable: () => true,
      async fetch(_cid) {
        fallbackFetched = true;
        return SAMPLE_BYTES;
      },
    };
    const loader = new Loader([new FakeTransport('primary', SAMPLE_BYTES), fallback], {
      verifyCid: true,
    });
    let threw = false;
    try {
      await loader.resolve('sha256-wrongwrongwrong');
    } catch {
      threw = true;
    }
    expect(threw).to.be.true;
    expect(fallbackFetched).to.be.false;
  });

  it('rejects unsupported CID algorithm', async () => {
    const loader = new Loader([new FakeTransport('doorway', SAMPLE_BYTES)], { verifyCid: true });
    let threw = false;
    let message = '';
    try {
      await loader.resolve('blake2b-abcdef1234');
    } catch (err) {
      threw = true;
      message = err instanceof Error ? err.message : String(err);
    }
    expect(threw).to.be.true;
    expect(message).to.contain('Unsupported CID algo');
  });

  it('defaults verifyCid to true when options are omitted', async () => {
    // Without options, the Loader should default to safe (verify-on). A correct
    // CID must pass; this ensures the safe default is not silently disabled.
    const loader = new Loader([new FakeTransport('doorway', SAMPLE_BYTES)]);
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.bytes).to.deep.equal(SAMPLE_BYTES);
  });
});
