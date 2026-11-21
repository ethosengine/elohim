

### Row 64, delta 2 (2026-11-21): the second seat is a carrier behind a contract, landed in shadow

Item 2 is built, and not as logic in the push hook. `genesis/orchestrator/gate-carry.mjs` holds a
carrier contract (admit, offer, collect, withdraw), one pure algorithm (`planCarry`) and the first
carrier, `second-seat`. `berth room cargo --json` is the one capacity answer, and the push hook
only calls in. The switch `GATE_CARRY=off|shadow|on` defaults to shadow, which prints what it would
carry and starts nothing (native-dev-tooling-over-peers-plan §9, "Interfaces").

Found while building it:

- Two writers to one gate run's event log could take the same sequence number: four concurrent
  writers gave 481 unique numbers for 601 events, so a cursor could skip an event. Two gates
  finishing together could also lose a row of the cycle ledger. Both now go through
  `genesis/orchestrator/file-lock.mjs`.
- `elohim-storage` and `epr-storage` build in one cargo target, as do `eprfs`, `memory-ceremony`
  and `orchestrator-tree`. Two gates that share a target queue on cargo's own lock, so `planCarry`
  refuses to carry one beside the other. A landing that selects both storage gates therefore
  carries nothing today. The refinement that makes the 21-gate landing pay: the unit carried is
  the group of steps that share a target, run in turn on the second seat (3,535 s and 1,360 s).
- The build hook counts every gate-runner as a seat, including the fixture gate-runners that the
  orchestrator's own hook tests start. A real build was refused for seven minutes by a fixture a
  killed test run had left behind. The tests now reap their fixtures; the count is unchanged and
  stays the safe direction.

Not yet observed: `GATE_CARRY=on` with a real gate. Item 3, the measured landing, is still owed,
and the measurement taken the same day (below, and the sprint journal) bears on it.
