---
id: placement-stories-t2-report
title: "T2 report: placement-contract section 16 stories under the blind-reader loop"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#16
actor: agent:author@claude-opus-5-5
cites: [genesis/docs/superpowers/plans/task-placement-stories-brief.md]
---

# T2 report: placement stories

All seven files parse as Gherkin. None closed: each ran six blind-reader rounds and was still left with findings in round 6. Status is therefore DONE_WITH_CONCERNS.

Per-round counts are correctness/interpretability/preference (c/i/p). R = READY, V = REVISE.

| File | New / extended | Scenarios added (at authoring) | Concern tags (new?) | Rounds | Per-round c/i/p | Parse |
|---|---|---|---|---|---|---|
| genesis/a2o/features/compute/berth-offer.feature | new | 3 (a 4th, the two-blade ranking scenario, was added in round 4) | berth-offer-matched (new) | 6, last V | R 1/5/3 · V 0/6/4 · R 0/3/2 · V 1/4/4 · V 0/5/3 · V 1/4/3 | ok (suite dry run) |
| genesis/a2o/features/compute/measure-on-a-peer.feature | extended | 1 | measure-runs-on-a-peer (existing) | 6, last V | V 1/6/2 · V 0/6/2 · V 0/5/3 · V 0/5/3 · V 0/6/2 · V 1/6/2 | ok (gherkin direct, 7 children) |
| genesis/a2o/features/compute/placement-floor-converges.feature | new | 3 | placement-floor (new) | 6, last R | V 0/5/2 · R 0/3/4 · R 1/3/4 · V 1/5/3 · R 0/6/2 · R 0/3/4 | ok (suite dry run) |
| genesis/a2o/features/compute/placement-tuning-ratified.feature | new | 4 (grew to 7 scenarios through split and two limit scenarios; 8 children with Background) | placement-ceiling (new) | 6, last V | V 1/5/3 · R 1/4/3 · V 2/5/2 · V 0/6/3 · R 0/6/1 · V 0/5/2 | ok (gherkin direct, 8 children) |
| genesis/a2o/features/dataplane/device-role-refusal.feature | new | 4 (grew to 5 when scenario 3 was split) | device-capacity-declared (new) | 6, last R | R 1/3/3 · V 1/3/2 · V 0/4/3 · V 0/4/3 · R 1/4/1 · R 0/3/3 | ok (suite dry run) |
| genesis/a2o/features/compute/peer-executed-stage.feature | extended | 2 | operator-runtime-surface (existing) | 6, last V | V 1/5/1 · R 2/5/1 · R 1/6/1 · V 2/4/2 · V 1/5/3 · V 1/6/2 | ok (suite dry run) |
| genesis/a2o/features/compute/eligible-is-not-enlisted.feature | new | 3 (grew to 4 with the privacy scenario split out) | placement-floor (new, shared with placement-floor-converges) | 6, last R | R 1/4/2 · R 1/4/2 · V 1/4/2 · V 0/2/3 · V 1/3/2 · R 1/3/3 | ok (suite dry run) |

Suite dry run: `npx cucumber-js --dry-run` from genesis/a2o finished with no parse error, 1627 scenarios (884 undefined, 743 skipped). The config loads the whole tree, so this proves the suite parses but gives no count per file. Round 2 of eligible-is-not-enlisted caught a transient parse error in berth-offer.feature (10:3, a tag with whitespace). Later runs show it fixed.

## Findings left for the operator to defer

Open round-6 findings, by file:

- **berth-offer**: what "honoured" distinguishes a holder from (i); "as described below" promises a propagation mechanism that is never described (c); the compound Then on line 96 should be split (i); how "floor controller" relates to "floor" (i); the "targets, not readings taken today" wording (i).
- **measure-on-a-peer**: all findings are on pre-existing text the authors did not own: "household" and "desk" undefined (i); the grant's contents (i); how the lease is acquired (i); scenario 3's title claims a cause its steps do not prove (c); neighbour/provider naming drift (i); "the same long check" (i); scope carried in the scenario 4 comment (i).
- **placement-floor-converges**: the need's identifier is computed in the Background before the need exists (i); "honoured" is used for both a local fact and an observed fact (i); the offer-versus-holdings partition in the scenario 2 Given (i).
- **placement-tuning-ratified**: no actor is named for the forged-vote record (i); withdrawing offered room is never introduced (i); "the test chose" breaks the story voice (i); no way is given to observe the "at no moment" claims (i); the time-target caveat sits far from the steps that use it (i).
- **device-role-refusal**: "the order includes it" crosses the eligibility-only scope (i); what Jessica actually does to make the battery offer (i); Matthew's and James's devices are never set up in the Gherkin (i).
- **peer-executed-stage**: all on pre-existing text: the grant's properties and "one-time" (i); the content-addressing rejection is never tested (c); "economic event" and "admission" undefined (i); two When blocks with unclear state between them (i); write-safety carried only in a comment (i); "recovered" is ambiguous (i); "expiring bytes leased" (i). Also carried from round 4: nothing tests refusal of a mis-scoped, spent or revoked grant, and nothing tests rejection of a tampered stage.
- **eligible-is-not-enlisted**: the offer paragraph should be split so the list-composition rule leads (i); scenario 3 has two When/Then movements (c); no way is given to observe "none of it passes through matthew" (i); "matthew sends no message and does no work" has no time window (i).

## Author's notes

- I staged and committed nothing. The five new files are feature-level `@wip`. Additions to existing files are scenario-level `@wip` and carry a comment naming the slice they wait for: slice 5 for the notarized fulfilment record, slice 6 (task to berth) for the build stage.
- None of the new concern tags (`berth-offer-matched`, `placement-floor`, `placement-ceiling`, `device-capacity-declared`) is named yet by any habit's `checks:` in genesis/manifests/habits.yaml.
- Readings against the spec:
  - device-role-refusal keeps the §16 "holds its own household's bytes when it charges" clause (§7.2, §15) and adds that charging changes are held back so they do not reveal a routine (§7.1).
  - berth-offer has the blade adopt for itself once its own offer comes back, not "the hub re-places" (§7.2 self-candidacy).
  - measure-on-a-peer may already be partly met by the carrier's notarized accept and work events (§5.5).
- The times, counts and the household's 2-of-3 ratification rule are the story's own choices, not values from the spec. Each file says so.
- Outside my files: the `purpose` in genesis/a2o/features/compute/.epr-meta still says the directory holds two feature files. I did not change it because I do not own it.
- Covenant floor: findings on pre-existing text in measure-on-a-peer and peer-executed-stage were left alone under the brief's ownership rule. They belong to the owner of that text.

## Correctness pass (2026-10-20)

Counts are per round: correctness (c) / interpretability (i).

| File | Rounds | Confirming reader | Correctness still open | Parse |
|---|---|---|---|---|
| berth-offer.feature | R1 READY 2c/4i; R2 READY 0c/4i | READY, 0c / 4i | none | PARSED (cucumber-js dry run, no parse error) |
| eligible-is-not-enlisted.feature | R1 REVISE 1c/2i; R2 READY 2c/1i | READY, 2c / 1i | 2 (below) | PARSED (cucumber-js dry run, no parse error) |
| measure-on-a-peer.feature | R1 REVISE 1c/4i; R2 READY 1c/4i | READY, 1c / 4i | 1 (below) | PARSED |
| peer-executed-stage.feature | R1 REVISE 2c/5i; R2 REVISE 1c/7i | REVISE, 1c / 7i | 1 (below) | PARSED (dry run: 1628 scenarios, no syntax failures) |

Open correctness findings, for the operator:

- **eligible-is-not-enlisted**, Background line 73 and scenario 1 line 79: the Background declares matthew eligible ("always on, with room and bandwidth to spare") and scenario 1 then asserts that same conclusion as a Then. The proof is circular: the machine's self-assessment is never shown. Repair: either state the scope boundary (self-assessment is proven elsewhere), or set raw machine facts in the Background and have scenario 1 derive eligibility from them.
- **eligible-is-not-enlisted**, scenario 4 lines 96-101: nothing asserts that matthew's presence message count rises. Scenarios 2 and 6 use that count for the negative case, but the positive case never does. Repair: add a Then saying the count rises during the session.
- **measure-on-a-peer**, line 19: the preamble promises a grant "up to a stated rate", but no scenario tests that the rate is enforced. Repair: add a rate-enforcement scenario, or narrow the preamble's claim and defer rate to a later station.
- **peer-executed-stage**, scenario 1 lines 42-51: a second When at line 49 starts a separate proof (resubmit, no new work) with nothing linking it to the first. Repair: split it into two scenarios that share the Background, or add a bridging Given ("the stage has already completed") before the second When. The text is pre-existing, so it belongs to its owner under the brief's ownership rule.
