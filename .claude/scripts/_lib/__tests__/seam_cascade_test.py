"""seam_cascade.py test — the pin-keyed cascade (plan P4.2). Run:
python3 .claude/scripts/_lib/__tests__/seam_cascade_test.py  (exit 0 = pass)

Every fixture lives under a fresh tempfile.TemporaryDirectory() — the real
`.claude/epr-meta/*.yaml`, the real `.epr-meta` manifests and the real ledgers are NEVER written.
Two assertions DO read the live tree, deliberately and read-only: the live cascade must report
zero stale pins (phase 1 re-declared both storage bindings to their tips, and a regression there
is exactly what this instrument exists to catch), and the live ledger path must not be created as
a side effect of computing the cascade.
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
from _lib import seam_cascade as sc  # noqa: E402

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


def _w(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


POLICIES_HEAD = "epr-meta-policies-version: 1\npolicies:\n"
CONCERNS_HEAD = "concern-canon-version: 1\nconcerns:\n"


def _policy(pid, ver, *, concern=None, status="active", superseded_by=None, extra=""):
    """A minimal registry row that survives `load_policies` validation: class `inject` needs
    exactly one actionable predicate, so every fixture row carries `dedupe-of`."""
    s = (f"  - id: {pid}\n"
         f"    version: {ver}\n"
         f"    title: fixture {pid}@{ver}\n"
         f"    binding: binding-local\n"
         f"    class: inject\n"
         f"    scope: {{ write: \"*.rs\" }}\n"
         f"    dedupe-of: \"fixture\"\n"
         f"    dispatch-agent: rust-architect\n"
         f"    status: {status}\n"
         f"    why: fixture row\n")
    if concern:
        s += f"    concern: {concern}\n"
    if superseded_by:
        s += f"    superseded_by: {superseded_by}\n"
    return s + extra


def _concern_row(pid, ver, *, concern, status="active", superseded_by=None, extra=""):
    s = (f"  - id: {pid}\n"
         f"    version: {ver}\n"
         f"    concern: {concern}\n"
         f"    title: fixture concern {pid}@{ver}\n"
         f"    statement: fixture\n"
         f"    status: {status}\n"
         f"    dispatch-agent: general-purpose\n"
         f"    why: fixture\n")
    if superseded_by:
        s += f"    superseded_by: {superseded_by}\n"
    return s + extra


def _manifest(rule_id, ref, *, mid="fixture-gate"):
    return (f"---\n"
            f"epr-meta-version: 1\n"
            f"id: {mid}\n"
            f"covers: subtree\n"
            f"purpose: fixture\n"
            f"rules:\n"
            f"  - id: {rule_id}\n"
            f"    policy: {ref}\n"
            f"    when: {{ write: \"*.rs\" }}\n"
            f"    why: fixture binding\n"
            f"---\n\n# fixture\n")


# ═══════════════════════════════════════════════════════════════
# 1. STALE PIN — the core cascade behaviour
# ═══════════════════════════════════════════════════════════════
with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / ".claude/epr-meta/policies.yaml",
       POLICIES_HEAD
       + _policy("p-alpha", 1, concern="C2", status="superseded", superseded_by="p-alpha@2")
       + _policy("p-alpha", 2, concern="C2"))
    _w(root / "crate/.epr-meta", _manifest("bind-alpha", "p-alpha@1"))

    d = sc.cascade_data(root)
    check("stale pin: one binding at @1 with tip @2 yields exactly one stale pin",
          len(d["stale_pins"]) == 1 and d["n_bindings"] == 1)
    sp = d["stale_pins"][0]
    check("stale pin: names the from/to refs and the binding site",
          sp["from"] == "p-alpha@1" and sp["to"] == "p-alpha@2"
          and sp["site"] == "crate/.epr-meta#bind-alpha")
    check("stale pin: carries the tip row's concern class", sp["concern"] == "C2")
    check("stale pin: emits exactly one finding, kind cascade-stale-pin",
          len(d["findings"]) == 1 and d["findings"][0]["rule"] == "cascade-stale-pin")
    check("stale pin: finding carries the tip policy's dispatch-agent",
          d["findings"][0]["dispatch_agent"] == "rust-architect")
    fp_first = sp["fp"]

    # Fingerprint stability: re-running changes nothing.
    d2 = sc.cascade_data(root)
    check("fingerprint is stable across runs", d2["stale_pins"][0]["fp"] == fp_first)

    # Fingerprint EXCLUDES prose: rewriting the policy's `why` must not re-mint the finding.
    _w(root / ".claude/epr-meta/policies.yaml",
       (POLICIES_HEAD
        + _policy("p-alpha", 1, concern="C2", status="superseded", superseded_by="p-alpha@2")
        + _policy("p-alpha", 2, concern="C2")).replace("why: fixture row",
                                                        "why: a completely different rationale"))
    d3 = sc.cascade_data(root)
    check("fingerprint excludes policy prose — rewording `why` does not re-mint the finding",
          d3["stale_pins"][0]["fp"] == fp_first)

    # Re-declaring the binding drains the finding.
    _w(root / "crate/.epr-meta", _manifest("bind-alpha", "p-alpha@2"))
    d4 = sc.cascade_data(root)
    check("re-declaring the binding to the tip clears the stale pin",
          d4["stale_pins"] == [] and d4["findings"] == [])

# ═══════════════════════════════════════════════════════════════
# 2. CROSS-HOME lineage (the C6a graduation shape)
# ═══════════════════════════════════════════════════════════════
with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / ".claude/epr-meta/concerns.yaml",
       CONCERNS_HEAD + _concern_row("c-grad", 1, concern="C6a", status="superseded",
                                     superseded_by="policies.yaml#c-grad@2"))
    _w(root / ".claude/epr-meta/policies.yaml",
       POLICIES_HEAD + _policy("c-grad", 2, concern="C6a"))
    _w(root / "crate/.epr-meta", _manifest("bind-grad", "c-grad@1"))

    d = sc.cascade_data(root)
    check("cross-home lineage: a concerns.yaml row superseded by a policies.yaml row resolves",
          len(d["stale_pins"]) == 1 and d["stale_pins"][0]["to"] == "c-grad@2")
    check("cross-home lineage: the finding detail names both homes",
          "concerns.yaml" in d["findings"][0]["detail"]
          and "policies.yaml" in d["findings"][0]["detail"])

# ═══════════════════════════════════════════════════════════════
# 3. FAIL-LOUD paths — unknown pin, broken lineage, cycle
# ═══════════════════════════════════════════════════════════════
with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / ".claude/epr-meta/policies.yaml", POLICIES_HEAD + _policy("p-known", 1))
    _w(root / "crate/.epr-meta", _manifest("bind-ghost", "p-ghost@1"))
    d = sc.cascade_data(root)
    check("unknown pin fails LOUD as its own finding, never scored 'not stale'",
          len(d["findings"]) == 1
          and d["findings"][0]["rule"] == "cascade-binding-unknown-policy")

with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / ".claude/epr-meta/policies.yaml",
       POLICIES_HEAD + _policy("p-broken", 1, status="superseded"))
    _w(root / "crate/.epr-meta", _manifest("bind-broken", "p-broken@1"))
    d = sc.cascade_data(root)
    check("a row marked superseded with no parseable superseded_by reports broken lineage",
          any("lineage is broken" in e for e in d["errors"]))

with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / ".claude/epr-meta/policies.yaml",
       POLICIES_HEAD
       + _policy("p-cyc", 1, status="superseded", superseded_by="p-cyc@2")
       + _policy("p-cyc", 2, status="superseded", superseded_by="p-cyc@1"))
    _w(root / "crate/.epr-meta", _manifest("bind-cyc", "p-cyc@1"))
    d = sc.cascade_data(root)
    check("a supersession CYCLE is reported, never spun on",
          any("cycle" in e for e in d["errors"]))

# ═══════════════════════════════════════════════════════════════
# 4. GRADUATION FAN-OUT (C13 scaffold -> successor)
# ═══════════════════════════════════════════════════════════════
with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / ".claude/epr-meta/policies.yaml",
       POLICIES_HEAD
       + _policy("p-c13", 1, concern="C13", status="superseded", superseded_by="p-c13@2",
                  extra="    scaffold: god-mode canonical-head declarer\n")
       + _policy("p-c13", 2, concern="C13",
                  extra="    scaffold: god-mode canonical-head declarer\n"
                        "    successor: reach-cohort arbitration (R2)\n"
                        "    graduation_trigger: earned-tier signature set lands\n"))
    _w(root / "crate/.epr-meta", _manifest("bind-c13", "p-c13@2"))

    d_nomatrix = sc.cascade_data(root)
    check("graduation: a successor field landing on the new version IS a graduation event",
          len(d_nomatrix["graduations"]) == 1
          and set(d_nomatrix["graduations"][0]["changed_fields"]) == {"successor",
                                                                       "graduation_trigger"})
    check("graduation: without a matrix the fan-out is empty AND says so (never a silent zero)",
          d_nomatrix["graduation_fanout"] == []
          and any("no matrix was supplied" in n for n in d_nomatrix["notes"]))

    fake_matrix = {"cells": [
        {"concern": "C13", "seam": "S3.10", "state": "variant"},
        {"concern": "C13", "seam": "S3.9", "state": "conformant"},
        {"concern": "C13", "seam": "S3.6", "state": "unexamined"},
        {"concern": "C2", "seam": "S3.10", "state": "conformant"},
    ]}
    d_matrix = sc.cascade_data(root, matrix=fake_matrix)
    cells = sorted(f["cell"] for f in d_matrix["graduation_fanout"])
    check("graduation fan-out fires over every EXAMINED cell citing the class",
          cells == ["C13xS3.10", "C13xS3.9"])
    check("graduation fan-out skips unexamined cells (no examination to invalidate) and "
          "other classes",
          "C13xS3.6" not in cells and "C2xS3.10" not in cells)
    check("graduation fan-out emits one finding per cell, kind cascade-graduation-reexamine",
          len([f for f in d_matrix["findings"]
               if f["rule"] == "cascade-graduation-reexamine"]) == 2)
    fps = {f["fp"] for f in d_matrix["graduation_fanout"]}
    check("graduation fingerprints are distinct per cell and stable across runs",
          len(fps) == 2
          and fps == {f["fp"] for f in sc.cascade_data(root, matrix=fake_matrix)
                      ["graduation_fanout"]})

    empty_matrix = {"cells": [{"concern": "C2", "seam": "S3.10", "state": "conformant"}]}
    d_zero = sc.cascade_data(root, matrix=empty_matrix)
    check("graduation over ZERO citing cells says so rather than reporting silence",
          d_zero["graduation_fanout"] == []
          and any("ZERO cells" in n for n in d_zero["notes"]))

# ═══════════════════════════════════════════════════════════════
# 5. LEDGER discipline — file once, drain when cured
# ═══════════════════════════════════════════════════════════════
with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "repo"
    _w(root / ".claude/epr-meta/policies.yaml",
       POLICIES_HEAD
       + _policy("p-led", 1, concern="C2", status="superseded", superseded_by="p-led@2")
       + _policy("p-led", 2, concern="C2"))
    _w(root / "crate/.epr-meta", _manifest("bind-led", "p-led@1"))

    d = sc.cascade_data(root)
    r1 = sc.file_findings(root, d)
    check("filing a new finding writes exactly one ledger row",
          len(r1["new"]) == 1 and len(r1["already"]) == 0)
    check("filing emits a dispatch directive naming the agent and the ledger",
          any("rust-architect" in t and sc.CASCADE_LEDGER_REL in t for t in r1["dispatches"]))
    r2 = sc.file_findings(root, sc.cascade_data(root))
    check("re-filing the same live finding files nothing (stable fp + read-check-append)",
          len(r2["new"]) == 0 and len(r2["already"]) == 1)
    ledger = root / sc.CASCADE_LEDGER_REL
    check("the ledger holds ONE line per live fingerprint",
          len(fl.read_rows(ledger)) == 1)
    check("the ledger row mirrors the measure tier's shape field-for-field",
          set(fl.read_rows(ledger)[0]) >= {"fp", "rule", "policy", "path", "detail",
                                            "first_seen", "status"})

    _w(root / "crate/.epr-meta", _manifest("bind-led", "p-led@2"))
    r3 = sc.file_findings(root, sc.cascade_data(root))
    check("curing the stale pin DRAINS the row rather than parking it as closed",
          r3["drained"] == 1 and fl.read_rows(ledger) == [])

# ═══════════════════════════════════════════════════════════════
# 6. LIVE tree — the phase-1 state this instrument must keep true
# ═══════════════════════════════════════════════════════════════
_ledger_lines_before = len(fl.read_rows(REPO / sc.CASCADE_LEDGER_REL))
live = sc.cascade_data(REPO)
check("LIVE: every .epr-meta policy binding in the repo is at its lineage tip (zero stale pins)",
      live["stale_pins"] == [])
check("LIVE: the cascade found the repo's real bindings (>= 9) and no registry errors",
      live["n_bindings"] >= 9 and live["errors"] == [])
check("LIVE: computing the cascade writes NOTHING to the ledger (read-only by construction)",
      len(fl.read_rows(REPO / sc.CASCADE_LEDGER_REL)) == _ledger_lines_before)
check("LIVE: c2-monotonic-authority@1 still resolves forward to @2 (lineage intact)",
      sc.resolve_tip(sc.canon_rows(REPO)[0], "c2-monotonic-authority@1")[0]
      == "c2-monotonic-authority@2")
check("LIVE: the cross-home C6a graduation resolves concerns.yaml@1 -> policies.yaml@2",
      sc.resolve_tip(sc.canon_rows(REPO)[0], "c6a-bounded-work@1")[0] == "c6a-bounded-work@2")

print(f"\n  {_passed} cascade assertions passed ✅")
