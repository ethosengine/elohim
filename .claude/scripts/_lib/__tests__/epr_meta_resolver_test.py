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

# Edit is evaluated on its POST-edit content, so a content-triggered rule sees what the edit
# INTRODUCES — not the stale on-disk pre-image. Modeled on the specs/ p2p-design-gate contains-any
# rule; `new:` is dropped so the rule also applies to edits (the class this fix targets — a
# `new: true` rule is Write-only by design, since Edit never sets is_new).
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: p2p-design-gate
            class: ask
            when: { write: "*.md", contains-any: ["GET /api/v1", "PRIMARY KEY", "uuid"] }
            validator: epr:validator-p2p-design-gate
            why: "new data-entity designs pass the p2p-design-gate"
        ---
    """)
    doc = root / "design.md"
    doc.write_text("# Design\n\nA plain paragraph, no data-entity patterns yet.\n")
    # Edit that INTRODUCES a matched pattern → the rule now fires (was silent pre-fix, which
    # evaluated the stale pre-edit on-disk content that lacked the pattern).
    r = _hook({"tool_name": "Edit", "tool_input": {
        "file_path": str(doc),
        "old_string": "no data-entity patterns yet.",
        "new_string": "add a GET /api/v1/things route."}})
    check("Edit introducing a matched pattern → post-edit content fires (ask)",
          r.returncode == 0
          and json.loads(r.stdout)["hookSpecificOutput"]["permissionDecision"] == "ask")
    # Edit unrelated to any rule → post-edit content has no match → stays silent.
    r = _hook({"tool_name": "Edit", "tool_input": {
        "file_path": str(doc),
        "old_string": "A plain paragraph",
        "new_string": "A revised paragraph"}})
    check("Edit unrelated to any rule stays silent (post-edit content, not pre-image)",
          r.returncode == 0 and r.stdout.strip() == "")
    # old_string absent from disk → the Edit will fail anyway → hook exits 0 silently.
    r = _hook({"tool_name": "Edit", "tool_input": {
        "file_path": str(doc),
        "old_string": "this text is not in the file",
        "new_string": "GET /api/v1/whatever"}})
    check("Edit whose old_string is absent → silent (the Edit will fail anyway)",
          r.returncode == 0 and r.stdout.strip() == "")

# Seam birth-rule (plan task P4.5): a `dedupe-of`-anchored inject fires on a decision-surface
# shape (a new verdict/decision/outcome/reason enum, a `decide_*` fn, a route registration) and
# stays quiet on an unrelated write — modeled on the live rules in
# doorway/doorway-service/src/.epr-meta, steward/node/src/.epr-meta, crates/seam-contracts/.epr-meta.
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: seam-birth-rule
            class: inject
            when:
              write: "*.rs"
              contains-any: ["Decision {", "Verdict {", "Outcome {", "Reason {",
                             "Disposition {", "fn decide_", "Answer<", ".route(",
                             "match (method"]
            dedupe-of: ".claude/skills/p2p-design-gate/SKILL.md (Step 4: Concern-Canon Answer)"
            why: "answer the concern canon and register in seam-registry.yaml before shipping"
        ---
    """)
    r = _hook({"tool_name": "Write", "tool_input": {
        "file_path": str(root / "decide.rs"),
        "content": "pub enum FetchOutcome { Present, Absent, Unreachable }\n"}})
    out = json.loads(r.stdout)
    # inject is advisory (permit, never blocks) — it surfaces as additionalContext, not
    # permissionDecision (that field is reserved for deny/ask, the two classes that can block).
    check("decision-surface shape (Outcome {) fires inject (additionalContext, no permissionDecision)",
          r.returncode == 0
          and "permissionDecision" not in out["hookSpecificOutput"]
          and "additionalContext" in out["hookSpecificOutput"])
    check("inject verdict cites the p2p-design-gate Step 4 pointer",
          "Step 4" in out["hookSpecificOutput"]["additionalContext"])
    r = _hook({"tool_name": "Write", "tool_input": {
        "file_path": str(root / "plain.rs"),
        "content": "pub fn add(a: i32, b: i32) -> i32 { a + b }\n"}})
    check("plain .rs diff (no decision-surface shape) stays silent",
          r.returncode == 0 and r.stdout.strip() == "")
    r = _hook({"tool_name": "Write", "tool_input": {
        "file_path": str(root / "route.rs"),
        "content": "fn decide_dispatch() -> Disposition { Disposition::NotFound }\n"}})
    out = json.loads(r.stdout)
    check("a `decide_*` fn returning a *-suffixed enum also fires inject",
          r.returncode == 0 and "additionalContext" in out["hookSpecificOutput"])

print(f"\n  {_passed} assertions passed ✅")
