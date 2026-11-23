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

DELTA 2026-11-23 (RED preserved; steps 1 and 2 on dev at 09016b5a99fc, the router drafted): the four session hooks that spoke on every call or every start now speak only when it changes the next action (937b4d31), and the session-open context is metered (`session-open-context-bytes@1`; declared sum 94,531 bytes at the baseline, soft 95,000, hard 100,000). Read live on shem after the main checkout followed dev: `ls elohim/eprfs/epr-cli/src/main.rs` draws no seam-guidance line and no rust-style notice, where the same call drew both before. The five-question router is added to the root instruction package with one duplicated CI pointer merged and nothing else cut, so root CLAUDE.md grows from 72,205 to 76,635 bytes and the declared sum to about 99.6 KB, over soft and under hard; the first section cut is owed next. Verifying the router's commands found two defects this habit depends on: `epr flow context` reads notes by the file's current bytes, so a correction leaves the context read once its subject is edited, and the flow sidecar is per checkout, so a note written in a worktree is not seen from the main checkout. Both are recorded as preconditions of step 4 in the plan. No status change.

DELTA 2026-11-23b (RED preserved; the router on dev at 8fbf7eb1436a, the first cut of the root file made): the 55-line fenced block of `just` and `MESH_*` forms under "Build & Test Commands" (14,690 bytes) moved verbatim into the hc-dev-orchestrator skill, whose description now names the household mesh so an agent about to run a mesh command loads it; a line-by-line check finds 0 of 55 lines missing in the Claude, Codex and Antigravity projections. Root keeps the verbs, a pointer and three traps nothing enforces (the restart arm's transport, heap between hosted lanes, MESH_RESET). Root CLAUDE.md 76,635 to 63,347 bytes; declared session-open sum about 86.3 KB, under its soft bound of 95,000 again. A fresh agent given a mesh task and the first 140 root lines reached the skill from the root pointer and its description and produced a correct start-and-test sequence; what it could not find (the cold-start order, that the commands claim the lease themselves) is now a six-step block at the top of the skill's reference and one root sentence. The move ledger for the whole root file (144 rows, 71.3 KB reviewed, about 11.3 KB to keep) found that an `.epr-meta` rule is not surfaced to Codex and that no deny rule exists for kubectl or the Jenkins trigger tools, so command-triggered rules stay in root. No status change.
