#!/usr/bin/env python3
"""Tests for concern_canon_projection — the SDK projection of the concern canon (plan P4.6).

Run: python3 .claude/scripts/_lib/__tests__/concern_canon_projection_test.py  (exit 0 = pass).
pytest is NOT installed — this is the bespoke harness, same shape as seam_census_test.py.

The freshness check is the point: `concern-classes.json` is GENERATED, so the only thing standing
between the published canon and a silent lie is a test that regenerates it and compares bytes.
Every negative case below is exercised against a COPIED registry tree, never the live one.
"""
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
_root = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        _root = here
        break
    here = here.parent
from _lib import concern_canon_projection as ccp  # noqa: E402
from _lib import seam_census as sc  # noqa: E402

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


def sandbox() -> Path:
    """A throwaway repo holding copies of the four files the projection spans."""
    d = Path(tempfile.mkdtemp())
    (d / ".claude" / "epr-meta").mkdir(parents=True)
    (d / "elohim" / "sdk" / "schemas" / "v1" / "registries").mkdir(parents=True)
    for rel in (ccp.POLICIES_REL, ccp.CONCERNS_REL, ccp.CLASSES_REL, ccp.CHANGELOG_REL):
        src = _root / rel
        if src.is_file():
            shutil.copy2(src, d / rel)
    return d


def read_json(root: Path, rel: str) -> dict:
    return json.loads((root / rel).read_text(encoding="utf-8"))


def write_json(root: Path, rel: str, doc: dict) -> None:
    (root / rel).write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


# ══════════════════════════════ the LIVE tree is fresh and coherent ══════════════════════════
_live_problems = ccp.check(_root)
check("live tree: projection is fresh and lineage is coherent (no problems)",
      _live_problems == [])

_rows, _errs = ccp.canon_rows(_root)
check("live tree: registries load without error", _errs == [])
check("live tree: 16 concern classes projected (C0-C14, C6 split)", len(_rows) == 16)
check("live tree: every ALL_CONCERN_IDS class is present exactly once",
      [r["concern"] for r in _rows] == list(sc.ALL_CONCERN_IDS))
check("live tree: every row carries id/statement/version/contentHash (the published contract)",
      all(r["id"] and r["statement"] and isinstance(r["version"], int)
          and r["contentHash"].startswith("sha256:") for r in _rows))
check("live tree: TIPS ONLY — the graduated classes project their @2 rows",
      {r["concern"]: r["version"] for r in _rows}["C2"] == 2
      and {r["concern"]: r["version"] for r in _rows}["C6a"] == 2)
check("live tree: C6a's tip names policies.yaml (it graduated ACROSS homes)",
      next(r for r in _rows if r["concern"] == "C6a")["home"] == "policies.yaml")
check("live tree: statements are single-line normalized (byte-comparable output)",
      all("\n" not in r["statement"] and "  " not in r["statement"] for r in _rows))

_edges, _lerrs = ccp.lineage_edges(_root)
check("live tree: lineage parses cleanly", _lerrs == [])
check("live tree: exactly the two phase-1 graduations are declared",
      {(e["policy_id"], e["from"], e["to"]) for e in _edges}
      == {("c2-monotonic-authority", 1, 2), ("c6a-bounded-work", 1, 2)})
check("live tree: C6a's edge records the cross-home move",
      next(e for e in _edges if e["policy_id"] == "c6a-bounded-work")["from_home"]
      == "concerns.yaml")

# ══════════════════════════════ determinism / idempotence ══════════════════════════════
_a, _ = ccp.render_classes(_root)
_b, _ = ccp.render_classes(_root)
check("render is deterministic (two runs, identical bytes)", _a == _b)
check("render matches the file on disk byte for byte",
      _a == (_root / ccp.CLASSES_REL).read_text(encoding="utf-8"))
check("render ends with exactly one trailing newline",
      _a.endswith("}\n") and not _a.endswith("\n\n"))

_sb = sandbox()
ccp.write(_sb)
_first = (_sb / ccp.CLASSES_REL).read_bytes()
ccp.write(_sb)
check("--write is idempotent", (_sb / ccp.CLASSES_REL).read_bytes() == _first)
check("--write leaves no .tmp behind",
      not (_sb / (ccp.CLASSES_REL + ".tmp")).exists())

# ══════════════════════════════ NEGATIVE: staleness is caught, loudly ══════════════════════
_sb = sandbox()
_doc = read_json(_sb, ccp.CLASSES_REL)
_doc["classes"][0]["statement"] = "a statement nobody canonized"
write_json(_sb, ccp.CLASSES_REL, _doc)
_probs = ccp.check(_sb)
check("a hand-edited projection is caught as STALE", any("STALE" in p for p in _probs))
check("the staleness message names the regeneration command",
      any(ccp.FIX_CMD in p for p in _probs))

_sb = sandbox()
os.remove(_sb / ccp.CLASSES_REL)
check("a MISSING projection is caught", any("MISSING" in p for p in ccp.check(_sb)))

# a canon row moving in the registry makes the projection stale (the real cascade trigger)
_sb = sandbox()
_yaml = (_sb / ccp.CONCERNS_REL).read_text()
_yaml = _yaml.replace("c14-witnessed-residual", "c14-witnessed-residual-renamed", 1)
(_sb / ccp.CONCERNS_REL).write_text(_yaml)
check("a registry edit makes the published projection stale",
      any("STALE" in p for p in ccp.check(_sb)))

# ══════════════ NEGATIVE: the changelog must match the registries BOTH ways (C7) ═══════════
_sb = sandbox()
_cl = read_json(_sb, ccp.CHANGELOG_REL)
_cl["changes"] = [c for c in _cl["changes"] if c["policy_id"] != "c6a-bounded-work"]
write_json(_sb, ccp.CHANGELOG_REL, _cl)
_probs = ccp.check(_sb)
check("an UNRECORDED graduation fails loud",
      any("c6a-bounded-work@1 -> @2" in p and "NO row" in p for p in _probs))
check("the message hands over the exact hashes to record",
      any("to_content_hash sha256:88b41ea4" in p for p in _probs))

_sb = sandbox()
_cl = read_json(_sb, ccp.CHANGELOG_REL)
_cl["changes"].append({"policy_id": "c9-identity-lineage-continuity", "from": 1, "to": 2,
                       "migration_note": "n", "affected_contract": "c"})
write_json(_sb, ccp.CHANGELOG_REL, _cl)
check("a changelog row naming a supersession that never happened fails loud",
      any("names a supersession the registries do not declare" in p for p in ccp.check(_sb)))

_sb = sandbox()
_cl = read_json(_sb, ccp.CHANGELOG_REL)
_cl["changes"][0]["to_content_hash"] = "sha256:" + "0" * 64
write_json(_sb, ccp.CHANGELOG_REL, _cl)
check("a STALE changelog hash fails loud (it points at bytes that no longer exist)",
      any("but the registry row hashes to" in p for p in ccp.check(_sb)))

for _field in ("migration_note", "affected_contract"):
    _sb = sandbox()
    _cl = read_json(_sb, ccp.CHANGELOG_REL)
    _cl["changes"][0][_field] = "   "
    write_json(_sb, ccp.CHANGELOG_REL, _cl)
    check(f"an empty `{_field}` fails loud (the row not existing, wearing a costume)",
          any(f"empty `{_field}`" in p for p in ccp.check(_sb)))

_sb = sandbox()
os.remove(_sb / ccp.CHANGELOG_REL)
_probs = ccp.check(_sb)
check("a MISSING changelog is caught and lists the edges to record",
      any("MISSING" in p and "c2-monotonic-authority" in p for p in _probs))

_sb = sandbox()
(_sb / ccp.CHANGELOG_REL).write_text("{not json", encoding="utf-8")
check("an unparseable changelog fails loud rather than reading as empty (C4)",
      any("unreadable/invalid JSON" in p for p in ccp.check(_sb)))

# ══════════════════════════════ the CLI contract ══════════════════════════════
import subprocess  # noqa: E402

_script = str(_root / ".claude" / "scripts" / "_lib" / "concern_canon_projection.py")
_r = subprocess.run(["python3", _script], capture_output=True, text=True, timeout=60)
check("default (check) mode exits 0 on a fresh tree", _r.returncode == 0)
check("check mode reports the class count and canon version",
      "16 classes" in _r.stdout and "canon version 1" in _r.stdout)
check("check mode names both homes", "both homes" in _r.stdout)

print(f"\n  {_p} assertions passed ✅")
