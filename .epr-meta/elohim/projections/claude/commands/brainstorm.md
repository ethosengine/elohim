# Brainstorm (coherence-wrapped)

Wraps `superpowers:brainstorming` with a deterministic **pre-step** (compose-from-canonical) and
**post-step** (land it auditable), so brainstorming targets real gaps instead of minting duplicate specs.

This command IS the pre/post seam — the harness has no pre-/post-brainstorming hook, so the wrapper
provides it (same pattern as `/gap-analysis`, `/close-loop`, `/shift`).

Topic: `$ARGUMENTS`

**When NOT to brainstorm:** if the user reframes your proposal with an architectural model (e.g. "actually the
substrate does X, doorway just caches"), that is a signal the design already exists and you haven't absorbed it
yet — research / `systematic-debugging` first. Brainstorm only when the user is genuinely in exploration mode
("I'm not sure how this should work") or when both prior-art lenses below return empty.

## Step 1 — PRE: deterministic prior-art + scope preload (cheap, always run)

```bash
python3 .claude/scripts/memory-kit/prep-brainstorm.py --check-drift "$ARGUMENTS"
```

Read the preload it prints. It tells you, deterministically:
- **PRIOR ART** — specs already touching this topic, ranked, with state.
- **TESTABLE SURFACE** — what's in scope vs `BLOCKED-BY-ENV` (held, don't plan it).
- **BUDGET** — outstanding pressure, and a **drift advisory** if the surface is too messy to brainstorm against.

If the drift advisory fires, STOP and run a structuring pass first (classify `needs-triage`, link unlinked
memory) — do not brainstorm against a dumping ground.

This is the **FRONT fire point** of the Spec/Plan Compaction Loop
(`genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md`, §4): before any design
proposal, surface the canonical seed(s) this topic already descends from so the new artifact is **born linked**
to them (compose, don't fork) instead of minting a duplicate.

## Step 1b — DISCOVERY: semantic surfacing (JIT-scoped MemPalace, recall lens)

The lexical preload above is the always-available floor, but it is **provably blind to vocabulary drift** — it
matches token overlap, so the same concept under different words returns zero matches (the spec's own surfacing
probe got 0 lexical hits for "decompose-self / dump / forget" while the canonical `2026-05-10-memory-lifecycle-design.md`
sat right there under the names *compact* / *forget*). So run a **second, semantic lens** that catches
same-concept-different-words prior art the lexical floor misses (compaction-loop spec §4.1).

Pull MemPalace **just-in-time** — scope exactly the two tools needed at the surfacing step, then release them
(do NOT carry the full ~18-tool MCP as ambient context; an always-on MCP is itself a dump, §4.2):

```
ToolSearch "select:mempalace_search,mempalace_check_duplicate"
```

Then, with those two schemas loaded, query the palace semantically for the topic:
- `mempalace_search` — recall the nearest canonical seeds / history lessons / graduated stories by embedding
  similarity (defeats the lexical blindness above).
- `mempalace_check_duplicate` — confirm whether this topic is already covered (a near-duplicate ⇒ COMPOSE, do
  not fork).

**Staleness guard (§4.4):** the MemPalace index is frozen at mine-time and does not auto-update. If the search
returns nothing, or the index is older than the last BACK-fire dissolve, treat the semantic lens as **STALE —
degraded to lexical-only**, and say so explicitly ("semantic surfacing degraded; trusting lexical floor only").
Never present stale, incomplete recall as authoritative "no prior art" — that false confidence waves through a
fork. The lexical floor (Step 1) always stands regardless.

Carry the **surfaced seeds as plain text** into the brainstorm (exactly as the lexical preload is injected as
text); the scoped MemPalace tools are released after this step, not kept live through the session.

## Step 1c — FRONT-DISCOVERY: locate the work on the MAP and the ROADMAP

The two lenses above surface *prior art* (what spec this descends from). This step surfaces *position* — the
two standing legibility/prioritization maps the compaction-loop machinery keeps continuously current, so the
brainstorm is born oriented, not just born linked. Both are plain-text reads; carry their answers into the
brainstorm exactly as the surfaced seeds are.

**(0) CLASSIFY-SUBJECT — substrate or process? (read `.claude/subject-routing.yaml`).** BEFORE MAP-PATH,
name the subject **class**, because MAP-PATH's D#+pillar lookup is HONEST ONLY for substrate work. Ask the
discriminator: *whose experience does the DELIVERABLE change, and where does the landed change physically
live?* — never "what vocabulary does it use."
- **protocol-canonical** (learner/peer; lands in `app/` + `architecture/` + `a2o/features/<pillar>`) →
  proceed into (1) MAP-PATH + (2) ROADMAP-PRIORITY UNCHANGED. (`status: vision` = a subsumption archetype,
  born in `architecture/applications/`.)
- **process-meta** (developer/agent; lands in `.claude/` + a CLAUDE.md gospel) → **SKIP MAP-PATH's
  D#+pillar lookup** — a process topic has no honest D#; forcing one is the D4 name-collision that mis-filed
  four specs (see `history/2026-06-02-d4-name-collision`). Instead name the **process home**: the
  `genesis/docs/superpowers/` ACTIVE source + the `process_subdomain` (memory / ci / doc-lifecycle / skills /
  agents / hooks / …) whose gospel-diff + `.claude/` tool the residue will land in. ROADMAP-PRIORITY still
  runs (process work is prioritizable) against the process backlog.
- **`provisional`** (a spike whose deliverable-target isn't decided yet) → defer the class; it is *reconciled
  at the BACK-fire* (decompose) from where the residue actually lands. Do **not** force a `domain:`.

The classifier is the cascade resolver `_lib.subject_routing` (the root `.claude/subject-routing.yaml` merged
with any sub-tree manifest on the path — the one-repo→mono-repo→submodule cascade). A product-vocabulary
`derived_from:` (dogfooding EPR / comet / cites) is a **lineage breadcrumb, never a routing key**.

**(1) MAP-PATH — where on the canonical surface does this live?** *(substrate classes only — skip for
process-meta/provisional.)* Read
[`architecture/MAP.md`](../../genesis/docs/content/elohim-protocol/architecture/MAP.md) and name, in one line:
- **which concern-domain D# (Section 1's D1–D10 table)** this topic owns — *"you are working in domain D#"* —
  and the **owning architecture seed(s)** in that domain (cite them `informed-by:` per INDEX's frontmatter
  contract);
- **which pillar(s)** the code lives in (Section 1's pillar axis), and the **per-pillar reading order**
  (Section 2's walk) the implementer should follow — **default to the Household Living Core path** when the
  topic touches care/recovery/memory at the household (it is the seed, not one of equals);
- any **Gap Ledger row (Section 3)** this topic collides with — if it is a *known* OPEN / STRADDLE / CODE-NO-DOC
  gap, say so (you may be filling a tracked hole, not discovering a new one). MAP is the **walk**; INDEX is the
  **graph** — point the dev at MAP first.

**(2) ROADMAP-PRIORITY — where does this sit in vision × readiness?** Read
[`vision-readiness-sprint-roadmap.md`](../../genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md)
(the maintained prioritization home, regenerated each ceremony) and name, in one line, where this work sits:
- a ranked **Sprint-N** (§1) — quote its readiness verdict (READY / MOSTLY-READY / PARTIALLY-READY) and the
  household-living-core *why-it-ranks-there*;
- the **verification track** (§2) — if this is built-but-unverified (CLAIMED-ONLY), the move is *verify*, not
  *re-build*;
- **BLOCKED-BY-ENV** (§3) — if it needs harbor / alpha-cluster / shem, it is HELD: **do not brainstorm an
  implementation against it**, only the design that can land when the operator flips `cluster-state.yaml`;
- or **vision-deferred** (network-scale breadth ranked DOWN of the single-household seed).

**(3) CAPTURE COMPLEMENTARY WORK — keep the executed scope genuine.** As you orient, you will surface
*supportive / adjacent* work — a gap this topic brushes, a dependency it implies, a fix it would benefit from.
Do **not** absorb it into this brainstorm (scope-bloat — a bloated plan is a dump waiting to form). Do **not**
drop it either (a dropped discovery is a dump). **Capture it**: write a one-line item to
[`genesis/data/timeline/backlog/`](../../genesis/data/timeline/backlog/) and link it to its domain D# + roadmap
rung, so it plays nice with whatever brainstorm → spec → plan → sprint actually executes. The genuine task stays
*one thing*; the complementary work becomes a future roadmap entry, not a tangent that derails this one.

**Staleness guard (mirror of Step 1b §4.4):** both maps are regenerated each ceremony, not live. If the
roadmap body is stale against today's `placement-audit.py --ledger` / `--focus`, or MAP's stanza predates the
last seed that landed, treat the orientation as **degraded — trust the ledger/focus numbers over the prose**
and say so. Never let a stale ranking wave through a rebuild of verified work or a pick of a HELD item.

See the compaction-loop spec
(`genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md`, §4): MAP-PATH and
ROADMAP-PRIORITY are the *legibility* and *prioritization* disciplines promoted into the same FRONT-fire
discovery the lexical+semantic lenses already run — surfaced in-flight, additively, every brainstorm.

## Step 2 — Apply the composition rule (binding for this session)

Apply this rule to the seeds surfaced by the lenses above (Step 1 lexical PRIOR ART + Step 1b semantic recall;
Step 1c MAP-PATH names the owning architecture seed in the work's D# domain) — a CANONICAL match found by *any*
lens binds the session to compose:

- **CANONICAL / done match** → COMPOSE from it. Extend the canonical spec; do **not** fork a new one.
- **SUPERSEDED match** → do **NOT** revive. Open its history record, read the gotcha, design *around* it.
- **claimed-UNVERIFIED match** → treat as unverified; note the verification gap, don't assume it works.
- **PRIOR ART empty (BOTH lenses)** → a standalone spec is justified. Proceed. Only an empty result from *both*
  the lexical floor and a *fresh* (non-stale) semantic lens justifies a `cites: []` standalone (§4.3).

## Step 3 — Brainstorm

Invoke `superpowers:brainstorming` on `$ARGUMENTS`, carrying the Step-1 preload as binding context and the
Step-2 rule. Prefer "add a section to `<canonical spec>`" over "new spec" whenever a canonical match exists.

## Step 4 — POST: land it auditable (no orphan, no dump)

Whatever the brainstorm produces, it must be **instantly auditable the moment it lands** — never a no-status
orphan (that is the #1 debt). So the output spec MUST carry PLACEMENT frontmatter:

The frontmatter is **class-conditional** (Step 1c.0 stamped the class):

```yaml
---
title: <name>
status: Draft            # the lifecycle state — NEVER omit
class: <protocol-canonical | process-meta | provisional>   # the subject class (Step 1c.0) — ALWAYS
topic: [<tokens>]        # what it's about (feeds the prior-art index)
cites: [<prior-art paths you composed from>]   # the verifiable links back

# ── protocol-canonical ONLY (substrate work has an honest D#) ──
domain: D#               # the MAP-PATH concern-domain (Step 1c) — where it lives on architecture/MAP.md
informed-by: [<owning architecture seed(s) in that D#>]   # the INDEX/MAP graph edge
# status: vision         #   add for a subsumption archetype (born in architecture/applications/)

# ── process-meta ONLY (NO domain: — a process topic has no honest D#) ──
process_subdomain: <memory | ci | doc-lifecycle | a2o | skills | agents | hooks | build-and-test | schema-sdk>
informed-by: [<process gospel/seed — NOT an architecture seed>]
# derived_from: [<product seed>]   #   ONLY if it dogfoods a product primitive (the lineage breadcrumb)

# ── provisional ONLY (a spike) ── omit domain:/informed-by:; the BACK-fire reconciles class from residue.

# requires_env: [<env>]  # if it can only be validated on a specific node/cluster (a ROADMAP §3 HELD item)
---
```

decompose.py (the BACK gate) re-reads `class:`, reconciles `provisional`, and fails loud if the residue
contradicts the stamp (e.g. `domain: D#` + all-`.claude/` targets — the mis-class the gate catches).

**BORN LINKED (§4.3):** `cites:` is **not** a retroactive afterthought — it is **front-loaded** from the seeds
surfaced at Steps 1 + 1b. Write the lexical PRIOR-ART paths *and* the semantic MemPalace hits into `cites:`, and
add the lineage edge that names the relationship: `refines: <seed>` when extending a canonical seed (the
preferred "add a section to `<canonical>`" path), or `derived_from:` / `compacted_from:` as appropriate. A spec
that surfaced a CANONICAL match but forks a standalone doc anyway is a placement violation the BACK fire point
will catch. `cites: []` is legitimate **only** when both lenses came back empty (and the semantic lens was not
stale).

**SEAL the cites (born-linked, deterministic).** `cites:` was written as plain paths; make them
content-addressed in one shot so the link survives the target moving (e.g. into `held/`) and carries a
progressive-discovery hint:

```bash
python3 .claude/scripts/memory-kit/cite-gen.py --seal <new-spec-path>   # assign-id + path-cites → slug|desc|fingerprint + verify
```

If `--seal` flags `N cite(s) on the title-default desc`, author the relationship hint for each (what the
target is AND why THIS spec points at it) — `cite-describe.py <doc> '{"<ref>":"<hint>"}'`. The cite-seal
postHook nudges if you skip this; the seal is what makes the spec a stable, relocatable cite target.

Then re-audit so the new artifact shows up in the budget immediately:

```bash
python3 .claude/scripts/memory-kit/spec-coherence-index.py   # refresh prior-art index with the new spec
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | head -20
```

**Decompose into gap-items** — run:

```bash
python3 .claude/scripts/memory-kit/decompose.py <new-spec-path>
```

It writes `.claude/memory-kit/gap-items/<slug>.json` — the bounded, cited gap list the next `/plan`
targets (**OPEN** = implement, **CLAIMED** = verify; a checked box is a claim, never trusted as done). If it
reports "needs AGENT decomposition" (a prose design spec with no checkboxes/requirements), extract the
spec's components yourself: 5–15 bounded items, each citing a spec line, `OPEN` unless already
implemented-and-verified. Then `placement-audit.py --ledger` shows the gaps rolled into the budget.
