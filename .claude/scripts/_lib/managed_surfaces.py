"""managed_surfaces — the EDIT-TIME axis of the managed-memory discipline.

`.claude/subject-routing.yaml` is the CLASS axis: what kind of WORK routes to which home, read at the
brainstorm/plan FRONT-fire and the decompose BACK-fire. THIS module is the orthogonal SURFACE axis:
given a FILE being touched, which managed-memory surface class is it, and which discipline + tooling
apply at edit time. It exists because the 2026-06-05 gospel-cites episode showed each hook hardcoding
its own scope (cite-seal-signal knew doc-roots, not gospels) — scope drift recurs per-hook unless ONE
registry answers "is this file managed, and how."

Consumers:
  - .claude/hooks/managed-surface-context.py   PreToolUse Edit|Write — inject the discipline BEFORE the edit
  - .claude/hooks/cite-seal-signal.py          PostToolUse — born-linked nudge scope (in_cite_graph)
  - cite-gen --seal-all / cites-migrate        sweep scope for the cite graph

Path patterns anchor to the HOMES subject-routing.yaml names (write_location / gospel_homes /
decomposition_flow); the cross-check in __tests__/managed_surfaces_test.py fails if the two axes drift.
First match wins — order entries most-specific-first. Pure-stdlib; composes _lib.cite_graph for the
gospel matcher. Spec: 2026-06-05-managed-surface-edit-discipline-design.md.
"""
from __future__ import annotations

from fnmatch import fnmatch
from pathlib import Path

from _lib import cite_graph as _cg

_CITE_TOOLING = (
    "cites are TOOL-generated envelopes (slug | desc | sha256 [| status:] [| path:]) — never hand-write "
    "a slug/fingerprint/path"
)

# Ordered registry — first match wins. `match` = fnmatch patterns on the repo-relative posix path
# (fnmatch's `*` crosses `/`, so `genesis/docs/*.md` matches the whole subtree); `matcher` = a named
# special matcher (gospel walk-scope). `in_cite_graph` = the file is a slug-envelope graph member
# (doc-roots + gospel CLAUDE.mds); entity/story/feature surfaces deliberately stay plain-path targets.
SURFACES: list[dict] = [
    {
        "key": "routing-config",
        "label": "axis manifest (hand-tuned constitution)",
        "match": [".claude/subject-routing.yaml", "genesis/manifests/cluster-state.yaml"],
        "in_cite_graph": False,
        "discipline": "Hand-tuned axis manifest: subject-routing.yaml is the class axis, cluster-state.yaml "
                      "the env axis (evidence-backed flips only — mirror the probe, never aspirational). "
                      "A flip cascades: run scope-reconcile.py --apply after changing cluster-state.",
        "tools": [".claude/scripts/scope-reconcile.py --apply"],
    },
    {
        "key": "process-gospel",
        "label": "process operating gospel",
        "match": [".claude/CLAUDE.md", ".claude/*/CLAUDE.md", ".claude/scripts/memory-kit/LIFECYCLE.md",
                  "genesis/orchestrator/README.md", "genesis/data/timeline/CONVENTIONS.md"],
        "in_cite_graph": False,
        "discipline": "Process operating gospel (a subject-routing gospel_home): durable operating-discipline "
                      "residue lands here via gospel-diff at decompose. Keep it the single home for its "
                      "subdomain; route detail to specs/history, not into the gospel.",
        "tools": [".claude/scripts/memory-kit/claude-md-audit.py"],
    },
    {
        "key": "gospel",
        "label": "contextual gospel (CLAUDE.md, cite-graph member)",
        "matcher": "gospel-claude-md",
        "in_cite_graph": True,
        "discipline": f"Contextual gospel — a managed-memory surface IN the cite graph: {_CITE_TOOLING}. "
                      "Concern-routing lines reference slugs from this file's own cites:. After editing "
                      "cites, run cite-gen --seal; author relationship hints with cite-describe. Keep rails "
                      "short — detail belongs in the cited doc. claude-md drift accumulators watch this scope.",
        "tools": ["python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>",
                  "python3 .claude/scripts/memory-kit/cite-describe.py <file> '{\"<ref>\":\"<hint>\"}'"],
    },
    {
        "key": "memory",
        "label": "native memory entry",
        "match": [".claude/memory/*.md"],
        "in_cite_graph": True,
        "discipline": "Native memory stays LIGHT (2026-06-03 pair-off): durable knowledge GRADUATES to its "
                      f"managed home per subject-routing.yaml — don't pile it up here. {_CITE_TOOLING}.",
        "tools": ["python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>",
                  "python3 .claude/scripts/memory-kit/memory-coherence-audit.py"],
    },
    {
        "key": "spec",
        "label": "spec (ACTIVE source, subject-routed)",
        "match": ["genesis/docs/superpowers/specs/*.md"],
        "in_cite_graph": True,
        "discipline": f"Spec — born-linked: {_CITE_TOOLING}; frontmatter carries class: per "
                      "subject-routing.yaml. Data-entity designs pass the p2p-design-gate. Terminal status "
                      "→ decompose to zero residue (decompose.py), never parked in the live tree.",
        "tools": ["python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>",
                  "python3 .claude/scripts/memory-kit/decompose.py <file>",
                  "python3 .claude/scripts/memory-kit/spec-coherence-index.py --query '<topic>'"],
    },
    {
        "key": "plan",
        "label": "plan (ACTIVE source, placement-watched)",
        "match": ["genesis/docs/superpowers/plans/*.md", "genesis/docs/plans/*.md"],
        "in_cite_graph": True,
        "discipline": f"Plan — born-linked: {_CITE_TOOLING}. Landing a terminal status (landed/superseded/"
                      "abandoned) queues decompose-due (placement drift): a finished plan decomposes to zero "
                      "residue. Gap-granular env scope: prefer per-gap @requires:<cap> over doc-level holds.",
        "tools": ["python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>",
                  "python3 .claude/scripts/memory-kit/decompose.py <file>",
                  "python3 .claude/scripts/memory-kit/placement-audit.py --ledger"],
    },
    {
        "key": "architecture-seed",
        "label": "canonical architecture seed",
        "match": ["genesis/docs/content/elohim-protocol/architecture/*.md"],
        "in_cite_graph": True,
        "discipline": "Canonical architecture seed — editing one may stale MAP.md (map-drift watches; the "
                      "walk refresh is cartographer-gated). Compose, don't fork: query prior art before "
                      f"adding seeds. {_CITE_TOOLING}.",
        "tools": ["python3 .claude/scripts/memory-kit/spec-coherence-index.py --query '<topic>'",
                  "python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>"],
    },
    {
        "key": "history",
        "label": "curated history lesson",
        "match": ["genesis/docs/content/elohim-protocol/history/*.md"],
        "in_cite_graph": True,
        "discipline": "Curated history (tier:history) — written once, cited forever; a lesson record, not a "
                      f"live obligation. {_CITE_TOOLING}.",
        "tools": ["python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>"],
    },
    {
        "key": "doc",
        "label": "managed doc-root surface",
        "match": ["genesis/docs/*.md"],
        "in_cite_graph": True,
        "discipline": f"Doc-root surface in the cite graph: {_CITE_TOOLING}. Position + state are governed "
                      "by genesis/docs/PLACEMENT.md (every file instantly auditable; pressure dirs stay empty).",
        "tools": ["python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>",
                  "python3 .claude/scripts/memory-kit/placement-audit.py --ledger"],
    },
    {
        "key": "story",
        "label": "canonical story (storyteller-owned)",
        "match": ["genesis/data/stories/*.md"],
        "in_cite_graph": False,
        "discipline": "Canonical human story — a graduation artifact (storyteller pen): ordinary people must "
                      "recognize themselves in it. Composes with humans/devices/epics/scenarios; technical "
                      "detail routes to specs/history, not here.",
        "tools": [".claude/agents/storyteller.md (dispatch for graduate/memorialize/hold)"],
    },
    {
        "key": "timeline-entity",
        "label": "timeline/backlog entity doc",
        "match": ["genesis/data/timeline/*.md"],
        "in_cite_graph": False,
        "discipline": "Entity doc — DELIBERATELY a plain-path cite target (no id:, no envelope; the audit "
                      "treats path-cites to entity docs as healthy). Follow genesis/data/timeline/CONVENTIONS.md.",
        "tools": ["genesis/data/timeline/CONVENTIONS.md"],
    },
    {
        "key": "a2o-feature",
        "label": "a2o acceptance scenario",
        "match": ["genesis/a2o/features/*.feature"],
        "in_cite_graph": False,
        "discipline": "a2o scenario — the human-experience SPECIFICATION (story-first: implementation is done "
                      "when this passes). Per-scenario @requires:<cap> tags gate env; selectors must match "
                      "real data-testids (testid-sync); commit scenario together with implementation.",
        "tools": [".claude/skills/page-model (testid audit)", "genesis/a2o/CLAUDE.md"],
    },
    {
        "key": "skill",
        "label": "skill doc",
        "match": [".claude/skills/*/SKILL.md"],
        "in_cite_graph": False,
        "discipline": "Skill doc — the description IS the dispatch trigger (skill-audit watches drift). Keep "
                      "triggers concrete and current with the tooling the skill fronts.",
        "tools": ["python3 .claude/scripts/memory-kit/skill-audit.py"],
    },
    {
        "key": "agent",
        "label": "agent definition",
        "match": [".claude/agents/*.md"],
        "in_cite_graph": False,
        "discipline": "Agent definition — description + tools drive dispatch (agent-audit watches drift). "
                      "Keep the role boundary crisp; route how-to detail to skills.",
        "tools": ["python3 .claude/scripts/memory-kit/agent-audit.py"],
    },
]


def _rel_posix(path, repo_root) -> str | None:
    p = Path(path)
    rr = Path(repo_root).resolve()
    if p.is_absolute():
        try:
            return p.resolve().relative_to(rr).as_posix()
        except ValueError:
            return None
    return p.as_posix()


def classify(path, repo_root) -> dict | None:
    """The edit-time question: is this file a managed-memory surface, and which one?
    First match wins (SURFACES is ordered most-specific-first); None = unmanaged."""
    rel = _rel_posix(path, repo_root)
    if rel is None:
        return None
    for entry in SURFACES:
        if entry.get("matcher") == "gospel-claude-md":
            if _cg.is_gospel_claude_md(Path(repo_root) / rel, repo_root):
                return entry
            continue
        if any(fnmatch(rel, pat) for pat in entry.get("match", ())):
            return entry
    return None


def in_cite_graph(path, repo_root) -> bool:
    """True iff the file is a member of the slug-envelope cite graph (doc-roots + gospel CLAUDE.mds) —
    the scope cite-seal-signal / --seal-all / cites-migrate sweep."""
    e = classify(path, repo_root)
    return bool(e and e["in_cite_graph"])
