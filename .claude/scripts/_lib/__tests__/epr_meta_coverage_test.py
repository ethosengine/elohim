"""Subtree-coverage walk test — the .epr-meta `covers: subtree` self-responsibility claim and the
deterministic coverage signal it feeds (analog of claude-md-audit's missing-candidate census). Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_coverage_test.py  (exit 0 = pass)"""
import os, sys, tempfile, textwrap
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

from _lib import epr_meta  # noqa: E402

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
    """One file per extension — drives the substantiality (file-count + distinct-ext) predicate."""
    d.mkdir(parents=True, exist_ok=True)
    for i, ext in enumerate(exts):
        (d / f"f{i}{ext}").write_text("x")

# ── claims_subtree: the self-contained responsibility declaration ──
check("claims_subtree True on covers: subtree", epr_meta.claims_subtree({"covers": "subtree"}) is True)
check("claims_subtree False on empty manifest", epr_meta.claims_subtree({}) is False)
check("claims_subtree False on covers: dir-only", epr_meta.claims_subtree({"covers": "dir-only"}) is False)
check("claims_subtree False on root:true alone (root != subtree claim)",
      epr_meta.claims_subtree({"root": True}) is False)

# ── covers is a validated top-level key: a typo must be CAUGHT, not silently read as unclaimed ──
check("validate_meta accepts covers: subtree",
      epr_meta.validate_meta({"epr-meta-version": 1, "covers": "subtree"}) == [])
check("validate_meta accepts covers: dir-only",
      epr_meta.validate_meta({"epr-meta-version": 1, "covers": "dir-only"}) == [])
check("validate_meta REJECTS a typo'd covers value",
      any("covers" in e for e in epr_meta.validate_meta({"epr-meta-version": 1, "covers": "subtre"})))

# small thresholds so the temp tree stays cheap; complexity_exts pinned so the test is self-contained
_TUNE = dict(min_files=3, min_subdirs=4, min_exts=2, complexity_exts={".ts", ".py", ".rs"})

# ── 1. the load-bearing case: root's INCIDENTAL manifest must NOT trivially cover the repo ──
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", """
        ---
        epr-meta-version: 1
        ci-trigger: ignore
        ---
        incidental root manifest — not a governance responsibility claim
    """)
    _mkfiles(root / "big", [".ts", ".py", ".rs"])  # substantial, unclaimed
    cov = epr_meta.subtree_coverage(root, **_TUNE)
    check("incidental root manifest does NOT claim subtree", cov["covered_count"] == 0)
    check("substantial dir under un-claiming root is a GAP", "big" in [g["path"] for g in cov["gaps"]])
    check("ratio is 0.0 when the one substantial region is an unclaimed gap", cov["ratio"] == 0.0)

# ── 2. covers: subtree TERMINATES the walk — the manifest owns everything beneath it ──
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / "lib" / ".epr-meta", """
        ---
        epr-meta-version: 1
        covers: subtree
        ---
        lib is fully responsible for its subtree
    """)
    _mkfiles(root / "lib" / "deep", [".ts", ".py", ".rs"])  # substantial, but UNDER lib's claim
    cov = epr_meta.subtree_coverage(root, **_TUNE)
    check("covers: subtree counts as one covered region", cov["covered_count"] == 1)
    check("claimed dir is reported covered", "lib" in cov["covered"])
    check("substantial dir UNDER a claim is NOT a gap (walk terminated)",
          "lib/deep" not in [g["path"] for g in cov["gaps"]])
    check("ratio 1.0 when every region is claimed", cov["ratio"] == 1.0)

# ── 3. mixed: one claimed region + one gap → ratio 0.5; trivial dirs never nag ──
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / "governed" / ".epr-meta", """
        ---
        epr-meta-version: 1
        covers: subtree
        ---
    """)
    _mkfiles(root / "governed", [".ts", ".py"])
    _mkfiles(root / "ungoverned", [".ts", ".py", ".rs"])  # substantial gap
    _mkfiles(root / "tiny", [".ts"])                        # 1 file → NOT substantial
    cov = epr_meta.subtree_coverage(root, **_TUNE)
    gap_paths = [g["path"] for g in cov["gaps"]]
    check("one covered + one gap → ratio 0.5", cov["ratio"] == 0.5)
    check("the ungoverned substantial dir is the gap", gap_paths == ["ungoverned"])
    check("trivial single-file dir is NOT flagged a gap", "tiny" not in gap_paths)

# ── 4. excluded dirs (node_modules etc.) are never walked ──
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _mkfiles(root / "node_modules" / "pkg", [".ts", ".py", ".rs"])  # substantial but excluded
    cov = epr_meta.subtree_coverage(root, **_TUNE)
    check("skip-dir contents never become a gap",
          all("node_modules" not in g["path"] for g in cov["gaps"]))
    check("empty governable tree → ratio 1.0 (nothing to govern is covered)", cov["ratio"] == 1.0)

# ── 5. exclude_globs prunes by path pattern (git submodules, genesis/docs/_state/**, …) ──
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _mkfiles(root / "sophia" / "core", [".ts", ".py", ".rs"])   # substantial submodule subtree
    _mkfiles(root / "real", [".ts", ".py", ".rs"])              # substantial first-party gap
    cov = epr_meta.subtree_coverage(root, **_TUNE, exclude_globs=["sophia", "sophia/**"])
    gap_paths = [g["path"] for g in cov["gaps"]]
    check("exclude_globs prunes the submodule subtree", all("sophia" not in p for p in gap_paths))
    check("first-party substantial dir still surfaces as a gap", "real" in gap_paths)

# ── 6. a schema-INVALID covers manifest must NOT be credited (degrade to gap, not over-claim) ──
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / "broken" / ".epr-meta", """
        ---
        epr-meta-version: 2
        covers: subtree
        ---
        invalid version — the resolver would reject this; coverage must NOT credit it
    """)
    _mkfiles(root / "broken", [".ts", ".py", ".rs"])  # substantial
    cov = epr_meta.subtree_coverage(root, **_TUNE)
    check("schema-invalid covers manifest is NOT credited as covered", cov["covered_count"] == 0)
    check("the dir under an invalid covers manifest is a GAP (census agrees with resolver)",
          "broken" in [g["path"] for g in cov["gaps"]])

# ── 7. the walk must NOT follow symlinked dirs (double-count + escape-root guard) ──
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _mkfiles(root / "realdir", [".ts", ".py", ".rs"])          # substantial first-party dir
    os.symlink(root / "realdir", root / "linkdir", target_is_directory=True)  # symlink to it
    cov = epr_meta.subtree_coverage(root, **_TUNE)
    gap_paths = [g["path"] for g in cov["gaps"]]
    check("symlinked dir is not walked (no double-count)", gap_paths == ["realdir"])
    check("the symlink is not itself a gap", "linkdir" not in gap_paths)

# ── 8. coverage_advice: the ASCENDING per-edit check that delivers the in-flight signal ──
# (an agent editing inside an unclaimed substantial region gets told WHERE to author governance)
_AT = dict(min_files=3, min_subdirs=4, min_exts=1, complexity_exts={".ts", ".py", ".rs"})

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td); (root / ".git").mkdir()
    _mkfiles(root / "big", [".ts", ".py", ".rs"])
    adv = epr_meta.coverage_advice(root / "big" / "newfile.ts", repo_root=root, **_AT)
    check("edit in an unclaimed substantial dir yields advice", adv is not None)
    check("advice names the gap-root to govern", adv["gap_root"] == "big")

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td); (root / ".git").mkdir()
    _wr(root / "lib" / ".epr-meta", "---\nepr-meta-version: 1\ncovers: subtree\n---\n")
    _mkfiles(root / "lib" / "deep", [".ts", ".py", ".rs"])
    adv = epr_meta.coverage_advice(root / "lib" / "deep" / "x.ts", repo_root=root, **_AT)
    check("edit UNDER a covers:subtree claim yields NO advice (already owned)", adv is None)

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td); (root / ".git").mkdir()
    (root / "tiny").mkdir(); (root / "tiny" / "only.ts").write_text("x")
    adv = epr_meta.coverage_advice(root / "tiny" / "x.ts", repo_root=root, **_AT)
    check("edit in a trivial (non-substantial) dir yields NO advice (no nag)", adv is None)

with tempfile.TemporaryDirectory() as _td:
    # gap-root is the SHALLOWEST substantial ancestor (consistent with the --epr-meta queue)
    root = Path(_td); (root / ".git").mkdir()
    _mkfiles(root / "area", [".ts", ".py"])                      # substantial (files+exts)
    for s in ("a", "b", "c", "d"):
        _mkfiles(root / "area" / s, [".ts"])
    _mkfiles(root / "area" / "sub", [".ts", ".py", ".rs"])      # also substantial, but DEEPER
    adv = epr_meta.coverage_advice(root / "area" / "sub" / "x.ts", repo_root=root, **_AT)
    check("advice points at the SHALLOWEST substantial ancestor (altitude-first)",
          adv["gap_root"] == "area")

with tempfile.TemporaryDirectory() as _td:
    # an INVALID covers manifest does not claim — advice still fires (consistent with the census)
    root = Path(_td); (root / ".git").mkdir()
    _wr(root / "z" / ".epr-meta", "---\nepr-meta-version: 9\ncovers: subtree\n---\n")
    _mkfiles(root / "z", [".ts", ".py", ".rs"])
    adv = epr_meta.coverage_advice(root / "z" / "x.ts", repo_root=root, **_AT)
    check("invalid covers manifest does not suppress advice", adv is not None and adv["gap_root"] == "z")

# ── 9. governance_cfg: ONE config source (submodules + yaml excludes + tunables) for census + nudge ──
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td); (root / ".git").mkdir()
    _wr(root / ".gitmodules", '[submodule "sophia"]\n\tpath = sophia\n\turl = x\n')
    _wr(root / ".claude" / "memory-kit" / "context-coverage.yaml",
        'epr_meta_governance:\n  min_files: 9\n  min_exts: 1\n  exclude:\n    - "**/generated"\n')
    cfg = epr_meta.governance_cfg(root)
    check("governance_cfg reads yaml tunables", cfg["min_files"] == 9)
    check("governance_cfg auto-excludes submodules (name + subtree)",
          "sophia" in cfg["exclude_globs"] and "sophia/**" in cfg["exclude_globs"])
    check("governance_cfg includes the yaml exclude globs", "**/generated" in cfg["exclude_globs"])

print(f"\n  {_passed} assertions passed ✅")
