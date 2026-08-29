/**
 * `Answer<T>` — what a peer told us, including when it told us nothing.
 *
 * Ported from the shared Rust contract at `crates/seam-contracts/src/answer.rs`,
 * deliberately WITHOUT growing a fourth state. That file argues the point
 * directly: "Refusal and unverifiability are *reasons*, not states: a seam that
 * must distinguish 'the peer declined' from 'the peer timed out' pairs an
 * `Answer` with its own `ReasonLabel` enum rather than growing a fourth variant
 * here. Keeping the state set at three is what lets every seam share one
 * vocabulary."
 *
 * There is a real argument for a fourth `refused` state over HTTP — a peer
 * rejecting who you are is the single most likely three-peer disagreement, and
 * collapsing it into `unreachable` misinforms, because `unreachable` is
 * specified as "absence NOT established", so a client reading two-of-three
 * unreachable concludes the network is degraded rather than that its identity
 * is not recognised there.
 *
 * That argument is answered by making the reason REQUIRED rather than by
 * forking the vocabulary. `{ state: 'unreachable', reason: 'refused' }` carries
 * exactly the information a fourth variant would, cannot be produced by
 * accident, and keeps one state set across Rust and TypeScript. The Rust side
 * already pairs `Answer` with a reason enum; requiring the pairing here is
 * closer to that design than adding a variant would be.
 *
 * # The distinction that actually matters
 *
 * `absent` is a POSITIVE claim and may only be constructed where absence was
 * genuinely observed — the responder answered, and answered "no such thing".
 * `unreachable` establishes nothing in either direction. Conflating them is how
 * a client concludes a human has no recovery contacts because a peer was slow.
 */

/** Why a non-present answer is non-present. Never optional — see the module note. */
export type AnswerReason =
  /** The responder answered and reported the thing does not exist (e.g. 404). */
  | 'not-found'
  /** The responder declined us: 401/403, an unknown issuer, a rejected identity. */
  | 'refused'
  /** No response within the deadline. */
  | 'timeout'
  /** Transport failed outright — DNS, connection refused, abort. */
  | 'transport'
  /**
   * An https page cannot fetch an http peer. Named because the failure is
   * structural and permanent for that candidate, not a transient outage, and a
   * client that cannot tell them apart will retry forever.
   */
  | 'mixed-content-blocked'
  /** A response arrived but was not a usable document — see `docRejected`. */
  | 'document-rejected'
  /** The responder answered with a status we do not model. */
  | 'unexpected-status';

export interface AnswerPresent<T> {
  readonly state: 'present';
  readonly value: T;
}

export interface AnswerAbsent {
  readonly state: 'absent';
  /** Always `'not-found'` today; carried so every non-present answer explains itself. */
  readonly reason: AnswerReason;
  readonly detail?: string;
}

export interface AnswerUnreachable {
  readonly state: 'unreachable';
  readonly reason: AnswerReason;
  readonly detail?: string;
}

export type Answer<T> = AnswerPresent<T> | AnswerAbsent | AnswerUnreachable;

export function present<T>(value: T): AnswerPresent<T> {
  return { state: 'present', value };
}

/**
 * An OBSERVED absence. Use only where the "no such thing" provably came back
 * from a responder — there is deliberately no conversion from a bare
 * `undefined`, because a bare conversion makes the wrong mapping the ergonomic
 * default (the same reason the Rust side omits `impl From<Option<T>>`).
 */
export function absent(reason: AnswerReason = 'not-found', detail?: string): AnswerAbsent {
  return { state: 'absent', reason, ...(detail === undefined ? {} : { detail }) };
}

export function unreachable(reason: AnswerReason, detail?: string): AnswerUnreachable {
  return { state: 'unreachable', reason, ...(detail === undefined ? {} : { detail }) };
}

/**
 * A response arrived but is not usable.
 *
 * NOT `absent`: the doorway did not tell us it has no document, so we have not
 * observed absence and must not claim it. `unreachable` is the honest state —
 * we could not obtain a usable document — and the reason says why, so this is
 * never confused with a network outage.
 */
export function docRejected(detail: string): AnswerUnreachable {
  return unreachable('document-rejected', detail);
}

export function isPresent<T>(answer: Answer<T>): answer is AnswerPresent<T> {
  return answer.state === 'present';
}

/** The value, or a caller-supplied stand-in. Never throws. */
export function valueOr<T>(answer: Answer<T>, fallback: T): T {
  return isPresent(answer) ? answer.value : fallback;
}

/**
 * Classify a fetch rejection.
 *
 * `AbortError` is the deadline this client imposed, which is a timeout and not
 * a transport failure; everything else is transport. Both are `unreachable` —
 * a request that never completed establishes nothing about what exists.
 */
export function unreachableFromError(error: unknown): AnswerUnreachable {
  const name = error instanceof Error ? error.name : '';
  const message = error instanceof Error ? error.message : String(error);
  if (name === 'AbortError' || name === 'TimeoutError') {
    return unreachable('timeout', message);
  }
  return unreachable('transport', message);
}

/**
 * Classify a non-OK HTTP status.
 *
 * 404 is the ONLY status that yields `absent`, because it is the only one where
 * the responder answered the question we asked. 401/403 is `refused` — it says
 * something about us, not about whether the thing exists.
 */
export function answerFromStatus(
  status: number,
  detail?: string
): AnswerAbsent | AnswerUnreachable {
  if (status === 404) return absent('not-found', detail);
  if (status === 401 || status === 403) return unreachable('refused', detail);
  return unreachable('unexpected-status', detail ?? `HTTP ${status}`);
}
