"""ci-trigger collect + render test. Run:
python3 .claude/scripts/_lib/__tests__/ci_trigger_test.py  (exit 0 = pass)"""
import sys, tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

from _lib import ci_trigger  # noqa: E402

_passed = 0
def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    (root / ".epr-meta").write_text(
        "---\nepr-meta-version: 1\nroot: true\n"
        "ci-trigger:\n  ignore:\n    - .claude/\n    - CLAUDE.md\n    - .claude/\n---\n")  # dup on purpose
    pats = ci_trigger.collect_ci_trigger(root)
    check("collect reads root ci-trigger.ignore in order", pats[:2] == [".claude/", "CLAUDE.md"])
    check("collect de-duplicates", pats == [".claude/", "CLAUDE.md"])

    rendered = ci_trigger.render_ci_ignore(pats)
    check("render carries a GENERATED-DO NOT EDIT header", "GENERATED" in rendered and "DO NOT EDIT" in rendered)
    check("render emits one pattern per line, body after header",
          rendered.rstrip().endswith("CLAUDE.md") and "\n.claude/\n" in rendered)
    check("render is deterministic", ci_trigger.render_ci_ignore(pats) == rendered)

    fresh, _ = ci_trigger.verify(root)
    check("verify: no .ci-ignore on disk -> stale", fresh is False)
    (root / ".ci-ignore").write_text(rendered)
    fresh, _ = ci_trigger.verify(root)
    check("verify: matching .ci-ignore -> fresh", fresh is True)

    (root / ".epr-meta").write_text("---\nepr-meta-version: 1\nroot: true\n---\n")
    check("collect tolerates absent ci-trigger", ci_trigger.collect_ci_trigger(root) == [])

print(f"\n  {_passed} assertions passed ✅")
