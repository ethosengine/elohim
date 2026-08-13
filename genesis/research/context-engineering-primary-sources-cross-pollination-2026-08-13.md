---
title: "Context engineering's primary sources — progressive context shaping, and what it means for the EPR's agentic memory"
id: context-engineering-primary-sources-cross-pollination-2026-08-13
status: Capture
date: 2026-08-13
---

# Context Engineering — Primary Sources Behind "Progressive Context Shaping"

**Grading key:** ✅ verified in primary source this pass (canonical page read directly) · ◐ canonical/stable, single-source or mirror-read, not re-derived · ⚠ web-only or inferred.

**Verdict vocabulary:** **TAKE** (mint a cluster row) · **STUDY** (real but needs a design pass before it is mintable) · **WATCH** (a failure mode to monitor, not a build) · **LEAVE** (examined and declined, with the reason).

**Provenance.** Prompted by a video transcript ("Three OpenAI Engineers Shipped A Million Lines…") that argues for *progressive context shaping* but cites everything by paraphrase and points at a paywalled starter kit. This pass traces every load-bearing claim in that transcript to its primary source, catalogues the repositories implementing the strategies, and does a first bridge to our own agentic-memory layer. This is **leg 2 of 4** in the operator's stated arc (VSM → context engineering → EPR governance → dev disciplines); the synthesis was delivered in-session on 2026-08-13 and the arc closed with the mint pass recorded in §6.

---

## 0. The one-paragraph version

The transcript's thesis is sound and its sources are real: several independent teams (OpenAI Codex, Anthropic Research, Anthropic Engineering, Arize) converged in ~6 months on one move — **stop treating the conversation transcript as the agent's state; maintain a durable state object that outranks it**. But the transcript stops at the single-agent framing, and the interesting layer is the one it skips. Two sources it never cites carry the mechanism: Arize's **PlanMessage** (the plan is re-derived from durable state and re-injected at a fixed position immediately after the system prompt on *every* model call — never remembered from history) and Anthropic's **managed-agents session log** (the window is a projection of a durable append-only event log, so **compaction and handoff are the same operation** — two slices of one log, and the log survives both). That is projection-over-memory, the same inversion our `plant-eprfs-*` family already applies to skills, agents, hooks and commands. We applied it to *tooling artifacts*; the field applied it to *run state*; neither side has done both. Meanwhile the workflow object every team improvised — Linear tickets, a feature-list JSON, lock files in `current_tasks/`, an in-RAM todo list — is a container-bound version of what we already run as **REA valueflows over EPRs**: 556 active commitments in a `commitment:claim:` state machine, recipes as ProcessSpecifications with economically meaningful edges, admission-controlled by `habits.yaml`'s WIP fence. Our representation is richer and peer-native; our *loop* is poorer — Symphony reconciles its ticket state around every turn (and only Arize re-injects state into every model call), we render ours once at session start. The gap is projective, not representational. The transcript also **misreports** the Anthropic rebuild (§1.4): they dropped the sprint construct *and* the context resets, keeping planner and evaluator — so the real lesson is that **harness scaffolding depreciates and must be stress-tested for load-bearingness, not accumulated**, which is our own "instrument with no reader" failure mode stated by an outside party.

---

## 1. Claim-by-claim source trace

Every substantive claim in the transcript, with its primary source and confidence grade.

### 1.1 "Three OpenAI engineers, ~1,500 PRs, >1M lines, no human-typed code"

**Source:** OpenAI, ["Harness engineering: leveraging Codex in an agent-first world"](https://openai.com/index/harness-engineering/) (Feb 2026).

- ✅ Three engineers growing to seven; ~1,500 PRs merged over five months; on the order of one million lines (the transcript's "over a million" slightly overstates — the post says ~1M and estimates ~1/10 the hand-written time); first commit into an empty repo late Aug 2025 with Codex CLI + GPT-5; ~3.5 PRs/engineer/day, and throughput *rose* as the team grew. Zero manually written lines.
- ✅ **`AGENTS.md` as a table of contents, not an encyclopedia** — ~100 lines, progressive disclosure into `docs/design-docs/`, `docs/product-specs/`, `docs/exec-plans/`, `docs/references/`. The stated failure mode is exactly the video's: "context is scarce, and a bloated instruction file crowded out the actual task."
- ✅ **Doc-gardening**: scheduled background Codex tasks scan the repo for outdated content and open cleanup PRs; linters and CI verify cross-links stay intact. Principle: *"if something isn't in context at runtime, it doesn't exist for the agent."*
- ✅ Architecture enforced *mechanically*, not by prose: strict dependency direction (Types → Config → Repo → Service → Runtime → UI) with custom linters and structural tests.
- ✅ **Quality grades are real, per product domain and architectural layer** — a quality document grades each declared unit, gaps tracked over time; background Codex tasks update the grades and open targeted refactor PRs. (The video's "every part of the codebase" is a fair paraphrase only when narrowed to those declared grading units.)
- ✅ **"Graveyard of stale rules" is verbatim** — the phrase appears in the primary, describing what a monolithic instruction manual becomes.
- ⚠ One boundary the post does *not* cross: `AGENTS.md` is startup instruction context plus tool-driven retrieval during the run — it is **not** per-call state projection. Reserve that mechanism claim for Arize (§1.5).

> **Method note.** The initial pass hit HTTP 403 on `openai.com` and read this section through mirrors ([alexlavaee](https://alexlavaee.me/blog/openai-agent-first-codebase-learnings/), [Milvus](https://milvus.io/blog/harness-engineering-ai-agents.md), [SWE Quiz](https://www.swequiz.com/articles/openai-harness-engineering), [InfoQ](https://www.infoq.com/news/2026/02/openai-harness-engineering-codex)). A Codex pass the same day read the primary directly and confirmed every claim above, upgrading the grades — including the two the mirror pass had left ⚠ (the quality grades and the "graveyard" phrasing).

### 1.2 "Anthropic uses a progress file as portable memory in long-running scientific computing"

**Source:** Anthropic, ["Long-running Claude for scientific computing"](https://www.anthropic.com/research/long-running-Claude). ✅ **This is the transcript's single most accurate citation.**

- ✅ The progress file is conventionally `CHANGELOG.md` — "portable long-term memory, acting as a sort of lab notes."
- ✅ It records: current status · completed tasks · **failed approaches and why they failed** · accuracy tables at checkpoints · known limitations. Rationale stated outright: without the dead ends, "successive sessions will re-attempt the same dead ends."
- ✅ The video's solver anecdote is verbatim from the source: *"Tried using Tsit5 for the perturbation ODE, system is too stiff. Switched to Kvaerno5."*
- ✅ Division of labor between the two files: `CLAUDE.md` at project root carries high-level objectives, design decisions, and a **quantified success criterion** ("0.1% accuracy against CLASS"); `CHANGELOG.md` carries the moving state. Fresh sessions are launched with "Read CHANGELOG.md and pick up the next task."
- ✅ Anti-laziness scaffold: a Ralph-loop plugin invoked with `--max-iterations 20 --completion-promise "DONE"` to counter premature completion claims. Execution: SLURM job → `tmux` on a compute node → detach, monitor via GitHub/SSH.
- ✅ **Real artifacts to read**, not just prose: [`smsharma/clax` CLAUDE.md](https://github.com/smsharma/clax/blob/6a6b2330cf25edded1bb31ec57a0091aa794a5d3/CLAUDE.md) · [CHANGELOG.md](https://github.com/smsharma/clax/blob/main/CHANGELOG.md) · [commit history](https://github.com/smsharma/clax/commits/main/).

### 1.3 "Anthropic's long-running coding harness: progress file, structured handoffs, version history"

**Source:** Anthropic, ["Effective harnesses for long-running agents"](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) (2025-11-26).

- ✅ **Initializer agent** produces three things on session one: `init.sh` (run dev servers), `claude-progress.txt` (the progress file), and an initial git commit as baseline.
- ✅ **Feature-list JSON** expanding the user's spec into granular testable features, *all initially marked "failing"* — so every later agent inherits an explicit outline of what full functionality means. (This is a definition-of-done that is machine-checkable rather than narrated.)
- ✅ **Fresh-session sequence:** verify working dir → read progress file + git log → consult feature list → run baseline tests via `init.sh` → implement **exactly one** unfinished feature → commit with descriptive message.
- ✅ Runnable implementation: [`anthropics/claude-quickstarts/autonomous-coding`](https://github.com/anthropics/claude-quickstarts/tree/main/autonomous-coding).

### 1.4 "Four months later Anthropic rebuilt the harness, dropped sprint scaffolding, kept structured handoffs" — **partly wrong**

**Source:** Anthropic, ["Harness design for long-running application development"](https://www.anthropic.com/engineering/harness-design-long-running-apps) (2026-03-24 — ~4 months after 1.3, so the video's timeline is right).

- ✅ Three roles: **Planner** (expands a 1-4 sentence prompt into a 10+ feature spec, deliberately *avoiding* granular implementation detail so spec errors don't cascade) · **Generator** · **Evaluator** (drives the live app via Playwright MCP, grades against criteria, returns findings with file/line locations).
- ✅ The **sprint construct was removed** on the move to Opus 4.6. Sprints had decomposed work into coherent chunks; the improved model "plans more carefully, sustains agentic tasks for longer" and no longer needed the decomposition. **Planner and evaluator were kept** because each stayed load-bearing.
- ⚠→❌ **The video's correction.** It claims they "kept the structured handoff between sessions because that still needed to happen." The primary says the reverse: **context resets** (clearing the window and handing off to a fresh instance) were a workaround for Claude Sonnet 4.5's *"context anxiety"* — prematurely wrapping up as the limit approached. With Opus 4.6 the harness moved to **one continuous session across the whole build**, with the Agent SDK's automatic compaction absorbing context growth. What survived was file-based inter-agent artifacts (one agent writes a file, another reads and responds in or beside it), not session handoffs.
- ✅ The real stated moral, and it cuts against the video's framing: harness complexity should be **stress-tested as models improve**, with components retained *only when load-bearing*.
- ◐ Cost datapoints worth keeping for our own agentic-developer economics: retro game maker, Opus 4.5 + full harness, 6h / $200 (solo baseline 20 min / $9); DAW app, Opus 4.6 + simplified harness, 3h50m / $124.70.

### 1.5 "Arize's agent Alex made 27 model calls reorganizing its own to-do list"

**Sources:** Arize, ["How to build planning into your agent"](https://arize.com/blog/how-to-build-planning-into-your-agent/) and ["Context management in agent harnesses"](https://arize.com/blog/context-management-in-agent-harnesses/). (The agent is **Alyx**, not "Alex".)

This is the mechanism-bearing source, and the video undersells it.

- ✅ Failure shape: after ~5 tool calls the original objective was buried under tool output — *"Alyx would return a beautifully sorted table… and then call `finish`. Two thirds of the request just evaporated."* Diagnosed as an **attention** problem, not a capability or hallucination problem.
- ✅ What did **not** work: hardcoded sequences (too rigid) and prompt-based planning (unstructured text that can't be persisted or enforced). Benchmark-validated techniques (Plan-and-Solve, Reflexion) failed under production tool-output volume — *"prompting alone wasn't enough."*
- ✅ **Four task statuses**: `pending` / `in_progress` / `completed` / `blocked`. Adding `in_progress` was called out as decisive: *"a working pointer, a concrete anchor for 'what am I doing right now?'"*
- ✅ **Three tools**, not prose instructions: `todo_write` · `todo_update` · `todo_read`.
- ✅ **The PlanMessage** — the core move. The plan is lifted out of conversation history entirely and **rebuilt from memory state on every loop iteration**, injected at a fixed position:
  ```
  [System prompt] → [PlanMessage] → [Session history] → [Current turn]
  ```
  with `[x] / [~] / [ ]` visual markers. It cannot drift deeper into context as tokens accumulate, because it is never *in* the accumulating region.
- ✅ **Hard gate**: the `finish` tool refuses to terminate while tasks are `pending`/`in_progress`, erroring with the incomplete list and forcing explicit completion or a `blocked` designation.
- ✅ Emergent effect: Alyx began chaining prompts on its own — a feature the team had planned to build by hand.
- ◐ The companion post carries concrete harness numbers worth having: Claude Code persists oversized tool results to disk and replaces them with 2KB previews (per-tool cap 50,000 chars, per-message 200,000); post-compaction it re-attaches up to 5 recently-read files within a token budget; 13,000-token buffer triggers compaction; Arize's own conversation checkpoint is 50,000 tokens.

### 1.6 "OpenAI Symphony: project board as control plane; 3-5 sessions; 500% PR lift"

**Sources:** OpenAI, ["An open-source spec for Codex orchestration: Symphony"](https://openai.com/index/open-source-codex-orchestration-symphony/) · [`openai/symphony` SPEC.md](https://github.com/openai/symphony/blob/main/SPEC.md).

- ✅ **The 3-5 ceiling is directly stated in the official post**: "most people could comfortably manage three to five sessions at a time before context switching became painful" — forgetting which session was doing what, jumping between terminals. Corroborated by OpenAI engineer Alex Kotliarskyi: "we can supervise about 3-5 coding agents. After that productivity drops… we built Symphony to remove that ceiling."
- ✅ **The 500% landed-PR result is in the official post directly** ("a 500% increase in landed pull requests on some teams"). The secondary "sixfold in three weeks" formulation is incompatible with it and is dropped.
- ✅ **The spec is the interesting part** (Elixir/OTP, BEAM, dozens of parallel agents per node):
  - Issues are normalized across trackers with an orchestrator claim state: `Unclaimed → Claimed → Running → RetryQueued → Released`. Note the type: this is **ephemeral scheduler state** — anti-duplication reservations owned by one orchestrator, not restored on restart (the service reconstructs by re-polling the tracker and reusing workspaces). See §4.1a for why this matters against our commitments.
  - **`WORKFLOW.md` is repository-owned, hot-reloaded, and carries the per-issue *first-turn* prompt template**, rendered with the normalized `issue` object plus an `attempt` integer. **Two loops, not one:** the *outer* orchestration loop polls the tracker (default 30s), reconciles, and re-checks issue state after every turn (up to `agent.max_turns`, default 20, on one live thread); the *model-visible* loop sends the full rendered prompt on the first turn only — continuation turns carry guidance, not a re-rendered state block, and `WORKFLOW.md` edits apply to future launches, not in-flight sessions. Direction changes reach the next agent action by editing the ticket at run/turn boundaries — **not** by per-call injection. In this corpus that mechanism exists only in Arize's PlanMessage.
  - Per-issue isolated workspace, path-confined. Tracker secrets stay host-side; the agent never gets adapter credentials.
  - **"A successful run can end at a workflow-defined handoff state (for example `Human Review`), not necessarily `Done`."** — a first-class **successful handoff boundary**. (Not a terminal state: by default `Human Review` is not in the spec's terminal list — moving there stops work without terminal-state cleanup.)

### 1.7 "~400,000 Claude Code sessions: 70% planning / 80% execution"

**Source:** Anthropic, ["How Claude Code is used in practice"](https://www.anthropic.com/research/claude-code-expertise).

- ◐ ~400,000 sessions, Oct 2025 – Apr 2026, privacy-preserving analysis. Decisions split into **planning** (*what* to build, what counts as done) and **execution** (*how* — which files, what code, which commands). Users made ~70% of planning decisions; Claude ~80% of execution decisions.
- ◐ Shape of a session: ~4 turns; each user prompt sets off ~10 Claude actions on average, sometimes >100; ~2,400 words of output per turn.
- ◐ The finding the video omits and which matters more to us: **domain expertise, not coding background, is what raises yield** — expert sessions verified-succeed at 28-33% vs ~15% for novice sessions. The lever is the quality of the planning decisions, which is precisely the artifact class this whole survey is about.

### 1.8 The durable session log — the corpus's best answer to "handoffs and compaction, consistently"

**Source:** Anthropic, ["Scaling managed agents"](https://www.anthropic.com/engineering/managed-agents) (2026-04-08). ✅ Not cited in the transcript at all, and it is the most important source for the inter-agent question.

- ✅ **The session log lives outside the harness.** Interface: `getSession(id)` returns the event log, `emitEvent(id, event)` appends durably. Consequence stated plainly: *"the session log sits outside the harness, nothing in the harness needs to survive a crash."* A dead harness is rebooted with `wake(sessionId)`, re-reads the log, and resumes from the last event.
- ✅ **The session is "a context object that lives outside Claude's context window."** `getEvents()` lets the harness pick up where it last stopped reading, **rewind a few events before a moment to see the lead-up**, or re-read context before a specific action.
- ✅ **Compaction is a projection-time transform, not a destructive edit.** *"Context can be transformed in the harness before being passed to Claude's context window"* — while the original event records persist in the session. Compaction never loses the log; it only changes what this turn sees.
- ✅ **"Brains can pass hands to one another"** — the sandbox/tool interface (`execute(name, input) → string`) is interchangeable, so an agent can delegate tool access without coupling to a specific sandbox.

This is the piece that makes handoff and compaction *the same mechanism*: both are re-projections of a durable append-only log, differing only in which slice they take.

### 1.9 What subagents are actually given — and when not to use them at all

**Sources:** Anthropic, ["How we built our multi-agent research system"](https://www.anthropic.com/engineering/multi-agent-research-system) · Claude, ["When to use multi-agent systems (and when not to)"](https://claude.com/blog/building-multi-agent-systems-when-and-how-to-use-them).

- ✅ **The isolation boundary is the whole design decision.** For research the answer is "almost nothing": each subagent gets a **fresh context window** plus four things — objective, output format, tool/source guidance, and explicit **task boundaries**. Lead is Opus, subagents Sonnet; the arrangement beat single-agent Opus by 90.2% on their internal research eval.
- ✅ **Subagents are compression devices.** They "operate in parallel with their own context windows… before condensing the most important tokens for the lead." The customer-support example is the clearest statement of the economics: the order-lookup agent reads the full order history and the main agent receives *only the 50-100 tokens it needs*.
- ✅ **The lead saves its plan to Memory** — explicitly *"to persist the context, since if the context window exceeds 200,000 tokens it will be truncated."* Independent rediscovery of Arize's PlanMessage, arrived at from the opposite direction (durability rather than attention).
- ✅ **Coordination failure modes, named:** over-spawning (50 subagents for a simple query); duplication and gaps when divisions are unclear; and — the root cause — vague instructions, where *"subagents misinterpreted the task or performed the exact same searches as other agents."*
- ✅ **The read/write asymmetry, and it is a warning for us.** Multi-agent excels at read-heavy work and struggles with write-heavy work. The guidance explicitly warns against **dividing by type of work** ("one agent writes features, another writes tests, a third reviews code") because *"each handoff loses context"* and teams observed agents *"spent more tokens on coordination than on actual work."* Split only when context can be **truly isolated** — independent research paths, separate components with clean interfaces — never sequential phases of the same work.

> This corroborates our own `[[feedback-subagent-disjointness-read-write]]` from an outside source: disjointness is a property of read-set ∩ write-set, not of task labels.

### 1.10 Leaderless coordination — the one public example

**Source:** Anthropic, ["Building a C compiler with a team of parallel Claudes"](https://www.anthropic.com/engineering/building-c-compiler) (2026-02-05) · [`anthropics/claudes-c-compiler`](https://github.com/anthropics/claudes-c-compiler).

- ◐ 16 parallel Claudes, ~2,000 sessions, ~two weeks, **no central orchestrator**: *"I leave it up to each Claude agent to decide how to act. In most cases, Claude picks up the 'next most obvious' problem."*
- ◐ Coordination is **stigmergic** — through the shared filesystem, not through messages. An agent claims a task by **writing a lock file into `current_tasks/`**; the cycle is pull upstream → merge others' changes → push → remove the lock. Progress lives in frequently-updated READMEs and progress files, plus a running doc of failed approaches and remaining tasks.
- ◐ The lock history is readable in git — you can watch agents take and release tasks.

This is the only public precedent for *many agents sharing durable state without a coordinator*, and it is the closest existing analog to our shared-worktree, multi-session reality.

---

## 2. Repositories implementing these strategies

Ranked by how much a reader of ours would learn per hour.

| Repo | What it actually implements | Why it matters here |
|---|---|---|
| [`smsharma/clax`](https://github.com/smsharma/clax) | A **real** `CLAUDE.md` + `CHANGELOG.md` pair from the Anthropic science run, with full commit history | The cleanest existing template for a progress file with failed-approaches-and-why. Read this before designing ours. |
| [`anthropics/claude-quickstarts` → `autonomous-coding`](https://github.com/anthropics/claude-quickstarts/tree/main/autonomous-coding) | Initializer + `claude-progress.txt` + all-failing feature-list JSON + fresh-session loop | The runnable version of §1.3. Machine-checkable definition-of-done. |
| [`openai/symphony`](https://github.com/openai/symphony) | Ticket-as-control-plane; `WORKFLOW.md` as per-issue prompt template; claim-state machine; per-issue workspaces | Direct prior art for "the backlog *is* the current-state artifact" — see §5 STUDY-5, with the §4.1a plane-typing caveat. |
| [`humanlayer/advanced-context-engineering-for-coding-agents`](https://github.com/humanlayer/advanced-context-engineering-for-coding-agents) (`ace-fca.md`) | **Frequent Intentional Compaction**: research → plan → implement, each step handing the next *only* the context it needs; target 40-60% window utilization | The most methodologically explicit public treatment. ~2.5k stars. The 40-60% utilization target is a falsifiable operating number we don't have an equivalent of. |
| [`anthropics/claudes-c-compiler`](https://github.com/anthropics/claudes-c-compiler) | 16 parallel Claudes, ~2,000 sessions, task claiming by **writing a lock file into `current_tasks/`**, pull→merge→push→unlock | The only public example of *many* agents sharing durable state without a central orchestrator — nearest analog to our shared-worktree, multi-session reality. |
| [`github/spec-kit`](https://github.com/github/spec-kit) | Spec-driven development toolkit | The "spec is the assignment" school; adjacent to our story-first default. |
| [`agentsmd/agents.md`](https://github.com/agentsmd/agents.md) + [issue #135](https://github.com/agentsmd/agents.md/issues/135) | The open `AGENTS.md` format; #135 proposes YAML frontmatter for **progressive disclosure** so harnesses can index guidance without loading it | The standards-track version of the "map, not encyclopedia" pattern. Watch #135 — it is the same problem our `.epr-meta` cascade solves. |
| [`gastownhall/beads`](https://github.com/gastownhall/beads) | Agent-optimized issue tracker storing work as a **dependency-aware graph** | Closest public shape to our backlog/valueflow as an agent-readable structure. |
| [`Picrew/awesome-agent-harness`](https://github.com/Picrew/awesome-agent-harness) | Catalog of harness/context/handoff projects | The index to re-scan next quarter rather than re-searching. |
| [`The-Swarm-Corporation/AdvancedResearch`](https://github.com/The-Swarm-Corporation/AdvancedResearch) | Open implementation of the orchestrator-worker pattern from Anthropic's multi-agent research post | The only public code for the "silence" topology (§3.1) — read it to see what isolation boundaries look like when written down. |
| [`hoangnb24/repository-harness`](https://github.com/hoangnb24/repository-harness) | Turns a repo agent-ready: compact entrypoint, repo map, durable plans only when needed, explicit judgment boundaries, mechanical validation | A packaged opinion on exactly the four-layer split — useful as a foil. |

*(Already catalogued in the 2026-05-14 horizon scan and not repeated here: `letta-ai/letta` memory blocks, `mem0`, MemPalace.)*

---

## 3. What the corpus actually converged on

Stripping the vocabulary differences, the convergence is narrow and specific:

1. **The transcript is not the state.** Every team stopped letting accumulated history be the agent's model of the job. (OpenAI: docs tree over instruction file. Anthropic: progress file / session log over session memory. Arize: PlanMessage over conversation.)
2. **State is re-derived, not remembered.** Arize states it as a mechanism and is the only team in the corpus that implements it *per model call*; Symphony approximates it at run/turn boundaries (full rendered issue prompt on the first turn, tracker-state reconciliation between turns); managed agents generalize it (the window is a *projection* of the event log); Anthropic's fresh-session sequence is the manual version.
3. **Position beats emphasis.** Pin the plan immediately after the system prompt. This replaced the instinct to write a more forceful sentence, and it is the most portable single tactic in the corpus.
4. **Compaction and handoff are the same operation.** Both are re-projections of a durable log — differing only in which slice they take. This is the corpus's most useful structural idea and the transcript misses it entirely.
5. **Failed approaches are load-bearing state.** Only the science post treats this as a first-class field. Cheapest high-value item here.
6. **Completion needs a mechanical gate.** Arize's `finish` refusing on pending tasks; the all-failing feature list; the Ralph loop's `--completion-promise`. Three independent inventions of "an agent may not declare done by assertion."
7. **Coordination is expensive and should be minimized, not maximized.** The isolation boundary — what one agent must know about another — is *the* multi-agent design decision, and the published answer for read-heavy work is "almost nothing." Splitting by *type of work* on a shared write surface is named as an anti-pattern.
8. **Documentation rot is a scheduled job, not a virtue.** Doc-gardening agents opening cleanup PRs is the only automated answer to staleness in the corpus.
9. **Scaffolding depreciates.** The one longitudinal datapoint says: delete scaffolding when the model outgrows it.

### 3.1 Three shapes of "agent radio" — and which one is ours

The corpus contains exactly three inter-agent communication topologies, and they are not interchangeable:

| Shape | Mechanism | Where it works | Where it fails |
|---|---|---|---|
| **Silence (orchestrator-worker)** | Subagents know nothing of each other; lead fans out and synthesizes | Read-heavy, parallel, independent paths | Any shared write surface — "each handoff loses context" |
| **Hub (ticket as sole channel)** | Direction changes reach the next agent action *only* by editing durable work state; the orchestrator reconciles that state around every turn (the model re-reads it at run/turn boundaries, not per call) | Long-horizon write work, many agents, human in loop | Needs a real work-state substrate; Symphony had to borrow Linear's |
| **Stigmergy (environment as channel)** | Claim by writing a lock into the shared tree; pull→merge→push→unlock | Leaderless, many agents, one repo | Silent collision when the claim granularity is wrong |

The published finding that matters most to us: **multi-agent is a read-heavy technique**, and the explicit warning is against dividing write work by role on a shared surface. Our reality — concurrent sessions co-committing in one worktree, write-heavy, long-horizon — puts us squarely outside the "silence" family and inside **hub + stigmergy**. Which is precisely the pair of organs we already have and have never named as such (§4.3).

---

## 4. The bridge — where this lands against our layers

### 4.1 The four context types, mapped to what we already have

The transcript's taxonomy (stable instruction · current project state · map · history) is a useful audit grid. Against our tree:

| Type | Our implementation | State |
|---|---|---|
| **Stable instruction** | `CLAUDE.md` gospel tier; `.epr-meta` cascading compose-gates; skills | Strong for Claude — and `.epr-meta`'s `class:` ladder (ask/inject/measure) is *finer-grained than anything in the corpus*. But **measurably broken for Codex**: the generated `AGENTS.md` projection (42,327 B) exceeds Codex's 32 KiB combined instruction budget and truncates at line 308 (STUDY-7). |
| **Map** | `MAP.md`, the seam map / concern-routing atlas, `MEMORY.md` index, `cites:` content-addressed links | Strong. Our `cites:` fingerprints survive file moves, which `AGENTS.md` progressive disclosure (issue #135) is only now proposing. |
| **History** | `genesis/data/timeline/` chronicle, museum records, sprint-results, git, MemPalace | Strong, and the corpus has no equivalent of the museum's frequency-ranked recurrence records. |
| **Current project state** | `habits.yaml` + SessionStart headline + the active `/shift` Objective | **The weak leg.** |

**The gap, stated precisely.** We have durable state (`habits.yaml`, roadmap, backlog, valueflow commitments) and we have a *rendering* of it at session start. What we do not have is the Arize invariant: **a small current-state block re-derived from that durable state and re-injected at a fixed position on every turn, outranking the accumulated transcript.** Our SessionStart headline is a frozen packet — exactly the failure the transcript names — and a mid-run correction from the operator lands in conversation, where it decays with distance and dies at the session boundary.

Note this is *not* an argument for a `current.md`. We have the durable state already; what's missing is the projection.

### 4.1a REA valueflows over EPRs *are* our platform-agnostic workflow layer

This is the operator's framing and it is the correct one — it names what the field is reaching for and cannot reach.

Every team in the corpus needed a durable representation of "what work exists, who holds it, what state it's in, and what has to be true for it to be done." Each reached for the nearest container: OpenAI took **Linear tickets**; Anthropic took a **feature-list JSON** with every entry pre-marked failing; the C-compiler team took **lock files in `current_tasks/`**; Arize took an in-memory **todo list with four statuses**. All four are workflow representations invented under time pressure, and all four are bound to their container — a vendor's board, a file format, a filesystem convention, a process's RAM.

We already have the substrate version of that object, and it is live (`epr flow status`, verified this pass):

```
resources labeled: 5245 · flow events: 343 · intents: 3667
active commitments: 556 · unfulfilled: 539
edges: 410 sealed · 0 governed · 108 stale · 0 held · 0 dangling
top unfulfilled: commitment:claim:specs__…-design#1 [gap:claimed, …]
```

Read against Symphony's spec, the correspondence is **structurally suggestive but typed at different planes** — and the plane difference is itself a finding:

| Symphony (ticket-bound) | Ours (REA over EPRs) | Correspondence |
|---|---|---|
| Issue normalized across trackers — the durable work object | **Intent** projected from the repo (3,667) | ◐ analogous |
| Claim state `Unclaimed → Claimed → Running → RetryQueued → Released` — **ephemeral scheduler reservations**, lost on restart | **`commitment:claim:<gap>`** — a durable REA `Commitment` (`Proposed → Active → Fulfilled/Revoked`), fulfillment carried by economic events (556 active / 539 unfulfilled) | ⚠ different semantics: an anti-duplication reservation vs an economic promise |
| `WORKFLOW.md` — policy + first-turn prompt template + runtime settings, hot-reloaded | **`recipes.yaml` ProcessSpecification** — stages + `meaningful: true` edges | ◐ analogous policy layer; neither substitutes for the other |
| Poll (30s) → reconcile → dispatch loop; issue state re-checked after every turn | *(no puller — see STUDY-5)* | ✅ the actual gap |
| `Human Review` as a workflow-defined **successful handoff boundary** | **Sealed contract edges** + the operator gate in the ceremonies | ◐ analogous |
| Tracker + workspaces survive orchestrator restart | **Content-addressed flow records, provenance-sealed, peer-native** | ◐ both durable, at different layers |

*(REA lifecycle verified in source this pass: `epr flow project` mints a commitment only from a textually `CLAIMED` decomposed item and stores it `Active` — `elohim/eprfs/epr-cli/src/flow/project.rs:526`; the lifecycle enum and event-carried fulfillment — `elohim/epr-rea/src/model.rs:90`; `epr flow status` counts but does not dispatch — `elohim/eprfs/epr-cli/src/flow/walk.rs:405`.)*

Three differences are ours and are real advantages, worth stating because they are not merely aesthetic:

1. **The recipe is a ProcessSpecification, not a prompt template.** Symphony's `WORKFLOW.md` renders prose for a model; our recipe declares stages and *economically meaningful edges* where events are expected. That makes the workflow **checkable** rather than merely instructive — a walk can tell you an edge is stale (108 of them right now), which no prompt template can do.
2. **Commitments carry provenance, not just status.** A Linear ticket's history is a vendor's audit log. Ours is the first hop of a chain (research → cluster row → spec → code + scenario → chronicle) over content-addressed records, so a claim can be *walked back to why it exists*. This is the `cites:` discipline applied to work rather than to prose.
3. **It is agent-agnostic by construction.** Symphony is Codex-bound and Linear-bound; the corpus's other answers are Claude-bound. A commitment graph over EPRs is readable by any agent that can read the repo — which is precisely the platform-agnostic ambition the `plant-eprfs-*` family already achieves for tooling artifacts.

And `habits.yaml` is the piece the field has *no analog for at all*. Nothing in the corpus has an **admission-controlled** work register: max 12 habits, **max 2 active (a WIP fence)**, `status: green | red | unwired`, flips requiring evidence, and — the genuinely novel state — `unwired`, meaning "we committed to this with no way to observe whether we keep it," declared and counted on purpose. Arize's four statuses are the nearest thing and they lack both the fence and the honest-absence state. The corpus repeatedly rediscovers "agents over-claim done"; `habits.yaml` is the only register I have seen that makes over-claiming *structurally impossible* rather than gated by a `finish` tool's validation.

**The gap is not representational. It is projective.** We have the richer workflow object and the poorer loop: Symphony reconciles ticket state *around* every turn (30s poll, re-check after each of up to 20 turns) — and even Symphony does not re-render that state into each model call (§1.6); only Arize does. We render habits once at session start and then run on conversational memory. The saga's chapter/frontier ordering and the commitment graph's 539 unfulfilled items are invisible to an agent mid-run unless it thinks to go look.

### 4.2 The VSM reading — why this is the same problem twice

Against [the Beer reading](epr:elohim-as-viable-system-2026-06-04), the four context types are not an arbitrary taxonomy. They are the VSM's systems seen from inside a run:

- **Stable instruction = System 5.** Policy and identity — what we will never become. Our constitution/gospel tier.
- **Current project state = System 3.** Here-and-now regulation. *This is what every long agent run lacks*, and it is why runs drift: a System 3 that is reconstructed by re-reading its own transcript is a regulator with no instruments.
- **Map = System 2.** The routing/anti-oscillation function that keeps units from colliding — which our VSM critique named as **the underbuilt system**. The seam map and concern-routing atlas *are* System 2 work, and OpenAI's mechanically-enforced dependency direction (Types → … → UI, verified by linters) is System 2 built as structure rather than prose. That is the corpus's best idea for our thinnest system.
- **History = System 4's raw material.** Kept, but explicitly demoted below current state — the transcript's "history must not masquerade as current instructions" is Beer's warning about reporting channels being mistaken for control channels.
- **The operator's mid-run correction = the algedonic channel.** "Stop the run and change the state" is a pain signal bypassing the hierarchy and rewriting policy. The VSM critique's finding that *our algedonic channel is mediated* — every signal passes through an agent — has an exact analog here: if the operator's correction only reaches the current draft and not the state, the pain signal was absorbed by the thing that caused it.

The elegant connection, stated once: **the context-engineering literature has independently rediscovered System 3 and System 2 without the vocabulary to know that's what they are, and without recursion.** They have one level and one project. We have `ConstitutionalLayer` recursion, per-directory governance, and a commitment graph. What they have that we don't is the *regulator's instrument panel refreshed every cycle*.

### 4.3 `.epr-meta` is stigmergic System 2 — and that is the novel piece

Our VSM reading named System 2 (anti-oscillation between autonomous units) as the underbuilt system, and diagnosed *why*: the protocol's ethos resists the bureaucratic damping that System 2 looks like. That diagnosis needs an amendment, because we built System 2 anyway and filed it under a different name.

`.epr-meta` compose-gates are **coordination through the environment rather than through messages**. A cascading, directory-local manifest fires at the moment of action (PreToolUse), on whoever is acting, without any agent knowing another agent exists. That is exactly the C-compiler's `current_tasks/` lock-file stigmergy, generalized from *task claiming* to *governance*: the rule lives in the place, and the place instructs whoever arrives.

Set against the corpus, this is a genuine lead:

- Anthropic's answer to write-heavy multi-agent conflict is essentially **"don't"** — split only when context is truly isolated, because handoffs lose context and coordination eats the token budget. That is variety-attenuation by refusal.
- Symphony's answer is **workspace isolation plus a single mutable authority** — one orchestrator, per-issue directories. That is a coordinator, i.e. a chokepoint.
- Ours is **neither**: no coordinator, no isolation requirement, and the damping is carried by the substrate at the point of edit. Beer's requirement — autonomy maximized consistent with the cohesion of the whole — implemented as a cascade rather than a referee.

Two more places the field is publishing as novel what we already have a mechanism for:

- **`retire-when:` on every rule** is a written answer to the "harness scaffolding depreciates" lesson. The corpus has the lesson; it has no mechanism. We make each scaffold declare its own obsolescence condition at birth.
- **`/memory-stasis-loop` + `substrate-currency-audit.py`** is doc-gardening *with a drift measure and a stasis condition*, rather than an unbounded scheduled scan.

And the authority inversion: `plant-eprfs-*` makes the package the source of truth while `.claude/` and `.codex/` are content-addressed, provenance-recorded **projections**. That is structurally identical to the PlanMessage and to the managed-agents session log — authority in durable state, consumed artifact regenerated. We arrived at it for *tooling artifacts*; the corpus arrived at it for *run state*. **Neither side has done both**, and we are the side already holding the projection machinery, the content addressing, and the provenance chain.

### 4.4 Where compaction actually breaks us

The operator's question — handling compaction and handoff *consistently with the plans* — has a precise answer in the corpus and a precise hole on our side.

The managed-agents architecture makes the window a **projection of a durable event log**: compaction transforms what this turn sees and never touches the log, so `wake(sessionId)` + `getEvents()` reconstructs. Handoff is the same operation with a different slice. One mechanism, two uses.

Our layers are inverted relative to that. Our durable layer (EPRs, commitments, chronicle, `cites:`) is *stronger* than theirs on identity and provenance — content-addressed, sealed, walkable. But it does not capture the run, and nothing projects it back. Concretely:

1. A mid-run operator correction lands in the conversation. Compaction is lossy summarization of exactly that region.
2. Nothing re-injects the correction afterward, because habits/commitments were never told about it.
3. So the correction's survival depends on the summarizer's judgment — the one place the transcript's warning lands hardest: *"an outdated judgment left in charge."*

The rule that follows is small, testable, and adoptable today, ahead of any tooling: **a correction that must survive compaction gets written to the commitment graph or to `habits.yaml`, not to the conversation.** Anything only said in chat is explicitly history, not state. That single discipline converts our existing durable layer into a compaction-proof one without building anything.

### 4.5 Human gates belong where the decision is genuinely the operator's

The corpus is unusually clear on gate placement, and it exposes a live anti-pattern of ours.

The 400k-session study splits decisions into **planning** (*what* to build, what counts as done — humans ~70%) and **execution** (*how* — Claude ~80%). Symphony makes the human gate a first-class *terminal* state (`Human Review`), not a mid-run interrupt. Both put the human at the boundary where judgment is actually required, and nowhere else.

Measured against that, `/memory-ceremony` Phase 1 escalates the wrong decision. It runs a deterministic ranked audit and then asks the operator to pick 1-2 surfaces off the top of its own ranking. That is an **execution** decision downstream of a measure the ceremony already holds — the ceremony has the drift counts, the edges gauge, the currency readouts, and four lenses. Asking is not caution here; it is the ceremony declining to read its own instrument, and it costs a round-trip on every cycle. It also contradicts our own standing guidance (`[[feedback-decide-clear-calls-not-over-ask]]`, `[[feedback_skip_brainstorm_gates_self_answer]]`).

The gates worth keeping in that ceremony are the ones the corpus would also keep: the **Phase 3 rewrite approval** (a judgment about what the gospel tier should say — planning) and the **holds menu for contested edges** (a scope decision the operator owns). Phase 1's pick is neither. *(Operator observation, 2026-08-13 — filed as TAKE-4.)*

### 4.6 The one warning

The harness rebuild is a direct hit on our own documented failure mode — "if you are about to write a new register, ledger, or ranking script, the answer is almost certainly one of the four above." Their sprint construct was genuinely load-bearing on one model tier and genuinely dead weight four months later. Any projection we build must ship with a `retire-when:` and a stress test, or it becomes the next instrument with no reader. Note that TAKE-1 below adds **no new register** for exactly this reason.

---

## 5. Verdicts

**TAKE-1 — Project the current-state block every turn; don't remember it.** The single highest-leverage item. Re-derive a short pinned block from state we *already keep* — `habits.yaml` top red + active habits (the WIP fence) + the saga frontier + this run's open `commitment:claim:` items — and place it immediately after the system prompt on every turn, not once at session start. Arize's positional hierarchy is the spec and the *only* per-call precedent in the corpus; Symphony contributes the around-the-run reconciliation cadence, not the injection; `epr flow project` is most of the query; `plant-eprfs-*` is the projection machinery. **Adds no new register** — it is a second reader for `habits.yaml` and the commitment graph.

**TAKE-2 — A compaction-survival rule, adoptable today.** Corrections that must outlive the window get written to the commitment graph or `habits.yaml`; conversation is history by definition. Zero tooling, immediate effect, and it is the precondition that makes TAKE-1 worth building.

**TAKE-3 — Failed-approaches-with-reasons at run scale.** We have this at repo scale (museum records, `feedback_*` memories) and nowhere at run scale. `smsharma/clax`'s `CHANGELOG.md` is the template. The value is that a fresh session inherits the *consequence* without re-reading the failure — and it is the cheapest item in this survey.

**TAKE-4 — Drop the Phase 1 surface-pick escalation in `/memory-ceremony`** (§4.5). The ceremony picks the top-N off its own ranking and proceeds, surfacing the ranking as *information*; the operator gate stays at Phase 3 rewrite approval and the holds menu. Note: `/memory-ceremony` is package-governed (`epr:elohim-agent/skills/memory-ceremony`), so this lands through the `plant-eprfs-skill` path, not a direct file edit.

**STUDY-5 — Close the loop from commitment to dispatch.** We have the richer control plane (§4.1a) and no puller. Symphony's poll→reconcile→dispatch loop, its scheduler claim-state, and `Human Review` as a first-class *successful handoff boundary* are the mature shape of what our 556 active commitments could drive — but respect the plane typing (§4.1a): its claims are ephemeral scheduler reservations, ours are durable economic promises; the puller design should join the two planes, not conflate them. This is where the System-2 gap and the multi-agent coordination gap turn out to be one gap. Needs a design pass; do not build from this survey.

**STUDY-6 — Name `.epr-meta` as stigmergic System 2** (§4.3) in the VSM reading, and re-open that critique's "System 2 is underbuilt" finding. It was accurate when written and is now partly superseded by a mechanism filed under governance rather than coordination. A short amendment to the Beer reading, not a new doc.

**STUDY-7 — The Codex instruction-budget mismatch (measured defect, repair held).** Originally logged as a soft comparison against OpenAI's ~100-line `AGENTS.md`; the Codex pass upgraded it to a **mechanically reproduced defect**. Codex builds its instruction chain once per run, root-to-cwd, with a combined project-doc budget defaulting to 32 KiB (`DEFAULT_PROJECT_DOC_MAX_BYTES` in [`openai/codex`](https://github.com/openai/codex/blob/main/codex-rs/config/src/config_toml.rs)); once the root spends the budget, deeper files get nothing. Our generated root `AGENTS.md` is **42,327 bytes / 364 lines** with no `.codex/config.toml` override — a Codex run receives exactly 32,768 bytes and truncates mid-line at `AGENTS.md:308`, cutting the view-schema contract, Critical Gotchas, CI/CD rules, and Code Style. (The Codex-local governance bridge does load — it is front-loaded at lines 1-83.) The *existence and boundary* of the defect are ✅; the **repair is held for the synthesis leg** and must respect package authority (the file is a `plant-eprfs-agentdoc` projection — never hand-edit it): shrink the root to a map, relocate scoped guidance to subtree files, raise the budget, or combine.

**WATCH-8 — Scaffolding depreciation.** Re-read §1.4 whenever a model tier lands: which of our harness parts are load-bearing *now*?

**WATCH-9 — Write-heavy fan-out.** The published guidance says multi-agent is a read-heavy technique and warns against splitting write work by role on a shared surface. We routinely fan out onto one worktree. Our `[[feedback-subagent-disjointness-read-write]]` is the right rail; this is external corroboration that the failure mode is real and expensive, not fussiness.

**WATCH-10 — Don't collapse the three loops.** Polling a tracker, refreshing an in-memory work object, and placing current state into the next inference request are three different operations; evidence for one must never be silently promoted into evidence for the others. This survey's first draft made exactly that collapse on Symphony (corrected in §1.6 and §4.1a) — and Arize's PlanMessage remains the corpus's only evidence for the third loop.

**LEAVE-11 — A `current.md` file.** Declined. A fifth register with no reader is the precise anti-pattern our gospel names. The function is real; the container is a projection of state we already keep (TAKE-1).

**LEAVE-12 — The paywalled starter kit.** The four files it packages (`README` / `current` / `contextmap` / `decisions`) repackage primaries that are all free, and it omits the two mechanisms that matter — the PlanMessage's re-derivation and the durable session log.

---

## 6. Outputs

**Mint pass ran 2026-08-13.** The synthesis leg (legs 3-4 collapsed: VSM x context engineering x EPR governance x dev disciplines) was delivered in-session; the elegant slice it named — point the plant-eprfs-* projection inversion at the run: every turn a projection of the EPR plane, the EPR plane the only legal write target for anything that must outlive a turn — became the organizing frame for the mint. Surviving items folded as **eight rows** into the new [agentic-harness-borrows-backlog](epr:agentic-harness-borrows-backlog) cluster (registered in CLUSTERS.md): per-turn run-state projection (TAKE-1), the write-path discipline (TAKE-2), the Codex instruction-budget repair (STUDY-7, measured), the commitment-to-dispatch puller (STUDY-5, plane-typed), run-scale failed-approaches (TAKE-3), the memory-ceremony gate de-escalation (TAKE-4), dev-system equilibrium as rates-against-rates (synthesis-born; covenant-gated habit candidate), and the Beer-reading amendment (STUDY-6). WATCH items stay in this survey as standing guards; LEAVE items died here honestly.

**Method + credit.** Claim-by-claim trace of a video transcript against primary sources, 2026-08-13, by two agents. **Claude pass**: web search + direct fetch; `openai.com` blocked its fetch path (403), so §1.1 was initially mirror-read; operator steering mid-pass re-centered the survey on the inter-agent layer (shared contextual memory, handoff, compaction-consistency) and named REA valueflows over EPRs as our platform-agnostic workflow representation — §3.1, §4.1a and §4.4 exist because of that steer; §4.5 records an operator observation about gate placement in `/memory-ceremony`. Its adversarial pass corrected two things the transcript got wrong: the harness rebuild kept planner/evaluator and dropped handoffs (§1.4), and the Arize agent is Alyx, with a named mechanism the transcript omits (§1.5). **Codex pass** (disjoint read-only lanes, integrated into this document same day; its companion note `context-engineering-codex-perspective-delta-2026-08-13` was retired on integration): read the OpenAI primaries directly and closed the 403 hole (§1.1 grades upgraded, including the two ⚠ claims); measured the live Codex instruction-budget defect (STUDY-7 — 42,327 B root vs 32 KiB budget, truncation reproduced at `AGENTS.md:308`); and corrected two overstatements in the Claude draft — the Symphony cadence collapse (around-the-turn reconciliation ≠ per-call state projection; §1.6, §4.1a, WATCH-10) and the "close to exact" claim-state mapping (an ephemeral scheduler reservation is not a durable economic promise; §4.1a). The Codex pass also reproduced the `epr flow status` measure (3,667 intents / 556 active / 539 unfulfilled) and grounded the REA lifecycle at file:line.

**Related.** [The Elohim Protocol as a Viable System](epr:elohim-as-viable-system-2026-06-04) · [Beer critique companion](epr:beer-designing-freedom-elohim-critique-2026-06-04) · `.claude/memory-kit/horizon-scans/2026-08-13.md` (the memory-substrate half of this scan).
