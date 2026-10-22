"""habits-status RED MASS — the progress grain under red/green
(spec genesis/docs/superpowers/specs/2026-10-22-habit-progress-scalar-design.md §6).

The scalar is computed natively (`epr flow stocks --stock red-mass --json`); this script only
renders it. So the load-bearing assertions are about the rendering's honesty: a payload renders
its numbers, an absent reading renders `not read` (never zero), a reader that raises cannot
break the headline, and the census refuses an `@concern:` tag smuggled into `rea:` — the one
place this layer could otherwise mint a CI route nobody declared.

Every payload here is a FIXTURE; nothing reads the tree's history. Runs standalone —
`python3 <this file>`, exit 0 = pass — like its siblings.
"""
import importlib.util
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("habits_status_rm", SCRIPTS / "habits-status.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
sys.path.insert(0, str(SCRIPTS))
from _lib import epr_habits as eh  # noqa: E402

fails = []


def check_true(name, cond):
    if not cond:
        fails.append(name)


def row(hid, status, floor, over=None, measured=0, declared=2, spent=0.0, returns=0.0, payback=None):
    return {
        "id": hid,
        "state": {"status": status, "composed": {"floor": floor, "overMeasured": over,
                                                 "measured": measured, "declared": declared},
                  "returnsHoursPerWeek": returns},
        "spent": {"allTime": spent},
        "paybackWeeks": payback,
    }


PAYLOAD = {
    "stock": "red-mass",
    "verdict": {"verdict": "draining"},
    "measures": {"level": 31.4, "inflow": 0.4, "outflow": 1.2},
    "habits": {
        "habits": 48, "nonGreen": 34, "redMass": 31.4, "hoursSpentInWindow": 6.5,
        "hoursReturnedPerWeek": 0.37, "hoursReturnedClaim": "estimated",
        "rows": [
            row("push-delivers-within-budget", "red", 0.38, over=0.75, measured=2, declared=4,
                spent=6.5, returns=0.37, payback=17.6),
            row("idle-is-free", "red", 0.0),
            row("notary-authority", "green", 1.0, over=1.0, measured=1, declared=1),
        ],
    },
}

line = m.red_mass_line(PAYLOAD)
check_true("renders the red mass and its denominator", "red mass 31.40 of 34 not green" in line)
check_true("renders the window's drain and fill", "drained 1.20" in line and "filled 0.40" in line)
check_true("counts reds partly won, not greens", "1 partly won" in line)
check_true("renders hours spent and returned with the claim",
           "6.5 h spent" in line and "0.37 h/wk returned (estimated)" in line)
check_true("renders the verdict", "(7d: DRAINING)" in line)

absent = m.red_mass_line(None)
check_true("an absent reading says not read, never zero", "not read" in absent and "0.00" not in absent)
check_true("a malformed payload says not read", "not read" in m.red_mass_line({"stock": "red-mass"}))

made = m.progress_part(PAYLOAD["habits"]["rows"][0])
check_true("progress names the floor, the measured value and the coverage",
           made.startswith("progress: 0.38 (0.75 on 2/4 measured)"))
check_true("progress carries spent, returned and payback",
           "spent 6.5 h" in made and "returns 0.37 h/wk" in made and "payback 17.6 wk" in made)
check_true("an unmeasured habit says how many parts it declares",
           m.progress_part(PAYLOAD["habits"]["rows"][1]) == "progress: 0.00 (0/2 measured)")
check_true("no row, no progress clause", m.progress_part(None) == "")

# The headline survives a reader that raises — session start must never break on this.
doc = {"habits": [{"id": "h", "status": "red", "checks": ["x"], "invariant": "i"}]}
orig = (m.READ_RED_MASS, m.not_measured_view, m.open_pain, m.habit_cost)
try:
    m.not_measured_view = lambda _d, **_k: None
    m.open_pain = lambda: {}
    m.habit_cost = lambda *_a, **_k: None

    def boom():
        raise RuntimeError("reader exploded")

    m.READ_RED_MASS = boom
    check_true("a raising reader renders not read", "red mass: not read" in m.headline(doc))
    m.READ_RED_MASS = lambda: PAYLOAD
    check_true("the headline carries the red-mass line under the counts",
               m.headline(doc).splitlines()[1].startswith("  red mass 31.40"))
finally:
    m.READ_RED_MASS, m.not_measured_view, m.open_pain, m.habit_cost = orig

# The census refuses a route smuggled into `rea:`.
base = {"epr-habit-version": 1, "id": "h", "invariant": "i", "status": "red",
        "checks": ["a2o @concern:real"], "retire-when": "when superseded"}
check_true("a clean rea block validates",
           eh.validate({**base, "rea": {"parts": [{"part": "real", "reading": "1/2"}]}}, "h") == [])
errs = eh.validate({**base, "rea": {"parts": [{"part": "x", "evidence": "@concern:wrong"}]}}, "h")
check_true("an @concern tag inside rea is refused", any("@concern:" in e for e in errs))
check_true("rea must be a mapping", eh.validate({**base, "rea": ["x"]}, "h") != [])

if fails:
    print("FAIL")
    for f in fails:
        print("  -", f)
    sys.exit(1)
print("ok — habits-status red mass + census rea guard")
