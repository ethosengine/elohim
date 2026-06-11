/**
 * Household-formation step definitions — Stage-1 spine.
 *
 * Asserts the OUTPUT of the formation ceremony as projected to elohim-storage:
 *   - the household collective is coherent (family layer, canonical CID)
 *   - the triad are affirmed participants (James sponsored, not self-granted)
 *   - the ambient custody mesh emerged (covered via the shared commitment steps
 *     in resilience.steps.ts — we reuse "I list active … commitments" there)
 *   - ceremony custody is DHT-anchored; fixture custody declares its retirement
 *
 * seed-household-formation.ts drives the real per-conductor choreography in CI;
 * these scenarios verify the wire-shape the projector exposes.
 *
 * Reuses storageGet + commitmentListKey from resilience.steps.ts (single owner
 * of the undici wrapper and the commitment-list stash) rather than duplicating.
 */

import { strict as assert } from 'node:assert';

import { Then, When } from '@cucumber/cucumber';

import { E2EWorld } from '../src/framework/world.js';

import { commitmentListKey, storageGet } from './resilience.steps.js';

const collectiveKey = Symbol('qahal:collective');
const participantsKey = Symbol('qahal:participants');

/** Read a collective record by id from the storage db projection. */
When('I fetch the collective {string}', async function (this: E2EWorld, id: string) {
  const data = await storageGet(`/db/collectives/${encodeURIComponent(id)}`);
  (this as unknown as Record<symbol, unknown>)[collectiveKey] = data;
});

Then('the collective has governance layer {string}', function (this: E2EWorld, layer: string) {
  const c = (this as unknown as Record<symbol, Record<string, unknown>>)[collectiveKey];
  assert.ok(c, 'no collective fetched');
  assert.strictEqual(c['governanceLayer'] ?? c['governance_layer'], layer);
});

Then('the collective is anchored with a canonical collective CID', function (this: E2EWorld) {
  const c = (this as unknown as Record<symbol, Record<string, unknown>>)[collectiveKey];
  assert.ok(c, 'no collective fetched');
  const cid = (c['collectiveCid'] ?? c['collective_cid']) as string | null | undefined;
  assert.ok(
    cid?.startsWith('collective:'),
    `collective_cid not stamped — formation projection has not run (got: ${cid})`
  );
});

/** List participation rows for a collective from the storage db projection. */
When('I list participants of collective {string}', async function (this: E2EWorld, id: string) {
  const data = await storageGet(`/db/collectives/${encodeURIComponent(id)}/participants`);
  const rows = Array.isArray(data)
    ? data
    : ((data['items'] ?? data['participants'] ?? []) as unknown[]);
  (this as unknown as Record<symbol, unknown>)[participantsKey] = rows;
});

Then('the participant set includes the canonical household triad', function (this: E2EWorld) {
  const rows = ((this as unknown as Record<symbol, unknown>)[participantsKey] ?? []) as Record<
    string,
    unknown
  >[];
  const triad = ['human-matthew-manager', 'human-jessica-spouse', 'human-james-son'];
  for (const member of triad) {
    assert.ok(
      rows.some(
        r => r['humanId'] === member || r['human_id'] === member || r['memberCid'] === member
      ),
      `triad member missing from participants: ${member}`
    );
  }
});

Then('the participation of {string} carries a sponsor', function (this: E2EWorld, humanId: string) {
  const rows = ((this as unknown as Record<symbol, unknown>)[participantsKey] ?? []) as Record<
    string,
    unknown
  >[];
  const row = rows.find(r => r['humanId'] === humanId || r['human_id'] === humanId);
  assert.ok(row, `no participation row for ${humanId}`);
  assert.ok(row['sponsorCid'] ?? row['sponsor_cid'], `participation of ${humanId} has no sponsor`);
});

interface ProvenanceCommitmentRow {
  id?: string;
  action?: string;
  dhtAnchorHash?: string | null;
  dht_anchor_hash?: string | null;
  metadata?: {
    seedGeneration?: string;
    fixture?: string;
    retireAt?: string;
  } | null;
}

Then(
  'every {string} commitment with ceremony provenance is DHT-anchored',
  function (this: E2EWorld, action: string) {
    const rows = ((this as unknown as Record<symbol, unknown>)[commitmentListKey] ??
      []) as ProvenanceCommitmentRow[];
    const ceremony = rows.filter(
      r => r.action === action && r.metadata?.seedGeneration === 'ceremony'
    );
    assert.ok(ceremony.length > 0, 'no ceremony-provenance commitments found');
    for (const r of ceremony) {
      assert.ok(r.dhtAnchorHash ?? r.dht_anchor_hash, `unanchored ceremony commitment: ${r.id}`);
    }
  }
);

Then(
  'every {string} commitment with fixture provenance declares its retirement',
  function (this: E2EWorld, action: string) {
    const rows = ((this as unknown as Record<symbol, unknown>)[commitmentListKey] ??
      []) as ProvenanceCommitmentRow[];
    for (const r of rows.filter(
      x => x.action === action && x.metadata?.fixture === 'formation-output'
    )) {
      assert.strictEqual(
        r.metadata?.retireAt,
        'ceremony-landing',
        `fixture row missing retireAt: ${r.id}`
      );
    }
  }
);
