"""Tests for scope-reconcile. Run: python3 .claude/scripts/memory-kit/__tests__/scope_reconcile_test.py."""
import importlib.util
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
spec = importlib.util.spec_from_file_location("sr", str(here / ".claude/scripts/memory-kit/scope-reconcile.py"))
sr = importlib.util.module_from_spec(spec)
sys.modules["sr"] = sr
spec.loader.exec_module(sr)

D = Path(tempfile.mkdtemp())
_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


def mkdoc(name, req_line):
    p = D / name
    p.write_text(f"---\ntitle: T\n{req_line}\n---\nbody\n")
    return p


# ── requires_env parsing (the minimal-frontmatter quirks) ──
check("inline list [shem]", sr.requires_env(mkdoc("a.md", "requires_env: [shem]")) == ["shem"])
check("inline list + comment", sr.requires_env(mkdoc("b.md", "requires_env: [shem]   # needs the canvas")) == ["shem"])
check("empty inline [] -> no requirement", sr.requires_env(mkdoc("c.md", "requires_env: []   # household-ok")) == [])
check("multi-cap inline", sr.requires_env(mkdoc("d.md", "requires_env: [harbor, alpha-cluster-6peer]")) == ["harbor", "alpha-cluster-6peer"])
check("no requires_env field -> []", sr.requires_env(mkdoc("e.md", "topic: x")) == [])
check("block list", sr.requires_env(mkdoc("f.md", "requires_env:\n  - shem\n  - harbor")) == ["shem", "harbor"])

# ── available_caps: lenient parse, tolerates multi-line notes ──
cs = D / "cluster-state.yaml"
cs.write_text(
    "schema_version: 1\nresources:\n"
    "  household-nodes:\n    role: local\n    available: true\n"
    "  shem:\n    role: remote\n    available: false\n"
    "    note: offline since the PSU failure; this is a long\n          multi-line note that strict yaml chokes on\n"
    "  alpha-cluster-6peer:\n    available: degraded\n"
    "requires_env_vocabulary: x\n"
)
sr.CLUSTER_STATE = cs
av = sr.available_caps()
check("available = only available:true (degraded/false excluded)", av == {"household-nodes"})
check("lenient parse survives multi-line note", "shem" not in av)

# ── the held/live decision (subset check) ──
check("requires unavailable -> held", not set(["shem"]).issubset(av))
check("requires available -> live", set(["household-nodes"]).issubset(av))
check("no requirement -> live (empty subset of anything)", set([]).issubset(av))

# ── _requires_split: known vs unknown (vocab-drift hardening) ──
known = {"shem", "alpha-cluster-6peer", "harbor-registry"}
rel, unk = sr._requires_split(mkdoc("g.md", "requires_env: [shem, harbor]"), known)
check("_requires_split: known cap -> relevant", rel == ["shem"])
check("_requires_split: unknown cap -> surfaced (not silently dropped)", unk == ["harbor"])

# ── held->live escape BLOCKED when an unknown cap is present (the latent-bug fix) ──
# [alpha-cluster-6peer(known, available), harbor(unknown)] must NOT escape even though the known cap is
# satisfied — the untracked cap could be the real blocker hiding under a wrong name.
av2 = {"alpha-cluster-6peer"}
rel2, unk2 = sr._requires_split(mkdoc("h.md", "requires_env: [alpha-cluster-6peer, harbor]"), known)
check("unknown cap blocks held->live escape", (bool(rel2) and set(rel2).issubset(av2) and not unk2) is False)
rel3, unk3 = sr._requires_split(mkdoc("i.md", "requires_env: [alpha-cluster-6peer]"), known)
check("clean known cap escapes when available", (bool(rel3) and set(rel3).issubset(av2) and not unk3) is True)

# ── report(): the SessionStart gate line (monkeypatch compute_drift, capture stdout) ──
import contextlib  # noqa: E402
import io  # noqa: E402


def _cap(fn):
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        fn()
    return buf.getvalue().strip()


_orig = sr.compute_drift
sr.compute_drift = lambda: ([], [], [], [])
check("report: aligned line", _cap(sr.report).startswith("scope: aligned ✅"))
sr.compute_drift = lambda: ([], [(Path("x"), Path("y"), [])], [], [])
_o = _cap(sr.report)
check("report: return-to-plate line", "1 to return to plate" in _o and "scope-reconcile.py --apply" in _o)
sr.compute_drift = lambda: ([(Path("x"), Path("y"), ["shem"])], [], [(Path("d"), ["harbor"])], [])
_o = _cap(sr.report)
check("report: hold + vocab note", "1 to hold (shem)" in _o and "unknown-cap: harbor" in _o)
sr.compute_drift = _orig

# ── developer flip control (cluster-state durable home + derived runtime signal stay coherent) ──
sr.CLUSTER_STATE = cs  # shem: available:false
check("derive runtime: shem false -> unavailable", sr.derive_remote_compute_status() == "unavailable")
check("_flip_resource shem->true changes a line", sr._flip_resource("shem", "true") is True)
check("derive runtime follows the flip -> available", sr.derive_remote_compute_status() == "available")
check("_flip_resource idempotent (already true)", sr._flip_resource("shem", "true") is False)
sr._flip_resource("shem", "false")  # restore
check("env_line emits derived export", "ELOHIM_REMOTE_COMPUTE_STATUS=unavailable" in _cap(sr.env_line))
check("set_state rejects unknown state (before any flip)", sr.set_state("shem=banana", False) == 2)
check("set_state rejects unknown resource", sr.set_state("nonexistent=on", False) == 2)

# ── gap-granular scope verdict (honors iroh ≠ shem: mixed doc stays on the plate) ──
av, kn = {"household-nodes"}, {"household-nodes", "shem"}
mixed = mkdoc("mixed.md", "topic: x")  # no doc-level requires_env
gidx_mixed = {"mixed.md": {"items": [{"state": "OPEN"}, {"state": "OPEN", "requires_env": ["shem"]}]}}
v, caps = sr._scope_verdict(mixed, av, kn, gidx_mixed)
check("verdict: mixed plan (testable + 1 shem gap) -> live", v == "live" and caps == ["shem"])

held = mkdoc("held.md", "requires_env: [shem]")
gidx_held = {"held.md": {"items": [{"state": "OPEN"}, {"state": "OPEN"}]}}  # no per-gap tags → inherit [shem]
v, caps = sr._scope_verdict(held, av, kn, gidx_held)
check("verdict: every gap inherits unavailable doc-req -> held", v == "held" and caps == ["shem"])

amb = mkdoc("amb.md", "topic: x")  # no req, no cache
check("verdict: no scope info -> ambiguous (stays held)", sr._scope_verdict(amb, av, kn, {})[0] == "ambiguous")

nc = mkdoc("nc.md", "requires_env: [shem]")  # doc-level req, no gap cache
check("verdict: no cache, doc-level blocked -> held", sr._scope_verdict(nc, av, kn, {})[0] == "held")

live = mkdoc("live.md", "requires_env: [household-nodes]")  # doc-level req satisfied
check("verdict: no cache, doc-level satisfied -> live", sr._scope_verdict(live, av, kn, {})[0] == "live")

print(f"\n  {_p} assertions passed ✅")
