import { strict as assert } from 'node:assert';

import { Then, When } from '@cucumber/cucumber';

const STORAGE_URL = process.env.STORAGE_URL ?? 'http://localhost:8090';
const DOORWAY_URL = process.env.DOORWAY_URL ?? 'http://localhost:8888';

interface DogfoodWorld {
  fetchedNode?: Record<string, unknown>;
  doorwayResponse?: Response;
  commitments?: Array<Record<string, unknown>>;
  scopedCommitment?: Record<string, unknown>;
}

// NOTE: `Given elohim-storage is healthy at {string}` is intentionally reused
// from steps/compute-allocation.steps.ts — the assertion is identical.

When('I fetch the ContentNode {string}', async function (this: DogfoodWorld, id: string) {
  const res = await fetch(`${STORAGE_URL}/db/content/${id}`);
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

Then(
  'the doorway response status is {int}',
  function (this: DogfoodWorld, expected: number) {
    assert.equal(this.doorwayResponse?.status, expected);
  },
);

Then(
  'the doorway response Content-Type contains {string}',
  function (this: DogfoodWorld, expected: string) {
    const ct = this.doorwayResponse?.headers.get('content-type') ?? '';
    assert.ok(ct.includes(expected), `Content-Type was '${ct}'`);
  },
);

When(
  'I list active REA commitments where provider is {string}',
  async function (this: DogfoodWorld, provider: string) {
    const url = new URL(`${STORAGE_URL}/api/v1/commitments`);
    url.searchParams.set('provider', provider);
    const res = await fetch(url);
    assert.ok(res.ok, `commitments list failed: ${res.status}`);
    this.commitments = (await res.json()) as Array<Record<string, unknown>>;
  },
);

Then(
  'at least one commitment has inScopeOf containing {string}',
  function (this: DogfoodWorld, expected: string) {
    const match = (this.commitments ?? []).find(c =>
      Array.isArray(c.inScopeOf) ? (c.inScopeOf as string[]).includes(expected) : false,
    );
    assert.ok(match, `No commitment had inScopeOf containing "${expected}"`);
    this.scopedCommitment = match;
  },
);

Then(
  'that commitment has inScopeOf containing {string}',
  function (this: DogfoodWorld, expected: string) {
    const scope = (this.scopedCommitment?.inScopeOf ?? []) as string[];
    assert.ok(scope.includes(expected), `scope was ${JSON.stringify(scope)}`);
  },
);

Then(
  "that commitment's metadata signalKind is {string}",
  function (this: DogfoodWorld, expected: string) {
    const meta = (this.scopedCommitment?.metadata ?? {}) as Record<string, unknown>;
    assert.equal(meta.signalKind, expected);
  },
);

Then(
  "that commitment's metadata triggerKind is {string}",
  function (this: DogfoodWorld, expected: string) {
    const meta = (this.scopedCommitment?.metadata ?? {}) as Record<string, unknown>;
    assert.equal(meta.triggerKind, expected);
  },
);
