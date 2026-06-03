"""env_scope — the gap-granular substrate-scope resolver (shared by decompose / placement-audit / scope-reconcile).

The scope model is gap-granular, isomorphic with a2o's per-scenario `@requires:<cap>` tags: a plan's gaps each
resolve a `requires_env`, defaulting to the document-level frontmatter value and overridable per gap. A gap is
BLOCKED-BY-ENV iff a cluster-TRACKED required capability is unavailable — independent of whether its parent doc
sits in `held/`. This is what honors "iroh ≠ shem": a mixed plan keeps its household-testable gaps pickable
while only its cross-node-assertion gaps wait for the unavailable canvas.

Convention (keeps scope-reconcile's hold decision simple):
  - doc-level `requires_env` in frontmatter ⇒ the default for EVERY gap → a UNIFORMLY-blocked doc → held whole.
  - a MIXED plan declares NO doc-level `requires_env` and tags only the divergent gaps `@requires:<cap>` → it
    stays on the plate; only the tagged gaps are BLOCKED-BY-ENV in the budget.
"""
from __future__ import annotations

import re

_REQUIRES_TAG = re.compile(r"@requires:([a-z0-9][a-z0-9-]*)")


def parse_requires_env(value) -> list:
    """Normalize a frontmatter `requires_env` value → list[str]. Handles a block list (already a list), an
    inline list `[a, b]` / `[]` (the minimal parser reads it as a string), or a scalar; strips inline comments
    + quotes. Mirrors scope-reconcile.requires_env's frontmatter handling."""
    if isinstance(value, list):
        items = value
    elif isinstance(value, str):
        s = re.sub(r"\s+#.*$", "", value.strip()).strip()
        if s.startswith("[") and s.endswith("]"):
            inner = s[1:-1].strip()
            items = list(inner.split(",")) if inner else []
        else:
            items = [s] if s else []
    else:
        items = []
    return [x.strip().strip("'\"") for x in items if isinstance(x, str) and x.strip()]


def requires_tags(text: str) -> list:
    """Parse `@requires:<cap>` markers from a gap's source text → list[str] (the per-gap override source —
    isomorphic with a2o's per-scenario `@requires:` tags). Empty if none."""
    return _REQUIRES_TAG.findall(text or "")


def resolved_requires_env(gap_env, doc_env) -> list:
    """gap-level requires_env if explicitly set (non-empty), else inherit the document default."""
    if gap_env:
        return list(gap_env)
    return list(doc_env or [])


def gap_blocked(resolved, available, known) -> bool:
    """A gap is BLOCKED-BY-ENV iff a cluster-TRACKED required cap is unavailable. Caps not in `known` (e.g. a
    stray a2o fixture tag) don't gate; an empty or fully-satisfied requirement is not blocked."""
    relevant = [c for c in resolved if c in known]
    return bool(relevant) and not set(relevant).issubset(set(available))
