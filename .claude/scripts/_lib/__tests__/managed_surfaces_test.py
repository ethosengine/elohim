"""Tests for _lib.managed_surfaces — the edit-time surface axis.
Run: python3 .claude/scripts/_lib/__tests__/managed_surfaces_test.py (exit 0 = pass)."""
import sys
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
REPO = here

from _lib import managed_surfaces as ms  # noqa: E402

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


def cls(rel):
    e = ms.classify(REPO / rel, REPO)
    return e["key"] if e else None


# ── classification: first-match-wins ordering ──
check("spec", cls("genesis/docs/superpowers/specs/2026-06-05-x-design.md") == "spec")
check("plan (superpowers home)", cls("genesis/docs/superpowers/plans/2026-06-05-x-plan.md") == "plan")
check("plan (impl home)", cls("genesis/docs/plans/x-plan.md") == "plan")
check("architecture seed beats doc catch-all",
      cls("genesis/docs/content/elohim-protocol/architecture/foo.md") == "architecture-seed")
check("history lesson", cls("genesis/docs/content/elohim-protocol/history/2026-06-02-x.md") == "history")
check("doc catch-all (PLACEMENT.md)", cls("genesis/docs/PLACEMENT.md") == "doc")
check("memory entry", cls(".claude/memory/project_foo.md") == "memory")
check("gospel CLAUDE.md (pillar)", cls("app/lamad/CLAUDE.md") == "gospel")
check("gospel CLAUDE.md (repo root)", cls("CLAUDE.md") == "gospel")
check("vendored CLAUDE.md is NOT managed", cls("app/x/node_modules/y/CLAUDE.md") is None)
check("process gospel (.claude CLAUDE.md)", cls(".claude/scripts/memory-kit/CLAUDE.md") == "process-gospel")
check("process gospel (LIFECYCLE.md)", cls(".claude/scripts/memory-kit/LIFECYCLE.md") == "process-gospel")
check("process gospel (orchestrator README)", cls("genesis/orchestrator/README.md") == "process-gospel")
check("process gospel (timeline CONVENTIONS beats entity)",
      cls("genesis/data/timeline/CONVENTIONS.md") == "process-gospel")
check("timeline entity doc", cls("genesis/data/timeline/backlog/bundle-styling-token-contract.md") == "timeline-entity")
check("story", cls("genesis/data/stories/foo.md") == "story")
check("a2o feature", cls("genesis/a2o/features/lms/foo.feature") == "a2o-feature")
check("skill", cls(".claude/skills/semantic-links/SKILL.md") == "skill")
check("agent", cls(".claude/agents/librarian.md") == "agent")
check("routing config (subject axis)", cls(".claude/subject-routing.yaml") == "routing-config")
check("routing config (env axis)", cls("genesis/manifests/cluster-state.yaml") == "routing-config")
check("ordinary code file is unmanaged", cls("app/elohim-app/src/app/app.component.ts") is None)
check("ordinary app md is unmanaged", cls("app/elohim-app/README.md") is None)

# ── entry shape ──
e = ms.classify(REPO / "app/lamad/CLAUDE.md", REPO)
check("entry carries discipline text", bool(e["discipline"]) and len(e["discipline"]) < 600)
check("entry carries tools", isinstance(e["tools"], list) and any("cite-gen" in t for t in e["tools"]))
check("gospel is in the cite graph", e["in_cite_graph"] is True)
check("timeline entity is NOT in the cite graph",
      ms.classify(REPO / "genesis/data/timeline/backlog/x.md", REPO)["in_cite_graph"] is False)
check("every entry has key/label/discipline/tools/in_cite_graph",
      all({"key", "label", "discipline", "tools", "in_cite_graph"} <= set(s) for s in ms.SURFACES))

# ── cite-graph membership helper ──
check("in_cite_graph: spec", ms.in_cite_graph(REPO / "genesis/docs/superpowers/specs/x.md", REPO))
check("in_cite_graph: gospel", ms.in_cite_graph(REPO / "app/lamad/CLAUDE.md", REPO))
check("in_cite_graph: not code", not ms.in_cite_graph(REPO / "app/x.py", REPO))
check("in_cite_graph: not vendored gospel", not ms.in_cite_graph(REPO / "x/node_modules/CLAUDE.md", REPO))

# ── cross-check vs subject-routing.yaml (the class axis): the two axes must not drift ──
import yaml  # noqa: E402

sr = yaml.safe_load((REPO / ".claude" / "subject-routing.yaml").read_text())
classes = sr["classes"]


def probe(home):
    """A home (dir or file path) -> the surface key classify() assigns it."""
    h = home.strip("/")
    rel = h if h.endswith((".md", ".yaml")) else f"{h}/probe-x.md"
    e = ms.classify(REPO / rel, REPO)
    return e["key"] if e else None


for home in classes["protocol-canonical"]["write_location"]["active_source"]:
    check(f"class-axis active_source covered: {home}", probe(home) in ("spec", "plan"))
check("class-axis impl_plan_home covered",
      probe(classes["protocol-canonical"]["write_location"]["impl_plan_home"]) == "plan")
check("class-axis canonical_truth covered",
      probe(classes["protocol-canonical"]["write_location"]["canonical_truth"]) == "architecture-seed")
gh = classes["process-meta"]["write_location"]["gospel_homes"]
_flat = []
for v in gh.values():
    _flat += v if isinstance(v, list) else [v]
for home in _flat:
    check(f"class-axis gospel_home is managed: {home}",
          probe(home) in ("gospel", "process-gospel", "doc"))
flow = classes["protocol-canonical"]["decomposition_flow"]
check("class-axis history home covered", probe(flow["history"]) == "history")
check("class-axis story home covered", probe(flow["story"]) == "story")
check("class-axis backlog home covered", probe(flow["backlog"]) == "timeline-entity")

print(f"\n  {_p} assertions passed ✅")
