---
epr-habit-version: 1
id: health-attests-on-transition
invariant: >
  A peer's health reaches the chain only as a record that closes a period or marks a
  transition, written by graduation at the manifest's diversity threshold — never by a timer,
  never by one observer about itself, and never while unchanged; the reader of that record
  ships with it; silence after period_end reads unobserved.
status: red
active: false
checks:
  - "a2o @concern:health-attests-on-transition (genesis/a2o/features/doorway/health-attests-on-transition.feature — twenty unchanged minutes write nothing; a return records one transition and the closed period is readable through the attestations reader; graduation refuses a single-household pool naming distinct_households; all @wip; default profile: just test mesh features/doorway/health-attests-on-transition.feature)"
  - "epr flow report --bound period-records-per-peer-day-ceiling@1 (hard 24 = 86400 / the manifest's 3600 s window, over period-records-per-peer-day@1 from a stopped-conductor sniff; reads skipped until folded)"
refs:
  - "canon: genesis/docs/content/elohim-protocol/observability/epic.md §3 (fruit), §7 steps 3–4, §12 rung 4"
  - "rulings: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md §5 rulings 1–4 and §8 rows §5.1–2; records-lifecycle §D.5 hard cutover (evaluator green with its reader before the per-probe writes retire)"
  - "the writer today: doorway/doorway-service/src/services/federation.rs ProbeRoster — every ~5 min probe calls record_health_attestation, which issues attestation:device-health; the Lane 3 attest_decision two-round hysteresis is +383 lines uncommitted in .worktrees/wt-lane3b (gradient sprint 2026-10-08)"
  - "the kind: elohim/sdk/domains/infrastructure/manifest.json infrastructure:doorway-heartbeat (community reach; distinct_households 3, min_count 5; graduates_to attestation:doorway-health-summary; window 3600) and elohim/sdk/schemas/v1/attestation/subtypes/doorway-health-summary-metadata.schema.json (period_start, period_end)"
  - "the bill: idle-is-free (the registration heartbeat and this writer are its two largest idle writers); the review rule: genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md row 16 (capture cannot review itself)"
guard: >
  Regression risks: (1) a scenario that asserts the write and not the read asserts an unread
  record (ruling 3); (2) proving graduation on the household mesh, where distinct_households: 3
  is unreachable by design — the household finish line is zero chain writes plus the health read
  from the observation substrate, and graduation is proven by the evaluator's fixture test;
  (3) node-registry HealthAttestation as the home (ruling 4: no period field, no validation).
retire-when: >
  when idle-is-free is green with the doorway as the only health attester for 30 days and
  get_doorway_attestations is the wired reader — the writer then holds the ceiling and nothing
  watches it from outside.
---
DELTA 2026-10-08 (BORN red): the household idle baseline of the gradient sprint reads attestation:device-health as about one third of ~17,000 lamad actions a day against a 5,000 ceiling (idle-is-free DELTA 2026-10-08), every doorway probe is a DHT write, and `get_doorway_attestations` is a stub. Lane 3's attest_decision (two-round hold; Unchanged writes nothing) is code-complete with 92 tests passing in wt-lane3b and gate-blocked by the disk ceiling. Cargo tests the first code pass adds: `cargo test -p doorway-service --lib services::federation::attest_decision` and `cargo test --lib graduation::evaluator -- doorway_health_summary_requires_distinct_households` (the issuer may not be the sole observer of its own evidence). FIRST MOVE: land Lane 3, then the doorway writes infrastructure:doorway-heartbeat observation rows instead of per-probe attestations, then the evaluator ticks with its reader. NO status change.
