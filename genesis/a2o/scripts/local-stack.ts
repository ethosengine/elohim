#!/usr/bin/env tsx
/**
 * Local Stack Health Check & Auto-Seed
 *
 * Thin health/seed client over the canonical local-stack surface owned by
 * `app/elohim-app/scripts/hc-start.sh`. hc-start starts the stack (conductor +
 * storage + doorway), writes the conductor ports file
 * (`elohim/holochain/local-dev/.hc_ports`), and documents the endpoints this
 * script probes (doorway `/health` on :8888; storage `/health` + `/db/stats`
 * on :8090). This script never starts services — it checks health and, with
 * `--seed`, invokes the SAME seeder entrypoint hc-start's --seed step runs
 * (`genesis/seeder/src/seed.ts`), so the doorway-fronted stack has exactly
 * one seeding pipeline.
 *
 * Exception: E2E_MODE=steward (Tauri sidecar testing) seeds the SQLite
 * content DB directly via `seed-sqlite.ts` — there is no doorway (or
 * conductor) in that deployment path, so the doorway-routed seed.ts pipeline
 * cannot apply.
 *
 * Usage:
 *   npx tsx scripts/local-stack.ts                # check health
 *   npx tsx scripts/local-stack.ts --seed         # seed if empty
 *   npx tsx scripts/local-stack.ts --wait 60      # wait up to 60s for services
 *
 * Environment:
 *   E2E_MODE         'local' (default) or 'steward' (Tauri sidecar, no doorway)
 *   E2E_DOORWAY_URL  doorway base URL (default http://localhost:8888)
 *   E2E_STORAGE_URL  storage base URL (default http://localhost:8090)
 *   SEED_LIMIT       items to seed in local mode (default 200, same as hc-start)
 */

import { execSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { request } from 'undici';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../..');
const SEEDER_DIR = resolve(REPO_ROOT, 'genesis/seeder');

// The SAME ports file hc-start.sh writes (its HC_PORTS_FILE) — the only source
// for the conductor's dynamically-assigned admin port. Never hardcode it.
const HC_PORTS_FILE = resolve(REPO_ROOT, 'elohim/holochain/local-dev/.hc_ports');

// hc-start.sh seeds 200 items by default (its SEED_LIMIT); mirror it.
const DEFAULT_SEED_LIMIT = 200;

interface ServiceStatus {
  name: string;
  url: string;
  healthy: boolean;
  details?: string;
}

interface CliArgs {
  mode: string;
  seed: boolean;
  waitSeconds: number;
  doorwayUrl: string;
  storageUrl: string;
}

function parseArgs(): CliArgs {
  const args = process.argv.slice(2);
  const mode = process.env['E2E_MODE'] ?? 'local';

  const result: CliArgs = {
    mode,
    seed: false,
    waitSeconds: 10,
    doorwayUrl:
      mode === 'steward' ? '' : (process.env['E2E_DOORWAY_URL'] ?? 'http://localhost:8888'),
    storageUrl: process.env['E2E_STORAGE_URL'] ?? 'http://localhost:8090',
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--seed') result.seed = true;
    else if (args[i] === '--wait' && args[i + 1])
      result.waitSeconds = Number.parseInt(args[++i], 10);
    else if (args[i] === '--doorway' && args[i + 1]) result.doorwayUrl = args[++i];
    else if (args[i] === '--storage' && args[i + 1]) result.storageUrl = args[++i];
  }

  return result;
}

async function checkService(name: string, url: string, path: string): Promise<ServiceStatus> {
  try {
    const { statusCode, body } = await request(`${url}${path}`, {
      method: 'GET',
    });
    const text = await body.text();
    return {
      name,
      url,
      healthy: statusCode >= 200 && statusCode < 300,
      details: text.slice(0, 200),
    };
  } catch (err) {
    return {
      name,
      url,
      healthy: false,
      details: err instanceof Error ? err.message : String(err),
    };
  }
}

async function waitForService(
  name: string,
  url: string,
  path: string,
  timeoutMs: number
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  let delay = 1000;

  while (Date.now() < deadline) {
    const status = await checkService(name, url, path);
    if (status.healthy) {
      console.log(`  ${name}: healthy`);
      return true;
    }
    console.log(`  ${name}: waiting... (${status.details?.slice(0, 60)})`);
    await new Promise(r => setTimeout(r, Math.min(delay, deadline - Date.now())));
    delay = Math.min(delay * 1.5, 5000);
  }

  console.log(`  ${name}: timed out after ${timeoutMs / 1000}s`);
  return false;
}

async function checkStorageEmpty(storageUrl: string): Promise<boolean> {
  // Fail-open to "empty" so a fresh (or unreadable-stats) stack still seeds.
  try {
    const { statusCode, body } = await request(`${storageUrl}/db/stats`, {
      method: 'GET',
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) return true;
    const stats = JSON.parse(text);
    const count = stats.content_count ?? stats.contentCount ?? 0;
    return count === 0;
  } catch {
    return true;
  }
}

/** True if something accepts a TCP connection on 127.0.0.1:port. */
async function portAccepts(port: number, timeoutMs = 1000): Promise<boolean> {
  return new Promise(resolvePromise => {
    const socket = createConnection({ port, host: '127.0.0.1' });
    const done = (ok: boolean) => {
      socket.destroy();
      resolvePromise(ok);
    };
    socket.setTimeout(timeoutMs);
    socket.once('connect', () => done(true));
    socket.once('timeout', () => done(false));
    socket.once('error', () => done(false));
  });
}

/**
 * Conductor admin port from hc-start's ports file. Mirrors hc-start.sh's
 * get_admin_port staleness guard: the file survives conductor crashes, so the
 * recorded port is only trusted if something actually accepts TCP there.
 */
async function readConductorAdminPort(): Promise<number | undefined> {
  if (!existsSync(HC_PORTS_FILE)) return undefined;
  const match = /admin_port=(\d+)/.exec(readFileSync(HC_PORTS_FILE, 'utf8'));
  if (!match) return undefined;
  const port = Number.parseInt(match[1], 10);
  return (await portAccepts(port)) ? port : undefined;
}

async function runSeed(args: CliArgs): Promise<void> {
  console.log('\nSeeding local storage...');
  try {
    if (args.mode === 'steward') {
      // KEEP: steward (Tauri sidecar) testing seeds the SQLite content DB
      // directly — no doorway or conductor exists in that deployment path, so
      // the canonical doorway-routed seed.ts pipeline cannot be used here.
      // eslint-disable-next-line sonarjs/no-os-command-from-path -- npx is the canonical seeder invocation (same pattern as steps/seeder.steps.ts)
      execSync('npx tsx src/seed-sqlite.ts', {
        cwd: SEEDER_DIR,
        stdio: 'inherit',
        timeout: 60_000, // seed-sqlite is the "<1 minute" fast path by design
        env: { ...process.env, STORAGE_URL: args.storageUrl },
      });
    } else {
      // Canonical path: the SAME invocation hc-start.sh --seed runs (its
      // "Step 5: Optional seeding") — one seeder pipeline, not a parallel one.
      const adminPort = await readConductorAdminPort();
      if (adminPort === undefined) {
        console.log(
          `  No live conductor admin port in ${HC_PORTS_FILE} — ` +
            'seed.ts will skip conductor pre/post-flight verification'
        );
      }
      const rawLimit = Number.parseInt(process.env['SEED_LIMIT'] ?? '', 10);
      const seedLimit = Number.isNaN(rawLimit) ? DEFAULT_SEED_LIMIT : rawLimit;
      // eslint-disable-next-line sonarjs/os-command -- seedLimit is a sanitized integer; mirrors hc-start.sh's `npx tsx src/seed.ts --limit $SEED_LIMIT`
      execSync(`npx tsx src/seed.ts --limit ${seedLimit}`, {
        cwd: SEEDER_DIR,
        stdio: 'inherit',
        // Doorway-routed seeding (blob sync + verification) is slower than the
        // sqlite fast path; allow up to 10 minutes.
        timeout: 600_000,
        env: {
          ...process.env,
          DOORWAY_URL: args.doorwayUrl,
          // Direct storage URL for blob sync — seed.ts never derives it from
          // DOORWAY_URL (doorway proxies /db/* itself).
          STORAGE_URL: args.storageUrl,
          ...(adminPort === undefined
            ? {}
            : { HOLOCHAIN_ADMIN_URL: `ws://localhost:${adminPort}` }),
        },
      });
    }
    console.log('  Seeding complete');
  } catch (err) {
    console.error('  Seeding failed:', err instanceof Error ? err.message : err);
  }
}

async function main() {
  const args = parseArgs();
  const { seed, waitSeconds, doorwayUrl, storageUrl } = args;
  const timeoutMs = waitSeconds * 1000;

  console.log('Local Stack Health Check');
  console.log(`  Doorway: ${doorwayUrl || '(steward mode — skipped)'}`);
  console.log(`  Storage: ${storageUrl}`);
  console.log('');

  const services: ServiceStatus[] = [];

  // Endpoints below are hc-start.sh's documented surface (it waits on the same
  // /health probes at startup); /db/stats is only used for the emptiness check.

  // Check storage first (both modes need it)
  console.log('Checking storage...');
  const storageOk = await waitForService('elohim-storage', storageUrl, '/health', timeoutMs);
  services.push({ name: 'elohim-storage', url: storageUrl, healthy: storageOk });

  // Check doorway (local mode only)
  if (doorwayUrl) {
    console.log('Checking doorway...');
    const doorwayOk = await waitForService('doorway', doorwayUrl, '/health', timeoutMs);
    services.push({ name: 'doorway', url: doorwayUrl, healthy: doorwayOk });
  }

  // Summary
  console.log('\n--- Status ---');
  let allHealthy = true;
  for (const s of services) {
    const icon = s.healthy ? 'OK' : 'FAIL';
    console.log(`  [${icon}] ${s.name} (${s.url})`);
    if (!s.healthy) allHealthy = false;
  }

  if (!allHealthy) {
    console.error(
      '\nSome services are not healthy. Start them with: pnpm run hc:start' +
        ' (app/elohim-app — scripts/hc-start.sh owns the stack surface)'
    );
    process.exit(1);
  }

  // Auto-seed if requested and storage is empty
  if (seed && storageOk) {
    const empty = await checkStorageEmpty(storageUrl);
    if (empty) {
      console.log('\nStorage is empty, auto-seeding...');
      await runSeed(args);
    } else {
      console.log('\nStorage already has content, skipping seed');
    }
  }

  console.log('\nReady for E2E testing');
}

main().catch(err => {
  console.error('Local stack check failed:', err);
  process.exit(1);
});
