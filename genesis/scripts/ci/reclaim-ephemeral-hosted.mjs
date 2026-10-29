#!/usr/bin/env node
// Reclaims hosted-conductor slots leaked by a2o test runs on a doorway.
//
// Soft-deleting a hosted user only flips Mongo flags; the cell stays installed
// and keeps a pool-conductor slot. This lists /admin/hosted-users and
// deprovisions (DELETE /admin/hosted-users/{agentKey}) ephemeral test users:
// identifier local part starts with an ephemeral prefix AND (inactive OR an
// embedded 13-digit ms timestamp older than --min-age-hours).
//
// Never fails a build: HTTP errors are printed, exit 0. Exit 2 = bad arguments.
// The admin key is read from env API_KEY_ADMIN only (never argv).

import { pathToFileURL } from 'node:url';

export const EPHEMERAL_LOCAL_PART_PREFIXES = ['e2e-', 'portal-a2o-'];
export const DEFAULT_MIN_AGE_HOURS = 6;
const REQUEST_TIMEOUT_MS = 30_000;

function localPart(identifier) {
  const s = String(identifier ?? '');
  const at = s.indexOf('@');
  return at === -1 ? s : s.slice(0, at);
}

export function isEphemeralCandidate(user) {
  const lp = localPart(user?.identifier);
  return EPHEMERAL_LOCAL_PART_PREFIXES.some((p) => lp.startsWith(p));
}

/**
 * Pure selection. Returns { candidates, reclaimable, skippedActive }.
 * reclaimable are the candidates to deprovision; skippedActive the rest.
 */
export function selectReclaimable(users, nowMs, minAgeHours = DEFAULT_MIN_AGE_HOURS) {
  const candidates = (Array.isArray(users) ? users : []).filter(isEphemeralCandidate);
  const cutoff = nowMs - minAgeHours * 3_600_000;
  const reclaimable = [];
  const skippedActive = [];
  for (const u of candidates) {
    let old = false;
    const m = /(\d{13})/.exec(String(u.identifier));
    if (m) old = Number(m[1]) < cutoff;
    if (u.isActive === false || old) reclaimable.push(u);
    else skippedActive.push(u);
  }
  return { candidates, reclaimable, skippedActive };
}

async function request(fetchImpl, url, method, key) {
  const res = await fetchImpl(url, {
    method,
    headers: { 'X-API-Key': key },
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  if (!res.ok) throw new Error(`${method} ${url} -> HTTP ${res.status}`);
  return res;
}

export async function run({ baseUrl, dryRun, minAgeHours, env, fetchImpl, now, log }) {
  const key = env.API_KEY_ADMIN;
  if (!key) {
    log('reclaim-ephemeral-hosted: skipped (API_KEY_ADMIN not set)');
    return { skipped: true };
  }
  const base = baseUrl.replace(/\/+$/, '');
  let users = [];
  try {
    const body = await (await request(fetchImpl, `${base}/admin/hosted-users`, 'GET', key)).json();
    users = Array.isArray(body?.users) ? body.users : [];
  } catch (e) {
    log(`reclaim-ephemeral-hosted: list failed: ${e.message}`);
    log('reclaim-ephemeral-hosted: listed=0 candidates=0 freed=0 skipped-active=0 failed=1');
    return { listed: 0, candidates: 0, freed: 0, skippedActive: 0, failed: 1 };
  }
  const { candidates, reclaimable, skippedActive } = selectReclaimable(users, now, minAgeHours);
  let freed = 0;
  let failed = 0;
  for (const u of reclaimable) {
    if (dryRun) {
      log(`reclaim-ephemeral-hosted: dry-run would free ${u.identifier}`);
      continue;
    }
    try {
      await request(fetchImpl, `${base}/admin/hosted-users/${encodeURIComponent(u.agentPubKey)}`, 'DELETE', key);
      freed++;
    } catch (e) {
      failed++;
      log(`reclaim-ephemeral-hosted: ${u.identifier}: ${e.message}`);
    }
  }
  log(
    `reclaim-ephemeral-hosted:${dryRun ? ' (dry-run)' : ''} listed=${users.length} candidates=${candidates.length} ` +
      `freed=${freed} skipped-active=${skippedActive.length} failed=${failed}`,
  );
  return { listed: users.length, candidates: candidates.length, freed, skippedActive: skippedActive.length, failed };
}

export function parseArgs(argv) {
  const out = { baseUrl: null, dryRun: false, minAgeHours: DEFAULT_MIN_AGE_HOURS };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--dry-run') out.dryRun = true;
    else if (a === '--min-age-hours') {
      const v = Number(argv[++i]);
      if (!Number.isFinite(v) || v < 0) return null;
      out.minAgeHours = v;
    } else if (a.startsWith('--') || out.baseUrl) return null;
    else out.baseUrl = a;
  }
  return out.baseUrl ? out : null;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const args = parseArgs(process.argv.slice(2));
  if (!args) {
    console.error('usage: reclaim-ephemeral-hosted.mjs <doorwayBaseUrl> [--dry-run] [--min-age-hours N]');
    process.exit(2);
  }
  try {
    await run({ ...args, env: process.env, fetchImpl: fetch, now: Date.now(), log: console.log });
  } catch (e) {
    console.log(`reclaim-ephemeral-hosted: unexpected error: ${e?.message ?? e}`);
  }
  process.exit(0);
}
