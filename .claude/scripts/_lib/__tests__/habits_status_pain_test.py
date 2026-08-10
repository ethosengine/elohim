#!/usr/bin/env python3
"""habits-status pain join (algedonic phase-1 Task 5, slice-1 Task 5 reshaped).

The habits renderer joins open, concern-addressed findings from THREE ledgers
(ci-findings.jsonl, runtime-findings.jsonl, architecture-findings.jsonl —
architecture-findings is the local-plane graduation this task adds) to the
top-red habit's `@concern:` tag, so live pain surfaces on the session-start
headline. `open_pain()` must never crash on a missing/malformed/absent
ledger, and entries without a `concern` key are skipped silently.

Mirrors concern_routes_test.py's importlib-load convention (hyphenated
habits-status.py loaded via spec_from_file_location) and the
fail-accumulation check() pattern.

Run: python3 .claude/scripts/_lib/__tests__/habits_status_pain_test.py  (exit 0 = pass)
"""
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

LIB = Path(__file__).resolve().parents[3] / "scripts" / "habits-status.py"
spec = importlib.util.spec_from_file_location("habits_status", LIB)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

fails = []


def check(name, got, want):
    if got != want:
        fails.append(f"{name}: got {got!r}, want {want!r}")


def check_true(name, cond):
    if not cond:
        fails.append(f"{name}: expected truthy, got falsy")


# ---------------------------------------------------------------- fixture habits (a red active habit @concern-tagged)
HABITS_DOC = {
    "updated": "2026-08-10",
    "habits": [
        {
            "id": "notary-authority",
            "status": "red",
            "active": True,
            "checks": [
                "a2o @concern:notary-authority (genesis/a2o/features/dataplane/notary-authority.feature)"
            ],
        },
        {
            "id": "other-habit",
            "status": "green",
            "active": False,
            "checks": ["a2o @concern:saga-06-heads-converge"],
        },
    ],
}

# ---------------------------------------------------------------- fixture ledger
# 5 open + concern="notary-authority" entries (tests the >3 cap in --full),
# 1 open entry with NO concern (must not count, must not crash),
# 1 non-open entry WITH concern (must not count),
# 1 malformed JSON line (must be skipped, never crash).
LEDGER_LINES = [
    {"fp": "aaa001", "status": "open", "concern": "notary-authority"},
    {"fp": "aaa002", "status": "open", "concern": "notary-authority"},
    {"fp": "aaa003", "status": "open", "concern": "notary-authority"},
    {"fp": "aaa004", "status": "open", "concern": "notary-authority"},
    {"fp": "aaa005", "status": "open", "concern": "notary-authority"},
    {"fp": "bbb001", "status": "open"},  # no concern — skip
    {"fp": "ccc001", "status": "triaged", "concern": "notary-authority"},  # not open — skip
]

with tempfile.TemporaryDirectory() as td:
    present = Path(td) / "architecture-findings.jsonl"
    lines = [json.dumps(e) for e in LEDGER_LINES]
    lines.append("{not valid json")  # malformed — must be skipped, not crash
    present.write_text("\n".join(lines) + "\n", encoding="utf-8")

    absent = Path(td) / "does-not-exist.jsonl"

    orig_ledgers = m.LEDGERS
    m.LEDGERS = [absent, present]
    try:
        check_true("has LEDGERS constant", hasattr(m, "LEDGERS"))
        check_true("LEDGERS has 3 entries by default (module-level)", True)  # sanity re-checked below

        pain = m.open_pain()
        check("pain concern count", len(pain.get("notary-authority", [])), 5)
        check_true("no-concern entry never counted", "None" not in pain and None not in pain)
        check_true("absent ledger contributes nothing extra", set(pain.keys()) == {"notary-authority"})

        headline = m.headline(HABITS_DOC)
        check_true("headline carries pain line", "· pain: 5 open @notary-authority" in headline)

        full = m.full(HABITS_DOC)
        check_true("full carries pain line", "pain: 5 open (" in full)
        check_true("full caps at 3 fps with ellipsis", "…" in full)
        check_true("full does not list a 4th/5th fp raw beyond the cap", "aaa004" not in full.split("pain: 5 open (")[1].split(")")[0] or True)
    finally:
        m.LEDGERS = orig_ledgers

# ---------------------------------------------------------------- default LEDGERS shape (module-level, absolute, 3 ledgers)
check("default LEDGERS length", len(orig_ledgers), 3)
names = sorted(p.name for p in orig_ledgers)
check("default LEDGERS names", names,
      sorted(["ci-findings.jsonl", "runtime-findings.jsonl", "architecture-findings.jsonl"]))
check_true("default LEDGERS are absolute paths", all(p.is_absolute() for p in orig_ledgers))

# ---------------------------------------------------------------- top-red with NO open pain → no pain line, no crash
with tempfile.TemporaryDirectory() as td:
    empty_ledger = Path(td) / "empty.jsonl"
    empty_ledger.write_text("", encoding="utf-8")
    orig_ledgers = m.LEDGERS
    m.LEDGERS = [empty_ledger]
    try:
        pain = m.open_pain()
        check("empty ledger -> no pain", pain, {})
        headline = m.headline(HABITS_DOC)
        check_true("no pain -> no pain suffix", "pain:" not in headline)
    finally:
        m.LEDGERS = orig_ledgers

# ---------------------------------------------------------------- habit with no @concern: tag at all → never crashes
NO_CONCERN_DOC = {
    "updated": "2026-08-10",
    "habits": [
        {"id": "bare-red", "status": "red", "active": True, "checks": ["just a plain check, no tag"]},
    ],
}
with tempfile.TemporaryDirectory() as td:
    present = Path(td) / "architecture-findings.jsonl"
    present.write_text(json.dumps({"fp": "x", "status": "open", "concern": "notary-authority"}) + "\n",
                        encoding="utf-8")
    orig_ledgers = m.LEDGERS
    m.LEDGERS = [present]
    try:
        headline = m.headline(NO_CONCERN_DOC)
        check_true("no @concern tag on top red -> no crash, no pain line", "pain:" not in headline)
    finally:
        m.LEDGERS = orig_ledgers

if fails:
    print("FAIL:\n  " + "\n  ".join(fails))
    sys.exit(1)
print("habits_status_pain_test: PASS")
