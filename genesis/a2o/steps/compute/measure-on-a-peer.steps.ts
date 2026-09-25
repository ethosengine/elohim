/**
 * Measure-on-a-peer — the developer-side verb (`just measure`) that sends a measure-class
 * a2o stage to a household compute provider instead of holding the dev berth. Mirrors the
 * phrasing conventions of steps/compute/peer-executed-stage.steps.ts (the same delegated-
 * compute substrate underlies both) but owns its own state, per the same discipline that
 * file documents: "Own your state ... its phrasings cannot be reused without exporting that
 * map." The Background step text ("a household compute provider with a signed compute
 * grant for this requester") is reused VERBATIM from peer-executed-stage.steps.ts:175-195,
 * which is already registered against that exact text — this file must NOT redefine it
 * (a second Given with identical text would make cucumber's step matching ambiguous). This
 * file registers only the When/Then steps unique to measure-on-a-peer.feature.
 *
 * Sprint plan: /projects/.claude-config/plans/we-ran-into-a-cryptic-robin.md, lane ST. The
 * `just measure` verb (D2), the berth `--class measure` router (D1), the requester-side
 * admission of a peer's signed completion (S2), and the requester-local `compute-fulfilled`
 * projection of the neighbour's realized flow (S3b) are ALL landing in parallel under this
 * same sprint plan and are not wired yet — every step below is a named `pending` stub so the
 * feature COMPILES and reports pending, not undefined. Wiring these against the real `just
 * measure` / berth / admission surfaces is the next station, once those lanes land.
 */
import { Given, Then, When } from '@cucumber/cucumber';

Given(
  "the developer is at their own desk, where ordinary work already holds the household mesh's one lease",
  function () {
    return 'pending';
  }
);

When('the developer tries to start a long check there too', function () {
  return 'pending';
});

Then(
  'the desk refuses the run outright — a long check is never allowed to compete for that lease, held or not',
  function () {
    return 'pending';
  }
);

Then("the refusal names the neighbour's-peer path as where it belongs instead", function () {
  return 'pending';
});

Then('the refusal does not offer to queue the run and wait for the lease', function () {
  return 'pending';
});

Given('a long check the developer wants run', function () {
  return 'pending';
});

When("the developer sends it with one command to the household's compute provider", function () {
  return 'pending';
});

Then(
  'that one command returns to the developer without waiting for the run to finish',
  function () {
    return 'pending';
  }
);

Then("the developer's own desk holds no lease for the run that just left it", function () {
  return 'pending';
});

Given(
  "the household compute provider has run the developer's long check to completion",
  function () {
    return 'pending';
  }
);

When("the developer's own workspace reads back what the provider signed", function () {
  return 'pending';
});

Then('the workspace admits it as household evidence', function () {
  return 'pending';
});

Then(
  'what makes it admissible is the grant naming this developer, not whose key signed the run',
  function () {
    return 'pending';
  }
);

Then(
  'a signed run from a peer who never held that grant would be refused the same read',
  function () {
    return 'pending';
  }
);

When("the developer's workspace reads back the completed run", function () {
  return 'pending';
});

Then(
  "the developer's own record shows the neighbour's capacity as spent, not merely offered",
  function () {
    return 'pending';
  }
);

Then('that record did not exist before this read', function () {
  return 'pending';
});

Given(
  'Adam is provisioned as a household compute provider on a machine of his own, apart from the household mesh',
  function () {
    return 'pending';
  }
);

When('the developer sends the same long check to Adam', function () {
  return 'pending';
});

Then(
  "the developer's desk holds no lease on the household mesh at any point while Adam runs it",
  function () {
    return 'pending';
  }
);

Then(
  "the developer's desk is free to run its own ordinary verification at the same time",
  function () {
    return 'pending';
  }
);
