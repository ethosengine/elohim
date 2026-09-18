/* eslint-disable sonarjs/no-clear-text-protocols -- fixture-only invalid.test host, never dialed */
/**
 * Covers the shed-aware staging ladder restored after b344f533a dropped it
 * (genesis/a2o/steps/dataplane/epr-app-deliverability.helpers.ts,
 * `stageBundleThroughStorage`). Exercises the SAME retry/seatbelt contract
 * stage-spa-blob.sh's `stage_once` carries (scripts/ci/stage-spa-blob.sh
 * ~376-458), against a mocked `fetch` — no real storage peer needed.
 */
import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import { describe, it } from 'node:test';

import {
  buildFixtureBundle,
  classifyStorageResponse,
  defaultStorageRetryBudget,
  NonRetryableStageError,
  removeFixtureBundle,
  RetryableStageShed,
  stageBundleThroughStorage,
  type FixtureBundle,
} from '../epr-app-deliverability.helpers.js';

const STORAGE_URL = 'http://storage.test.invalid:9999';
const SLUG = 'staging-ladder-fixture';

class MockResponse {
  constructor(
    private readonly opts: { status: number; body: string; headers?: Record<string, string> }
  ) {}
  get ok(): boolean {
    return this.opts.status >= 200 && this.opts.status < 300;
  }
  get status(): number {
    return this.opts.status;
  }
  headers = { get: (name: string) => this.opts.headers?.[name.toLowerCase()] ?? null };
  async text(): Promise<string> {
    return Promise.resolve(this.opts.body);
  }
  async json(): Promise<unknown> {
    return Promise.resolve(JSON.parse(this.opts.body));
  }
}

/** A tiny fetch double: PUT/blob computes a real hash, PATCH plays a scripted
 * response queue, GET (the seatbelt) answers with the last uploaded hash
 * unless `seatbeltHash` overrides it (to simulate drift). No `async`: every
 * branch resolves synchronously, so the function itself never awaits. */
function makeFetchMock(opts: {
  patchResponses: MockResponse[];
  seatbeltHash?: string;
  /** Overrides the upload reply body (to simulate a wire-shape mismatch). */
  uploadBody?: string;
}): {
  fetchFn: typeof fetch;
  patchCallCount: () => number;
} {
  let patchCalls = 0;
  let uploadedHash: string | undefined;
  const fetchFn = (async (url: string | URL, init?: RequestInit): Promise<Response> => {
    const target = url.toString();
    const method = (init?.method ?? 'GET').toUpperCase();
    if (target === `${STORAGE_URL}/blob/` && method === 'PUT') {
      const bytes = init?.body as Uint8Array;
      uploadedHash = `sha256-${createHash('sha256').update(Buffer.from(bytes)).digest('hex')}`;
      return Promise.resolve(
        new MockResponse({
          // Shape captured from a live household `PUT /blob/` (201, camelCase wire).
          status: 201,
          body:
            opts.uploadBody ??
            JSON.stringify({ blobHash: uploadedHash, totalSize: bytes.byteLength }),
        }) as unknown as Response
      );
    }
    if (target === `${STORAGE_URL}/db/content/${SLUG}` && method === 'PATCH') {
      const response = opts.patchResponses[patchCalls] ?? opts.patchResponses.at(-1);
      patchCalls += 1;
      assert.ok(response, 'test scripted no PATCH response for this attempt');
      return Promise.resolve(response as unknown as Response);
    }
    if (target === `${STORAGE_URL}/db/content/${SLUG}` && method === 'GET') {
      return Promise.resolve(
        new MockResponse({
          status: 200,
          body: JSON.stringify({ blobHash: opts.seatbeltHash ?? uploadedHash }),
        }) as unknown as Response
      );
    }
    return Promise.reject(new Error(`unexpected fetch in test: ${method} ${target}`));
  }) as unknown as typeof fetch;
  return { fetchFn, patchCallCount: () => patchCalls };
}

async function withFixtureBundle<T>(run: (bundle: FixtureBundle) => Promise<T>): Promise<T> {
  const bundle = buildFixtureBundle({ coherent: true });
  return run(bundle).finally(() => removeFixtureBundle(bundle));
}

/**
 * `StorageClient.putBlob` calls the module-global `fetch` directly (no
 * injection point on the client) — the mock has to stand in for that too, so
 * every case installs it globally for the call's duration, restoring the
 * real `fetch` afterward regardless of outcome.
 */
async function withGlobalFetchMock<T>(fetchFn: typeof fetch, run: () => Promise<T>): Promise<T> {
  const original = globalThis.fetch;
  globalThis.fetch = fetchFn;
  try {
    return await run();
  } finally {
    globalThis.fetch = original;
  }
}

void describe('defaultStorageRetryBudget', () => {
  void it('clamps the budget strictly inside a supplied step timeout', () => {
    // marginMs default 20_000 -> ceiling = (300_000 - 20_000) / 1000 = 280s,
    // strictly inside the 300s step (the defect: a 360s budget in a 300s step).
    const budget = defaultStorageRetryBudget({ stepTimeoutMs: 300_000 });
    assert.equal(budget.budgetSecs, 280);
    assert.ok(budget.budgetSecs * 1000 < 300_000);
  });

  void it('honors a custom margin', () => {
    const budget = defaultStorageRetryBudget({ stepTimeoutMs: 60_000, marginMs: 10_000 });
    assert.equal(budget.budgetSecs, 50);
  });

  void it('clamps STAGE_BLOB_BUDGET_SECS to the same step ceiling', () => {
    const original = process.env['STAGE_BLOB_BUDGET_SECS'];
    process.env['STAGE_BLOB_BUDGET_SECS'] = '360';
    try {
      const budget = defaultStorageRetryBudget({ stepTimeoutMs: 300_000 });
      assert.equal(budget.budgetSecs, 280);
    } finally {
      if (original === undefined) delete process.env['STAGE_BLOB_BUDGET_SECS'];
      else process.env['STAGE_BLOB_BUDGET_SECS'] = original;
    }
  });

  void it('stays unbounded (legacy default) with no step timeout supplied', () => {
    const original = process.env['STAGE_BLOB_BUDGET_SECS'];
    delete process.env['STAGE_BLOB_BUDGET_SECS'];
    try {
      assert.equal(defaultStorageRetryBudget().budgetSecs, 360);
    } finally {
      if (original !== undefined) process.env['STAGE_BLOB_BUDGET_SECS'] = original;
    }
  });
});

void describe('classifyStorageResponse', () => {
  void it('treats 2xx as ok', () => {
    assert.deepEqual(classifyStorageResponse(200, '{}', null), { ok: true });
  });

  void it('reads retryAfter off the header first, then the JSON body field', () => {
    const fromHeader = classifyStorageResponse(
      503,
      JSON.stringify({ status: 'catching-up' }),
      '12'
    );
    assert.ok(fromHeader instanceof RetryableStageShed);
    assert.equal(fromHeader.retryAfterSecs, 12);

    const fromBody = classifyStorageResponse(
      429,
      JSON.stringify({ status: 'catching-up', retryAfter: 7 }),
      null
    );
    assert.ok(fromBody instanceof RetryableStageShed);
    assert.equal(fromBody.retryAfterSecs, 7);
  });

  void it('keeps a "not retrievable" 4xx retryable with no hint', () => {
    const verdict = classifyStorageResponse(404, 'target not retrievable yet', null);
    assert.ok(verdict instanceof RetryableStageShed);
    assert.equal(verdict.retryAfterSecs, undefined);
  });

  void it('treats any OTHER 4xx as structural and non-retryable', () => {
    const verdict = classifyStorageResponse(409, JSON.stringify({ error: 'slug locked' }), null);
    assert.ok(verdict instanceof NonRetryableStageError);
  });
});

void describe('stageBundleThroughStorage', () => {
  void it('retries a 503 catching-up shed within budget and then succeeds', async () => {
    const sleepCalls: number[] = [];
    const { fetchFn, patchCallCount } = makeFetchMock({
      patchResponses: [
        new MockResponse({
          status: 503,
          body: JSON.stringify({ status: 'catching-up', retryAfter: 1 }),
        }),
        new MockResponse({ status: 200, body: JSON.stringify({ dhtAnchorHash: 'action-1' }) }),
      ],
    });

    const outcome = await withGlobalFetchMock(fetchFn, async () =>
      withFixtureBundle(async bundle =>
        stageBundleThroughStorage({
          bundle,
          slug: SLUG,
          storageUrl: STORAGE_URL,
          fetchFn,
          sleep: async seconds => {
            sleepCalls.push(seconds);
            return Promise.resolve();
          },
          budget: { budgetSecs: 30, attempts: 5, maxWaitSecs: 5 },
        })
      )
    );

    assert.equal(outcome.code, 0);
    assert.equal(outcome.authoredActionHash, 'action-1');
    assert.equal(patchCallCount(), 2);
    assert.deepEqual(sleepCalls, [1]);
  });

  void it('fails immediately on a non-retryable 4xx (no backoff, no retry)', async () => {
    let sleepCalled = false;
    const { fetchFn, patchCallCount } = makeFetchMock({
      patchResponses: [
        new MockResponse({ status: 409, body: JSON.stringify({ error: 'slug locked' }) }),
      ],
    });

    await assert.rejects(
      withGlobalFetchMock(fetchFn, async () =>
        withFixtureBundle(async bundle =>
          stageBundleThroughStorage({
            bundle,
            slug: SLUG,
            storageUrl: STORAGE_URL,
            fetchFn,
            sleep: async () => {
              sleepCalled = true;
              return Promise.resolve();
            },
            budget: { budgetSecs: 30, attempts: 5, maxWaitSecs: 5 },
          })
        )
      ),
      /structural failure \(HTTP 409\)/
    );
    assert.equal(patchCallCount(), 1);
    assert.equal(sleepCalled, false);
  });

  void it(
    'throws a terminal error naming the leg, attempts, elapsed time, last status, ' +
      'last body, and last Retry-After when the budget exhausts',
    async () => {
      const { fetchFn } = makeFetchMock({
        patchResponses: [
          new MockResponse({
            status: 503,
            body: JSON.stringify({ status: 'catching-up' }),
          }),
        ],
      });
      let now = 0;
      const sleepCalls: number[] = [];

      await assert.rejects(
        withGlobalFetchMock(fetchFn, async () =>
          withFixtureBundle(async bundle =>
            stageBundleThroughStorage({
              bundle,
              slug: SLUG,
              storageUrl: STORAGE_URL,
              fetchFn,
              now: () => now,
              sleep: async seconds => {
                sleepCalls.push(seconds);
                now += seconds * 1000;
                return Promise.resolve();
              },
              // Budget exhausts on attempt 2 (see the "never sleeps past the
              // deadline" test below for the arithmetic).
              budget: { budgetSecs: 8, attempts: 60, maxWaitSecs: 60 },
            })
          )
        ),
        (error: Error) => {
          assert.match(error.message, /staging ladder exhausted at leg "PATCH"/);
          assert.match(error.message, /after 2 attempt\(s\)/);
          assert.match(error.message, /elapsed/);
          assert.match(error.message, /last HTTP status: 503/);
          assert.match(error.message, /last body: .*catching-up/);
          assert.match(error.message, /last Retry-After: n\/a/);
          return true;
        }
      );
      assert.deepEqual(sleepCalls, [3]);
    }
  );

  void it('never sleeps past the remaining budget deadline, even with a large Retry-After hint', async () => {
    const { fetchFn } = makeFetchMock({
      patchResponses: [
        new MockResponse({
          status: 503,
          body: '{}',
          headers: { 'retry-after': '50' },
        }),
      ],
    });
    let now = 0;
    const sleepCalls: number[] = [];

    await assert.rejects(
      withGlobalFetchMock(fetchFn, async () =>
        withFixtureBundle(async bundle =>
          stageBundleThroughStorage({
            bundle,
            slug: SLUG,
            storageUrl: STORAGE_URL,
            fetchFn,
            now: () => now,
            sleep: async seconds => {
              sleepCalls.push(seconds);
              now += seconds * 1000;
              return Promise.resolve();
            },
            budget: { budgetSecs: 10, attempts: 60, maxWaitSecs: 60 },
          })
        )
      ),
      (error: Error) => {
        // The hint is preserved in the terminal error even though it was
        // clamped down for the actual sleep.
        assert.match(error.message, /last Retry-After: 50s/);
        return true;
      }
    );

    // The 50s Retry-After hint was clamped to the ~5s actually left in the
    // 10s budget — never slept past the deadline.
    assert.deepEqual(sleepCalls, [5]);
    const totalSlept = sleepCalls.reduce((sum, s) => sum + s, 0);
    assert.ok(totalSlept < 10, `slept ${totalSlept}s, past the 10s budget`);
  });

  void it('fails when the post-PATCH seatbelt GET finds a different hash', async () => {
    const { fetchFn } = makeFetchMock({
      patchResponses: [
        new MockResponse({ status: 200, body: JSON.stringify({ dhtAnchorHash: 'action-1' }) }),
      ],
      seatbeltHash: 'sha256-deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef',
    });

    await assert.rejects(
      withGlobalFetchMock(fetchFn, async () =>
        withFixtureBundle(async bundle =>
          stageBundleThroughStorage({
            bundle,
            slug: SLUG,
            storageUrl: STORAGE_URL,
            fetchFn,
            sleep: async () => Promise.resolve(),
            budget: { budgetSecs: 30, attempts: 5, maxWaitSecs: 5 },
          })
        )
      ),
      /blobHash drift after PATCH/
    );
  });

  void it('refuses an upload reply that names no blob hash, on the first attempt', async () => {
    // The defect this pins: a reply shaped unlike the wire used to surface as a TypeError that the
    // ladder retried as a shed for its whole budget (11 attempts / 275 s on the household).
    const { fetchFn, patchCallCount } = makeFetchMock({
      patchResponses: [],
      uploadBody: JSON.stringify({ cid: 'cid-fixture' }),
    });
    const sleepCalls: number[] = [];

    await assert.rejects(
      withGlobalFetchMock(fetchFn, async () =>
        withFixtureBundle(async bundle =>
          stageBundleThroughStorage({
            bundle,
            slug: SLUG,
            storageUrl: STORAGE_URL,
            fetchFn,
            sleep: async seconds => {
              sleepCalls.push(seconds);
              return Promise.resolve();
            },
            budget: { budgetSecs: 30, attempts: 5, maxWaitSecs: 5 },
          })
        )
      ),
      /answered the blob upload without a blob hash/
    );
    assert.equal(sleepCalls.length, 0, 'a shape defect must not be retried');
    assert.equal(patchCallCount(), 0);
  });
});
