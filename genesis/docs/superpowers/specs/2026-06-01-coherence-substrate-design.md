---
id: coherence-substrate-design
status: Design
informed-by:
  - ../../content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md   # the EPR feedback-graph substrate this coherence design turns inward onto the docs
---

# Coherence Substrate for the Elohim Monorepo

**A design for closing implementation and improving architecture every session — instead of minting conflicting specs.**

Author: Opus 4.8 · Date: 2026-06-01 · Status: Design (brainstorm complete, awaiting operator review)

> **Verification note (read first).** This design was checked against the live tree, and three of its earlier load-bearing assumptions were *false* and are now corrected in place:
> 1. **There is no `Skill` matcher and no hook reads `tool_input.skill`.** Every registered `PreToolUse` matcher is a tool-name (`*`, `Edit|Write`, `Bash`, `mcp__sonarqube__search_sonar_issues_in_projects`); `sonar-issues-guard.py` matches a *tool* (`tool_name == "mcp__sonarqube__..."`), not a skill argument. The keystone (§4 Layer 1) is therefore built on the **verified** `*`-matcher + ppid-flag pattern from `pre-tool-memory.py`, which self-gates and reads every tool call — **NOT** on a `Skill` matcher.
> 2. **The architecture INDEX graph is sparse, not rich.** Live tree: **16 top-level `architecture/*.md`** (~30 incl. `applications/` + `horizons/` subdirs), **only 3 carry `realizes:`**, and INDEX.md row-lists ~25 lines. The frontmatter contract is *defined* but *thinly populated*. Earlier "30 architecture specs with a populated graph" was an overclaim.
> 3. **`coverage-gap-report.json` does not exist on disk.** `genesis/a2o/reports/` holds only cucumber/sprint reports. `scan-coverage.ts` *writes* `coverage-gap-report.json` at run time (`REPORT_PATH`, line 487). Any step that READS it must first run `scan-coverage.ts` to regenerate it locally.
>
> The cites-edge plumbing (`memory-coherence-audit.py`, `cites-index.json`, `memory-coherence-signal.py`) and the command surfaces (`close-loop`/`gap-analysis`/`generate-scenarios` as `.claude/commands/*.md`) are present as described.

---

## 1. Diagnosis — how this repo manufactures duplicate specs

The pain is real and the corpus proves it: **182 markdown spec/plan docs across 7 directories, all six weeks old or less** (`genesis/docs/plans` 38, `genesis/docs/superpowers/specs` 41, `genesis/docs/superpowers/plans` 64, …), with **191 checked tasks against 3,940 open checkboxes** and **24+ distinct, non-normalized status strings** (live sample: `DONE`, `Approved`, `approved`, `Brainstorm`, `Design`, `design`, `Draft`, `Vision`, `SUPERSEDED`, `protected`). This is not a writing problem. It is a **missing-read** problem. Five concrete mechanisms produce it, each traceable to an absence in the map:

**(a) Brainstorming has no "search canonical first" step — and authors write no prior-art section.** `superpowers:brainstorming` step 1 is generic — "check files, docs, recent commits." It does not read `genesis/docs/content/elohim-protocol/architecture/INDEX.md` (verified to exist and to *define* the `realizes/informed-by/informs/defers` contract — though only 3 of 16 top-level specs actually fill `realizes:`), does not scan sprint-spec frontmatter for `informed-by:` overlap, and does not query MemPalace for prior art. The survived organizational claim is blunt and names the **cheapest, highest-leverage fix**: the most effective structural gate against re-solving solved problems is a mandatory *Prior Art / Alternatives Considered* section the author **writes before the proposal enters review** (Rust RFC 2333; Google design docs). The *writing* step — not the reading step — is what makes authors abandon dupes before review. This repo has neither the read nor the write gate.

**(b) `/gap-analysis` reads coverage, not prior-art — and its report isn't even on disk.** `.claude/commands/gap-analysis.md` runs `genesis/a2o/scripts/scan-coverage.ts`, which correlates *conceptual* `.feature` files against *executable* ones on `@epic:` tags and **writes `coverage-gap-report.json` at run time** (the file is not persisted in `genesis/a2o/reports/` between runs). It answers "which scenarios lack tests." It has **no spec-coverage dimension** — no "which specs are landed / in-flight / abandoned." So the one tool an agent might run before designing (i) must be re-run to even produce its report, and (ii) tells it nothing about what's already been *designed*.

**(c) There is no spec index for the sprint-shape tier.** The `architecture/INDEX.md` row-lists only the architecture tier (~25 rows; 16 top-level files). The 41 `superpowers/specs` and 38 `genesis/docs/plans` have **no index, manifest, or registry**. You cannot enumerate "all specs touching doorway" without grepping filenames. Confirmed topic duplication already exists and is invisible to tooling: `doorway-ssr-runtime-design` + `ssr-capability-design` (one audits the other, **no supersede marker**); `light-up-the-graph` + `light-up-the-topology`; multiple EPR-phase docs with no machine-readable supersedence chain.

**(d) No "abandoned / superseded" signal at the sprint tier.** Exactly **1 of 182 docs** carries a formal supersedure record (`2026-04-21-recovery-protocol-phase-2-design.md`: `Status: SUPERSEDED`, `Superseded by:`, `Superseded because:`). The survived claim names the loss precisely: *rejected ADRs with attached rationale are the highest-value category* — "the next time someone proposes the rejected idea, the conversation starts from 'here is what we learned last time' rather than from zero." This repo throws that value away on 181 of 182 docs. Brainstorming cannot distinguish a parked idea from a live one from a dead one — so it re-proposes all three.

**(e) Plans have no "Landed" signal — and the Status field itself is heterogeneous.** A reader cannot tell a completed plan from an abandoned one. Live tree: of 41 sprint specs, only **28 carry a markdown `**Status:**` field and only 8 carry a YAML `status:` key** — **13 have no Status field at all**, and those that do span 24+ non-normalized strings. The `dev-intent.jsonl` capture side fires (7 entries) but the `/close-loop` drain points at coverage scaffolding, not prior-art reconciliation.

**The missing feedback loop, named precisely:** *the corpus is write-only at design time.* Every primitive needed to make it read-first already exists in scattered form (the INDEX contract, the cites-edge, MemPalace's KG, the hook system). They have never been pointed back at the act of brainstorming. The survived indictment: **more documentation without governance makes prior-art findability worse, not better** — and this corpus is six weeks of pure accumulation with one supersede marker. The fix is not more docs. It is a **read gate plus a write gate** wired onto the existing substrate.

---

## 2. The unifying model — one typed coherence graph

> **Scope discipline (operator mandate = stop duplicate specs with minimum new machinery).** The full graph below is the *eventual* model. The **stated pain lives at the spec+history tier**: duplicate-prevention needs only (i) normalized status, (ii) supersede/abandon markers, (iii) a prior-art read+write at brainstorm. The `covers` coverage-spine and `grounds-ux` edges are a *separate, valuable* traceability project; they are **deferred to later phases (§7)** and must not balloon the first deliverable. IDs (below) are stamped on the **spec+history tier first**, where dedup actually bites — the scenario tier already keys on path+`@epic:` tag via `scan-coverage.ts` and stays as-is until Phase 3.

Define a single directed graph `G = (N, E)` whose nodes are the artifact-chain tiers and whose edges are the relationships the operator wants progressively surfaced. This is **not a new store** — it is a typed view materialized from artifacts that already exist.

**Node types** (each maps to existing artifacts):

| Node type | Lives as (existing path) |
|---|---|
| `story` | `genesis/docs/content/elohim-protocol/<epic>/epic.md` |
| `archetype` | `genesis/docs/content/elohim-protocol/value_scanner/<life-stage>/` (21 life-stage) + `genesis/data/devices/devices.json` (15 device) |
| `scenario` | `genesis/a2o/features/**/*.feature` (executable) + `…/value_scanner/**/*.feature` (conceptual) |
| `glue` | `genesis/a2o/steps/**/*.ts` |
| `impl` | `elohim/elohim-storage/src/`, `doorway/doorway-service/src/`, `app/elohim-app/src/app/<pillar>/` |
| `spec` | `architecture/*.md` (16 top-level; ~30 incl. subdirs) + `genesis/docs/superpowers/specs/*.md` (41 sprint) + `elohim/sdk/schemas/v1/**` |
| `history` | NEW: `genesis/docs/superpowers/specs/decisions/*.md` (ADR) + a compacted gotcha registry (§5) |

**Edge types** (patterns borrowed from OpenFastTrace `needs/covers`, StrictDoc typed relations, Doorstop `normative/derived` + `active`, and the repo's own INDEX frontmatter — **patterns only; zero new tool dependencies**, see §8):

- `covers` (scenario→story, glue→scenario, impl→glue) — the coverage spine. **Deferred to Phase 3** at the scenario tier; stays path/`@epic:`-keyed via `scan-coverage.ts` until then.
- `derives` / `informed-by` — the existing architecture-spec frontmatter edge (sparsely filled today: 3/16).
- `informs` — existing; what a spec constrains downstream.
- `supersedes` / `superseded-by` — the existing SUPERSEDED marker, generalized.
- `attempted-abandoned` — NEW edge: topic → a `history` node carrying the gotcha + rationale (the duplicate-prevention edge).
- `cites` — the **existing memory `cites:` frontmatter edge**, generalized.
- `grounds-ux` (scenario/spec → storybook / elohim-element / `selectors.ts`) — **named here but assigned to Phase 4 in §7**; it is *not* a dangling type. If Phase 4 is descoped, this edge is cut from the model rather than left unlanded.

**Node identity — recommendation: stable tag-ID, scoped to the spec+history tier first.** Three candidates:

1. *Content-address (CID)* — the EPR-native option. Correct for *published* EPRs, but the artifacts here are **live, mutating git files**. A CID changes on every whitespace edit, re-firing every downstream signal — the "suspect-link fatigue" the DOORS survived-caution warns against, and the "embedding drift" the memory map already suffers. Reject for the live tier.
2. *Path* — what the corpus uses today. **Reject**: `path-update-scan.py` exists precisely because renames break path citations; 87 CITE-CANDIDATE entries already reference paths in prose no tool can traverse.
3. *Stable tag-ID with revision* (OFT `type~name~revision`, StrictDoc UID, Doorstop prefix+number) — **recommend this, scoped to spec+history first.** A small, rename-proof key: `spec~graph-native-projection~1`. Identity survives renames (path is an *attribute*); the `~revision` integer is the change-signal that re-opens downstream coverage — exactly what the cites-edge already implements (`memory-coherence-signal.py` bumps a counter when a cited file changes). **Do not stamp the full 7-tier chain in the first pass** — that is a large vocabulary + author-discipline surface (the OFT/StrictDoc cautions the design itself cites: revision bumps are author-responsibility with silent-drift risk; global uniqueness collisions; no rename tooling). Stamp the **spec + history tier only** (≈41 sprint + 16 architecture docs), where the duplicate-spec problem lives. The scenario/glue/impl tiers stay path/tag-based where `scan-coverage.ts` already works.

**Mapping onto the three existing substrates** (the operator's intuition, made concrete and *honest about what exists*):

- **EPR first-class-graph** (`elohim/epr/`, `EprKind::Manifest` exists). **The avodah subgraph (spec→EPR ingestion) has ZERO code** — `grep avodah elohim/epr/src/` returns nothing; the master design's §11 describes it but nothing implements it. Supersedence is **not** a table named `epr_supersedence`; it is `supersedes: Option<Cid>` on the EPR envelope (`epr.rs`/`envelope.rs`), with `superseded_by` explicitly "DERIVED from supersedence index, NOT in canonical bytes," and it is built for **content/agent EPRs, not specs**. **There is no spec-ingestion extension point today.** Treat EPR-as-spec-store as a Phase-5 *graduation target requiring NEW machinery*, not an available hook. **Phases 0–4 must not depend on any EPR ingestion path.**
- **MemPalace KG** (`/projects/elohim/.mempalace/palace`, 12,866 drawers, ChromaDB + SQLite — **not GraphRAG**) is the *semantic* layer — "is there a spec *like* this?" via embedding similarity. Already wired to historian + librarian via MCP.
- **The `cites:` edge** (`.claude/memory/*.md` frontmatter + `memory-coherence-audit.py` + `cites-index.json`) is the **already-working deterministic edge engine** and the load-bearing primitive. The whole design is "generalize cites: from memory→code to spec↔story↔scenario↔impl, and make brainstorming read it."

---

## 3. Three problems, one mechanism

The operator's instinct — treat duplicate-prevention, living traceability, and spec-compaction as **one** substrate — is correct *at the read layer*. They are three *reads* of `G`. (Per §2's scope discipline, only the first ships in the first deliverable; the other two phase in.)

**Duplicate-prevention is a read of the `attempted-abandoned` + `informs` neighborhood before write.** When an agent brainstorms topic *T*, "have we built/spec'd/abandoned this?" is a graph query: find `spec`/`history` nodes whose `informs`/`cites` neighborhood or embedding overlaps *T*, return their `status` and any `attempted-abandoned` edge with its sentence-sized gotcha. The deterministic backbone answers *named/tagged* overlap structurally; MemPalace answers *conceptual* near-kin the tags miss. **A note on the graph-vs-vector evidence:** GraphRAG's survived "86% vs 57% comprehensiveness" is for **multi-entity/relational** queries ("what touched doorway *and* was superseded"); the companion KILLED result shows GraphRAG is ~13% *worse* on **single-hop factual** queries, and "have we spec'd X?" is often single-hop. So the design uses a **deterministic floor (handles the common single-hop "is there a spec named/tagged ~X") with semantic escalation only for multi-hop/un-named kin** — it does *not* claim graph search wins the common case.

**The living traceability graph is a read of the `covers` spine** — **deferred (Phase 3+).** "Which story has no covering scenario, which spec drifts from impl" is OFT deep-coverage over `G`. The repo already half-does this at the scenario tier (`scan-coverage.ts`); extending the walk up to story/spec and down to impl is a *separate* deliverable, not part of the spec-dedup slice. The coverage read MUST distinguish **`pending` (scenario authored, glue not wired) from `regressed`** — Serenity's "Pending ≠ Failing" and the repo's own `4/15 red baseline` (shakeout oracle) are the model. Otherwise the report mislabels the 1,681 conceptual value-scanner scenarios as *failures* rather than *not-yet-wired*.

**Spec-compaction/history is a read of node age × supersede/abandon markers, producing `attempted-abandoned` edges.** "Which raw specs are unused, superseded, or never landed" is the librarian's stale-candidate query (`memory-review.py` finds `>90 days untouched`; `cleanup-scan.py` finds `status: superseded` + completed plans) applied to the spec corpus. Compaction *writes* `history` nodes (gotcha + pointer) and `superseded-by` edges — which immediately become inputs to the duplicate-prevention read. **The three problems share one substrate at the read layer because compaction's output is duplicate-prevention's input** — but they ship in sequence (§7), not as one co-required bundle.

---

## 4. Firing mechanism — the recommended blend

The harness exposes only `SessionStart`, `Stop`, `PreToolUse`, `PostToolUse` — **there is no `PreBrainstorm`/`PrePlan` event, and there is no `Skill` matcher.** Verified live: every registered `PreToolUse` matcher is a tool-name (`*`, `Edit|Write`, `Bash`, `mcp__sonarqube__...`). The seam is therefore **not** "match the Skill tool by name"; it is the existing **`PreToolUse[*]` once-per-process injector** that `pre-tool-memory.py` already implements. Three layers, escalating cost, each placed where its strength pays:

### Layer 1 — Deterministic `PreToolUse[*]` injector (cheap, always-on backbone — the VERIFIED keystone)

**Fires: `PreToolUse` matched to `*`** (the universal matcher already in `settings.json`), implemented by a new script `.claude/hooks/pre-coherence-context.py` that **copies the verified `pre-tool-memory.py` pattern, not the sonar pattern**:

- It self-gates **once per parent process tree** via a flag file `/tmp/claude-coherence-{ppid}` (the exact mechanism in `pre-tool-memory.py` lines 47–48, 52–54, 78–81), so it pays sub-10ms after the first call and cannot be forgotten.
- On first call it reads stdin, takes `tool_name` like every other hook, and emits prior-art `additionalContext` for the session. It does **not** inspect `tool_input.skill` (that field is unverified and no hook reads it). Because it fires on the first tool of the process tree — which in a brainstorm/plan session is the brainstorming Skill invocation itself — the prior-art context lands *before* the design is written, achieving the same effect without depending on a Skill matcher.

What it injects:

1. it reads a precomputed `spec-coherence-index.json` (the spec-tier analog of `cites-index.json`, built by the extended `memory-coherence-audit.py`),
2. emits `additionalContext`: *"Before proposing a spec, these specs touch areas in this corpus: `spec~ssr-capability~1` (Landed), `spec~doorway-ssr-runtime~1` (SUPERSEDED by ssr-capability — gotcha: 'EPR proxy client must be pooled, not per-request'). Run `/gap-analysis --prior-art` (which regenerates `coverage-gap-report.json` and reads `spec-coherence-index.json`) to scope a topic semantically."*

This is **deterministic, no LLM, no embeddings** — pure stdlib reading a JSON index built exactly the way `memory-coherence-audit.py` builds `cites-index.json`. **Latency claim is deferred, not asserted:** "sub-100ms" is *expected* given the cites-index precedent, but the index does not yet exist and its build cost (walking 41 sprint + 16 architecture docs' heterogeneous frontmatter) is unbenchmarked — measure once the walker lands (Phase 1 check). It cannot hard-block, matching the `additionalContext` advisory protocol the repo already uses.

**Also fires: `SessionStart`** (extend `load-project-context.py`). Inject the *aggregate* drift signal: "Spec corpus: 41 sprint specs, 13 with no Status field, 1 superseded, N flagged stale-candidate. CLAUDE.md drift overdue." This surfaces accumulator state that today appears only on manual ceremony.

**Why deterministic here:** the survived findability claim — volume degrades signal — means the *first* read must be cheap and universal or agents skip it. A `*`-matcher injector that fires on the first tool of every process tree cannot be forgotten. This is reliability-by-construction.

### Layer 2 — MemPalace semantic retrieval (escalation, near-kin the tags miss)

**Fires: inside `/gap-analysis --prior-art` and inside the `p2p-design-gate` prior-spec step.** The deterministic index catches *named/tagged* overlap; semantic search catches *conceptual* overlap the author didn't name. **Cost framing, corrected:** the LazyGraphRAG "~1000× cheaper ($33K→$33)" figure is an *indexing* cost for a large corpus, **not a per-query cost, and MemPalace is ChromaDB+SQLite, not GraphRAG** — so that figure supports only "indexing is cheap," not "this MemPalace query is trivial." MemPalace per-query cost is its own (small, embedding-lookup) number and is bounded by the existing `mempalace_search` MCP path; we rely on *that* being already-wired and cheap, not on a transferred GraphRAG indexing figure. **Before any read, `/gap-analysis --prior-art` MUST run `scan-coverage.ts` to regenerate `coverage-gap-report.json` locally** (it is not persisted). The remaining gap is the per-session `~/.mempalace` symlink and a `specs` wing re-mine. Semantic fires *second* because it costs a query and can hallucinate near-matches; the deterministic layer is the trustworthy floor.

### Layer 3 — Agent-mediated gap report (deepest, on demand or pre-plan)

**Fires: the cartographer's `/converge` and a new spec-coherence ceremony lens, on operator request or when drift accumulates.** When Layers 1–2 surface a *cluster* of related prior specs, an Opus agent (historian for precedent/risk, cartographer for "what's next") reads them deeply and writes the `history`/ADR nodes the cheap layers then surface forever after. **Scope honesty:** these agents are currently wired for **MEMORY entries**; no agent prompt references the spec corpus today. Pointing their remit at specs is a *reasonable reuse* but requires **agent-frontmatter/prompt edits** — scoped explicitly in §7 Phase 2, not assumed free. The four-agent ceremony fan-out (`memory-ceremony/SKILL.md` Phase 2b) is the reusable shape.

**The blend in one line:** *deterministic `*`-injector is the floor every session pays; semantic retrieval escalates for un-tagged kin; agents escalate for judgment and to write durable history.*

---

## 5. The spec lifecycle pipeline

```
brainstorming (reads canonical-first via Layer-1 injector + WRITES the prior-art section, §5C)
   → raw spec (date-prefixed, genesis/docs/superpowers/specs/)
   → SPLIT at land-or-abandon:
        A. /history   — compacted gotcha + pointer (ADR-shaped)
        B. /canonical — linked hyper-structure (the §2 graph, frontmatter-glued)
```

### A. The `/history` record — "deliberate forgetting with a provenance pointer"

The **memory-comet tail** applied to specs: don't delete, distill to a sentence + a pointer, preserve trajectory. It fuses the ADR pattern (`decisions/2026-04-22-reach-backfill-policy.md` is the exemplar), the survived "rejected-ADR-with-rationale" claim, and the survived **11× distillation with provenance** result — which the schema below honors by carrying a **`code_anchor`** field so the record is grep-linkable to code (the "grep-ADR test" for decision↔code traceability). It also borrows Doorstop's **`active: false` (park, don't delete)** + **`derived: true` (no-parent-required)** flags so the stale-scan doesn't raise false "orphan spec" positives on intentionally parked or root specs.

**Record format** (lands in `genesis/docs/superpowers/specs/decisions/`):

```yaml
---
id: history~doorway-ssr-runtime~1
status: Superseded            # Draft | Proposed | Approved | Superseded | Abandoned
active: false                 # Doorstop park-don't-delete: false = parked, not orphaned
derived: false                # true = root/no-parent-required, suppresses false orphan flags
supersedes: spec~doorway-ssr-runtime~1
superseded-by: spec~ssr-capability~1
topic: [doorway, ssr]
pointer: genesis/docs/superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md
gotcha: "Per-request EPR proxy clients exhausted the pool; the runtime must hold one pooled client."
code_anchor: "37c822d1c — doorway/doorway-service/src/epr_proxy.rs"   # grep-linkable provenance
attempted-abandoned: true
created: 2026-05-08
---
```

The **`gotcha` is one sentence**, the **`pointer` is one path**, **`code_anchor` is a commit+path** (provenance the survived claim says preserves retrieval fidelity), `status` is the normalized lifecycle (fixing the 24+-string gap), and `superseded-by`/`supersedes` form the bidirectional chain. This small object is what the Layer-1 injector surfaces at the next brainstorm. The corpus already has raw material: `wip-disposition`, `cross-pillar-cleanup-dispositions`, `BATCH-C-PIVOT`, `archaeology-decisions-digest` are **informal versions of exactly this** — the disposition-manifest pattern (`2026-05-25-cross-pillar-cleanup-dispositions.md`) is the reusable compaction shape.

### B. The `/canonical` record — link hyper-structure

A landed spec gets the architecture frontmatter contract that already exists (`realizes/informed-by/informs/defers/memory_anchors`), extended with a stable `id:` tag, normalized `status:`, and `cites:` code anchors. (`covers` and `grounds-ux` edges attach in Phases 3–4, not Phase 0–2.) If substrate-defining, it **graduates** to `architecture/` and gets a row in `INDEX.md` (the graduation workflow the map flags as missing). The frontmatter *is* the edge set — no separate registry, git-tracked, diffable, reviewable in PR.

### C. The prior-art *write* gate — the cheapest highest-leverage fix (Phase 0)

The survived evidence says the **writing** of a prior-art section is what makes authors abandon dupes before review. So the sprint-spec template gains a **required** section + frontmatter, gated by the `p2p-design-gate` prior-spec step:

```yaml
prior_art:
  supersedes-or-conforms-to: [spec~ssr-capability~1]   # what this builds on / replaces (or: none-found)
  searched: "Layer-1 injector + /gap-analysis --prior-art on 2026-06-01; no live spec on <topic>"
```

```markdown
## Prior art / specs this supersedes-or-conforms-to
<!-- REQUIRED before review. List prior specs/history nodes found, or state "searched, none found". -->
```

This is **pure convention, zero code**, and directly attacks diagnosis (a). It is the Phase-0 companion to the Layer-1 *read* gate: the injector makes prior art *visible*, the template makes the author *write down what they found*.

### Compaction trigger & ownership

- **Trigger (signal-driven, not calendar):** a spec-tier analog of `memory-review.py`'s stale-candidate scan fires a counter when (i) a sprint spec is `>N days` untouched with `status` not in {Landed, Superseded} and `active: true`, or (ii) two specs share `>threshold` embedding similarity (the **`dedupe-memory-scan.py` TF-IDF/cosine pattern applied to specs** — the single most important tool to clone). Crossing threshold surfaces at SessionStart. Parked (`active: false`) and root (`derived: true`) specs are excluded to avoid false orphans.
- **Distillation ownership:** the **storyteller** owns graduate/memorialize/abandon disposition; the **historian** supplies precedent + gotcha rationale; the **librarian** runs the deterministic scan that feeds them. **This requires editing those four agent prompts to reference the spec corpus** (they reference only MEMORY entries today) — scoped in §7 Phase 2, not assumed.
- **Tools to extend:** clone `dedupe-memory-scan.py` → `dedupe-specs-scan.py`; extend `memory-coherence-audit.py` to emit `spec-coherence-index.json`. **Parser caveat:** memory entries use *normalized* YAML frontmatter; specs do **not** (13/41 have no Status field; markdown `**Status:**` vs YAML `status:` vs prose). The walker extension must therefore **first normalize spec frontmatter (Phase 0)** before it can mirror the memory walker — this is real parser work for heterogeneous input, not a free mirror.

---

## 6. Concrete extension points (minimum new machinery)

| Existing asset (exact path) | How to extend | What it gains | New code? |
|---|---|---|---|
| `architecture/INDEX.md` + frontmatter contract | Add a **sprint-shape registry section**; require `id:` + normalized `status:` on sprint specs; add graduation-row workflow | The sprint tier becomes enumerable; status machine-parseable | Pure convention |
| Sprint-spec template + `p2p-design-gate/SKILL.md` | Add a **required `prior_art:` frontmatter + "Prior art" section** (§5C); insert a "Prior spec lookup" gate step | The **author WRITES prior art before review** — attacks diagnosis (a) at root | Pure convention |
| `.claude/scripts/memory-kit/memory-coherence-audit.py` + `cites-index.json` | Add a spec walker that **first normalizes heterogeneous spec frontmatter** (markdown `**Status:**` / YAML / prose → one schema), then reads `informs/cites`, emits `spec-coherence-index.json` | The deterministic prior-art index the Layer-1 injector reads | **Extend** (mirror + a real frontmatter normalizer — not a pure mirror) |
| `.claude/scripts/memory-kit/dedupe-memory-scan.py` | Clone → `dedupe-specs-scan.py`; TF-IDF cosine over `superpowers/specs/` + `plans/` | Surfaces confirmed dupes (ssr pair, EPR-phase docs) as ranked clusters | **New** (small clone) |
| `.claude/hooks/pre-tool-memory.py` (**verified pattern**) | Copy → `.claude/hooks/pre-coherence-context.py`; **register on the existing `PreToolUse[*]` matcher; self-gate once-per-ppid via `/tmp/claude-coherence-{ppid}`; read `tool_name` + emit prior-art `additionalContext`** | The always-on "search canonical first" read gate — **the keystone** (NOT a Skill matcher; that does not exist) | **New** (~60 lines, ppid-flag clone) |
| `.claude/hooks/load-project-context.py` | Append spec-corpus drift summary to `context_parts` | Aggregate drift visible at SessionStart | **Extend** |
| `.claude/memory/*.md` `cites:` edge | Roll out `cites:` to the 87 CITE-CANDIDATEs | Wires the dormant `memory-coherence-signal.py` for the spec chain | Pure convention |
| `genesis/a2o/scripts/scan-coverage.ts` (writes `coverage-gap-report.json`) | `.feature`-cite indexing requires a **second walker** so the audit indexes feature-file `cites:` (it indexes MEMORY only today); coverage read must add a **`pending` vs `regressed`** status | `.feature` cites become live; value-scanner scenarios labeled not-yet-wired, not failed | **Extend** (second walker — Phase 3, not free) |
| `.claude/commands/gap-analysis.md` | Add `--prior-art` mode that **first runs `scan-coverage.ts` to regenerate `coverage-gap-report.json`**, then reads `spec-coherence-index.json` + escalates to MemPalace | `/gap-analysis` answers prior-art; never assumes the report is on disk | **Extend** |
| `.claude/commands/close-loop.md` + `dev-intent.jsonl` | Write a `history~` gotcha record when intent abandons an approach | Captures paths-not-taken at the cheapest moment | **Extend** |
| `.claude/commands/generate-scenarios.md` + `generate-step-skeletons.ts --tag` | Point at `@epic:value_scanner` to scaffold the 1,681 conceptual scenarios | Repairs the largest broken coverage join (Phase 3) | Pure invocation |
| `superpowers:brainstorming` step 1 | **Project-level CLAUDE.md "Brainstorming" section** mandating: read `INDEX.md`, surface `informs:` hits + status, before clarifying questions | Spec-first read without touching the plugin | Pure convention |
| `.claude/agents/{historian,librarian,storyteller,cartographer}.md` | **Edit prompts to reference the spec corpus** (they reference MEMORY only today) | The four-agent division of labor pointed at specs — an explicit edit, not a free reuse | **Extend** (prompt edits) |
| MemPalace (`historian`/`librarian` MCP) | Re-mine a `specs` wing; ensure `~/.mempalace` symlink | Layer-2 semantic prior-art | Config + re-mine |
| `decisions/2026-04-22-reach-backfill-policy.md` (ADR template) | Adopt as the `/history` gotcha-record template (§5A) with `code_anchor` + `active`/`derived` | Normalized, grep-linkable abandoned-path records | Pure convention |
| `elohim/epr/` (`EprKind::Manifest`; **avodah = ZERO code**) | **Defer. NEW machinery required.** No spec-ingestion path exists; supersedence is an envelope `Option<Cid>` field for content/agent EPRs, not a spec table. Phase 5 only | Specs-as-EPRs, native supersedence | **New (later sprint — net-new ingestion)** |

**Genuinely new code is tiny:** one hook (`pre-coherence-context.py`, ppid-flag clone), one scan clone (`dedupe-specs-scan.py`), one walker extension (`spec-coherence-index.json`, including a frontmatter normalizer). Everything else is convention + extension. This honors the extend-don't-reinvent mandate.

---

## 7. Build sequencing — spec-dedup first, traceability later

Each phase ships a deliverable and a testable check. Ordered so the cheapest highest-leverage move lands first; the coverage-spine and grounds-ux work is **explicitly later** so the first deliverable stays the spec-dedup slice.

**Phase 0 — Normalize status, stamp spec/history IDs, add the prior-art WRITE gate.**
Deliverable: every sprint spec under `superpowers/specs/` + `genesis/docs/plans/` gets a normalized `status:` (`Draft|In-flight|Landed|Superseded|Abandoned`) and an `id:` tag (spec+history tier only); the confirmed pair (`doorway-ssr-runtime` ← `ssr-capability`) gets the SUPERSEDED marker + a `history~` gotcha record with `code_anchor`; the sprint-spec template + `p2p-design-gate` gain the required `prior_art:` section (§5C). *Check:* `grep -L "^status:" superpowers/specs/*.md` returns empty (today 13/41 lack any Status field); the template's required section is present and the gate references it. Fixes diagnosis (a)/(d)/(e).

**Phase 1 — Deterministic index + the `PreToolUse[*]` keystone.**
Deliverable: `spec-coherence-index.json` built by the extended `memory-coherence-audit.py` (with the frontmatter normalizer); `pre-coherence-context.py` registered on the **existing `PreToolUse[*]` matcher**, self-gating via `/tmp/claude-coherence-{ppid}`. *Check (a2o-style):* `Given the first tool call of a brainstorm session / When pre-coherence-context.py fires / Then additionalContext names spec~ssr-capability~1 (Landed) and the doorway-ssr SUPERSEDED gotcha`. Verified by a unit test feeding synthetic stdin to the hook. **Also benchmark** the index build + injector read here (the latency claim is unmeasured until the walker exists).

**Phase 2 — Dedupe scan + compaction ceremony (with agent-prompt edits).**
Deliverable: `dedupe-specs-scan.py`; **edits to the four agent prompts** so they reference the spec corpus; a storyteller/historian compaction pass producing `history~` records. *Check:* the scan flags the `ssr` pair + EPR-phase docs above threshold; a ceremony run converts the top cluster into a gotcha record; the Phase-1 injector surfaces it next related brainstorm.

**Phase 3 — Coverage-spine + `.feature` cites (the deferred traceability slice).**
Deliverable: `cites:` rolled out to the 87 CITE-CANDIDATEs and a **second walker** so the audit indexes feature-file `cites:`; `scan-coverage.ts` gains a **`pending` vs `regressed`** status; `generate-step-skeletons.ts --tag @epic:value_scanner` scaffolds the 1,681 conceptual scenarios. *Check:* `scan-coverage.ts` shows `@epic:value_scanner` coverage rising; the 1,681 scenarios report as `pending`, not `failing`; `memory-coherence-signal.py` fires on a value-scanner `.feature` edit.

**Phase 4 — Semantic + agent escalation + `grounds-ux`.**
Deliverable: `/gap-analysis --prior-art` (regenerates the report, then escalates to MemPalace); `p2p-design-gate` prior-spec step live; SessionStart drift summary live; `grounds-ux` edges attached (or this edge type is cut from §2 if descoped). *Check:* a brainstorm on an un-tagged-but-conceptually-prior topic surfaces the near-kin spec via semantic recall the deterministic index missed.

**Phase 5 (deferred — NEW machinery) — Graduate canonical specs to EPRs (avodah subgraph).**
Only after Phases 0–4 prove the git-file graph works. **This is net-new code: there is no spec→EPR ingestion path today** (`grep avodah elohim/epr/src/` is empty; supersedence is an envelope `Option<Cid>` field for content/agent EPRs). Deliverable: build the ingestion that lands canonical specs as `EprKind::Manifest` atoms with `supersedes` Cids. *Check:* the EPR supersedence index answers "prior versions of this spec" without reading frontmatter.

---

## 8. Risks & anti-patterns (each with the mitigation already in the design)

- **Scope balloon (spec-dedup → full traceability project).** The operator mandate is minimum machinery to stop duplicate specs. *Mitigation:* the first deliverable is **spec+history tier only** (§2 scope box); the coverage-spine and `grounds-ux` edges are explicitly Phases 3–4 (§7), and `grounds-ux` is cut from the model if Phase 4 is descoped rather than left dangling.

- **ID-convention rollout cost.** Stamping IDs across the full 7-tier chain is a large author-discipline surface (OFT/StrictDoc cautions: silent revision drift, cross-pillar collisions, no rename tooling). *Mitigation:* IDs are scoped to spec+history first; pillar-prefixed namespace (`spec~lamad.content-node~1`); the dedupe scan's first pass is an ambiguity check. The scenario tier stays path/`@epic:`-keyed where `scan-coverage.ts` already works.

- **Traceability rot / suspect-link fatigue** (DOORS + OFT cautions): if every cosmetic edit re-fires coverage, agents stop reading the signal. *Mitigation:* identity is **tag-ID + explicit `~revision`**, not a content-hash — only an author-declared revision bump invalidates `covers` edges; scope the signal to spec-frontmatter changes, not body whitespace.

- **Embedding/MemPalace drift** (palace carries frozen mine-time content): semantic prior-art could surface stale matches. *Mitigation:* the deterministic index is the **trustworthy floor**; MemPalace is escalation-only, explicitly labeled "semantic candidate, verify against the tagged spec." Re-mine the `specs` wing on the compaction cadence.

- **False "orphan spec" positives.** A naive stale-scan flags parked or root specs as dead. *Mitigation:* Doorstop's `active: false` (park, don't delete) + `derived: true` (no-parent-required) flags in the §5A schema exclude them.

- **`pending` mislabeled as `failing`.** The coverage read could report the 1,681 not-yet-wired value-scanner scenarios as failures. *Mitigation:* adopt Serenity's Pending≠Failing distinction + the repo's own `4/15 red baseline` model — `pending` (authored, glue unwired) is a separate status from `regressed`.

- **Doc-rot / findability paradox** (more docs make prior-art *harder* to find): *Mitigation:* the design adds **near-zero net documents** — it adds *edges* (frontmatter) and *compacts* (gotcha records that replace, not supplement). The dedupe scan + storyteller disposition actively *shrink* the live corpus.

- **Over-engineering vs extend-mandate / tool-import temptation.** The five horizon tools surveyed (OFT, StrictDoc, Sphinx-Needs, Doorstop, Serenity/Concordion/Gauge/Pickles) are Python/.NET/JVM and **substrate-incompatible**. *Mitigation:* the design adopts their **patterns only — ID+revision, needs/covers, park-don't-delete, Pending≠Failing — and ZERO new tool dependencies.** The EPR/avodah subgraph is explicitly Phase 5 (net-new); the keystone is one ppid-flag hook + one scan clone + one walker extension over working tools.

- **Compaction destroys recoverable context** (recursive-summarization noise / long-horizon collapse): *Mitigation:* compaction is **distillation-with-provenance** (11× pattern + memory-comet tail) — a one-sentence `gotcha` + a `pointer` + a `code_anchor`, never a lossy body summary. The raw spec stays at its path; only its *index entry* compacts.

---

## 9. Open questions for the operator

1. **Node-identity scheme — confirm tag-ID over CID, scoped to spec+history first?** I recommend `type~name~revision` for live git files (rename-proof, low churn), stamped on the **spec+history tier only** in the first pass, with CIDs reserved for *published* avodah EPRs in Phase 5. The fork: if you want canonical specs as EPR atoms *now*, that is **net-new ingestion code** (no avodah path exists) and Phase 5 moves up. Strong recommendation: tag-ID now, CID at graduation.

2. **Do canonical specs become real DHT/EPR entries, or stay git files?** Confirmed: **the avodah/spec-ingestion subgraph has ZERO code** — supersedence today is an envelope `Option<Cid>` field for content/agent EPRs, not a spec store. I recommend specs stay **git files** through Phase 4 (git is source of truth; MemPalace/EPR are projections), graduating substrate-defining specs to EPRs only in Phase 5. Confirm you don't want the (net-new) EPR ingestion sooner.

3. **Compaction cadence — signal-driven threshold values?** Accumulator-triggered, not calendar. Set: days-untouched + status-not-in-{Landed,Superseded} + `active: true` before a sprint spec is a stale-candidate; TF-IDF cosine that flags a dupe-cluster.

4. **Hard-gate or advisory at the brainstorm injector?** Layer-1 is advisory `additionalContext` by default (cannot block — the harness has no Skill matcher to deny on anyway; the `*`-injector only *adds context*). Do you ever want a `permissionDecision: deny` on the **`Superseded`/`Abandoned`-topic** case? I lean advisory-first, flip-to-deny only on that case.

5. **Who runs Phase 0?** The status-normalization + ID-stamp + prior-art-template pass over the 41 sprint docs (13 with no Status field) is mechanical but interpretive. A one-shot disposition pass — the `cross-pillar-cleanup-dispositions` shape. Single librarian-led ceremony, or folded into the next `/converge`?

---

**Relevant files (all absolute):**
- Architecture graph + frontmatter contract (16 top-level, 3 with `realizes:`): `/projects/elohim/genesis/docs/content/elohim-protocol/architecture/INDEX.md`
- ADR exemplar (gotcha-record template): `/projects/elohim/genesis/docs/superpowers/specs/decisions/2026-04-22-reach-backfill-policy.md`
- SUPERSEDED marker (only 1 of 182 docs): `/projects/elohim/genesis/docs/superpowers/specs/2026-04-21-recovery-protocol-phase-2-design.md`
- Disposition-manifest pattern (compaction shape): `/projects/elohim/genesis/docs/superpowers/notes/2026-05-25-cross-pillar-cleanup-dispositions.md`
- Deterministic edge engine to extend: `/projects/elohim/.claude/scripts/memory-kit/memory-coherence-audit.py`, `/projects/elohim/.claude/memory-kit/cites-index.json`
- Dedupe pattern to clone: `/projects/elohim/.claude/scripts/memory-kit/dedupe-memory-scan.py`
- **Keystone hook template (VERIFIED `*`-matcher + ppid-flag pattern — NOT the sonar Skill-matcher myth):** `/projects/elohim/.claude/hooks/pre-tool-memory.py`; register in `/projects/elohim/.claude/settings.json` under the existing `PreToolUse[*]` entry
- SessionStart injector to extend: `/projects/elohim/.claude/hooks/load-project-context.py`
- Prior-art read entry points: `/projects/elohim/.claude/commands/gap-analysis.md`, `/projects/elohim/.claude/commands/close-loop.md`, `/projects/elohim/.claude/commands/generate-scenarios.md`
- Coverage scanner (writes `coverage-gap-report.json` at run time; NOT persisted): `/projects/elohim/genesis/a2o/scripts/scan-coverage.ts`; skeleton generator: `/projects/elohim/genesis/a2o/scripts/generate-step-skeletons.ts`
- EPR graduation target (Phase 5 — **avodah/spec-ingestion has ZERO code; net-new machinery**): `/projects/elohim/genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md` §11; `/projects/elohim/elohim/epr/src/kind.rs` (`EprKind::Manifest`), `/projects/elohim/elohim/epr/src/envelope.rs` (`supersedes: Option<Cid>`)
- Agent prompts requiring spec-corpus edits (Phase 2): `/projects/elohim/.claude/agents/{historian,librarian,storyteller,cartographer}.md`
- MemPalace semantic layer (ChromaDB+SQLite, NOT GraphRAG): `/projects/elohim/.mempalace/config.json` (symlink `~/.mempalace` per session)
