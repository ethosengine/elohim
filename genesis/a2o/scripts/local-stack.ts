#!/usr/bin/env tsx
/**
 * Local Stack Health Check & Auto-Seed
 *
 * Checks health of local doorway (:8888) and storage (:8090).
 * Can trigger local seed from genesis data if storage is empty.
 *
 * Usage:
 *   npx tsx scripts/local-stack.ts                # check health
 *   npx tsx scripts/local-stack.ts --seed         # seed if empty
 *   npx tsx scripts/local-stack.ts --wait 60      # wait up to 60s for services
 */

import { execSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { request } from 'undici';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../..');

interface ServiceStatus {
  name: string;
  url: string;
  healthy: boolean;
  details?: string;
}

interface CliArgs {
  seed: boolean;
  waitSeconds: number;
  doorwayUrl: string;
  storageUrl: string;
}

function parseArgs(): CliArgs {
  const args = process.argv.slice(2);
  const mode = process.env['E2E_MODE'] ?? 'local';

  const result: CliArgs = {
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

function runSeed(storageUrl: string): void {
  console.log('\nSeeding local storage...');
  const seederDir = resolve(REPO_ROOT, 'genesis/seeder');
  try {
    execSync(`npx tsx src/seed-sqlite.ts`, {
      cwd: seederDir,
      stdio: 'inherit',
      timeout: 60_000,
      env: {
        ...process.env,
        STORAGE_URL: storageUrl,
      },
    });
    console.log('  Seeding complete');
  } catch (err) {
    console.error('  Seeding failed:', err instanceof Error ? err.message : err);
  }
}

async function main() {
  const { seed, waitSeconds, doorwayUrl, storageUrl } = parseArgs();
  const timeoutMs = waitSeconds * 1000;

  console.log('Local Stack Health Check');
  console.log(`  Doorway: ${doorwayUrl || '(steward mode — skipped)'}`);
  console.log(`  Storage: ${storageUrl}`);
  console.log('');

  const services: ServiceStatus[] = [];

  // Check storage first (both modes need it)
  console.log('Checking storage...');
  const storageOk = await waitForService('elohim-storage', storageUrl, '/db/stats', timeoutMs);
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
    console.error('\nSome services are not healthy. Start them with: npm run hc:start');
    process.exit(1);
  }

  // Auto-seed if requested and storage is empty
  if (seed && storageOk) {
    const empty = await checkStorageEmpty(storageUrl);
    if (empty) {
      console.log('\nStorage is empty, auto-seeding...');
      runSeed(storageUrl);
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
