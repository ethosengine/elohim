import { strict as assert } from 'node:assert';

import { Then, When } from '@cucumber/cucumber';

// Every fetch goes THROUGH the doorway — the web2 boundary / swap-test surface. The doorway
// proxies /db/* and /api/* to its own elohim-storage (storage_proxy.rs → forward_to_storage),
// so a client never needs (or should know) the internal storage URL. This is precisely the
// steward↔doorway hosting model this feature exists to prove: pointing the test at internal
// storage directly would bypass the very boundary under test (and couple to cluster topology).
const DOORWAY_URL =
  process.env.E2E_DOORWAY_ALPHA ?? process.env.DOORWAY_URL ?? 'http://localhost:8888';

interface DogfoodWorld {
  fetchedNode?: Record<string, unknown>;
  doorwayResponse?: Response;
  commitments?: Record<string, unknown>[];
  scopedCommitment?: Record<string, unknown>;
}

// Background `Given doorway "alpha" at "E2E_DOORWAY_ALPHA"` (mode-aware.steps.ts) registers
// the doorway from the env var — the protocol features depend on the doorway being reachable,
// not on a direct storage health probe (that would re-introduce the boundary bypass).

When('I fetch the ContentNode {string}', async function (this: DogfoodWorld, id: string) {
  // Through the doorway, not internal storage — the doorway proxies /db/content/{id}.
  const res = await fetch(`${DOORWAY_URL}/db/content/${id}`);
  assert.ok(res.ok, `content fetch failed: ${res.status}`);
  this.fetchedNode = (await res.json()) as Record<string, unknown>;
});

Then('the contentFormat is {string}', function (this: DogfoodWorld, expected: string) {
  assert.equal(this.fetchedNode?.contentFormat, expected);
});

Then('the content.slug is {string}', function (this: DogfoodWorld, expected: string) {
  const content = this.fetchedNode?.content as Record<string, unknown> | undefined;
  assert.equal(content?.slug, expected);
});

Then('the content.entryPoint is {string}', function (this: DogfoodWorld, expected: string) {
  const content = this.fetchedNode?.content as Record<string, unknown> | undefined;
  assert.equal(content?.entryPoint, expected);
});

Then('the blobHash is a sha256 hex string', function (this: DogfoodWorld) {
  const hash = this.fetchedNode?.blobHash;
  assert.equal(typeof hash, 'string');
  assert.match(hash as string, /^[a-f0-9]{64}$/);
});

When('I GET {string} from the doorway', async function (this: DogfoodWorld, path: string) {
  this.doorwayResponse = await fetch(`${DOORWAY_URL}${path}`);
});

Then('the doorway response status is {int}', function (this: DogfoodWorld, expected: number) {
  assert.equal(this.doorwayResponse?.status, expected);
});

Then(
  'the doorway response Content-Type contains {string}',
  function (this: DogfoodWorld, expected: string) {
    const ct = this.doorwayResponse?.headers.get('content-type') ?? '';
    assert.ok(ct.includes(expected), `Content-Type was '${ct}'`);
  }
);

When(
  'I list active REA commitments where provider is {string}',
  async function (this: DogfoodWorld, provider: string) {
    const url = new URL(`${DOORWAY_URL}/api/v1/commitments`);
    url.searchParams.set('provider', provider);
    const res = await fetch(url);
    assert.ok(res.ok, `commitments list failed: ${res.status}`);
    this.commitments = (await res.json()) as Record<string, unknown>[];
  }
);

/** Normalise inScopeOf to an array regardless of wire shape.
 *  Storage may return an array OR a pipe-delimited string (e.g. "doorway:x|epr:y"). */
function normaliseScopeOf(raw: unknown): string[] {
  if (Array.isArray(raw)) return raw as string[];
  if (typeof raw === 'string') return raw.split('|');
  return [];
}

Then(
  'at least one commitment has inScopeOf containing {string}',
  function (this: DogfoodWorld, expected: string) {
    const match = (this.commitments ?? []).find(c =>
      normaliseScopeOf(c.inScopeOf).includes(expected)
    );
    assert.ok(match, `No commitment had inScopeOf containing "${expected}"`);
    this.scopedCommitment = match;
  }
);

Then(
  'that commitment has inScopeOf containing {string}',
  function (this: DogfoodWorld, expected: string) {
    const scope = normaliseScopeOf(this.scopedCommitment?.inScopeOf);
    assert.ok(scope.includes(expected), `scope was ${JSON.stringify(scope)}`);
  }
);

Then(
  "that commitment's metadata signalKind is {string}",
  function (this: DogfoodWorld, expected: string) {
    const meta = (this.scopedCommitment?.metadata ?? {}) as Record<string, unknown>;
    assert.equal(meta.signalKind, expected);
  }
);

Then(
  "that commitment's metadata triggerKind is {string}",
  function (this: DogfoodWorld, expected: string) {
    const meta = (this.scopedCommitment?.metadata ?? {}) as Record<string, unknown>;
    assert.equal(meta.triggerKind, expected);
  }
);
