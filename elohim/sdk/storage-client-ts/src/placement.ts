/**
 * The placement status surface: the whole app-author runtime surface for placement (interfaces
 * spec §H.2).
 *
 * An app declares needs in its manifest. At runtime it can ask what state each need is in, why,
 * and watch it change. No function here takes a device, host, hub or URL, so an app cannot place
 * anything: placement is decided by the space (the operator's ruling of 2026-10-20: the app
 * declares closure, loss and signing as needs and the space resolves them).
 *
 * Two planes answer:
 * - **network** needs (`loss`, `compute`, `projectable`, `indexed`) are answered by the storage
 *   route, read through the `StorageClient`;
 * - **device** needs (`closure`, whose residency is a `PrivateNeed` that never leaves the device,
 *   R6) are answered by the device's own tier, in the browser the service worker, through the
 *   `DevicePlacement` port. The remote client never reports one, and a device answer carries no
 *   device facts (H12).
 *
 * `unreachable`, the unmeasured state, never reads as met (spec §9): a need the route did not
 * answer, or answered in a form this surface cannot read, is `unreachable` with its reason.
 */
import { isUnanswered, type Unanswered } from '@elohim/eprfs';

import type { AnswerReason } from './generated/placement/answer-reason.js';
import type {
  NotCarriedReason,
  PlacementExplanationView,
} from './generated/placement/placement-explanation-view.js';
import type { StorageClient } from './client.js';
import { StorageError } from './types.js';

export type { AnswerReason, NotCarriedReason, PlacementExplanationView };

/** Typed: the manifest field plus its sub-key, mapped to a NeedKey (H12). */
export type NeedRef =
  | { field: 'closure' }
  | { field: 'loss'; contentType: string }
  | { field: 'compute'; scope: string }
  | { field: 'projectable' }
  | { field: 'indexed'; contentType: string };

export type NeedStatus =
  | { need: NeedRef; state: 'present' }
  | { need: NeedRef; state: 'absent'; unanswered: Unanswered<NotCarriedReason> }
  | { need: NeedRef; state: 'unreachable'; reason: AnswerReason }; // never reads as met (I11)

export type PendingStatus<T, R> =
  | { status: 'queued' }
  | { status: 'retrying-at'; epochSecs: number }
  | { status: 'unanswered'; unanswered: Unanswered<R> }
  | { status: 'answered'; value: T } // terminal
  | { status: 'withdrawn' }; // terminal

/**
 * Network needs explain through the storage route; device-private needs (residency) are answered
 * by the device's own tier (in the browser, the worker) and carry no device facts (H12).
 */
export type PlacementExplanation =
  | { plane: 'network'; view: PlacementExplanationView }
  | { plane: 'device'; kept: boolean; deferred: boolean; unanswered: Unanswered<string> | null };

export interface Placement {
  status(): Promise<readonly NeedStatus[]>;
  /** R7's channel: reports until answered or withdrawn; a quiet need yields nothing. */
  watch(need: NeedRef): AsyncIterable<PendingStatus<NeedStatus, NotCarriedReason>>;
  why(need: NeedRef): Promise<PlacementExplanation>;
}

/**
 * The worker's port. `status` answers a device-plane need. `declared` lists the needs the
 * installed app's manifest declares, as refs (the manifest's needs are placement-neutral, so this
 * names no device fact); `status()` enumerates from it, so a network need the route fails to answer
 * still appears, as `unreachable`, rather than disappearing (revision of §H.2, which gives the port
 * `status` only).
 */
export interface DevicePlacement {
  status(need: NeedRef): Promise<NeedStatus>;
  declared(): Promise<readonly NeedRef[]>;
}

/** What `placementFor` reads from a `StorageClient`: its JSON GET. */
export type PlacementRoutes = Pick<StorageClient, 'getJson'>;

export interface PlacementOptions {
  /** How long `watch` waits between reads when nothing gave a come-back time. Default 30 s. */
  pollMs?: number;
  /**
   * The least `watch` waits before reading again when an answer gave a come-back time. Default
   * 1 s. A come-back time already past (a stale answer, or a clock behind the server's) waits
   * `pollMs` instead, so a route that keeps naming the same past time is not read in a loop.
   */
  minWaitMs?: number;
  /** The clock and the wait `watch` uses; injected by tests. */
  now?: () => number;
  sleep?: (ms: number) => Promise<void>;
}

/** The planes, by field. */
export function isDevicePlane(need: NeedRef): boolean {
  return need.field === 'closure';
}

/** One string per need, stable across encodings: the field and its sub-key. */
export function needId(need: NeedRef): string {
  switch (need.field) {
    case 'loss':
    case 'indexed':
      return `${need.field}:${need.contentType}`;
    case 'compute':
      return `compute:${need.scope}`;
    default:
      return need.field;
  }
}

function needQuery(need: NeedRef): string {
  const params = new URLSearchParams({ field: need.field });
  if (need.field === 'loss' || need.field === 'indexed') params.set('contentType', need.contentType);
  if (need.field === 'compute') params.set('scope', need.scope);
  return params.toString();
}

/** The storage routes this surface reads (§H.2 names "the storage route"; its path is a revision). */
export function placementRoutes(appId: string): { status: string; why: (need: NeedRef) => string } {
  const base = `/api/v1/apps/${encodeURIComponent(appId)}/placement`;
  return { status: base, why: (need) => `${base}/why?${needQuery(need)}` };
}

/*
 * Membership tables keyed by the generated unions: adding, renaming or removing a value in
 * `answer-reason.schema.json` or in `NotCarriedReason`'s variants fails this file's typecheck
 * until the table matches, so neither list can drift from its source of record silently. (A
 * generated constant list would remove the table; the codegen has none for these yet.)
 */
const ANSWER_REASONS: Record<AnswerReason, true> = {
  observed_absent: true,
  timeout: true,
  transport_error: true,
  refused: true,
  unverifiable: true,
  not_yet_delivered: true,
};
const NOT_CARRIED_REASONS: Record<NotCarriedReason['reason'], true> = {
  'no-eligible': true,
  'domain-short': true,
  'served-first': true,
  'no-priority-chosen': true,
  'method-not-run': true,
  'inputs-gone': true,
};

function isAnswerReason(value: unknown): value is AnswerReason {
  return typeof value === 'string' && Object.hasOwn(ANSWER_REASONS, value);
}

/** The tag only: a variant's own fields are the view's to read, not this surface's. */
function isNotCarriedReason(value: unknown): value is NotCarriedReason {
  return isRecord(value) && typeof value.reason === 'string' && Object.hasOwn(NOT_CARRIED_REASONS, value.reason);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Why a route read failed, in the answer vocabulary. */
export function reasonForFailure(error: unknown): AnswerReason {
  if (error instanceof StorageError) {
    if (error.statusCode === 401 || error.statusCode === 403) return 'refused';
    if (error.statusCode === undefined && /timeout/i.test(error.message)) return 'timeout';
  }
  return 'transport_error';
}

/** A network status entry, read strictly: anything this surface cannot read is null. */
function readStatus(value: unknown, need: NeedRef): NeedStatus | null {
  if (!isRecord(value)) return null;
  switch (value.state) {
    case 'present':
      return { need, state: 'present' };
    case 'absent':
      // The same guard the HTTP tier reads its trailer with, so both adapters read `Unanswered`
      // at one strictness: a come-back time is a number, a 'nothing' says why.
      return isUnanswered(value.unanswered, isNotCarriedReason)
        ? { need, state: 'absent', unanswered: value.unanswered as Unanswered<NotCarriedReason> }
        : null;
    case 'unreachable':
      return isAnswerReason(value.reason) ? { need, state: 'unreachable', reason: value.reason } : null;
    default:
      return null;
  }
}

function readNeedRef(value: unknown): NeedRef | null {
  if (!isRecord(value)) return null;
  switch (value.field) {
    case 'closure':
    case 'projectable':
      return { field: value.field };
    case 'loss':
    case 'indexed':
      return typeof value.contentType === 'string' ? { field: value.field, contentType: value.contentType } : null;
    case 'compute':
      return typeof value.scope === 'string' ? { field: 'compute', scope: value.scope } : null;
    default:
      return null;
  }
}

/**
 * The network answers by need id. A device-plane entry from the remote is dropped: the remote
 * never reports a residency need (R6).
 */
async function networkAnswers(
  routes: PlacementRoutes,
  appId: string,
): Promise<{ byId: Map<string, unknown> } | { failed: AnswerReason }> {
  let body: unknown;
  try {
    body = await routes.getJson<unknown>(placementRoutes(appId).status);
  } catch (error) {
    return { failed: reasonForFailure(error) };
  }
  const byId = new Map<string, unknown>();
  const needs = isRecord(body) && Array.isArray(body.needs) ? body.needs : [];
  for (const entry of needs) {
    if (!isRecord(entry)) continue;
    const need = readNeedRef(entry.need);
    if (need === null || isDevicePlane(need)) continue;
    byId.set(needId(need), entry);
  }
  return { byId };
}

function pendingOf(status: NeedStatus, nowSecs: number, pollMs: number): PendingStatus<NeedStatus, NotCarriedReason> {
  switch (status.state) {
    case 'present':
      return { status: 'answered', value: status };
    case 'absent':
      return { status: 'unanswered', unanswered: status.unanswered };
    case 'unreachable':
      return { status: 'retrying-at', epochSecs: nowSecs + Math.ceil(pollMs / 1000) };
  }
}

/**
 * What a report says about the need, without the retry time `watch` derives from the clock: an
 * unreachable need's reading is its reason (or that the worker did not answer), never the moving
 * `retrying-at` instant. `watch` yields only when this changes, so a need that stays unreachable
 * stays quiet (R7: a quiet need yields nothing; §2.0: unchanged inputs return nothing).
 */
function readingOf(status: NeedStatus | null, next: PendingStatus<NeedStatus, NotCarriedReason>): string {
  if (next.status === 'retrying-at') {
    return status === null ? 'retrying:declared-unknown' : `retrying:${status.state === 'unreachable' ? status.reason : status.state}`;
  }
  return JSON.stringify(next);
}

/**
 * The `Placement` for one app. `client` is the app's `StorageClient` (any object with its
 * `getJson`), `device` the worker's port.
 */
export function placementFor(
  appId: string,
  client: PlacementRoutes,
  device: DevicePlacement,
  options: PlacementOptions = {},
): Placement {
  const pollMs = options.pollMs ?? 30_000;
  const minWaitMs = options.minWaitMs ?? 1_000;
  const now = options.now ?? (() => Date.now());
  const sleep = options.sleep ?? ((ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms)));

  async function statusOf(needs: readonly NeedRef[]): Promise<NeedStatus[]> {
    const network = needs.some((n) => !isDevicePlane(n)) ? await networkAnswers(client, appId) : null;
    return Promise.all(
      needs.map(async (need): Promise<NeedStatus> => {
        if (isDevicePlane(need)) {
          try {
            return await device.status(need);
          } catch {
            return { need, state: 'unreachable', reason: 'transport_error' };
          }
        }
        if (network === null || 'failed' in network) {
          return { need, state: 'unreachable', reason: network?.failed ?? 'transport_error' };
        }
        const entry = network.byId.get(needId(need));
        // Declared but not answered by the route: delivery has not reached it yet.
        if (entry === undefined) return { need, state: 'unreachable', reason: 'not_yet_delivered' };
        return readStatus(entry, need) ?? { need, state: 'unreachable', reason: 'unverifiable' };
      }),
    );
  }

  return {
    /** Rejects when the worker cannot say what the app declares: an empty list would read as "no needs". */
    async status() {
      return statusOf(await device.declared());
    },

    watch(need) {
      const id = needId(need);
      return {
        async *[Symbol.asyncIterator]() {
          let last: string | null = null;
          for (;;) {
            let declared: readonly NeedRef[] | null = null;
            try {
              declared = await device.declared();
            } catch {
              declared = null; // the worker did not answer: not a withdrawal
            }
            let next: PendingStatus<NeedStatus, NotCarriedReason>;
            let status: NeedStatus | null = null;
            if (declared !== null && !declared.some((n) => needId(n) === id)) {
              next = { status: 'withdrawn' };
            } else if (declared === null) {
              next = { status: 'retrying-at', epochSecs: Math.floor(now() / 1000) + Math.ceil(pollMs / 1000) };
            } else {
              [status] = await statusOf([need]);
              next = pendingOf(status, Math.floor(now() / 1000), pollMs);
            }
            const reading = readingOf(status, next);
            if (reading !== last) {
              last = reading;
              yield next;
            }
            if (next.status === 'answered' || next.status === 'withdrawn') return;
            // Come back when the answer said it can change, else on the poll interval.
            let wait = pollMs;
            if (next.status === 'unanswered' && next.unanswered.comeBack.when === 'at') {
              const until = next.unanswered.comeBack.epochSecs * 1000 - now();
              // A time already past says nothing about when to look again: poll, never spin.
              wait = until > 0 ? Math.max(minWaitMs, until) : pollMs;
            }
            await sleep(wait);
          }
        },
      };
    },

    async why(need) {
      if (isDevicePlane(need)) {
        const status = await device.status(need);
        switch (status.state) {
          case 'present':
            return { plane: 'device', kept: true, deferred: false, unanswered: null };
          case 'absent':
            return {
              plane: 'device',
              kept: false,
              deferred: true,
              unanswered: { ...status.unanswered, why: status.unanswered.why.reason },
            };
          case 'unreachable':
            return {
              plane: 'device',
              kept: false,
              deferred: false,
              unanswered: {
                why: status.reason,
                comeBack: { when: 'on-event', event: 'tier-reachable' },
                meanwhile: { local: 'nothing', because: 'the device tier did not answer' },
                escalation: null,
              },
            };
        }
      }
      const view = await client.getJson<PlacementExplanationView>(placementRoutes(appId).why(need));
      return { plane: 'network', view };
    },
  };
}
