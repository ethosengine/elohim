"""In-flight coverage-nudge integration test (drives the resolver hook as a subprocess, the way the
harness does). Proves: an agent editing inside an UNCLAIMED substantial region gets the signal; an
agent in an OWNED region (or authoring the manifest) does not; and the nudge is debounced. Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_coverage_nudge_test.py  (exit 0 = pass)"""
import json, subprocess, sys, tempfile, textwrap
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        break
    here = here.parent
REPO = here
HOOK = REPO / ".claude/hooks/epr-meta-resolver.py"

_passed = 0
def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")

def _wr(p, body):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(textwrap.dedent(body).lstrip())

def _mkfiles(d, exts):
    d.mkdir(parents=True, exist_ok=True)
    for i, ext in enumerate(exts):
        (d / f"f{i}{ext}").write_text("x")

def _ctx(payload):
    res = subprocess.run([sys.executable, str(HOOK)], input=json.dumps(payload),
                         capture_output=True, text=True)
    try:
        return json.loads(res.stdout or "{}").get("hookSpecificOutput", {}).get("additionalContext", "")
    except Exception:
        return f"<bad stdout: {res.stdout!r} stderr: {res.stderr!r}>"

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td); (root / ".git").mkdir()
    # the hook reads tunables from governance_cfg → the temp repo's yaml; small thresholds keep it cheap
    _wr(root / ".claude" / "memory-kit" / "context-coverage.yaml",
        "epr_meta_governance:\n  min_files: 3\n  min_subdirs: 4\n  min_exts: 1\n")
    _mkfiles(root / "ungoverned", [".ts", ".py", ".rs"])          # substantial, unclaimed
    _wr(root / "owned" / ".epr-meta", "---\nepr-meta-version: 1\ncovers: subtree\n---\n")
    _mkfiles(root / "owned" / "sub", [".ts", ".py", ".rs"])       # substantial, but UNDER a claim

    c1 = _ctx({"tool_name": "Edit", "tool_input": {"file_path": str(root / "ungoverned" / "contributors.ts")}})
    check("nudge fires editing an UNCLAIMED substantial region", "epr-meta coverage" in c1 and "ungoverned" in c1)
    check("nudge names the authoring move (covers: subtree)", "covers: subtree" in c1)

    c2 = _ctx({"tool_name": "Edit", "tool_input": {"file_path": str(root / "ungoverned" / "other.ts")}})
    check("nudge is DEBOUNCED on the next edit in the same region (no nag)", "epr-meta coverage" not in c2)

    c3 = _ctx({"tool_name": "Edit", "tool_input": {"file_path": str(root / "owned" / "sub" / "x.ts")}})
    check("no nudge when editing an already-OWNED region", "epr-meta coverage" not in c3)

    c4 = _ctx({"tool_name": "Write", "tool_input": {
        "file_path": str(root / "fresh" / ".epr-meta"),
        "content": "---\nepr-meta-version: 1\ncovers: subtree\n---\n"}})
    check("no nudge when AUTHORING an .epr-meta (you're already governing)", "epr-meta coverage" not in c4)

print(f"\n  {_passed} assertions passed ✅")
