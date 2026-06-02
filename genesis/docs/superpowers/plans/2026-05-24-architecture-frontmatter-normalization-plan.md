---
status: Draft
---

# Architecture Specs — Frontmatter Normalization Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans inline. Single agent (Sonnet); mechanical YAML normalization across 14 files. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Normalize the YAML frontmatter on 14 architecture-tier specs (migrated 2026-05-24 from `genesis/docs/superpowers/specs/` to `genesis/docs/content/elohim-protocol/architecture/`) to the architecture-contract shape declared in `architecture/INDEX.md`. Body content is canonical-correct and unchanged; only frontmatter is touched.

**Architecture:** Single agent, mechanical pass. For each spec: parse existing frontmatter; ensure `tier: architecture` is present; convert legacy field names to the contract (`narrative spine:` → `realizes:`, `related:` / `related specs:` → `informed-by:`); add `informs:` field with reasonable inferred content; preserve all other existing frontmatter fields (memory_anchors, defers, authors, dates, etc.) unchanged.

**Tech Stack:** YAML frontmatter editing in 14 markdown files; validation via simple parse check.

---

## The architecture frontmatter contract (paste-from-INDEX)

Every architecture-tier spec MUST have this frontmatter shape (from `genesis/docs/content/elohim-protocol/architecture/INDEX.md`):

```yaml
---
title: <descriptive title>
tier: architecture
status: <Draft | In-flight | Landed | Superseded>
created: <YYYY-MM-DD>
authors: <human + AI co-authors>
pillar coupling: <which pillars touch this primitive>
realizes:                          # ← epics this spec gives technical form to
  - genesis/docs/content/elohim-protocol/<epic-dir>/epic.md (one-line context)
informed-by:                       # ← architecture or sprint specs this builds on
  - <path> (one-line context)
informs:                           # ← downstream specs / code this constrains
  - <category or specific spec>
memory_anchors:                    # ← MemPalace entries the spec leans on
  - project_<slug>
defers:                            # ← things explicitly out of scope
  - <one-line description>
---
```

## Legacy field → contract mapping

The 14 migrated specs have varied existing frontmatter. The mapping rules:

| Legacy field (varies by spec) | Contract field | Notes |
|---|---|---|
| `narrative spine:` | `realizes:` | Same semantic — what epic this spec realizes |
| `narrative_spine:` (underscore variant) | `realizes:` | Same |
| `related:` | `informed-by:` | Upstream specs this builds on |
| `related specs:` | `informed-by:` | Same |
| `depends on:` | `informed-by:` | Upstream dependency = informs-this |
| `Plan kinship:` (in markdown body) | `informed-by:` (move to frontmatter if absent there) | Some specs put kinship in body H2; pull into frontmatter |
| (missing) `informs:` | NEW — infer from spec scope | Add what downstream this constrains |
| (missing) `tier:` | NEW — add `tier: architecture` | Required for the contract |
| `memory_anchors:` | UNCHANGED | Already correct field name |
| `defers:` | UNCHANGED | Already correct |
| `authors:` | UNCHANGED | Already correct |
| `created:` | UNCHANGED | Already correct |
| `status:` | UNCHANGED | Already correct |
| `pillar coupling:` | UNCHANGED | Already correct |

## The 14 files to normalize

```
genesis/docs/content/elohim-protocol/architecture/
├── 2026-04-18-experience-story-epr-design.md
├── 2026-04-21-elohim-core-graph-substrate-design.md
├── 2026-04-21-elohim-epr-integrator-compatibility-contract.md
├── 2026-04-23-epr-phase-2c-libp2p-federation-design.md
├── 2026-05-02-elohim-hub-boundaries-design.md
├── 2026-05-08-iroh-libp2p-complementarity.md
├── 2026-05-10-memory-lifecycle-design.md
├── 2026-05-11-attestation-consolidation-design.md
├── 2026-05-11-observation-event-layer-design.md
├── 2026-05-11-tiered-quilt-stewardship-design.md
├── 2026-05-15-dna-signal-as-epr-envelope.md
├── 2026-05-20-wave3-valueflows-hrea-interop-design.md
├── 2026-05-23-doorway-access-tier-patterns.md
└── 2026-05-23-multi-collective-collaboration-epr-design.md
```

Each file has slightly different existing frontmatter; the agent must read each before deciding edits.

## Inferring `informs:` per spec (the only judgment-call field)

The `informs:` field is what's downstream of this spec — what's bound or constrained by it. The agent should infer from the spec's scope. Guidelines:

| Spec scope | Likely `informs:` content |
|---|---|
| Defines a substrate primitive (Observation, EPR, Attestation) | "All future sprint specs that touch <primitive>"; "Any new pillar manifest declaration using <related vocabulary>" |
| Defines transport architecture (iroh-libp2p complementarity, hub boundaries, doorway access tiers) | "All future planes / sync work"; "All future transport-level optimization specs" |
| Defines an integration contract (EPR integrator compatibility, valueflows interop) | "All future bridges following this contract"; "All apps consuming this interop layer" |
| Defines lifecycle vocabulary (memory-lifecycle) | "Records-lifecycle (sibling vocabulary)"; "Any future memory-class extensions" |
| Defines collaboration patterns (multi-collective) | "Any cross-collective EPR custody work"; "Future shared-stewardship patterns" |
| Signal envelope pattern (dna-signal-as-epr-envelope) | "All future DNA signals that need EPR-shape envelope semantics" |

If unclear, default `informs:` to `["See related architecture specs for downstream coupling"]` and flag in the agent's return for operator review.

## Task: Normalize all 14 specs

**Files:** All 14 listed above; each modified.

- [ ] **Step 1: For each of the 14 files, read the current frontmatter**

```bash
for f in genesis/docs/content/elohim-protocol/architecture/2026-*-*.md; do
  echo "=== $f ==="
  awk '/^---$/{c++; if(c==2) exit} c>=1{print}' "$f"
  echo
done
```

Capture: which legacy field names exist, which contract fields are missing, what spec scope is (read first paragraph if needed for `informs:` inference).

- [ ] **Step 2: For each file, draft normalized frontmatter**

Apply the legacy-to-contract mapping. Preserve every value (just rename fields and add `tier:` + `informs:`).

- [ ] **Step 3: Apply edits via Edit tool**

For each file, the Edit replaces the OLD frontmatter block with the NEW frontmatter block. The old_string is the entire `---\n...frontmatter content...\n---` block. The new_string is the normalized block. Preserve content; only rename fields and add the two required ones.

- [ ] **Step 4: Validate each file's frontmatter parses**

```bash
for f in genesis/docs/content/elohim-protocol/architecture/2026-*-*.md; do
  python3 -c "
import sys, re
content = open('$f').read()
m = re.match(r'^---\n(.*?)\n---\n', content, re.DOTALL)
if not m:
    print('NO FRONTMATTER: $f')
    sys.exit(1)
import yaml
try:
    fm = yaml.safe_load(m.group(1))
    assert 'tier' in fm and fm['tier'] == 'architecture', 'missing tier: architecture'
    print('OK: $f')
except Exception as e:
    print('FAIL: $f — \\$e')
" 2>/dev/null || echo "  (python or yaml not available — manual review needed)"
done
```

Expected: 14 lines of "OK"; no FAIL or NO FRONTMATTER.

If python/yaml not available, fall back to manual visual inspection — each file's frontmatter must:
- Start with `---` on line 1
- End with `---` somewhere within the first ~30 lines
- Contain `tier: architecture`
- Contain at least one of `realizes:` and `informed-by:` (most have both)
- Not contain `narrative spine:`, `narrative_spine:`, `related:`, `related specs:` (these are now renamed)

- [ ] **Step 5: Commit**

```bash
git add genesis/docs/content/elohim-protocol/architecture/2026-*-*.md
git commit -m "spec(architecture): normalize frontmatter on 14 migrated specs to architecture contract"
```

## Self-Review

**1. Did I preserve all existing frontmatter values?** — Memory anchors, authors, dates, pillar coupling, defers all retained.

**2. Did I rename consistently?** — `narrative spine:` → `realizes:`; `related:` / `related specs:` → `informed-by:`; no orphan fields.

**3. Did I add `tier: architecture` to every file?** — All 14 have it.

**4. Did I add a reasonable `informs:` to every file?** — Inferred from scope; uncertain cases flagged.

**5. Did I leave body content untouched?** — Only frontmatter changed; no edits below the second `---`.

## Quality bar checklist (for the orchestrator's review)

- [ ] All 14 files have `tier: architecture`
- [ ] All 14 files have `realizes:` (or marked N/A explicitly for substrate-foundational specs)
- [ ] All 14 files have `informed-by:` (or marked N/A explicitly for ground-floor specs)
- [ ] All 14 files have `informs:` (operator may revise inferred content)
- [ ] No legacy field names remain (`narrative spine:`, `related:`, `related specs:`)
- [ ] Memory anchors, defers, authors, dates, status, pillar coupling are all preserved
- [ ] Body content is unchanged

## Notes / known special cases

- `2026-05-20-wave3-valueflows-hrea-interop-design.md` and `2026-05-23-multi-collective-collaboration-epr-design.md` have unusual frontmatter shape — using H2 sections in body for "Source references:" / "Memory anchors:" / "Plan kinship:" rather than YAML. **Leave body H2 sections alone**; only add the YAML frontmatter at the top with `tier: architecture` + `realizes:` + `informed-by:` + `informs:`. The body H2 sections become alternate cross-reference surfaces and are preserved.

- `2026-04-18-experience-story-epr-design.md` uses H2/H3 in-body for frontmatter-equivalent fields. Same treatment: add YAML frontmatter at the top; leave body untouched.

## Execution handoff

Single agent dispatch. Inline execution. After the 14 files normalize and commit, the agent returns a brief summary of: which files needed `tier:` added, which had `informs:` inferred (with the inferred values for operator review), any files where the legacy frontmatter was unusual enough to warrant operator review.
