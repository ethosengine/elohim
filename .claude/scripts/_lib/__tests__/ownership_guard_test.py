#!/usr/bin/env python3
"""Harness for the ownership-ontology guard (`epr:validator-ownership-ontology-guard`).

Locks the two decisions that are easy to silently regress:
  1. the PROPERTY-vs-RESPONSIBILITY precision line in `_OWN_APEX_PHRASES` — English overloads
     "ownership", and the accountability idiom ("take full ownership of this bug", "the full
     ownership matrix") must never fire an enclosure guard; and
  2. the contract mirrored from `_sovereignty_ontology_guard` — net-new-only, frame-marker escape
     (either marker), `is_new` handling, fail-toward-surfacing on an unreadable prior file.

Run: python3 .claude/scripts/_lib/__tests__/ownership_guard_test.py   (no pytest in this repo)
"""
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from _lib import epr_meta as em  # noqa: E402

GUARD = em.REFERENCE_VALIDATORS["epr:validator-ownership-ontology-guard"]
checks = 0
failures = []


def check(name, got, expected):
    global checks
    checks += 1
    if got != expected:
        failures.append(f"{name}: got {got!r}, expected {expected!r}")


def fires(content, *, is_new=True, path="does-not-exist.md"):
    return GUARD({"content": content, "is_new": is_new, "path": path})


# ── 1. Apex framing fires on a new file ────────────────────────────────────────────────────────
check("data ownership fires", fires("Members get true data ownership."), True)
check("own your data fires", fires("Own your data, forever."), True)
check("ownership rights fires", fires("The platform grants ownership rights to each member."), True)
check("owns the commons fires", fires("Whoever owns the commons decides."), True)
check("outright ownership fires", fires("Contributors receive outright ownership."), True)

# ── 2. The RESPONSIBILITY idiom must stay silent (the audit-found false positives) ──────────────
check("full ownership matrix quiet", fires("See LIFECYCLE.md for the full ownership matrix."), False)
check("take full ownership quiet", fires("Please take full ownership of this bug."), False)
check("sole ownership burden quiet", fires("Runs without sole ownership burden."), False)
check("bare owner quiet", fires("The owner of the file is the steward."), False)
check("bare own quiet", fires("You own a bicycle and it has its own wheels."), False)

# ── 3. Frame markers silence the guard (both travel together) ───────────────────────────────────
check("stewardship-frame silences",
      fires("stewardship-frame: adversary\nPlatforms promise true data ownership."), False)
check("sovereignty-frame silences",
      fires("sovereignty-frame: adversary\nPlatforms promise true data ownership."), False)

# ── 4. Net-new only — cleaning/maintenance never fires ──────────────────────────────────────────
with tempfile.TemporaryDirectory() as td:
    prior = Path(td) / "doc.md"
    prior.write_text("Platforms promise true data ownership and ownership rights.")

    check("unchanged count quiet",
          fires("Platforms promise true data ownership and ownership rights.",
                is_new=False, path=str(prior)), False)
    check("removing apex framing quiet",
          fires("Platforms promise stewardship.", is_new=False, path=str(prior)), False)
    check("adding one more fires",
          fires("Platforms promise true data ownership, ownership rights, and outright ownership.",
                is_new=False, path=str(prior)), True)

# ── 5. Unreadable prior state fails toward surfacing (it is only an `ask`) ──────────────────────
check("unreadable prior surfaces",
      fires("true data ownership", is_new=False, path="/nonexistent/dir/missing.md"), True)

# ── 6. Registry wiring + policy binding ────────────────────────────────────────────────────────
check("validator registered", "epr:validator-ownership-ontology-guard" in em.REFERENCE_VALIDATORS, True)
_pols, _errs = em.load_policies(Path(__file__).resolve().parents[4])  # repo root
check("policies load clean", _errs, [])
check("policy row present", "ownership-ontology-guard@1" in _pols, True)

if failures:
    print(f"\n{len(failures)} FAILURE(S) of {checks} checks:")
    for f in failures:
        print(f"  ✗ {f}")
    sys.exit(1)
print(f"\n{checks} checks passed ✅")
