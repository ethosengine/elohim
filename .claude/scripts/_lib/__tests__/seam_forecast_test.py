"""seam_forecast.py test — the seeded forecast + calibration ledger (plan P4.4). Run:
python3 .claude/scripts/_lib/__tests__/seam_forecast_test.py  (exit 0 = pass)

Fixture ledgers live under tempfile.TemporaryDirectory(); the REAL
`.claude/data/seam-calibration.jsonl` is never appended to by this test — the append-only ledger
is a historical record, and a test that seeded it with synthetic outcomes would corrupt the very
canon-growth-rate measure the instrument exists to compute.

The live half is read-only and load-bearing: every fingerprint stored in the real
`.claude/epr-meta/seam-forecast.yaml` must equal its recomputed value, because a hand-edited
fingerprint silently orphans a row's calibration entries and nothing downstream can detect it
later.
"""
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        REPO = here
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

from _lib import findings_ledger as fl  # noqa: E402
from _lib import seam_forecast as sf  # noqa: E402
from _lib import seam_matrix as sm  # noqa: E402

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


def _w(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


# ═══════════════════════════════════════════════════════════════
# 1. FINGERPRINT STABILITY — the contract the ledger depends on
# ═══════════════════════════════════════════════════════════════
fp = sf.forecast_fingerprint
check("fingerprint is deterministic", fp("r", "S1", ["C4"]) == fp("r", "S1", ["C4"]))
check("fingerprint is class-ORDER independent (classes are sorted before hashing)",
      fp("r", "S1", ["C7", "C12"]) == fp("r", "S1", ["C12", "C7"]))
check("fingerprint changes when the class SET changes",
      fp("r", "S1", ["C7"]) != fp("r", "S1", ["C7", "C12"]))
check("fingerprint changes when the seam changes", fp("r", "S1", ["C4"]) != fp("r", "S2", ["C4"]))
check("fingerprint changes when the rung changes", fp("a", "S1", ["C4"]) != fp("b", "S1", ["C4"]))
check("fingerprint is 12 hex chars (the measure tier's width)",
      len(fp("r", "S1", ["C4"])) == 12 and all(c in "0123456789abcdef" for c in fp("r", "S1", ["C4"])))

_FORECAST = """\
seam-forecast-version: 1
rows:
  - fp: {fp1}
    seeded_rank: 1
    rung: fix-red-active
    seam: SX
    classes: [C2, C4]
    status: predicted
    title: fixture row one
    prediction: fixture
  - fp: {fp2}
    seeded_rank: 2
    rung: none
    seam: SY
    classes: [C5]
    status: confirmed-live
    title: fixture row two
    prediction: fixture
""".format(fp1=fp("fix-red-active", "SX", ["C2", "C4"]), fp2=fp("none", "SY", ["C5"]))

with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / sf.FORECAST_REL, _FORECAST)
    rows, errs = sf.load_forecast(root)
    check("load_forecast: clean file loads with zero errors", len(rows) == 2 and errs == [])
    check("load_forecast: rung defaults are preserved verbatim (`none` is a real value)",
          rows[1]["rung"] == "none")

    _w(root / sf.FORECAST_REL, _FORECAST.replace(fp("none", "SY", ["C5"]), "deadbeefcafe"))
    rows_bad, errs_bad = sf.load_forecast(root)
    check("load_forecast: a HAND-EDITED fingerprint fails LOUD",
          any("stored fp" in e for e in errs_bad))
    check("load_forecast: the COMPUTED fingerprint wins over the stored one",
          rows_bad[1]["fp"] == fp("none", "SY", ["C5"]))

    # Two rows with the SAME (rung, seam, classes) triple — differing only in prose, which the
    # fingerprint deliberately ignores. They are one forecast wearing two titles.
    _w(root / sf.FORECAST_REL, """\
seam-forecast-version: 1
rows:
  - seeded_rank: 1
    rung: fix-red-active
    seam: SX
    classes: [C2, C4]
    title: first phrasing
    prediction: fixture
  - seeded_rank: 2
    rung: fix-red-active
    seam: SX
    classes: [C4, C2]
    title: same triple, different words
    prediction: fixture
""")
    rows_dup, errs_dup = sf.load_forecast(root)
    check("load_forecast: a duplicated identity triple is reported, never silently merged",
          len(rows_dup) == 1
          and any("duplicates the identity triple" in e for e in errs_dup))

with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "empty"
    root.mkdir(parents=True)
    rows, errs = sf.load_forecast(root)
    check("load_forecast: a MISSING forecast file is an error, not an empty prediction set",
          rows == [] and errs and "ships pre-seeded" in errs[0])

# ═══════════════════════════════════════════════════════════════
# 2. CALIBRATION LEDGER — baseline, flips, outcomes, append-only
# ═══════════════════════════════════════════════════════════════
_SPINE_A = """\
version: 1
nodes:
  - id: fix-red-active
    status: red
    active: true
  - id: fix-other
    status: unwired
    active: false
"""
_SPINE_B = _SPINE_A.replace("id: fix-red-active\n    status: red",
                             "id: fix-red-active\n    status: green")
_SPINE_C = _SPINE_B.replace("id: fix-other\n    status: unwired",
                             "id: fix-other\n    status: red")

with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / sf.FORECAST_REL, _FORECAST)
    _w(root / sm.SPINE_REL, _SPINE_A)

    cal0 = sf.calibrate(root)
    check("calibrate: with no snapshot on file every node reads `first-run`",
          cal0["baseline_only"] and all(f["kind"] == "first-run" for f in cal0["flips"]))
    check("calibrate: a baseline proposes NO calibration rows (a baseline is not an event)",
          cal0["proposals"] == [])
    res0 = sf.record(root, cal0)
    check("record: the baseline writes only the spine snapshot",
          res0["written"] == 0 and res0["snapshot"] is True)

    cal_same = sf.calibrate(root)
    check("calibrate: an unchanged spine produces zero flips (safe on a loop)",
          cal_same["flips"] == [] and cal_same["proposals"] == [])
    res_same = sf.record(root, cal_same)
    check("record: an unchanged spine appends NOTHING at all", res_same["written"] == 0)
    n_after_baseline = len(fl.read_rows(root / sf.CALIBRATION_LEDGER_REL))

    # A forecast row names fix-red-active -> a HIT.
    _w(root / sm.SPINE_REL, _SPINE_B)
    cal1 = sf.calibrate(root)
    check("calibrate: a status flip on a FORECAST rung proposes `hit`",
          len(cal1["proposals"]) == 1 and cal1["proposals"][0]["outcome"] == "hit")
    check("calibrate: the hit carries the forecast row's fingerprint (the calibration link)",
          cal1["proposals"][0]["fp"] == fp("fix-red-active", "SX", ["C2", "C4"]))
    check("calibrate: the hit records what triggered it",
          "spine-flip:fix-red-active red->green" in cal1["proposals"][0]["trigger"])
    sf.record(root, cal1)

    # No forecast row names fix-other -> a MISS-OF-RANKING.
    _w(root / sm.SPINE_REL, _SPINE_C)
    cal2 = sf.calibrate(root)
    check("calibrate: a flip on an UNFORECAST rung proposes `miss-of-ranking`, never "
          "`miss-of-canon` (the canon is fine; the ordering was wrong)",
          len(cal2["proposals"]) == 1
          and cal2["proposals"][0]["outcome"] == "miss-of-ranking")
    sf.record(root, cal2)

    check("the ledger is APPEND-ONLY — every earlier row survives",
          len(fl.read_rows(root / sf.CALIBRATION_LEDGER_REL)) > n_after_baseline)
    summ = sf.ledger_summary(root)
    check("ledger_summary counts outcomes by class",
          summ["counts"]["hit"] == 1 and summ["counts"]["miss-of-ranking"] == 1
          and summ["counts"]["miss-of-canon"] == 0)
    check("ledger_summary reports a hit rate once there are outcomes",
          abs(summ["hit_rate"] - 0.5) < 1e-9)

    check("miss-of-canon REFUSES an empty note (a growth signal with no evidence is not one)",
          sf.record_miss_of_canon(root, note="   ") is False)
    check("miss-of-canon with evidence appends",
          sf.record_miss_of_canon(root, note="two dated instances at distinct seams: X, Y",
                                   seam="SX") is True)
    check("miss-of-canon is counted as canon-growth signal",
          sf.ledger_summary(root)["canon_growth_signals"] == 1)

# ═══════════════════════════════════════════════════════════════
# 3. LIVE, READ-ONLY — the seeded forecast's own integrity
# ═══════════════════════════════════════════════════════════════
live_rows, live_errs = sf.load_forecast(REPO)
check("LIVE: the seeded forecast loads with ZERO fingerprint mismatches",
      live_errs == [])
check("LIVE: the plan's eight ranked rows + two runners-up are all present",
      len(live_rows) == 10
      and len([r for r in live_rows if r.get("runner_up")]) == 2)
check("LIVE: rows 7 and 8 are transcribed as confirmed-live",
      sorted(r["seeded_rank"] for r in live_rows if r.get("status") == "confirmed-live") == [7, 8])
check("LIVE: every row's seam id is a real column in the seam catalog",
      {r["seam"] for r in live_rows}
      <= {s["id"] for s in sm.load_catalog(REPO)[0]["seams"]})
_live_spine, _ = sm.load_spine(REPO)
check("LIVE: every row's rung is either a real spine node or the explicit `none`",
      all(r["rung"] == "none" or r["rung"] in _live_spine for r in live_rows))
check("LIVE: every class named by a forecast row is a canon class",
      all(c in sm._sc.ALL_CONCERN_IDS for r in live_rows for c in r["classes"]))

live_ranked = sf.derived_rank(REPO, live_rows)
check("LIVE: derived_rank scores every row and assigns a dense 1..N ordering",
      sorted(r["derived_rank"] for r in live_ranked) == list(range(1, len(live_rows) + 1)))
check("LIVE: a confirmed-live row gets the zero-distance proximity floor (an instrument that "
      "buries its only confirmed findings is worse than no instrument)",
      all(r["score_parts"]["rung_proximity"] == 4.0
          for r in live_ranked if r.get("status") == "confirmed-live"))
check("LIVE: derived_rank annotates each row with its cells' CURRENT states",
      all(set(r["cell_states"]) == set(r["classes"]) for r in live_ranked))

_live_before = len(fl.read_rows(REPO / sf.CALIBRATION_LEDGER_REL))
sf.calibrate(REPO)
sf.render_forecast(REPO)
check("LIVE: calibrate/render are READ-ONLY — the real ledger is untouched",
      len(fl.read_rows(REPO / sf.CALIBRATION_LEDGER_REL)) == _live_before)

print(f"\n  {_passed} forecast assertions passed ✅")
