---
epr-habit-version: 1
id: zome-call-cost-bounded
invariant: >
  What a zome call costs the conductor does not grow with the agent's history. Authorising the
  caller, and any read a coordinator function makes of its own source chain, cost the same at ten
  capability grants or chain entries as at ten thousand: lookups are indexed by what is being
  looked for, chain reads are range-bounded, and nothing on the per-call path loads a collection
  in order to filter it.
status: red
active: false
checks:
  - "cargo test -p holochain_state --lib cap_grant (elohim/holochain-conductor, fork branch int/2026-09-23-diagnostics-throttle-perf or later) — cap_grant_lookup_cost_does_not_scale_with_grant_count is the first_move's first half stated as a test: the same secret-keyed lookup costs the same statement count at ten and at many grants; the randomised differential and the fail-closed corrupt-action test hold the new path to the kept reference body"
  - "cargo test -p holochain_state --test chain_query_cost (elohim/holochain-conductor, fork branch int/2026-09-23-diagnostics-throttle-perf or later) — chain_query_range_cost_does_not_scale_with_chain_length: a sequence-range-bounded query(include_entries(true)) issues the same statement count and loads the same 10 action rows and 10 entry rows at 10 and at 2 000 chain actions, and equals the whole-chain-then-filter result; source_chain_query_pushdown_matches_whole_chain_filter holds the new read to the kept reference body over 1 152 filter shapes"
  - "NOT YET WIRED — an entry_type-only chain read (no sequence range) still reads every committed header (Action has no entry_type column); content_store's attention_tending and Content-typed reads are that shape. A check for it needs an indexed column, and until it exists this habit stays red."
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

DELTA 2026-09-23 (unwired → RED: first half is a passing fork test, second half has no check): fork commit 61565f320 on int/2026-09-23-diagnostics-throttle-perf replaces the per-row 3N+ cap-grant lookup with one joined statement per access class; `cargo test -p holochain_state` ran 193/193 on rust 1.96.1 including cap_grant_lookup_cost_does_not_scale_with_grant_count. The household (36 GB fixture, three conductors on that build) authorises zome calls on every role cell. RED, not green: the invariant's other clause — a coordinator's own-chain read bounded at two chain lengths — still has no test, and the retire-when (per-call cost as a running metric) is untouched because the conductor's 28 OpenTelemetry instruments reach no exporter. Fleet confirmation of the cap-grant half waits on the conductor pin moving (backlog: conductor-cap-grant-scan-per-zome-call.md).

DELTA 2026-09-23b (second half wired as a fork test; still RED for the entry_type shape): fork commit cb61633c2 pushes ChainQueryFilter's sequence range, action_type and up to 500 entry_hashes into the chain read (an `elohim_Action_author_seq_idx (author, seq)` index created idempotently after migrations, like the arc index) and loads entries only for surviving actions; ChainQueryFilter still runs in Rust afterwards, so results are identical (differential over 1 152 filter shapes). Measured in the fork: a 10-action range with entries reads 2 statements / 10 action rows / 10 entry rows at both 10 and 2 000 chain actions, where the old read cost 6 statements / 2 003 / 2 001 at 2 000. Two clauses of the invariant now have passing checks (cap-grant lookup; range-bounded chain read). Still red: an entry_type-only read reads every header (no column to index) — content_store carries that shape — and the retire-when is still unmet, though the conductor now EXPORTS per-call cost (fork 0f26f6703: `hc_ribosome_zome_call_duration` and `hc_db_connections_use_time` served at HOLOCHAIN_PROMETHEUS_LISTEN); a declared ceiling reading it on every peer is the remaining step. Coordinator-side follow-on once this pin is on the fleet: node_registry's paginated walk, content_store `get_my_custody_epr_scopes` and imagodei `get_my_household_collective_cids` can drop their headers-then-get_details workaround for a bounded `query(...).include_entries(true)`.
