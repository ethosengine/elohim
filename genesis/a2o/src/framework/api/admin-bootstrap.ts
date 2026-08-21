/**
 * Admin bootstrap — make the suite's steward actually an admin on THIS doorway.
 *
 * The admin scenarios (hosted user management, conductor pool visibility,
 * operator onboarding) all assume the fixture steward Matthew logs in with
 * `permission_level: ADMIN`. That is not a property of the a2o suite — it is a
 * property of how the deployment was PROVISIONED. `genesis/seeder/src/seed-humans.ts`
 * only promotes Matthew when `API_KEY_ADMIN` is present in the seeder's own
 * environment (it passes it as `adminBootstrapKey` on register, and
 * `auth_routes.rs` promotes only on an exact match). A deployment seeded without
 * that variable holds a perfectly valid Matthew at `AUTHENTICATED`, and every
 * admin scenario then dies on
 *
 *   GET /admin/users returned 403: {"error":"Admin permission required"}
 *
 * — which reads as a product red but is a provisioning gap. Measured on the
 * local mesh 2026-08-21: `POST /auth/login` as matthew.dowell@alpha.elohim.host
 * returned a token claiming `"permission_level":"AUTHENTICATED"`.
 *
 * This module closes that gap the same way CI does (`genesis/scripts/ci/
 * doorway-seed-ensure.sh`): from the admin key alone, with no second stored
 * secret and no product change.
 *
 *   1. Idempotently register a dedicated bootstrap actor carrying
 *      `adminBootstrapKey` — the register handler promotes it to Admin.
 *      Its password is DERIVED from the key (sha256), so nothing is stored:
 *      only a holder of the admin key can derive it, and that key is already a
 *      total-admin secret, so the actor adds zero marginal exposure.
 *   2. Log that actor in to mint an Admin JWT.
 *   3. `PUT /admin/users/{id}/permission {"permissionLevel":"ADMIN"}` on the
 *      fixture steward's row — an ordinary admin operation, and the only way to
 *      lift an ALREADY-EXISTING row (register returns 409 on conflict and never
 *      re-promotes).
 *
 * ENV-GATED AND FAIL-SOFT BY DESIGN. With no admin key in the environment this
 * is a no-op, so a run against a deployment where the caller holds no operator
 * credential behaves exactly as before — it simply fails later on the real 403.
 * Every step is best-effort: if provisioning cannot be completed the scenario
 * proceeds and reports the doorway's own error rather than this module's.
 */

import { createHash } from 'node:crypto';

import { request } from 'undici';

/** Identifier of the dedicated bootstrap actor (one per deployment). */
const BOOTSTRAP_IDENTIFIER = 'a2o-admin-bootstrap@test.elohim.host';

/**
 * Domain-separation salt for the DERIVED password (mirrors doorway-seed-ensure.sh).
 * Not a password: the password is `sha256(adminKey + ':' + this)`, so it exists
 * only for a holder of the admin key and is stored nowhere.
 */
// eslint-disable-next-line sonarjs/no-hardcoded-passwords -- salt, not a credential
const PASSWORD_SALT = 'a2o-admin-bootstrap';

const REQUEST_TIMEOUT_MS = 15_000;

/** Memoized per `${doorwayUrl}|${identifier}` — one ensure per suite run. */
const ensured = new Map<string, Promise<boolean>>();

/**
 * The admin bootstrap key this run should send, if any.
 *
 * `E2E_ADMIN_BOOTSTRAP_KEY` is the suite's own override; `API_KEY_ADMIN` is the
 * name the doorway, the seeder, and `just mesh status` all use for the same
 * secret, so a mesh run that exports it needs no a2o-specific variable.
 */
export function adminBootstrapKey(): string | undefined {
  const explicit = process.env['E2E_ADMIN_BOOTSTRAP_KEY'];
  if (explicit) return explicit;
  const shared = process.env['API_KEY_ADMIN'];
  return shared === undefined || shared === '' ? undefined : shared;
}

/** The part before the `@` — stable across the doorway's gateway re-qualification. */
function localPart(identifier: string): string {
  return identifier.split('@')[0];
}

function derivedPassword(key: string): string {
  return createHash('sha256').update(`${key}:${PASSWORD_SALT}`).digest('hex').slice(0, 48);
}

interface HttpResult {
  status: number;
  text: string;
}

async function send(
  url: string,
  method: 'GET' | 'POST' | 'PUT',
  token?: string,
  payload?: unknown
): Promise<HttpResult> {
  const headers: Record<string, string> = { accept: 'application/json' };
  if (token) headers['authorization'] = `Bearer ${token}`;
  if (payload !== undefined) headers['content-type'] = 'application/json';
  const { statusCode, body } = await request(url, {
    method,
    headers,
    body: payload === undefined ? undefined : JSON.stringify(payload),
    headersTimeout: REQUEST_TIMEOUT_MS,
    bodyTimeout: REQUEST_TIMEOUT_MS,
  });
  return { status: statusCode, text: await body.text() };
}

/** Mint an Admin JWT by provisioning (or reusing) the bootstrap actor. */
async function mintAdminToken(doorwayUrl: string, key: string): Promise<string | null> {
  const base = doorwayUrl.replace(/\/$/, '');
  const password = derivedPassword(key);

  // Register is idempotent in effect: 200/201 provisions, 409 means a previous
  // run already did. Any other status means the key is not the doorway's key.
  const reg = await send(`${base}/auth/register`, 'POST', undefined, {
    identifier: BOOTSTRAP_IDENTIFIER,
    identifierType: 'email',
    password,
    displayName: 'A2O admin bootstrap',
    bio: 'Non-human a2o provisioning actor — lifts the fixture steward to Admin.',
    profileReach: 'private',
    adminBootstrapKey: key,
  });
  if (reg.status !== 200 && reg.status !== 201 && reg.status !== 409) return null;

  const login = await send(`${base}/auth/login`, 'POST', undefined, {
    identifier: BOOTSTRAP_IDENTIFIER,
    password,
  });
  if (login.status !== 200) return null;
  const parsed = JSON.parse(login.text) as { token?: string };
  return parsed.token ?? null;
}

/**
 * Ensure `identifier` carries `ADMIN` on `doorwayUrl`.
 *
 * Returns true when the account is (now) Admin, false when nothing could be
 * done — no key configured, key rejected, or the row is not there. Never throws:
 * a scenario that still needs admin will fail on its own assertion with the
 * doorway's own message, which is the honest red.
 */
export async function ensureFixtureAdmin(doorwayUrl: string, identifier: string): Promise<boolean> {
  const cacheKey = `${doorwayUrl}|${identifier}`;
  const cached = ensured.get(cacheKey);
  if (cached) return cached;

  const run = (async (): Promise<boolean> => {
    const key = adminBootstrapKey();
    if (!key) return false;
    try {
      const base = doorwayUrl.replace(/\/$/, '');
      const token = await mintAdminToken(base, key);
      if (!token) return false;

      const search = encodeURIComponent(localPart(identifier));
      const list = await send(`${base}/admin/users?search=${search}&limit=50`, 'GET', token);
      if (list.status !== 200) return false;
      const users = (
        JSON.parse(list.text) as {
          users?: { id: string; identifier: string; permissionLevel: string }[];
        }
      ).users;
      const row = users?.find(u => localPart(u.identifier) === localPart(identifier));
      if (!row) return false;
      if (row.permissionLevel.toUpperCase() === 'ADMIN') return true;

      const promote = await send(
        `${base}/admin/users/${encodeURIComponent(row.id)}/permission`,
        'PUT',
        token,
        { permissionLevel: 'ADMIN' }
      );
      return promote.status === 200;
    } catch {
      // Unreachable doorway / malformed response: leave the run to fail on its
      // own assertion rather than on provisioning.
      return false;
    }
  })();

  ensured.set(cacheKey, run);
  return run;
}
