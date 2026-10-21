/**
 * The wire-crossing types of seams A and 2.0: `Unanswered`, `ItemAbsence`, `TierRecord`.
 *
 * The interfaces spec (§H.1) has these GENERATED from lane S's schemas (`objects/unanswered`,
 * `item-absence`, `tier-record`) into `src/generated/`. Those schemas and the codegen entries are
 * not in the tree yet, so these are the provisional shapes, deliberately outside `generated/`,
 * pinned by the shared vectors that Rust's serializer wrote (`fixtures/tree-vectors.json`) and
 * checked by the guards below. When generation lands this module re-exports the generated types
 * and keeps only the guards and constructors.
 */

export type ReachReason =
  | 'outside-standing'
  | 'no-record'
  | 'not-under-root'
  | 'identity-required'
  | 'authorizer-unavailable';

export type ItemAbsence =
  | { kind: 'not-held'; tried: readonly string[] }
  | { kind: 'refused'; by: string; refusal: { reason: ReachReason; needsStanding: string | null } }
  | { kind: 'over-budget'; by: string; rule: string }
  | { kind: 'unreachable'; tier: string }
  | { kind: 'membership-unknown'; by: string }
  | { kind: 'integrity'; from: string };
export type ComeBack = { when: 'at'; epochSecs: number } | { when: 'on-event'; event: string };
export type Meanwhile =
  | { local: 'use-held'; what: string }
  | { local: 'run-here' }
  | { local: 'built-in-in-force' }
  | { local: 'nothing'; because: string };
export interface Unanswered<R> {
  why: R;
  comeBack: ComeBack;
  meanwhile: Meanwhile;
  escalation: { asked: number; changed: string; whoCanBeAsked: string[] } | null;
}

export type TierClass =
  | 'own'
  | 'person-device'
  | 'household'
  | 'hub'
  | 'holon-hub'
  | 'pool'
  | 'doorway-peer'
  | 'doorway';
/** Narrowest first: the order `TierClass` derives in Rust. */
export const TIER_CLASS_ORDER: readonly TierClass[] = [
  'own',
  'person-device',
  'household',
  'hub',
  'holon-hub',
  'pool',
  'doorway-peer',
  'doorway',
];
export type TierSource = { source: 'held-record'; record: string } | { source: 'doorway'; doorway: string };
export type TierAddress = { form: 'url'; base: string } | { form: 'multiaddr'; addr: string };
export interface Proximity {
  networkRank: 0 | 1 | 2;
  warmRoots: string[];
  servesExtracted: boolean;
  rttMs: number | null;
  lastSeenMs: number | null;
}
export interface TierRecord {
  id: string;
  class: TierClass;
  sources: TierSource[];
  endpoint: TierAddress | null;
  /** Reach::openness(); null = unknown. */
  standing: number | null;
  proximity: Proximity | null;
}

export const EVENT_TIER_REACHABLE = 'tier-reachable';
export const EVENT_STANDING_CHANGED = 'standing-changed';

function nothing(because: string): Meanwhile {
  return { local: 'nothing', because };
}

function onEvent(event: string): ComeBack {
  return { when: 'on-event', event };
}

/** The same three parts Rust's `ItemAbsence` constructors give. */
export const absence = {
  notHeld(tried: readonly string[]): Unanswered<ItemAbsence> {
    return {
      why: { kind: 'not-held', tried: [...tried] },
      comeBack: onEvent(EVENT_TIER_REACHABLE),
      meanwhile: nothing(
        tried.length === 0 ? 'the object is not held here and no tier was asked' : 'no tier asked holds it',
      ),
      escalation: null,
    };
  },
  refused(by: string, reason: ReachReason, needsStanding: string | null = null): Unanswered<ItemAbsence> {
    return {
      why: { kind: 'refused', by, refusal: { reason, needsStanding } },
      comeBack: onEvent(EVENT_STANDING_CHANGED),
      meanwhile: nothing('the requester lacks the standing this root needs'),
      escalation: null,
    };
  },
  overBudget(by: string, rule: string, retryAtEpochSecs: number): Unanswered<ItemAbsence> {
    return {
      why: { kind: 'over-budget', by, rule },
      comeBack: { when: 'at', epochSecs: retryAtEpochSecs },
      meanwhile: nothing('the forwarding budget is spent until the retry time'),
      escalation: null,
    };
  },
  unreachable(tier: string): Unanswered<ItemAbsence> {
    return {
      why: { kind: 'unreachable', tier },
      comeBack: onEvent(EVENT_TIER_REACHABLE),
      meanwhile: nothing('the tier did not answer'),
      escalation: null,
    };
  },
  membershipUnknown(by: string): Unanswered<ItemAbsence> {
    return {
      why: { kind: 'membership-unknown', by },
      comeBack: onEvent(EVENT_TIER_REACHABLE),
      meanwhile: nothing("the tier does not hold the path's spine"),
      escalation: null,
    };
  },
  integrity(from: string): Unanswered<ItemAbsence> {
    return {
      why: { kind: 'integrity', from },
      comeBack: onEvent(EVENT_TIER_REACHABLE),
      meanwhile: nothing('the bytes offered failed their address'),
      escalation: null,
    };
  },
};

/** Replace every indexical `own` in a remote answer with the id the requester dialled (A2). */
export function rewriteOwn(why: ItemAbsence, dialled: string): ItemAbsence {
  const fix = (id: string) => (id === 'own' ? dialled : id);
  switch (why.kind) {
    case 'not-held':
      return { kind: 'not-held', tried: [...new Set(why.tried.map(fix))] };
    case 'refused':
      return { ...why, by: fix(why.by) };
    case 'over-budget':
      return { ...why, by: fix(why.by) };
    case 'membership-unknown':
      return { ...why, by: fix(why.by) };
    case 'unreachable':
      return { ...why, tier: fix(why.tier) };
    case 'integrity':
      return { ...why, from: fix(why.from) };
  }
}

const TIER_ID = /^(agent|transport):.+$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function nullableString(value: unknown): boolean {
  return value === null || typeof value === 'string';
}

/** A serialized tier id: `agent:<key>` or `transport:<peer id>`; `own` never crosses the wire. */
export function isTierId(value: unknown): value is string {
  return typeof value === 'string' && TIER_ID.test(value);
}

export function isItemAbsence(value: unknown): value is ItemAbsence {
  if (!isRecord(value)) return false;
  switch (value.kind) {
    case 'not-held':
      return Array.isArray(value.tried) && value.tried.every(isTierId);
    case 'refused':
      return (
        isTierId(value.by) &&
        isRecord(value.refusal) &&
        typeof value.refusal.reason === 'string' &&
        nullableString(value.refusal.needsStanding)
      );
    case 'over-budget':
      return isTierId(value.by) && typeof value.rule === 'string';
    case 'unreachable':
      return isTierId(value.tier);
    case 'membership-unknown':
      return isTierId(value.by);
    case 'integrity':
      return isTierId(value.from);
    default:
      return false;
  }
}

export function isUnanswered(value: unknown, why: (v: unknown) => boolean): value is Unanswered<unknown> {
  if (!isRecord(value) || !why(value.why)) return false;
  const cb = value.comeBack;
  const mw = value.meanwhile;
  const comeBackOk =
    isRecord(cb) &&
    ((cb.when === 'at' && typeof cb.epochSecs === 'number') || (cb.when === 'on-event' && typeof cb.event === 'string'));
  const meanwhileOk =
    isRecord(mw) &&
    ((mw.local === 'use-held' && typeof mw.what === 'string') ||
      mw.local === 'run-here' ||
      mw.local === 'built-in-in-force' ||
      (mw.local === 'nothing' && typeof mw.because === 'string'));
  const esc = value.escalation;
  const escalationOk =
    esc === null ||
    (isRecord(esc) && typeof esc.asked === 'number' && typeof esc.changed === 'string' && Array.isArray(esc.whoCanBeAsked));
  return comeBackOk && meanwhileOk && escalationOk;
}

export function isTierRecord(value: unknown): value is TierRecord {
  if (!isRecord(value) || !isTierId(value.id)) return false;
  if (typeof value.class !== 'string' || !TIER_CLASS_ORDER.includes(value.class as TierClass)) return false;
  if (!Array.isArray(value.sources) || value.sources.length === 0) return false;
  const sourcesOk = value.sources.every(
    (s) =>
      isRecord(s) &&
      ((s.source === 'held-record' && typeof s.record === 'string') || (s.source === 'doorway' && isTierId(s.doorway))),
  );
  const ep = value.endpoint;
  const endpointOk =
    ep === null || (isRecord(ep) && ((ep.form === 'url' && typeof ep.base === 'string') || (ep.form === 'multiaddr' && typeof ep.addr === 'string')));
  const standingOk = value.standing === null || typeof value.standing === 'number';
  const px = value.proximity;
  const proximityOk =
    px === null ||
    (isRecord(px) &&
      typeof px.networkRank === 'number' &&
      Array.isArray(px.warmRoots) &&
      typeof px.servesExtracted === 'boolean' &&
      (px.rttMs === null || typeof px.rttMs === 'number') &&
      (px.lastSeenMs === null || typeof px.lastSeenMs === 'number'));
  return sourcesOk && endpointOk && standingOk && proximityOk;
}
