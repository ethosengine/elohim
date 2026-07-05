"""Git-native adapter for the .epr-meta compose-gate — pure-function + integration tests. Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_git_test.py  (exit 0 = pass)"""
import sys
from pathlib import Path

here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        REPO = here
        break
    here = here.parent

from _lib import epr_meta_git as gg  # noqa: E402
from _lib.epr_meta import Verdict  # noqa: E402

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


# ── build_write (PURE) ──
w = gg.build_write("a/b/new.md", "A", "x", head_has_parent=False)
check("new file in new subdir -> is_new + is_new_subdir",
      w == {"path": "a/b/new.md", "content": "x", "is_new": True, "is_new_subdir": True})
check("modified existing -> neither new nor new_subdir",
      gg.build_write("a/b.md", "M", "x", head_has_parent=True) == {
          "path": "a/b.md", "content": "x", "is_new": False, "is_new_subdir": False})
check("new file in EXISTING dir -> is_new but not is_new_subdir",
      gg.build_write("a/b/new.md", "A", "x", head_has_parent=True)["is_new_subdir"] is False)

# ── decide (PURE) — the ask->git mapping fork ──
check("deny blocks", gg.decide([Verdict("deny", "no", "r1")], ack=False)[0] == 1)
check("ask blocks without ack", gg.decide([Verdict("ask", "c", "r1")], ack=False)[0] == 1)
check("ask allows WITH ack", gg.decide([Verdict("ask", "c", "r1")], ack=True)[0] == 0)
check("inject advises but allows", gg.decide([Verdict("inject", "fyi", "r1")], ack=False)[0] == 0)
check("None (ungoverned) allows", gg.decide([None], ack=False)[0] == 0)
check("most-severe wins (deny over inject)",
      gg.decide([Verdict("inject", "f", "r1"), Verdict("deny", "n", "r2")], ack=False)[0] == 1)
check("empty changeset allows", gg.decide([], ack=False)[0] == 0)

# ── run() integration through the REAL cascade, via an injected fake git runner (no real repo) ──
# A frontmatter-less .md staged under genesis/docs/superpowers/plans/ must be DENIED by the real
# `doc-frontmatter-at-birth` rule — the same rule that denied this plan's own doc.
NOFM = str(REPO / "genesis/docs/superpowers/plans/__gate_probe__.md")


def fake_runner(*args):
    if args[:2] == ("diff", "--cached"):
        return f"A\t{NOFM}\n"
    if args[0] == "show":
        return "no frontmatter, just prose\n"
    if args[0] == "ls-tree":
        return "40000 tree abc\tgenesis/docs/superpowers/plans\n"  # parent dir exists
    return ""


code, msgs = gg.run("staged", None, ack=False, runner=fake_runner)
check("run() BLOCKS a frontmatter-less plan via the real .epr-meta cascade", code == 1)
check("run() surfaces the frontmatter reason",
      any("frontmatter" in m.lower() for m in msgs))

# A compliant plan (all required fields present) staged in the same tree must PASS.
def fake_runner_ok(*args):
    if args[:2] == ("diff", "--cached"):
        return f"A\t{NOFM}\n"
    if args[0] == "show":
        return "---\nid: x\nstatus: Draft\ncites: []\n---\n# ok\n"
    if args[0] == "ls-tree":
        return "40000 tree abc\tgenesis/docs/superpowers/plans\n"
    return ""


code_ok, _ = gg.run("staged", None, ack=False, runner=fake_runner_ok)
check("run() ALLOWS a plan carrying id+status+cites", code_ok == 0)

print(f"\n{_passed} checks passed")
