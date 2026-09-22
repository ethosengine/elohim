/**
 * A doorway's own node identity across a restart of that doorway
 * (@concern:doorway-failover, @act:i, @requires:owned-substrate).
 *
 * Three surfaces, joined:
 *   - the BOOT LINE the doorway writes (`Doorway node identity resolved
 *     (persisted)`, with `path`, `fingerprint` and `mode` in
 *     doorway/doorway-service/src/main.rs, minted by node_identity.rs),
 *   - the KEY SET at `/.well-known/doorway-keys` (routes/federation.rs), whose
 *     `kid` is the doorway's own configured id,
 *   - the DID DOCUMENT at `/.well-known/did.json` (routes/identity.rs), which
 *     carries no key material today and so witnesses the NAME only.
 *
 * Two facts about this household make the reads below what they are:
 *
 * 1. `hc-mesh.sh restart_doorway` re-execs the doorway with the environment
 *    CAPTURED from `/proc/<pid>/environ`, and redirects the new process's
 *    output to `logs/doorway-restart-<name>.log` — NOT the original
 *    `logs/doorway.log`. So the post-restart boot line is read from the restart
 *    log, at an offset taken before the restart (the file is append-only across
 *    restarts, so a bare `includes` would happily match a previous run's line).
 * 2. That same captured environment is where `DOORWAY_NODE_KEY_FILE` and
 *    `DOORWAY_ID` are read from here. The doorway states neither on any HTTP
 *    surface, and re-deriving them from `hc-mesh.sh`'s conventions would make a
 *    second hand-maintained home for a value the running process already holds.
 *
 * Nothing here induces a fault beyond the restart itself. The `After` hook
 * asserts every doorway this file restarted is answering again, so a failed
 * assertion mid-scenario cannot leave the household short a doorway.
 *
 * THE DEFERRED TOKEN LEG, in the detail the story deliberately keeps out of
 * itself: a token minted before the restart and verified at the sibling after
 * it would be the sharpest form of scenario 2, and it is not asserted because
 * on this household it would measure the wrong thing. Minting algorithm is
 * `DOORWAY_JWT_SIGN_ALG` (doorway/doorway-service/src/auth/jwt.rs), default
 * `hs256`; `hc-mesh.sh` sets no value for either doorway and no `JWT_SECRET`
 * either, so both fall back to the same dev placeholder secret and a token
 * crossing the pair proves that shared secret survived. Only
 * `DOORWAY_JWT_SIGN_ALG=eddsa` makes the node key the signer, at which point the
 * sibling's `PeerJwksCache` (services/federation.rs) fetches
 * `/.well-known/doorway-keys` on first sight of a foreign `kid` and REFUSES to
 * replace a live anchor with a different key — which is exactly the harm a
 * rotated identity causes, and exactly what the key-set assertions below stand
 * in for until a lane boots the pair with EdDSA minting.
 */

import { strict as assert } from 'node:assert';
import { readFile, stat } from 'node:fs/promises';
import { join } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

import { resolvePeerUrl } from '../../src/framework/dataplane/surfaces.js';
import {
  householdMeshDir,
  loadHouseholdMeshFixture,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

import { doorwayRestartLog, meshControl, pollUntil } from './epr-app-deliverability.helpers.js';

/** Raw Ed25519 secret-key length — what a key file must be, exactly. */
const KEY_FILE_BYTES = 32;

/** A doorway must answer again within this long after its restart. */
const BOOT_BOUND_MS = 120_000;

/** Its boot line must appear in the restart log within this long after it answers. */
const BOOT_LINE_BOUND_MS = 60_000;

/** Fixture doorway names -> the `a|b|c` arm `hc-mesh.sh doorway-restart` takes. */
const MESH_ARMS: Record<string, string> = {
  'alpha-A': 'a',
  'elohim.host': 'b',
};

/** What a doorway's environment says about the identity it is meant to keep. */
interface DeclaredIdentity {
  /** The `a|b|c` arm that restarts this doorway. */
  arm: string;
  /** `DOORWAY_NODE_KEY_FILE` — the file it keeps its identity in. */
  keyFile: string;
  /** `DOORWAY_ID` — the name its published key is filed under. */
  configuredName: string;
}

/** What a doorway is publishing as its identity, right now. */
interface PublishedIdentity {
  /** The key-set document verbatim, so "byte for byte" means it. */
  keySetBytes: string;
  /** The single entry's key id. */
  keyId: string;
  /** Its base64url public key. */
  publicKey: string;
  /** The first eight bytes of that key in hex — the doorway's own spelling. */
  fingerprint: string;
  /** The `id` of the DID document — the name the doorway answers to. */
  didId: string;
}

/**
 * What the doorway's boot line claimed about the identity it resolved.
 *
 * `mode` carries the doorway's own vocabulary verbatim: `loaded` (the file was
 * there), `generated` (no file, so a new key was minted and written), or
 * `generated-ephemeral` (no file was configured at all, so the key is a
 * throwaway). `keyFile` is empty for the ephemeral case, which names no path.
 */
interface BootClaim {
  keyFile: string;
  fingerprint: string;
  mode: string;
}

/** The mode a doorway reports when it was told no key file to keep. */
const EPHEMERAL_MODE = 'generated-ephemeral';

interface IdentityState {
  declared: Map<string, DeclaredIdentity>;
  recorded: Map<string, PublishedIdentity>;
  boot: Map<string, BootClaim>;
  restarted: Set<string>;
}

const states = new WeakMap<E2EWorld, IdentityState>();

function state(world: E2EWorld): IdentityState {
  let current = states.get(world);
  if (!current) {
    current = { declared: new Map(), recorded: new Map(), boot: new Map(), restarted: new Set() };
    states.set(world, current);
  }
  return current;
}

function meshArm(doorway: string): string {
  const arm = MESH_ARMS[doorway];
  assert.ok(
    arm,
    `no household restart arm is known for doorway "${doorway}" ` +
      `(this file knows ${Object.keys(MESH_ARMS).join(', ')})`
  );
  return arm;
}

/** The doorway's live process, or the substrate fact that there isn't one. */
async function ownedDoorwayPid(doorway: string, arm: string): Promise<number> {
  const fixture = loadHouseholdMeshFixture();
  assert.equal(
    fixture.processControl,
    true,
    `this run does not own the doorway processes (${fixture.processControlReason ?? 'processControl is not declared'}): ` +
      'a doorway identity cannot be read from its environment, nor restarted, where every ' +
      'doorway is a remote pod. Run this on a household you own (`just mesh start`).'
  );
  const pidFile = join(householdMeshDir(), 'pids', `doorway-${arm}`);
  const recorded = await readFile(pidFile, 'utf8').catch(() => '');
  const pid = Number.parseInt(recorded.trim().split(/\s+/)[0] ?? '', 10);
  assert.ok(
    Number.isInteger(pid) && pid > 0,
    `no owned doorway "${doorway}": ${pidFile} names no process identifier`
  );
  const alive = await stat(`/proc/${pid}`).then(
    () => true,
    () => false
  );
  assert.ok(alive, `owned doorway "${doorway}" (pid ${pid}) is not running`);
  return pid;
}

/** The environment the doorway is actually running with — its own, not a re-derivation. */
async function doorwayEnvironment(pid: number): Promise<Map<string, string>> {
  const raw = await readFile(`/proc/${pid}/environ`, 'utf8');
  const environment = new Map<string, string>();
  for (const item of raw.split('\0')) {
    const split = item.indexOf('=');
    if (split > 0) environment.set(item.slice(0, split), item.slice(split + 1));
  }
  return environment;
}

/** The doorway's own spelling of a public key: its first eight bytes, in hex. */
function fingerprintOf(base64UrlKey: string): string {
  return Buffer.from(base64UrlKey, 'base64url').subarray(0, 8).toString('hex');
}

async function fetchText(
  url: string,
  timeoutMs = 15_000
): Promise<{ status: number; body: string }> {
  const response = await fetch(url, { signal: AbortSignal.timeout(timeoutMs) });
  return { status: response.status, body: await response.text() };
}

async function readPublishedIdentity(doorway: string): Promise<PublishedIdentity> {
  const base = resolvePeerUrl(doorway);

  const keySet = await fetchText(`${base}/.well-known/doorway-keys`);
  assert.equal(
    keySet.status,
    200,
    `doorway "${doorway}" did not publish a key set: ${keySet.status} ${keySet.body.slice(0, 200)}`
  );
  const parsed = JSON.parse(keySet.body) as {
    keys?: { kty?: string; crv?: string; use?: string; kid?: string; x?: string }[];
  };
  const keys = parsed.keys ?? [];
  assert.equal(
    keys.length,
    1,
    `doorway "${doorway}" published ${keys.length} signing keys; a doorway has one identity, ` +
      'and a reader with a choice cannot tell which one signed'
  );
  const [key] = keys;
  assert.equal(key.kty, 'OKP', `doorway "${doorway}" key type must be OKP, got ${key.kty}`);
  assert.equal(key.crv, 'Ed25519', `doorway "${doorway}" curve must be Ed25519, got ${key.crv}`);
  assert.equal(key.use, 'sig', `doorway "${doorway}" key use must be sig, got ${key.use}`);
  assert.ok(
    key.kid,
    `doorway "${doorway}" published a key with no key id for a reader to file it under`
  );
  assert.ok(key.x, `doorway "${doorway}" published a key entry with no public key in it`);

  const did = await fetchText(`${base}/.well-known/did.json`);
  assert.equal(
    did.status,
    200,
    `doorway "${doorway}" did not publish a DID document: ${did.status} ${did.body.slice(0, 200)}`
  );
  const didId = (JSON.parse(did.body) as { id?: string }).id;
  assert.ok(didId, `doorway "${doorway}" published a DID document that names no identifier`);

  return {
    keySetBytes: keySet.body,
    keyId: key.kid,
    publicKey: key.x,
    fingerprint: fingerprintOf(key.x),
    didId,
  };
}

function recorded(world: E2EWorld, doorway: string): PublishedIdentity {
  const before = state(world).recorded.get(doorway);
  assert.ok(before, `nothing was recorded for doorway "${doorway}" before this step`);
  return before;
}

function declared(world: E2EWorld, doorway: string): DeclaredIdentity {
  const value = state(world).declared.get(doorway);
  assert.ok(
    value,
    `doorway "${doorway}" was never established as keeping its identity in a file of its own`
  );
  return value;
}

/**
 * The LAST identity line the doorway wrote after `offset`.
 *
 * Both spellings of the line are read — `(persisted)` and `(ephemeral)` — so an
 * ephemeral boot is reported as itself rather than as an absence: a doorway that
 * minted a throwaway key has answered the scenario's question, in the negative.
 */
function bootClaimAfter(arm: string, offset: number): BootClaim | undefined {
  let claim: BootClaim | undefined;
  for (const line of doorwayRestartLog(arm).slice(offset).split('\n')) {
    if (!line.includes('Doorway node identity resolved')) continue;
    let fields: { message?: string; path?: string; fingerprint?: string; mode?: string };
    try {
      fields = (JSON.parse(line) as { fields?: typeof fields }).fields ?? {};
    } catch {
      continue;
    }
    if (!fields.message?.startsWith('Doorway node identity resolved')) continue;
    claim = {
      keyFile: fields.path ?? '',
      fingerprint: fields.fingerprint ?? '',
      mode: fields.mode ?? '',
    };
  }
  return claim;
}

Given(
  'doorway {string} keeps its identity in a file of its own',
  { timeout: 30_000 },
  async function (this: E2EWorld, doorway: string) {
    const arm = meshArm(doorway);
    const environment = await doorwayEnvironment(await ownedDoorwayPid(doorway, arm));
    const keyFile = environment.get('DOORWAY_NODE_KEY_FILE');
    assert.ok(
      keyFile,
      `doorway "${doorway}" is running with an EPHEMERAL identity: DOORWAY_NODE_KEY_FILE is unset, ` +
        'so it mints a throwaway signing key every boot and there is no identity for a restart to ' +
        'keep. This is a substrate declaration, not a defect in the doorway.'
    );
    const keyFileStat = await stat(keyFile).catch(() => undefined);
    const keyFileBytes = keyFileStat?.isFile() ? keyFileStat.size : undefined;
    assert.ok(
      keyFileBytes !== undefined,
      `doorway "${doorway}" names ${keyFile} as the file it keeps its identity in, but no such file exists`
    );
    assert.equal(
      keyFileBytes,
      KEY_FILE_BYTES,
      `doorway "${doorway}"'s identity file ${keyFile} holds ${keyFileBytes} bytes, not ${KEY_FILE_BYTES}: ` +
        'the doorway refuses to boot from a wrong-length key rather than mint a silently different one'
    );
    const configuredName = environment.get('DOORWAY_ID');
    assert.ok(
      configuredName,
      `doorway "${doorway}" is running with no configured name (DOORWAY_ID unset), so it would publish ` +
        'its key under a shared placeholder id that a sibling cannot tell two doorways apart by'
    );
    state(this).declared.set(doorway, { arm, keyFile, configuredName });
  }
);

Given(
  'I record what doorway {string} publishes as its identity',
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string) {
    state(this).recorded.set(doorway, await readPublishedIdentity(doorway));
  }
);

When(
  'the household restarts doorway {string}',
  { timeout: 240_000 },
  async function (this: E2EWorld, doorway: string) {
    const arm = meshArm(doorway);
    const base = resolvePeerUrl(doorway);
    const offset = doorwayRestartLog(arm).length;

    await meshControl('doorway-restart', arm);
    state(this).restarted.add(doorway);

    const answered = await pollUntil(
      async () => (await fetchText(`${base}/health`, 3_000).catch(() => undefined))?.status === 200,
      BOOT_BOUND_MS,
      2_000
    );
    assert.notEqual(
      answered,
      null,
      `doorway "${doorway}" did not answer /health within ${BOOT_BOUND_MS / 1000}s of its restart`
    );

    const appeared = await pollUntil(
      async () => await Promise.resolve(bootClaimAfter(arm, offset) !== undefined),
      BOOT_LINE_BOUND_MS,
      1_000
    );
    assert.notEqual(
      appeared,
      null,
      `doorway "${doorway}" answered after its restart but wrote no node-identity line to ` +
        `logs/doorway-restart-${arm}.log within ${BOOT_LINE_BOUND_MS / 1000}s — nothing states which ` +
        'identity it came back with'
    );
    const claim = bootClaimAfter(arm, offset);
    assert.ok(claim, `doorway "${doorway}" wrote no readable node-identity line after its restart`);
    assert.notEqual(
      claim.mode,
      EPHEMERAL_MODE,
      `doorway "${doorway}" came back with an EPHEMERAL identity: it was told no key file, so the key ` +
        'it signs with now is a fresh one nobody has ever trusted'
    );
    state(this).boot.set(doorway, claim);
  }
);

Then(
  'doorway {string} says at boot that it loaded the identity it already had',
  function (this: E2EWorld, doorway: string) {
    const claim = state(this).boot.get(doorway);
    assert.ok(claim, `doorway "${doorway}" has not been restarted in this scenario`);
    assert.equal(
      claim.mode,
      'loaded',
      `doorway "${doorway}" reported mode "${claim.mode}" at boot: "generated" means it minted a new ` +
        'identity rather than keeping the one every sibling and session already trusts'
    );
    assert.equal(
      claim.keyFile,
      declared(this, doorway).keyFile,
      `doorway "${doorway}" resolved its identity from a different file than the one it was told`
    );
  }
);

Then(
  'the fingerprint doorway {string} named at boot is the key it publishes',
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string) {
    const claim = state(this).boot.get(doorway);
    assert.ok(claim, `doorway "${doorway}" has not been restarted in this scenario`);
    const published = await readPublishedIdentity(doorway);
    assert.equal(
      published.fingerprint,
      claim.fingerprint,
      `doorway "${doorway}" named ${claim.fingerprint} at boot but publishes ${published.fingerprint}: ` +
        'a log line nobody can check against the wire is not evidence of an identity'
    );
  }
);

Then(
  'doorway {string} publishes the same key set, byte for byte, under the same key id',
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string) {
    const before = recorded(this, doorway);
    const now = await readPublishedIdentity(doorway);
    assert.equal(
      now.keyId,
      before.keyId,
      `doorway "${doorway}" changed the key id it publishes under (${before.keyId} -> ${now.keyId}): ` +
        "a sibling files this doorway's key by that id, so the old entry now names nothing"
    );
    assert.equal(
      now.publicKey,
      before.publicKey,
      `doorway "${doorway}" publishes a different signing key than before ` +
        `(fingerprint ${before.fingerprint} -> ${now.fingerprint}): everything it signed before is ` +
        'now uncheckable, and a sibling holding the old key will refuse the new one as a takeover'
    );
    assert.equal(
      now.keySetBytes,
      before.keySetBytes,
      `doorway "${doorway}" publishes a key set that differs from the one a reader took before`
    );
  }
);

Then(
  'doorway {string} names itself the same in its DID document',
  { timeout: 60_000 },
  async function (this: E2EWorld, doorway: string) {
    const before = recorded(this, doorway);
    const now = await readPublishedIdentity(doorway);
    assert.equal(
      now.didId,
      before.didId,
      `doorway "${doorway}" renamed itself (${before.didId} -> ${now.didId}): a sibling that kept the ` +
        'key would still be looking for a doorway that no longer answers to that name'
    );
  }
);

Then(
  'doorways {string} and {string} publish different keys under different key ids',
  function (this: E2EWorld, first: string, second: string) {
    const one = recorded(this, first);
    const other = recorded(this, second);
    assert.notEqual(
      one.publicKey,
      other.publicKey,
      `doorways "${first}" and "${second}" publish the SAME signing key (${one.fingerprint}): they are ` +
        'one party wearing two addresses, and "the key did not change" would then be a statement ' +
        'about a constant rather than about either doorway'
    );
    assert.notEqual(
      one.keyId,
      other.keyId,
      `doorways "${first}" and "${second}" publish under the same key id (${one.keyId}): a sibling's ` +
        'cache is keyed by it, so one of the two would be refused as an impostor of the other'
    );
  }
);

Then(
  'each of doorways {string} and {string} publishes its key under its own configured name',
  function (this: E2EWorld, first: string, second: string) {
    for (const doorway of [first, second]) {
      assert.equal(
        recorded(this, doorway).keyId,
        declared(this, doorway).configuredName,
        `doorway "${doorway}" publishes its key under an id that is not its own configured name: a ` +
          'doorway that falls back to the shared placeholder id makes every foreign-issued token ' +
          'unverifiable, because the reader looks the key up by the issuer it expected'
      );
    }
  }
);

/**
 * Restoration has priority over reporting: a restarted doorway left unanswering
 * breaks every scenario after this one, so the check that it came back runs even
 * when an assertion above already failed.
 */
After({ tags: '@concern:doorway-failover', timeout: 180_000 }, async function (this: E2EWorld) {
  const current = states.get(this);
  if (!current || current.restarted.size === 0) return;
  for (const doorway of current.restarted) {
    const base = resolvePeerUrl(doorway);
    const answered = await pollUntil(
      async () => (await fetchText(`${base}/health`, 3_000).catch(() => undefined))?.status === 200,
      BOOT_BOUND_MS,
      2_000
    );
    assert.notEqual(
      answered,
      null,
      `restart teardown: doorway "${doorway}" is not answering /health again`
    );
  }
  this.attach(
    JSON.stringify({ restarted: [...current.restarted], restored: true }),
    'application/json'
  );
});
