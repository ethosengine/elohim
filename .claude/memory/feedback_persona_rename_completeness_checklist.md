---
name: persona-rename-completeness-checklist
description: "A persona rename touches content + filenames + generated indices + test fixtures + cross-doc references. Updating canonical sources is necessary but not sufficient; generators don't clean up old files. Audit per surface before claiming the rename is complete."
metadata:
  node_type: memory
  type: feedback
---

When renaming a persona (or any first-class identity used across genesis stories, simulacra, and gherkin), updating `genesis/data/humans/*.md` + regenerating `humans.json` is the *beginning* of the work, not the end. Generators write new files but do not delete old ones, so stale files linger with the old name. Verified twice now: the 2026-05-14 chronicle claimed the `timothy → terrance` rename completed with "zero stale references"; a coherence pass on 2026-05-17 found eight residual surfaces.

**Why:** Renames cascade across at least five surface classes, each with its own currency lifecycle:

1. **Canonical content** — `genesis/data/humans/*.md`, `humans.json`, `relationships.md`, `humans.schema.json`. Usually updated first because they're the source of truth the rename was *about*.
2. **Generated downstream files with persona-derived filenames** — e.g. `genesis/data/account-packages/<persona>.json`, `genesis/data/lamad/content/humans/human-<persona>.json`. Generators key filenames off the canonical id, so a new file appears under the new name *and the old file is never deleted*.
3. **Filenames with persona slugs that aren't auto-generated** — e.g. `genesis/a2o/features/.../m1-matthew-<persona>-delivery.feature`, `genesis/a2o/scripts/__tests__/fixtures/console-<persona>-errors.json`, story files in `genesis/data/stories/`. Often the *contents* are updated by hand or codemod, but the filename gets missed.
4. **Test / orchestrator fixtures** — string literals like `'human-<persona>-tutor'` or pod hostnames like `'elohim-<persona>-alpha'` in `*.test.mjs`. Grep-discoverable but won't fail loudly until a fixture flow runs.
5. **Cross-doc references** — plans, specs, design docs, chronicles that name the old artefacts by path string. These rot quietly.

**How to apply:** Before claiming a rename is complete, run this per-surface audit:

```bash
# 1. Find every file whose NAME contains the old persona slug
find . -type f -name "*<old>*" -not -path './node_modules/*'

# 2. Find every string occurrence (excluding legitimate content like Bible refs)
grep -rn "<old>\|<Old>" --include="*.md" --include="*.json" --include="*.py" \
  --include="*.ts" --include="*.mjs" --include="*.feature" .

# 3. Cross-check the generated indices vs current canonical state
diff <(jq -r '.humans[].id' genesis/data/humans/humans.json | sort) \
     <(ls genesis/data/lamad/content/humans/human-*.json | sed 's|.*/human-||;s|\.json$||;s|^|human-|' | sort)
```

Each residual hit needs explicit closure (rename, edit, or "leave — this is legitimate content"). Never write a "zero stale references" chronicle entry without running the audit *after* the rename, not just inspecting the canonical sources.

Related: [[feedback_signature_changes_grep_callers]] (small-edit / wide-blast pattern in Rust); [[project_memory_in_repo_two_tier]] (substrate-currency: filenames and indices are part of the substrate, not below it). Companion incident: chronicle correction at `genesis/data/timeline/chronicle/2026-05-14-first-memory-team-ceremony.md` line 39 (updated 2026-05-17).
