/**
 * Seed Presences — POSTs each presence from genesis/data/presences/ to /db/presences.
 *
 * Reads presence markdown files, validates them via validate-presences.ts, and
 * posts to the doorway API. Idempotent: HTTP 409 (already exists) is treated
 * as success.
 *
 * Ordering:
 *   Must run AFTER seed-humans.ts  (presences reference humans as observers/stewards)
 *   Must run BEFORE seed-accounts.ts (account packages reference presence IDs in stewardship)
 *
 * Wire shape: matches CreateContributorPresenceInputView in elohim-storage/src/views.rs
 * (see /db/presences POST handler in http.rs). Fields not supported at the top level
 * (presenceType, observations, primaryStewardId, stewardshipStartedAt, works,
 * sameAsPresenceIds, suggestedCollectiveIds, tags, externalIdentifiers, image)
 * ride through `metadata` as a JsonValue blob. `establishingContentIds` is the one
 * top-level extraction derived from observations[].contextContentId.
 *
 * Environment:
 *   DOORWAY_URL   Doorway URL (default: http://localhost:8888)
 *
 * Exit codes:
 *   0 — all presences created or already exist
 *   1 — one or more presences failed
 */

import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { validatePresencesDirectory, type PresenceFrontmatter } from './validate-presences.js';

// =============================================================================
// Types
// =============================================================================

type Outcome = 'created' | 'exists' | 'failed';

interface PostResult {
  id: string;
  outcome: Outcome;
  error?: string;
}

// =============================================================================
// Body shaping
// =============================================================================

/**
 * Map a validated PresenceFrontmatter to the POST body shape expected by
 * elohim-storage's /db/presences endpoint (CreateContributorPresenceInputView).
 *
 * Fields without a first-class home on the Rust side are stashed in `metadata`.
 */
function frontmatterToBody(fm: PresenceFrontmatter): Record<string, unknown> {
  const establishingContentIds = fm.observations
    .map(o => o.contextContentId)
    .filter((id): id is string => id != null);

  const metadata: Record<string, unknown> = {
    presenceType: fm.presenceType,
    observations: fm.observations,
    primaryStewardId: fm.primaryStewardId ?? null,
    stewardshipStartedAt: fm.stewardshipStartedAt ?? null,
    establishedAt: fm.observations[0]?.observedAt ?? null,
    sameAsPresenceIds: fm.sameAsPresenceIds ?? [],
    works: fm.works ?? [],
    suggestedCollectiveIds: fm.suggestedCollectiveIds ?? [],
    tags: fm.tags ?? [],
    image: fm.image ?? null,
  };

  return {
    id: fm.id,
    schemaVersion: 1,
    displayName: fm.displayName,
    externalIdentifiers: fm.externalIdentifiers ?? [],
    establishingContentIds,
    image: fm.image?.local ?? null,
    note: fm.bio ?? null,
    metadata,
  };
}

// =============================================================================
// POST
// =============================================================================

async function postPresence(
  doorwayUrl: string,
  body: Record<string, unknown>,
): Promise<PostResult> {
  const id = String(body.id);
  try {
    const res = await fetch(`${doorwayUrl}/db/presences`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (res.ok) return { id, outcome: 'created' };
    if (res.status === 409) return { id, outcome: 'exists' };
    const errorText = await res.text();
    return { id, outcome: 'failed', error: `HTTP ${res.status}: ${errorText}` };
  } catch (err) {
    return {
      id,
      outcome: 'failed',
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const doorwayUrl = (process.env.DOORWAY_URL ?? 'http://localhost:8888').replace(/\/$/, '');

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const presencesDir = resolve(__dirname, '../../data/presences');

  console.log('=== Seed Presences ===\n');
  console.log(`Doorway:  ${doorwayUrl}`);
  console.log(`Source:   ${presencesDir}\n`);

  const validation = await validatePresencesDirectory(presencesDir);
  if (validation.errors.length > 0) {
    console.error('Validation failed:');
    for (const e of validation.errors) console.error(`  x ${e}`);
    process.exit(1);
  }
  console.log(`Validated ${validation.presences.size} presences\n`);

  const results: PostResult[] = [];
  for (const [, fm] of validation.presences) {
    const body = frontmatterToBody(fm);
    const result = await postPresence(doorwayUrl, body);
    results.push(result);

    const icon = result.outcome === 'created' ? '+' : result.outcome === 'exists' ? '=' : 'x';
    const errPart = result.error ? ` — ${result.error}` : '';
    console.log(`  [${icon}] ${result.id}${errPart}`);
  }

  const created = results.filter(r => r.outcome === 'created').length;
  const existing = results.filter(r => r.outcome === 'exists').length;
  const failed = results.filter(r => r.outcome === 'failed').length;

  console.log(`\n=== ${created} created, ${existing} existing, ${failed} failed ===`);

  if (failed > 0) {
    console.error(`\n${failed} presences failed to seed.`);
    process.exit(1);
  }
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
