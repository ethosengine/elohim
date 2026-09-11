/**
 * Seed Hosted Humans — Prologue casts N=3 hosted registrants through the real
 * portal (D4, doorway-federation-three-reds-to-green-plan S1 Task 5).
 *
 * D4, verbatim: "fixture hosted humans come from the Prologue, through the
 * real portal (`POST /auth/register`). They are real registrants with real
 * cells and real commitments — never `humans` rows claiming presence."
 *
 * This is deliberately the SAME call a stranger's browser makes — no admin
 * route, no direct DB insert. All three register at doorway A so the
 * `doorway-humans-served` story has a sibling doorway (B) hosting none.
 *
 *   identifier          | doorway        | intended pool
 *   --------------------|----------------|------------------------
 *   prologue-hosted-1   | DOORWAY_A_URL  | adam's pool conductor
 *   prologue-hosted-2   | DOORWAY_A_URL  | matthew's pool conductor
 *   prologue-hosted-3   | DOORWAY_A_URL  | matthew's pool conductor
 *
 * "intended pool" is recorded in the roster as documentation of what pool
 * selection SHOULD eventually route each registrant onto once provisioning
 * keys on declared NetworkStage (S2 Task 7) instead of `dev_mode` — this
 * seeder has no request field to force it (`AgentProvisioner.provision_agent`
 * takes only the identifier). Until Task 7 lands, every deployed doorway
 * runs `dev_mode=true`, so `POST /auth/register`'s "hosted" arm never reaches
 * the conductor-registry provisioning branch at all: it falls back to the
 * singleton `call_create_human` / recovery path, and all three registrants
 * land on the SAME agent key. That is the measured red this seeder is
 * expected to reproduce and print plainly — not "fix" here.
 *
 * `installedAppId` (AuthResponse's only conductor-identifying field today)
 * and `hostedCellGrantCid` (not minted until S2 Task 13) are both typically
 * absent on the current tree; this seeder prints `-` for either when the
 * response omits it, per the task's contract.
 *
 * Idempotent: a re-run against an existing identifier reports `exists`
 * (verified via `POST /auth/login`, mirroring seed-humans.ts's
 * verifyExisting) and does not fail.
 *
 * Soft, not hard: this leg is wired into hc-mesh-prologue.sh as `soft` — a
 * doorway with no pool conductor (or unreachable altogether) must not fail
 * the whole Prologue. Every registrant that cannot be seeded is reported on
 * its own line and the roster is still written (with per-entry results) so
 * downstream steps see the actual state rather than guessing.
 *
 * Environment variables:
 *   DOORWAY_URL   Doorway URL to register through (default: http://localhost:8888)
 *   MESH_DIR      Mesh data root the roster is written under
 *                 (default: /tmp/elohim-local-mesh)
 *
 * Exit codes (mirrors the Jenkinsfile runProbedSeeder contract used by the
 * other Act I Prologue seeders):
 *   0 — all three registrants registered or already exist
 *   2 — partial: at least one registrant hit a soft failure (no pool,
 *       doorway unreachable, or an unexpected error) but not all three
 *   1 — total failure: the doorway could not be reached for any registrant
 */

import { mkdirSync, writeFileSync } from 'node:fs';

// =============================================================================
// Cast — verbatim identifiers/pools from the plan's Task 5 table
// =============================================================================

interface Registrant {
  identifier: string;
  displayName: string;
  intendedPool: string;
}

const REGISTRANTS: Registrant[] = [
  { identifier: 'prologue-hosted-1', displayName: 'Prologue Hosted One', intendedPool: "adam's pool conductor" },
  { identifier: 'prologue-hosted-2', displayName: 'Prologue Hosted Two', intendedPool: "matthew's pool conductor" },
  { identifier: 'prologue-hosted-3', displayName: 'Prologue Hosted Three', intendedPool: "matthew's pool conductor" },
];

// eslint-disable-next-line sonarjs/no-hardcoded-passwords -- test fixture credential, not production
const DEFAULT_PASSWORD = 'Prologue2026!';

// =============================================================================
// Registration
// =============================================================================

type Outcome = 'registered' | 'exists' | 'no-pool' | 'unreachable' | 'failed';

interface RosterEntry {
  identifier: string;
  displayName: string;
  doorwayUrl: string;
  intendedPool: string;
  result: Outcome;
  agentPubKey: string;
  conductorId: string;
  hostedCellGrantCid: string;
  error?: string;
}

interface AuthResponseShape {
  agentPubKey?: string;
  installedAppId?: string;
  hostedCellGrantCid?: string;
}

async function registerHosted(doorwayUrl: string, registrant: Registrant): Promise<RosterEntry> {
  const base: Omit<RosterEntry, 'result' | 'agentPubKey' | 'conductorId' | 'hostedCellGrantCid' | 'error'> = {
    identifier: registrant.identifier,
    displayName: registrant.displayName,
    doorwayUrl,
    intendedPool: registrant.intendedPool,
  };

  try {
    const res = await fetch(`${doorwayUrl}/auth/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        identifier: registrant.identifier,
        password: DEFAULT_PASSWORD,
        displayName: registrant.displayName,
        agencyPhase: 'hosted',
      }),
    });

    if (res.ok) {
      const body = (await res.json()) as AuthResponseShape;
      return {
        ...base,
        result: 'registered',
        agentPubKey: body.agentPubKey ?? '-',
        conductorId: body.installedAppId ?? '-',
        hostedCellGrantCid: body.hostedCellGrantCid ?? '-',
      };
    }

    // 409 — already registered by an earlier Prologue pass. Verify credentials
    // match via login, exactly seed-humans.ts's verifyExisting pattern.
    if (res.status === 409) {
      return await verifyExisting(doorwayUrl, registrant, base);
    }

    const errorText = await res.text();

    // 503 with no pool conductor available: soft, not hard — print a clear
    // line and move on rather than treating the whole Prologue as broken.
    if (res.status === 503 && errorText.includes('No conductors available')) {
      console.warn(
        `  [!] ${registrant.identifier}: doorway ${doorwayUrl} has no pool conductor available (503) — hosted registration deferred until a pool is configured. Continuing (soft leg).`
      );
      return { ...base, result: 'no-pool', agentPubKey: '-', conductorId: '-', hostedCellGrantCid: '-', error: errorText };
    }

    // 503 "Agent already has a Human profile" — the DHT already knows this
    // identity (conductor wasn't reset) but doorway's DB doesn't. Same
    // recovery seed-humans.ts uses: verify via login.
    if (res.status === 503 && errorText.includes('Agent already has a Human profile')) {
      return await verifyExisting(doorwayUrl, registrant, base);
    }

    return {
      ...base,
      result: 'failed',
      agentPubKey: '-',
      conductorId: '-',
      hostedCellGrantCid: '-',
      error: `HTTP ${res.status}: ${errorText}`,
    };
  } catch (err) {
    // Doorway unreachable (connection refused, DNS failure, closed port,
    // DOORWAY_URL unset and defaulting nowhere reachable, etc.) — caught here
    // so a network failure prints one clear line instead of an unhandled
    // rejection stack trace. Soft: report and continue to the next registrant.
    const message = err instanceof Error ? err.message : String(err);
    console.error(
      `  [x] ${registrant.identifier}: doorway ${doorwayUrl} unreachable (${message}) — soft failure, continuing.`
    );
    return { ...base, result: 'unreachable', agentPubKey: '-', conductorId: '-', hostedCellGrantCid: '-', error: message };
  }
}

async function verifyExisting(
  doorwayUrl: string,
  registrant: Registrant,
  base: Omit<RosterEntry, 'result' | 'agentPubKey' | 'conductorId' | 'hostedCellGrantCid' | 'error'>
): Promise<RosterEntry> {
  try {
    const res = await fetch(`${doorwayUrl}/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ identifier: registrant.identifier, password: DEFAULT_PASSWORD }),
    });

    if (res.ok) {
      const body = (await res.json()) as AuthResponseShape;
      return {
        ...base,
        result: 'exists',
        agentPubKey: body.agentPubKey ?? '-',
        conductorId: body.installedAppId ?? '-',
        hostedCellGrantCid: body.hostedCellGrantCid ?? '-',
      };
    }

    return {
      ...base,
      result: 'failed',
      agentPubKey: '-',
      conductorId: '-',
      hostedCellGrantCid: '-',
      error: `exists but login verification failed (HTTP ${res.status})`,
    };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error(
      `  [x] ${registrant.identifier}: doorway ${doorwayUrl} unreachable during exists-verification (${message}) — soft failure, continuing.`
    );
    return { ...base, result: 'unreachable', agentPubKey: '-', conductorId: '-', hostedCellGrantCid: '-', error: message };
  }
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const doorwayUrl = (process.env.DOORWAY_URL || 'http://localhost:8888').replace(/\/+$/, '');
  const meshDir = process.env.MESH_DIR || '/tmp/elohim-local-mesh';
  const rosterPath = `${meshDir}/prologue-hosted-humans.json`;

  console.log('=== Seed Hosted Humans (Act I Prologue, D4) ===\n');
  console.log(`Doorway:  ${doorwayUrl}`);
  console.log(`Roster:   ${rosterPath}`);
  console.log('');

  const entries: RosterEntry[] = [];
  for (const registrant of REGISTRANTS) {
    const entry = await registerHosted(doorwayUrl, registrant);
    entries.push(entry);
    const icon =
      entry.result === 'registered' || entry.result === 'exists'
        ? entry.result === 'registered'
          ? '+'
          : '='
        : entry.result === 'no-pool'
          ? '!'
          : 'X';
    console.log(
      `  [${icon}] ${entry.identifier.padEnd(20)} agentPubKey=${entry.agentPubKey.padEnd(24)} conductorId=${entry.conductorId.padEnd(10)} hostedCellGrantCid=${entry.hostedCellGrantCid} (${entry.result})`
    );
  }

  // Distinct-agent-key check — the first live proof of S2 Task 7. Before
  // that task lands, expect all three registrations sharing one agent key
  // (the dev_mode singleton recovery) — this is the measured red, not a bug
  // in this seeder.
  const liveKeys = entries
    .filter(e => (e.result === 'registered' || e.result === 'exists') && e.agentPubKey !== '-')
    .map(e => e.agentPubKey);
  const distinctKeys = new Set(liveKeys).size;
  const distinct = liveKeys.length > 0 && distinctKeys === liveKeys.length;

  console.log('');
  if (liveKeys.length > 0) {
    console.log(
      `Distinct agent keys: ${distinct ? 'yes' : 'no'} (${distinctKeys} distinct across ${liveKeys.length} live registrant(s))`
    );
    if (!distinct) {
      console.log(
        '  ^ expected before S2 Task 7 lands: provisioning still keys on dev_mode, so hosted registration falls back to the singleton agent key.'
      );
    }
  } else {
    console.log('Distinct agent keys: n/a (no registrant reached a live agentPubKey)');
  }

  mkdirSync(meshDir, { recursive: true });
  writeFileSync(rosterPath, JSON.stringify({ doorwayUrl, generatedAt: new Date().toISOString(), registrants: entries }, null, 2));
  console.log(`\nRoster written: ${rosterPath}`);

  const succeeded = entries.filter(e => e.result === 'registered' || e.result === 'exists').length;
  const softFailed = entries.filter(e => e.result === 'no-pool' || e.result === 'unreachable').length;
  const hardFailed = entries.filter(e => e.result === 'failed').length;

  console.log(
    `\n=== Results: ${succeeded} live, ${softFailed} soft-failed, ${hardFailed} failed (of ${entries.length}) ===`
  );

  if (succeeded === 0 && softFailed === entries.length) {
    console.error(
      'SOFT-FAIL: no registrant reached a live doorway response — roster written with soft-failure entries only. Continuing (soft leg); this must not fail the whole Prologue.'
    );
    process.exit(1);
  }

  if (softFailed > 0 || hardFailed > 0) {
    process.exit(2);
  }

  process.exit(0);
}

main();
