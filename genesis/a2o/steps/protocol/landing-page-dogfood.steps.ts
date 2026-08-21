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
  doorways?: Map<string, { url: string }>;
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

// html5-app metadata ({slug, entryPoint, fallbackUrl}) is stored in `contentBody`
// as a JSON string — the seeder's conceptToInput() stringifies the content object,
// and ContentView exposes only `contentBody: Option<String>`. Some shapes also expose
// a parsed `content`; prefer it, fall back to parsing contentBody (mirrors the app's
// ProjectionApiService). This is the real wire contract — not a `content` object.
function landingContent(node: Record<string, unknown> | undefined): Record<string, unknown> {
  const parsed = node?.content;
  if (parsed && typeof parsed === 'object') return parsed as Record<string, unknown>;
  const body = node?.contentBody;
  if (typeof body === 'string' && body.length) {
    try {
      return JSON.parse(body) as Record<string, unknown>;
    } catch {
      return {};
    }
  }
  return {};
}

Then('the content.slug is {string}', function (this: DogfoodWorld, expected: string) {
  assert.equal(landingContent(this.fetchedNode).slug, expected);
});

Then('the content.entryPoint is {string}', function (this: DogfoodWorld, expected: string) {
  assert.equal(landingContent(this.fetchedNode).entryPoint, expected);
});

Then('the blobHash is a sha256 hex string', function (this: DogfoodWorld) {
  const hash = this.fetchedNode?.blobHash;
  assert.equal(typeof hash, 'string');
  // Canonical blob-hash wire format is `sha256-<64 hex>` (storage vocabulary),
  // not bare hex — the doorway serves blobHash with the algorithm prefix.
  assert.match(hash as string, /^sha256-[a-f0-9]{64}$/);
});

When('I GET {string} from the doorway', async function (this: DogfoodWorld, path: string) {
  // Most protocol scenarios use alpha. Federation failover scenarios can
  // register a more specific doorway (currently "apex") in the same World;
  // honor that registration instead of silently continuing to probe alpha.
  // Unset env names are registered literally by the generic Background step,
  // so accept only real HTTP(S) URLs before falling back to the legacy default.
  const registeredUrl = this.doorways?.get('apex')?.url ?? this.doorways?.get('alpha')?.url;
  const targetUrl =
    registeredUrl && /^https?:\/\//.test(registeredUrl) ? registeredUrl : DOORWAY_URL;
  this.doorwayResponse = await fetch(`${targetUrl}${path}`);
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

/**
 * One inScopeOf array entry may itself be a JSON-encoded array string rather
 * than a bare scope value. This is the live wire shape today for commitments
 * created from a plain-string `inScopeOf` body (the seeder's
 * `JSON.stringify(spec.scopes)` convention, `genesis/seeder/src/
 * seed-operator-bindings.ts`): elohim-storage's `normalize_create_input`
 * (`elohim/elohim-storage/src/api/rea_commitments.rs`, "pre-wrap plain-string
 * array fields") wraps that already-JSON-array-encoded string in ANOTHER
 * single-element array instead of parsing it first, so the value that
 * round-trips is `["[\"host:...\",\"epr_root:...\"]"]` — a JSON array whose
 * only element is itself a JSON array literal. Try to parse each entry as
 * JSON before treating it as a scalar scope value so this step reads the
 * real scope list regardless of which encoding shape the wire carries.
 */
function normaliseScopeOfEntry(entry: unknown): string[] {
  if (typeof entry !== 'string') return [];
  const trimmed = entry.trim();
  if (trimmed.startsWith('[')) {
    try {
      const parsed: unknown = JSON.parse(trimmed);
      if (Array.isArray(parsed)) {
        return parsed.filter((v): v is string => typeof v === 'string');
      }
    } catch {
      // Not actually JSON — fall through and treat it as a scalar/pipe value.
    }
  }
  return trimmed.includes('|') ? trimmed.split('|') : [trimmed];
}

/** Normalise inScopeOf to a flat array of scope strings regardless of wire shape.
 *  Storage may return a native array, a pipe-delimited string (e.g. "doorway:x|epr:y"),
 *  or (see normaliseScopeOfEntry) an array whose entries are themselves
 *  JSON-encoded arrays. */
function normaliseScopeOf(raw: unknown): string[] {
  if (Array.isArray(raw)) return raw.flatMap(normaliseScopeOfEntry);
  if (typeof raw === 'string') return normaliseScopeOfEntry(raw);
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
