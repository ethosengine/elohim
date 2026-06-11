"""Tests for _lib.subject_routing. Run: python3 .claude/scripts/_lib/__tests__/subject_routing_test.py
Exit 0 = pass (the process-meta "verified" — the script runs)."""
import sys
from pathlib import Path

# _lib bootstrap (the documented walk-up snippet)
here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

from _lib import subject_routing as sr  # noqa: E402

REPO = here  # the dir holding .claude/

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


# ── classify (the deliverable-TARGET discriminator) ──
check("process targets -> process-meta",
      sr.classify([".claude/scripts/memory-kit/decompose.py", "genesis/docs/PLACEMENT.md"]) == "process-meta")
check("product targets -> protocol-canonical",
      sr.classify(["app/elohim-app/foo.ts", "genesis/docs/content/elohim-protocol/architecture/x.md"]) == "protocol-canonical")
check("no targets -> provisional", sr.classify([]) == "provisional")
check("a2o HARNESS (scripts) is process", sr.classify(["genesis/a2o/scripts/look.ts"]) == "process-meta")
check("a2o FEATURES (pillar scenario) is product", sr.classify(["genesis/a2o/features/lamad/x.feature"]) == "protocol-canonical")
check("root CLAUDE.md is process", sr.classify(["CLAUDE.md"]) == "process-meta")
check("product wins over process (mixed)",
      sr.classify([".claude/x.py", "app/y.ts"]) == "protocol-canonical")

# ── gate_check (fail-loud mis-class detectors) ──
sigs = sr.gate_check({"domain": "D4", "class": "process-meta"}, [".claude/x.py", "genesis/docs/PLACEMENT.md"])
check("vocab-vs-target-mismatch fires on D4 + all-process (the relocation case)",
      any(s.id == "vocab-vs-target-mismatch" for s in sigs))
check("vocab-vs-target-mismatch is HARD", any(s.id == "vocab-vs-target-mismatch" and s.severity == "hard" for s in sigs))

sigs2 = sr.gate_check({"domain": "D2"}, ["app/x.ts"])
check("no mismatch when a product target is present",
      not any(s.id == "vocab-vs-target-mismatch" for s in sigs2))

sigs3 = sr.gate_check({"class": "process-meta"}, ["genesis/a2o/features/lamad/x.feature"])
check("forced-a2o-leg fires on features/<pillar>", any(s.id == "forced-a2o-leg" for s in sigs3))
sigs4 = sr.gate_check({"class": "process-meta"}, ["genesis/a2o/scripts/look.ts"])
check("forced-a2o-leg does NOT fire on a2o harness", not any(s.id == "forced-a2o-leg" for s in sigs4))

arch = ["genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md"]
check("forced-architecture-seed fires on process->architecture (non-cite)",
      any(s.id == "forced-architecture-seed" for s in sr.gate_check({"class": "process-meta"}, arch)))
check("forced-architecture-seed scoped OFF for ceremony-rewrite (run-output)",
      not any(s.id == "forced-architecture-seed" for s in sr.gate_check({"class": "process-meta", "artifact_kind": "run-output"}, arch)))

# ── reconcile (BACK-fire) ──
check("reconcile provisional -> process-meta from residue", sr.reconcile("provisional", [".claude/x.py"]) == "process-meta")
check("reconcile keeps a resolved stamp", sr.reconcile("protocol-canonical", ["app/x.ts"]) == "protocol-canonical")

# ── targets_from_frontmatter (derived_from is lineage, NOT a target) ──
fm = {"proposed_amendments": [".claude/scripts/memory-kit/decompose.py"],
      "derived_from": ["genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md"],
      "cites": ["genesis/docs/PLACEMENT.md"]}
tgt = sr.targets_from_frontmatter(fm)
check("targets prefer proposed_amendments", ".claude/scripts/memory-kit/decompose.py" in tgt)
check("derived_from EXCLUDED from targets (lineage, not residue)",
      not any("memory-lifecycle-design" in t for t in tgt))
check("a process spec with product-vocab derived_from still classifies process-meta",
      sr.classify(tgt) == "process-meta")

# ── load_routing (the cascade reads the root constitution) ──
routing = sr.load_routing(str(REPO / ".claude"))
check("load_routing reads the 2 classes", set(["protocol-canonical", "process-meta"]).issubset(routing.classes.keys()))
check("load_routing carries gate_signals", any(s.get("id") == "vocab-vs-target-mismatch" for s in routing.gate_signals))
check("load_routing default_class", routing.default_class == "protocol-canonical")

# ── cascade: a sub-manifest COMPOSES on top of root, does NOT shadow it (regression for the 2026-06-11
#    first-sub-manifest bug — find_repo_root must reach .git, not stop at the nearer sub-manifest) ──
import tempfile  # noqa: E402

with tempfile.TemporaryDirectory() as _td:
    _tdp = Path(_td)
    (_tdp / ".git").mkdir()
    (_tdp / ".claude").mkdir()
    (_tdp / ".claude" / "subject-routing.yaml").write_text(
        "version: 1\ndefault_class: protocol-canonical\n"
        "classes:\n  protocol-canonical: {has_pillar: true}\n  process-meta: {has_pillar: false}\n"
    )
    _sub = _tdp / "app" / "lamad"
    (_sub / ".claude").mkdir(parents=True)
    (_sub / ".claude" / "subject-routing.yaml").write_text(
        "version: 1\ndefault_class: protocol-canonical\nlocus: {subject: lamad}\n"
    )
    (_sub / "docs").mkdir()
    _doc = _sub / "docs" / "x.md"
    _doc.write_text("# x")
    _r = sr.load_routing(str(_doc), repo_root=str(_tdp))
    check("cascade: sub-manifest composes — root classes survive", {"protocol-canonical", "process-meta"}.issubset(_r.classes.keys()))
    check("cascade: both manifests merged into sources", len(_r.sources) == 2)
    # without explicit repo_root: find_repo_root must reach .git, NOT stop at the sub-manifest dir
    _r2 = sr.load_routing(str(_doc))
    check("cascade: find_repo_root reaches .git, sub-manifest does not shadow root", "process-meta" in _r2.classes)

print(f"\n  {_passed} assertions passed ✅")
