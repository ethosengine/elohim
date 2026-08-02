"""findings_ledger.py test — the shared fingerprinted-ledger discipline (plan P4.2/P4.4). Run:
python3 .claude/scripts/_lib/__tests__/findings_ledger_test.py  (exit 0 = pass)

This module is the extraction of the measure tier's inline protocol
(`.claude/hooks/epr-meta-resolver.py::_handle_measures`), so the tests pin the properties that
protocol depends on — one line per live fingerprint, corrupt lines survivable, the two ledger
modes kept distinct, and the debounce store shared with the resolver hook rather than forked.
All fixtures are temporary; no real ledger is written.
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

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


with tempfile.TemporaryDirectory() as td:
    root = Path(td)
    led = root / "ledger.jsonl"

    check("read_rows on a missing file is [] (never an exception)", fl.read_rows(led) == [])
    check("live_fingerprints on a missing file is empty", fl.live_fingerprints(led) == set())

    check("append_if_new files a new fingerprint", fl.append_if_new(led, {"fp": "aaa", "n": 1}) is True)
    check("append_if_new refuses to double-file the same fingerprint",
          fl.append_if_new(led, {"fp": "aaa", "n": 2}) is False)
    check("the second attempt did NOT rewrite the first row's payload",
          fl.read_rows(led)[0]["n"] == 1)
    check("append_if_new rejects an entry with no fingerprint",
          fl.append_if_new(led, {"no_fp": True}) is False)
    fl.append_if_new(led, {"fp": "bbb"})
    check("live_fingerprints reflects every filed row",
          fl.live_fingerprints(led) == {"aaa", "bbb"})

    # A corrupt line must not kill the reader: a ledger that refuses to load is a dead
    # instrument, and a dead instrument reads as "nothing to report".
    with open(led, "a") as fh:
        fh.write("{this is not json\n")
    check("a corrupt line is skipped, not fatal", len(fl.read_rows(led)) == 2)
    check("append_if_new still works past a corrupt line",
          fl.append_if_new(led, {"fp": "ccc"}) is True)

    check("drain removes rows whose fingerprint is no longer live",
          fl.drain(led, {"aaa"}) == 2 and fl.live_fingerprints(led) == {"aaa"})
    check("drain is a no-op when every row is still live", fl.drain(led, {"aaa"}) == 0)

    # APPEND-ONLY mode: no dedupe, no drain semantics — every call is a historical fact.
    hist = root / "history.jsonl"
    fl.append(hist, {"fp": "same", "v": 1})
    fl.append(hist, {"fp": "same", "v": 2})
    check("append (append-only mode) does NOT dedupe — a repeated event is two facts",
          len(fl.read_rows(hist)) == 2)

    check("utc_now is a Z-suffixed UTC stamp",
          fl.utc_now().endswith("Z") and len(fl.utc_now()) == 20)

    # Debounce shares the resolver hook's advice store, under a caller-chosen key prefix.
    check("first advice of a key is not debounced", fl.debounced(root, "test-fp:1") is False)
    check("a repeat within the window IS debounced", fl.debounced(root, "test-fp:1") is True)
    check("a different key is independent", fl.debounced(root, "test-fp:2") is False)
    check("the debounce store is the SAME file the resolver hook uses (never a fork)",
          (root / fl.ADVICE_STORE_REL).is_file())

print(f"\n  {_passed} findings-ledger assertions passed ✅")
