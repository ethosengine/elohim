#!/usr/bin/env python3
"""Habit atoms in the `.epr-meta` governance package — the composed delivery register.

The register stopped being one hand-written file on 2026-08-27. A habit is now declared where its
concern lives (`<dir>/.epr-meta/<id>.habit.md`), resolved by the same walk the compose-gate uses,
and `genesis/manifests/habits.yaml` is the GENERATED projection of that walk.

Two classes of assertion here, and the second is the load-bearing one:

  * SCHEMA — a habit is born observable (`status`, `checks` unless `unwired`) and born retirable
    (`retire-when`, and never a bare `never`). These were prose in the covenant for seven months
    and no script ever checked them.
  * ROLL-UP — the assertions no single manifest can make: one habit per id, one habit per
    `@concern:` (the id is the join across register / CI / Gherkin, and composing declaration
    must not fragment that namespace), and `max 2 active` (the WIP fence bounds ATTENTION, which
    does not compose). Every one of these is newly enforceable BECAUSE declaration composed —
    a flat file could rely on the eye.

And one fidelity assertion over the LIVE tree: the projection round-trips. A generated register
that cannot be proven equal to its source is the drift it exists to prevent.

Fixtures are temp trees. Nothing here mutates the repo. Standalone — `python3 <this file>`,
exit 0 = pass — and pytest-collectable if pytest is ever installed here (it is not, as of
2026-08-27, so the standalone runner is the real gate).
"""
import sys
import tempfile
from pathlib import Path

_SCRIPTS = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(_SCRIPTS))

from _lib import epr_habits as eh  # noqa: E402

REPO = Path(__file__).resolve().parents[3].parent

FULL = """---
epr-habit-version: 1
id: {hid}
invariant: >
  {inv}
status: {status}
active: {active}
checks:
  - "a2o @concern:{concern} (some.feature)"
retire-when: >
  when the thing it watches becomes unrepresentable rather than merely refused
---
2026-08-27: declared.
"""


def _write(root: Path, where: str, hid: str, **kw) -> Path:
    d = root / where / ".epr-meta"
    d.mkdir(parents=True, exist_ok=True)
    p = d / f"{hid}.habit.md"
    body = FULL.format(hid=hid, inv=kw.get("inv", "it holds"), status=kw.get("status", "green"),
                       active=str(kw.get("active", False)).lower(),
                       concern=kw.get("concern", hid))
    for old, new in kw.get("patch", []):
        body = body.replace(old, new)
    p.write_text(body, encoding="utf-8")
    return p


def test_governed_dir_is_the_parent_of_the_package():
    """A habit's scope is the directory its package sits in — that is the whole point of moving
    declaration out of a central file, so it is asserted directly rather than implied."""
    p = Path("/repo/elohim/elohim-storage/.epr-meta/x.habit.md")
    assert eh.governed_dir(p) == Path("/repo/elohim/elohim-storage"), eh.governed_dir(p)


def test_evidence_is_the_body_and_survives_the_round_trip():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        p = _write(root, "svc", "a-habit")
        h = eh.load(p)
        assert h["evidence"] == "2026-08-27: declared.", repr(h["evidence"])
        assert h["id"] == "a-habit"
        lines = eh.project_habit(h)
        assert lines[0] == "  - id: a-habit", lines[0]
        assert "    evidence: >" in lines, lines


def test_list_valued_evidence_stays_in_the_frontmatter():
    """`reach-enforced-everywhere` carries a LIST of evidence entries, not a prose ledger. Moving
    it to the body would silently change the field's TYPE — the projection would emit a folded
    scalar and every consumer would start reading one string where there had been five entries.
    This is the bug the first cut of the migration actually shipped; it is fixed and pinned."""
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        p = _write(root, "svc", "listed", patch=[
            ("retire-when: >", "evidence:\n  - \"first\"\n  - \"second\"\nretire-when: >")])
        h = eh.load(p)
        assert h["evidence"] == ["first", "second"], h["evidence"]
        assert "    evidence: >" not in eh.project_habit(h)


def test_unwired_refuses_checks_and_checked_refuses_emptiness():
    """`unwired` means committed-to with NO way to observe it. A habit that declares checks is
    not unwired, and a green/red habit with no check is a declaration standing on intention."""
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        bad = eh.load(_write(root, "a", "wired-unwired", status="unwired"))
        errs = eh.validate(bad, "x")
        assert any("NO runnable check" in e for e in errs), errs

        p = _write(root, "b", "no-checks", patch=[
            ('checks:\n  - "a2o @concern:no-checks (some.feature)"\n', "")])
        errs = eh.validate(eh.load(p), "x")
        assert any("non-empty list of runnable checks" in e for e in errs), errs


def test_retire_when_is_required_and_a_bare_never_is_refused():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        p = _write(root, "a", "no-exit", patch=[
            ("retire-when: >\n  when the thing it watches becomes unrepresentable rather than "
             "merely refused\n", "")])
        assert any("missing `retire-when`" in e for e in eh.validate(eh.load(p), "x"))

        q = _write(root, "b", "bare-never", patch=[
            ("retire-when: >\n  when the thing it watches becomes unrepresentable rather than "
             "merely refused", "retire-when: never")])
        assert any("bare `retire-when: never`" in e for e in eh.validate(eh.load(q), "x"))

        r = _write(root, "c", "reasoned-never", patch=[
            ("when the thing it watches becomes unrepresentable rather than merely refused",
             "never: a constitutional floor, because the failure mode is silent")])
        assert eh.validate(eh.load(r), "x") == [], eh.validate(eh.load(r), "x")


def test_filename_must_match_the_id():
    """The filename is the address the cascade resolves; an id that disagrees with it is a habit
    with two names, and the id is the join across register, CI and Gherkin."""
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        p = _write(root, "a", "declared-name")
        moved = p.with_name("other-name.habit.md")
        p.rename(moved)
        assert any("must match `id`" in e for e in eh.validate(eh.load(moved), "x"))


def test_rollups_no_single_manifest_could_make():
    """Duplicate id, a concern claimed twice, and the WIP fence — the three assertions that only
    exist once declaration composes, and the reason composing does not mean unbounded."""
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / ".git").mkdir()  # census discovery falls back to a walk without a git index
        _write(root, "a", "twice", concern="ca")
        _write(root, "b", "twice", concern="cb")
        _write(root, "c", "one", concern="shared")
        _write(root, "d", "two", concern="shared")
        _write(root, "e", "act-1", active=True)
        _write(root, "f", "act-2", active=True)
        _write(root, "g", "act-3", active=True)
        _, errs = eh.census(root)
        assert any("duplicate habit id `twice`" in e for e in errs), errs
        assert any("@concern:shared is claimed by two habits" in e for e in errs), errs
        assert any("WIP fence breached: 3" in e for e in errs), errs


def test_covenant_order_is_priority_and_a_stale_rank_is_an_error():
    """Declaration is local; PRIORITY is not — "the top red" is an operator judgment across
    domains. An unranked habit sorts last (visible, never jumping the queue); a rank naming no
    habit is a queue position nobody can take."""
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / ".git").mkdir()
        _write(root, "aaa", "zzz-last-alphabetically")
        _write(root, "zzz", "aaa-first-alphabetically")
        _write(root, "mid", "unranked")
        cov = root / ".epr-meta"
        cov.mkdir(parents=True, exist_ok=True)
        (cov / "habits-covenant.md").write_text(
            "---\nversion: 1\norder:\n  - zzz-last-alphabetically\n  - aaa-first-alphabetically\n"
            "  - ghost-habit\n---\nprose\n", encoding="utf-8")
        habits, errs = eh.census(root)
        assert [h["id"] for h in habits] == [
            "zzz-last-alphabetically", "aaa-first-alphabetically", "unranked"], \
            [h["id"] for h in habits]
        assert any("ranks `ghost-habit`" in e for e in errs), errs


def test_in_scope_walks_up_nearest_first():
    """The affordance a flat register could never have: "the top red IN SCOPE". A habit governs a
    path when its declaring directory is an ancestor of it — the repo root governing everything
    is exactly what a cross-cutting habit means."""
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / ".git").mkdir()
        _write(root, ".", "cross-cutting", concern="cc")
        _write(root, "svc", "crate-wide", concern="cw")
        _write(root, "other", "unrelated", concern="ur")
        (root / "svc" / "src").mkdir(parents=True, exist_ok=True)
        target = root / "svc" / "src" / "file.rs"
        target.write_text("", encoding="utf-8")
        ids = [h["id"] for h in eh.in_scope(target, root)]
        assert ids == ["crate-wide", "cross-cutting"], ids


def test_a_habit_outside_the_governance_package_declares_nothing():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / ".git").mkdir()
        loose = root / "svc"
        loose.mkdir(parents=True)
        (loose / "stray.habit.md").write_text(FULL.format(
            hid="stray", inv="i", status="green", active="false", concern="s"), encoding="utf-8")
        assert eh.discover(root) == []


def test_live_register_projection_is_current():
    """The fidelity gate, over the real tree: the census is valid and the checked-in projection
    equals what the atoms render. Two hand-written homes for one truth is a failure mode this
    repo has already paid for twice — this is the assertion that keeps it to one."""
    habits, errs = eh.census(REPO)
    assert errs == [], errs
    assert habits, "no habit atoms discovered in the live tree"
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "habits_project", REPO / ".claude" / "scripts" / "habits-project.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    rendered, errs = mod.render(REPO)
    assert errs == [], errs
    on_disk = (REPO / "genesis" / "manifests" / "habits.yaml").read_text(encoding="utf-8")
    assert rendered == on_disk, ("genesis/manifests/habits.yaml is STALE — run "
                                 ".claude/scripts/habits-project.py")


if __name__ == "__main__":
    fns = [(n, f) for n, f in sorted(globals().items()) if n.startswith("test_")]
    failed = 0
    for name, fn in fns:
        try:
            fn()
            print(f"  ok   {name}")
        except AssertionError as e:
            failed += 1
            print(f"  FAIL {name}: {e}")
    print(f"\n{len(fns) - failed}/{len(fns)} passed")
    raise SystemExit(1 if failed else 0)
