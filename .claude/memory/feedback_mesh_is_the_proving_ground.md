---
name: feedback_mesh_is_the_proving_ground
title: The local mesh is the proving ground
description: Operator rule 2026-08-21 — drive design and development on the local mesh (`just mesh`, Act I) and prove there; the fleet CONFIRMS delivery, it does not discover. Land the valueflow chain locally first.
metadata:
  type: feedback
---

"This is the place we want to really drive development — prove here that it's going to work on delivery,
and do the design and development locally to land the valueflow chain as best we can." (operator, 2026-08-21,
after the first full a2o inventory run on the mesh surfaced 17 genuine code-reds + 8 seed-reds for free.)

**Why:** the fleet measure costs a 7-pod deploy (~20 min churn + hours of catch-up) and legitimately
no-measures in churn; the mesh owns its substrate (processControl), runs in minutes, and produced the
same class of findings (breaker-shed diagnostics, reach gate, sync producer, seeder swallow) that had been
costing fleet deploys to see. A household run is a real proving ground; the suite had been paying fleet
prices to learn less.

**How to apply:** reproduce on the mesh before touching code; fix with TDD against the mesh; re-measure
the affected scenarios there; only then let a deploy confirm. Dispatch bounded fixes to agents with
disjoint write-sets (doorway / storage-by-module / seeder / a2o-steps); file what is not bounded. A red
that cannot be reproduced on the mesh is either Act II/III by declaration or an env gap in the mesh
Prologue — name which. Related: [[project_tests_layered_as_acts_of_one_story]],
[[project_local_pair_failover_validation_rail]].
