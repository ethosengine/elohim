/**
 * The chaperone connect presents ONE signing key per device, not a fresh one
 * per page load — and heals exactly once when the conductor rejects it.
 *
 * Every `POST /hc/connect` with an unknown key makes the doorway author a
 * permanent Holochain CapGrant per role cell, and the conductor re-reads every
 * grant on every zome call. This spec pins the two behaviours that bound that:
 * reuse across reloads, and a heal that cannot loop.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { CHAPERONE_CREDENTIALS_KEY } from './chaperone-credential-store';
import { DoorwayConnectionStrategy } from './doorway-connection-strategy';

import type { ConnectionConfig, Logger } from './connection-strategy';

// A doorway JWT carrying the agent claim the credential slot is scoped by.
function tokenFor(agent: string): string {
  const payload = Buffer.from(JSON.stringify({ agent_pub_key: agent }))
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
  return `header.${payload}.signature`;
}

const SILENT: Logger = {
  debug: () => undefined,
  info: () => undefined,
  warn: () => undefined,
  error: () => undefined,
};

function config(agent = 'uhCAkAgentOne'): ConnectionConfig {
  return {
    mode: 'doorway',
    adminUrl: 'https://doorway.test',
    appUrl: 'https://doorway.test',
    hAppId: 'elohim',
    doorwayToken: tokenFor(agent),
    logger: SILENT,
  };
}

/** Records every `/hc/connect` body and answers with a TERMINAL 401 (no retry). */
function captureConnects(): string[] {
  const bodies: string[] = [];
  (globalThis as Record<string, unknown>).fetch = (
    _url: string,
    init?: { body?: string }
  ): Promise<unknown> => {
    bodies.push(init?.body ?? '');
    return Promise.resolve({
      ok: false,
      status: 401,
      text: () => Promise.resolve('Invalid or expired token'),
    });
  };
  return bodies;
}

function installStorage(): Map<string, string> {
  const map = new Map<string, string>();
  (globalThis as Record<string, unknown>).localStorage = {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => void map.set(key, value),
    removeItem: (key: string) => void map.delete(key),
  };
  return map;
}

function signingKeyOf(body: string): string {
  return (JSON.parse(body) as { signingKey: string }).signingKey;
}

describe('DoorwayConnectionStrategy — device signing credential', () => {
  let realFetch: unknown;

  beforeEach(() => {
    realFetch = (globalThis as Record<string, unknown>).fetch;
    delete (globalThis as Record<string, unknown>).localStorage;
  });

  afterEach(() => {
    (globalThis as Record<string, unknown>).fetch = realFetch;
    delete (globalThis as Record<string, unknown>).localStorage;
  });

  /**
   * THE INVARIANT. A page reload is a NEW strategy instance over the SAME
   * browser storage, and it must present the SAME signing key — otherwise the
   * doorway grants a fresh capability on every reload, which is exactly the
   * 15 000 rows measured on matthew and adam.
   */
  it('presents the same signing key across a simulated reload', async () => {
    installStorage();
    const bodies = captureConnects();

    await new DoorwayConnectionStrategy().connect(config());
    await new DoorwayConnectionStrategy().connect(config());

    expect(bodies).toHaveLength(2);
    expect(signingKeyOf(bodies[1])).toBe(signingKeyOf(bodies[0]));
    // The cap secret is half of the Assigned grant's identity, so it must be
    // reused too — a rolled secret does not match the existing grant.
    expect(JSON.parse(bodies[1])).toEqual(JSON.parse(bodies[0]));
  });

  /** A second hosted human on the same browser gets their own witnessed grant. */
  it('presents a different key for a different agent', async () => {
    installStorage();
    const bodies = captureConnects();

    await new DoorwayConnectionStrategy().connect(config('uhCAkAgentOne'));
    await new DoorwayConnectionStrategy().connect(config('uhCAkAgentTwo'));

    expect(signingKeyOf(bodies[1])).not.toBe(signingKeyOf(bodies[0]));
  });

  /**
   * STORAGE UNAVAILABLE (private window, blocked storage, SSR) must degrade to
   * the previous behaviour — a fresh key per session — and never fail the
   * connect.
   */
  it('falls back to a per-session key when storage is unavailable', async () => {
    const bodies = captureConnects();

    const first = await new DoorwayConnectionStrategy().connect(config());
    const second = await new DoorwayConnectionStrategy().connect(config());

    // The connect still ran and failed honestly on the stubbed 401 — it did not
    // throw on the missing storage.
    expect(first.success).toBe(false);
    expect(second.success).toBe(false);
    expect(signingKeyOf(bodies[1])).not.toBe(signingKeyOf(bodies[0]));
  });

  /**
   * HEAL ONCE. When the conductor has lost the grant, the device discards its
   * stored credential and reconnects with a FRESH key — once. The second
   * request is refused so the caller surfaces the error instead of authoring
   * another grant. This bound is what keeps the heal from re-creating the
   * growth it exists to cure.
   */
  it('heals with a fresh key exactly once per session', async () => {
    const storage = installStorage();
    const bodies = captureConnects();
    const strategy = new DoorwayConnectionStrategy();

    await strategy.connect(config());
    expect(storage.has(CHAPERONE_CREDENTIALS_KEY)).toBe(true);

    const healed = await strategy.healSigningCredentials(config());
    expect(healed).not.toBeNull();
    expect(bodies).toHaveLength(2);
    expect(signingKeyOf(bodies[1])).not.toBe(signingKeyOf(bodies[0]));

    const again = await strategy.healSigningCredentials(config());
    expect(again).toBeNull();
    expect(bodies).toHaveLength(2);
  });

  /** SIGN-OUT drops the device credential; the next sign-in is granted afresh. */
  it('clearPersistedSigningCredentials forgets the device key', async () => {
    const storage = installStorage();
    const bodies = captureConnects();
    const strategy = new DoorwayConnectionStrategy();

    await strategy.connect(config());
    strategy.clearPersistedSigningCredentials();

    expect(storage.has(CHAPERONE_CREDENTIALS_KEY)).toBe(false);
    expect(strategy.getSigningCredentials()).toBeNull();

    await new DoorwayConnectionStrategy().connect(config());
    expect(signingKeyOf(bodies[1])).not.toBe(signingKeyOf(bodies[0]));
  });
});
