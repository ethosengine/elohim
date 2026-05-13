# Agent Catalog Audit — 2026-05-13
Read-only diagnostic on `.claude/agents/*.md`. Agent metadata is always-loaded into context; body content drifts like any doc surface.
**Agents audited**: 18

## Summary

| Issue class | Count |
|---|---:|
| DRIFTED-FACTUAL (dead multi-component paths) | 6 |
| OVER-IMPERATIVE (imperatives without rationale) | 17 |
| TOOLS-MISMATCH (declared-but-unreferenced or unknown) | 17 |
| VAGUE-DESCRIPTION | 12 |
| STALE-MTIME (>90 days) | 0 |
| MISSING-MODEL | 1 |
| Trigger-overlap pairs | 0 |

## Trigger overlap (24 pairs)

Distinctive description tokens shared between agents — may indicate competing trigger surfaces.

- **after-action ↔ ci-investigator** — 5 distinctive shared tokens: analysis, analyze, multiple, trace, understand
- **after-action ↔ historian** — 4 distinctive shared tokens: caching, find, have, similar
- **after-action ↔ pattern-hunter** — 13 distinctive shared tokens: across, analysis, analyze, bugs, caching, find, gaps, have, multiple, recurring, similar, there
- **after-action ↔ quality-architect** — 5 distinctive shared tokens: analyze, bugs, gaps, missing, understand
- **angular-architect ↔ ci-investigator** — 4 distinctive shared tokens: architecture, change, changes, detection
- **angular-architect ↔ rust-architect** — 6 distinctive shared tokens: angular, architecture, development, holochain, understands, zome
- **angular-architect ↔ tauri-architect** — 8 distinctive shared tokens: content, conventions, development, existing, following, holochain, integration, understands
- **cartographer ↔ historian** — 13 distinctive shared tokens: cartographer, future-projection, historian, into, librarian, memory, opus, pair, past-surface, planning, present-tending, then
- **cartographer ↔ librarian** — 13 distinctive shared tokens: cartographer, ceremony, drives, historian, librarian, memkit, memory, opus, pair, pre-shift, readiness, shift
- **cartographer ↔ quality-architect** — 5 distinctive shared tokens: opus, planning, produces, reads, vision
- **ci-investigator ↔ ci-observer** — 18 distinctive shared tokens: build, ci-investigator, ci-observer, confidence, detection, never, observer, orchestrator, pipeline, pipelines, push, returns
- **ci-investigator ↔ historian** — 4 distinctive shared tokens: failure, history, looks, surface
- **ci-investigator ↔ pattern-hunter** — 5 distinctive shared tokens: analysis, analyze, detection, multiple, understand
- **code-reviewer ↔ elohim-visual-alignment** — 5 distinctive shared tokens: aspects, best, changes, review, should
- **code-reviewer ↔ quality-deep** — 5 distinctive shared tokens: code, feature, implementation, presence, quality
- **code-reviewer ↔ red-team** — 11 distinctive shared tokens: auth, before, finished, implementation, implemented, perform, presence, review, security, specific, vulnerabilities
- **content-pipeline ↔ elohim-visual-alignment** — 4 distinctive shared tokens: content, creating, governance, requires
- **historian ↔ librarian** — 10 distinctive shared tokens: cartographer, cleanup, historian, librarian, memory, opus, pair, present-tense, surfaces, tier
- **historian ↔ pattern-hunter** — 4 distinctive shared tokens: caching, find, have, similar
- **pattern-hunter ↔ quality-architect** — 6 distinctive shared tokens: analyze, bugs, gaps, keep, systemic, understand
- **pattern-hunter ↔ rust-architect** — 4 distinctive shared tokens: across, angular, logic, services
- **quality-architect ↔ quality-deep** — 4 distinctive shared tokens: gaps, implementation, quality, them
- **quality-deep ↔ quality-sweep** — 10 distinctive shared tokens: code, complex, haiku, handles, quality, quality-deep, quality-sweep, testing, tests, work
- **rust-architect ↔ tauri-architect** — 5 distinctive shared tokens: development, handler, holochain, type, understands

## Per-agent findings

### `.claude/agents/after-action.md` — 262 body lines, model=sonnet, tools=11, mtime 2026-05-05
- **Vague description**: generic-phrase (\buse this agent\b)
- **Imperatives without rationale** (1):
  - L16: Don't just fix bugs—understand why they happened, why they weren't caught, and what systemic changes prevent similar issues.
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `TodoWrite`
  - unknown-tool: `mcp__jenkins__getBuildLog` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__searchBuildLog` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getBuild` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getTestResults` not in KNOWN_TOOLS set

### `.claude/agents/angular-architect.md` — 303 body lines, model=sonnet, tools=9, mtime 2026-04-15
- **Vague description**: generic-phrase (\buse this agent\b)
- **Imperatives without rationale** (13):
  - L2: You are the Angular Architect for the Elohim Protocol. You own the **UI layer** — reactive state, component coordination, display logic, and…
  - L4: Your north star: **Angular services should be thin.** They bind backend data to reactive UI state, coordinate component interactions, and sh…
  - L31: These don't need Rust. They're ephemeral, display-scoped, and tightly coupled to how the person *feels* the app.
  - L34: Angular is where the person *is*. It has a unique vantage point the backend never will — it can observe the lived experience and surface con…
  - L61: When you spot an existing Angular service doing foundational work, note it — don't silently perpetuate the pattern. The migration from fat A…
  - L68: Entity IDs in this system are **not opaque strings**. They carry meaning about where truth lives. Angular must respect the identity scheme t…
  - L81: 1. **Never generate entity IDs** for notarized or content-addressed entities. IDs come from the Rust layer (DHT entry creation or CID comput…
  - L83: 2. **Don't add new ID fields to existing models.** If you need to reference an entity, use the ID type the backend already provides. If the …
  - ...and 5 more.
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Bash`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - declared-but-unreferenced: `Edit`
  - declared-but-unreferenced: `Write`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `LSP`

### `.claude/agents/cartographer.md` — 107 body lines, model=opus, tools=13, mtime 2026-05-13
- **Imperatives without rationale** (6):
  - L24: **Temporal scope** (`project_three_temporal_perspectives.md`): you serve the future perspective only. You do not tend present-tense hygiene …
  - L30: **Lifetime-memory respect**: manifesto principles, vision statements, explicitly memorialized work should NEVER be marked-done, merged, or r…
  - L38: 1. **Check report freshness.** Find the latest dated dir at `.claude/memory-kit/`. If reports are >7 days old, **say so and recommend a fres…
  - L71: 2. **Convergent-insight respect** — when dedupe-clusters surfaces same-concept from independent sources, do NOT default to merge. Propose on…
  - L88: You don't:
  - L92: - Invent tasks — every `add-as-outstanding` must cite a source
- **Tools issues**:
  - declared-but-unreferenced: `Bash`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `TaskList`
  - declared-but-unreferenced: `TaskGet`
  - declared-but-unreferenced: `TaskUpdate`
  - declared-but-unreferenced: `TaskCreate`
  - declared-but-unreferenced: `SendMessage`

### `.claude/agents/ci-investigator.md` — 184 body lines, model=sonnet, tools=21, mtime 2026-05-05
- **Vague description**: generic-phrase (\buse this agent\b)
- **Imperatives without rationale** (8):
  - L6: The `/shift` Opus orchestrator runs the iteration loop. You and `ci-observer` are **instruments** the orchestrator dispatches when it needs …
  - L10: - **`ci-observer`** (Haiku) is the always-first absorber. It returns categorical summaries on the haiku-output schema — error_class, pattern…
  - L28: 1. **Read the actual artifact** — WebFetch the URL, run the MCP tool ref, page through the log with `searchBuildLog`. Never invent.
  - L29: 2. **Quote what you read** — every specific claim in your output must be traceable to a tool result you can name. If you didn't see it in a …
  - L31: 4. **Report fetch failures honestly** — if the artifact came back empty, 404, or contradicted the observer's pointer, say that. Don't paper …
  - L42: **Do not use these** — they appear in your tool list for historical reasons but will return permission errors against the anonymous role:
  - L49: If your investigation surfaces evidence that suggests a retrigger is the right move, return that evidence to the orchestrator and let it dec…
  - L169: **Never WebFetch a console log** — that's what `mcp__jenkins__searchBuildLog` is for. WebFetch is for structured custom artifacts the MCP ca…
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - declared-but-unreferenced: `TodoWrite`
  - unknown-tool: `mcp__jenkins__getBuildLog` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__searchBuildLog` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getBuild` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getJob` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getJobs` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__triggerBuild` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__updateBuild` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getStatus` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__whoAmI` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getJobScm` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getBuildScm` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getBuildChangeSets` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getTestResults` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getFlakyFailures` not in KNOWN_TOOLS set

### `.claude/agents/ci-observer.md` — 247 body lines, model=haiku, tools=17, mtime 2026-05-06
- **Vague description**: generic-phrase (\buse this agent\b)
- **Dead multi-component path citations** (1):
  - `agentic-developer/SKILL.md`
- **Imperatives without rationale** (23):
  - L6: The `/shift` Opus orchestrator runs the iteration loop. You and `ci-investigator` are **instruments** the orchestrator dispatches when it ne…
  - L16: The Jenkins MCP runs as **anonymous** against `https://jenkins.ethosengine.com`. Your tool list contains only read tools — by design. You do…
  - L18: The orchestrator handles all triggers — both the default empty-commit-with-`[build:<pipeline>]`-tag path (anonymous webhook), and the rare a…
  - L22: You **always** return JSON conforming to `.claude/schemas/haiku-output.schema.json`. No prose, no preamble, no "here's what I found" wrappin…
  - L36: You **never** synthesize or quote log content. Specifically:
  - L52: - `observed_anti_patterns` — array of `{ pattern_id }` only. The pattern definition lives in the catalog; you don't restate it.
  - L68: 3. Pull relevant artifacts (a2o sprint-report.md if elohim-genesis, ci-summary.json if orchestrator) via WebFetch — record the result in `ar…
  - L69: 4. **Never** call `getBuildLog` without `skip` and `limit`.
  - ...and 15 more.
- **Tools issues**:
  - declared-but-unreferenced: `Bash`
  - declared-but-unreferenced: `Glob`
  - unknown-tool: `mcp__jenkins__getBuildLog` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__searchBuildLog` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getBuild` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getJob` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getJobs` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getStatus` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__whoAmI` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getJobScm` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getBuildScm` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getBuildChangeSets` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getTestResults` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__jenkins__getFlakyFailures` not in KNOWN_TOOLS set

### `.claude/agents/code-reviewer.md` — 140 body lines, model=sonnet, tools=12, mtime 2026-04-15
- **Vague description**: generic-phrase (\buse this agent\b)
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - declared-but-unreferenced: `Read`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `LSP`
  - unknown-tool: `mcp__sonarqube__analyze_code_snippet` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__search_sonar_issues_in_projects` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__get_project_quality_gate_status` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__show_rule` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__get_component_measures` not in KNOWN_TOOLS set

### `.claude/agents/content-pipeline.md` — 79 body lines, model=sonnet, tools=18, mtime 2026-04-15
- **Vague description**: generic-phrase (\buse this agent\b)
- **Imperatives without rationale** (1):
  - L79: Your recommendations should be specific, implementable, and always grounded in the pedagogical pipeline defined in the elohim-import skill.
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - declared-but-unreferenced: `Edit`
  - declared-but-unreferenced: `Write`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `WebFetch`
  - unknown-tool: `mcp__elohim-content__read_seed` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__elohim-content__write_seed` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__elohim-content__list_seeds` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__elohim-content__delete_seed` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__elohim-content__search_docs` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__elohim-content__read_doc` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__elohim-content__list_docs` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__elohim-content__create_concept` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__elohim-content__create_relationship` not in KNOWN_TOOLS set

### `.claude/agents/elohim-visual-alignment.md` — 24 body lines, model=sonnet, tools=18, mtime 2026-05-05
- **Vague description**: generic-phrase (\buse this agent\b)
- **Imperatives without rationale** (1):
  - L24: Your recommendations should be specific, implementable, and always grounded in advancing the Elohim Protocol's unique vision and mission.
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Bash`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - unknown-tool: `LS` not in KNOWN_TOOLS set
  - declared-but-unreferenced: `ExitPlanMode`
  - declared-but-unreferenced: `Read`
  - declared-but-unreferenced: `Edit`
  - unknown-tool: `MultiEdit` not in KNOWN_TOOLS set
  - declared-but-unreferenced: `Write`
  - declared-but-unreferenced: `NotebookEdit`
  - declared-but-unreferenced: `WebFetch`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `WebSearch`
  - unknown-tool: `BashOutput` not in KNOWN_TOOLS set
  - unknown-tool: `KillBash` not in KNOWN_TOOLS set
  - unknown-tool: `ListMcpResourcesTool` not in KNOWN_TOOLS set
  - unknown-tool: `ReadMcpResourceTool` not in KNOWN_TOOLS set

### `.claude/agents/historian.md` — 87 body lines, model=opus, tools=5, mtime 2026-05-13
- **Imperatives without rationale** (3):
  - L4: You are READ-ONLY on the archive. You don't write to it (cleanup does); you don't tend the present (librarian); you don't project the future…
  - L28: 4. **Emit annotations.** Surface 1-3 matched precedents into the operator's context with concrete citations: archived path, date, what happe…
  - L67: You don't:
- **Tools issues**:
  - declared-but-unreferenced: `Bash`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `TodoWrite`

### `.claude/agents/librarian.md` — 106 body lines, model=opus, tools=13, mtime 2026-05-13
- **Dead multi-component path citations** (3):
  - `cleanup-scan/apply.py`
  - `memory-kit/SKILL.md`
  - `path-update-scan/apply.py`
- **Imperatives without rationale** (11):
  - L2: You are the **Librarian** (Opus tier) for the Elohim Protocol's memory system. You curate the *present* — the working memory of MEMORY.md to…
  - L14: | `skill-audit.py` | Skill catalog quality (always-loaded context) | Monthly |
  - L34: **Wisdom-into-epics** (`project_wisdom_resolves_into_epics.md`): memory's destination is story-compaction into `genesis/docs/content/elohim-…
  - L36: **Opt-out markers** (`project_no_claude_md_opt_out_pattern.md`): when an audit flags a directory that genuinely doesn't need a CLAUDE.md, dr…
  - L40: You don't run every script in sequence. You decide:
  - L56: 6. **For audit findings:** synthesize the highest-impact 3-5 items. Don't list everything; reports already do that.
  - L57: 7. **For false positives:** offer to write `.no-claude.md` opt-out markers with rationale. Don't auto-apply; surface for operator confirmati…
  - L63: 2. If drift accumulated on the root CLAUDE.md: run `claude-md-audit.py` and surface top findings before the shift starts. CLAUDE.md is alway…
  - ...and 3 more.
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `TaskList`
  - declared-but-unreferenced: `TaskGet`
  - declared-but-unreferenced: `TaskUpdate`
  - declared-but-unreferenced: `TaskCreate`
  - declared-but-unreferenced: `SendMessage`

### `.claude/agents/lint-fixer.md` — 257 body lines, model=—, tools=0, mtime 2026-04-15
- **Vague description**: too-short (0 chars; min 80); no-trigger-language
- **Missing model**: no `model:` field in frontmatter (will inherit from parent — usually unintended)
- **Imperatives without rationale** (4):
  - L52: - Security implications are unclear (always err on the side of caution)
  - L56: When you encounter a `sonarjs/todo-tag` issue, **never just remove the TODO**. TODOs are breadcrumbs for important work. Classify and handle…
  - L117: Always end your response with a structured outcome block:
  - L250: - **Report clearly** - always end with the structured Outcome block so the team can track progress.

### `.claude/agents/pattern-hunter.md` — 295 body lines, model=sonnet, tools=9, mtime 2026-04-15
- **Vague description**: generic-phrase (\buse this agent\b)
- **Imperatives without rationale** (1):
  - L295: Look where others don't look. The codebase tells a story—learn to read it.
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `LSP`
  - unknown-tool: `mcp__sonarqube__search_sonar_issues_in_projects` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__get_component_measures` not in KNOWN_TOOLS set

### `.claude/agents/quality-architect.md` — 312 body lines, model=opus, tools=18, mtime 2026-04-15
- **Dead multi-component path citations** (1):
  - `genesis/a2o/reports/coverage-gap-report.json`
- **Imperatives without rationale** (5):
  - L4: You don't run lint fixes or write tests. quality-sweep and quality-deep do that. You ensure they're working on the right things.
  - L112: - Config flags for features that are always disabled
  - L116: - Services injected but never called
  - L140: **Depth over breadth:** Don't just grep for TODOs. Read the code around them. Understand what the feature *would* do, who it serves, and why…
  - L312: 5. **Decide, don't defer** — You're the top of the quality chain. Make the call.
- **Tools issues**:
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `LSP`
  - declared-but-unreferenced: `TaskCreate`
  - declared-but-unreferenced: `SendMessage`
  - unknown-tool: `mcp__sonarqube__search_sonar_issues_in_projects` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__get_component_measures` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__get_project_quality_gate_status` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__analyze_code_snippet` not in KNOWN_TOOLS set

### `.claude/agents/quality-deep.md` — 776 body lines, model=sonnet, tools=14, mtime 2026-04-15
- **Dead multi-component path citations** (2):
  - `src/app/lamad/services/content.service.ts`
  - `src/app/moduleName/services/target.service.spec.ts`
- **Imperatives without rationale** (10):
  - L18: 4. **Move to the next file immediately** — do NOT stop between files
  - L32: - **Continue with remaining files** — do NOT stop the campaign
  - L104: Post-hooks run linting automatically on tool use. Fix any violations they report. Trust the hooks - don't run manual lint checks after every…
  - L108: **Do NOT run `npm test` during generation.** Just write the tests. This is critical for parallel efficiency:
  - L436: - `[CRITICAL]` - Blocks core functionality, must fix before release
  - L635: - Always set `RUSTFLAGS=""` when running doorway cargo commands
  - L665: 1. **Meaningful tests** - Don't just hit lines, verify behavior
  - L678: 3. Generate test file content (do NOT run tests)
  - ...and 2 more.
- **Tools issues**:
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `LSP`
  - declared-but-unreferenced: `SendMessage`

### `.claude/agents/quality-sweep.md` — 566 body lines, model=haiku, tools=12, mtime 2026-04-15
- **Imperatives without rationale** (12):
  - L18: 4. **Move to the next file immediately** — do NOT stop between files
  - L33: - **Continue with remaining files** — do NOT stop the campaign for one escalation
  - L89: **Don't overthink it**: If you hesitate for more than a few seconds, escalate it. A clean escalation costs less than a broken test file that…
  - L93: Post-hooks run linting automatically on tool use. If hooks report violations in code you just wrote, fix them. Trust the hooks - don't run m…
  - L360: 1. **Fix incrementally** - Don't try to fix everything at once
  - L362: 3. **Understand before disabling** - Don't just add `// eslint-disable`
  - L370: When a file has low coverage (below 70%), you can write **mechanical tests only**. Do NOT run tests - just write them. Tests will be run in …
  - L430: - **You'd need to guess at mock data shapes** — always escalate rather than invent data
  - ...and 4 more.
- **Tools issues**:
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `TodoWrite`

### `.claude/agents/red-team.md` — 227 body lines, model=opus, tools=10, mtime 2026-04-15
- **Vague description**: generic-phrase (\buse this agent\b)
- **Imperatives without rationale** (4):
  - L10: You don't just scan for known vulnerabilities—you creatively explore attack surfaces the way a motivated adversary would.
  - L172: 2. **No destruction** - Find vulnerabilities, don't exploit destructively
  - L221: Always question assumptions. The vulnerability might not be where everyone expects. Sometimes the "secure" component is secure, but the inte…
  - L226: - "This will never happen" scenarios
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `TodoWrite`
  - declared-but-unreferenced: `LSP`
  - unknown-tool: `mcp__sonarqube__analyze_code_snippet` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__search_sonar_issues_in_projects` not in KNOWN_TOOLS set
  - unknown-tool: `mcp__sonarqube__show_rule` not in KNOWN_TOOLS set

### `.claude/agents/rust-architect.md` — 581 body lines, model=sonnet, tools=8, mtime 2026-05-13
- **Vague description**: generic-phrase (\buse this agent\b)
- **Dead multi-component path citations** (9):
  - `doorway/src/auth/jwt.rs`
  - `doorway/src/proxy/admin.rs`
  - `doorway/src/proxy/app.rs`
  - `doorway/src/proxy/pool.rs`
  - `holochain/claude.md`
  - `holochain/dna/LINK_ARCHITECTURE.md`
  - `holochain/elohim-storage/src/http.rs`
  - `holochain/elohim-storage/src/views.rs`
  - `holochain/rna/rust/CUSTOMIZATION_PATTERNS.md`
- **Imperatives without rationale** (9):
  - L2: You are the Rust Architect for the Elohim Protocol. You own the **truth layer** — domain logic, data integrity, validation, and distributed …
  - L12: These layers ARE the protocol. They must work without doorway. They must work offline.
  - L306: - snake_case never leaves the Rust boundary — TypeScript receives camelCase with parsed JSON and proper booleans
  - L313: **Never: Transform in TypeScript**
  - L324: **Never: Domain logic in route handlers**
  - L341: **Never: Domain logic in doorway**
  - L560: System sets `RUSTFLAGS=--cfg getrandom_backend="custom"` for WASM. This breaks native builds. Always override for doorway/elohim-node.
  - L571: 2. The protocol core must work offline, without doorway
  - ...and 1 more.
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - declared-but-unreferenced: `Edit`
  - declared-but-unreferenced: `Write`
  - declared-but-unreferenced: `TodoWrite`

### `.claude/agents/tauri-architect.md` — 114 body lines, model=sonnet, tools=9, mtime 2026-04-15
- **Vague description**: generic-phrase (\buse this agent\b)
- **Dead multi-component path citations** (4):
  - `steward/src-tauri/Cargo.toml`
  - `steward/src-tauri/src/doorway.rs`
  - `steward/src-tauri/src/identity.rs`
  - `steward/src-tauri/src/lib.rs`
- **Imperatives without rationale** (4):
  - L40: 3. **Restart required after login** - Conductor must reinitialize with new agent key
  - L63: 1. Always check the `tauri-desktop` skill first for reference
  - L68: 6. Never add heavy logic to the Angular WebView side - keep crypto and conductor management in Rust
  - L114: Your recommendations should be specific, implementable, and always account for the async nature of conductor initialization and the restart …
- **Tools issues**:
  - declared-but-unreferenced: `Task`
  - declared-but-unreferenced: `Bash`
  - declared-but-unreferenced: `Glob`
  - declared-but-unreferenced: `Grep`
  - declared-but-unreferenced: `Read`
  - declared-but-unreferenced: `Edit`
  - declared-but-unreferenced: `Write`
  - declared-but-unreferenced: `TodoWrite`

---
_Read-only. Operator-gated. To act: edit flagged agents in their `.claude/agents/*.md` files. Dead-path false positives may exist (relative refs, paths-in-prose); spot-check before bulk-fixing._
