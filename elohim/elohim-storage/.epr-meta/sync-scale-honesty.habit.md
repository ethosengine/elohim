---
epr-habit-version: 1
id: sync-scale-honesty
invariant: >
  The sync plane's cost is sub-quadratic and measured: announce-on-change
  (not poll-only), head-diff transfer, healthy request-response to every
  mesh peer, recursion (CoverageRollup) doing cross-scope aggregation.
status: green
active: false
checks:
  - "cargo test --test sync_scale_honesty -- --ignored (elohim/elohim-storage) — 2 scenarios: head-diff transfer + announce-on-change. The `--ignored` is load-bearing: the storage pre-push gate is `just gate` → `just test` → plain `cargo test`, which runs every tests/ target, so a standing red left in the default sweep would wedge the gate for everyone touching this crate. #[ignore] keeps both tests COMPILED on every gate run (no silent rot) and leaves execution to this check. Verified 2026-07-25: explicit run exits 101 (0 passed, 2 failed); default sweep exits 0 (2 ignored)."
guard: >
  Regression risk = a second construction site for the opener. p2p/sync_round.rs
  must remain the ONLY constructor; tests/sync_libp2p_convergence.rs must call
  round_opener rather than hand-rolling a mirror, or the test measures the
  mirror instead of the wire. Watch elohim_sync_in_sync_total: a flat zero in
  a converged mesh means the shortcut never fires and the cure is inert.
refs:
  - "task #9 (sync-timeout peers: 12D3KooWNguL…/GPmV…/Rnj3…) — still unwritten as a health scenario; needs a /health sync-outcome field that does not exist yet, and adding a failing scenario to the edge Dataplane Validation tag set changes what a CI-gated measure counts (operator call, per the blob-durability precedent)"
  - "memory: project_weave_epic_arc (CoverageRollup built, unconsumed)"
retire-when: >
  when sync cost is bounded by a declared measure in the protocol itself — a peer refuses a
  plane whose cost class it cannot afford — rather than by our measuring it after the fact.
---
GREEN 2026-07-27: `cargo test --test sync_scale_honesty -- --ignored` →
2 passed, 0 failed. Cure = round_opener now emits
ListDocumentsSince{corpus_digest} (p2p/sync_round.rs), so a converged
peer answers InSync with one hash and enumerates nothing; divergent
peers fall through to the unchanged ListDocuments path. InSync is an
explicit counted client arm (elohim_sync_in_sync_total) rather than a
catch-all fallthrough.
CORRECTION to the prior evidence block (written 2026-07-25, "0 passed,
2 failed"): the announce leg was already cured before this work — the
send site exists at p2p/mod.rs:3612 and
a_local_change_is_announced_to_connected_peers was ALREADY passing on
2026-07-27. Only the head-diff half was outstanding.
HELD 2026-08-06 across the automerge 0.5.12 → 0.10.0 bump (host-verified):
2 passed, 0 failed on the explicit `-- --ignored` run. The save_after /
load_incremental + ListDocumentsSince{corpus_digest} path survives 0.10
unchanged. Note this check is exactly the class a dependency bump would rot
silently — the default sweep reports it "ignored", proving nothing, so it
must be run explicitly after any automerge move.
HELD 2026-08-08: T5 digest rollout stays default-off and abstains while any distribution-safe row lacks a DHT witness; focused lib filters passed (4 digest + 1 requester-gate + 6 responder-inventory tests, 0 failed).
