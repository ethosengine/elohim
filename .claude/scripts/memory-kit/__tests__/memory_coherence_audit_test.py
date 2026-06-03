"""Tests for memory-coherence-audit's CITE-FORMAT-CANDIDATE scope gate.
Run: python3 .claude/scripts/memory-kit/__tests__/memory_coherence_audit_test.py (exit 0 = pass).

The format-candidate verdict must use the SAME doc-root scope as cite-gen's
migrator (_doc_roots = genesis/docs + .claude/memory). A legacy path-cite to an
id-bearing .md OUTSIDE those roots (e.g. a genesis/data entity doc — human,
presence, device, chronicle) is a healthy plain-path reference, NOT a
format-candidate: cite-gen --into would refuse to convert it, so flagging it
emits an un-actionable, permanently-stuck signal. This guards that regression.
"""
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

# import memory-coherence-audit.py (hyphenated filename) by path
_audit_path = here / ".claude/scripts/memory-kit/memory-coherence-audit.py"
spec = importlib.util.spec_from_file_location("memory_coherence_audit", str(_audit_path))
audit = importlib.util.module_from_spec(spec)
sys.modules["memory_coherence_audit"] = audit  # register before exec (dataclass needs it)
spec.loader.exec_module(audit)

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1


# ── Synthetic, fully self-contained: point the module's doc-roots at a temp tree.
D = Path(tempfile.mkdtemp())
docroot = D / "docs"
docroot.mkdir()
dataroot = D / "data"
dataroot.mkdir()
audit.REPO_ROOT = D
audit._DOC_ROOTS = [docroot]

(docroot / "spec.md").write_text("---\nid: my-spec\n---\nbody\n")
(docroot / "noid.md").write_text("---\ntitle: no id here\n---\nbody\n")
(dataroot / "entity.md").write_text('---\nid: "human-x"\n---\nentity body\n')  # quoted id, out of scope
(docroot / "code.py").write_text("x = 1\n")
nested = docroot / "superpowers" / "specs"
nested.mkdir(parents=True)
(nested / "deep.md").write_text("---\nid: deep-spec\n---\nbody\n")

# ── _within_doc_roots: pure scope logic (works on synthetic paths) ──
check("doc-root child is in scope", audit._within_doc_roots(docroot / "spec.md") is True)
check("nested doc-root descendant is in scope", audit._within_doc_roots(nested / "deep.md") is True)
check("sibling data tree is OUT of scope", audit._within_doc_roots(dataroot / "entity.md") is False)

# ── _doc_target_has_id: scope-gated format-candidate detection ──
check("in-scope id doc → format-candidate", audit._doc_target_has_id("docs/spec.md") is True)
check("in-scope nested id doc → format-candidate", audit._doc_target_has_id("docs/superpowers/specs/deep.md") is True)
check("in-scope NO-id doc → ok (not a candidate)", audit._doc_target_has_id("docs/noid.md") is False)
check(
    "OUT-of-scope id doc (genesis/data entity analog) → ok, NOT a candidate",
    audit._doc_target_has_id("data/entity.md") is False,
)
check("code path → ok (not a candidate)", audit._doc_target_has_id("docs/code.py") is False)
check("nonexistent path → ok (not a candidate)", audit._doc_target_has_id("docs/missing.md") is False)
check("leading-slash ref is normalized", audit._doc_target_has_id("/docs/spec.md") is True)

print(f"\n  {_p} assertions passed ✅")
