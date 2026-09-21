/**
 * Fixtures for the EPR-app deliverability story
 * (features/dataplane/epr-app-deliverability.feature).
 *
 * Three jobs, and nothing else lives here:
 *   1. build a tiny but REAL browser bundle on disk (index.html + one entry
 *      script + one stylesheet + version.json), coherent or deliberately not;
 *   2. publish it through the SAME script CI publishes with
 *      (scripts/ci/stage-spa-blob.sh), so the story exercises the deploy path
 *      rather than a re-implementation of it;
 *   3. open a page in a real browser and report what the browser saw.
 *
 * Spec: genesis/docs/superpowers/specs/2026-09-08-epr-app-deliverability-through-doorway.md
 */

import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { StorageClient } from '@elohim/storage-client';

import { householdMeshDir } from '../../src/framework/fixtures/household-mesh.js';
import { OwnedDoorwayPair } from '../../src/framework/fixtures/owned-doorway-pair.js';

import type { HouseholdMeshFixture } from '../../src/framework/fixtures/household-mesh.js';
import type { E2EWorld } from '../../src/framework/world.js';

const execFileAsync = promisify(execFile);

/** <repo>/genesis/a2o/steps/dataplane/<this file> → up four is the repo root. */
export const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');

/** The deploy path this story must exercise — never a re-implementation of it. */
export const STAGE_SPA_BLOB = join(REPO_ROOT, 'scripts', 'ci', 'stage-spa-blob.sh');

/**
 * Each doorway re-reads its peer's declared head on a fixed tick —
 * BUNDLE_HEADS_TICK_SECS in doorway-service (spec D1). Two ticks plus slack for
 * the read itself is the bound every convergence assertion in this story uses.
 */
export const BUNDLE_HEADS_TICK_SECS = 30;
export const CONVERGENCE_BOUND_MS = BUNDLE_HEADS_TICK_SECS * 2 * 1000 + 15_000;

/**
 * Where fixture bundles are built. Under the a2o reports tree (gitignored, and
 * owned by this repo) rather than a world-writable system temp directory.
 */
function fixtureRoot(): string {
  const root = process.env['A2O_FIXTURE_DIR'] ?? join(process.cwd(), 'reports', 'fixtures');
  mkdirSync(root, { recursive: true });
  return root;
}

export interface FixtureBundle {
  /** Directory holding the bundle's files — what stage-spa-blob.sh zips. */
  dir: string;
  /** `main-<token>.js`, the browser entry point the index page names. */
  entryScript: string;
  /** `styles-<token>.css`. */
  styleSheet: string;
  /** The build stamp written into version.json (`commit`). */
  stamp: string;
  /** False when the entry script was deliberately left out of the directory. */
  coherent: boolean;
}

/**
 * Build a bundle on disk that a browser can genuinely boot.
 *
 * The page leaves an empty `<app-root>` for the script to fill, which is the
 * same shape elohim-app's own index.html has — so one "the app root has
 * content" assertion reads the fixture app and the real app identically.
 *
 * `coherent: false` writes the index page that NAMES the entry script and then
 * does not write the script. That is the 2026-09-04 outage in miniature: every
 * other file resolves, and the app never starts.
 */
export function buildFixtureBundle(opts: { coherent: boolean; baseHref?: string }): FixtureBundle {
  // Content-addressed names, so two bundles built in the same millisecond are still
  // distinguishable — a collision here would make "the PREVIOUS era" unobservable.
  const token = `${Date.now().toString(36)}${randomBytes(4).toString('hex')}`.toUpperCase();
  const dir = mkdtempSync(join(fixtureRoot(), 'epr-app-'));
  const entryScript = `main-${token}.js`;
  const styleSheet = `styles-${token}.css`;
  const stamp = `fixture-${token}`;

  writeFileSync(
    join(dir, 'index.html'),
    [
      '<!doctype html>',
      '<html lang="en">',
      '  <head>',
      '    <meta charset="utf-8" />',
      `    <title>EPR app deliverability fixture ${token}</title>`,
      `    <base href="${opts.baseHref ?? './'}" />`,
      `    <link rel="stylesheet" href="${styleSheet}" />`,
      '  </head>',
      '  <body>',
      '    <app-root></app-root>',
      `    <script src="${entryScript}" type="module"></script>`,
      '  </body>',
      '</html>',
      '',
    ].join('\n')
  );

  writeFileSync(join(dir, styleSheet), 'app-root { display: block; font-family: sans-serif; }\n');

  if (opts.coherent) {
    writeFileSync(
      join(dir, entryScript),
      [
        '// The whole app: fill the root element the page left empty.',
        "const root = document.querySelector('app-root');",
        `root.textContent = 'EPR app deliverability fixture ${token} is running.';`,
        `root.setAttribute('data-fixture-stamp', ${JSON.stringify(stamp)});
root.setAttribute('data-app-ready', 'true');`,
        '',
      ].join('\n')
    );
  }

  writeFileSync(
    join(dir, 'version.json'),
    `${JSON.stringify(
      {
        commit: stamp,
        version: '0.0.0-fixture',
        buildTime: new Date().toISOString(),
        environment: 'mesh-fixture',
        service: 'epr-app-deliverability-fixture',
      },
      null,
      2
    )}\n`
  );

  return { dir, entryScript, styleSheet, stamp, coherent: opts.coherent };
}

export function removeFixtureBundle(bundle: FixtureBundle): void {
  rmSync(bundle.dir, { recursive: true, force: true });
}

export interface StageOutcome {
  /** stage-spa-blob.sh's exit code. 2 = the peer judged the bundle BROKEN. */
  code: number;
  output: string;
  /** `sha256-…`, parsed off the script's own "blob hash:" line. */
  blobHash: string;
  /** Exact action returned by the authoring PATCH; absent for byte-only staging. */
  authoredActionHash?: string;
}

/**
 * Publish a bundle exactly the way CI publishes one.
 *
 * `declare: false` hands the peer the BYTES only (DO_PATCH unset) — the leg CI
 * runs against every serving host. `declare: true` also PATCHes the head onto
 * the record (DO_PATCH=1) — the leg CI runs exactly once, through whichever
 * doorway can reach a live conductor.
 */
export async function stageBundle(opts: {
  bundle: FixtureBundle;
  slug: string;
  doorwayUrl: string;
  declare: boolean;
  kind?: 'browser' | 'server';
}): Promise<StageOutcome> {
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    DO_PATCH: opts.declare ? '1' : '0',
    // Use the publisher's bounded retry policy. Canonical concurrent writers
    // can conflict after the deliberately induced peer restart; verdict2 stays terminal.
  };
  let code = 0;
  let output = '';
  try {
    const { stdout, stderr } = await execFileAsync(
      'bash',
      [STAGE_SPA_BLOB, opts.bundle.dir, opts.slug, opts.doorwayUrl, opts.kind ?? 'browser'],
      { env, cwd: REPO_ROOT, maxBuffer: 8 * 1024 * 1024 }
    );
    output = `${stdout}\n${stderr}`;
  } catch (error) {
    const failure = error as { code?: number; stdout?: string; stderr?: string; message?: string };
    code = typeof failure.code === 'number' ? failure.code : 1;
    output = `${failure.stdout ?? ''}\n${failure.stderr ?? ''}\n${failure.message ?? ''}`;
  }
  const match = /blob hash:\s*(sha256-[0-9a-f]+)/i.exec(output);
  const action = /authored action:\s*([^\s]+)/i.exec(output);
  return {
    code,
    output,
    blobHash: match?.[1] ?? '',
    authoredActionHash: action?.[1],
  };
}

/**
 * Retry budget mirroring stage-spa-blob.sh's STAGE_BLOB_BUDGET_SECS /
 * STAGE_BLOB_ATTEMPTS / MAX_WAIT_SECS (scripts/ci/stage-spa-blob.sh ~13-26,
 * ~554-574,607-619) — read from the same env vars so an operator tuning the
 * shell ladder tunes this direct-to-storage path identically.
 */
export interface StorageRetryBudget {
  /** Total wall-clock retry budget in seconds (default 360, clamped to the caller's step). */
  budgetSecs: number;
  /** Safety-net ceiling on attempt count on top of the budget (default 60). */
  attempts: number;
  /** Cap on any single backoff wait, including an advertised retryAfter (default 60). */
  maxWaitSecs: number;
}

/**
 * Deadline-aware default: when a caller hands us its own cucumber step
 * timeout, the budget is clamped strictly inside it (minus `marginMs`, which
 * has to leave room for the seatbelt verify fetch and the exhaustion error's
 * own formatting after the ladder gives up). A 360s budget inside a 300s step
 * is exactly the defect this guards — cucumber's generic `function timed out`
 * kills the step first and every diagnostic the ladder would have thrown is
 * lost. `STAGE_BLOB_BUDGET_SECS` is still honored, but only up to that same
 * ceiling — an operator cannot configure a budget that outlives its step.
 * With no `stepTimeoutMs` (no step context), behavior matches the historical
 * unbounded default.
 */
export function defaultStorageRetryBudget(
  opts: { stepTimeoutMs?: number; marginMs?: number } = {}
): StorageRetryBudget {
  const marginMs = opts.marginMs ?? 20_000;
  const envSecs = Number(process.env['STAGE_BLOB_BUDGET_SECS'] ?? 360);
  const rawBudgetSecs = Number.isFinite(envSecs) && envSecs > 0 ? envSecs : 360;
  const ceilingSecs =
    opts.stepTimeoutMs === undefined
      ? Number.POSITIVE_INFINITY
      : Math.max(5, (opts.stepTimeoutMs - marginMs) / 1000);
  return {
    budgetSecs: Math.min(rawBudgetSecs, ceilingSecs),
    attempts: Number(process.env['STAGE_BLOB_ATTEMPTS'] ?? 60),
    maxWaitSecs: 60,
  };
}

/** A backpressure/lag answer the ladder should re-offer within budget. */
export class RetryableStageShed extends Error {
  /** HTTP body of the last answer that produced this shed, trimmed to ~300 chars. */
  public readonly bodySnippet?: string;

  constructor(
    message: string,
    public readonly retryAfterSecs?: number,
    /** Which leg of the publish produced this shed — 'blob upload' | 'PATCH' | 'seatbelt verify'. */
    public readonly leg?: string,
    public readonly httpStatus?: number,
    bodySnippet?: string
  ) {
    super(message);
    this.name = 'RetryableStageShed';
    this.bodySnippet = bodySnippet?.slice(0, 300);
  }
}

/** A structural answer the peer has already given — retrying just burns budget. */
export class NonRetryableStageError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'NonRetryableStageError';
  }
}

/**
 * Classify one storage HTTP response the way stage-spa-blob.sh's stage_once
 * PATCH leg does: 503/429 is backpressure (retryable, honoring an advertised
 * Retry-After); a "not retrievable" 4xx is the pre-existing DHT-publish-lag
 * class (also retryable, no hint); any OTHER 4xx is a structural,
 * non-retryable answer; anything else non-2xx (5xx, transport-adjacent) is a
 * generic retryable failure, matching the shell's bare `*)` arm.
 *
 * One class the shell carries and this does not: a `CellDisabled` body says an
 * installed cell is not among the conductor's running cells, which measurement
 * says usually clears on its own (2026-09-21 — up to ~11min after a
 * local-household start, 0-4min after a fleet conductor's "Conductor ready."
 * line). The shell waits it out on a SEPARATE per-doorway-host budget and, if
 * that budget runs out, reports "still unavailable after Ns" rather than a
 * cause. Here it stays plainly retryable, which is the same direction and the
 * right one for a scenario's own bounded retry.
 */
export function classifyStorageResponse(
  status: number,
  body: string,
  retryAfterHeader: string | null | undefined,
  leg = 'PATCH'
): { ok: true } | RetryableStageShed | NonRetryableStageError {
  if (status >= 200 && status < 300) return { ok: true };
  const headerSecs =
    retryAfterHeader === null || retryAfterHeader === undefined
      ? Number.NaN
      : Number(retryAfterHeader);
  const bodyMatch = /"retryAfter"\s*:\s*(\d+)/.exec(body)?.[1];
  const bodySecs = bodyMatch === undefined ? Number.NaN : Number(bodyMatch);
  let retryAfterSecs: number | undefined;
  if (Number.isFinite(headerSecs)) retryAfterSecs = headerSecs;
  else if (Number.isFinite(bodySecs)) retryAfterSecs = bodySecs;
  if (status === 503 || status === 429) {
    return new RetryableStageShed(
      `peer shed (HTTP ${status}): ${body}`,
      retryAfterSecs,
      leg,
      status,
      body
    );
  }
  if (status >= 400 && status < 500) {
    if (body.includes('not retrievable')) {
      return new RetryableStageShed(
        `target not retrievable yet (HTTP ${status}): ${body}`,
        undefined,
        leg,
        status,
        body
      );
    }
    return new NonRetryableStageError(`structural failure (HTTP ${status}): ${body}`);
  }
  return new RetryableStageShed(
    `peer failed (HTTP ${status}): ${body}`,
    undefined,
    leg,
    status,
    body
  );
}

/** Minimum remaining budget a new attempt is allowed to start with. */
const MIN_REMAINING_SECS_FOR_ANOTHER_ATTEMPT = 5;

/**
 * Build the terminal, non-retryable error thrown when the ladder exhausts its
 * budget or attempt count — names the leg, attempt count, elapsed time, and
 * the last answer seen, since that is exactly the diagnostic evidence a
 * generic cucumber `function timed out` would otherwise erase.
 */
function storageLadderExhaustedError(
  shed: RetryableStageShed,
  attemptNumber: number,
  elapsedSecs: number
): Error {
  const leg = shed.leg ?? 'unknown leg';
  const lastStatus = shed.httpStatus === undefined ? 'n/a' : String(shed.httpStatus);
  const lastBody = shed.bodySnippet ?? shed.message.slice(0, 300);
  const lastRetryAfter = shed.retryAfterSecs === undefined ? 'n/a' : `${shed.retryAfterSecs}s`;
  return new Error(
    `staging ladder exhausted at leg "${leg}" after ${attemptNumber} attempt(s) / ` +
      `${elapsedSecs.toFixed(1)}s elapsed — last HTTP status: ${lastStatus}, ` +
      `last body: ${JSON.stringify(lastBody)}, last Retry-After: ${lastRetryAfter}`
  );
}

/**
 * Bounded-budget retry loop mirroring stage-spa-blob.sh's outer while-loop
 * (scripts/ci/stage-spa-blob.sh ~577-623): a `NonRetryableStageError` from
 * `attempt` aborts immediately (never retried); a `RetryableStageShed` backs
 * off by its own `retryAfterSecs` hint (capped at `maxWaitSecs`) or, absent a
 * hint, by `attempt * 5` seconds (also capped) — until either the attempt
 * count or the wall-clock budget is exhausted, at which point a terminal,
 * diagnostic-bearing error is thrown (`storageLadderExhaustedError`).
 *
 * Every backoff is clamped to the remaining wall-clock budget so the ladder
 * never sleeps past its own deadline, and a new attempt never starts with
 * less than `MIN_REMAINING_SECS_FOR_ANOTHER_ATTEMPT` left — at that point the
 * ladder exhausts immediately instead of gambling one more round-trip it
 * cannot afford to finish and report on.
 */
export async function withStorageRetryBudget<T>(
  attempt: (attemptNumber: number) => Promise<T>,
  opts: {
    budget?: StorageRetryBudget;
    sleep?: (seconds: number) => Promise<void>;
    now?: () => number;
    /** Log-line prefix identifying which caller is retrying. */
    label?: string;
  } = {}
): Promise<T> {
  const budget = opts.budget ?? defaultStorageRetryBudget();
  const sleep =
    opts.sleep ??
    (async (seconds: number) => {
      await new Promise(resolve => setTimeout(resolve, seconds * 1000));
    });
  const now = opts.now ?? Date.now;
  const label = opts.label ?? 'storage-retry-budget';
  const startedAt = now();
  let attemptNumber = 0;
  for (;;) {
    attemptNumber += 1;
    try {
      return await attempt(attemptNumber);
    } catch (error) {
      if (error instanceof NonRetryableStageError) throw error;
      const shed =
        error instanceof RetryableStageShed
          ? error
          : new RetryableStageShed((error as Error).message ?? String(error));
      const elapsedSecs = (now() - startedAt) / 1000;
      const remainingSecs = budget.budgetSecs - elapsedSecs;
      if (
        attemptNumber >= budget.attempts ||
        remainingSecs <= MIN_REMAINING_SECS_FOR_ANOTHER_ATTEMPT
      ) {
        throw storageLadderExhaustedError(shed, attemptNumber, elapsedSecs);
      }
      const waitSecs = Math.max(
        0,
        Math.min(
          shed.retryAfterSecs ?? attemptNumber * 5,
          budget.maxWaitSecs,
          remainingSecs - MIN_REMAINING_SECS_FOR_ANOTHER_ATTEMPT
        )
      );
      console.warn(
        `[${label}] attempt ${attemptNumber} shed at leg "${shed.leg ?? 'unknown'}": ` +
          `${shed.message} — retrying in ${waitSecs.toFixed(1)}s ` +
          `(elapsed ${elapsedSecs.toFixed(1)}s / budget ${budget.budgetSecs.toFixed(1)}s)`
      );
      await sleep(waitSecs);
    }
  }
}

/**
 * The blob hash a `PUT /blob/` reply names, normalized to `sha256-<hex>`.
 *
 * The storage wire is camelCase (`blobHash`); the SDK's hand-written `BlobManifest` still types it
 * `blob_hash`, so read the wire first. A reply naming neither is a shape defect, not a shed — it
 * is refused at once rather than retried for the whole ladder budget.
 */
export function uploadedBlobHash(reply: unknown): string {
  const manifest = (reply ?? {}) as { blobHash?: unknown; blob_hash?: unknown };
  const wireHash = manifest.blobHash ?? manifest.blob_hash;
  if (typeof wireHash !== 'string' || wireHash.length === 0) {
    throw new NonRetryableStageError(
      `source storage answered the blob upload without a blob hash: ${JSON.stringify(reply).slice(0, 200)}`
    );
  }
  return wireHash.startsWith('sha256-') ? wireHash : `sha256-${wireHash}`;
}

/**
 * Upload and author through one storage peer, without touching a survivor
 * doorway — the intent b344f533a fixed (elohim/dev, publish authority
 * through source storage: the PATCH must reach the AUTHOR/source storage
 * peer directly, never via a doorway). That commit dropped the shed-aware
 * retry ladder AND the post-PATCH seatbelt GET that stage-spa-blob.sh's
 * stage_once carries (~376-458); both are restored here, ported rather than
 * re-invoked because stage-spa-blob.sh's byte-upload leg is hard-wired to
 * the doorway-only `/admin/seed/blob` route (doorway/doorway-service/src/
 * routes/seed.rs) and has no source-storage target override.
 *
 * The whole attempt (re-upload + PATCH + seatbelt) is retried as a unit on a
 * shed: re-uploading identical content-addressed bytes is idempotent-safe
 * (stage-spa-blob.sh's own re-PUT-short-circuit comment, ~313-320), so this
 * matches the shell ladder's behavior of retrying `stage_once` wholesale.
 */
export async function stageBundleThroughStorage(opts: {
  bundle: FixtureBundle;
  slug: string;
  storageUrl: string;
  budget?: StorageRetryBudget;
  /**
   * The calling cucumber step's own `{ timeout }`. Used (when `budget` is not
   * given explicitly) to derive a budget that is strictly inside the step's
   * deadline — see `defaultStorageRetryBudget`.
   */
  stepTimeoutMs?: number;
  marginMs?: number;
  fetchFn?: typeof fetch;
  sleep?: (seconds: number) => Promise<void>;
  now?: () => number;
}): Promise<StageOutcome> {
  const archiveDir = mkdtempSync(join(fixtureRoot(), 'source-author-package-'));
  const doFetch = opts.fetchFn ?? fetch;
  try {
    const archive = join(archiveDir, 'browser.zip');
    await execFileAsync('zip', ['-X', '-qr', archive, '.'], { cwd: opts.bundle.dir });
    const bytes = readFileSync(archive);
    const expectedHash = `sha256-${createHash('sha256').update(bytes).digest('hex')}`;
    const apiKey = process.env['STORAGE_API_KEY_ADMIN'] ?? '';
    const client = new StorageClient({ baseUrl: opts.storageUrl, apiKey, timeout: 30_000 });

    return await withStorageRetryBudget(
      async () => {
        let uploadedHash: string;
        try {
          uploadedHash = uploadedBlobHash(
            await client.putBlob(new Uint8Array(bytes), 'application/zip')
          );
        } catch (error) {
          if (error instanceof NonRetryableStageError || error instanceof RetryableStageShed)
            throw error;
          throw new RetryableStageShed(
            `blob upload failed: ${(error as Error).message ?? String(error)}`,
            undefined,
            'blob upload'
          );
        }
        if (uploadedHash !== expectedHash) {
          throw new NonRetryableStageError(
            `source storage returned a different blob hash: expected ${expectedHash}, got ${uploadedHash}`
          );
        }

        let response: Response;
        let body: string;
        try {
          response = await doFetch(`${opts.storageUrl}/db/content/${opts.slug}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', 'X-API-Key': apiKey },
            body: JSON.stringify({ blobHash: expectedHash }),
            signal: AbortSignal.timeout(30_000),
          });
          body = await response.text();
        } catch (error) {
          if (error instanceof NonRetryableStageError || error instanceof RetryableStageShed)
            throw error;
          throw new RetryableStageShed(
            `PATCH request failed: ${(error as Error).message ?? String(error)}`,
            undefined,
            'PATCH'
          );
        }
        const verdict = classifyStorageResponse(
          response.status,
          body,
          response.headers.get('retry-after'),
          'PATCH'
        );
        if (verdict instanceof Error) throw verdict;

        const action = (JSON.parse(body) as { dhtAnchorHash?: string }).dhtAnchorHash;
        if (!action) {
          throw new NonRetryableStageError(
            `source storage publication returned no exact action: ${body}`
          );
        }

        // Seatbelt (stage-spa-blob.sh ~450-458): re-GET the row and confirm the
        // hash field actually landed before calling this staged.
        let verifyResponse: Response;
        let verifyText: string;
        try {
          verifyResponse = await doFetch(`${opts.storageUrl}/db/content/${opts.slug}`, {
            signal: AbortSignal.timeout(30_000),
          });
          verifyText = await verifyResponse.text();
        } catch (error) {
          if (error instanceof NonRetryableStageError || error instanceof RetryableStageShed)
            throw error;
          throw new RetryableStageShed(
            `seatbelt verify failed: ${(error as Error).message ?? String(error)}`,
            undefined,
            'seatbelt verify'
          );
        }
        if (!verifyResponse.ok) {
          throw new NonRetryableStageError(
            `seatbelt GET failed (HTTP ${verifyResponse.status}): ${verifyText}`
          );
        }
        const actual = (JSON.parse(verifyText) as { blobHash?: string }).blobHash;
        if (actual !== expectedHash) {
          throw new NonRetryableStageError(
            `blobHash drift after PATCH: expected ${expectedHash}, got ${actual ?? '<empty>'}`
          );
        }

        const outcome: StageOutcome = {
          code: 0,
          output: body,
          blobHash: expectedHash,
          authoredActionHash: action,
        };
        return outcome;
      },
      {
        budget:
          opts.budget ??
          defaultStorageRetryBudget({
            stepTimeoutMs: opts.stepTimeoutMs,
            marginMs: opts.marginMs,
          }),
        sleep: opts.sleep,
        now: opts.now,
        label: 'stage-through-storage',
      }
    );
  } finally {
    rmSync(archiveDir, { recursive: true, force: true });
  }
}

export interface BrowserVisit {
  url: string;
  /** Uncaught JS errors the page threw. */
  pageErrors: string[];
  /** Network-level failures (aborted, DNS, connection refused). */
  failedRequests: { url: string; failure: string }[];
  /** HTTP responses the page received with a status >= 400. */
  httpErrors: { url: string; status: number }[];
  /** Text content of the app's root element after load, trimmed. */
  rootText: string;
  /** True when an `<app-root>` element was present at all. */
  rootPresent: boolean;
  bootstrapReady: boolean;
}

/**
 * Open one URL in a real headless browser and report what the browser saw.
 *
 * Deliberately NOT PlaywrightDevice: that device exists to drive a logged-in
 * human through the app, and every assertion here is about a page a stranger is
 * handed before any identity exists. Same capture surface (console, pageerror,
 * requestfailed) plus the HTTP error RESPONSES `look` had to add, because a
 * 404'd entry script never appears as a network FAILURE.
 */
export async function visitInBrowser(url: string, timeoutMs = 30_000): Promise<BrowserVisit> {
  const { captureBrowserShell } = await import('../../scripts/browser-shell.js');
  const started = Date.now();
  const capture = await captureBrowserShell(url, timeoutMs);
  if (Date.now() - started > 45_000)
    capture.pageErrors.push('Browser visit exceeded its 45-second deadline');
  return capture;
}

/** Poll `check` until it returns true or the bound expires; returns the elapsed ms or null. */
export async function pollUntil(
  check: () => Promise<boolean>,
  boundMs: number,
  intervalMs = 3_000
): Promise<number | null> {
  if (boundMs <= 0) return null;
  const started = Date.now();
  for (;;) {
    const ready = await check().catch(() => false);
    if (Date.now() - started >= boundMs) return null;
    if (ready) return Date.now() - started;
    await new Promise(done => setTimeout(done, intervalMs));
  }
}

/** Only the requests the browser sent to THIS doorway — nobody else's contract. */
export function sameOrigin(entryUrl: string, candidate: string): boolean {
  try {
    return new URL(candidate).origin === new URL(entryUrl).origin;
  } catch {
    return false;
  }
}

/** Server fixture implements the same export contract AngularRenderer loads. */
export function buildServerFixture(browser: FixtureBundle): FixtureBundle {
  const dir = mkdtempSync(join(fixtureRoot(), 'epr-server-'));
  writeFileSync(join(dir, 'version.json'), readFileSync(join(browser.dir, 'version.json')));
  const html = readFileSync(join(browser.dir, 'index.html'), 'utf8').replace(
    '<app-root></app-root>',
    `<app-root data-ssr-stamp="${browser.stamp}">SSR ${browser.stamp}</app-root>`
  );
  writeFileSync(
    join(dir, 'main.server.mjs'),
    `export default function bootstrap() {}\nexport async function renderApplication() { return ${JSON.stringify(html)}; }\n`
  );
  return { ...browser, dir };
}

export async function meshControl(action: string, ...args: string[]): Promise<void> {
  await execFileAsync(
    'bash',
    [join(REPO_ROOT, 'app/elohim-app/scripts/hc-mesh.sh'), action, ...args],
    {
      cwd: REPO_ROOT,
      env: process.env,
      timeout: 180_000,
      maxBuffer: 4 * 1024 * 1024,
    }
  );
}

const ownedDoorwayPairs = new WeakMap<E2EWorld, OwnedDoorwayPair>();

export async function startOwnedDoorwayPair(
  world: E2EWorld,
  fixture: HouseholdMeshFixture
): Promise<OwnedDoorwayPair> {
  const pair = await OwnedDoorwayPair.start({ storagePeers: fixture.storagePeers ?? {} });
  ownedDoorwayPairs.set(world, pair);
  return pair;
}

export function ownedDoorwayPair(world: E2EWorld): OwnedDoorwayPair | undefined {
  return ownedDoorwayPairs.get(world);
}

export async function restartStoryDoorway(
  world: E2EWorld,
  name: string,
  extraSsrSlug?: string
): Promise<void> {
  const pair = ownedDoorwayPairs.get(world);
  if (!pair) return meshControl('doorway-restart', name, ...(extraSsrSlug ? [extraSsrSlug] : []));
  assert.ok(name === 'a' || name === 'b', `story doorway must be a or b (got ${name})`);
  await pair.restart(name, extraSsrSlug);
}

/** Deliberate fault injection after the normal package/publish path refused. */
export async function stageInvalidFixture(
  bundle: FixtureBundle,
  doorwayUrl: string
): Promise<string> {
  if (bundle.coherent)
    throw new Error('fault injector accepts only a deliberately incoherent fixture');
  const archiveDir = mkdtempSync(join(fixtureRoot(), 'invalid-package-'));
  try {
    const archive = join(archiveDir, 'invalid.zip');
    await execFileAsync('zip', ['-X', '-qr', archive, '.'], { cwd: bundle.dir });
    const bytes = readFileSync(archive);
    const hash = `sha256-${createHash('sha256').update(bytes).digest('hex')}`;
    const response = await fetch(`${doorwayUrl}/admin/seed/blob`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/zip',
        'X-Blob-Hash': hash,
        'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? '',
      },
      body: new Uint8Array(bytes),
      signal: AbortSignal.timeout(30_000),
    });
    const body = await response.text();
    if (
      !response.ok ||
      !(JSON.parse(body) as { forwarded_to_storage?: boolean }).forwarded_to_storage
    ) {
      throw new Error(`fault fixture did not reach storage: ${response.status} ${body}`);
    }
    return hash;
  } finally {
    rmSync(archiveDir, { recursive: true, force: true });
  }
}

/** Recorded PID plus start tick: PID reuse cannot disguise a renderer restart. */
export function doorwayIncarnations(world?: E2EWorld): string[] {
  const pair = world && ownedDoorwayPairs.get(world);
  if (pair) return pair.incarnations();

  const mesh = householdMeshDir();
  return ['a', 'b'].map(name => readFileSync(join(mesh, 'pids', `doorway-${name}`), 'utf8').trim());
}

/** Read only the owned doorway's restart log; offsets distinguish this incarnation. */
export function doorwayRestartLog(name: string, world?: E2EWorld): string {
  const pair = world && ownedDoorwayPairs.get(world);
  if (pair && (name === 'a' || name === 'b')) return pair.log(name);

  const mesh = householdMeshDir();
  try {
    return readFileSync(join(mesh, 'logs', `doorway-restart-${name}.log`), 'utf8');
  } catch {
    return '';
  }
}

/** Fixture authoring retries only the conductor's explicit optimistic-write conflict. */
export async function postFixtureCommitment(url: string, init: RequestInit) {
  const deadline = Date.now() + 60_000;
  for (let attempt = 0; ; attempt++) {
    const response = await fetch(url, {
      ...init,
      signal: AbortSignal.timeout(Math.max(1, deadline - Date.now())),
    });
    const text = await response.text();
    if (
      response.status !== 503 ||
      !text.includes('source chain head has moved since the bundle began') ||
      Date.now() >= deadline
    ) {
      return { ok: response.ok, status: response.status, text };
    }
    await new Promise(resolve =>
      setTimeout(resolve, Math.min(5000, 1000 * (attempt + 1), Math.max(1, deadline - Date.now())))
    );
  }
}
