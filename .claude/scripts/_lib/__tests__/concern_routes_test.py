#!/usr/bin/env python3
"""concern_routes: findings get the @concern address of the promise they threaten."""
import importlib.util, sys
from pathlib import Path

LIB = Path(__file__).resolve().parents[1] / "concern_routes.py"
spec = importlib.util.spec_from_file_location("concern_routes", LIB)
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)

HABITS_FIXTURE = """
habits:
  - id: notary-authority
    status: red
    active: true
    checks:
      - "a2o @concern:notary-authority (genesis/a2o/features/dataplane/notary-authority.feature)"
  - id: other-habit
    status: green
    active: false
    checks:
      - "a2o @concern:saga-06-heads-converge"
"""

fails = []
def check(name, got, want):
    if got != want: fails.append(f"{name}: got {got!r}, want {want!r}")

# active_concern: first @concern tag of the first active habit
check("active", m.active_concern(HABITS_FIXTURE), "notary-authority")
check("active-empty", m.active_concern("habits: []"), None)

# route: no-measure class resolves to the active habit's concern via context
check("no-measure", m.route("ci-no-measure", {"active_concern": "notary-authority"}), "notary-authority")
# route: explicit concern in context wins
check("explicit", m.route("ci-failure", {"concern": "saga-04-doorway-serves"}), "saga-04-doorway-serves")
# route: unknown class, no context → None (honest absence)
check("none", m.route("ci-failure", {}), None)

if fails:
    print("FAIL:\n  " + "\n  ".join(fails)); sys.exit(1)
print("concern_routes_test: PASS")
