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

## `@act:<i|ii|iii|host>` — the a2o feature-file act baseline (mirrors substrate-scope.ts)

`.feature` files carry no frontmatter; a2o's runtime arm (genesis/a2o/src/framework/fixtures/substrate-scope.ts)
resolves an `@act:i|ii|iii|host` tag against that ACT's OWN lane-contract file
(`genesis/manifests/cluster-state.act1-household.yaml` / `.act2-neighbourhood.yaml`; `iii` reads the live
`cluster-state.yaml` itself, plus `shem` by definition; `host` needs no substrate at all) — because that lane
file is what the mesh stage actually points the runtime at when exercising that act
(`ELOHIM_CLUSTER_STATE_PATH_OVERRIDE`), not the live/default `cluster-state.yaml` a non-mesh dev loop reads.
`act_baseline_caps()` below is this module's mirror of `actBaselineCaps()`: it ADDS a capability to what a
`.feature`'s `@requires:<cap>` tag can be satisfied by, on top of live cluster-state.yaml — never removes one.
That is deliberately narrower than substrate-scope.ts's full `unavailableRequiredCaps()` (which also gates on
lane vocabulary and env overrides): the planning layer only needs "would this feature run in ITS OWN act's
lane", not full runtime gating, so a false HELD (a feature that only fails to run in the wrong/default lane)
never benches work the mesh stage can actually exercise.
"""
from __future__ import annotations

import re

_REQUIRES_TAG = re.compile(r"@requires:([a-z0-9][a-z0-9-]*)")
_ACT_TAG = re.compile(r"@act:(i|ii|iii|host)\b")

# Which cluster-state file declares each act's BASELINE — mirrors substrate-scope.ts's
# ACT_BASELINE_FILES. `host` -> None (no substrate at all, so no baseline file to read).
ACT_BASELINE_FILES = {
    "i": "cluster-state.act1-household.yaml",
    "ii": "cluster-state.act2-neighbourhood.yaml",
    "iii": "cluster-state.yaml",
    "host": None,
}


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


def feature_act(text: str) -> str | None:
    """The `@act:<i|ii|iii|host>` tag a .feature file declares, or None if it declares none. Scans only
    the tag line(s) ABOVE the first `Feature:` keyword (gherkin `#` comments skipped, so prose mentioning
    the tag isn't read as one) — mirrors scope-reconcile.py's `_feature_requires` tag-line walk and
    substrate-scope.ts's `actFromTags()`/ACT_TAG. First tag wins if a doc mistakenly declares two (an
    authoring error the a2o runtime warns about; the planning layer just takes the first, same as the
    runtime)."""
    for ln in (text or "").splitlines():
        stripped = ln.lstrip()
        if stripped.startswith("Feature:"):
            break
        if stripped.startswith("#"):
            continue
        m = _ACT_TAG.search(stripped)
        if m:
            return m.group(1)
    return None


def act_baseline_caps(act: str, manifests_dir) -> set:
    """Caps `<act>` provides BY DEFINITION — the `available: true` resources in the act's OWN lane-contract
    file (mirrors substrate-scope.ts's `actBaselineCaps()`). `host` -> empty set (no substrate at all).
    `iii` reads the live `cluster-state.yaml` (its own baseline IS the live fleet) plus `shem`, its stage
    by definition. An unreadable/missing act file -> empty set — fail-open, a missing file must never
    invent a gate (mirrors the TS side's `catch { return []; }`).

    This is additive-only scope: callers UNION this into an already-known `available` set for `.feature`
    docs (never replace it), so a cap the live cluster-state.yaml withholds from the shared/default lane
    on purpose (e.g. `owned-substrate: false` there, so destructive scenarios don't run against alpha —
    see cluster-state.yaml's own note) is still rescued for a scenario tagged with the act whose OWN lane
    contract grants it — that scenario is exercised only via that act's lane (the mesh stage's
    ELOHIM_CLUSTER_STATE_PATH_OVERRIDE), never against the default/live lane, so gating it on the
    live/default lane's deliberate withholding is a false HELD, not a real one."""
    from . import cluster_state as _cs  # local import: only paid when a .feature actually declares an act

    fname = ACT_BASELINE_FILES.get(act)
    if fname is None:
        return set()
    state = _cs.load(manifests_dir / fname)
    caps = set(state.available_names())
    if act == "iii":
        caps.add("shem")  # Act III's stage, by definition (mirrors substrate-scope.ts)
    return caps
