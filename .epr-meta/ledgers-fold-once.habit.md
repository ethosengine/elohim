---
epr-habit-version: 1
id: ledgers-fold-once
invariant: >
  Every dev-system witness — a berth claim or shed, a guard event, a harvested finding, a
  quiesce verdict, a sovereignty landing, a governance decision — is one FlowEvent on one
  SidecarFlowStore per host, read through epr flow report; a second JSONL that a reader other
  than its own appender parses with its own cursor and closure rule is a defect.
status: red
active: false
checks:
  - "a2o @concern:ledgers-fold-once (genesis/a2o/features/devflow/ledgers-fold-once.feature — four kinds of witness land as four folds on one store and the scoreboard derives from the report; @wip @act:host)"
  - "epr flow report --bound ledger-planes-ceiling@1 (hard 1 over ledger-planes@1: .claude/data/*.jsonl and the berth ledger parsed by a script other than their appender; today 7)"
refs:
  - "canon: genesis/docs/content/elohim-protocol/observability/epic.md §7 (the fold is the dev system's one store), §13 ('.epr-meta is not the telemetry store')"
  - "the seven planes today: $CLAUDE_CONFIG_DIR/berth/ledger.jsonl; ram-guard and io-guard events.jsonl; .claude/data/{runtime-findings,ci-findings,quiesce-timeline,sovereignty-guard,governance-findings}.jsonl — each with its own cursor and closure rule; only dev-berth-held-by-measure@1 and sovereignty-landings reach the shared fold"
  - "the emitter: .claude/hooks/_observation.py (already the home of the JSON-accumulator retirement); the rotation question: genesis/data/timeline/backlog/witness-ledger-rotation-is-an-unsolved-design-question.md"
  - "sibling: dev-system-equilibrium (every stock drains at least as fast as it fills); measure-honesty-local (the kinds the folds must carry); the ark's <peer>/ark/.eprfs/status/flows.jsonl is one store per host and not a violation"
retire-when: >
  when the scripts that mint dev findings obtain their writer from epr flow by construction and
  no .claude/data/*.jsonl appender remains.
---
DELTA 2026-10-08 (BORN red): the runtime inventory of 2026-10-08 counts seven accumulated JSONL ledgers with seven cursors and closure rules, of which two reach the shared fold. Tests the first code pass adds: `python3 -m unittest genesis.agentic.berth_test -k fold` (every Ledger.record(kind) emits dev-berth-events@1 through _observation.emit; a missing emitter costs the fold, never the lease) and a ledger-fold test over the harvesters and guards. FIRST MOVE: berth emits on every kind (consolidation order step 6), then the harvesters fold findings instead of keeping their own files. NO status change.
