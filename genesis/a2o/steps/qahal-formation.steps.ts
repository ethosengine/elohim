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
const collectiveIdKey = Symbol('qahal:collectiveId');
const participantsKey = Symbol('qahal:participants');

/** Read a collective record by id from the storage db projection. */
When('I fetch the collective {string}', async function (this: E2EWorld, id: string) {
  const data = await storageGet(`/db/collectives/${encodeURIComponent(id)}`);
  const w = this as unknown as Record<symbol, unknown>;
  w[collectiveKey] = data;
  w[collectiveIdKey] = id;
});

Then('the collective has governance layer {string}', function (this: E2EWorld, layer: string) {
  const c = (this as unknown as Record<symbol, Record<string, unknown>>)[collectiveKey];
  assert.ok(c, 'no collective fetched');
  assert.strictEqual(c['governanceLayer'] ?? c['governance_layer'], layer);
});

Then('the collective is anchored with a canonical collective CID', function (this: E2EWorld) {
  const w = this as unknown as Record<symbol, unknown>;
  const c = w[collectiveKey] as Record<string, unknown> | undefined;
  const slug = (w[collectiveIdKey] as string | undefined) ?? '<unfetched>';
  assert.ok(c, 'no collective fetched');
  const cid = (c['collectiveCid'] ?? c['collective_cid']) as string | null | undefined;
  // Diagnosis note (genesis #1489): "the formation projection has not run" was
  // the WRONG reading of this red and cost a triage cycle. Two distinct causes
  // wear the same `undefined`, and only the row's KEYS tell them apart:
  //
  //   1. The field is not projected at all. GET /db/collectives/{id} answers
  //      with elohim-views `CollectiveView`, which carries no collectiveCid —
  //      while `collectives.collective_cid` is a real column that the formation
  //      projection stamps. When the key is simply absent from every row, no
  //      ceremony can turn this green; the WIRE SHAPE is the defect.
  //   2. The field is projected but null on this peer — the cid genuinely is
  //      unstamped here, because `CollectiveCommitted` is a post_commit signal
  //      local to the authoring conductor and cross-peer projection_reconcile
  //      has not gap-filled it (or the ceremony reused an existing collective,
  //      so no fresh signal was emitted for this peer to observe at all).
  //
  // Printing the row's keys is what separates them, so it is not optional here.
  assert.ok(
    cid?.startsWith('collective:'),
    `collective "${slug}" carries no canonical collective_cid on this storage peer ` +
      `(got: ${JSON.stringify(cid)}). The row EXISTS — governanceLayer=` +
      `${JSON.stringify(c['governanceLayer'] ?? c['governance_layer'])} — so the projection ` +
      `plainly ran. Row keys: ${JSON.stringify(Object.keys(c))}. If no cid key appears there, ` +
      `the read view does not project the column and no ceremony can satisfy this; if the key ` +
      `is present and null, the cid is genuinely unstamped on THIS peer — check which ` +
      `conductor authored create_collective and whether projection_reconcile has gap-filled it.`
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

/**
 * The canonical Dowell triad, founder first — byte-identical with
 * `HOUSEHOLD_MEMBERS` in genesis/seeder/src/seed-household-formation.ts. Note
 * `human-james-son` (NOT `human-james-student`): that is the id the seeder, the
 * genesis corpus and orchestrator/data/deployments.json all carry.
 */
const HOUSEHOLD_TRIAD = ['human-matthew-manager', 'human-jessica-spouse', 'human-james-son'];

/** The human id a participation row names, under any of the shapes we accept. */
function participantHumanId(row: Record<string, unknown>): string | undefined {
  const id = row['humanId'] ?? row['human_id'] ?? row['memberCid'] ?? row['member_cid'];
  return typeof id === 'string' ? id : undefined;
}

Then('the participant set includes the canonical household triad', function (this: E2EWorld) {
  const rows = ((this as unknown as Record<symbol, unknown>)[participantsKey] ?? []) as Record<
    string,
    unknown
  >[];
  const present = rows.map(participantHumanId).filter((v): v is string => v !== undefined);
  const missing = HOUSEHOLD_TRIAD.filter(member => !present.includes(member));
  // Report the WHOLE gap, not just the first member of the triad to be absent:
  // a partial ceremony (some members affirmed, some unbindable) and a wholly
  // unprojected collective are different defects, and only the full missing set
  // plus what IS present distinguishes them at a glance (genesis #1489 reported
  // one name and read as "nothing landed" when in fact two of three had).
  assert.ok(
    missing.length === 0,
    `triad member(s) missing from participants: ${missing.join(', ')}. ` +
      `Present (${present.length} of ${rows.length} row(s)): ${present.join(', ') || '<none>'}. ` +
      `A member is absent here when their conductor never affirmed membership, or when ` +
      `their Membership was authored on a peer whose projection has not reached this one.`
  );
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
