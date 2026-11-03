// The Rust integration fixtures own setup and behavioral assertions. Cucumber
// exposes the same checks as developer stories without duplicating the evaluator.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

import { Given, When, Then } from '@cucumber/cucumber';

const ROOT = resolve(__dirname, '../../../..');
interface StyleWorld {
  rustStyleFixture?: string;
  rustStyleResult?: ReturnType<typeof spawnSync>;
}

for (const [statement, fixture] of [
  [
    'two isolated repositories hold an addressed guide, full discipline and checker method',
    'addressed_discipline_is_offline_scoped_and_never_auto_upgrades',
  ],
  [
    'a committed discipline artifact is damaged and its working copy is repaired',
    'committed_closure_cannot_borrow_worktree_repair',
  ],
  [
    'a repository holds an incomplete or unsupported discipline method',
    'missing_or_malformed_method_refuses_before_source_evaluation',
  ],
] as const) {
  Given(statement, function (this: StyleWorld) {
    this.rustStyleFixture = fixture;
  });
}

Given(
  'a Rust change with a committed unexplained suppression and an uncommitted repair',
  function (this: StyleWorld) {
    this.rustStyleFixture = 'committed_violation_cannot_be_hidden_by_a_dirty_fix';
  }
);
Given(
  'a Rust crate with two unexplained suppressions and a change repairing one',
  function (this: StyleWorld) {
    this.rustStyleFixture = 'repeat_scans_do_not_demand_repeated_paydown';
  }
);
Given(
  'a Rust crate whose baseline has one legacy suppression and a refactor adding a new unexplained suppression in a strict module',
  function (this: StyleWorld) {
    this.rustStyleFixture = 'strict_module_requires_zero_while_siblings_can_retain_debt';
  }
);
When(
  /^(?:I exercise the Rust style reach gate on isolated Git commits|I check each repository offline and make a successor policy available without adopting it|I check the committed artifacts separately from the repaired working tree|I check the source against its locally held discipline|I check that repair twice, accept it as the integration baseline, and check a later change touching the same crate without further debt reduction|I check the refactor, repair the strict module, and finally repair the legacy suppression)$/,
  { timeout: 300000 },
  function (this: StyleWorld) {
    assert.ok(this.rustStyleFixture);
    this.rustStyleResult = spawnSync(
      // eslint-disable-next-line sonarjs/no-os-command-from-path -- runs the repository's own checker script under the host bash, as CI does
      'bash',
      [resolve(ROOT, 'genesis/orchestrator/rust-style.sh'), '--acceptance', this.rustStyleFixture],
      {
        cwd: ROOT,
        encoding: 'utf8',
        timeout: 290000,
      }
    );
  }
);
function verified(this: StyleWorld) {
  assert.ok(this.rustStyleResult);
  assert.equal(
    this.rustStyleResult.status,
    0,
    `${this.rustStyleResult.stdout}\n${this.rustStyleResult.stderr}`
  );
  assert.match(String(this.rustStyleResult.stdout), /1 passed; 0 failed/);
}
Then('the committed violation is refused while the repaired working tree passes', verified);
Then('both checks of the repair pass and the later change is refused', verified);
Then(
  'their local scopes remain independent and discovering a successor changes neither adoption',
  verified
);
Then(
  'the committed check refuses the damaged artifact while the repaired local check passes',
  verified
);
Then('checking refuses before reporting source compliance', verified);
Then(
  'the gate refuses both incomplete stages and accepts the clean strict module with its crate debt reduced',
  verified
);
