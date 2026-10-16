---
epr-habit-version: 1
id: operator-dashboard-under-standing
invariant: >
  A doorway's operator surface (the doorway-app dashboard and every /admin route behind it) is a
  view of the network, opened by standing the network records: it answers only a holder of an
  active operate-doorway commitment scoped to THIS doorway's registration, read per request from
  the doorway's peer projection of that notarized commitment; it says which commitment it opened
  under; every acting control checks that commitment for the capability the act needs and names
  it in the response; and no doorway env value, Mongo permission field or sign-in token confers
  standing on its own. Holders manage what the doorway hosts, never which key is a person's: a
  stuck account (key with no installed app on any pool conductor) is shown to the holder and its
  remedy is addressed to the account's owner.
status: red
active: false
checks:
  - "a2o @concern:operator-dashboard-under-standing (genesis/a2o/features/doorway/operator-standing.feature — @act:i; authority is the household lane: `just test mesh features/doorway/operator-standing.feature`; the two @act:ii scenarios (Adam on alpha-B, Matthew as deputy) need the alpha fleet)"
refs:
  - "doorway/doorway-service/src/auth/operator.rs (capability resolver, no production caller yet) · src/server/http.rs op_gate_capability (the per-request gate pattern to reuse)"
  - "elohim/elohim-storage/src/db/rea_commitments.rs find_active_operator_binding · src/services/rea_commitment_service.rs (operate-doorway is diesel-direct today, not conductor-notarized)"
  - "genesis/seeder/src/seed-operator-bindings.ts (binds alpha and apex to Matthew with '*'; Adam unbound)"
  - "genesis/local-dev/doorway-operator-surface/ (census + design receipts, 2026-10-16)"
retire-when: >
  when the doorway-app operator surface is served from the holder's own peer settings surface (the
  operator-runtime-surface product) and the doorway keeps no admin route of its own: standing is
  then checked where the commitment lives, and the doorway only relays.
---
DELTA 2026-10-16b (story authored, status unchanged RED): genesis/a2o/features/doorway/operator-standing.feature,
two tagged Rules (Act I household, 13 pickles; Act II alpha fleet, 2). Blind-reader loop, 10 fresh readers,
per-round correctness/interpretability/preference: 1/4/2 READY, 0/6/3 REVISE, 0/5/3, 1/3/1, 1/4/3, 1/3/2 (real
defect: feature-level @act:i leaked onto Act II pickles; fixed with tagged Rule blocks), 1/4/3, 0/2/2, 0/4/2,
0/4/3, all READY after round 2. Every correctness and interpretability finding through round 10 was resolved in
the text; the loop was stopped by the operator's deferral of further rounds, not by a clean reader.
DELTA 2026-10-16 (born RED on source evidence): /dashboard is gated by sign-in alone
(doorway-app core/guards/auth.guard.ts:37); POST/DELETE /admin/hosted-users, POST
/admin/graduation/force, POST/DELETE /admin/federation/peers and the steward-peers / ssr-bundle /
federation refreshes have no handler-level authorization (Codex census, CONFIRMED at source,
ingress reachability PLAUSIBLE); the remaining admin checks read a scalar Mongo permission level;
the operate-doorway resolver has no production caller and the commitment it reads is written
diesel-direct, so it is not yet a network record.
