/** Live household bindings for accountable-correction contract revision 3. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { After, Before, Given, When, Then } from '@cucumber/cucumber';

import { MESH_PEER_ORDER } from '../../src/framework/dataplane/carried-election.js';
import { destructiveAllowed } from '../../src/framework/fixtures/substrate-scope.js';
import { E2EWorld } from '../../src/framework/world.js';

import {
  ctx,
  hash,
  raw,
  url,
  rail,
  call,
  http,
  until,
  peerEnv,
  rows,
  notify,
  standing,
  file,
  accept,
  lineage,
  amend,
  applied,
  contribution,
  served,
  pending,
  key,
  attachLag,
  rotationBudgetMs,
  DHT_STEP_TIMEOUT_MS,
  acceptedGroups,
  aggregateRows,
  groupRow,
  CONTRIBUTION_STEP_TIMEOUT_MS,
  REBUILD_BUDGET_MS,
  REBUILD_STEP_TIMEOUT_MS,
} from './accountable-correction.helpers.js';

Before({ tags: '@concern:accountable-correction' }, function (this: E2EWorld, { pickle }) {
  ctx(this).crash = pickle.name.startsWith('A storage restart');
});
After({ tags: '@concern:accountable-correction' }, async function (this: E2EWorld) {
  const s = ctx(this);
  try {
    if (s.originalConfig !== undefined) {
      writeFileSync(peerEnv(this, 'james')['ELOHIM_RUNTIME_CONFIG_PATH'], s.originalConfig);
      await http('james', '/admin/runtime-config/reload', {});
    }
  } finally {
    const directory = s.env.jessica?.['STORAGE_DIR'];
    if (directory) rmSync(resolve(directory, 'feedback-crash-once'), { force: true });
    for (const connection of Object.values(s.rails)) await connection.close();
  }
});
function canonicalRows(world: E2EWorld): Record<string, unknown>[] {
  return rows(
    world,
    'jessica',
    `SELECT g.policy_manifest_cid, a.group_key, a.accepted, a.contribution,
    hex(a.subject_pubkey) subject, m.action_hash, m.member_role
    FROM standing_generations g JOIN feedback_application a USING(generation_id)
    JOIN feedback_application_member m USING(generation_id,group_key)
    WHERE g.generation_id = (SELECT max(generation_id) FROM standing_generations WHERE status='published')
    ORDER BY a.group_key,m.action_hash`
  );
}

Given(
  'Jessica has authored a content record visible to all three peers',
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const s = ctx(this);
    const created = await call(this, 'jessica', 'create_content', {
      id: s.id,
      title: 'Accountable correction fixture',
      description: 'Owned by this scenario',
      content_type: 'concept',
      content_format: 'markdown',
      content: 'Original claim',
      tags: [],
      reach: 'commons',
      metadata_json: '{}',
    });
    s.root = hash(created.action_hash);
    for (const peer of MESH_PEER_ORDER) {
      await until(`${peer} sees Jessica's record`, async () => {
        const found = await call(this, peer, 'get_content', raw(s.root));
        return Boolean(found && hash(found.action_hash) === s.root);
      });
      peerEnv(this, peer);
    }
    if (s.crash)
      writeFileSync(
        resolve(peerEnv(this, 'jessica')['STORAGE_DIR'], 'feedback-crash-once'),
        'armed'
      );
  }
);

Given(
  "James's peer has peer-to-peer feedback notifications disabled",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await notify(this, false);
  }
);

Given(
  "James's peer has peer-to-peer feedback notifications enabled",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await notify(this, true);
  }
);

When(
  "James files a correction against Jessica's record with no notification sent",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await file(this);
  }
);

Then(
  "Matthew's peer discovers James's correction within its next discovery scan, unnotified",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await applied(this, 'matthew');
    const subscriptions = rows(
      this,
      'matthew',
      'SELECT source FROM feedback_subscriptions WHERE member_key = ?',
      [ctx(this).correction]
    );
    assert.ok(subscriptions.length > 0);
    assert.ok(subscriptions.every(r => r['source'] !== 'notified'));
  }
);

Given(
  'James has filed an earlier correction whose target link is not published until after the later one has been applied',
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    return pending(
      this,
      "create_feedback_signal commits the entry and BOTH index links in one extern call (content_store/src/feedback_signal.rs:168-192: create_entry, then create_link TargetToFeedbackSignal, then create_link SignerToFeedbackSignal); there is no link-only extern, so an act's link cannot be published later than the act. Delaying it needs a new coordinator extern authored for the product, not for this fixture."
    );
  }
);

Given(
  "Matthew's peer has already applied the first correction it discovered",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await applied(this, 'matthew');
  }
);

When(
  "the earlier correction's link surfaces to Matthew's peer on a later scan",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    return pending(
      this,
      'The delayed-link fixture in the preceding Given does not exist: link creation is bound to entry creation in create_feedback_signal.'
    );
  }
);

Then(
  "Matthew's peer applies the earlier correction exactly once",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    return pending(
      this,
      'No delayed-link act was staged; asserting exactly-once against an act that arrived in order would be vacuous.'
    );
  }
);

Then(
  "Matthew's peer still shows the first correction applied exactly once, undisturbed by the late arrival",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await applied(this, 'matthew');
  }
);

When(
  "James files a correction against Jessica's record with a notification sent",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await notify(this, true);
    await file(this);
  }
);

Then(
  "Matthew's peer applies the correction before its next scheduled discovery scan would otherwise have found it",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    return pending(
      this,
      'Notifications only enqueue subscriptions; there is no notification-driven wake or observable next-scan deadline. The current loop cannot prove acceleration before its scheduled tick.'
    );
  }
);

Then(
  "Matthew's peer's applied correction matches the fetched, verified record James actually authored",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await applied(this, 'matthew');
    const r = await call(this, 'matthew', 'get_feedback_signal_record', raw(ctx(this).correction));
    assert.ok(r, 'signed correction is fetchable');
    this.attach(JSON.stringify(r), 'application/json');
  }
);

When(
  "a feedback notification arrives at Matthew's peer naming a foreign origin DNA hash",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    return pending(
      this,
      'No household fixture can inject a foreign-DNA feedback envelope through the P2P receive path. HTTP submission is not that path.'
    );
  }
);

Then(
  "Matthew's peer rejects the foreign-DNA notification without applying anything from it",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    return pending(
      this,
      'No foreign-DNA envelope was delivered; absence of an application row would not prove rejection.'
    );
  }
);

Then(
  "Matthew's peer's own content cell DNA hash is unchanged by the rejected notification",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    return pending(
      this,
      'The preceding foreign-DNA transport injection is unavailable; no rejection was exercised.'
    );
  }
);

Given(
  "James has filed a correction against Jessica's record",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await file(this);
  }
);

When(
  "Jessica accepts James's correction with an accept-correction vouch",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await accept(this);
  }
);

Given(
  "Jessica has accepted James's correction with an accept-correction vouch",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await accept(this);
  }
);

Then(
  "Jessica's acceptance vouch is a distinct, visible record from James's correction",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    assert.notEqual(ctx(this).acceptance, ctx(this).correction);
    for (const action of [ctx(this).acceptance, ctx(this).correction]) {
      assert.ok(await call(this, 'jessica', 'get_feedback_signal_record', raw(action)));
    }
  }
);

When(
  "Jessica publishes the amended content naming the corrected record's exact predecessor action",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    ctx(this).successor = await amend(this, ctx(this).root, 'Corrected claim');
  }
);

Then(
  "Jessica's successor is a third record, distinct from both the correction and the acceptance",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    assert.equal(
      new Set([ctx(this).successor, ctx(this).correction, ctx(this).acceptance]).size,
      3
    );
  }
);

Then(
  "all three of Matthew's, Jessica's, and James's peers adopt Jessica's successor as the served head",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    for (const peer of MESH_PEER_ORDER) {
      await until(
        `${peer} serves successor`,
        async () => (await served(this, peer)) === ctx(this).successor
      );
    }
  }
);

Then(
  "a dependent view reading the record re-renders the successor's content",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    return pending(
      this,
      'The dependent-view re-render is a browser assertion, but no correction-dependent view/route is specified or implemented. An HTTP read alone would not prove re-rendering.'
    );
  }
);

When(
  'James attempts to accept his own correction with an accept-correction vouch',
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    try {
      await call(this, 'james', 'create_vouch', {
        target_action_hash: raw(ctx(this).correction),
        vouch_kind: 'accept-correction',
        standing_impact: 'debit-soft',
      });
    } catch (e) {
      ctx(this).refusal = String(e);
    }
  }
);

Then(
  "James's acceptance attempt is refused because he is not the record's root author",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    assert.ok(ctx(this).refusal, 'James must be refused');
    assert.match(ctx(this).refusal!, /self|own|same|signer/i);
  }
);

Then(
  "Jessica's earlier acceptance and successor are unaffected by James's refused attempt",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    assert.ok(await call(this, 'jessica', 'get_feedback_signal_record', raw(ctx(this).acceptance)));
    assert.equal(await served(this), ctx(this).successor);
  }
);

When(
  "Jessica's peer's storage is restarted after the correction is marked applied but before her standing tally reflects it",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    assert.ok(destructiveAllowed(), 'mesh destructive opt-in required');
    const arm = resolve(peerEnv(this, 'jessica')['STORAGE_DIR'], 'feedback-crash-once');
    // The arm is consumed at the same commit the accepted contribution lands on
    // (feedback_projector.rs:571), so it waits on the SAME rotation the tally does. A
    // fixed 210 s here failed station 4 twice for the reason station 7 failed once.
    await until('projector consumes crash arm', () => !existsSync(arm), rotationBudgetMs(this));
    const result = spawnSync(
      '/bin/bash',
      [resolve('../../app/elohim-app/scripts/hc-mesh.sh'), 'storage-restart', 'jessica'],
      { encoding: 'utf8', timeout: 180_000 }
    );
    this.attach(result.stdout + result.stderr, 'text/plain');
    assert.equal(result.status, 0, 'storage restart failed');
  }
);

Then(
  "Jessica's peer's projection shows neither a correction marked applied with no matching tally change, nor a tally change with no correction marked applied",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    const result = rows(
      this,
      'jessica',
      `SELECT a.accepted, a.contribution,
    (SELECT coalesce(sum(debit_weight_sum),0) FROM standing_generation_aggregate g WHERE g.generation_id=a.generation_id) total
    FROM feedback_application a JOIN feedback_application_member m USING(generation_id,group_key)
    WHERE m.action_hash = ? ORDER BY a.generation_id DESC LIMIT 1`,
      [ctx(this).correction]
    );
    if (result[0]?.['accepted'] === 1)
      assert.ok(Number(result[0]['total']) >= Number(result[0]['contribution']));
    else assert.equal(result[0]?.['contribution'] ?? 0, 0);
  }
);

When(
  "Jessica's peer's discovery resumes after the restart",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await applied(this);
  }
);

Then(
  "the accepted correction is applied exactly once to Jessica's standing tally",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await contribution(this);
  }
);

When(
  "the same accepted correction is replayed against Jessica's peer after it was already settled",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    ctx(this).beforeStanding = await standing(this);
    const before = rows(
      this,
      'jessica',
      'SELECT visit_count FROM feedback_subscriptions WHERE member_key = ?',
      [ctx(this).correction]
    )[0];
    await until(
      'next discovery replay',
      () =>
        Number(
          rows(
            this,
            'jessica',
            'SELECT visit_count FROM feedback_subscriptions WHERE member_key = ?',
            [ctx(this).correction]
          )[0]?.['visit_count']
        ) > Number(before?.['visit_count']),
      // One more visit to this member is one more ROTATION of the subscription set.
      rotationBudgetMs(this)
    );
  }
);

Then(
  "Jessica's standing tally is unchanged by the replay",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    assert.deepEqual(await standing(this), ctx(this).beforeStanding);
  }
);

// Read as a fact about the generation rather than against a captured baseline: James is
// never a corrected record's root author, so no accepted group names him as subject and no
// aggregate row for him exists. Filing costs the filer nothing, in every run order.
Then(
  "James's own standing is unaffected by the correction he filed",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const evaluator = key((await rail(this, 'jessica')).agent);
    const james = key((await rail(this, 'james')).agent);
    assert.equal(acceptedGroups(this, james).groups, 0, 'no accepted group names the filer');
    assert.equal(aggregateRows(this, evaluator, james).rows, 0, 'the filer has no row at all');
    assert.equal((await standing(this, 'james'))['score'], 'unknown');
  }
);

// A GENERATION is (evaluator, pinned policy) — not (evaluator, scenario). Opening a
// fresh one does not isolate a scenario: `FeedbackProjector::tick` replays every
// RETAINED subscription member into whichever generation `resolve_generation` returns,
// so a fresh or rebuilt generation lands on the same subject aggregate it started from.
// And a fresh evaluator IDENTITY is worse than useless here: no projector runs for a key
// nobody evaluates with, so it reads Unknown whether or not the correction is ever
// accepted, and the acceptance half of this station could never pass. Standing is kept
// PER AUTHOR, so Jessica accumulates across every station that corrects a record of hers,
// and "no row at all" is only observable for a subject with no accepted correction
// anywhere. Assert what is true in every order, against the generation's own arithmetic
// rather than a captured baseline: this correction contributed nothing, and the served
// tally is exactly the sum of the accepted groups — which does not include it.
Then(
  "Jessica's standing is exactly what it was before he filed, and the unaccepted correction adds no row and no weight of its own",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const evaluator = key((await rail(this, 'jessica')).agent);
    // The aggregate republishes on a clean sweep, so it can trail the group table by a
    // tick: poll the two into agreement rather than sampling them and calling a lagging
    // republication a wrong tally.
    let accepted = acceptedGroups(this, evaluator);
    let served = Number.NaN;
    await until(
      'the served tally agrees with the accepted groups',
      async () => {
        accepted = acceptedGroups(this, evaluator);
        served = Number((await standing(this))['debitWeightSum']);
        return served === accepted.total;
      },
      rotationBudgetMs(this)
    );
    const group = groupRow(this);
    if (group) {
      assert.equal(group['accepted'], 0, 'an unaccepted allegation must never read accepted');
      assert.equal(group['contribution'], 0, 'an allegation debits nobody');
    }
    const aggregate = aggregateRows(this, evaluator, evaluator);
    assert.equal(
      aggregate.rows,
      accepted.groups === 0 ? 0 : 1,
      'a subject whose only signal is an unaccepted correction has no aggregate row at all'
    );
    assert.equal(aggregate.total, accepted.total, 'the aggregate carries only accepted groups');
    assert.equal(served, accepted.total, 'the served tally carries only accepted groups');
  }
);

Then(
  "Jessica's standing now exists and reflects exactly one contribution",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await contribution(this);
  }
);

When(
  "Jessica accepts James's same correction with a second, redundant accept-correction vouch",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await accept(this);
  }
);

Then(
  "Jessica's standing still reflects exactly one contribution",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await until(
      'redundant acceptance observed',
      () =>
        rows(
          this,
          'jessica',
          'SELECT member_status FROM feedback_application_member WHERE action_hash = ?',
          [ctx(this).acceptance]
        ).some(r => r['member_status'] === 'member'),
      rotationBudgetMs(this)
    );
    await contribution(this);
  }
);

When(
  'Jessica publishes two amended successors that each name the same predecessor action',
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    ctx(this).branches = [
      await amend(this, ctx(this).root, 'Branch A'),
      await amend(this, ctx(this).root, 'Branch B'),
    ];
  }
);

Then(
  "Jessica's peer marks the record contested, visibly listing both branches",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const l = await lineage(this);
    assert.equal(l.contested, true);
    for (const h of ctx(this).branches)
      assert.ok(
        l.candidates.some(c => hash(c.action_hash) === h && hash(c.predecessor) === ctx(this).root)
      );
  }
);

When(
  "the deterministic pick resolves one of Jessica's two branches as the served head",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await until('served head is a branch', async () => {
      ctx(this).chosen = await served(this);
      return ctx(this).branches.includes(ctx(this).chosen);
    });
  }
);

Then(
  'the record is still marked contested after the deterministic pick',
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    assert.equal((await lineage(this)).contested, true);
  }
);

When(
  'Jessica publishes a further amendment naming her already-resolved served head as its predecessor',
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    ctx(this).sequentialPredecessor = ctx(this).chosen;
    ctx(this).successor = await amend(this, ctx(this).chosen, 'Sequential fix');
  }
);

Then(
  'this sequential amendment is not marked contested',
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const l = await lineage(this);
    assert.ok(
      !l.contested_predecessors.map(hash).includes(ctx(this).sequentialPredecessor),
      'Sequential edge must not create a new fork'
    );
    assert.equal(l.contested, true, 'Earlier observed fork must remain named');
  }
);

// This step spends TWO independent budgets and used to hide both behind one: the
// acceptance had to reach the live tally BEFORE a rebuild could be compared against it,
// and then the rebuild itself had to finish. Measured separately (see the attached lag
// records) so a slow replay can never again be reported as a wrong tally.
When(
  'a fresh standing generation is built from scratch from every retained correction and acceptance, then published',
  { timeout: REBUILD_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await contribution(this);
    const s = ctx(this);
    s.beforeRows = canonicalRows(this);
    const result = await http('jessica', '/api/v1/feedback/generations/rebuild', {});
    s.rebuilt = Number(result['generationId']);
    assert.equal(result['status'], 'rebuilding');
    const read = await fetch(
      `${url('jessica')}/api/v1/standing/${(await rail(this, 'jessica')).agent}?evaluator=${(await rail(this, 'jessica')).agent}`
    );
    assert.equal(read.status, 503, 'readers must see rebuilding');
    assert.match(await read.text(), /rebuilding/);
    const started = Date.now();
    let settled: number | null = null;
    try {
      await until(
        'rebuild publishes',
        () => {
          const published =
            rows(
              this,
              'jessica',
              'SELECT status FROM standing_generations WHERE generation_id = ?',
              [s.rebuilt]
            )[0]?.['status'] === 'published';
          if (published) settled = Date.now() - started;
          return published;
        },
        REBUILD_BUDGET_MS
      );
    } finally {
      attachLag(this, {
        label: 'rebuild requested to generation published',
        budgetMs: REBUILD_BUDGET_MS,
        firstMarkMs: null,
        settledMs: settled,
      });
    }
  }
);

Then(
  "the fresh generation's tallies are identical to the live generation's, excluding storage layout and operational timestamps",
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    assert.deepEqual(canonicalRows(this), ctx(this).beforeRows);
  }
);

Then(
  'readers of the fresh generation see it as still rebuilding until every retained correction and acceptance has replayed, never a partial tally',
  { timeout: DHT_STEP_TIMEOUT_MS },
  function (this: E2EWorld) {
    assert.ok(ctx(this).rebuilt, 'rebuild was actually requested');
    assert.equal(
      rows(
        this,
        'jessica',
        "SELECT count(*) n FROM feedback_application_member WHERE generation_id = ? AND member_status = 'pending'",
        [ctx(this).rebuilt]
      )[0]?.['n'],
      0
    );
  }
);

Given(
  "James's first-party view files a correction against Jessica's record under one operation id it minted",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await file(this, true);
  }
);

When(
  "the response to James's filing is lost before his view receives it",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    assert.equal(
      ctx(this).operation,
      undefined,
      'The first-party client never consumed the response body'
    );
    const outcome = await http('james', `/api/v1/feedback/operations/${ctx(this).operationId}`);
    assert.equal(
      outcome['status'],
      'resolved',
      'independent server witness proves commit preceded response loss'
    );
  }
);

When(
  "James's view reloads and retries the filing with the same operation id",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await file(this);
  }
);

Then(
  "James's peer reports the operation as resolved to exactly one correction, or unresolved, but never two",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const outcome = await http('james', `/api/v1/feedback/operations/${ctx(this).operationId}`);
    assert.ok(['resolved', 'unresolved'].includes(String(outcome['status'])));
    assert.equal(outcome['feedbackActionHash'], ctx(this).operation?.['feedbackActionHash']);
  }
);

Then(
  "the retried filing produces at most one contribution toward Jessica's eventual standing change",
  { timeout: CONTRIBUTION_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    await accept(this);
    await contribution(this);
  }
);

Then(
  "a second presentation reading the same subject agrees with James's view about what was filed",
  { timeout: DHT_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const operation = await http('james', `/api/v1/feedback/operations/${ctx(this).operationId}`);
    const record = await call(
      this,
      'james',
      'get_feedback_signal_record',
      raw(String(operation['feedbackActionHash']))
    );
    assert.ok(record);
    const evidence = await call(
      this,
      'james',
      'get_content',
      raw(String(operation['evidenceActionHash']))
    );
    assert.ok(evidence);
    const request = (
      JSON.parse(evidence.content.metadata_json) as { correctionRequest: Record<string, unknown> }
    ).correctionRequest;
    assert.equal(request.operationId, ctx(this).operationId);
    assert.equal(request.targetActionHash, ctx(this).root);
    assert.equal(request.signalKind, 'correction');
  }
);
