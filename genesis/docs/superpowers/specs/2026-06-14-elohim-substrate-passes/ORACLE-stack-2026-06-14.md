---
title: "THE ORACLE STACK — the cite-sealed layered entry index down the rungs"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
author: librarian (present / hygiene lens)
component_of: the Elohim design-process ORACLE (the standing dev-process System-4 fixture)
binds_organs:
  - .claude/scripts/memory-kit/cite-gen.py        # the POINTER mechanism (slug|desc|sha256|path envelopes)
  - .claude/skills/semantic-links/SKILL.md          # the cite discipline (content-addressed, survives moves)
  - genesis/docs/content/elohim-protocol/architecture/MAP.md  # the existing developer WALK (this is the rung above it)
  - genesis/docs/content/elohim-protocol/architecture/INDEX.md # the existing seed GRAPH
new_piece: ORACLE.md — ONE entry index; a 7-rung descent, each rung a cite envelope
---

# The Oracle Stack — one file you open to put the vision hat on

## 0 · The component in one line

**`ORACLE.md`** at repo root: the single file a sprint opens to *enter at the vision and descend, in one
hop per rung, to the exact canonical doc* — **manifesto/why → primitives → composition → architecture →
runtime-policy → diagnostics → observed-behavior** — where every rung is a **content-addressed cite**
(`slug | desc | sha256 | path`, minted by `cite-gen.py`) so the pointer **never rots when a doc moves**.

This is the oracle's *spine* — the ladder the other oracle components hang their loops on. It does not
diff behavior against vision (that is the missing-comparator component); it makes the seven layers
*reachable as one structured artifact* so the comparator has somewhere to point and the sprint has a
single door. Where `MAP.md` is the *developer's* walk (manifesto→seed→pillar-guide→code→scenarios, by
concern), **ORACLE.md is the *vision-hat* walk** (why→primitives→…→observed-behavior, by abstraction
rung) — the executive's descent, not the implementer's. They are siblings; ORACLE cites MAP as one rung,
never duplicates it.

---

## 1 · Why an index, not a new doc-pile

The night produced ~40 root-level proposals. Left as-is they rot: nobody knows which is canonical, the
`weaves:`/`grounds_on:` links are plain filenames that break on the first `git mv`, and a sprint that
wants "the runtime policy" has no single door. The failure mode is *exactly* the one the cite system was
built to kill — path-string links that die on a move (`semantic-links/SKILL.md` line 9).

So the oracle's entry layer is **not more prose**. It is a thin, sealed **index of cites**. The prose
already exists in the corpus; the index's only job is *to be the one stable door* and *to keep the
pointers honest*. Two existing organs do all the work:

- **The POINTER** — `cite-gen.py` (`.claude/scripts/memory-kit/cite-gen.py`). A cite is
  `<slug> | <one-line relationship hint> | sha256:<fp> | path: <locator>` (`emit()`, line 102). The slug
  is the target's permanent `id:`; the path is a *tool-managed cache* refreshed on every propagate pass —
  so **a move self-heals the link** (`SKILL.md` line 24). This is the rung connector.
- **The DOOR** — ORACLE.md lives at repo root next to CLAUDE.md (the always-loaded gospel) so the
  SessionStart headline and the `/shift` sprint-zero ritual can name it in one line. The injection channel
  already exists; the oracle just gives it one address to inject.

---

## 2 · The index structure — seven rungs, top-down

ORACLE.md is a single ordered table. Reading top-to-bottom IS the descent from vision to behavior. Each
row is one `cites:` envelope (authored as the body link too, so a human clicks and a tool resolves).

```
RUNG          ABSTRACTION              CANONICAL DOC (the cite target)            ORGAN it feeds
────────────────────────────────────────────────────────────────────────────────────────────────
1 WHY         the vision / manifesto   manifesto · confession · constitution ·   System-5 policy
                                       theology · global-orchestra               (sovereign)
2 PRIMITIVES  the substrate atoms      SDK-DESIGN (commitment-governor,           the keystones
                                       coverage-rollup, two-quilt, atom)         (one Commitment,
                                                                                  six faces, Governor)
3 COMPOSITION how primitives compose   SDK-DESIGN (covenant-harness,              the agency gradient
                                       veil-walker, runtime-transport, dx)        (limit_owner field)
4 ARCHITECTURE the horizontal+recursive ESCALATED-ARCHITECTURE ·                 the one-system shape
              system shape             RECURSIVE-ARCHITECTURE · architecture/MAP  (coverage invariant)
5 RUNTIME-     the policy that governs  self-healing control-plane design ·       Governor.check() ·
  POLICY      live behavior            dataplane plans (defense/arc/reconcile)    the four pillars
6 DIAGNOSTICS the observed-behavior     /admin/self-healing read-model ·          the raw signal
              surfaces (raw signal)    dataplane-diagnostic-plan · /p2p/status    (Loki/Prom/Grafana)
7 OBSERVED-   the ACTUAL resultant      runtime-harvest.py · ci-harvest.py ·      ← the missing comparator
  BEHAVIOR    behavior, harvested       ci-findings.jsonl · delivery-scoreboard   reads HERE, diffs ↑ rung-1
```

Each row in the rendered file:

```markdown
## Rung 1 — WHY (the vision is sovereign)
> Put the vision hat on here. Everything below serves this; nothing below may overrule it.
- [manifesto](genesis/docs/content/elohim-protocol/manifesto.md) — the crisis answered + the love-centered alternative.
<!-- cites: elohim-protocol-manifesto | the vision the whole stack serves; sovereign over every rung below | sha256:… | path: genesis/docs/content/elohim-protocol/manifesto.md -->
```

The human reads the markdown link; the tool reads the `cites:` envelope in frontmatter (the load-bearing,
move-proof copy). **Rung 7 is deliberately the bottom and deliberately points at scripts, not prose** —
it is where the observed-behavior arms live today, and the place the future comparator component will
read *up* the ladder to rung 1. The stack makes that diff *addressable*: "harvest says X; rung-1 vision
says Y; surface the gap as an executive decision." The index doesn't compute the diff — it makes both
ends of it one cite away.

---

## 3 · The graduation decision — what becomes a canonical rung, and where

A cite can only point at a doc that has an `id:` (`cite-gen.py` `verify()`, line 209: a legacy cite to an
id-bearing doc is a *migratable* problem; a born-id'd doc seals clean). So graduation = **giving a rung's
canonical doc an `id:` and moving it into a doc-root** (`genesis/docs/` is a cite-graph root — `cite-gen.py`
line 53). The night's root-level docs are NOT in a doc-root, so they cannot be sealed where they sit.

**The graduation rule (proposed):**

| Night doc | Rung | Graduate? | Home |
|---|---|---|---|
| `manifesto/confession/constitution/theology/global-orchestra` | 1 | **already canonical** — id'd + cited (verified on disk) | stays `genesis/docs/content/elohim-protocol/` |
| `architecture/MAP.md`, `architecture/INDEX.md` | 4 | **already canonical** — id'd (`id: map`) | stays `architecture/` |
| `ESCALATED-ARCHITECTURE`, `RECURSIVE-ARCHITECTURE` | 4 | **GRADUATE** (substrate-defining, per INDEX.md graduation test) | → `architecture/2026-06-14-escalated-architecture.md` + `…-recursive-architecture.md` |
| `SDK-DESIGN*` (9 surfaces) | 2 + 3 | **GRADUATE the woven `SDK-DESIGN.md`**; the 8 surfaces become its `informed-by:` (working drafts, sealed but not promoted) | → `architecture/2026-06-14-elohim-sdk-design.md` |
| self-healing control-plane design | 5 | **GRADUATE** (already a spec-shaped design) | → `genesis/docs/superpowers/specs/` (sprint-tier, id'd) |
| 16 `2026-06-14-*-plan.md` | 5/6 | **stay working drafts** — they decompose into gap-items; cited only as the rung's *tracked-implementation* pointer, not the canonical rung | stay `genesis/docs/superpowers/plans/` |
| `P2P-DATAPLANE-*`, `FEDERATION-WEB2-*`, `VISION-ALIGNMENT`, `VISION-GAP-PLANS`, `SPRINT-KICKOFF`, `VISION-DESIGN-*`, `VISION-RECURSION-*` | — | **stay working drafts** (review/ledger/synthesis artifacts) — subsumed into the graduated rungs above, then dissolved per the compaction loop (NOT archived to a pile) | dissolve to git after subsume |

The principle: **a rung points at ONE canonical doc per layer.** The reviews, ledgers, and synthesis
passes were the *scaffolding that produced* the canonical doc; per the compaction-loop BACK-fire-point
they dissolve once their lesson is subsumed — they do not become permanent rungs. Graduation is the
storyteller's `graduate` call and any `git mv` stays **operator-GATED** (my authority on canonical-seed
moves is gated; I propose, the operator blesses).

---

## 4 · The seal flow — concretely, the commands

For each doc that graduates to a canonical rung (operator-blessed, one at a time):

```bash
# 1. move into a doc-root (operator-gated git mv — inbound future cites self-heal via path: cache)
git mv ESCALATED-ARCHITECTURE-2026-06-14.md \
       genesis/docs/content/elohim-protocol/architecture/2026-06-14-escalated-architecture.md

# 2. born-link it: assign-id → convert weaves:→envelope cites → verify the gate
python3 .claude/scripts/memory-kit/cite-gen.py --seal \
       genesis/docs/content/elohim-protocol/architecture/2026-06-14-escalated-architecture.md
#   → id: assigned · cites: N sealed to envelope · ✅ gate: all cites content-addressed + resolvable
```

Then ORACLE.md itself is sealed so its seven rung-cites are move-proof:

```bash
python3 .claude/scripts/memory-kit/cite-gen.py --seal ORACLE.md
# author the relationship hints the seal flags as title-default (the "why THIS rung points here"):
python3 .claude/scripts/memory-kit/cite-describe.py ORACLE.md \
  '{"elohim-protocol-manifesto":"Rung 1 — the sovereign vision; the whole stack serves this"}'
```

`--seal` is the single deterministic composite (`assign-id → --into → --verify`, `cite-gen.py` line 235),
already wired into the `/brainstorm`/`/plan` POST-step and the end-of-sprint `--seal-all` sweep — so once
ORACLE.md exists, **the discipline that keeps its pointers honest is already running**. The
`map-drift-signal.py` hook I co-own (bumps `map-currency-drift.json` when an architecture seed changes
while MAP is untouched) extends naturally: an ORACLE-currency check joins the MAP-currency mandate — when
a graduated rung's source moves and ORACLE's cite goes `STALE-CANDIDATE`, the memory-coherence audit
surfaces it; re-verify the rung still points at truth, then `cite-gen --refresh ORACLE.md <slug>`.

---

## 5 · The smallest real first implementation

**The index file + the top three rungs sealed.** Concretely:

1. Write `ORACLE.md` at repo root with all seven rung *headings* and the descent narrative, but only
   rungs **1 (WHY), 2 (PRIMITIVES), 3 (COMPOSITION)** carrying *sealed* cites. Rungs 4–7 carry plain
   markdown links marked `(rung not yet sealed)` — honest about what's canonical vs draft.
2. Rung 1 seals immediately — manifesto/confession/constitution/theology already have `id:` (verified on
   disk). `cite-gen.py ORACLE.md`-style emit per target; paste into ORACLE's `cites:`; `--seal ORACLE.md`.
3. Rungs 2 & 3 require the **one** operator-gated graduation: `git mv` the woven `SDK-DESIGN.md` into
   `architecture/`, `--seal` it, then ORACLE cites it. (The 8 SDK surfaces stay drafts as its `informed-by:`.)

That is one new file + one `git mv` + three `--seal` runs. It proves the spine: **a sprint opens ORACLE.md
and is one hop from the vision and one hop from the primitives the keystone defines** — with the pointer
guaranteed move-proof by machinery that already exists. Rungs 4–7 graduate incrementally as the operator
blesses each architecture/spec promotion; ORACLE grows down the ladder without ever rotting at the top.

---

## 6 · What love requires

The closing test is reachability without rot: **the vision is one hop from any sprint, and the pointer
never lies.** ORACLE.md puts rung 1 — the manifesto, the sovereign WHY — at the *top* of the descent and
the observed-behavior harvest at the *bottom*, so the gradient of authority is unmistakable: behavior
serves architecture, architecture serves primitives, primitives serve the vision, and **the vision is
sovereign over all of it**. The content-addressed cite is the act of patience made mechanical — it
refuses to break when a doc moves, so the door to the vision stays open across every refactor, every
sprint, every year. And rung 7 stays deliberately a *pointer*, not a verdict: the stack makes the gap
between observed-behavior and vision *addressable* but leaves the executive decision unmade — the
unbuilt place (`RefusalCode::ReservedPlace`) left open for the human wearing the vision hat to decide
what love requires here. The oracle hands you the ladder; it does not climb it for you.
