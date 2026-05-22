import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  SUCCESSFUL_RESULTS,
  TERMINAL_FAILURE_RESULTS,
  classifyResult,
  isWasted,
  isSuccess,
  isFailure,
} from './pipeline-results.mjs';

test('SUCCESSFUL_RESULTS: SUCCESS + UNSTABLE only', () => {
  assert.deepEqual([...SUCCESSFUL_RESULTS].sort(), ['SUCCESS', 'UNSTABLE']);
});

test('TERMINAL_FAILURE_RESULTS: FAILURE only (ABORTED is waste, not failure)', () => {
  assert.deepEqual([...TERMINAL_FAILURE_RESULTS], ['FAILURE']);
});

test('classifyResult: maps to one of {success, failure, wasted, pending, skipped}', () => {
  assert.equal(classifyResult('SUCCESS'), 'success');
  assert.equal(classifyResult('UNSTABLE'), 'success');
  assert.equal(classifyResult('FAILURE'), 'failure');
  assert.equal(classifyResult('ABORTED'), 'wasted');
  assert.equal(classifyResult('NOT_BUILT'), 'skipped');
  assert.equal(classifyResult(null), 'pending');
  assert.equal(classifyResult(undefined), 'pending');
});

test('isSuccess / isFailure / isWasted: convenience predicates', () => {
  assert.ok(isSuccess('SUCCESS'));
  assert.ok(isSuccess('UNSTABLE'));
  assert.ok(!isSuccess('FAILURE'));
  assert.ok(isFailure('FAILURE'));
  assert.ok(!isFailure('ABORTED'));
  assert.ok(isWasted('ABORTED'));
  assert.ok(!isWasted('NOT_BUILT'));
});
