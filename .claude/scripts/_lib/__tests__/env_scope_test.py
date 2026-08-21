"""Tests for env_scope (the gap-granular substrate-scope resolver).
Run: python3 .claude/scripts/_lib/__tests__/env_scope_test.py (exit 0 = pass)."""
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
from _lib import env_scope as es  # noqa: E402

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


# ── parse_requires_env: the frontmatter-value quirks ──
check("parse block list", es.parse_requires_env(["shem", "harbor-registry"]) == ["shem", "harbor-registry"])
check("parse inline list", es.parse_requires_env("[shem]") == ["shem"])
check("parse inline + comment", es.parse_requires_env("[shem, alpha]   # needs canvas") == ["shem", "alpha"])
check("parse empty inline -> []", es.parse_requires_env("[]") == [])
check("parse scalar", es.parse_requires_env("shem") == ["shem"])
check("parse None -> []", es.parse_requires_env(None) == [])

# ── requires_tags: the per-gap @requires override source (a2o-isomorphic) ──
check("requires_tags single", es.requires_tags("Step 9.2: run e2e @requires:shem") == ["shem"])
check("requires_tags multi", es.requires_tags("@requires:harbor-registry @requires:alpha-cluster-6peer")
      == ["harbor-registry", "alpha-cluster-6peer"])
check("requires_tags none", es.requires_tags("plain task, no marker") == [])

# ── resolved_requires_env: gap override else doc default ──
check("resolved: gap override wins", es.resolved_requires_env(["shem"], []) == ["shem"])
check("resolved: inherit doc default", es.resolved_requires_env(None, ["shem"]) == ["shem"])
check("resolved: empty gap inherits doc", es.resolved_requires_env([], ["shem"]) == ["shem"])
check("resolved: both empty -> []", es.resolved_requires_env(None, []) == [])

# ── gap_blocked: the BLOCKED-BY-ENV predicate ──
AVAIL, KNOWN = {"household-nodes"}, {"household-nodes", "shem", "harbor-registry"}
check("blocked: required cap unavailable", es.gap_blocked(["shem"], AVAIL, KNOWN) is True)
check("not blocked: empty requirement", es.gap_blocked([], AVAIL, KNOWN) is False)
check("not blocked: required cap available", es.gap_blocked(["household-nodes"], AVAIL, KNOWN) is False)
check("not blocked: unknown cap (fixture tag) ignored", es.gap_blocked(["doorway"], AVAIL, KNOWN) is False)
check("blocked: mixed known-unavail + unknown", es.gap_blocked(["shem", "doorway"], AVAIL, KNOWN) is True)

# ── the iroh ≠ shem scenario end-to-end ──
# a mixed plan: doc default empty, only the cross-node gaps tagged @requires:shem
doc_default = []  # no doc-level requires_env
testable_gap = es.resolved_requires_env(es.requires_tags("write the loopback fixture"), doc_default)
crossnode_gap = es.resolved_requires_env(es.requires_tags("run e2e @requires:shem"), doc_default)
check("iroh transport gap stays pickable", es.gap_blocked(testable_gap, AVAIL, KNOWN) is False)
check("cross-node gap is held", es.gap_blocked(crossnode_gap, AVAIL, KNOWN) is True)

# ── feature_act: the a2o `@act:<id>` tag scan (mirrors substrate-scope.ts's actFromTags/ACT_TAG) ──
check("feature_act reads @act:i", es.feature_act(
    "@e2e @resilience @act:i @requires:owned-substrate @concern:x\nFeature: X\n") == "i")
check("feature_act reads @act:host", es.feature_act("@act:host @e2e\nFeature: X\n") == "host")
check("feature_act reads @act:iii", es.feature_act("@act:iii\nFeature: X\n") == "iii")
check("feature_act None when no act tag", es.feature_act("@e2e @requires:doorway\nFeature: X\n") is None)
check("feature_act ignores tags AFTER Feature: (scenario-level, not feature-level)",
      es.feature_act("@e2e\nFeature: X\n\n  @act:i\n  Scenario: Y\n") is None)
check("feature_act ignores gherkin comments (prose mentioning the tag is not a tag)",
      es.feature_act("# see @act:i for context\n@e2e\nFeature: X\n") is None)
check("feature_act on empty/None text -> None", es.feature_act("") is None and es.feature_act(None) is None)

# ── act_baseline_caps: the act's OWN lane-contract file, additive-only (mirrors actBaselineCaps()) ──
_mdir = Path(tempfile.mkdtemp())
(_mdir / "cluster-state.act1-household.yaml").write_text(
    "resources:\n"
    "  household-nodes:\n    available: true\n"
    "  owned-substrate:\n    available: true\n"
    "  harbor-registry:\n    available: false\n"
)
(_mdir / "cluster-state.act2-neighbourhood.yaml").write_text(
    "resources:\n"
    "  household-nodes:\n    available: true\n"
    "  owned-substrate:\n    available: false\n"
)
(_mdir / "cluster-state.yaml").write_text(
    "resources:\n"
    "  household-nodes:\n    available: true\n"
    "  shem:\n    available: true\n"
)
check("act_baseline_caps('i') = act1's available:true set",
      es.act_baseline_caps("i", _mdir) == {"household-nodes", "owned-substrate"})
check("act_baseline_caps('ii') reads act2 and DROPS owned-substrate (false there)",
      es.act_baseline_caps("ii", _mdir) == {"household-nodes"})
check("act_baseline_caps('iii') reads the live file and adds shem by definition",
      es.act_baseline_caps("iii", _mdir) == {"household-nodes", "shem"})
check("act_baseline_caps('host') -> empty (no substrate at all)",
      es.act_baseline_caps("host", _mdir) == set())
check("act_baseline_caps fails open (empty) when the act file is unreadable",
      es.act_baseline_caps("i", _mdir / "nope") == set())

print(f"\n  {_p} assertions passed ✅")
