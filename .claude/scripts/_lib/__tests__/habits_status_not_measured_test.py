#!/usr/bin/env python3
"""habits-status NOT MEASURED — the honesty leg (Law I of the 2026-08-22 guidestar).

Slice 1's honesty half: the register renders the DISAGREEMENT between a habit's
human-declared `status:` and what its evidence lane actually observed. The live
motivation is edge build #1376 — 80 scenarios, 3 passed, 1 failed, **76 skipped** —
under which `dataplane-convergence`, `notary-authority` and `operator-runtime-surface`
all read GREEN while their named concerns measured nothing at all.

Every report here is a FIXTURE. Nothing in this file reads
`genesis/a2o/reports/` — a test whose subject is "we render honest absence" must not
depend on whether a lane happened to run tonight.

The load-bearing assertion is the LAST one: `genesis/manifests/habits.yaml` is
byte-identical after a full render. Law I is that no machine writes `status:`; a test
that only checked the rendering would not have checked the law.

Mirrors habits_status_pain_test.py's importlib convention (hyphenated habits-status.py
via spec_from_file_location). Runs standalone — `python3 <this file>`, exit 0 = pass —
and is pytest-collectable (test_* functions) if pytest is ever installed here; it is NOT
installed in this container as of 2026-08-22, so the standalone runner is the real gate.
"""
import hashlib
import importlib.util
import json
import sys
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path

LIB = Path(__file__).resolve().parents[3] / "scripts" / "habits-status.py"
spec = importlib.util.spec_from_file_location("habits_status_nm", LIB)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

NOW = datetime(2026, 8, 22, 12, 0, 0, tzinfo=timezone.utc)


# ------------------------------------------------------------------ fixtures
def habit(hid, status, checks, **extra):
    h = {"id": hid, "status": status, "active": False,
         "invariant": f"{hid} holds", "checks": checks}
    h.update(extra)
    return h


# The real habits' declared concern sets, transcribed — a synthetic doc rather than the
# live register, so an operator status flip can never turn this test red.
DOC = {"updated": "2026-08-22", "habits": [
    habit("dataplane-convergence", "green", [
        "a2o @concern:federation-deploy (features/dataplane/federation-deploy.feature)",
        "a2o @concern:content-sync, @concern:blob-replication, @concern:peer-mesh, "
        "@concern:epr-projection-fallback, @concern:reconcile-inventory (edge stage)",
    ]),
    habit("notary-authority", "green", ["a2o @concern:notary-authority (…)"]),
    habit("operator-runtime-surface", "green", ["a2o @concern:operator-runtime-surface (…)"]),
    habit("doorway-failover", "red", ["a2o @concern:doorway-failover (…)"], active=True),
    habit("dev-system-equilibrium", "red", ["a2o @concern:dev-system-equilibrium (…)"]),
    habit("measure-honesty-local", "green", ["cargo test -p elohim-epr — no @concern tag"]),
]}

# The #1376 rollup, transcribed from the real report's summary.byConcern. Note
# reconcile-inventory PASSED 3 — dataplane-convergence must STILL be not-measured,
# because five of its six declared concerns have no witness. Partial coverage is not
# coverage; that is the whole of Law I.
BY_CONCERN_1376 = {
    "federation-deploy": {"passed": 0, "failed": 0, "pending": 2},
    "content-sync": {"passed": 0, "failed": 0, "pending": 4},
    "blob-replication": {"passed": 0, "failed": 0, "pending": 2},
    "peer-mesh": {"passed": 0, "failed": 0, "pending": 3},
    "epr-projection-fallback": {"passed": 0, "failed": 0, "pending": 2},
    "reconcile-inventory": {"passed": 3, "failed": 0, "pending": 4},
    "notary-authority": {"passed": 0, "failed": 0, "pending": 4},
    "operator-runtime-surface": {"passed": 0, "failed": 0, "pending": 3},
    "doorway-failover": {"passed": 0, "failed": 0, "pending": 9},
    "inventory-convergence": {"passed": 0, "failed": 1, "pending": 0},
    # dev-system-equilibrium is ABSENT ENTIRELY — the second not-measured class.
}


def report(run_id, profile, by_concern, generated_at, scenarios=None):
    return {
        "generatedAt": generated_at,
        "runId": run_id,
        "profile": profile,
        "summary": {
            "scenarios": scenarios or {"total": 80, "passed": 3, "failed": 1,
                                       "skipped": 76, "pending": 0},
            "findings": {"total": 0, "bySource": {}, "byPillar": {}},
            "byConcern": by_concern,
        },
        "findings": [],
    }


class reports_dir:
    """Point the module at a temp dir holding only the given fixture reports."""

    def __init__(self, *docs):
        self.docs = docs

    def __enter__(self):
        self.td = tempfile.TemporaryDirectory()
        base = Path(self.td.name)
        for i, doc in enumerate(self.docs):
            name = f"sprint-report-{doc.get('profile', 'x')}-{doc.get('runId', i)}.json"
            (base / name).write_text(json.dumps(doc), encoding="utf-8")
        self.orig = m.REPORTS_DIR
        m.REPORTS_DIR = base
        return base

    def __exit__(self, *exc):
        m.REPORTS_DIR = self.orig
        self.td.cleanup()
        return False


def view_of(*docs, doc=None, now=NOW):
    with reports_dir(*docs):
        return m.not_measured_view(doc or DOC, now=now)


def observed(view):
    return {r["id"]: r["observed"] for r in view["rows"]}


# ------------------------------------------------------------------ the tests
def test_1376_demotes_the_three_green_habits():
    """The live case: three habits read green while their lane measured nothing."""
    v = view_of(report("jenkins-elohim-edge-dev-1376", "dataplane", BY_CONCERN_1376,
                       "2026-08-22T11:45:42.118Z"))
    obs = observed(v)
    for hid in ("dataplane-convergence", "notary-authority", "operator-runtime-surface"):
        assert obs[hid] == m.NOT_MEASURED, f"{hid} observed {obs[hid]!r}, want not-measured"
    ids = [r["id"] for r in v["disagreements"]]
    assert ids[:3] == ["dataplane-convergence", "notary-authority",
                       "operator-runtime-surface"], ids
    # ...and each is rendered as a disagreement, never as a status edit.
    for r in v["disagreements"]:
        assert r["declared"] == "green"


def test_1376_headline_and_full_both_render_it():
    with reports_dir(report("jenkins-elohim-edge-dev-1376", "dataplane", BY_CONCERN_1376,
                            "2026-08-22T11:45:42.118Z")):
        head, full = m.headline(DOC), m.full(DOC)
    for text, where in ((head, "headline"), (full, "full")):
        assert "NOT MEASURED" in text, where
        assert "edge #1376" in text, f"{where} must name the run that produced it"
        assert "76 of 80 scenarios never ran" in text, where
        for hid in ("dataplane-convergence", "notary-authority", "operator-runtime-surface"):
            assert hid in text, f"{where} missing {hid}"
        assert "observed_status: not-measured" in text, where
        assert "last_measured:" in text, where
    # The NOT MEASURED block sits ABOVE the next-move line: a green habit with no witness
    # invalidates the selection that line is about to make.
    assert head.index("NOT MEASURED") < head.index("top red:")


def test_a_passing_concern_does_not_rescue_a_partly_unwitnessed_habit():
    """reconcile-inventory passed 3; dataplane-convergence's other five did not run."""
    v = view_of(report("jenkins-elohim-edge-dev-1376", "dataplane", BY_CONCERN_1376,
                       "2026-08-22T11:45:42.118Z"))
    row = next(r for r in v["rows"] if r["id"] == "dataplane-convergence")
    assert row["observed"] == m.NOT_MEASURED
    assert "reconcile-inventory" not in row["missing"]
    assert set(row["missing"]) == {"federation-deploy", "content-sync", "blob-replication",
                                   "peer-mesh", "epr-projection-fallback"}


def test_concern_absent_from_by_concern_entirely():
    """The second absence class: the runner never emitted the concern at all."""
    v = view_of(report("jenkins-elohim-edge-dev-1376", "dataplane", BY_CONCERN_1376,
                       "2026-08-22T11:45:42.118Z"))
    row = next(r for r in v["rows"] if r["id"] == "dev-system-equilibrium")
    assert row["observed"] == m.NOT_MEASURED
    assert row["missing"] == ["dev-system-equilibrium"]
    assert m.observe_concern(None) == m.NOT_MEASURED
    assert m.observe_concern({}) == m.NOT_MEASURED
    # An all-zero rollup is an apparatus that ran nothing — never a pass.
    assert m.observe_concern({"passed": 0, "failed": 0, "pending": 0}) == m.NOT_MEASURED


def test_no_report_at_all_is_itself_not_measured():
    """Absence of evidence is rendered, never treated as green."""
    v = view_of()  # empty reports dir
    obs = observed(v)
    assert v["newest"] is None
    assert set(obs.values()) == {m.NOT_MEASURED}, obs
    assert "no sprint report on disk" in v["run_label"]
    with reports_dir():
        head = m.headline(DOC)
    assert "NOT MEASURED" in head and "no sprint report on disk" in head
    # A missing reports DIRECTORY is the same absence, and never a crash.
    orig = m.REPORTS_DIR
    try:
        m.REPORTS_DIR = Path(tempfile.gettempdir()) / "habits-status-no-such-dir-9f2a"
        assert m.load_reports() == []
        assert "NOT MEASURED" in m.headline(DOC)
    finally:
        m.REPORTS_DIR = orig


def test_a_failure_is_a_measurement_and_outranks_absence():
    by = dict(BY_CONCERN_1376, **{"notary-authority": {"passed": 0, "failed": 2, "pending": 4}})
    v = view_of(report("r", "dataplane", by, "2026-08-22T11:45:42.118Z"))
    assert observed(v)["notary-authority"] == "red"


def test_all_passed_is_green_and_agrees_with_a_green_register():
    by = {c: {"passed": 2, "failed": 0, "pending": 0} for c in
          ["federation-deploy", "content-sync", "blob-replication", "peer-mesh",
           "epr-projection-fallback", "reconcile-inventory", "notary-authority",
           "operator-runtime-surface", "doorway-failover", "dev-system-equilibrium"]}
    v = view_of(report("r", "dataplane", by, "2026-08-22T11:45:42.118Z"))
    obs = observed(v)
    assert obs["dataplane-convergence"] == "green"
    assert obs["notary-authority"] == "green"
    assert v["not_measured"] == []
    # A red register with a green lane is the OTHER disagreement — a flip candidate,
    # still the operator's to make.
    flips = [r["id"] for r in v["disagreements"]]
    assert "doorway-failover" in flips and "dev-system-equilibrium" in flips
    assert "notary-authority" not in flips


def test_all_concerns_not_just_the_first():
    """_first_concern's regex, generalized: a check list declares EVERY tag it names."""
    checks = DOC["habits"][0]["checks"]
    assert m._concerns(checks) == ["federation-deploy", "content-sync", "blob-replication",
                                   "peer-mesh", "epr-projection-fallback",
                                   "reconcile-inventory"]
    assert m._first_concern(checks) == "federation-deploy"   # unchanged contract
    assert m._concerns(["no tags here"]) == [] and m._first_concern(["no tags"]) is None
    assert m._concerns(None) == []
    # deduped, order preserved
    assert m._concerns(["@concern:a @concern:b", "@concern:a"]) == ["a", "b"]


def test_habit_with_no_concern_tag_is_never_claimed_by_this_lane():
    v = view_of(report("r", "dataplane", BY_CONCERN_1376, "2026-08-22T11:45:42.118Z"))
    assert "measure-honesty-local" in v["no_concern"]
    assert "measure-honesty-local" not in observed(v)


def test_max_evidence_age_demotes_a_stale_green():
    green = {c: {"passed": 1, "failed": 0, "pending": 0} for c in
             ["notary-authority", "operator-runtime-surface"]}
    old = report("old-run", "dataplane", green, "2026-08-01T00:00:00.000Z")

    # Default (14d, no annotation needed): 21 days stale -> NOT MEASURED.
    v = view_of(old)
    row = next(r for r in v["rows"] if r["id"] == "notary-authority")
    assert row["observed"] == m.NOT_MEASURED and row["stale"] is True
    assert row["age"] == "21d ago"
    assert row["max_evidence_age"] == m.DEFAULT_MAX_EVIDENCE_AGE

    # A habit may declare its own, tighter or looser.
    doc = {"habits": [
        habit("notary-authority", "green", ["a2o @concern:notary-authority"],
              max_evidence_age="30d"),
        habit("operator-runtime-surface", "green", ["a2o @concern:operator-runtime-surface"],
              max_evidence_age="2d"),
    ]}
    obs = observed(view_of(old, doc=doc))
    assert obs["notary-authority"] == "green", "30d covers a 21d-old witness"
    assert obs["operator-runtime-surface"] == m.NOT_MEASURED, "2d does not"

    # `none` opts out of the clock; a malformed value does NOT silently demote.
    doc2 = {"habits": [habit("notary-authority", "green", ["a2o @concern:notary-authority"],
                             max_evidence_age="none")]}
    assert observed(view_of(old, doc=doc2))["notary-authority"] == "green"
    assert m._parse_age("14d") == 14 * 86400
    assert m._parse_age("36h") == 36 * 3600
    assert m._parse_age("2w") == 2 * 604800
    assert m._parse_age(14) == 14 * 86400
    assert m._parse_age("none") is None and m._parse_age(None) is None


def test_an_undateable_verdict_cannot_satisfy_the_clock():
    """No parseable generatedAt AND no readable mtime -> the witness has no vintage."""
    doc = {"habits": [habit("notary-authority", "green", ["a2o @concern:notary-authority"])]}
    rep = {"runId": "undated", "profile": "dataplane", "when": None,
           "byConcern": {"notary-authority": {"passed": 3, "failed": 0, "pending": 0}},
           "scenarios": {}}
    v = m.not_measured_view(doc, now=NOW, reports=[rep])
    row = v["rows"][0]
    assert row["observed"] == m.NOT_MEASURED
    assert row["undated"] is True and row["stale"] is True and row["age"] == "never"
    # ...but a habit that opted out of the clock keeps its verdict.
    doc2 = {"habits": [habit("notary-authority", "green", ["a2o @concern:notary-authority"],
                             max_evidence_age="none")]}
    assert m.not_measured_view(doc2, now=NOW, reports=[rep])["rows"][0]["observed"] == "green"


def test_last_measured_is_the_newest_real_verdict_not_the_newest_mention():
    """A run that merely MENTIONS a concern (all pending) does not reset its clock."""
    verdict = report("older-verdict", "dataplane",
                     {"notary-authority": {"passed": 4, "failed": 0, "pending": 0}},
                     "2026-08-16T12:00:00.000Z")
    mention = report("newest-mention", "dataplane", BY_CONCERN_1376,
                     "2026-08-22T11:45:42.118Z")
    v = view_of(verdict, mention)
    assert v["newest"]["runId"] == "newest-mention", "newest is by generatedAt"
    row = next(r for r in v["rows"] if r["id"] == "notary-authority")
    assert row["observed"] == m.NOT_MEASURED, "the NEWEST run measured nothing"
    assert row["age"] == "6d ago", "but a verdict does exist, 6 days back"


def test_both_lanes_and_the_legacy_single_slot_are_found():
    with tempfile.TemporaryDirectory() as td:
        base = Path(td)
        (base / "sprint-report-dataplane.json").write_text(json.dumps(
            report("jenkins-elohim-edge-dev-1376", "dataplane", BY_CONCERN_1376,
                   "2026-08-22T11:45:42.118Z")), encoding="utf-8")
        (base / "sprint-report-household-20260822T154402Z-abc.json").write_text(json.dumps(
            report("20260822T154402Z-abc", "mesh", {}, "2026-08-22T15:44:03.482Z",
                   scenarios={"total": 1, "passed": 1, "failed": 0, "skipped": 0,
                              "pending": 0})), encoding="utf-8")
        # bare `sprint-report.json` carries no lane — an April stub, not evidence.
        (base / "sprint-report.json").write_text("{}", encoding="utf-8")
        orig = m.REPORTS_DIR
        m.REPORTS_DIR = base
        try:
            got = [(r["profile"], r["runId"]) for r in m.load_reports()]
        finally:
            m.REPORTS_DIR = orig
    assert got == [("mesh", "20260822T154402Z-abc"),
                   ("dataplane", "jenkins-elohim-edge-dev-1376")], got


def test_hostile_report_bodies_never_break_session_start():
    with tempfile.TemporaryDirectory() as td:
        base = Path(td)
        (base / "sprint-report-a-1.json").write_text("{not json", encoding="utf-8")
        (base / "sprint-report-a-2.json").write_bytes(b"\xff\xfe not utf-8 \x80")
        (base / "sprint-report-a-3.json").write_text("[1,2,3]", encoding="utf-8")
        (base / "sprint-report-a-4.json").write_text(
            json.dumps({"summary": "not-a-dict", "runId": None}), encoding="utf-8")
        (base / "sprint-report-a-5.json").write_text(json.dumps(
            {"generatedAt": "not-a-date", "runId": "r5", "profile": "p",
             "summary": {"byConcern": {"notary-authority": "not-a-dict"},
                         "scenarios": {"total": "eight"}}}), encoding="utf-8")
        orig = m.REPORTS_DIR
        m.REPORTS_DIR = base
        try:
            reps = m.load_reports()          # must not raise
            head = m.headline(DOC)           # must not raise
            full = m.full(DOC)               # must not raise
        finally:
            m.REPORTS_DIR = orig
    # Only the two structurally-valid bodies survive; the other three contribute nothing.
    # Order between them is not asserted — neither carries a parseable `generatedAt`, so
    # they tie on the mtime fallback, and a tie is honestly unordered.
    assert sorted(r["runId"] for r in reps) == ["r5", "sprint-report-a-4"], \
        [r["runId"] for r in reps]
    assert "NOT MEASURED" in head and "notary-authority" in full
    assert "Traceback" not in head


def test_habits_yaml_is_never_written():
    """LAW I. No machine writes `status:` in the register — this one never opens it for
    writing at all. Bytes on disk, before and after a full render of the REAL file."""
    real = m.HABITS
    before = hashlib.sha256(real.read_bytes()).hexdigest()
    before_mtime = real.stat().st_mtime_ns
    doc = m.load()
    assert doc, "the real habits.yaml must parse for this assertion to mean anything"
    with reports_dir(report("jenkins-elohim-edge-dev-1376", "dataplane", BY_CONCERN_1376,
                            "2026-08-22T11:45:42.118Z")):
        m.headline(doc)
        m.full(doc)
        m.not_measured_view(doc)
        m.load_reports()
    after = hashlib.sha256(real.read_bytes()).hexdigest()
    assert after == before, f"habits.yaml MUTATED: {before} -> {after}"
    assert real.stat().st_mtime_ns == before_mtime, "habits.yaml was rewritten in place"
    # Belt and braces: the module has no write surface for the register at all.
    src = LIB.read_text(encoding="utf-8")
    for forbidden in ("write_text", "yaml.dump", "yaml.safe_dump", "open(HABITS",
                      "HABITS.write", ".unlink("):
        assert forbidden not in src, f"habits-status.py grew a write surface: {forbidden}"


TESTS = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]

if __name__ == "__main__":
    fails = []
    for fn in TESTS:
        try:
            fn()
        except AssertionError as e:
            fails.append(f"{fn.__name__}: {e}")
        except Exception as e:  # a crash is a failure, not a stack trace at session start
            fails.append(f"{fn.__name__}: {type(e).__name__}: {e}")
    if fails:
        print(f"FAIL ({len(fails)}/{len(TESTS)}):\n  " + "\n  ".join(fails))
        sys.exit(1)
    print(f"habits_status_not_measured_test: PASS ({len(TESTS)} tests)")
