/**
 * Common step definitions — health checks, doorway readiness, lifecycle hooks.
 */

import { strict as assert } from 'node:assert';
import { mkdirSync, writeFileSync } from 'node:fs';

import {
  Given,
  Before,
  After,
  AfterAll,
  BeforeAll,
  Status,
  setDefaultTimeout,
  setWorldConstructor,
} from '@cucumber/cucumber';

/**
 * The work window a step has always had here — remote doorway calls and retry
 * loops need far longer than cucumber's 5s default. This number is the
 * PRODUCT's time, and nothing in this file may spend it.
 *
 * Exported because the browser-step legibility helpers below size their budget
 * against it; the two numbers must never drift apart.
 */
export const CUCUMBER_STEP_TIMEOUT_MS = 30_000;

/**
 * Reserve added ON TOP of the work window so `browserStep` can turn a hung
 * browser step into a diagnosis instead of a bare "function timed out" line.
 *
 * It is 1s, not one probe timeout, because the probe does NOT run in this
 * window: `browserStep` fires it IN PARALLEL with the still-running work (see
 * the speculative probe below), so by the time the budget expires the answer
 * is already in hand and all that is left is formatting it.
 */
export const BROWSER_STEP_DIAGNOSIS_RESERVE_MS = 1_000;

/**
 * What cucumber actually kills a step at: the unchanged work window plus the
 * diagnosis reserve. Raising the deadline by exactly the reserve is what keeps
 * `browserStep` from taking its diagnosis out of the product's time — a step
 * that settled at 29s before still settles at 29s now.
 */
export const CUCUMBER_STEP_DEADLINE_MS =
  CUCUMBER_STEP_TIMEOUT_MS + BROWSER_STEP_DIAGNOSIS_RESERVE_MS;

setDefaultTimeout(CUCUMBER_STEP_DEADLINE_MS);

import { getRaw } from '../src/framework/dataplane/surfaces.js';
import {
  PlaywrightDevice,
  type CapturedConsoleLog,
  type CapturedPageError,
} from '../src/framework/devices/playwright-device.js';
import {
  isRemoteComputeAvailable,
  autoSkippedHumans,
  resetAutoSkippedHumans,
} from '../src/framework/fixtures/humans.js';
import {
  unavailableRequiredCaps,
  noteSubstrateSkip,
  substrateSkippedScenarios,
  resetSubstrateSkips,
} from '../src/framework/fixtures/substrate-scope.js';
import {
  isTestnetActive,
  stopTestnet,
  getComputeSummary,
  writeComputeReport,
} from '../src/framework/testnet-manager.js';
import { isSpaRoutingNoise } from '../src/framework/utils/console-filters.js';
import { retry } from '../src/framework/utils/retry.js';
import { E2EWorld } from '../src/framework/world.js';

setWorldConstructor(E2EWorld);

// ---------------------------------------------------------------------------
// Browser-step legibility — a browser step must never die as a bare timeout
// ---------------------------------------------------------------------------

/*
 * When a Playwright step outruns the cucumber step timeout, cucumber kills it
 * with exactly one line:
 *
 *   Error: function timed out, ensure the promise resolves within 30000 milliseconds
 *
 * That message names nothing — not the URL under test, not whether anything
 * answered it, not whether a document was served and the app then hung. Build
 * elohim-genesis/dev #1489 lost 30 browser scenarios to it, and no reader of
 * that report could tell a shedding fleet from a broken app.
 *
 * `browserStep()` closes that gap. It runs the browser work under a budget and
 * on ANY failure — the budget expiring, or Playwright throwing first — reports
 * a plain-HTTP (no browser) look at the same target: the URL, its direct status
 * (or transport error), the doorway's /health answer, and a verdict that
 * separates FLEET-SIDE from APP-SIDE.
 *
 * The budget is the FULL former work window (`CUCUMBER_STEP_TIMEOUT_MS`), not a
 * slice of it. Two things pay for the diagnosis instead of the product:
 *   1. the cucumber deadline is raised by `BROWSER_STEP_DIAGNOSIS_RESERVE_MS`,
 *      so the reserve is added, never subtracted; and
 *   2. the probe is SPECULATIVE — fired one probe-timeout (plus a settling
 *      margin) before the budget expires, in parallel with the work that is
 *      still running — so when the budget does expire the probe has already
 *      answered.
 * An earlier revision took 2 x REACH_PROBE_TIMEOUT_MS off the work window,
 * which was wrong twice over: `probeReach` runs its two probes with
 * `Promise.all`, so the reserve was never 2x, and taking it from the work at
 * all failed steps that legitimately settled between 22s and 30s.
 *
 * It does NOT convert a failure into a skip: a failing scenario still fails, it
 * just says what was unreachable.
 *
 * The HTTP probe reuses `getRaw` from src/framework/dataplane/surfaces.ts (the
 * existing bounded, never-throws-on-non-2xx probe primitive) rather than
 * adding a rival client. `classifyDoorwayState()` from the same module is the
 * closest existing diagnostic, but it is hardcoded to 10s per request across
 * up to three requests — three times this helper's whole remaining budget —
 * and it only ever classifies `/`, never the deep-link URL actually under test.
 *
 * THE INVARIANT, stated so it can be checked rather than believed: no path
 * through `browserStep` can outlive `CUCUMBER_STEP_DEADLINE_MS`. Every promise
 * it awaits is under a deadline — the work under `budgetMs`, the diagnosis
 * probe under whatever is LEFT of the step deadline — and the probe's own
 * `timeoutMs` is now a true total bound in surfaces.ts (it did not cover TCP
 * connect before, so a "4000ms" probe against an unroutable address ran
 * 10.5s and got the whole step killed with the bare one-liner this helper
 * exists to abolish). Raising the cucumber deadline is NOT an available fix
 * for a violation of this invariant: it would hide the bug and lengthen every
 * failing browser step in CI.
 */

/**
 * Per-request bound on each reach probe. The two probes run in parallel.
 *
 * Honoured end to end by `getRaw` (connect + headers + body), NOT just by
 * undici's per-phase timeouts — see `RequestBoundExceeded` in
 * src/framework/dataplane/surfaces.ts. All the budget arithmetic below is only
 * as true as that.
 */
export const REACH_PROBE_TIMEOUT_MS = 4_000;

/**
 * Extra lead time on the speculative probe, so that at budget-expiry the probe
 * has SETTLED rather than merely started. It is taken from the speculation
 * lead, never from the work: the probe runs in parallel with browser work that
 * still gets its full `BROWSER_STEP_BUDGET_MS`.
 */
export const PROBE_SPECULATION_MARGIN_MS = 500;

/**
 * How long browser work gets before `browserStep` cuts it off and diagnoses:
 * the whole unchanged work window. The room the diagnosis needs comes from
 * `CUCUMBER_STEP_DEADLINE_MS` being raised above it, never from this.
 */
export const BROWSER_STEP_BUDGET_MS = CUCUMBER_STEP_TIMEOUT_MS;

/** Result of a direct, browser-free HTTP look at what a browser step could not reach. */
export interface ReachProbe {
  /** The exact URL the browser step was trying to reach. */
  targetUrl: string;
  /** Status of a direct GET of `targetUrl` — null when nothing answered at all. */
  targetStatus: number | null;
  /** Transport-level failure text when the direct GET produced no status. */
  targetError?: string;
  /** Flattened head of the target's body — makes a `catching-up` shed self-identifying. */
  targetBody?: string;
  /** The /health URL probed alongside (the doorway this device is bound to). */
  healthUrl: string;
  healthStatus: number | null;
  healthError?: string;
  /** `healthy` off the /health body, when it answered and parsed. */
  healthHealthy?: boolean;
  /** `status` off the /health body ('online' | 'degraded' | 'offline' | …). */
  healthState?: string;
}

/** Collapse whitespace and cap length so a body/stack fragment stays one readable line. */
function flatten(text: string, max: number): string {
  const line = text.replaceAll(/\s+/g, ' ').trim();
  return line.length > max ? `${line.slice(0, max)}…` : line;
}

/** Indent every line after the first, so a multi-line cause stays aligned under its label. */
function indentContinuation(text: string, indent: string): string {
  return text
    .split('\n')
    .map((line, i) => (i === 0 ? line : `${indent}${line}`))
    .join('\n');
}

/**
 * GET a URL bounded to the probe timeout. A transport failure is a RESULT here,
 * not a throw — "never answered" is precisely one of the facts being reported.
 */
async function probeUrl(
  url: string
): Promise<{ status: number | null; text: string; error?: string }> {
  try {
    const { status, text } = await getRaw(url, { timeoutMs: REACH_PROBE_TIMEOUT_MS });
    return { status, text };
  } catch (err) {
    return {
      status: null,
      text: '',
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/** Best-effort read of `healthy` / `status` out of a /health body. */
function readHealthBody(text: string): { healthy?: boolean; state?: string } {
  if (!text) return {};
  try {
    const body = JSON.parse(text) as { healthy?: unknown; status?: unknown };
    const healthy = typeof body.healthy === 'boolean' ? { healthy: body.healthy } : {};
    const state = typeof body.status === 'string' ? { state: body.status } : {};
    return { ...healthy, ...state };
  } catch {
    return {};
  }
}

/**
 * Probe the URL a browser step could not reach, plus the /health of the
 * doorway serving it. Both requests are bounded and run in parallel.
 */
export async function probeReach(targetUrl: string, doorwayUrl: string): Promise<ReachProbe> {
  let doorwayBase = doorwayUrl;
  while (doorwayBase.endsWith('/')) doorwayBase = doorwayBase.slice(0, -1);
  const healthUrl = `${doorwayBase}/health`;
  const [target, health] = await Promise.all([probeUrl(targetUrl), probeUrl(healthUrl)]);
  const parsedHealth = readHealthBody(health.text);
  return {
    targetUrl,
    targetStatus: target.status,
    targetError: target.error,
    targetBody: target.text ? flatten(target.text, 160) : undefined,
    healthUrl,
    healthStatus: health.status,
    healthError: health.error,
    healthHealthy: parsedHealth.healthy,
    healthState: parsedHealth.state,
  };
}

/**
 * The one line a report reader needs: was this the fleet or the app?
 *
 * A 200 on the target means a document WAS served, so a browser step that
 * still hung is an app-side defect. A 503, or no answer at all, is the fleet.
 */
export function reachVerdict(probe: ReachProbe): string {
  const { targetStatus, healthStatus } = probe;
  if (targetStatus === null && healthStatus === null) {
    return 'NOTHING ANSWERED — neither the target URL nor /health produced an HTTP response. The fleet is down or this run is pointed at an env that does not exist; not an app defect.';
  }
  if (targetStatus === null) {
    return `FLEET-SIDE — /health answered ${healthStatus} but the target URL produced no HTTP response at all. The app origin is unreachable from this runner.`;
  }
  if (targetStatus === 503) {
    return 'FLEET-SIDE — the target answered 503 (doorway shedding / catching-up). The browser was handed no document to render.';
  }
  if (targetStatus >= 500) {
    return `SERVER-SIDE — the target answered ${targetStatus}; the origin is up but failing this URL.`;
  }
  if (targetStatus === 404) {
    return 'ROUTE-SIDE — the target answered 404. This URL is not served by this origin (wrong env, or the route is not mounted).';
  }
  if (targetStatus >= 400) {
    return `CLIENT-SIDE — the target answered ${targetStatus}; the request itself was refused.`;
  }
  if (targetStatus >= 300) {
    return `REDIRECT — the target answered ${targetStatus} (this probe does not follow redirects).`;
  }
  return `APP-SIDE — the target answered ${targetStatus}, so a document WAS served: the browser step hung AFTER a good response (bundle never settled, or the awaited element never appeared).`;
}

/** Render a ReachProbe as the block appended to a failed browser step's message. */
export function formatReachProbe(probe: ReachProbe): string {
  let target: string;
  if (probe.targetStatus === null) {
    target = `no HTTP response (${probe.targetError ?? 'unknown transport error'})`;
  } else if (probe.targetBody) {
    target = `${probe.targetStatus} body="${probe.targetBody}"`;
  } else {
    target = `${probe.targetStatus} (empty body)`;
  }

  let health: string;
  if (probe.healthStatus === null) {
    health = `no HTTP response (${probe.healthError ?? 'unknown transport error'})`;
  } else {
    health = `${probe.healthStatus} healthy=${probe.healthHealthy ?? '?'} status=${probe.healthState ?? '?'}`;
  }

  return [
    `  target : GET ${probe.targetUrl} -> ${target}`,
    `  health : GET ${probe.healthUrl} -> ${health}`,
    `  verdict: ${reachVerdict(probe)}`,
  ].join('\n');
}

/**
 * The origin a PlaywrightDevice resolves relative paths against.
 *
 * `PlaywrightDevice.appUrl` is `private` to TypeScript only — it is a plain
 * runtime field, and it is the ONLY record of that origin: the landing visitor
 * is pointed straight at the doorway while the deep-link visitor is pointed at
 * the paired app origin, so it cannot be re-derived from the world. Read it
 * structurally, falling back to the device's doorway client URL.
 */
function deviceAppUrl(device: PlaywrightDevice): string {
  const appUrl = (device as unknown as { appUrl?: unknown }).appUrl;
  return typeof appUrl === 'string' && appUrl ? appUrl : device.client.url;
}

/** Where the browser actually sits right now, or null if it never loaded a document. */
function currentPageUrl(device: PlaywrightDevice): string | null {
  try {
    const url = device.page.url();
    return url && url !== 'about:blank' ? url : null;
  } catch {
    return null;
  }
}

/** Absolute URL the step was aiming at: an explicit target, else wherever the page sits. */
function resolveTargetUrl(device: PlaywrightDevice, target: string | null): string {
  if (target?.startsWith('http')) return target;
  const base = deviceAppUrl(device);
  if (target === null) return currentPageUrl(device) ?? base;
  const path = target.startsWith('/') ? target : `/${target}`;
  return `${base}${path}`;
}

/** The browser's own view of what failed to load — often names the exact dead asset. */
function recentFailedRequests(device: PlaywrightDevice): string | null {
  const failed = device.getErrors().network.slice(-3);
  if (failed.length === 0) return null;
  return [
    '  network: last failed request(s) the browser itself captured —',
    ...failed.map(r => `           ${r.method} ${r.url} ${r.failure ?? r.status ?? ''}`.trimEnd()),
  ].join('\n');
}

/** Raised when browser work outruns the in-step budget, so the probe still gets to run. */
class BrowserStepBudgetExceeded extends Error {
  constructor(budgetMs: number) {
    super(
      `browser work did not settle within this step's ${budgetMs}ms budget ` +
        `(cucumber would have killed it at ${CUCUMBER_STEP_DEADLINE_MS}ms with no diagnosis at all)`
    );
    this.name = 'BrowserStepBudgetExceeded';
  }
}

/**
 * A reach probe already in flight for this step, if the speculative timer fired.
 * A box rather than a bare `let` so the value survives the closure boundary.
 */
interface PendingProbe {
  probe?: Promise<ReachProbe>;
}

/**
 * Time left for formatting once the probe has answered. Everything after the
 * probe is synchronous string work (`page.url()` and `getErrors()` are local
 * reads), so this is generous by an order of magnitude — it exists so the
 * arithmetic below can never round the diagnosis right onto the deadline.
 */
const DIAGNOSIS_FORMAT_MARGIN_MS = 250;

/** Build the legible replacement for whatever the browser threw (or failed to finish). */
async function explainBrowserFailure(
  device: PlaywrightDevice,
  what: string,
  target: string | null,
  cause: unknown,
  pending: PendingProbe = {},
  stepStartedAt: number = Date.now()
): Promise<Error> {
  // The probe is bounded by whatever is LEFT of the cucumber deadline, never by
  // hope. This await used to be the one promise in this file with no deadline
  // on it, which is exactly how the helper built to abolish
  // "Error: function timed out …" could itself be killed by it: getRaw's
  // `timeoutMs` did not bound TCP connect (undici's own 10s default did), so a
  // "4000ms" probe against an unreachable fleet ran 10.5s and pushed the step
  // past CUCUMBER_STEP_DEADLINE_MS. surfaces.ts now honours the bound, and this
  // deadline is the belt to that braces: whatever the probe does, the
  // diagnosis prints.
  const elapsed = Date.now() - stepStartedAt;
  const probeBudgetMs = Math.min(
    REACH_PROBE_TIMEOUT_MS,
    CUCUMBER_STEP_DEADLINE_MS - elapsed - DIAGNOSIS_FORMAT_MARGIN_MS
  );

  let probe: ReachProbe | null = null;
  let probeUnavailable: string | undefined;
  if (probeBudgetMs <= 0) {
    probeUnavailable = `no time left before the ${CUCUMBER_STEP_DEADLINE_MS}ms step deadline (${elapsed}ms already spent)`;
  } else {
    try {
      // Prefer the probe already running alongside the work: awaiting it costs
      // the step nothing, where starting one here costs a whole
      // REACH_PROBE_TIMEOUT_MS.
      probe = await withDeadline(
        pending.probe ?? probeReach(resolveTargetUrl(device, target), device.client.url),
        probeBudgetMs,
        () =>
          new Error(`reach probe did not answer within its ${probeBudgetMs}ms share of the step`)
      );
    } catch (probeErr) {
      probeUnavailable = probeErr instanceof Error ? probeErr.message : String(probeErr);
    }
  }
  const causeText = cause instanceof Error ? cause.message : String(cause);

  const lines = [
    `Browser step failed: ${what}`,
    `  cause  : ${indentContinuation(flatten(causeText, 400), '           ')}`,
    probe
      ? formatReachProbe(probe)
      : `  probe  : NOT RUN — ${probeUnavailable ?? 'unknown reason'} (target was ${resolveTargetUrl(device, target)})`,
    `  page   : ${currentPageUrl(device) ?? '(no document ever loaded)'}`,
  ];
  const network = recentFailedRequests(device);
  if (network) lines.push(network);

  return new Error(lines.join('\n'));
}

/**
 * Run browser work so that its failure NAMES what was unreachable.
 *
 * @param device  the Playwright device driving the work
 * @param what    human description of the attempt, e.g. `open the landing page "/"`
 * @param target  path or absolute URL under test; null = "wherever the page already is"
 * @param work    the browser work itself — unchanged, with its own inner timeouts intact
 * @param opts.budgetMs  override only when a step legitimately needs a different slice
 *
 * Failure stays failure: nothing here is skipped or softened, and no step gets
 * less working time than it had before this helper existed.
 */
export async function browserStep<T>(
  device: PlaywrightDevice,
  what: string,
  target: string | null,
  work: () => Promise<T>,
  opts: { budgetMs?: number } = {}
): Promise<T> {
  const budgetMs = opts.budgetMs ?? BROWSER_STEP_BUDGET_MS;
  // Wall-clock origin for the whole step, so the diagnosis can size its own
  // probe against what is LEFT of CUCUMBER_STEP_DEADLINE_MS rather than
  // assuming the failure arrived exactly at the budget. An early Playwright
  // throw at 5s leaves the probe its full REACH_PROBE_TIMEOUT_MS; a
  // budget-expiry at 30s leaves it the diagnosis reserve, and no path leaves
  // it more time than cucumber will actually allow.
  const startedAt = Date.now();

  // Speculative probe: if the work is still running one probe-timeout (plus a
  // settling margin) before its budget expires, start the diagnosis NOW, in
  // parallel with it. The target URL is resolved here rather than up front so
  // a `null` target still means "wherever the page actually got to". When the
  // work then settles in time the probe result is simply discarded; when it
  // does not, the answer is already waiting and the timeout costs no extra
  // time to explain. The margin buys nothing from the work — the probe runs
  // beside it — and it means that at budget-expiry the pending probe has
  // already SETTLED, so the await in explainBrowserFailure returns at once
  // instead of consuming its share of the diagnosis reserve.
  const speculationLeadMs = REACH_PROBE_TIMEOUT_MS + PROBE_SPECULATION_MARGIN_MS;
  const pending: PendingProbe = {};
  let speculationTimer: ReturnType<typeof setTimeout> | undefined;
  if (budgetMs > speculationLeadMs) {
    speculationTimer = setTimeout(() => {
      const probe = probeReach(resolveTargetUrl(device, target), device.client.url);
      // probeReach never rejects, but keep any future rejection handled so an
      // abandoned speculation can never crash the runner.
      probe.catch(() => undefined);
      pending.probe = probe;
    }, budgetMs - speculationLeadMs);
  }

  try {
    const value = await withDeadline(
      work(),
      budgetMs,
      () => new BrowserStepBudgetExceeded(budgetMs)
    );
    clearTimeout(speculationTimer);
    return value;
  } catch (cause) {
    clearTimeout(speculationTimer);
    throw await explainBrowserFailure(device, what, target, cause, pending, startedAt);
  }
}

/**
 * Bound `work` to `ms`, rejecting with `onTimeout()` if it does not settle.
 *
 * Promise.race subscribes to `work`, so if abandoned work rejects later it
 * stays handled — no unhandled-rejection crash after the deadline passes.
 */
async function withDeadline<T>(work: Promise<T>, ms: number, onTimeout: () => Error): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const deadline = new Promise<never>((_resolve, reject) => {
    timer = setTimeout(() => reject(onTimeout()), ms);
  });
  try {
    return await Promise.race([work, deadline]);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Bound on each best-effort artifact capture in the After hook.
 *
 * A page still hung on a dead origin makes `page.screenshot()` / `saveTrace()`
 * wait indefinitely, which burns the After hook's own 30s timeout and reports
 * a SECOND, contentless failed step for a scenario that already failed
 * legibly. Artifact capture is explicitly best-effort — it just needed a bound.
 */
const ARTIFACT_CAPTURE_TIMEOUT_MS = 8_000;

/** Reason attached to a capture that hit ARTIFACT_CAPTURE_TIMEOUT_MS. */
function captureTimedOut(what: string): Error {
  return new Error(`${what} did not settle within ${ARTIFACT_CAPTURE_TIMEOUT_MS}ms`);
}

/**
 * Compute slugs for a cucumber scenario the same way they're used in
 * observation reports. Idempotent and pure.
 */
function computeSlugs(
  scenarioUri: string,
  scenarioName: string
): {
  featureSlug: string;
  scenarioSlug: string;
} {
  const featureSlug = scenarioUri
    .replace(/^.*features\//, '')
    .replace(/\.feature$/, '')
    .replace(/\//g, '-');
  const scenarioSlug = scenarioName
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  return { featureSlug, scenarioSlug };
}

/**
 * Capture a full-page screenshot for a single device into the per-feature
 * subdir. Called for every Playwright scenario regardless of pass/fail.
 */
async function captureVisualEvidence(
  device: PlaywrightDevice,
  featureSlug: string,
  scenarioSlug: string,
  humanName: string
): Promise<void> {
  const dir = `reports/screenshots/${featureSlug}`;
  mkdirSync(dir, { recursive: true });
  try {
    await withDeadline(
      device.screenshot(`${featureSlug}/${scenarioSlug}--${humanName}`),
      ARTIFACT_CAPTURE_TIMEOUT_MS,
      () => captureTimedOut('screenshot')
    );
  } catch {
    // best-effort — page may have crashed, or still be hung on a dead origin
  }
}

/**
 * Write a sibling .error.json next to the screenshot for failed scenarios so
 * a reviewer can read the failure context without consulting the cucumber JSON.
 */
function writeErrorSidecar(
  featureSlug: string,
  scenarioSlug: string,
  humanName: string,
  failureMessage: string
): void {
  try {
    const path = `reports/screenshots/${featureSlug}/${scenarioSlug}--${humanName}.error.json`;
    writeFileSync(
      path,
      JSON.stringify(
        {
          status: 'failed',
          failureMessage: failureMessage.split('\n')[0],
        },
        null,
        2
      )
    );
  } catch {
    // best-effort
  }
}

/**
 * Capture failure-only artifacts (console errors JSON, Playwright trace) for a single
 * device. The screenshot is captured separately by captureVisualEvidence in the
 * universal-capture path of the After hook.
 */
async function captureFailureArtifactsExceptScreenshot(
  device: PlaywrightDevice,
  safeName: string,
  humanName: string
): Promise<void> {
  const errors = device.getErrors();
  const hasArtifacts = errors.console.length || errors.page.length || errors.network.length;
  if (hasArtifacts) {
    try {
      writeFileSync(
        `reports/console/${safeName}-${humanName}.json`,
        JSON.stringify(errors, null, 2)
      );
    } catch {
      // best-effort
    }
  }

  try {
    await withDeadline(
      device.saveTrace(`FAIL-${safeName}-${humanName}`),
      ARTIFACT_CAPTURE_TIMEOUT_MS,
      () => captureTimedOut('trace save')
    );
  } catch {
    // best-effort
  }
}

/** Collect errors from a single Playwright device and save artifact. */
function collectDeviceErrors(
  device: PlaywrightDevice,
  safeName: string,
  humanName: string
): string[] {
  const consoleErrors: CapturedConsoleLog[] = device.consoleLogs.filter(
    l => l.level === 'error' && !isSpaRoutingNoise(l)
  );
  const pageErrors: CapturedPageError[] = device.pageErrors;

  if (!consoleErrors.length && !pageErrors.length) return [];

  try {
    writeFileSync(
      `reports/console/${safeName}-${humanName}-errors.json`,
      JSON.stringify({ consoleErrors, pageErrors }, null, 2)
    );
  } catch {
    // best-effort
  }

  return [
    ...consoleErrors.map(e => `[${humanName}] console.error: ${e.text} (${e.url})`),
    ...pageErrors.map(e => `[${humanName}] uncaught: ${e.message} (${e.url})`),
  ];
}

/** Collect browser errors from all Playwright devices in the world. */
function collectBrowserErrors(world: E2EWorld, scenarioName: string): string[] {
  const safeName = scenarioName.replace(/[^a-zA-Z0-9]/g, '-');
  const errorReport: string[] = [];

  for (const [name, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        errorReport.push(...collectDeviceErrors(device, safeName, name));
      }
    }
  }

  // A scenario whose SUBJECT is a failure path deliberately provokes a browser
  // error — a wrong-password test that produced no 401 would be the broken one.
  // Such a step registers the error it expects via `world.expectBrowserError`,
  // and only exactly that is forgiven; every other console error still fails the
  // scenario, so the clean-console contract stays intact everywhere else.
  const expected = world.expectedBrowserErrors ?? [];
  if (!expected.length) return errorReport;
  return errorReport.filter(line => !expected.some(rx => rx.test(line)));
}

/**
 * Log runtime context and create report directories at the start of each test run.
 */
BeforeAll(function () {
  mkdirSync('reports/screenshots', { recursive: true });
  mkdirSync('reports/console', { recursive: true });
  mkdirSync('reports/traces', { recursive: true });

  // Reset node-pool auto-skip tracking so this run's reduced-scope summary
  // (emitted in AfterAll) reflects only what THIS run skipped.
  resetAutoSkippedHumans();
  resetSubstrateSkips();

  const env = {
    doorwayAlpha: process.env['E2E_DOORWAY_ALPHA'] ?? '(not set)',
    deviceMode: process.env['E2E_DEVICE_MODE'] ?? 'http',
    trace: process.env['E2E_TRACE'] ?? 'false',
    nodeEnv: process.env['NODE_ENV'] ?? '(not set)',
    remoteComputeStatus: process.env['ELOHIM_REMOTE_COMPUTE_STATUS'] ?? 'unknown',
  };
  // eslint-disable-next-line no-console
  console.log(`\n  Environment: ${JSON.stringify(env, null, 2)}\n`);

  // Loud substrate banner: when the remote pool is down, scenarios that name
  // remote-only personas auto-skip (the step defs return 'pending'). Announce
  // it up front so a reduced-scope run reads as deliberate, not broken.
  if (!isRemoteComputeAvailable()) {
    // eslint-disable-next-line no-console
    console.log(
      '  🛰️  SUBSTRATE: remote pool (shem) UNAVAILABLE — running reduced scope.\n' +
        '     Remote-only persona scenarios will auto-skip; on-prem household carries this run.\n'
    );
  }
});

/**
 * Substrate-scope gate — the RUNTIME arm of the cybernetic scope reconciler.
 *
 * A scenario (or its feature, via inherited tags) may declare `@requires:<cap>` where <cap> is a
 * capability tracked in genesis/manifests/cluster-state.yaml (shem, alpha-cluster-6peer, …). When
 * any required cap is unavailable for this run, the scenario is HELD — skipped, not failed —
 * exactly as the planning arm (scope-reconcile.py) moves a whole-feature artifact to held/. This
 * closes the seam where a scenario that needs the remote canvas but doesn't happen to name a
 * remote-only persona would otherwise run against down pods and fail, masking the real signal.
 *
 * Generic over any cap; nothing here is shem-specific. Registered before the setup hooks so it
 * short-circuits the scenario before any browser/doorway/peer work begins.
 */
Before(function (this: E2EWorld, scenario) {
  const tags = scenario.pickle.tags.map(t => t.name);
  const missing = unavailableRequiredCaps(tags);
  if (missing.length > 0) {
    noteSubstrateSkip(scenario.pickle.name, missing, scenario.pickle.uri);
    // eslint-disable-next-line no-console
    console.log(
      `  ⏭️  HELD (substrate): "${scenario.pickle.name}" requires unavailable ` +
        `${missing.join(', ')} — skipped, not failed.`
    );
    return 'skipped';
  }
  return undefined;
});

/**
 * @wip = work-in-progress: the scenario names a contract whose step definitions are not yet
 * wired (the cure it drives isn't built). The a2o CLAUDE.md documents `@wip` for exactly this,
 * but nothing enforced the skip — so under cucumber's default strict mode a @wip scenario's
 * undefined steps would FAIL the run instead of holding. This makes the convention real: a @wip
 * scenario is HELD (skipped, not failed), the same disposition as a substrate-held one, so a
 * RED-defining contract can live in its feature before its implementation lands (which is when it
 * sheds @wip and goes truly RED→green). Registered before setup so it short-circuits early.
 */
Before(function (this: E2EWorld, scenario) {
  const tags = scenario.pickle.tags.map(t => t.name);
  if (tags.includes('@wip')) {
    // A2O_RUN_WIP=1: deliberately execute @wip scenarios for a LOCAL red
    // measurement (e.g. habit evidence). CI counting is unaffected — the edge
    // Dataplane Validation tag filter (`not @wip`) excludes them upstream of
    // this gate; dropping @wip for CI remains an operator decision.
    if (process.env.A2O_RUN_WIP === '1') {
      // eslint-disable-next-line no-console
      console.log(`  ▶️  RUNNING @wip (A2O_RUN_WIP=1): "${scenario.pickle.name}"`);
      return undefined;
    }
    // eslint-disable-next-line no-console
    console.log(
      `  ⏭️  HELD (@wip): "${scenario.pickle.name}" — steps not yet wired; skipped, not failed.`
    );
    return 'skipped';
  }
  return undefined;
});

/**
 * Clear Playwright capture state before each scenario so errors don't bleed across scenarios.
 * Also begins an observation session on the first registered doorway (best-effort).
 */
Before(async function (this: E2EWorld, scenario) {
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        device.clearCapture();
      }
    }
  }

  const { featureSlug, scenarioSlug } = computeSlugs(scenario.pickle.uri, scenario.pickle.name);
  this.featureSlug = featureSlug;
  this.scenarioSlug = scenarioSlug;

  // Start observation session if a doorway is registered
  for (const [, doorway] of this.doorways) {
    try {
      const scenarioId = `${featureSlug}--${scenarioSlug}`;
      await doorway.client.beginObservation({
        scenario: scenario.pickle.name,
        scenarioId,
        tags: scenario.pickle.tags.map(t => t.name),
        feature: scenario.pickle.uri,
      });
    } catch {
      // Observation is best-effort — don't block scenarios if storage is down
    }
    break; // Only observe on the first doorway
  }
});

/**
 * Where else to look when a doorway env var is unset.
 *
 * `E2E_DOORWAY_STAGING` names the SECOND doorway in a cross-doorway scenario. It
 * is the deployed fleet's spelling for it; a local mesh spells the same peer
 * `E2E_DOORWAY_BETA` / `E2E_DOORWAY_B` (doorway-B, its own storage peer and its
 * own projection DB — a federation peer of doorway-A, not an alias of it). A run
 * that declares a real second doorway under either of those names can prove a
 * cross-doorway claim, so refusing it purely over the variable's NAME would hold
 * a provable scenario for a spelling.
 *
 * This is a name fallback only. Whether the resolved host is genuinely a SECOND
 * doorway is a separate question, answered below.
 */
const DOORWAY_ENV_FALLBACKS: Record<string, string[]> = {
  E2E_DOORWAY_STAGING: ['E2E_DOORWAY_BETA', 'E2E_DOORWAY_B'],
  E2E_DOORWAY_BETA: ['E2E_DOORWAY_B', 'E2E_DOORWAY_STAGING'],
};

function resolveDoorwayEnv(envVar: string): { url?: string; from: string } {
  const direct = process.env[envVar];
  if (direct) return { url: direct, from: envVar };
  for (const alt of DOORWAY_ENV_FALLBACKS[envVar] ?? []) {
    const url = process.env[alt];
    if (url) return { url, from: alt };
  }
  return { from: envVar };
}

/**
 * Background step: verify a doorway is healthy.
 * The URL comes from an environment variable.
 *
 * A cross-doorway feature registers two doorways here. Some harnesses point
 * several doorway names at ONE host, and a cross-doorway claim proved against a
 * single host is not proved at all — it is a tautology wearing a federation
 * costume. When the URL resolved for this doorway is already registered under a
 * DIFFERENT name, the scenario HOLDS ('pending') and names the collapse rather
 * than passing. Same guard, same reason, as `distinctPeers` in
 * `steps/federation-epr.steps.ts`; it must never be bypassed to buy a green.
 */
Given(
  'doorway {string} is healthy at env {string}',
  async function (this: E2EWorld, doorwayId: string, envVar: string) {
    const { url, from } = resolveDoorwayEnv(envVar);
    assert.ok(url, `Environment variable ${envVar} is not set`);

    for (const [otherId, other] of this.doorways) {
      if (otherId !== doorwayId && other.url === url) {
        return 'pending';
      }
    }
    if (from !== envVar) {
      this.contentIds.set(`doorway:${doorwayId}:resolvedFrom`, from);
    }

    const entry = this.addDoorway(doorwayId, url);

    await retry(
      async () => {
        const health = await entry.client.health();
        assert.ok(
          health.healthy,
          `Doorway "${doorwayId}" at ${url} is not healthy: status=${health.status}`
        );
      },
      { maxAttempts: 5, initialDelayMs: 2000, timeoutMs: 30_000 }
    );
  }
);

/**
 * Collect observation report from the first doorway (best-effort).
 * Works for both HTTP and Playwright modes.
 */
async function collectObservationReport(
  world: E2EWorld,
  scenario: { pickle: { name: string }; result?: { status: string } }
): Promise<void> {
  const safeName = scenario.pickle.name.replace(/[^a-zA-Z0-9]/g, '-');
  for (const [, doorway] of world.doorways) {
    if (!doorway.client.observationId) break;
    try {
      const report = await doorway.client.getObservationReport();
      const errorCount = report.summary.bySeverity['error'] ?? 0;
      if (errorCount > 0 || scenario.result?.status === Status.FAILED) {
        mkdirSync('reports/observations', { recursive: true });
        writeFileSync(`reports/observations/${safeName}.json`, JSON.stringify(report, null, 2));
      }
    } catch {
      // Best-effort — don't mask the real test result
    }
    break;
  }
}

/**
 * Capture per-scenario screenshots for all Playwright devices (universal, pass or fail).
 */
async function captureAllDeviceScreenshots(
  world: E2EWorld,
  featureSlug: string,
  scenarioSlug: string
): Promise<void> {
  for (const [name, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        await captureVisualEvidence(device, featureSlug, scenarioSlug, name);
      }
    }
  }
}

/**
 * Capture failure-only artifacts for all Playwright devices (trace, console errors,
 * and error sidecar JSON). Called only when the scenario status is FAILED.
 */
async function captureAllFailureArtifacts(
  world: E2EWorld,
  featureSlug: string,
  scenarioSlug: string,
  safeName: string,
  failureMessage: string
): Promise<void> {
  for (const [name, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        await captureFailureArtifactsExceptScreenshot(device, safeName, name);
        writeErrorSidecar(featureSlug, scenarioSlug, name, failureMessage);
      }
    }
  }
}

/**
 * Capture artifacts on failure, assert console cleanliness on pass, then run cleanup.
 */
After(async function (this: E2EWorld, scenario) {
  const featureSlug = this.featureSlug ?? 'unknown-feature';
  const scenarioSlug = this.scenarioSlug ?? 'unknown-scenario';

  // Universal capture: every Playwright device gets a screenshot regardless
  // of pass/fail outcome. Failures additionally get a sibling .error.json,
  // and the existing console-errors / trace artifacts (failure-only).
  await captureAllDeviceScreenshots(this, featureSlug, scenarioSlug);

  if (scenario.result?.status === Status.FAILED) {
    const safeName = scenario.pickle.name.replace(/[^a-zA-Z0-9]/g, '-');
    const failureMessage = scenario.result.message ?? 'unknown failure';
    // Write console-errors JSON and trace under reports/console and reports/traces.
    // We no longer write a FAIL- prefixed screenshot — the universal capture above
    // covered it and a sidecar .error.json carries the failure context.
    await captureAllFailureArtifacts(this, featureSlug, scenarioSlug, safeName, failureMessage);
  }

  // For passing scenarios, assert that no real console errors were logged.
  // This makes console cleanliness an automatic test contract for all browser scenarios.
  if (scenario.result?.status === Status.PASSED) {
    const errorReport = collectBrowserErrors(this, scenario.pickle.name);
    if (errorReport.length) {
      throw new Error(
        `Scenario passed but had ${errorReport.length} browser error(s):\n` +
          errorReport.map(e => `  ${e}`).join('\n')
      );
    }
  }

  await collectObservationReport(this, scenario);
  await this.runCleanup();
});

/**
 * Close the shared Playwright browser and tear down testnet after all scenarios complete.
 */
AfterAll(async function () {
  await E2EWorld.closeBrowser();

  // Reduced-scope summary: name the personas this run auto-skipped because
  // their node pool (shem) was unavailable. This is the planning signal —
  // "here is what we could NOT exercise under the available compute" — and a
  // machine-readable artifact the pipeline can fold into its run record.
  const skipped = autoSkippedHumans();
  const capSkips = substrateSkippedScenarios();
  const scope = {
    remoteComputeStatus: process.env['ELOHIM_REMOTE_COMPUTE_STATUS'] ?? 'unknown',
    reducedScope: skipped.length > 0 || capSkips.length > 0,
    autoSkippedHumans: skipped,
    // Scenarios held by the substrate-cap gate (the @requires:<cap> runtime arm), with the
    // unavailable cap(s) that held each — the machine-readable "what we could NOT exercise".
    substrateSkippedScenarios: capSkips,
  };
  try {
    writeFileSync('reports/substrate-scope.json', JSON.stringify(scope, null, 2));
  } catch {
    // reports/ may not exist in some local runs — non-fatal.
  }
  if (skipped.length > 0) {
    // eslint-disable-next-line no-console
    console.log(
      `\n  🛰️  REDUCED SCOPE this run — ${skipped.length} persona(s) auto-skipped ` +
        `(remote pool unavailable): ${skipped.join(', ')}\n` +
        '     Re-run when shem returns to exercise the full topology.\n'
    );
  }
  if (capSkips.length > 0) {
    const caps = [...new Set(capSkips.flatMap(s => s.caps))]
      .sort((a, b) => a.localeCompare(b))
      .join(', ');
    // eslint-disable-next-line no-console
    console.log(
      `\n  🛰️  REDUCED SCOPE this run — ${capSkips.length} scenario(s) held by @requires gate ` +
        `(unavailable: ${caps}).\n` +
        '     These are HELD, not failed; they return when the capability does.\n'
    );
  }

  // Testnet lifecycle cleanup — settle, report, stop
  if (isTestnetActive()) {
    const summary = getComputeSummary();
    writeComputeReport(summary);
    await stopTestnet();
  }
});
