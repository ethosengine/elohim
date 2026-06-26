"""Resolver-hook test (subprocess; verdict JSON on stdout). Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_resolver_test.py  (exit 0 = pass)"""
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

def _hook(payload=None, stdin=None):
    text = stdin if stdin is not None else json.dumps(payload)
    return subprocess.run([sys.executable, str(HOOK)], input=text, capture_output=True, text=True)

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: fm-at-birth
            class: deny
            when: { write: "*.md", new: true }
            require-frontmatter: [id, status]
            why: "no doc without id+status"
        ---
    """)
    r = _hook({"tool_name": "Write", "tool_input": {"file_path": str(root / "new.md"), "content": "bare"}})
    check("deny exit 0", r.returncode == 0)
    check("deny verdict", json.loads(r.stdout)["hookSpecificOutput"]["permissionDecision"] == "deny")
    r = _hook({"tool_name": "Write", "tool_input":
               {"file_path": str(root / "ok.md"), "content": "---\nid: x\nstatus: Draft\n---\n"}})
    check("silent-allow when frontmatter present", r.returncode == 0 and r.stdout.strip() == "")
    r = _hook({"tool_name": "Write", "tool_input": {"file_path": str(root / "x.py"), "content": "print(1)"}})
    check("silent on non-md", r.returncode == 0 and r.stdout.strip() == "")

r = _hook(stdin="not json")
check("fails open on malformed stdin", r.returncode == 0 and r.stdout.strip() == "")

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: orphan
            class: ask
            require-sibling: .epr-meta
            why: "new tree needs a manifest"
        ---
    """)
    r = _hook({"tool_name": "Write", "tool_input":
               {"file_path": str(root / "brand" / "note.md"), "content": "x"}})
    out = json.loads(r.stdout)
    check("wires is_new_subdir (ask on orphan tree)",
          out["hookSpecificOutput"]["permissionDecision"] == "ask")
    check("orphan-tree reason names .epr-meta",
          ".epr-meta" in out["hookSpecificOutput"]["permissionDecisionReason"])

# Strict-but-recoverable: a MALFORMED manifest downgrades the subtree to ASK (not a hard deny),
# and never blocks an edit of the manifest itself (so the typo is always fixable).
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", "---\nepr-meta-version: 2\n---\n")  # malformed (wrong version)
    r = _hook({"tool_name": "Write", "tool_input": {"file_path": str(root / "anything.py"), "content": "x"}})
    out = json.loads(r.stdout)
    check("malformed manifest → subtree downgraded to ASK, not deny (recoverable)",
          out["hookSpecificOutput"]["permissionDecision"] == "ask"
          and "malformed" in out["hookSpecificOutput"]["permissionDecisionReason"])
    r = _hook({"tool_name": "Write", "tool_input":
               {"file_path": str(root / ".epr-meta"), "content": "---\nepr-meta-version: 1\n---\n"}})
    check("editing the malformed .epr-meta itself is NEVER blocked (the fix path)",
          r.returncode == 0 and "permissionDecision" not in r.stdout)

# Parse-bomb manifest (deep flow nesting) must NOT hang or RecursionError → refused pre-parse → ASK.
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", "---\nx: " + "[" * 300 + "]" * 300 + "\n---\n")
    r = _hook({"tool_name": "Write", "tool_input": {"file_path": str(root / "f.py"), "content": "x"}})
    check("parse-bomb manifest → ASK (refused pre-parse, no hang/RecursionError)",
          r.returncode == 0 and json.loads(r.stdout)["hookSpecificOutput"]["permissionDecision"] == "ask")

print(f"\n  {_passed} assertions passed ✅")
