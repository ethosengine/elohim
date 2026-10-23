// The Rust integration fixtures own setup and behavioral assertions. Cucumber
// exposes the same checks as developer stories without duplicating the evaluator.
import { Given, When, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const ROOT = resolve(__dirname, '../../../..');
type StyleWorld = { rustStyleFixture?: string; rustStyleResult?: ReturnType<typeof spawnSync> };

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
  /^(?:I exercise the Rust style reach gate on isolated Git commits|I check that repair twice, accept it as the integration baseline, and check a later change touching the same crate without further debt reduction|I check the refactor, repair the strict module, and finally repair the legacy suppression)$/,
  { timeout: 300000 },
  function (this: StyleWorld) {
    assert.ok(this.rustStyleFixture);
    this.rustStyleResult = spawnSync(
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
  'the gate refuses both incomplete stages and accepts the clean strict module with its crate debt reduced',
  verified
);
