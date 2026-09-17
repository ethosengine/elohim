import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

import { requireFixtureDoorwayUrl, type HouseholdMeshFixture } from '../fixtures/household-mesh.js';

const MANIFESTO_MARKERS = ['Executive Summary', 'Love as Technology'] as const;

export function publicDoorwayUrl(origin: string, publicHostname: string): string {
  const url = new URL(origin);
  url.hostname = publicHostname;
  return url.origin;
}

/** The immutable socket legs use fixture topology keys, not story-facing actor labels. */
export function publicDoorwayPorts(fixture: HouseholdMeshFixture): string[] {
  return ['alpha', 'apex'].map(id => new URL(requireFixtureDoorwayUrl(fixture, id)).port);
}

/**
 * True when `hostname` IS `elohim.host` or a genuine subdomain of it. Matches
 * on the parsed `URL.hostname` with exact/`.`-suffix comparison only — never
 * a string prefix/substring check, which would also accept an attacker host
 * like `evil-elohim.host` (no dot before the label) as if it were owned.
 */
function isElohimHostFamily(hostname: string): boolean {
  return hostname === 'elohim.host' || hostname.endsWith('.elohim.host');
}

export function requiredRequestRoutingFailure(input: {
  requestUrl: string;
  selectedOrigin: string;
  publicHostname: string;
  ownedPorts: readonly string[];
}): string | undefined {
  const request = new URL(input.requestUrl);
  const selected = new URL(input.selectedOrigin);
  const targetsOwnedDoorway =
    request.hostname === input.publicHostname ||
    ((request.hostname === 'localhost' || request.hostname === '127.0.0.1') &&
      input.ownedPorts.includes(request.port));
  const targetsElohimHost = isElohimHostFamily(request.hostname);
  // Stricter substrate-path rule, unconditional on host: a request for a
  // notarized/federation route always needs the selected origin.
  const requiresSelectedOriginByPath =
    request.pathname.startsWith('/db/') ||
    request.pathname.startsWith('/epr-head/') ||
    (request.pathname.startsWith('/api/') && targetsElohimHost);
  // ANY resource on elohim.host or a subdomain — root document, static
  // asset, any other route — that isn't already recognized as an owned
  // doorway is a routing failure regardless of path. Genuinely third-party
  // hosts (YouTube, a CDN, …) are unaffected and stay governed by the
  // existing per-owner optional-negative baseline.
  const requiresSelectedOriginByHost = targetsElohimHost && !targetsOwnedDoorway;
  if (!targetsOwnedDoorway && !requiresSelectedOriginByPath && !requiresSelectedOriginByHost)
    return undefined;
  return request.origin === selected.origin
    ? undefined
    : `${request.origin} bypassed selected doorway ${selected.origin}`;
}

/** Maps a websocket URL's scheme to its http(s) counterpart, host/path/query
 * untouched — so a `ws(s)://` origin can be compared against the same
 * owned-origin rule an ordinary `http(s)://` request is checked against. */
export function httpOriginForWebSocketUrl(wsUrl: string): string {
  const url = new URL(wsUrl);
  if (url.protocol === 'ws:') url.protocol = 'http:';
  else if (url.protocol === 'wss:') url.protocol = 'https:';
  return url.toString();
}

export function selectCanonicalManifestoCandidate(candidates: readonly string[]): number {
  const matches = candidates
    .map((text, index) => ({ index, text }))
    .filter(candidate => MANIFESTO_MARKERS.every(marker => candidate.text.includes(marker)));
  if (matches.length !== 1)
    throw new Error(`expected one canonical manifesto renderer, observed ${matches.length}`);
  return matches[0].index;
}

export async function awaitCanonicalManifestoCandidate(
  readCandidates: () => Promise<readonly string[]>,
  deadlineAt: number,
  sleep: (milliseconds: number) => Promise<void> = async milliseconds => {
    await new Promise(resolve => setTimeout(resolve, milliseconds));
  },
  now: () => number = Date.now
): Promise<{ index: number; text: string }> {
  let stableSignature: string | undefined;
  let lastError: unknown;
  while (now() < deadlineAt) {
    const candidates = await readCandidates();
    try {
      const index = selectCanonicalManifestoCandidate(candidates);
      const text = candidates[index];
      if (text.length <= 1000) throw new Error('canonical manifesto content is not substantive');
      const signature = JSON.stringify(candidates);
      if (signature === stableSignature) return { index, text };
      stableSignature = signature;
    } catch (error) {
      lastError = error;
      stableSignature = undefined;
    }
    await sleep(Math.min(250, Math.max(1, deadlineAt - now())));
  }
  throw lastError ?? new Error('canonical manifesto renderer did not settle before deadline');
}

function safeSegment(value: string): string {
  return value.replaceAll(/[^a-zA-Z0-9._-]/g, '-');
}

export function persistRealAppPhaseArtifacts(input: {
  runId: string;
  phase: string;
  screenshot: Buffer;
  receipt: unknown;
  reportsRoot?: string;
}): { screenshotPath: string; receiptPath: string } {
  const relativeDir = join('doorway-apex-transition', safeSegment(input.runId));
  const outputDir = resolve(input.reportsRoot ?? 'reports', relativeDir);
  mkdirSync(outputDir, { recursive: true });
  const basename = safeSegment(input.phase);
  const screenshotPath = join(outputDir, `${basename}.png`);
  const receiptPath = join(outputDir, `${basename}.json`);
  const artifactPaths = { screenshotPath, receiptPath };
  const persistedReceipt =
    typeof input.receipt === 'object' && input.receipt !== null
      ? { ...input.receipt, artifactPaths }
      : { receipt: input.receipt, artifactPaths };
  writeFileSync(screenshotPath, input.screenshot);
  writeFileSync(receiptPath, `${JSON.stringify(persistedReceipt, null, 2)}\n`);
  return artifactPaths;
}

export function isOptionalNavigationCancellation(input: {
  path: string;
  errorText?: string;
}): boolean {
  return input.path === '/api/v1/federation/doorways' && input.errorText === 'net::ERR_ABORTED';
}

export interface FirstPartyHttpError {
  path: string;
  status: number;
}

/**
 * Negative reads intentionally issued by the real Lamad journey. The EPR-head
 * reads are optional relationship discovery, the three documents are optional
 * related-content cards, and the love map is private to its owner. Keep this
 * exact: an arbitrary 404/403 is still a major-regression signal.
 */
const EXPECTED_NEGATIVE_HTTP = new Set([
  '404 /epr-head/value-scanner-epic',
  '404 /epr-head/autonomous-entity-epic',
  '404 /epr-head/governance-epic',
  '404 /epr-head/social-medium-epic',
  '404 /epr-head/economic-coordination-epic',
  '404 /epr-head/public-observer-epic',
  '404 /epr-head/quiz-who-are-you',
  '404 /epr-head/concept-path-forward-policymakers',
  '404 /epr-head/concept-path-forward-developers',
  '404 /epr-head/concept-path-forward-communities',
  '404 /db/content/constitution',
  '404 /db/content/theology',
  '404 /db/content/confession',
  '403 /db/content/love-map-matthew-jessica',
]);

export function classifyFirstPartyHttpError(
  input: FirstPartyHttpError
): 'expected-optional-negative' | 'required-failure' {
  return EXPECTED_NEGATIVE_HTTP.has(`${input.status} ${input.path}`)
    ? 'expected-optional-negative'
    : 'required-failure';
}

export function unexpectedOptionalNegatives(
  baseline: ReadonlySet<string>,
  observed: ReadonlySet<string>
): string[] {
  return [...observed]
    .filter(item => !baseline.has(item))
    .sort((left, right) => left.localeCompare(right));
}

/**
 * Origins the landing page KNOWINGLY loads content from beyond the household's
 * own doorways. Declared, not discovered: a request failing here is a known
 * third-party's own flakiness (telemetry beacons, ad-blocked pixels, …), never
 * evidence of a household routing/failover defect. An UNDECLARED third-party
 * origin still fails the scenario exactly as before — this list only narrows
 * the blast radius to origins someone named on purpose.
 *
 * Typed external references (content-addressed, contract-checked) are the
 * principled successor to this hand-maintained list — see
 * genesis/data/timeline/backlog/legacy-web-content-projected-inward.md.
 */
export interface DeclaredExternalEmbedOrigin {
  /** Exact scheme+host, e.g. `https://www.youtube.com` — never a bare hostname. */
  origin: string;
  /** Short reason naming the declaring component. */
  reason: string;
}

export const DECLARED_EXTERNAL_EMBED_ORIGINS: readonly DeclaredExternalEmbedOrigin[] = [
  {
    origin: 'https://www.youtube.com',
    reason:
      "app/elohim-app hero.component.html: the landing page's summary and deep-dive video " +
      'iframes embed https://www.youtube.com/embed/<id>, so the player and its own telemetry ' +
      '(api/stats/atr, youtubei/v1/log_event) load from this origin on every landing visit.',
  },
];

/**
 * True when `requestUrl` targets a DECLARED external embed origin — matched on
 * the parsed URL's scheme + hostname, exact or a genuine `.`-suffix subdomain
 * of the declared hostname, never a string prefix/substring (which would also
 * accept an attacker host like `www.youtube.com.evil.example` or
 * `evil-youtube.com`). An unparsable `requestUrl` matches nothing.
 */
export function declaredExternalEmbedOriginFor(
  requestUrl: string
): DeclaredExternalEmbedOrigin | undefined {
  let request: URL;
  try {
    request = new URL(requestUrl);
  } catch {
    return undefined;
  }
  return DECLARED_EXTERNAL_EMBED_ORIGINS.find(declared => {
    const declaredUrl = new URL(declared.origin);
    return (
      request.protocol === declaredUrl.protocol &&
      (request.hostname === declaredUrl.hostname ||
        request.hostname.endsWith(`.${declaredUrl.hostname}`))
    );
  });
}

export interface PartitionedByDeclaredOrigin<T> {
  /** Failures to a DECLARED external embed origin — recorded, never fatal. */
  declared: T[];
  /** Everything else — household-owned origins, `*.elohim.host` escapes, and
   *  any UNDECLARED third-party origin — unchanged, still fatal. */
  undeclared: T[];
}

/**
 * Partition a list of URL-bearing entries (request failures, HTTP error
 * responses, …) by whether their `url` field targets a declared external
 * embed origin. Generic over the entry shape so both the "network failure"
 * `{ url, failure }` records and the "HTTP error response" `{ url, status }`
 * records use the same partition.
 */
export function partitionByDeclaredExternalEmbedOrigin<T extends { url: string }>(
  entries: readonly T[]
): PartitionedByDeclaredOrigin<T> {
  const declared: T[] = [];
  const undeclared: T[] = [];
  for (const entry of entries) {
    if (declaredExternalEmbedOriginFor(entry.url)) declared.push(entry);
    else undeclared.push(entry);
  }
  return { declared, undeclared };
}
