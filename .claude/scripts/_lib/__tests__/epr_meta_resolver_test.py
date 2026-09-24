"""Resolver-hook test (subprocess; verdict JSON on stdout). Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_resolver_test.py  (exit 0 = pass)"""
import hashlib, json, shutil, subprocess, sys, tempfile, textwrap
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

# Algedonic phase-1 Task 4: the measure mint graduates to typed evidence. A hard-ceiling write
# additively mints stock/limit/bound_ref (+ concern when the binding's `params` carries one) onto
# the SAME fingerprinted architecture finding the pre-slice code filed — and the fingerprint
# formula (sha256(f"{rule_id}|{rel}")[:12]) is untouched by any of it.
def _mint_hard_ceiling(with_concern: bool):
    """Fresh tempdir repo with a policy-bound measure rule (`test-loc-ceiling@1`, loc-hard=5) at
    root, optionally carrying `params: {concern: "notary-authority"}`. Writes a NON-EXISTENT
    6-line target so the hard ceiling fires immediately; returns (entry, root)."""
    _td = tempfile.mkdtemp()
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".claude" / "epr-meta" / "policies.yaml", """
        epr-meta-policies-version: 1
        policies:
          - id: test-loc-ceiling
            version: 1
            class: measure
            measure:
              kind: level
              loc-soft: 3
              loc-hard: 5
            why: "test ceiling — algedonic Task 4"
    """)
    if with_concern:
        _wr(root / ".epr-meta", """
            ---
            epr-meta-version: 1
            root: true
            rules:
              - id: rs-loc-ceiling
                policy: test-loc-ceiling@1
                params: { concern: "notary-authority" }
            ---
        """)
    else:
        _wr(root / ".epr-meta", """
            ---
            epr-meta-version: 1
            root: true
            rules:
              - id: rs-loc-ceiling
                policy: test-loc-ceiling@1
            ---
        """)
    target = root / "big.py"  # non-existent target — nothing pre-exists on disk
    assert not target.exists()
    content = "x\n" * 6  # 6 lines >= loc-hard 5
    r = _hook({"tool_name": "Write", "tool_input": {"file_path": str(target), "content": content}})
    assert r.returncode == 0, r.stderr
    # Fixture ledger only — this tempdir's own .claude/data/, never the live repo ledger.
    ledger = root / ".claude" / "data" / "architecture-findings.jsonl"
    lines = ledger.read_text().splitlines()
    assert len(lines) == 1, f"expected exactly one minted finding, got {lines}"
    return json.loads(lines[0]), root

entry_with, root_with = _mint_hard_ceiling(with_concern=True)
entry_without, root_without = _mint_hard_ceiling(with_concern=False)

expected_fp = hashlib.sha256("rs-loc-ceiling|big.py".encode()).hexdigest()[:12]
check("mint (concern) fp matches the documented sha256(rule_id|path)[:12] formula",
      entry_with["fp"] == expected_fp)
check("mint (no concern) fp matches the SAME formula",
      entry_without["fp"] == expected_fp)
check("fingerprint is IDENTICAL with and without the new evidence/concern fields present",
      entry_with["fp"] == entry_without["fp"] == expected_fp)

check("mint (concern) carries structured stock (measured LoC)", entry_with["stock"] == 6)
check("mint (concern) carries structured limit (the ceiling)", entry_with["limit"] == 5)
check("mint (concern) carries bound_ref = <policy-id>@<version>#<declaring-manifest>",
      entry_with["bound_ref"] == "test-loc-ceiling@1#.epr-meta")
check("mint (concern) carries the explicit binding-param concern",
      entry_with["concern"] == "notary-authority")

check("mint (no concern) carries the SAME structured stock", entry_without["stock"] == 6)
check("mint (no concern) carries the SAME structured limit", entry_without["limit"] == 5)
check("mint (no concern) carries the SAME bound_ref shape",
      entry_without["bound_ref"] == "test-loc-ceiling@1#.epr-meta")
check("mint (no concern) OMITS the concern key — honest absence, never guessed",
      "concern" not in entry_without)

# Pre-existing fields survive untouched (append-compat: old ledger rows lack the new keys but
# every reader uses `.get(...)`; these are the ORIGINAL fields the pre-slice mint always wrote).
check("mint keeps the original fp/rule/policy/path/detail/status shape",
      entry_with["rule"] == "rs-loc-ceiling" and entry_with["policy"] == "test-loc-ceiling@1"
      and entry_with["path"] == "big.py" and entry_with["status"] == "open"
      and "detail" in entry_with and "first_seen" in entry_with)

shutil.rmtree(root_with, ignore_errors=True)
shutil.rmtree(root_without, ignore_errors=True)

# ── C7: the native frame evidence reaches the PreToolUse surface and the witness ───────────
# `epr govern` returns each verdict's opaque `evidence`; a frame verdict carries
# `classificationCid`. The resolver surfaces an advisory `[frame] <reason>` line and witnesses
# `classification <cid>`. Decision authority is unchanged (the stub agrees: permit/dispatch).
_STUB_CID = "bafyreifhbla6a66gg7u34dbpodxcp4h6pcuqxtxv2jkvsynwfjgnicbpxi"
_FRAME_REASON = ("net-new apex-sovereignty framing needs an explicit bounded frame · frame "
                 "bafyreif…zo4e · classification bafyreif…bpxi · abstain")


def _stub_epr(dirpath: Path, evidence) -> Path:
    verdict = {"class": "dispatch", "ruleId": "sov-guard", "policyRef": None,
               "reason": "validator flagged this write: " + _FRAME_REASON}
    if evidence is not None:
        verdict["evidence"] = evidence
    payload = {"decision": "permit", "winningClass": "dispatch", "ruleId": "sov-guard",
               "reason": verdict["reason"], "referReason": None, "diagnostics": [],
               "verdicts": [verdict],
               "evaluator": {"id": "stub-epr", "version": "0", "cid": "sha256:stub"}}
    stub = dirpath / "epr-stub"
    stub.write_text("#!/usr/bin/env python3\nimport sys\nsys.stdin.read()\n"
                    f"print({json.dumps(json.dumps(payload))})\n")
    stub.chmod(0o755)
    return stub


def _frame_case(evidence):
    import os
    td = Path(tempfile.mkdtemp())
    (td / ".git").mkdir()
    _wr(td / ".epr-meta", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: sov-guard
            class: dispatch
            when: { write: "*.md" }
            validator: epr:validator-sovereignty-ontology-guard
            parameters: { dispatch-agent: storyteller, dispatch-prompt: "review the framing" }
            why: test
        ---
    """)
    env = {**os.environ, "EPR_BIN": str(_stub_epr(td, evidence))}
    env.pop("CLAUDE_PROJECT_DIR", None)
    r = subprocess.run([sys.executable, str(HOOK)], capture_output=True, text=True, env=env,
                       input=json.dumps({"tool_name": "Write", "session_id": "frame-test",
                                         "tool_input": {"file_path": str(td / "w.md"),
                                                        "content": "Members hold a self-sovereign identity.\n"}}))
    ledger = td / ".claude/data/governance-findings.jsonl"
    rows = [json.loads(ln) for ln in ledger.read_text().splitlines()] if ledger.is_file() else []
    shutil.rmtree(td, ignore_errors=True)
    return r, rows


# native_frame_evidence_is_surfaced_and_witnessed
_r, _rows = _frame_case({"classificationCid": _STUB_CID, "reason": _FRAME_REASON,
                         "frameRef": "bafyreif2vuz6tnzwtvkgogulw25u2h65edipkbyxw5guf2yyezy5i5zo4e",
                         "verdict": "abstain"})
check("frame: hook exits 0 (dispatch never blocks)", _r.returncode == 0)
_ctx = json.loads(_r.stdout)["hookSpecificOutput"].get("additionalContext", "") if _r.stdout.strip() else ""
check("frame: decision authority unchanged — no permissionDecision on the dispatch permit",
      _r.stdout.strip() and "permissionDecision" not in json.loads(_r.stdout)["hookSpecificOutput"])
check("frame: the emitted context carries the advisory `[frame]` line with `frame bafy`",
      "[frame] " in _ctx and "frame bafy" in _ctx)
check("frame: the dispatch directive still rides along", "DISPATCH NOW" in _ctx)
_disp = [row for row in _rows if row.get("class") == "dispatch" and row.get("ruleId") == "sov-guard"]
check("frame: the dispatch witness row carries `classification <cid>`",
      len(_disp) == 1 and f"classification {_STUB_CID}" in _disp[0]["witness"])

# no_evidence_no_frame_line
_r, _rows = _frame_case(None)
_ctx = json.loads(_r.stdout)["hookSpecificOutput"].get("additionalContext", "") if _r.stdout.strip() else ""
check("no evidence: no `[frame]` line", "[frame]" not in _ctx and "DISPATCH NOW" in _ctx)
check("no evidence: no `classification` check witnessed",
      not any(str(c).startswith("classification ") for row in _rows for c in row.get("witness", [])))

print(f"\n  {_passed} assertions passed ✅")
