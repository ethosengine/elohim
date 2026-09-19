---
epr-habit-version: 1
id: zome-call-cost-bounded
invariant: >
  What a zome call costs the conductor does not grow with the agent's history. Authorising the
  caller, and any read a coordinator function makes of its own source chain, cost the same at ten
  capability grants or chain entries as at ten thousand: lookups are indexed by what is being
  looked for, chain reads are range-bounded, and nothing on the per-call path loads a collection
  in order to filter it.
status: unwired
active: false
checks: []
first_move: >
  A conductor-fork test that seeds one agent with 10 and then 10 000 capability grants, makes the
  same zome call signed by a registered (non-author) key, and asserts the SQL statement count is
  equal — red today on `valid_cap_grants` (holochain_state/src/dht_store/reads.rs), which loads
  every grant and issues three queries per row. Then the same shape for a coordinator function
  that reads the chain, at two chain lengths.
refs:
  - "genesis/data/timeline/backlog/conductor-cap-grant-scan-per-zome-call.md — measured 2026-09-19: 11 619–18 516 grant rows read per zome call on matthew, ~47 000 SQL queries, 50–60 s per call"
  - "genesis/data/timeline/backlog/conductor-admission-saturated-for-hours-after-restart.md — what it breaks: no head can be authored through either public doorway"
  - "elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs — the ActionSeqRange-bounded chain read that already fixed this pattern once; content_store and imagodei still carry the unbounded one"
retire-when: >
  when the conductor exposes per-call cost (statements, rows read) as a metric and a declared
  ceiling reads it on every peer, so that growth with history is caught by the running system
  rather than by a fixture at two sizes.
---
DELTA 2026-09-19 (declared, unwired): born from the alpha stall. All seven conductors were pinned at their CPU limit while serving nobody; on the genesis pair every storage zome call re-read the agent's entire capability-grant history because the fork's lookup assumes "cap-grant counts are small" and storage authorises a new signing credential on every connect. No check exists yet — the conductor exports no metrics and is not profiled, which is how this grew unseen.
