/** Checks the maintained publisher's receipts; it grants no authority and performs no writes. */
import { strict as assert } from 'node:assert';

import type { PublicationReceipt } from '../../scripts/lib/steward-publication-receipt.js';

export interface ClosingPublicationContext {
  id: string;
  root: string;
  matthew: string;
  che: string;
  humanId: string;
  dna: string;
  adam: string;
  doorways: [string, string];
}

interface Observation {
  id?: string;
  head?: string;
  seedHash?: string;
  peer?: string;
  servedAt?: number;
  deadlineMs?: number;
  agent?: string;
  dna?: string;
  accepted?: boolean;
  acceptedAt?: number;
  electionLink?: string;
  target?: { id: string; head: string; root: string; dna: string };
}
interface Check {
  outcome: string;
  observed: Observation;
}

/** Stage-local deliberately exits pending after saving its exact authored action. */
export function checkClosingStage(
  stage: { code?: number; stdout: string },
  id: string,
  head: string
): void {
  assert.equal(stage.code, 1, 'only the expected local-stage pending exit may advance');
  const prefix = `${id}  reach=commons  action=`;
  const outcomes = stage.stdout.split('\n').filter(line => line.startsWith(prefix));
  assert.deepEqual(
    outcomes,
    [
      `${prefix}pending  head=${head}  (publication pending: authored locally; connected acceptance and declaration required)`,
    ],
    'local stage must report this exact authored head and the known pending reason'
  );
}

/** A successful exit alone cannot prove another native peer or either public doorway. */
export function checkClosingPublication(
  context: ClosingPublicationContext,
  receipt: PublicationReceipt,
  stdout: string,
  startedAt: number,
  previousHead?: string
): { head: string; deadlineMs: number } {
  assert.equal(receipt.id, context.id);
  assert.equal(receipt.agent, context.che);
  assert.equal(receipt.dna, context.dna);
  assert.equal(receipt.execution?.executor, context.che);
  assert.equal(receipt.execution?.humanId, context.humanId);
  assert.ok(Date.parse(receipt.authoredAt) >= startedAt, 'receipt must belong to this update');
  assert.ok(receipt.declaredAt, 'publication must finish declaration');
  assert.ok(receipt.head && receipt.head !== previousHead, 'each update must author a new head');
  const grant = receipt.acceptedDelegation;
  assert.equal(grant?.grantor, context.matthew);
  assert.equal(grant?.delegate, context.che);
  assert.equal(grant?.scope, context.id);
  assert.equal(grant?.rootActionHash, context.root);
  assert.equal(grant?.dnaHash, context.dna);
  assert.equal(grant?.acceptance?.headActionHash, receipt.head);
  assert.ok(grant?.acceptance?.witnessActionHash, 'exact root-author acceptance required');
  assert.ok(grant?.acceptance?.deviceWitnessActionHash, 'exact Human device witness required');
  assert.notEqual(context.adam, context.che);
  assert.notEqual(context.adam, context.matthew);
  assert.equal(new Set(context.doorways).size, 2, 'two distinct doorway origins required');

  const checks: Check[] = stdout
    .split('\n')
    .filter(line => line.startsWith('{'))
    .map(line => JSON.parse(line) as { checkId?: string } & Check)
    .filter(check => check.checkId === 'federation-deploy');
  const native = checks.filter(check => check.observed.agent === context.adam);
  assert.equal(native.length, 1, 'one independent Adam native observation required');
  const proof = native[0];
  assert.equal(proof.outcome, 'passed');
  assert.equal(proof.observed.accepted, true);
  assert.equal(proof.observed.dna, context.dna);
  assert.deepEqual(
    proof.observed.target && {
      id: proof.observed.target.id,
      head: proof.observed.target.head,
      root: proof.observed.target.root,
      dna: proof.observed.target.dna,
    },
    { id: context.id, head: receipt.head, root: context.root, dna: context.dna }
  );
  assert.ok(proof.observed.electionLink, 'Adam must observe the earned native election');
  const deadline = proof.observed.deadlineMs;
  assert.ok(typeof deadline === 'number' && deadline > startedAt);
  assert.ok(deadline <= startedAt + 75_000, 'budget starts with authoring, not observation');
  assert.ok(typeof proof.observed.acceptedAt === 'number' && proof.observed.acceptedAt <= deadline);
  for (const peer of context.doorways) {
    const served = checks.filter(check => check.observed.peer === peer);
    assert.equal(served.length, 1, 'one exact head/body observation per doorway required');
    assert.equal(served[0].outcome, 'passed');
    assert.equal(served[0].observed.id, context.id);
    assert.equal(served[0].observed.head, receipt.head);
    assert.equal(served[0].observed.seedHash, receipt.seedHash);
    assert.equal(
      served[0].observed.deadlineMs,
      deadline,
      'native and public reads share one deadline'
    );
    assert.ok(
      typeof served[0].observed.servedAt === 'number' && served[0].observed.servedAt <= deadline
    );
  }
  return { head: receipt.head, deadlineMs: deadline };
}
