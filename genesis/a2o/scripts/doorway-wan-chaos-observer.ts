/**
 * Read-only WAN doorway chaos observer.
 *
 * The operator causes and recovers the fault. This process stays outside the
 * failure domain and records fresh, bounded observations for one named phase.
 * Expected authority is always an input; a response can never nominate its
 * own expected truth.
 */
import { createHash } from 'node:crypto';
import { lookup } from 'node:dns/promises';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { request as httpRequest } from 'node:http';
import { request as httpsRequest } from 'node:https';
import { isIP } from 'node:net';
import { dirname, resolve } from 'node:path';

export type PhaseName = 'baseline' | 'fault' | 'recovery';
export type Verdict = 'PERMITTED' | 'REFUSED' | 'NOT MEASURED';
const NOT_MEASURED: Verdict = 'NOT MEASURED';

export interface ExpectedAuthority {
  actionHash: string;
  blobHash: string;
  version: string;
  entryScript: string;
}

export interface FixtureAuthorReceipt {
  schema: 'doorway-chaos-author-receipt/v1';
  source: 'fixture-author-operation';
  receiptId: string;
  authorId: string;
  authoredAt: string;
  contentId: string;
  authority: ExpectedAuthority;
}

export interface AuthorReceiptProvenance {
  path: string;
  sha256: string;
  receipt: FixtureAuthorReceipt;
}

export interface Leg {
  name: string;
  address: string;
}

export interface ObserverConfig {
  phase: PhaseName;
  publicName: string;
  contentId: string;
  expected: ExpectedAuthority;
  authorReceipt: AuthorReceiptProvenance;
  boundaryAt: string;
  faultOnset?: string;
  recoveryOnset?: string;
  maxEventObservationDelayMs?: number;
  timeoutMs: number;
  windowMs: number;
  cadenceMs: number;
  maxGapMs?: number;
  legs: Leg[];
  reportPath: string;
  withdrawnLeg?: string;
}

export interface RawObservation {
  status: number;
  text: string;
  headers: Record<string, string>;
  startedAt: string;
  endedAt: string;
  remoteAddress?: string;
  error?: string;
}

export interface RequestTarget {
  publicName: string;
  path: string;
  timeoutMs: number;
  address?: string;
}

export type Requester = (target: RequestTarget) => Promise<RawObservation>;
export type DnsResolver = (hostname: string) => Promise<string[]>;
export const networkDnsResolver: DnsResolver = async hostname =>
  (await lookup(hostname, { all: true })).map(answer => answer.address);

interface MarkerObservations {
  action?: RawObservation;
  content?: RawObservation;
  version?: RawObservation;
  root?: RawObservation;
  rootVersion?: RawObservation;
}

export interface EntranceObservation {
  name: string;
  attribution: {
    declaredAddress?: string;
    kernelVerifiedRemoteAddress?: string;
    remotelyReportedDoorway?: string;
  };
  requests: MarkerObservations;
  observed?: ExpectedAuthority;
  verdict: Verdict;
  reason: string;
}

export interface PhaseReceipt {
  phase: PhaseName;
  publicName: string;
  contentId: string;
  expected: ExpectedAuthority;
  authorReceipt: AuthorReceiptProvenance;
  boundaryAt: string;
  faultOnset?: string;
  recoveryOnset?: string;
  observedAt: string;
  cacheNonce: string;
  withdrawnLeg?: string;
  ordinary: EntranceObservation;
  legs: EntranceObservation[];
  samples: {
    observedAt: string;
    completedAt: string;
    dnsAddresses: string[];
    ordinary: EntranceObservation;
    legs: EntranceObservation[];
  }[];
  continuity: {
    measurementStartedAt: string;
    measurementEndedAt: string;
    observedWindowMs: number;
    boundaryToFirstSampleMs: number;
    eventToFirstSampleMs?: number;
    eventToFirstCompleteSampleMs?: number;
    declaredMaxEventObservationDelayMs?: number;
    declaredMaxGapMs?: number;
    largestSampleGapMs?: number;
    firstFailureAt?: string;
    firstRecoveryAt?: string;
    verdict: Verdict;
  };
  verdict: Verdict;
  verdictReason?: string;
}

export interface ObserverReport {
  schema: 'doorway-wan-chaos-observer/v1';
  source: 'external-observer';
  artifact: 'history-receipt-not-protocol-truth';
  publicName: string;
  contentId: string;
  phases: Partial<Record<PhaseName, PhaseReceipt>>;
  claims: {
    survivesViaRemainingDeclaredEntrance: Verdict;
    sameHostnameSelectsAfterWithdrawal: Verdict;
    dnsMembershipWithdrawal: Verdict;
    currentAuthorityDuringSplitAndRecovery: Verdict;
    continuousAcrossUnobservedPhaseGaps: Verdict;
  };
}

function headerMap(headers: NodeJS.Dict<string | string[]>): Record<string, string> {
  return Object.fromEntries(
    Object.entries(headers).map(([name, value]) => [
      name.toLowerCase(),
      Array.isArray(value) ? value.join(', ') : (value ?? ''),
    ])
  );
}

export const networkRequester: Requester = async target => {
  const startedAt = new Date().toISOString();
  const url = new URL(`https://${target.publicName}${target.path}`);
  const request = target.address?.startsWith('http://') ? httpRequest : httpsRequest;
  const direct = target.address ? new URL(target.address) : undefined;
  if (direct) {
    url.protocol = direct.protocol;
    url.port = direct.port;
  }
  return await new Promise(resolve => {
    const req = request(
      url,
      {
        headers: {
          accept: 'application/json, text/html;q=0.9, */*;q=0.8',
          'cache-control': 'no-cache, no-store, max-age=0',
          pragma: 'no-cache',
        },
        lookup: direct
          ? (_hostname, _options, callback) =>
              callback(null, direct.hostname.replace(/^\[|\]$/g, ''), isIP(direct.hostname) || 4)
          : undefined,
        servername: target.publicName,
        timeout: target.timeoutMs,
      },
      response => {
        const chunks: Buffer[] = [];
        response.on('data', chunk => chunks.push(Buffer.from(chunk as Uint8Array)));
        response.on('end', () =>
          resolve({
            status: response.statusCode ?? 0,
            text: Buffer.concat(chunks).toString('utf8'),
            headers: headerMap(response.headers),
            startedAt,
            endedAt: new Date().toISOString(),
            remoteAddress: response.socket.remoteAddress,
          })
        );
      }
    );
    req.on('timeout', () => req.destroy(new Error(`request exceeded ${target.timeoutMs}ms`)));
    req.on('error', error =>
      resolve({
        status: 0,
        text: '',
        headers: {},
        startedAt,
        endedAt: new Date().toISOString(),
        error: String(error),
      })
    );
    req.end();
  });
};

function parseJson(raw: RawObservation): Record<string, unknown> | undefined {
  try {
    return JSON.parse(raw.text) as Record<string, unknown>;
  } catch {
    return undefined;
  }
}

function versionOf(raw: RawObservation): string | undefined {
  const body = parseJson(raw);
  const value = body?.['commit'];
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

function entryScriptOf(raw: RawObservation): string | undefined {
  const scripts = [...raw.text.matchAll(/<script\b[^>]*\bsrc=["']([^"']+)["']/gi)];
  const source = scripts.at(-1)?.[1];
  if (!source) return undefined;
  try {
    return new URL(source, 'https://observer.invalid/').pathname.split('/').at(-1);
  } catch {
    return undefined;
  }
}

function firstString(
  body: Record<string, unknown> | undefined,
  names: string[]
): string | undefined {
  for (const name of names) {
    const value = body?.[name];
    if (typeof value === 'string' && value.length > 0) return value;
  }
  return undefined;
}

function addNonce(path: string, nonce: string): string {
  return `${path}${path.includes('?') ? '&' : '?'}chaos_observer=${encodeURIComponent(nonce)}`;
}

function normalizeAddress(value?: string): string | undefined {
  if (!value) return undefined;
  try {
    return new URL(value).hostname.replace(/^\[|\]$/g, '').replace(/^::ffff:/, '');
  } catch {
    return value.replace(/^\[|\]$/g, '').replace(/^::ffff:/, '');
  }
}

async function observeEntrance(
  name: string,
  config: ObserverConfig,
  nonce: string,
  requester: Requester,
  address?: string
): Promise<EntranceObservation> {
  const ask = async (path: string) =>
    await requester({
      publicName: config.publicName,
      path: addNonce(path, nonce),
      timeoutMs: config.timeoutMs,
      address,
    });
  const action = await ask(`/db/content/${encodeURIComponent(config.contentId)}/head`);
  const content = await ask(`/db/content/${encodeURIComponent(config.contentId)}`);
  const root = await ask('/');
  const rootVersion = await ask('/version.json');
  const actionHash = firstString(parseJson(action), ['headActionHash', 'head_action_hash']);
  const blobHash = firstString(parseJson(content), ['blobHash', 'blob_hash']);
  const versionRequest = blobHash
    ? await ask(`/apps/${encodeURIComponent(blobHash)}/version.json`)
    : undefined;
  const version = versionRequest ? versionOf(versionRequest) : undefined;
  const entryScript = entryScriptOf(root);
  const observed =
    actionHash && blobHash && version && entryScript
      ? { actionHash, blobHash, version, entryScript }
      : undefined;
  const requests = { action, content, version: versionRequest, root, rootVersion };
  const reported =
    action.headers['x-doorway-id'] ??
    content.headers['x-doorway-id'] ??
    root.headers['x-doorway-id'];
  const kernel = action.remoteAddress ?? content.remoteAddress ?? root.remoteAddress;
  if (!observed) {
    return {
      name,
      attribution: {
        declaredAddress: address,
        kernelVerifiedRemoteAddress: kernel,
        remotelyReportedDoorway: reported,
      },
      requests,
      verdict: NOT_MEASURED,
      reason: 'one or more exact authority markers were unavailable',
    };
  }
  const matches =
    observed.actionHash === config.expected.actionHash &&
    observed.blobHash === config.expected.blobHash &&
    observed.version === config.expected.version &&
    observed.entryScript === config.expected.entryScript;
  const shellServed =
    root.status === 200 &&
    root.text.includes('app-root') &&
    observed.entryScript === config.expected.entryScript &&
    rootVersion.status === 200 &&
    versionOf(rootVersion) === config.expected.version;
  const declaredPeer = normalizeAddress(address);
  const kernelPeer = normalizeAddress(kernel);
  const attributionMatches = !address || (kernelPeer !== undefined && kernelPeer === declaredPeer);
  return {
    name,
    attribution: {
      declaredAddress: address,
      kernelVerifiedRemoteAddress: kernel,
      remotelyReportedDoorway: reported,
    },
    requests,
    observed,
    verdict: matches && shellServed && attributionMatches ? 'PERMITTED' : 'REFUSED',
    reason: matches
      ? shellServed
        ? attributionMatches
          ? 'exact action, blob, addressed version, public-root entry script and socket peer agree'
          : `declared leg ${declaredPeer} connected to ${kernelPeer ?? 'no verified socket peer'}`
        : 'authority markers agree but the public root did not bind the expected entry script/version'
      : `expected ${JSON.stringify(config.expected)}, observed ${JSON.stringify(observed)}`,
  };
}

function computeClaims(report: ObserverReport): ObserverReport['claims'] {
  const { baseline, fault, recovery } = report.phases;
  const complete = baseline && fault && recovery;
  const faultLegs = fault?.legs ?? [];
  const survivesViaRemainingDeclaredEntrance =
    !fault || fault.continuity.verdict === NOT_MEASURED || faultLegs.length === 0
      ? NOT_MEASURED
      : fault.continuity.verdict === 'REFUSED'
        ? 'REFUSED'
        : faultLegs.some(value => value.verdict === 'PERMITTED')
          ? 'PERMITTED'
          : faultLegs.some(value => value.verdict === 'REFUSED')
            ? 'REFUSED'
            : NOT_MEASURED;
  let sameHostnameSelectsAfterWithdrawal: Verdict = NOT_MEASURED;
  if (fault?.withdrawnLeg) {
    const withdrawn = fault.legs.find(leg => leg.name === fault.withdrawnLeg);
    const survivor = fault.legs.find(
      leg => leg.name !== fault.withdrawnLeg && leg.verdict === 'PERMITTED'
    );
    const baselineWithdrawn = baseline?.legs.find(leg => leg.name === fault.withdrawnLeg);
    const withdrawnAddress = normalizeAddress(
      baselineWithdrawn?.attribution.kernelVerifiedRemoteAddress
    );
    const survivorAddress = normalizeAddress(survivor?.attribution.kernelVerifiedRemoteAddress);
    const survivorName = survivor?.name;
    const baselineUsedWithdrawn =
      withdrawnAddress !== undefined &&
      (baseline?.samples.every(
        sample =>
          sample.ordinary.verdict === 'PERMITTED' &&
          normalizeAddress(sample.ordinary.attribution.kernelVerifiedRemoteAddress) ===
            withdrawnAddress
      ) ??
        false);
    const faultUsedSurvivor =
      survivorAddress !== undefined &&
      survivorName !== undefined &&
      fault.samples.every(sample => {
        const pinnedSurvivor = sample.legs.find(leg => leg.name === survivorName);
        const pinnedWithdrawn = sample.legs.find(leg => leg.name === fault.withdrawnLeg);
        return (
          sample.ordinary.verdict === 'PERMITTED' &&
          normalizeAddress(sample.ordinary.attribution.kernelVerifiedRemoteAddress) ===
            survivorAddress &&
          pinnedSurvivor?.verdict === 'PERMITTED' &&
          normalizeAddress(pinnedSurvivor.attribution.kernelVerifiedRemoteAddress) ===
            survivorAddress &&
          pinnedWithdrawn?.requests.root?.status !== 200
        );
      });
    if (withdrawn && survivor && withdrawnAddress !== survivorAddress) {
      sameHostnameSelectsAfterWithdrawal =
        baselineUsedWithdrawn && faultUsedSurvivor && fault.continuity.verdict === 'PERMITTED'
          ? 'PERMITTED'
          : 'REFUSED';
    }
  }
  let currentAuthorityDuringSplitAndRecovery: Verdict = NOT_MEASURED;
  if (complete) {
    const authorityMoved =
      baseline.expected.actionHash !== fault.expected.actionHash &&
      baseline.expected.blobHash !== fault.expected.blobHash;
    const baselineObserved =
      baseline.verdict === 'PERMITTED' &&
      baseline.continuity.verdict === 'PERMITTED' &&
      baseline.ordinary.verdict === 'PERMITTED' &&
      baseline.legs.every(leg => leg.verdict === 'PERMITTED');
    const faultObserved =
      fault.verdict === 'PERMITTED' &&
      fault.continuity.verdict === 'PERMITTED' &&
      fault.ordinary.verdict === 'PERMITTED' &&
      fault.legs.some(leg => leg.verdict === 'PERMITTED');
    const recovered =
      recovery.verdict === 'PERMITTED' &&
      recovery.continuity.verdict === 'PERMITTED' &&
      recovery.ordinary.verdict === 'PERMITTED' &&
      recovery.legs.length > 0 &&
      recovery.legs.every(leg => leg.verdict === 'PERMITTED');
    currentAuthorityDuringSplitAndRecovery =
      baselineObserved &&
      authorityMoved &&
      faultObserved &&
      recovered &&
      fault.expected.actionHash === recovery.expected.actionHash &&
      fault.expected.blobHash === recovery.expected.blobHash &&
      fault.expected.version === recovery.expected.version &&
      fault.expected.entryScript === recovery.expected.entryScript &&
      baseline.authorReceipt.sha256 !== fault.authorReceipt.sha256 &&
      fault.authorReceipt.sha256 === recovery.authorReceipt.sha256
        ? 'PERMITTED'
        : 'REFUSED';
  }
  return {
    survivesViaRemainingDeclaredEntrance,
    sameHostnameSelectsAfterWithdrawal,
    dnsMembershipWithdrawal: NOT_MEASURED,
    currentAuthorityDuringSplitAndRecovery,
    continuousAcrossUnobservedPhaseGaps: NOT_MEASURED,
  };
}

export async function observePhase(
  config: ObserverConfig,
  requester: Requester = networkRequester,
  resolver: DnsResolver = networkDnsResolver
): Promise<ObserverReport> {
  if (!Number.isFinite(Date.parse(config.boundaryAt))) {
    throw new Error(`--boundary-at must be an ISO timestamp, got ${config.boundaryAt}`);
  }
  if (config.faultOnset && !Number.isFinite(Date.parse(config.faultOnset))) {
    throw new Error(`--fault-onset must be an ISO timestamp, got ${config.faultOnset}`);
  }
  if (config.recoveryOnset && !Number.isFinite(Date.parse(config.recoveryOnset))) {
    throw new Error(`--recovery-onset must be an ISO timestamp, got ${config.recoveryOnset}`);
  }
  const authorReceipt = config.authorReceipt.receipt;
  if (
    authorReceipt.schema !== 'doorway-chaos-author-receipt/v1' ||
    authorReceipt.source !== 'fixture-author-operation' ||
    !authorReceipt.receiptId ||
    !authorReceipt.authorId ||
    !Number.isFinite(Date.parse(authorReceipt.authoredAt)) ||
    authorReceipt.contentId !== config.contentId ||
    JSON.stringify(authorReceipt.authority) !== JSON.stringify(config.expected)
  ) {
    throw new Error('expected authority must come from a valid independent fixture-author receipt');
  }
  if (
    !Number.isFinite(config.windowMs) ||
    !Number.isFinite(config.cadenceMs) ||
    config.windowMs < config.cadenceMs * 2 ||
    config.cadenceMs <= 0
  ) {
    throw new Error('--window-ms must cover at least two positive --cadence-ms intervals');
  }
  if (
    config.maxGapMs !== undefined &&
    (!Number.isFinite(config.maxGapMs) || config.maxGapMs < config.cadenceMs)
  ) {
    throw new Error('--max-gap-ms must be at least --cadence-ms');
  }
  if (
    config.maxEventObservationDelayMs !== undefined &&
    (!Number.isFinite(config.maxEventObservationDelayMs) || config.maxEventObservationDelayMs < 0)
  ) {
    throw new Error('--max-event-observation-delay-ms must be finite and non-negative');
  }
  const nonce = createHash('sha256')
    .update(`${config.phase}:${config.boundaryAt}:${Date.now()}`)
    .digest('hex')
    .slice(0, 16);
  const sampleCount = Math.max(3, Math.ceil(config.windowMs / config.cadenceMs) + 1);
  const samples: PhaseReceipt['samples'] = [];
  const measurementStartedMs = Date.now();
  for (let sample = 0; sample < sampleCount; sample += 1) {
    if (sample > 0) {
      const scheduledAt = measurementStartedMs + sample * config.cadenceMs;
      const remaining = scheduledAt - Date.now();
      if (remaining > 0) await new Promise(resolve => setTimeout(resolve, remaining));
    }
    const observedAt = new Date().toISOString();
    const ordinary = await observeEntrance('ordinary-dns', config, `${nonce}-${sample}`, requester);
    const dnsAddresses = await resolver(config.publicName).catch(() => []);
    const legs: EntranceObservation[] = [];
    for (const leg of config.legs) {
      legs.push(
        await observeEntrance(leg.name, config, `${nonce}-${sample}`, requester, leg.address)
      );
    }
    samples.push({
      observedAt,
      completedAt: new Date().toISOString(),
      dnsAddresses,
      ordinary,
      legs,
    });
  }
  const measurementEndedAt = new Date().toISOString();
  const last = samples.at(-1) as (typeof samples)[number];
  const ordinary = last.ordinary;
  const legs = last.legs;
  const requiredSampleFailed = (sample: (typeof samples)[number]): boolean => {
    const requiredLegs =
      config.phase === 'fault'
        ? sample.legs.filter(leg => leg.name !== config.withdrawnLeg)
        : sample.legs;
    return (
      sample.ordinary.verdict !== 'PERMITTED' ||
      requiredLegs.length === 0 ||
      requiredLegs.some(leg => leg.verdict !== 'PERMITTED')
    );
  };
  const failure = samples.find(requiredSampleFailed);
  const failureIndex = failure ? samples.indexOf(failure) : -1;
  const recovery =
    failureIndex >= 0
      ? samples.slice(failureIndex + 1).find(sample => !requiredSampleFailed(sample))
      : undefined;
  const gaps = samples
    .slice(1)
    .map((sample, index) => Date.parse(sample.observedAt) - Date.parse(samples[index].observedAt));
  const largestSampleGapMs = gaps.length > 0 ? Math.max(...gaps) : undefined;
  const boundaryToFirstSampleMs = Date.parse(samples[0].observedAt) - Date.parse(config.boundaryAt);
  const phaseEventAt =
    config.phase === 'fault'
      ? config.faultOnset
      : config.phase === 'recovery'
        ? config.recoveryOnset
        : undefined;
  const eventToFirstSampleMs = phaseEventAt
    ? Date.parse(samples[0].observedAt) - Date.parse(phaseEventAt)
    : undefined;
  const eventToFirstCompleteSampleMs = phaseEventAt
    ? Date.parse(samples[0].completedAt) - Date.parse(phaseEventAt)
    : undefined;
  const observedWindowMs =
    Date.parse(samples.at(-1)?.observedAt ?? measurementEndedAt) -
    Date.parse(samples[0].observedAt);
  const continuityVerdict: Verdict =
    config.maxGapMs === undefined || samples.length < 3
      ? NOT_MEASURED
      : failure ||
          boundaryToFirstSampleMs < 0 ||
          boundaryToFirstSampleMs > config.maxGapMs ||
          observedWindowMs < config.windowMs ||
          (largestSampleGapMs ?? Number.POSITIVE_INFINITY) > config.maxGapMs
        ? 'REFUSED'
        : 'PERMITTED';
  const prior = existsSync(config.reportPath)
    ? (JSON.parse(readFileSync(config.reportPath, 'utf8')) as ObserverReport)
    : undefined;
  const phase: PhaseReceipt = {
    phase: config.phase,
    publicName: config.publicName,
    contentId: config.contentId,
    expected: config.expected,
    authorReceipt: config.authorReceipt,
    boundaryAt: config.boundaryAt,
    faultOnset: config.phase === 'recovery' ? prior?.phases.fault?.faultOnset : config.faultOnset,
    recoveryOnset: config.recoveryOnset,
    observedAt: new Date().toISOString(),
    cacheNonce: nonce,
    withdrawnLeg: config.withdrawnLeg,
    ordinary,
    legs,
    samples,
    continuity: {
      measurementStartedAt: samples[0].observedAt,
      measurementEndedAt,
      observedWindowMs,
      boundaryToFirstSampleMs,
      eventToFirstSampleMs,
      eventToFirstCompleteSampleMs,
      declaredMaxEventObservationDelayMs: config.maxEventObservationDelayMs,
      declaredMaxGapMs: config.maxGapMs,
      largestSampleGapMs,
      firstFailureAt: failure?.observedAt,
      firstRecoveryAt: recovery?.observedAt,
      verdict: continuityVerdict,
    },
    verdict:
      ordinary.verdict === 'REFUSED' || legs.some(leg => leg.verdict === 'REFUSED')
        ? 'REFUSED'
        : ordinary.verdict === 'PERMITTED' && legs.some(leg => leg.verdict === 'PERMITTED')
          ? 'PERMITTED'
          : NOT_MEASURED,
  };
  if (prior && (prior.publicName !== config.publicName || prior.contentId !== config.contentId)) {
    throw new Error('report belongs to a different public name or content id');
  }
  const prerequisite =
    config.phase === 'baseline'
      ? undefined
      : config.phase === 'fault'
        ? prior?.phases.baseline
        : prior?.phases.fault;
  const withdrawn = legs.find(leg => leg.name === config.withdrawnLeg);
  const survivors = legs.filter(leg => leg.name !== config.withdrawnLeg);
  const everyEntrancePermitted =
    ordinary.verdict === 'PERMITTED' &&
    legs.length > 0 &&
    legs.every(leg => leg.verdict === 'PERMITTED');
  if (continuityVerdict === NOT_MEASURED) {
    phase.verdict = NOT_MEASURED;
    phase.verdictReason = 'continuity requires at least three samples and --max-gap-ms';
  } else if (continuityVerdict === 'REFUSED') {
    phase.verdict = 'REFUSED';
    phase.verdictReason =
      'the measured phase violated its boundary, window, cadence, or required-sample contract';
  } else if (config.phase === 'baseline' || config.phase === 'recovery') {
    phase.verdict = everyEntrancePermitted ? 'PERMITTED' : 'REFUSED';
  } else if (config.phase === 'fault') {
    if (config.withdrawnLeg) {
      const faultRouteHeld = samples.every(sample => {
        const sampleWithdrawn = sample.legs.find(leg => leg.name === config.withdrawnLeg);
        const sampleSurvivors = sample.legs.filter(leg => leg.name !== config.withdrawnLeg);
        return (
          sample.ordinary.verdict === 'PERMITTED' &&
          sampleWithdrawn?.requests.root?.status !== 200 &&
          sampleSurvivors.length > 0 &&
          sampleSurvivors.every(leg => leg.verdict === 'PERMITTED')
        );
      });
      phase.verdict =
        faultRouteHeld && ordinary.verdict === 'PERMITTED' && withdrawn && survivors.length > 0
          ? 'PERMITTED'
          : 'REFUSED';
    } else {
      phase.verdict = NOT_MEASURED;
    }
  }
  if (config.phase !== 'baseline' && !prerequisite) {
    phase.verdict = NOT_MEASURED;
    phase.verdictReason = `${config.phase} has no preceding phase receipt`;
  } else if (prerequisite && Date.parse(config.boundaryAt) <= Date.parse(prerequisite.observedAt)) {
    phase.verdict = 'REFUSED';
    phase.verdictReason = `${config.phase} boundary is not later than the ${prerequisite.phase} observation`;
  } else if (
    config.phase === 'fault' &&
    prerequisite?.authorReceipt.sha256 === config.authorReceipt.sha256
  ) {
    phase.verdict = 'REFUSED';
    phase.verdictReason =
      'fault phase reused authority A author receipt instead of an independent B';
  } else if (
    config.phase === 'recovery' &&
    prerequisite?.authorReceipt.sha256 !== config.authorReceipt.sha256
  ) {
    phase.verdict = 'REFUSED';
    phase.verdictReason = 'recovery phase did not reuse the exact fault-phase authority B receipt';
  } else if (config.phase === 'fault') {
    const faultOnsetMs = Date.parse(config.faultOnset ?? '');
    const authoredAtMs = Date.parse(config.authorReceipt.receipt.authoredAt);
    const firstSampleMs = Date.parse(samples[0].observedAt);
    if (!config.faultOnset || config.maxEventObservationDelayMs === undefined) {
      phase.verdict = NOT_MEASURED;
      phase.verdictReason =
        'fault timing requires --fault-onset and --max-event-observation-delay-ms';
    } else if (
      faultOnsetMs <= Date.parse(prerequisite?.observedAt ?? '') ||
      authoredAtMs <= faultOnsetMs ||
      authoredAtMs > firstSampleMs ||
      Date.parse(samples[0].completedAt) - faultOnsetMs > config.maxEventObservationDelayMs
    ) {
      phase.verdict = 'REFUSED';
      phase.verdictReason =
        'fault timing must follow completed baseline, precede B authorship, and reach observation within its declared delay';
    }
  } else if (config.phase === 'recovery') {
    const recoveryOnsetMs = Date.parse(config.recoveryOnset ?? '');
    const firstSampleMs = Date.parse(samples[0].observedAt);
    if (!config.recoveryOnset || config.maxEventObservationDelayMs === undefined) {
      phase.verdict = NOT_MEASURED;
      phase.verdictReason =
        'recovery timing requires --recovery-onset and --max-event-observation-delay-ms';
    } else if (
      recoveryOnsetMs <= Date.parse(prerequisite?.observedAt ?? '') ||
      recoveryOnsetMs <= Date.parse(config.authorReceipt.receipt.authoredAt) ||
      firstSampleMs < recoveryOnsetMs ||
      Date.parse(samples[0].completedAt) - recoveryOnsetMs > config.maxEventObservationDelayMs
    ) {
      phase.verdict = 'REFUSED';
      phase.verdictReason =
        'recovery timing must follow fault observation and B authorship and reach observation within its declared delay';
    }
  }
  const report: ObserverReport = prior ?? {
    schema: 'doorway-wan-chaos-observer/v1',
    source: 'external-observer',
    artifact: 'history-receipt-not-protocol-truth',
    publicName: config.publicName,
    contentId: config.contentId,
    phases: {},
    claims: {
      survivesViaRemainingDeclaredEntrance: NOT_MEASURED,
      sameHostnameSelectsAfterWithdrawal: NOT_MEASURED,
      dnsMembershipWithdrawal: NOT_MEASURED,
      currentAuthorityDuringSplitAndRecovery: NOT_MEASURED,
      continuousAcrossUnobservedPhaseGaps: NOT_MEASURED,
    },
  };
  report.phases[config.phase] = phase;
  report.claims = computeClaims(report);
  mkdirSync(dirname(config.reportPath), { recursive: true });
  writeFileSync(config.reportPath, `${JSON.stringify(report, null, 2)}\n`);
  return report;
}

function option(argv: string[], name: string, required = true): string | undefined {
  const at = argv.indexOf(name);
  const value = at >= 0 ? argv[at + 1] : undefined;
  if (required && !value) throw new Error(`missing ${name}`);
  return value;
}

function parseLeg(value: string): Leg {
  const at = value.indexOf('=');
  if (at < 1 || at === value.length - 1)
    throw new Error(`invalid --leg ${value}; use name=url-or-ip`);
  const leg = { name: value.slice(0, at), address: value.slice(at + 1) };
  const hostname = new URL(leg.address).hostname.replace(/^\[|\]$/g, '');
  if (isIP(hostname) === 0) {
    throw new Error(`invalid --leg ${value}; the WAN observer requires a literal public IP`);
  }
  return leg;
}

export function loadAuthorReceipt(path: string): AuthorReceiptProvenance {
  const resolvedPath = resolve(path);
  const raw = readFileSync(resolvedPath, 'utf8');
  const receipt = JSON.parse(raw) as FixtureAuthorReceipt;
  if (
    receipt.schema !== 'doorway-chaos-author-receipt/v1' ||
    receipt.source !== 'fixture-author-operation'
  ) {
    throw new Error(`${path} is not a fixture-author operation receipt`);
  }
  return {
    path: resolvedPath,
    sha256: createHash('sha256').update(raw).digest('hex'),
    receipt,
  };
}

export function parseArgs(argv: string[]): ObserverConfig {
  const phase = option(argv, '--phase') as PhaseName;
  if (!['baseline', 'fault', 'recovery'].includes(phase))
    throw new Error(`invalid --phase ${phase}`);
  const legs = argv.flatMap((value, index) =>
    value === '--leg' ? [parseLeg(argv[index + 1])] : []
  );
  const authorReceipt = loadAuthorReceipt(option(argv, '--author-receipt') as string);
  return {
    phase,
    publicName: option(argv, '--public-name') as string,
    contentId: option(argv, '--content-id') as string,
    expected: authorReceipt.receipt.authority,
    authorReceipt,
    boundaryAt: option(argv, '--boundary-at') as string,
    faultOnset: option(argv, '--fault-onset', false),
    recoveryOnset: option(argv, '--recovery-onset', false),
    maxEventObservationDelayMs: option(argv, '--max-event-observation-delay-ms', false)
      ? Number(option(argv, '--max-event-observation-delay-ms', false))
      : undefined,
    timeoutMs: Number(option(argv, '--timeout-ms', false) ?? '15000'),
    windowMs: Number(option(argv, '--window-ms', false) ?? '0'),
    cadenceMs: Number(option(argv, '--cadence-ms', false) ?? '5000'),
    maxGapMs: option(argv, '--max-gap-ms', false)
      ? Number(option(argv, '--max-gap-ms', false))
      : undefined,
    legs,
    reportPath:
      option(argv, '--report', false) ??
      `reports/doorway-wan-chaos-${new Date().toISOString().slice(0, 10)}.json`,
    withdrawnLeg: option(argv, '--withdrawn-leg', false),
  };
}

async function main(): Promise<void> {
  const config = parseArgs(process.argv.slice(2));
  const report = await observePhase(config);
  console.log(JSON.stringify({ phase: config.phase, claims: report.claims }, null, 2));
  if (report.phases[config.phase]?.verdict === 'REFUSED') process.exitCode = 1;
  else if (report.phases[config.phase]?.verdict === NOT_MEASURED) process.exitCode = 2;
}

if (process.argv[1]?.endsWith('doorway-wan-chaos-observer.ts')) {
  void main().catch(error => {
    console.error(String(error));
    process.exitCode = 2;
  });
}
