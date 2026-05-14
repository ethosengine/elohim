---
name: correct-reindex-grows-index
description: Re-indexing orphaned topic files into MEMORY.md can grow the file even after tightening 10 entries — the orphans were always-load-bearing, just invisible. Tightening alone insufficient; umbrella consolidation needed.
type: feedback
---

Wave 4 of the first memory-team ceremony: librarian was tasked with tightening MEMORY.md to budget (24.4 KB) AND re-indexing 12 orphaned topic files (files on disk but missing from the index). After tightening 10 longest entries (-1.4 KB) AND re-indexing 12 orphans (+2.2 KB), net result: MEMORY.md grew from 26.2 KB → 28.5 KB. Still over budget.

**Why:** The orphans weren't fat — they were always load-bearing, just invisible to the index. Folding them in correctly is *required* for index integrity (every file → index entry; every entry → real file). Tightening can only reduce by character-count-per-entry; once entry shape (link + em-dash hook) is at its floor, the path forward is umbrella consolidation (multiple entries fold into one umbrella with sub-links) or graduation (entries graduate to stories and archive).

**How to apply:**

- When MEMORY.md is over budget AND the corpus has orphan files, expect re-index to GROW the file. Don't treat that as failure; it's a successful correctness pass.
- Tightening alone is bounded by entry shape. The floor is roughly `- [Title](file.md) — hook` ≈ 100-150 chars/entry; below that, hooks become uninformative.
- Real compression comes from: (a) umbrella entries that fold related items under one link with sub-bullets, (b) graduation to stories (one story can carry 6+ memory entries), (c) archive-with-pointer (memorialize forensic value, drop from active index).
- The librarian's "tighten 10 entries" authority is tiny-correction; umbrella/graduate/archive decisions require operator review or storyteller dispatch.

Pairs with [[project_memory_lifecycle_comet_shape]], [[project_wisdom_resolves_into_epics]], [[feedback_first_memory_team_ceremony]].
