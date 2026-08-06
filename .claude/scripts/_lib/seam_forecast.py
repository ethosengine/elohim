"""seam_forecast.py — the seeded forecast + the CALIBRATION LEDGER (plan P4.4, surface 6).

THE PAIR. The forecast (`.claude/epr-meta/seam-forecast.yaml`) is a set of FINGERPRINTED
predictions: rung x seam x concern classes x predicted collision. The calibration ledger
(`.claude/data/seam-calibration.jsonl`) is the append-only record of how those predictions fared,
so two things stop being opinions and become measurements: (a) whether forecasting works at all,
and (b) THE CANON'S OWN GROWTH RATE — every `miss-of-canon` is the evidence a new class must
present, and the birth rule is tunable only because this ledger counts them.

THREE OUTCOMES, and the distinction between the last two is the whole instrument:
  hit             — a forecast row named the rung AND the class set that actually collided.
  miss-of-ranking — a KNOWN class collided at an unforecast seam/rung. The canon was fine; the
                    ordering was wrong. This tunes the ranking function, not the canon.
  miss-of-canon   — the collision fit NO existing class. This is the canon's only admission
                    intake, and it is NEVER auto-proposed: a new class costs |seams| new matrix
                    cells and |concerns| new birth-rule questions, so it is a budgeted, deliberate
                    act by a human or an agent that argues for it. The tooling records the claim
                    with its evidence; it does not mint the class.

INTAKE PATH — WHO APPENDS, CONCRETELY (this must be runnable, never aspirational):
  1. `python3 .claude/scripts/seam-audit.py --calibrate`
       Read-only. Compares `genesis/manifests/habits.yaml` against the LAST `habits-snapshot` row in
       the ledger and prints every flip (a node whose status changed, or a node that appeared),
       each with its proposed classification against the forecast. Prints nothing to the ledger.
  2. `python3 .claude/scripts/seam-audit.py --calibrate --record`
       Appends one `calibration` row per flip with the proposed outcome, then one fresh
       `habits-snapshot` . Idempotent in the only sense that matters: with no flips it appends
       nothing at all (an unchanged habits is not an event), so it is safe on a `/loop`.
  3. `python3 .claude/scripts/seam-audit.py --record-miss-of-canon --note "<evidence>"`
       The canon-growth intake. Deliberately a separate, explicit verb.
The natural caller of (1)/(2) is whoever reads a habits flip or a new red — the delivery-stasis
loop, a shift close, or the operator. The ledger is APPEND-ONLY: rows are never drained or
mutated, because a hit/miss is a historical fact and a "live set" of them would destroy the very
growth-rate measure the ledger exists to compute (see `_lib.findings_ledger`'s two modes).

FINGERPRINT STABILITY IS A CONTRACT. fp = sha256("<rung>|<seam>|<classes lexicographically
sorted, comma-joined>")[:12]. Prose is excluded so re-wording a prediction never re-mints its row
and never orphans its ledger entries. `load_forecast` RECOMPUTES every fingerprint and reports a
mismatch loud — a hand-edited fp is a silently-broken calibration link, which is the one failure
this instrument cannot self-detect later.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

from _lib import findings_ledger as _fl
from _lib import seam_matrix as _sm

try:
    import yaml
except Exception:  # pragma: no cover
    yaml = None

FORECAST_REL = ".claude/epr-meta/seam-forecast.yaml"
CALIBRATION_LEDGER_REL = ".claude/data/seam-calibration.jsonl"

OUTCOMES = ("hit", "miss-of-ranking", "miss-of-canon")


def forecast_fingerprint(rung: str, seam: str, classes) -> str:
    """The identity triple, and ONLY the identity triple. Classes are sorted lexicographically
    (so `C12` precedes `C7` — a byte order, deliberately, because it is stable and obvious rather
    than clever)."""
    cls = ",".join(sorted(str(c) for c in (classes or [])))
    return hashlib.sha256(f"{rung}|{seam}|{cls}".encode("utf-8")).hexdigest()[:12]


def load_forecast(repo_root: Path) -> tuple[list[dict], list[str]]:
    """Load the seeded forecast -> (rows, errors). Every row's fingerprint is RECOMPUTED and
    compared to the stored one; a mismatch is an error and the row keeps its COMPUTED fingerprint
    (the computed value is the truth — a stored fp is a human-readable convenience, never an
    authority)."""
    p = Path(repo_root) / FORECAST_REL
    if not p.is_file():
        return [], [f"{FORECAST_REL} not found — the forecast instrument ships pre-seeded; an "
                    f"empty one is a missing file, not an empty prediction set"]
    if yaml is None:
        return [], [f"PyYAML unavailable — {FORECAST_REL} not loaded"]
    try:
        data = yaml.safe_load(p.read_text()) or {}
    except Exception as e:  # noqa: BLE001
        return [], [f"{FORECAST_REL} unreadable/invalid: {e}"]
    if not isinstance(data, dict) or data.get("seam-forecast-version") != 1:
        return [], [f"{FORECAST_REL} missing/invalid `seam-forecast-version` (must be 1)"]
    rows: list[dict] = []
    errs: list[str] = []
    seen: dict[str, str] = {}
    for i, r in enumerate(data.get("rows") or []):
        if not isinstance(r, dict) or not r.get("seam") or not r.get("classes"):
            errs.append(f"{FORECAST_REL} rows[{i}] needs `seam` + non-empty `classes`")
            continue
        rung = str(r.get("rung") or "none")
        fp = forecast_fingerprint(rung, str(r["seam"]), r["classes"])
        if r.get("fp") and r["fp"] != fp:
            errs.append(f"{FORECAST_REL} rows[{i}] (`{r.get('title', '?')}`) stored fp "
                        f"`{r['fp']}` != computed `{fp}` — a hand-edited fingerprint silently "
                        f"orphans this row's calibration entries")
        if fp in seen:
            errs.append(f"{FORECAST_REL} rows[{i}] duplicates the identity triple of "
                        f"`{seen[fp]}` (same rung/seam/classes -> same fingerprint)")
            continue
        seen[fp] = str(r.get("title", i))
        rows.append(dict(r, fp=fp, rung=rung))
    return rows, errs


# ───────────────────────────── derived ranking ─────────────────────────────

def derived_rank(repo_root: Path, rows: list[dict], *, matrix: dict | None = None) -> list[dict]:
    """Score each forecast row with the MATRIX's own ranking function (recurrence x severity x
    rung-proximity) so the seeded human ordering and the arithmetic ordering can be read side by
    side. Disagreement is a signal, not an error: the seeded order carries judgment the arithmetic
    lacks, the arithmetic carries substrate state (which rung is red right now) the seeding did
    not have. A row's severity/recurrence is the MAX over its class set — a multi-class prediction
    is at least as bad as its worst class."""
    m = matrix if matrix is not None else _sm.matrix_data(repo_root)
    seam_by_id = {s["id"]: s for s in m.get("seams", [])}
    habits, _ = _sm.load_habits(repo_root)
    tips = m.get("concern_tips", {})
    out = []
    for r in rows:
        rec = max([int(tips.get(c, {}).get("recurrence_shifts") or 1) for c in r["classes"]] or [1])
        sev = max([_sm._SEVERITY_WEIGHT.get(tips.get(c, {}).get("severity_class"), 1.0)
                   for c in r["classes"]] or [1.0])
        seam = seam_by_id.get(r["seam"], {})
        prox, why = _sm.rung_proximity(seam, habits)
        # A row that names a habits rung directly is anchored to THAT node, not to the seam's
        # strongest linkage — the rung is the more specific fact and must win.
        node = habits.get(r["rung"])
        if node:
            prox, why = _sm.rung_proximity({"habits": [r["rung"]]}, habits)
        # A CONFIRMED-LIVE row has already collided, so its distance to the next rung is zero.
        # Without this floor the two bridges rows (rung `none`, no habits linkage) would sort to
        # the BOTTOM of a ranking whose whole purpose is "what will bite next" — an instrument
        # that buries its only confirmed findings is worse than no instrument.
        if r.get("status") == "confirmed-live":
            prox, why = 4.0, "confirmed live — distance to collision is zero"
        cell_states = {c: next((x["state"] for x in m.get("cells", [])
                                if x["concern"] == c and x["seam"] == r["seam"]), "?")
                       for c in r["classes"]}
        out.append(dict(r, score=round(rec * sev * prox, 3),
                        score_parts={"recurrence": rec, "severity": sev,
                                     "rung_proximity": prox, "rung_reason": why},
                        cell_states=cell_states))
    ranked = sorted(out, key=lambda r: (-r["score"], r.get("seeded_rank", 999)))
    for i, r in enumerate(ranked, 1):
        r["derived_rank"] = i
    return ranked


# ───────────────────────────── the calibration ledger ─────────────────────────────

def last_snapshot(repo_root: Path) -> dict | None:
    rows = _fl.read_rows(Path(repo_root) / CALIBRATION_LEDGER_REL)
    for r in reversed(rows):
        if r.get("kind") == "habits-snapshot" and isinstance(r.get("nodes"), dict):
            return r
    return None


def detect_flips(repo_root: Path) -> tuple[list[dict], dict]:
    """Compare the live habits to the last recorded snapshot. Returns (flips, live_nodes).
    A flip is {node, from, to, kind}: `kind` is `status-flip` for a changed status, `new-node` for
    a node that did not exist in the snapshot, and `first-run` for every node when no snapshot has
    ever been taken (the first `--record` establishes the baseline and files NO calibration rows —
    a baseline is not a set of events)."""
    habits, _ = _sm.load_habits(repo_root)
    live = {k: v.get("status") for k, v in habits.items()}
    snap = last_snapshot(repo_root)
    if snap is None:
        return [{"node": k, "from": None, "to": v, "kind": "first-run"} for k, v in sorted(live.items())], live
    prev = snap.get("nodes") or {}
    flips = []
    for node, status in sorted(live.items()):
        if node not in prev:
            flips.append({"node": node, "from": None, "to": status, "kind": "new-node"})
        elif prev[node] != status:
            flips.append({"node": node, "from": prev[node], "to": status, "kind": "status-flip"})
    return flips, live


def classify_flip(flip: dict, rows: list[dict]) -> list[dict]:
    """Proposed calibration entries for one flip. A forecast row whose `rung` names the flipped
    node is a HIT candidate; a flip with no forecast row at all is a MISS-OF-RANKING candidate.
    `miss-of-canon` is never proposed here — see the module docstring."""
    matches = [r for r in rows if r.get("rung") == flip["node"]]
    if matches:
        return [{"outcome": "hit", "fp": r["fp"], "rung": r["rung"], "seam": r["seam"],
                 "classes": list(r["classes"]), "title": r.get("title", ""),
                 "trigger": f"habits-flip:{flip['node']} {flip['from']}->{flip['to']}"}
                for r in matches]
    return [{"outcome": "miss-of-ranking", "fp": "", "rung": flip["node"], "seam": "",
             "classes": [], "title": "",
             "trigger": f"habits-flip:{flip['node']} {flip['from']}->{flip['to']}",
             "note": "no forecast row names this rung — the canon may be fine and only the "
                     "ranking wrong; confirm the colliding class before recording"}]


def calibrate(repo_root: Path) -> dict:
    """READ-ONLY calibration pass: what would be recorded, and why."""
    rows, errs = load_forecast(repo_root)
    flips, live = detect_flips(repo_root)
    proposals: list[dict] = []
    baseline_only = bool(flips) and all(f["kind"] == "first-run" for f in flips)
    if not baseline_only:
        for f in flips:
            if f["kind"] == "first-run":
                continue
            proposals.extend(classify_flip(f, rows))
    return {"forecast_rows": len(rows), "errors": errs, "flips": flips,
            "baseline_only": baseline_only, "proposals": proposals,
            "live_nodes": live, "has_snapshot": last_snapshot(repo_root) is not None}


def record(repo_root: Path, cal: dict) -> dict:
    """Append the proposed calibration rows + a fresh habits snapshot. Append-only; never drains.
    With zero proposals and an unchanged habits this writes NOTHING (an unchanged habits is not an
    event) — which is what makes it safe to run on a loop."""
    root = Path(repo_root)
    led = root / CALIBRATION_LEDGER_REL
    written = 0
    for p in cal.get("proposals", []):
        entry = {"ts": _fl.utc_now(), "kind": "calibration", **p}
        if _fl.append(led, entry):
            written += 1
    snap_needed = cal.get("baseline_only") or cal.get("flips") or not cal.get("has_snapshot")
    if snap_needed:
        _fl.append(led, {"ts": _fl.utc_now(), "kind": "habits-snapshot",
                          "nodes": cal.get("live_nodes", {})})
    return {"written": written, "snapshot": bool(snap_needed)}


def record_miss_of_canon(repo_root: Path, *, note: str, seam: str = "", rung: str = "none",
                         trigger: str = "manual") -> bool:
    """The canon-growth intake. A miss-of-canon claim MUST carry evidence prose — the admission
    bar is >=2 dated instances at distinct seams, and this row is where that argument starts its
    life. Refuses an empty note rather than filing a content-free growth signal."""
    if not note or not note.strip():
        return False
    return _fl.append(Path(repo_root) / CALIBRATION_LEDGER_REL, {
        "ts": _fl.utc_now(), "kind": "calibration", "outcome": "miss-of-canon",
        "fp": "", "rung": rung, "seam": seam, "classes": [], "trigger": trigger,
        "note": note.strip()})


def ledger_summary(repo_root: Path) -> dict:
    rows = _fl.read_rows(Path(repo_root) / CALIBRATION_LEDGER_REL)
    counts = {o: 0 for o in OUTCOMES}
    for r in rows:
        o = r.get("outcome")
        if o in counts:
            counts[o] += 1
    n_cal = sum(counts.values())
    return {"n_rows": len(rows), "n_calibration": n_cal, "counts": counts,
            "hit_rate": (counts["hit"] / n_cal) if n_cal else None,
            "canon_growth_signals": counts["miss-of-canon"]}


# ───────────────────────────── rendering ─────────────────────────────

def render_forecast(repo_root: Path, *, matrix: dict | None = None, as_json: bool = False) -> str:
    rows, errs = load_forecast(repo_root)
    ranked = derived_rank(repo_root, rows, matrix=matrix) if rows else []
    summary = ledger_summary(repo_root)
    if as_json:
        return json.dumps({"rows": ranked, "errors": errs, "ledger": summary}, indent=1)
    L = ["FORECAST  (plan P4.4 — seeded; the unexamined cells ARE the forecast)", "=" * 72]
    L.append(f"{len(rows)} row(s) · calibration ledger: {summary['n_calibration']} outcome(s) "
             f"[hit {summary['counts']['hit']} · miss-of-ranking "
             f"{summary['counts']['miss-of-ranking']} · miss-of-canon "
             f"{summary['counts']['miss-of-canon']}]")
    L.append("")
    L.append(f"  {'fp':<12} {'seed':>4} {'derv':>4}  {'rung':<26} {'seam':<6} "
             f"{'classes':<12} cell state(s)")
    for r in ranked:
        cells = ",".join(f"{c}:{s}" for c, s in sorted(r["cell_states"].items()))
        tag = " (runner-up)" if r.get("runner_up") else ""
        live = " ⚑LIVE" if r.get("status") == "confirmed-live" else ""
        L.append(f"  {r['fp']:<12} {r.get('seeded_rank', '?'):>4} {r['derived_rank']:>4}  "
                  f"{r['rung']:<26} {r['seam']:<6} {','.join(r['classes']):<12} "
                  f"{cells}{tag}{live}")
    for e in errs:
        L.append(f"  ⛔ {e}")
    L.append("")
    L.append("  seed = the plan's own ordering (never recomputed) · derv = recurrence x severity x")
    L.append("  rung-proximity. Disagreement is a signal: judgment vs live substrate state.")
    return "\n".join(L)
