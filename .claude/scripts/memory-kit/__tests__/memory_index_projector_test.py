"""Tests for memory-index-projector. Run: python3 .claude/scripts/memory-kit/__tests__/memory_index_projector_test.py (exit 0 = pass)."""
import contextlib
import importlib.util
import io
import json
import os
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

# fixture repo under a tempdir — MEMORY_INDEX_ROOT keeps the module off the live index
ROOT = Path(tempfile.mkdtemp())
MEM = ROOT / ".claude" / "memory"
KIT = ROOT / ".claude" / "memory-kit"
MEM.mkdir(parents=True)
KIT.mkdir(parents=True)
os.environ["MEMORY_INDEX_ROOT"] = str(ROOT)

(MEM / "project_beta.md").write_text(
    '---\nname: project_beta\ntitle: Beta thing\ndescription: "Beta hook with an escaped \\"quote\\" inside."\n---\nbody\n'
)
(MEM / "feedback_alpha.md").write_text(
    "---\nname: feedback_alpha\ntitle: Alpha lesson\ndescription: Alpha hook, plain scalar.\n---\nbody\n"
)
(MEM / "project_nodesc.md").write_text("---\nname: project_nodesc\n---\nbody\n")
(MEM / "project_longdesc.md").write_text(
    "---\nname: project_longdesc\ntitle: Long\ndescription: " + "x" * 260 + "\n---\nbody\n"
)
(MEM / "reference_optout.md").write_text(
    "---\nname: reference_optout\ntitle: Opt out\ndescription: hidden\nindex: false\n---\nbody\n"
)
(MEM / "MEMORY.md").write_text("stale hand-written index\n")

spec = importlib.util.spec_from_file_location(
    "memory_index_projector", str(here / ".claude/scripts/memory-kit/memory-index-projector.py")
)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


entries, violations = mod.collect_entries(MEM)
names = [e["file"] for e in entries]

check("MEMORY.md excluded from entries", "MEMORY.md" not in names)
check("index: false file excluded", "reference_optout.md" not in names)
check("entries sorted by filename", names == sorted(names, key=str.lower))
check("4 entries collected", len(entries) == 4)

beta = next(e for e in entries if e["file"] == "project_beta.md")
check("escaped quotes unescaped", 'an escaped "quote" inside' in beta["desc"])
check("title read from frontmatter", beta["title"] == "Beta thing")

nodesc = next(e for e in entries if e["file"] == "project_nodesc.md")
check("missing description -> placeholder", nodesc["desc"].startswith("⚠ no description"))
check("missing description -> violation", "desc-missing:project_nodesc.md" in violations)

longd = next(e for e in entries if e["file"] == "project_longdesc.md")
check("overlong description truncated with ellipsis", len(longd["desc"]) <= mod.DESC_MAX and longd["desc"].endswith("…"))
check("overlong description -> violation", "desc-overlong:project_longdesc.md" in violations)

text = mod.render(entries)
check("render carries GENERATED header", text.startswith("<!-- GENERATED"))
check("render line format", "- [Alpha lesson](feedback_alpha.md) — Alpha hook, plain scalar.\n" in text)

p = mod.project(ROOT)
check("stale hand-written index detected", p["fresh"] is False)
check("byte count matches render", p["bytes"] == len(text.encode("utf-8")))

p = mod.apply(ROOT, quiet=True)
check("apply writes projection", (MEM / "MEMORY.md").read_text() == text)
check("apply reports fresh", p["fresh"] is True)
check("apply is idempotent", mod.project(ROOT)["fresh"] is True)

drift = json.loads((KIT / "memory-index-drift.json").read_text())
check("drift snapshot carries violations", "desc-missing:project_nodesc.md" in drift["items"])
check("under soft budget -> no over-budget item", "index-over-soft-budget" not in drift["items"])

# force over-soft: shrink the budget, re-apply, drift gains the item; restore after
_soft = mod.SOFT_BUDGET_BYTES
mod.SOFT_BUDGET_BYTES = 10
p = mod.apply(ROOT, quiet=True)
drift = json.loads((KIT / "memory-index-drift.json").read_text())
check("over soft budget -> drift item + flag", p["over_soft"] and "index-over-soft-budget" in drift["items"])
mod.SOFT_BUDGET_BYTES = _soft

# violation resolved -> drift item removed (snapshot semantics)
(MEM / "project_nodesc.md").write_text(
    "---\nname: project_nodesc\ntitle: No longer\ndescription: now described.\n---\nbody\n"
)
mod.apply(ROOT, quiet=True)
drift = json.loads((KIT / "memory-index-drift.json").read_text())
check("resolved violation leaves drift snapshot", "desc-missing:project_nodesc.md" not in drift["items"])

# --hook: hand-edit of MEMORY.md while stale -> advisory JSON on stdout
(MEM / "MEMORY.md").write_text("hand edit\n")
payload = {"tool_input": {"file_path": str(MEM / "MEMORY.md")}}
sys.stdin = io.StringIO(json.dumps(payload))
buf = io.StringIO()
with contextlib.redirect_stdout(buf):
    rc = mod.hook()
out = buf.getvalue()
check("hook exits 0 on MEMORY.md hand-edit", rc == 0)
check("hook advises MEMORY.md is generated", "GENERATED" in out and "additionalContext" in out)

# --hook: topic-file write -> re-projects (MEMORY.md regenerated from the hand-edited state)
sys.stdin = io.StringIO(json.dumps({"tool_input": {"file_path": str(MEM / "feedback_alpha.md")}}))
buf = io.StringIO()
with contextlib.redirect_stdout(buf):
    rc = mod.hook()
check("hook exits 0 on topic write", rc == 0)
check("hook re-projects the index", (MEM / "MEMORY.md").read_text().startswith("<!-- GENERATED"))

# --hook: non-memory path -> silent no-op
sys.stdin = io.StringIO(json.dumps({"tool_input": {"file_path": str(ROOT / "other.md")}}))
buf = io.StringIO()
with contextlib.redirect_stdout(buf):
    rc = mod.hook()
check("hook silent outside memory dir", rc == 0 and buf.getvalue() == "")

# --hook: malformed stdin -> fail-open
sys.stdin = io.StringIO("not json")
buf = io.StringIO()
with contextlib.redirect_stdout(buf):
    rc = mod.hook()
check("hook fail-open on malformed stdin", rc == 0 and buf.getvalue() == "")

sys.stdin = sys.__stdin__
print(f"\nmemory_index_projector_test: {_p} checks passed")
