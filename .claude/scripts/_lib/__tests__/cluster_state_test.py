"""Tests for cluster_state (the ONE parser for genesis/manifests/cluster-state.yaml).

Asserts through the CALL SITES as well as the library: a regression here is only a regression if
`placement-audit.load_cluster_state` / `focus-baseline._cluster_detail` / `scope-reconcile._parse_cluster`
actually behave correctly, and asserting on `cluster_state.load()` alone would stay green if someone
re-inlined a private parser at a call site.

Run: python3 .claude/scripts/_lib/__tests__/cluster_state_test.py (exit 0 = pass)
 or: python3 -m pytest .claude/scripts/_lib/__tests__/cluster_state_test.py
"""
import importlib.util
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        REPO = here
        break
    here = here.parent
from _lib import cluster_state as cs  # noqa: E402

MK = REPO / ".claude" / "scripts" / "memory-kit"

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


def _load_script(name, filename):
    """Import a hyphenated memory-kit script. sys.argv is neutralized first: placement-audit parses
    argv AT IMPORT (`ROOT = Path(ARGS[0])` when a bare arg is present), so pytest's argv would
    otherwise silently repoint its ROOT at this test file."""
    saved = sys.argv
    sys.argv = [str(MK / filename)]
    try:
        spec = importlib.util.spec_from_file_location(name, MK / filename)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        return mod
    finally:
        sys.argv = saved


PA = _load_script("placement_audit_under_test", "placement-audit.py")
SR = _load_script("scope_reconcile_under_test", "scope-reconcile.py")
FB = _load_script("focus_baseline_under_test", "focus-baseline.py")


def _write(text):
    d = tempfile.mkdtemp()
    p = Path(d) / "cluster-state.yaml"
    p.write_text(text, encoding="utf-8")
    return p


def call_sites(path):
    """Every consumer's cluster read, against `path`. Returns a dict of their outputs."""
    SR.CLUSTER_STATE = path
    FB.CLUSTER_STATE = path
    avail_pa, updated_pa = PA.load_cluster_state(path)
    avail_fb, roles_fb = FB._cluster_detail()
    known_sr, available_sr = SR._parse_cluster()
    return {
        "pa_avail": avail_pa,
        "pa_updated": updated_pa,
        "fb_avail": avail_fb,
        "fb_roles": roles_fb,
        "sr_known": known_sr,
        "sr_available": available_sr,
        "sr_provides": SR._parse_provides(),
    }


FIXTURE = """\
schema_version: 1
updated: 2026-06-04

resources:
  household-nodes:
    role: local P2P
    available: true
    provides_node_types: [operations, edge]

  alpha-cluster-6peer:
    role: 6-peer alpha soak cluster
    available: degraded
    note: 10/13 peers CrashLooping.

  shem:
    role: cross-node canvas
    available: true
    provides_node_types: [remote]

# requires_env vocabulary used by tests/stories/gates:
#   shem — needs the shem canvas
"""

# ── basic parse ──
p = _write(FIXTURE)
state = cs.load(p)

check("updated captured", state.updated == "2026-06-04")
check("all_names has all three", state.all_names() == {"household-nodes", "alpha-cluster-6peer", "shem"})
check("available_names excludes degraded", state.available_names() == {"household-nodes", "shem"})
check("available_map keeps raw scalar for degraded",
      state.available_map()["alpha-cluster-6peer"] == "degraded")
check("available_map true for available resource", state.available_map()["household-nodes"] == "true")
check("roles captured", state.roles()["shem"] == "cross-node canvas")
check("provides_map aggregates across resources",
      state.provides_map() == {"operations": {"household-nodes"}, "edge": {"household-nodes"}, "remote": {"shem"}})
check("is_available property true", state.resources["household-nodes"].is_available is True)
check("is_available property false for degraded", state.resources["alpha-cluster-6peer"].is_available is False)

# ── missing file: fail-open, never crash ──
missing = cs.load(Path("/nonexistent/cluster-state.yaml"))
check("missing file -> empty resources", missing.all_names() == set())
check("missing file -> updated stays '?'", missing.updated == "?")
check("missing file -> call sites survive", call_sites(Path("/nonexistent/cluster-state.yaml"))["pa_avail"] == {})

# ══════════════════════════════════════════════════════════════════════════════════════════
# DECISION 1 — a column-0 COMMENT is not a block terminator.
# The merge of the three HEAD parsers briefly adopted `_parse_provides`'s `^[A-Za-z#]`, the
# narrowest of the four regexes. cluster-state.yaml already carries five comment blocks; an
# operator adding one ordinary section comment between two resources would then drop every
# resource below it — and `scope-reconcile.py --apply` git-mv's the specs/plans/features that
# require them into held/ and flips deployments.json `suspended` flags. Pinned BOTH directions.
# ══════════════════════════════════════════════════════════════════════════════════════════
COMMENT_TRAP = """\
schema_version: 1
updated: 2026-06-04

resources:
  household-nodes:
    role: local P2P
    available: true
    provides_node_types: [operations]

# ---- remote capabilities ----

  iroh:
    role: iroh dataplane
    available: true
    provides_node_types: [remote]

  shem:
    role: cross-node canvas
    available: true
"""
p = _write(COMMENT_TRAP)
c = call_sites(p)
check("comment-in-block: library keeps every resource",
      cs.load(p).available_names() == {"household-nodes", "iroh", "shem"})
check("comment-in-block: scope-reconcile (the MOVER) keeps every resource",
      c["sr_available"] == {"household-nodes", "iroh", "shem"})
check("comment-in-block: provides_node_types below the comment survive",
      c["sr_provides"] == {"operations": {"household-nodes"}, "remote": {"iroh"}})
check("comment-in-block: placement-audit (the BUDGET) keeps every resource",
      set(c["pa_avail"]) == {"household-nodes", "iroh", "shem"})
check("comment-in-block: focus-baseline keeps every resource + role",
      set(c["fb_avail"]) == {"household-nodes", "iroh", "shem"} and c["fb_roles"]["shem"] == "cross-node canvas")

# …and the OTHER direction: a genuine top-level key MUST still terminate the block.
BLOCK_END = """\
schema_version: 1
updated: 2026-06-04

resources:
  household-nodes:
    available: true

policies:
  not-a-resource:
    available: true
"""
p = _write(BLOCK_END)
c = call_sites(p)
check("top-level key still TERMINATES the resources block (library)",
      cs.load(p).all_names() == {"household-nodes"})
check("top-level key still terminates: placement-audit", set(c["pa_avail"]) == {"household-nodes"})
check("top-level key still terminates: scope-reconcile", c["sr_known"] == {"household-nodes"})

# ══════════════════════════════════════════════════════════════════════════════════════════
# DECISION 2 — a resource must DECLARE `available:` to be an availability CLAIM.
# `available_map()` (placement-audit / focus-baseline `known`) omits a role-only block, so a
# merely-planned capability never silently benches work. The file format cannot distinguish
# "forgot to state availability" from "planned, never a runtime claim", so we take the
# conservative option: do not newly block. `all_names()` (scope-reconcile's VOCABULARY, used for
# --set validation and unknown-cap drift) still contains it — unchanged HEAD behaviour.
# ══════════════════════════════════════════════════════════════════════════════════════════
ROLE_ONLY = """\
schema_version: 1
updated: 2026-06-04

resources:
  household-nodes:
    role: local P2P
    available: true

  planned-cap:
    role: a capability we intend to stand up
"""
p = _write(ROLE_ONLY)
st = cs.load(p)
c = call_sites(p)
check("role-only: declared_available False", st.resources["planned-cap"].declared_available is False)
check("role-only: declared_names excludes it", st.declared_names() == {"household-nodes"})
check("role-only: available_map OMITS it (no '' entry — does not newly block)",
      "planned-cap" not in st.available_map())
check("role-only: placement-audit budget does NOT treat it as cluster-tracked",
      set(c["pa_avail"]) == {"household-nodes"})
check("role-only: focus-baseline `known` does NOT include it", set(c["fb_avail"]) == {"household-nodes"})
check("role-only: its role is still rendered (the WHY text)",
      c["fb_roles"]["planned-cap"] == "a capability we intend to stand up")
check("role-only: all_names (vocabulary) DOES include it — --set + unknown-cap detection",
      st.all_names() == {"household-nodes", "planned-cap"})
# The residual, deliberate asymmetry: the MOVER gates on the vocabulary set, the BUDGET does not.
check("role-only asymmetry pinned: mover gates (in known, not in available), budget reader does not",
      "planned-cap" in c["sr_known"] and "planned-cap" not in c["sr_available"]
      and "planned-cap" not in c["pa_avail"])
# Not gating on it in the budget is what `gap_blocked` actually consumes:
from _lib import env_scope as _es  # noqa: E402
check("role-only: gap_blocked() says NOT blocked for the budget reader's `known`",
      _es.gap_blocked(["planned-cap"], {k for k, v in c["pa_avail"].items() if v == "true"},
                      set(c["pa_avail"])) is False)

# ══════════════════════════════════════════════════════════════════════════════════════════
# DECISION 3 — a repeated declaration MERGES, never RESETS; the merge policy is PER FIELD.
#   available: TRUE-WINS   role: FIRST-wins   provides_node_types: UNION
# A copy-paste re-declaration used to wipe the first block's `available: true` → available_names()
# loses the cap → spurious held/ moves. But merging FIRST-explicit-wins for availability is WORSE
# than the reset it replaced in the ordering below (a stale `false` sitting above a fresh `true`):
# the reset reported `true`, all three HEAD readers report `true`, and first-wins reports `false`
# — which is `scope-reconcile --apply` git-mv'ing every spec/plan/feature requiring that cap into
# held/ and flipping deployments.json `suspended` flags. Availability merges TRUE-WINS, which is
# literally HEAD scope-reconcile's `avail.add(cur)` set-union. BOTH orderings pinned, and both the
# duplicate-KEY and the two-lines-in-ONE-block shapes, through the CALL SITES.
# ══════════════════════════════════════════════════════════════════════════════════════════
DUP = """\
schema_version: 1
updated: 2026-06-04

resources:
  shem:
    role: cross-node canvas
    available: true
    provides_node_types: [remote]

  shem:
    role: RE-DECLARED by copy-paste
    provides_node_types: [extra]
"""
p = _write(DUP)
st = cs.load(p)
c = call_sites(p)
check("duplicate key: available: true from the first block SURVIVES (library)",
      st.available_names() == {"shem"})
check("duplicate key: the mover still sees it available", c["sr_available"] == {"shem"})
check("duplicate key: the budget reader still sees it available", c["pa_avail"] == {"shem": "true"})
check("duplicate key: role stays FIRST-wins (focus-baseline's old `cur not in roles` guard)",
      c["fb_roles"]["shem"] == "cross-node canvas")
check("duplicate key: provides_node_types accumulate across both blocks",
      c["sr_provides"] == {"remote": {"shem"}, "extra": {"shem"}})
check("duplicate key: recorded on duplicate_keys for any consumer that wants to warn",
      st.duplicate_keys == ["shem"])

# ── THE HARMFUL ORDERING: `available: false` FIRST, `available: true` SECOND ──────────────
# The realistic operator edit (a stale false left above a freshly-added true). All three HEAD
# readers say AVAILABLE here: scope-reconcile by set-union, placement-audit and focus-baseline by
# last-wins. First-explicit-wins would say UNAVAILABLE — a fail-CLOSED parser regression that
# benches every doc requiring the cap. Assert through the call sites: the library alone cannot
# catch a re-inlined divergent parser.
DUP_FALSE_FIRST = """\
schema_version: 1
updated: 2026-06-04

resources:
  shem:
    role: cross-node canvas
    available: false

  shem:
    role: RE-DECLARED with the fresh value
    available: true
    provides_node_types: [remote]
"""
p = _write(DUP_FALSE_FIRST)
st = cs.load(p)
c = call_sites(p)
check("dup key false-THEN-true: library reports AVAILABLE (true-wins, not first-wins)",
      st.available_names() == {"shem"})
check("dup key false-THEN-true: available_map raw scalar is 'true'", st.available_map() == {"shem": "true"})
check("dup key false-THEN-true: the MOVER sees it available (matches HEAD _parse_cluster set-union)",
      c["sr_available"] == {"shem"})
check("dup key false-THEN-true: the BUDGET reader sees 'true' (matches HEAD load_cluster_state)",
      c["pa_avail"] == {"shem": "true"})
check("dup key false-THEN-true: focus-baseline sees 'true' (matches HEAD _cluster_detail)",
      c["fb_avail"] == {"shem": "true"})
check("dup key false-THEN-true: role still FIRST-wins", c["fb_roles"]["shem"] == "cross-node canvas")

# ── SAME SHAPE, ONE BLOCK: two `available:` lines under a single resource key ─────────────
# No duplicate resource key at all, so `duplicate_keys` stays empty — yet the identical
# fail-closed flip occurs under first-wins. This is the case the merge-policy tests missed.
SAME_BLOCK_FALSE_FIRST = """\
schema_version: 1
updated: 2026-06-04

resources:
  shem:
    role: cross-node canvas
    available: false
    available: true
    provides_node_types: [remote]
"""
p = _write(SAME_BLOCK_FALSE_FIRST)
st = cs.load(p)
c = call_sites(p)
check("same-block false-THEN-true: library reports AVAILABLE",
      st.available_names() == {"shem"} and st.available_map() == {"shem": "true"})
check("same-block false-THEN-true: the MOVER sees it available", c["sr_available"] == {"shem"})
check("same-block false-THEN-true: the BUDGET reader sees 'true'", c["pa_avail"] == {"shem": "true"})
check("same-block false-THEN-true: focus-baseline sees 'true'", c["fb_avail"] == {"shem": "true"})
check("same-block false-THEN-true: NOT a duplicate resource key (duplicate_keys is empty)",
      st.duplicate_keys == [])

SAME_BLOCK_TRUE_FIRST = """\
schema_version: 1
updated: 2026-06-04

resources:
  shem:
    role: cross-node canvas
    available: true
    available: false
"""
p = _write(SAME_BLOCK_TRUE_FIRST)
c = call_sites(p)
check("same-block true-THEN-false: still AVAILABLE — a parser never takes a cap away (library)",
      cs.load(p).available_names() == {"shem"})
check("same-block true-THEN-false: the MOVER sees it available (matches HEAD set-union)",
      c["sr_available"] == {"shem"})
check("same-block true-THEN-false: budget reader sees 'true' — deliberate divergence from HEAD "
      "placement-audit/focus-baseline last-wins; this IS decision 3's non-reset, permissive direction",
      c["pa_avail"] == {"shem": "true"} and c["fb_avail"] == {"shem": "true"})

# ── a non-true scalar is NOT clobbered by another non-true scalar (first-wins fallback) ───
DUP_DEGRADED = """\
schema_version: 1
updated: 2026-06-04

resources:
  alpha-cluster-6peer:
    available: degraded

  alpha-cluster-6peer:
    available: false
"""
p = _write(DUP_DEGRADED)
st = cs.load(p)
c = call_sites(p)
check("dup key degraded-THEN-false: no `true` anywhere, so the FIRST scalar stands ('degraded')",
      st.available_map() == {"alpha-cluster-6peer": "degraded"} and c["pa_avail"] == st.available_map())
check("dup key degraded-THEN-false: neither is available to the mover", c["sr_available"] == set())

# ══════════════════════════════════════════════════════════════════════════════════════════
# DECISION 4 (the original consolidation) — the parser stays SCOPED to the `resources:` block.
# placement-audit.py's HEAD `load_cluster_state()` had NO block-boundary check: it matched a bare
# 2-space key followed by a 4-space `available:` line ANYWHERE in the file. It is vulnerable BOTH
# before and after `resources:` — cover both, and assert through the CALL SITE, so re-inlining an
# unscoped parser in placement-audit.py turns this suite red.
# ══════════════════════════════════════════════════════════════════════════════════════════
DECOY_AFTER = """\
schema_version: 1
updated: 2026-06-04

resources:
  household-nodes:
    available: true

decoys:
  not-a-resource:
    available: true
"""
p = _write(DECOY_AFTER)
c = call_sites(p)
check("decoy AFTER resources: not read as a resource (library)",
      "not-a-resource" not in cs.load(p).all_names())
check("decoy AFTER resources: placement-audit CALL SITE is scoped",
      c["pa_avail"] == {"household-nodes": "true"})
check("decoy AFTER resources: focus-baseline CALL SITE is scoped", set(c["fb_avail"]) == {"household-nodes"})
check("decoy AFTER resources: scope-reconcile CALL SITE is scoped", c["sr_known"] == {"household-nodes"})

DECOY_BEFORE = """\
schema_version: 1
updated: 2026-06-04

decoys:
  phantom-cap:
    available: true

resources:
  household-nodes:
    available: true
"""
p = _write(DECOY_BEFORE)
c = call_sites(p)
check("decoy BEFORE resources: not read as a resource (library)",
      "phantom-cap" not in cs.load(p).all_names())
check("decoy BEFORE resources: placement-audit CALL SITE is scoped",
      c["pa_avail"] == {"household-nodes": "true"})
check("decoy BEFORE resources: focus-baseline CALL SITE is scoped", set(c["fb_avail"]) == {"household-nodes"})
check("decoy BEFORE resources: scope-reconcile CALL SITE is scoped", c["sr_known"] == {"household-nodes"})
check("decoy BEFORE resources: `updated:` still captured through the call site",
      c["pa_updated"] == "2026-06-04")

# ── the REAL file: the consumers agree with the library on the shipped cluster-state ──
REAL = REPO / "genesis" / "manifests" / "cluster-state.yaml"
if REAL.is_file():
    real = cs.load(REAL)
    c = call_sites(REAL)
    check("real cluster-state: placement-audit == library available_map",
          c["pa_avail"] == real.available_map() and c["pa_updated"] == real.updated)
    check("real cluster-state: focus-baseline == library available_map/roles",
          c["fb_avail"] == real.available_map() and c["fb_roles"] == real.roles())
    check("real cluster-state: scope-reconcile == library all_names/available_names",
          (c["sr_known"], c["sr_available"]) == (real.all_names(), real.available_names()))
    check("real cluster-state: no duplicate resource keys", real.duplicate_keys == [])
    check("real cluster-state: every resource declares availability (decision 2 is inert today)",
          real.declared_names() == real.all_names())

print(f"\n  {_p} assertions passed ✅")

_EXPECTED_ASSERTIONS = 65


def test_cluster_state_suite():
    """pytest entry point. The assertions above run at IMPORT (repo convention — these files are
    `python3 <file>` scripts), so a failure is already a pytest collection error; this collected
    test turns a clean run into a reported PASS instead of exit-5 'no tests ran', and pins the
    assertion count so a silently-skipped block cannot read as green."""
    assert _p >= _EXPECTED_ASSERTIONS, f"only {_p} assertions ran (expected >= {_EXPECTED_ASSERTIONS})"
