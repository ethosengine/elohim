---
id: "backlog-eprfs-ci-research-cite-into-uninitialized-brit-submodule"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "eprfs CI reds research_corpus_has_no_dead_cite_after_the_address_crossing because a genesis/research cite targets a file inside the brit submodule, which the eprfs pipeline does not check out, so a present-at-pin target reads as DEAD-CITE"
slug: "eprfs-ci-research-cite-into-uninitialized-brit-submodule"
written: "2026-11-08"
author: "artifact-store sprint 2 orchestrator (claude-opus-5-5, shem)"
status: "resolved"
priority: "medium"
jobs: [ci, eprfs]
cluster: "citation-apparatus"
tags: [cites, submodule, ci, eprfs, dead-cite, false-red]
---

**Observed 2026-11-08.** `elohim-eprfs/dev` #103 and #104 each fail one test out of about 2,100:
`elohim-epr-cli::flow_cites research_corpus_has_no_dead_cite_after_the_address_crossing`
(`elohim/eprfs/epr-cli/tests/flow_cites.rs:1052`). #104 was dispatched by orchestrator #2260,
which therefore reads FAILURE even though elohim-edge #1840 and #1841 rolled SUCCESS.

```
genesis/research/commons-composition-at-scale-2026-10-15.md carries a DEAD-CITE:
- DEAD-CITE (target missing): elohim/brit/docs/specs/2026-06-29-epr-meta-composition-snapshot-canonical-cites-design.md
```

The target exists at the brit pin `origin/dev` names (10d18eacc, checked with `git cat-file -e`).
Locally `epr flow cites verify` on the same doc reads "all cites content-addressed + resolvable".
So the cite is sound. The eprfs CI workspace leaves `elohim/brit` uninitialized, and the
verifier reports a target inside an uninitialized submodule as missing.

**Cure options (pick one):**
1. The cites verifier reads a target under an uninitialized gitlink from the pinned commit's
   tree, or reports it as HELD-CITE (unverifiable here) rather than DEAD-CITE. A test covers a
   cite into an uninitialized submodule.
2. The eprfs pipeline initializes the submodules this test reads (`elohim/brit`). This is the
   cheaper fix, but it adds clone time to every eprfs run.
3. The test skips docs whose cites point into a gitlink that is not checked out, and prints the
   skip.

Option 1 matches the HELD/DEAD distinction the verifier already draws. A missing checkout is
"not verifiable here", not "target missing".

**2026-11-08 21:40Z: owned by ethosengine.** They are fixing the verifier with option 1: a submodule path is judged against the pinned gitlink, reads HELD when it is unavailable, and never DEAD. Close this row when their push lands and eprfs dev runs green.

**Resolved 2026-11-08 by ethosengine fa8bc213f** (on origin/dev 39e897c1b): the cites verifier judges a cite into a submodule at the pinned gitlink and never reports DEAD for a tree it cannot see. Confirm on the next eprfs dev run.
