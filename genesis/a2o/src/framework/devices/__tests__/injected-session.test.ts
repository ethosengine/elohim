/**
 * The harness's injected sign-in persists exactly what a real sign-in persists
 * (ruling R-A14). A browser lane injects a session instead of typing into the
 * login form; if the injected key set drifts from the one the shell's
 * `BrowserSessionTokenStore.set` + `setProviderType` write, `restoreSession()`
 * clears it at boot and every signed-in scenario runs signed out, silently.
 *
 * The app's key set is read from the app's own source, not restated here, so a
 * key added to (or dropped from) a real sign-in reds this test until the
 * harness follows it.
 */

import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { signedInSessionEntries } from '../playwright-device.js';

import type { AuthResponse } from '../../api/doorway-client.js';

const here = fileURLToPath(new URL('.', import.meta.url));
const identityPillarSrc = resolve(here, '../../../../../../app/elohim-app/src/app/imagodei');

/** `export const AUTH_TOKEN_KEY = 'elohim-auth-token'` → { AUTH_TOKEN_KEY: 'elohim-auth-token' }. */
function appKeyConstants(): Map<string, string> {
  const source = readFileSync(resolve(identityPillarSrc, 'models/auth.model.ts'), 'utf8');
  const constants = new Map<string, string>();
  for (const m of source.matchAll(/export const (\w+_KEY) = '([^']+)'/g)) {
    constants.set(m[1], m[2]);
  }
  return constants;
}

/** Every key the shell's session store writes on a sign-in (its only `setItem` sites). */
function keysARealSignInPersists(): Set<string> {
  const constants = appKeyConstants();
  const source = readFileSync(
    resolve(identityPillarSrc, 'services/browser-session-token.store.ts'),
    'utf8'
  );
  const keys = new Set<string>();
  for (const m of source.matchAll(/localStorage\.setItem\((\w+),/g)) {
    const value = constants.get(m[1]);
    assert.ok(value, `the session store writes ${m[1]}, which auth.model.ts does not declare`);
    keys.add(value);
  }
  assert.ok(keys.size > 0, 'no setItem sites found in the session store — the reader is stale');
  return keys;
}

/** A JWT whose payload carries `claims` (unsigned; only the payload is read). */
function jwt(claims: Record<string, unknown>): string {
  const part = (o: unknown) => Buffer.from(JSON.stringify(o)).toString('base64url');
  return `${part({ typ: 'JWT', alg: 'HS256' })}.${part(claims)}.signature`;
}

const nowSeconds = () => Math.floor(Date.now() / 1000);

function loginAnswer(overrides: Partial<AuthResponse> = {}): AuthResponse {
  return {
    token: jwt({ sub: 'human-jessica-sp', exp: nowSeconds() + 3600 }),
    humanId: 'human-jessica-sp',
    agentPubKey: 'uhCAkWvdv8-oTHK6',
    identifier: 'jessica@household.test',
    expiresAt: nowSeconds() + 3600,
    installedAppId: 'elohim-jessica',
    ...overrides,
  };
}

void test('the injected key set equals the key set a real sign-in persists', () => {
  const written = new Set(Object.keys(signedInSessionEntries(loginAnswer())));
  const byName = (a: string, b: string) => a.localeCompare(b);
  assert.deepEqual([...written].sort(byName), [...keysARealSignInPersists()].sort(byName));
});

void test('the expiry is the one the login answer carries, in the unix seconds the store writes', () => {
  const answer = loginAnswer();
  const entries = signedInSessionEntries(answer);
  assert.equal(entries['elohim-auth-expiry'], String(answer.expiresAt));
  assert.equal(entries['elohim-auth-provider'], 'password');
  assert.equal(entries['elohim-auth-token'], answer.token);
});

void test('the session reads as restorable to the shell: an expiry after now and a provider', () => {
  const entries = signedInSessionEntries(loginAnswer());
  // DoorwaySessionClient.restoreSession clears a session whose expiresAt <= now;
  // AuthService.restoreSession refuses one with no provider.
  assert.ok(Number(entries['elohim-auth-expiry']) > nowSeconds());
  assert.ok(entries['elohim-auth-provider']);
});

void test('absent optional fields are left unwritten, exactly as the store leaves them', () => {
  const entries = signedInSessionEntries(loginAnswer({ installedAppId: undefined }));
  assert.equal('elohim-installed-app-id' in entries, false);
});

void test("an answer with no expiresAt takes the token's own exp claim", () => {
  const exp = nowSeconds() + 1800;
  const answer = loginAnswer({ token: jwt({ exp }) });
  delete (answer as Partial<AuthResponse>).expiresAt;
  assert.equal(signedInSessionEntries(answer)['elohim-auth-expiry'], String(exp));
});

void test('an answer with no expiresAt and a token with no exp claim is refused, naming the field', () => {
  const answer = loginAnswer({ token: jwt({ sub: 'someone' }) });
  delete (answer as Partial<AuthResponse>).expiresAt;
  assert.throws(() => signedInSessionEntries(answer), /expiresAt/);
});
