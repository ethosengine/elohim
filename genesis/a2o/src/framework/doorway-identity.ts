/**
 * What a doorway calls a human — the one place that answers it.
 *
 * A doorway GATEWAY-SCOPES identifiers on register AND on login: it re-qualifies
 * the local part with its own configured domain
 * (`doorway/doorway-service/src/routes/auth_routes.rs`, `gateway_domain` +
 * `normalize_identifier`). So `susan@test.elohim.host` is stored and returned as
 * `susan@alpha.elohim.host`, and a bare `portal-a2o-<uuid>` comes back as
 * `portal-a2o-<uuid>@alpha.elohim.host`. **The credentials a scenario TYPED are
 * not the credentials the doorway KEEPS.**
 *
 * A doorway with no URL configured leaves identifiers untouched, so the same
 * human is named differently depending on the deployment under test.
 *
 * There are two ways a test could cope with that, and only one of them is right.
 *
 * WRONG: re-derive the rule — reimplement `gateway_domain` here and compose the
 * expected identifier. That makes a second hand-maintained home for a convention
 * that already has one, guarded by nothing but a comment asking the two to agree,
 * and it is still wrong wherever a test reaches a doorway at an address other
 * than the one that doorway was configured with.
 *
 * RIGHT: ASK. The doorway states its own convention in every auth response, so
 * the identifier it returned when this human's session was established is
 * authoritative. The identifier the scenario requested is kept only as a
 * fallback, for a run where no session has been established yet.
 *
 * Naive `assert.strictEqual(me.identifier, human.credentials.identifier)` is how
 * the fleet, rather than the household mesh, discovered a portal scenario
 * asserting a bare name (genesis #1519, 2026-08-29 — backlog
 * `portal-login-step-domain-scoped-identifier`). The mesh could not have caught
 * it: its doorways ran with no configured URL until the same day, so they never
 * expressed the convention at all (`MESH_DOORWAY_GATEWAY_SCOPING` in
 * `app/elohim-app/scripts/hc-mesh.sh`).
 */

import { BrowserDevice } from './devices/browser-device.js';

import type { Human } from './human.js';

/** The part before the `@` — stable across gateway re-qualification. */
export function localPart(identifier: string): string {
  return identifier.split('@')[0];
}

/**
 * Every identifier the doorway could be storing this human under, most
 * authoritative first: what the doorway itself named in this human's session,
 * then what the scenario asked for.
 *
 * Note the second entry is also what a LEGACY row carries — alpha holds both
 * `susan@alpha.elohim.host` and a pre-normalization `susan@test.elohim.host` —
 * so keeping it serves lookup as well as un-scoped deployments.
 */
export function identifierCandidates(human: Human): string[] {
  const candidates: string[] = [];
  const device = human.devices[0];
  if (device instanceof BrowserDevice) {
    const sessionIdentifier = device.client.session?.identifier;
    if (sessionIdentifier) candidates.push(sessionIdentifier);
  }
  if (!candidates.includes(human.credentials.identifier)) {
    candidates.push(human.credentials.identifier);
  }
  return candidates;
}

/**
 * Does `actual` name this human on the doorway under test?
 *
 * True for the identifier the doorway issued for their session, and for the one
 * the scenario requested. Deliberately NOT true for a matching local part under
 * some other domain: that is a different account, and tolerating it would let a
 * cross-doorway confusion pass as a match.
 */
export function namesHuman(actual: string | undefined, human: Human): boolean {
  return typeof actual === 'string' && identifierCandidates(human).includes(actual);
}

/** Assertion message that shows what was accepted, so a red is self-explaining. */
export function expectedIdentifiersFor(human: Human): string {
  return identifierCandidates(human)
    .map(c => JSON.stringify(c))
    .join(' or ');
}
