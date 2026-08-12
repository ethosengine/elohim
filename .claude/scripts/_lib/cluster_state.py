"""_lib.cluster_state — the ONE parser for genesis/manifests/cluster-state.yaml.

Line-based, not pyyaml: the file carries multi-line unquoted `note:` scalars that strict YAML
rejects (see the file's own header comment). This module replaces THREE independent hand-rolled
line-regex walks that had drifted into slightly different shapes (agentic-context-tooling-
consolidation-queue.md item 6):

  - scope-reconcile.py `_parse_cluster()` / `_parse_provides()` — correctly scoped to the
    `resources:` block, two separate walks (availability + provides_node_types).
  - placement-audit.py `load_cluster_state()` — UNSCOPED: no `resources:` block-boundary check
    at all, so any 2-space-indented bare key followed by a 4-space `available:` line ANYWHERE in
    the file (before OR after `resources:`) would be read as a resource. No live disagreement was
    found against the current file (nothing outside `resources:` matches the shape), but it is a
    latent bug the moment the file gains another nested block — pinned by
    `__tests__/cluster_state_test.py` (both the before- and after-`resources:` decoy).
  - focus-baseline.py `_cluster_detail()` — correctly scoped, but a third independent regex walk
    (role only, no provides_node_types).

Because the budget reader (placement-audit) and the mover (scope-reconcile) used different
parsers, they could in principle disagree about which capabilities are available — exactly how
the scope homes drifted before (see CLAUDE.md's `[[scope-flag-beats-prose-note]]`). One parser
now backs both.

THREE deliberate semantic decisions, each pinned by a test — read these before editing a regex:

1. **A column-0 comment is NOT a block terminator.** Only a real top-level key (`^[A-Za-z]`)
   ends the `resources:` block. Merging the three parsers naively adopted `_parse_provides`'s
   `^[A-Za-z#]`, which is the NARROWEST of the four regexes: with it, an operator writing an
   ordinary `# ---- remote capabilities ----` section comment at column 0 between two resources
   silently truncates the resource list — and `scope-reconcile.py --apply` then `git mv`s every
   spec/plan/feature below it into `held/` and flips `deployments.json` `suspended` flags.
   `cluster-state.yaml` already carries five comment blocks. A comment is prose, not structure.

2. **A resource must DECLARE `available:` to be an availability CLAIM.** `available_map()` (the
   placement-audit / focus-baseline `known` source) contains only resources that actually carry
   an `available:` line; a block with just `role:` is a placeholder, not a runtime claim, and is
   omitted. This preserves the pre-consolidation behaviour of both readers, and it is the
   conservative call: the file format has NO way to distinguish "declared capability whose
   availability we forgot to state" from "planned capability that was never a runtime claim", so
   we refuse to newly bench work on an assertion nobody made. cluster-state's own header says the
   gate is `requires_env ⊆ the available: true resources here` — availability is asserted by the
   `available:` key, and evidence-backed flips ("mirror the probe, never aspirational") is the
   surrounding discipline.

   RESIDUAL ASYMMETRY (deliberate, documented, tested): `all_names()` — scope-reconcile's `known`
   — still contains EVERY resource name, including undeclared ones. That is unchanged HEAD
   behaviour and it is load-bearing for two things `available_map()` does not do: `--set`
   resource validation, and unknown-cap vocab-drift detection (an unknown cap conservatively
   blocks held→live escape, so demoting a role-only resource to "unknown" would block MORE, not
   less). Net effect on the one edge case: a doc requiring a role-only resource is held by the
   mover but still counted OPEN by the budget reader. Both old readers behaved exactly this way;
   closing it means changing the MOVER's policy, which is a scope decision, not a parser one.
   Pinned by `test: role-only resource — mover gates, budget reader does not`.

3. **A repeated declaration MERGES, never RESETS — and the merge policy is PER FIELD.** Re-declaring
   `shem:` used to reset the record, wiping an `available: true` from the first block (→
   `available_names()` loses it → spurious `held/` moves). The same applies to two `available:`
   lines inside ONE block. Merging replaces the reset; each field merges as follows:

   - `available:` — **TRUE-WINS**: if ANY `available: true` line appears for a resource, in any of
     its blocks and at any position, the resource is available; otherwise the FIRST non-true scalar
     stands (so `degraded` is not clobbered by a later `false`). `declared_available` is set by any
     `available:` line at all. True-wins is EXACTLY HEAD `scope-reconcile._parse_cluster`'s
     `avail.add(cur)` set-union. HEAD `placement-audit.load_cluster_state` and HEAD
     `focus-baseline._cluster_detail` were both LAST-wins for availability, so true-wins matches
     them too in the ordering that actually happens — a stale `available: false` left standing
     above a freshly-added `available: true` — and deliberately diverges from them in the opposite
     ordering (`true` then a non-true), where we keep `true`. That divergence is the whole point of
     not resetting, and it is the PERMISSIVE direction: on a scope gate the expensive error is
     wrongly benching work (`scope-reconcile --apply` `git mv`s every spec/plan/a2o feature that
     requires the cap into `held/` and flips `deployments.json` `suspended` flags), while the cheap
     error is leaving a doc on the plate that a probe will red anyway. A parser must never be the
     thing that takes an available capability away.
     FIRST-wins here would be strictly worse than the reset it replaced: with `false` above `true`
     it reports UNAVAILABLE where the reset, and all three HEAD readers, report AVAILABLE.
   - `role:` — **FIRST-wins**, matching HEAD `focus-baseline._cluster_detail`'s `if r and cur not
     in roles` guard (the only HEAD reader that read `role` at all). Role is human-readable WHY
     text; it gates nothing.
   - `provides_node_types:` — **UNION** (accumulate, order-preserving, deduped), matching HEAD
     `scope-reconcile._parse_provides`'s `mapping.setdefault(nt, set()).add(cur)`. It can only make
     a human placeable, never less.

   Duplicates are recorded on `ClusterState.duplicate_keys` for any consumer that wants to warn;
   nothing gates on them (a parse crash here breaks every session — these scripts run on every
   Edit/Write via hooks). Both orderings of every field are pinned through the CALL SITES by
   `__tests__/cluster_state_test.py`.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

_UPDATED_RE = re.compile(r"^updated:\s*(\S+)")
_RESOURCES_START_RE = re.compile(r"^resources:\s*$")
# A top-level KEY ends the resources block. A column-0 `#` comment does NOT — see decision 1 above.
_BLOCK_END_RE = re.compile(r"^[A-Za-z]")
_RESOURCE_KEY_RE = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
_AVAILABLE_RE = re.compile(r"^    available:\s*(\S+)")
_ROLE_RE = re.compile(r"^    role:\s*(.+?)\s*$")
_PROVIDES_RE = re.compile(r"^    provides_node_types:\s*\[(.*)\]\s*$")


@dataclass
class Resource:
    name: str
    available: str = ""  # raw scalar: "true" / "false" / "degraded" / "" if NO available: key
    declared_available: bool = False  # was an `available:` line actually present? (decision 2)
    role: str = ""
    provides_node_types: list = field(default_factory=list)

    @property
    def is_available(self) -> bool:
        """available == true (degraded/false/missing are NOT available) — mirrors every prior copy's
        `available:\\s*true\\b` comparison for the unquoted scalars this file actually uses."""
        return self.available == "true"


@dataclass
class ClusterState:
    resources: dict  # name -> Resource
    updated: str = "?"
    duplicate_keys: list = field(default_factory=list)  # resource names declared more than once

    def all_names(self) -> set:
        """ALL resource names — the env-availability VOCABULARY (a `requires_env` cap not in this
        set, e.g. an a2o `@requires:doorway` fixture tag, is not a hardware-availability concern).
        Includes resources with no `available:` key; see decision 2's residual-asymmetry note."""
        return set(self.resources)

    def declared_names(self) -> set:
        """Resource names that actually CLAIM an availability (`available:` key present) — the
        cluster-TRACKED set for gating purposes (decision 2)."""
        return {n for n, r in self.resources.items() if r.declared_available}

    def available_names(self) -> set:
        """Names where available == true."""
        return {n for n, r in self.resources.items() if r.is_available}

    def available_map(self) -> dict:
        """name -> raw available scalar string, for resources that DECLARE `available:` (decision 2:
        a role-only block makes no runtime claim and must not silently bench work). Kept raw rather
        than boolean so degraded/false render distinctly, not collapsed to unavailable."""
        return {n: r.available for n, r in self.resources.items() if r.declared_available}

    def roles(self) -> dict:
        """name -> role string, only for resources that declare one."""
        return {n: r.role for n, r in self.resources.items() if r.role}

    def provides_map(self) -> dict:
        """nodeType -> set(resource name) — the deployments.json arm of the reconciler (each
        resource declares which human `nodeTypes` it can place)."""
        mapping: dict = {}
        for name, r in self.resources.items():
            for nt in r.provides_node_types:
                mapping.setdefault(nt, set()).add(name)
        return mapping


def load(path: Path) -> ClusterState:
    """Parse cluster-state.yaml. Missing file -> empty ClusterState (fail-open, mirrors every
    prior copy: an absent file means 'nothing declared available', never a crash)."""
    if not path.is_file():
        return ClusterState(resources={})
    resources: dict = {}
    duplicates: list = []
    cur = None
    in_resources = False
    updated = "?"
    for ln in path.read_text(encoding="utf-8", errors="replace").splitlines():
        u = _UPDATED_RE.match(ln)
        if u:
            updated = u.group(1)
        if _RESOURCES_START_RE.match(ln):
            in_resources = True
            continue
        if not in_resources:
            continue
        if _BLOCK_END_RE.match(ln):
            in_resources, cur = False, None
            continue
        m = _RESOURCE_KEY_RE.match(ln)
        if m:
            cur = m.group(1)
            if cur in resources:  # decision 3: MERGE into the existing record, never reset
                duplicates.append(cur)
            else:
                resources[cur] = Resource(name=cur)
            continue
        if cur is None:
            continue
        rec = resources[cur]
        a = _AVAILABLE_RE.match(ln)
        if a:
            # decision 3: TRUE-WINS. Any `available: true` for a resource makes it available, no
            # matter which block or line it sits on — exactly HEAD scope-reconcile's `avail.add(cur)`
            # set-union. Never let a later (or an earlier) non-true scalar take an available cap away.
            val = a.group(1).strip().strip("'\"")
            if not rec.declared_available or (val == "true" and rec.available != "true"):
                rec.available = val
            rec.declared_available = True
            continue
        r = _ROLE_RE.match(ln)
        if r:
            if not rec.role:  # first explicit wins (focus-baseline's old `cur not in roles` guard)
                rec.role = r.group(1).split("#")[0].strip()
            continue
        p = _PROVIDES_RE.match(ln)
        if p:
            for nt in p.group(1).split(","):
                nt = nt.strip().strip("'\"")
                if nt and nt not in rec.provides_node_types:  # accumulate, dedup
                    rec.provides_node_types.append(nt)
    return ClusterState(resources=resources, updated=updated, duplicate_keys=duplicates)
