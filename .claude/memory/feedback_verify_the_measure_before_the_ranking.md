---
name: feedback_verify_the_measure_before_the_ranking
title: Verify a drift audit's measure before acting on its ranking
description: "A drift audit can be the dominant source of its own findings — spot-check the top-ranked surface against disk before the ranking drives work."
metadata:
  type: feedback
---

Before acting on any audit's **ranked** output, verify a sample of the top-ranked item's findings
against disk. A ranking is only as trustworthy as its resolver, and a mis-resolving check does not
degrade gracefully — it *reorders* the list, so the loudest surface is the one the tool
misunderstands rather than the one that has drifted.

**Why:** on 2026-07-30 `substrate-currency-audit.py` reported 586 path-existence findings across 98
gospel surfaces. 80% were artifacts: paths were resolved against the repo root ONLY, so every path
cited relative to the citing file's own directory (the per-crate CLAUDE.md convention) and every path
cited relative to a context set earlier in the prose read as missing. `rust-architect.md` ranked #1
with 52 findings and had ~zero real path drift; `doorway/doorway-service/CLAUDE.md` ranked #2 and
every one of its 23 findings was a correctly-cited relative path. The genuinely-drifted surfaces sat
below them. Non-path token classes inflated it further — HTTP routes, libp2p protocol ids
(`/elohim/sync/2.0.0`), slash-commands (`/converge`), elided abbreviations (`app/.../foo.ts`),
vocabulary triples (`own/ownership/sovereign`). Separately the process-status regex flagged
`in-flight` where it is a *proper noun* ("the in-flight hook") and flagged lines that **quote** the
anti-pattern in order to teach it — so the discipline's own statement of itself became its
top-ranked violation.

**How to apply:** spot-check 3-5 findings from the #1 surface with `ls`/`test -e` before dispatching
any agent or writing any rewrite. If they resolve, fix the resolver first and re-rank — rewriting
from a noisy list is worse than not rewriting, because agents "correct" paths that were already
right. When you fix a resolver, resolve against every convention the corpus actually uses
(repo-root, surface-relative, context-relative suffix, absolute host path) and exclude token classes
that are never filesystem claims. Keep drift audits **under**-suppressing rather than over-: a
residual false positive costs one check, a suppressed real one goes unfixed. Sibling discipline:
[[feedback_sprint_dod_includes_prepush_gates]] (a green number is not a passed gate),
[[feedback_pvc_deferral_hides_gate_debt]] (deferred ≠ passed).
