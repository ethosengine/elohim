import { strict as assert } from 'node:assert';
import { test } from 'node:test';

import {
  checkClosingPublication,
  checkClosingStage,
} from '../commons-path-steward-publish.helpers.js';

import type { PublicationReceipt } from '../../../scripts/lib/steward-publication-receipt.js';

function fixture() {
  const context = {
    id: 'one-course-item',
    root: 'matthew-root',
    matthew: 'matthew',
    che: 'original-che',
    humanId: 'actual-matthew-human',
    dna: 'shared-dna',
    adam: 'independent-adam',
    doorways: ['https://alpha.example', 'https://apex.example'] as [string, string],
  };
  const receipt: PublicationReceipt = {
    id: context.id,
    seedHash: 'body-hash',
    agent: context.che,
    dna: context.dna,
    storage: 'http://che.example',
    head: 'che-head-one',
    authoredAt: new Date(1000).toISOString(),
    declaredAt: new Date(1200).toISOString(),
    execution: {
      requester: 'che-requester',
      executor: context.che,
      humanId: context.humanId,
      humanAction: 'actual-human-root',
      binding: 'original-device-binding',
    },
    acceptedDelegation: {
      grantor: context.matthew,
      delegate: context.che,
      scope: context.id,
      rootActionHash: context.root,
      dnaHash: context.dna,
      validUntil: 90000,
      signature: 'signature',
      acceptance: {
        headActionHash: 'che-head-one',
        witnessActionHash: 'root-acceptance',
        deviceWitnessActionHash: 'human-witness',
        acceptedAt: 1100,
        signature: 'signature',
      },
    },
  };
  const checks = [
    {
      checkId: 'federation-deploy',
      outcome: 'passed',
      observed: {
        agent: context.adam,
        dna: context.dna,
        accepted: true,
        acceptedAt: 2000,
        electionLink: 'adam-election-link',
        deadlineMs: 76000,
        target: { id: context.id, head: receipt.head, root: context.root, dna: context.dna },
      },
    },
    ...context.doorways.map(peer => ({
      checkId: 'federation-deploy',
      outcome: 'passed',
      observed: {
        peer,
        id: context.id,
        head: receipt.head,
        seedHash: receipt.seedHash,
        servedAt: 3000,
        deadlineMs: 76000,
      },
    })),
  ];
  return {
    context,
    receipt,
    checks,
    output: () => checks.map(check => JSON.stringify(check)).join('\n'),
  };
}

void test('closing publication requires independent native election and both doorway receipts', () => {
  const f = fixture();
  assert.equal(
    checkClosingPublication(f.context, f.receipt, f.output(), 1000).head,
    f.receipt.head
  );
});

void test('closing publication rejects a second budget and a late doorway response', () => {
  for (const mutate of [
    (f: ReturnType<typeof fixture>) => {
      f.checks[1].observed.deadlineMs++;
    },
    (f: ReturnType<typeof fixture>) => {
      assert.ok('servedAt' in f.checks[1].observed);
      f.checks[1].observed.servedAt = 76001;
    },
  ]) {
    const f = fixture();
    mutate(f);
    assert.throws(() => checkClosingPublication(f.context, f.receipt, f.output(), 1000));
  }
});

void test('closing publication rejects local proof, wrong root, duplicate doorway and replayed update', () => {
  const local = fixture();
  local.context.adam = local.context.che;
  assert.throws(() => checkClosingPublication(local.context, local.receipt, local.output(), 1000));
  const wrong = fixture();
  wrong.receipt.acceptedDelegation!.rootActionHash = 'another-root';
  assert.throws(() => checkClosingPublication(wrong.context, wrong.receipt, wrong.output(), 1000));
  const duplicate = fixture();
  duplicate.checks.push(duplicate.checks[1]);
  assert.throws(() =>
    checkClosingPublication(duplicate.context, duplicate.receipt, duplicate.output(), 1000)
  );
  const replay = fixture();
  assert.throws(() =>
    checkClosingPublication(
      replay.context,
      replay.receipt,
      replay.output(),
      1000,
      replay.receipt.head
    )
  );
});

void test('a saved receipt cannot turn an unrelated stage failure into permission to advance', () => {
  const f = fixture();
  const expected = `${f.context.id}  reach=commons  action=pending  head=${f.receipt.head}  (publication pending: authored locally; connected acceptance and declaration required)`;
  checkClosingStage({ code: 1, stdout: expected }, f.context.id, f.receipt.head);
  for (const stage of [
    {
      code: 1,
      stdout: expected.replace(
        'authored locally; connected acceptance and declaration required',
        'signature verification failed'
      ),
    },
    { code: 143, stdout: expected },
    { code: 0, stdout: expected },
    { code: 1, stdout: expected.replace(f.receipt.head, 'another-head') },
    { code: 1, stdout: `${expected}\n${expected}` },
  ])
    assert.throws(() => checkClosingStage(stage, f.context.id, f.receipt.head));
});
