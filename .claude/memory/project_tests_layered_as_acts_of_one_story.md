---
name: project_tests_layered_as_acts_of_one_story
title: Tests layered as acts of one story
description: "Test layers are ACTS of one story — household → neighbourhood → commons, each act a NetworkStage and a substrate; coherence across the suite, not per feature file."
metadata:
  type: project
---

Operator direction (2026-08-21, during the local-mesh layering work): the a2o suite is not "micro vs
Jenkins" — it is ONE story told at three scales, and the unifying story is what gives every scenario
narrative interpretability coherence (a cold reader knows who is in relationship with whom, at what
maturity, hence where it runs and why).

| Act | Cast | Relationship milestone → `NetworkStage` | Substrate |
|---|---|---|---|
| I — the household | matthew, jessica, james (the `just mesh` peers ARE the "Core family — Matthew's household" fixtures); doorway A, then B | device awakens → household forms → first in-family co-steward agreement → `Simulacra → Bootstrap` | local mesh / CI mesh (strict — owns its substrate) |
| II — the neighbourhood | adam's household federates; doorway B becomes adam's (on alpha it is) | cross-household agreement notarized → `Coordinated` | CI mesh for mechanism; alpha fleet for real hosts / churn / global projection |
| III — the commons | community humans, affinity groups, local economy | earned reach across communities → `Enforced` | shem (held by scope until available) |

**Why:** `@requires:shem` was found over-applied ~8:1 — tags were written from where a scenario was
first observed, not from what the story needs ("Core family — Matthew's household" was tagged shem yet
it is the mesh). Act-framing fixes the vocabulary at the root: a scenario declares its act (cast +
stage), the caps fall out, the scope gate casts it.

**How to apply:** one cluster-state per act; the seed chain is the Prologue (household humans, in story
order, as the saga's Givens); stage transitions are REAL moves through the network-stakes manifest leg
(`ALLOW_SEED_NETWORK_STAKES=1` on mesh peers — a preproduction lever, not a prod default); the saga
README's chapter table becomes the three-act table; Jenkins suites = Acts II/III. Kill-a-peer chapters
are Act I only (`processControl` is false on the fleet). Related: [[project_local_pair_failover_validation_rail]],
[[project_freshness_graded_by_declared_stakes]], [[feedback_story_maintainer_atom_perspective]].

**Landed 2026-08-21 (evening):** `@act:` resolution in substrate-scope.ts, `just test mesh [scope]`, the tag plan applied across all 167 features (12 persona swaps, holds to `held/`), `owned-substrate` declared FALSE on the live cluster-state (it failed OPEN before — destructive scenarios would have run against alpha), `A2O_ALLOW_DESTRUCTIVE` as the runtime lever, a `@wip` sweep of 103 unbound scenarios, `layering/wip-inventory.md` (482 @wip Act I/host: 21% bound, 63% partial, 16% none). First Act I inventory: 363 eligible → 106/55/159u/35p.
