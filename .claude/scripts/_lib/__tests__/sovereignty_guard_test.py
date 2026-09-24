#!/usr/bin/env python3
"""Harness for the sovereignty-ontology guard (`epr:validator-sovereignty-ontology-guard`).

The mirror of `ownership_guard_test.py`. It locks the contract that is easy to silently regress:
  1. apex sovereignty framing (self-sovereignty as the protocol's own top value, "true/full data
     sovereignty" as a goal) fires, and the sovereignty that canon keeps legitimate does not;
  2. the frame marker: a declared legitimate frame is silent, a declared `apex` is drift;
  3. net-new only, `is_new` handling, and fail-toward-surfacing on an unreadable prior file;
  4. the verdict names its content-addressed frame atom (`frame-sovereignty-apex`, read through
     `_lib/frame_atoms.py`), carries no class of its own, and mints no classification CID (only
     the native host does, ruling R-C4 amended).

Run: python3 .claude/scripts/_lib/__tests__/sovereignty_guard_test.py   (no pytest in this repo)
"""
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from _lib import epr_meta as em  # noqa: E402

GUARD = em.REFERENCE_VALIDATORS["epr:validator-sovereignty-ontology-guard"]
checks = 0
failures = []


def check(name, got, expected):
    global checks
    checks += 1
    if got != expected:
        failures.append(f"{name}: got {got!r}, expected {expected!r}")


def verdict(content, *, is_new=True, path="does-not-exist.md"):
    return GUARD({"content": content, "is_new": is_new, "path": path})


def fires(content, **kw):
    return verdict(content, **kw) is not None


# ── 1. Apex framing fires on a new file ────────────────────────────────────────────────────────
check("self-sovereign fires", fires("Every member is self-sovereign."), True)
check("true data sovereignty fires", fires("We deliver true data sovereignty."), True)
check("sovereign identity fires", fires("A sovereign identity for everyone."), True)
check("digital sovereignty fires", fires("Digital sovereignty is the goal."), True)
check("fully sovereign fires", fires("Each node is fully sovereign."), True)

# ── 2. Sovereignty that is not the apex assertion stays silent ─────────────────────────────────
check("bare sovereign quiet", fires("Nation-states are near-sovereign yet bounded."), False)
check("stewardship prose quiet", fires("Stewardship, not sovereignty, is the frame."), False)
check("sovereign within bounds quiet", fires("The floor is sovereign within higher bounds."), False)
check("homoglyph evasion still fires", fires("self‑sоvereign"), True)

# ── 3. The frame marker: legitimate is silent, apex is drift ───────────────────────────────────
check("legitimate adversary frame silences",
      fires("sovereignty-frame: adversary\nCrypto promises true data sovereignty."), False)
check("legitimate bridge-legibility frame silences",
      fires("sovereignty-frame: bridge-legibility\nA self-sovereign wallet for the credit union."),
      False)
check("declared apex is drift",
      verdict("sovereignty-frame: apex\nWe are self-sovereign.").evidence["verdict"], "drift")

# ── 4. Net-new only — cleaning/maintenance never fires ──────────────────────────────────────────
with tempfile.TemporaryDirectory() as td:
    prior = Path(td) / "doc.md"
    prior.write_text("Crypto promises self-sovereign identity and digital sovereignty.")

    check("unchanged count quiet",
          fires("Crypto promises self-sovereign identity and digital sovereignty.",
                is_new=False, path=str(prior)), False)
    check("removing apex framing quiet",
          fires("Crypto promises stewardship.", is_new=False, path=str(prior)), False)
    check("adding one more fires",
          fires("Crypto promises self-sovereign identity and digital sovereignty.\n"
                "Each node is fully sovereign.", is_new=False, path=str(prior)), True)

# ── 5. Unreadable prior state fails toward surfacing (the guard is advisory) ────────────────────
check("unreadable prior surfaces",
      fires("fully sovereign", is_new=False, path="/nonexistent/dir/missing.md"), True)

# ── 5b. A fired verdict names its content-addressed frame; the classification CID is native-only ─
_v = verdict("Every member is self-sovereign.")
check("fired verdict carries evidence", isinstance(_v.evidence, dict), True)
check("evidence frameRef is a CID", str(_v.evidence.get("frameRef", "")).startswith("bafy"), True)
check("evidence classificationCid is None (Python mints none)", _v.evidence["classificationCid"], None)
check("verdict carries no class of its own (inherits the rule's)", _v.cls, None)
check("evidence verdict is abstain (hits, no marker)", _v.evidence["verdict"], "abstain")
check("evidence span indexes the phrase", _v.evidence["spans"], [{"start": 16, "end": 30}])
check("reason leads with the sovereignty clause",
      _v.reason.startswith("net-new apex-sovereignty framing needs an explicit bounded frame "
                           "· frame bafyrei"), True)

# ── 6. Registry wiring + policy binding ────────────────────────────────────────────────────────
check("validator registered",
      "epr:validator-sovereignty-ontology-guard" in em.REFERENCE_VALIDATORS, True)
_pols, _errs = em.load_policies(Path(__file__).resolve().parents[4])  # repo root
check("policies load clean", _errs, [])
check("policy row present", "sovereignty-ontology-guard@1" in _pols, True)

if failures:
    print(f"\n{len(failures)} FAILURE(S) of {checks} checks:")
    for f in failures:
        print(f"  ✗ {f}")
    sys.exit(1)
print(f"\n{checks} checks passed ✅")
