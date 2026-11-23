---
epr-habit-version: 1
id: agents-arrive-briefed
invariant: >
  Every agent, at the point where it is invoked, is handed, or reaches in one search, what it is
  for (its goal), what done means (the rung its task climbs to on the done ladder), why the work is
  this way (guide-stars layered from the system's standing purposes through the target's
  governance and covering habit down to rulings on the subject), who decides (authority and bound
  as cited evidence, never as an assertion) and where to look (the target's gate, its open notes,
  the agent's own working record). The brief is shaped to the agent's capability level, named by
  capability and never by a model's name; it is resolved from the target, not from whoever sent the
  agent; and it is a read over records the repository and device already keep, each line carrying
  its source address, with no new store. Isolated seats (the blind reader, the stasis keeper, the
  acceptor while it judges) receive nothing. A successor inherits what was said and never what was
  granted: its authority reads `none held` until its own covenant exists.
status: red
active: false
checks:
  - "a2o @concern:agents-arrive-briefed (genesis/a2o/features/devflow/agent-arrives-briefed.feature — an interactive session, a subagent sent at a target, a valueflow implementer and a shift step each answer the five questions from what they were handed; a successor gets the carried parts and `none held` for authority; isolated seats get nothing; a brief is shaped to its level; a correction left by a poor journey reaches the next agent there; a context reset keeps goal, rung and next action; a runner's brief is read from its grant). @wip: no step definitions yet, red by declaration"
  - ".claude/workflows/recall-standing-reader.js arrival mode — a real subagent, a `claude -p` step and a `codex exec` step launched at a target with a one-line task, what each received captured, the five questions asked and judged by a different tier, the method CID recorded with each result. NOT BUILT YET: the file has no arrival mode"
  - "epr flow report --bound session-open-context-bytes-ceiling (soft 95000, hard 100000 over session-open-context-bytes@1, the declared sum load-project-context.py folds once per session; first ratchet step, to be moved DOWN as root instructions are cut)"
  - "root-instruction size check — pre-push refuses a projected root CLAUDE.md/AGENTS.md that grows past its declared bound. OWED: no such check exists yet"
guard: >
  Regression risks: (1) greening by handing every agent everything — a brief that inlines the
  whole corpus answers the five questions and defeats the habit; the context meter's ceiling is
  the counterweight; (2) a brief that becomes a second authority or a stored record — it is a
  class-C derived read, recomputed from its sources, and a brief that asserts what its sources do
  not say, or is persisted as truth, is a defect; (3) a hook composing its own orientation —
  native `epr` composes, hooks only render; (4) naming levels by model — levels are capability
  names bound per runtime in agent-runtimes.json, and a brief keyed to a model name is stale by
  construction; (5) stamping an isolated seat — the blind reader, stasis keeper and judging
  acceptor must receive nothing, and a test asserts it; (6) counting the appreciation verdict as
  the steward's acceptance — it is testimony under his principal with the seat as provenance,
  never acceptance and never a counted voice.
refs:
  - "genesis/docs/superpowers/plans/2026-11-22-agents-arrive-briefed.md — the plan this habit serves (arrival brief, capability shaping, done ladder, what crosses devices)"
  - "genesis/docs/superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md — designs the `placement` node the brief builds (station, commitment, upwardChain, nearestNeighbours, confidence, recall; never authority)"
  - "genesis/docs/superpowers/specs/2026-11-22-participation-read-design.md — the pulled read an always-on, untasked agent asks for instead of a pushed brief"
  - ".epr-meta/elohim/algorithms/recall-contract.json — the recall contract whose guiding_context and lens_table carry guide-stars and level shaping"
retire-when: >
  when the session runtime carries the brief natively for every agent kind (interactive, subagent,
  valueflow seat, shift step, sponsored runner, always-on) without repository-specific hook
  adapters, and the checks above have been green for a full quarter.
---
DELTA 2026-11-22 (BORN red, not active): declared with its story authored at genesis/a2o/features/devflow/agent-arrives-briefed.feature and blind-read (16 context-isolated rounds in total; the last round reported correctness 0, interpretability 4, preference 8, majors 0; the loop CLOSED: the last reader reported READY). Nothing is built: the feature is @wip with no step definitions, the arrival reader has no arrival mode, and the root-instruction size check does not exist. The context meter's ceiling row (session-open-context-bytes-ceiling, soft 95000, hard 100000) is declared in .claude/epr-meta/measures.yaml and has not been read against a fold here. Owed first: the arrival-reader baseline (one tier, dry run first), then the router landing in root instructions.
