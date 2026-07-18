---
id: superpowers-held-stop-marker
status: active
cites:
  - scope-tree-reconciler-design | the reconciler that files docs here when a requires_env cap is unavailable and git mv-s them back when it returns | sha256:5332b1422eb86eb2 | path: genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md
  - placement | held/ is the BLOCKED-BY-ENV state in the placement contract — held, never regressed; partial availability is the steady state | sha256:95be31e6724bb9f5 | path: genesis/docs/PLACEMENT.md
---

# ⛔ STOP — held artifacts (require unavailable hardware)

The docs under this `held/` tree declare a `requires_env:` capability that is **not currently available**
(see `genesis/manifests/cluster-state.yaml`). They are sequestered here, OUTSIDE the planner/runner scan
path, so they don't false-fail or consume planning focus. This is **held, NOT deleted or regressed** —
partial availability is the steady state.

- Do NOT edit or "fix" these as if broken. They are correct; the hardware is absent.
- Inbound `cites:` to these resolve as **HELD-CITE** (content-addressed; not dead) — do not delete the link.
- They move back automatically when their capability returns: `scope-reconcile.py --apply`.

Managed by `.claude/scripts/memory-kit/scope-reconcile.py`. The mover, not a human, files things here.
