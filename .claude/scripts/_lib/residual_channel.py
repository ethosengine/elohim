"""residual_channel — the INTAKE and REPORT half of C14's residual channel (plan task P4.7).

Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
(canon row C14; Design surface 8 "The residual channel — metal to dashboard"). The capture half
is `crates/seam-contracts/src/residual.rs` (`ResidualWitness`); this module is the three stations
after it — **collect**, **report**, **learn**.

BIND, DO NOT FORK. `.claude/data/runtime-findings.jsonl` + `runtime-harvest.py` + the
`runtime-triage` agent is the LIVE instance of exactly this pipeline for *known* exhaustion
classes. A residual rides it unchanged: same ledger file, same `runtime_harvest.fingerprint`, same
`runtime_harvest.reconcile` fold, same dispatch. Only the `class` differs — and `class` was
already an opaque field of the ledger entry, which is why the pure core needed **no change at
all** (proven, not asserted, by `__tests__/residual_channel_test.py`). Forking a second ledger
would have given the fleet two half-drained queues and a second closure convention to keep
honest.

THE NO-RUNTIME-WRITE RULE STILL HOLDS. Runtime Rust never writes `.claude/data`. A node SERVES its
witnessed capsules at an admin read-endpoint (`GET /admin/residuals`, a JSON array of serialized
`ResidualWitness`); the external poller reads them and writes the ledger. This is the same shape
`/admin/self-healing` already has, including the PENDING part: the endpoint may not exist yet, and
`get_json` degrading quiet means an absent endpoint contributes NO signal rather than a false one
(C4 — unmeasured never reads as zero).

MISHPAT, NOT PUNISHMENT. The dispatch this module feeds exists to RESTORE THE CAPABILITY that
failed, never to penalize the node that surfaced it. A fleet that treats residual reports as blame
gets fewer reports, not fewer residuals — and the reports it loses are exactly the novel ones, the
only kind the canon can grow from.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

# The findings-ledger namespace declared by the C14 canon row's `findings_namespace` field
# (.claude/epr-meta/concerns.yaml, `c14-witnessed-residual`). The Rust capsule derives the same
# string from `PIN_C14_WITNESSED_RESIDUAL.id`; both are checked against this hand-written golden
# in their own language's test, so neither implementation can be the other's mirror.
CLASS = "concern:c14-witnessed-residual"

#: A capsule that arrives unreadable is still a residual — C14 says WITNESSED, NEVER DROPPED, and
#: "the producer emitted something we cannot parse" is itself a finding worth a dispatch. It gets
#: this stable low-cardinality kind so the malformed cases converge onto ONE fingerprint per node
#: instead of storming the ledger.
MALFORMED_KIND = "malformed-capsule"

LEDGER_REL = ".claude/data/runtime-findings.jsonl"

#: The calibration ledger is the forecast/matrix leg's artifact (plan P4.4,
#: `_lib/seam_forecast.py`), APPEND-ONLY by design: a hit/miss is a historical fact, and a "live
#: set" of them would destroy the growth-rate measure it exists to compute. This module READS it
#: and appends exactly one additive row kind (`outcome: residual-disposition`); it never mutates,
#: drains, or re-fingerprints anything the forecast leg owns.
CALIBRATION_LEDGER_REL = ".claude/data/seam-calibration.jsonl"

#: The three dispositions C14 names. `new-concern-class` is ALSO recorded by the forecast leg's
#: own `--record-miss-of-canon` verb (its `outcome: miss-of-canon` row is the canon's admission
#: intake); the two are joined on `residual_fp`, so one act never double-counts.
DISPOSITIONS = ("cure", "declared-variance", "new-concern-class")


# ───────────────────────────── collect: capsule -> ledger finding ─────────────────────────────

def capsule_provenance(capsule: dict) -> str:
    """`residual:<seam>:<residualKind>` — the fingerprint axis, and the whole cardinality budget.

    Derived here rather than read from a capsule field on purpose: a serialized `provenance`
    would be a second truth about `seam` + `residualKind` sitting inside the same object, and a
    producer could then advertise a provenance its own seam contradicts. The Rust side computes
    the identical string in `ResidualWitness::finding_provenance`; both are pinned against a
    hand-written golden literal in their own test, which is the anti-mirror discipline (compute
    the two sides independently, compare each to something neither derived).
    """
    seam = str(capsule.get("seam") or "").strip() or "unknown-seam"
    kind = str(capsule.get("residualKind") or "").strip() or MALFORMED_KIND
    return f"residual:{seam}:{kind}"


def _decision_detail(capsule: dict) -> str:
    state = capsule.get("decisionState")
    if isinstance(state, dict):
        kind, value = state.get("kind"), str(state.get("value") or "")
        if kind == "wouldHaveEmitted":
            return f"would_have_emitted={value}"
        if kind == "raw":
            return f"raw={value}"
    return "state=unreadable"


def _context_census(capsule: dict) -> str:
    """`clock=2 lag=1 peer_state=3` — the census of relative context the capsule carries.

    C8's "every meter names its semantics": a reader can tell AT THE LEDGER, without opening the
    capsule, whether this residual was witnessed with enough relative picture to unstack a
    distributed cause (the 2026-07-12 five-defect arc: distributed causes arrive as STACKS) or
    whether it arrived blind.
    """
    counts: dict[str, int] = {}
    for entry in capsule.get("context") or []:
        if isinstance(entry, dict) and entry.get("kind"):
            k = str(entry["kind"])
            counts[k] = counts.get(k, 0) + 1
    order = ["clock", "lag", "peerState", "peer_state", "input", "env"]
    parts = [f"{k}={counts[k]}" for k in order if k in counts]
    parts += [f"{k}={v}" for k, v in sorted(counts.items()) if k not in order]
    return " ".join(parts) if parts else "none"


def capsule_line(capsule: dict) -> str:
    """The ≤300-char ledger line for one capsule (mirrors `ResidualWitness::finding_line`)."""
    seam = str(capsule.get("seam") or "unknown-seam")
    kind = str(capsule.get("residualKind") or MALFORMED_KIND)
    digest = str(capsule.get("inputsDigest") or "-")
    line = (f"{seam} residual [{kind}] {_decision_detail(capsule)} "
            f"| inputs={digest} | ctx: {_context_census(capsule)}")
    return line if len(line) <= 300 else line[:299] + "…"


def findings_from_capsules(node: str, capsules) -> list[dict]:
    """PURE. Serialized `ResidualWitness` capsules -> finding dicts in the EXACT shape
    `runtime_harvest.evaluate` returns, so the caller fingerprints and reconciles them with the
    unmodified pure core.

    Degrades PER CAPSULE (the EprRouter poisoned-scope lesson: a fail-closed `collect` over rows
    degrades the whole set to silence). One unreadable capsule becomes one malformed-capsule
    finding; it never suppresses its well-formed siblings, and it is never dropped.
    """
    out: list[dict] = []
    if not isinstance(capsules, list):
        return out
    for raw in capsules:
        if not isinstance(raw, dict):
            out.append({
                "node": node, "class": CLASS,
                "provenance": f"residual:unknown-seam:{MALFORMED_KIND}",
                "line": f"non-object capsule of type {type(raw).__name__} served at "
                        f"/admin/residuals — witnessed, not dropped (C14)",
            })
            continue
        out.append({
            "node": node,
            "class": CLASS,
            "provenance": capsule_provenance(raw),
            "line": capsule_line(raw),
        })
    # De-duplicate WITHIN one poll: N occurrences of one residual shape in a single window are one
    # finding (C6b — replaying a decision against settled state mints nothing). Recurrence across
    # polls is what `reconcile` counts, via `seen`.
    seen, deduped = set(), []
    for f in out:
        if f["provenance"] in seen:
            continue
        seen.add(f["provenance"])
        deduped.append(f)
    return deduped


# ───────────────────────────── report: the developer RCA surface ─────────────────────────────

def _load_jsonl(path: Path) -> list[dict]:
    entries = []
    if not path.is_file():
        return entries
    with path.open(encoding="utf-8", errors="replace") as fh:
        for raw in fh:
            raw = raw.strip()
            if not raw:
                continue
            try:
                entries.append(json.loads(raw))
            except json.JSONDecodeError:
                continue
    return entries


def residual_entries(repo_root, ledger_path=None) -> list[dict]:
    """Every LIVE residual finding on the runtime ledger, newest-recurrence first."""
    path = Path(ledger_path) if ledger_path else Path(repo_root) / LEDGER_REL
    rows = [e for e in _load_jsonl(path) if e.get("class") == CLASS]
    rows.sort(key=lambda e: (-int(e.get("last_poll") or 0), str(e.get("fp"))))
    return rows


def render_residual_panel(repo_root, ledger_path=None, *, limit: int = 8) -> str:
    """The residual panel for the developer RCA surface — rendered BESIDE the forecast.

    Design surface 8: *"residuals delivered into the developer-facing RCA surface beside the
    forecast — never parked at a log level the deployment drops."* Forecasting narrows the
    unanticipated; this panel is what remains, and putting them side by side is the point: an
    unexamined matrix cell and a live residual at the same seam are the same finding twice.

    WIRING (coordinate-by-contract, not by collision): `placement-audit.py --epr-meta` is owned by
    another leg this phase, so this is a standalone renderer with a one-line call site::

        from _lib.residual_channel import render_residual_panel
        print(render_residual_panel(repo_root))

    Returns "" when there is nothing to report, so the call site needs no guard.
    """
    rows = residual_entries(repo_root, ledger_path)
    metab = exception_metabolism(repo_root, ledger_path=ledger_path)
    if not rows and metab["status"] == "unreachable":
        return ""

    out = ["Residual channel (C14 — witnessed residuals, beside the forecast):"]
    if not rows:
        out.append("  • no live residuals — the anticipated is holding")
    for e in rows[:limit]:
        thin = " ⚠ thin capsule (no clock/lag/peer_state — cannot unstack a distributed cause)" \
            if "ctx: none" in str(e.get("line", "")) else ""
        out.append(
            f"  • {e.get('fp', '?')} [{e.get('node', '?')}] seen×{e.get('seen', 1)} "
            f"{e.get('provenance', '?')}{thin}"
        )
        out.append(f"      {str(e.get('line', ''))[:180]}")
    if len(rows) > limit:
        out.append(f"  … {len(rows) - limit} more (read {LEDGER_REL}, class={CLASS})")

    if metab["status"] == "unreachable":
        out.append(f"  exception-metabolism rate: UNREACHABLE — no calibration ledger at "
                   f"{CALIBRATION_LEDGER_REL} (never read as 0; C4). "
                   f"See residual_channel.exception_metabolism for the intake contract.")
    else:
        out.append(
            f"  exception-metabolism rate: {metab['rate']:.0%} "
            f"({metab['dispositioned']}/{metab['novel']} novel residuals dispositioned, "
            f"{metab['outstanding']} outstanding; {metab['by_disposition']})"
        )
    out.append("  Disposition = cure | declared-variance | new-concern-class. Recovery is "
               "restored capability — Mishpat, never punishment.")
    return "\n".join(out)


# ───────────────────────────── learn: disposition -> calibration ─────────────────────────────

def find_calibration_ledger(repo_root) -> Path | None:
    """The calibration ledger, or None. The forecast leg mints it; this module never creates it."""
    p = Path(repo_root) / CALIBRATION_LEDGER_REL
    return p if p.is_file() else None


def record_residual_disposition(repo_root, residual_fp: str, disposition: str, *,
                                note: str = "", seam: str = "") -> bool:
    """Append ONE disposition row for a witnessed residual — the `learn` station.

    THE INTAKE CONTRACT, stated as the row this writes (additive to `_lib.seam_forecast`'s
    vocabulary, never a mutation of it)::

        {"ts": …, "kind": "calibration", "outcome": "residual-disposition",
         "residual_fp": "<runtime-ledger fp>", "disposition": "cure|declared-variance|
         new-concern-class", "seam": "<registered decision point>", "note": "<why>"}

    Why this shape composes rather than collides:

    * ``kind: "calibration"`` and the `ts`/`fp` conventions match the forecast leg's rows, so one
      reader walks the whole file.
    * ``outcome`` is a value **outside** `seam_forecast.OUTCOMES`, and that module counts only
      outcomes it knows (`if o in counts`) — so these rows are inert to the forecast's hit-rate
      and canon-growth summaries. Two measures, one append-only file, neither distorting the other.
    * ``residual_fp`` is the runtime-ledger fingerprint, so a residual and its verdict join
      **without a second id space** — the join C14 needs and the one thing a free-text note could
      never provide.
    * A ``new-concern-class`` disposition is expected to be accompanied by the forecast leg's own
      ``--record-miss-of-canon`` row (its `outcome: miss-of-canon` IS the canon's admission
      intake). This function does not write that row: minting a canon class is a budgeted,
      deliberate act with an evidence bar, and a side effect of a triage helper is exactly the
      wrong way for one to be born.

    Refuses an unrecognized disposition and an empty fingerprint rather than filing a row that
    would inflate the numerator with nothing behind it.
    """
    if disposition not in DISPOSITIONS or not str(residual_fp or "").strip():
        return False
    try:
        from _lib import findings_ledger as _fl
    except ImportError:  # the ledger primitives are not available in this checkout
        return False
    return _fl.append(Path(repo_root) / CALIBRATION_LEDGER_REL, {
        "ts": _fl.utc_now(), "kind": "calibration", "outcome": "residual-disposition",
        "fp": "", "residual_fp": str(residual_fp).strip(), "disposition": disposition,
        "seam": seam, "note": str(note or "").strip(),
    })


def exception_metabolism(repo_root, calibration_path=None, ledger_path=None) -> dict:
    """**Exception-metabolism rate** — novel residuals dispositioned over time, reported as a
    fleet health measure beside uptime (plan surface 8's closing clause).

    ``rate = |dispositioned residual fps| / |novel residual fps|`` where

    * **dispositioned** = residual fps carrying a ``residual-disposition`` row (and/or a
      ``miss-of-canon`` row that names one) on the append-only calibration ledger;
    * **novel** = those, PLUS every residual fp still LIVE and undispositioned on the runtime
      ledger. A residual that was cured and closed by disappearance leaves the runtime ledger
      entirely (D5), so its only durable trace is its disposition row — which is precisely why the
      numerator's home has to be the append-only file and not the draining one.

    THE ONE HONEST LIMIT, stated rather than hidden: a residual that was cured and closed WITHOUT
    a disposition row is invisible to both terms. It is neither counted as metabolized nor as
    outstanding. That is a known undercount of the denominator, not a silent zero, and the fix is
    social — triage records the disposition — not arithmetic.

    Returns ``{"status": "unreachable"}`` while no calibration ledger exists. UNREACHABLE, never
    0.0: C4 is the canon row this instrument is likeliest to violate against itself, and
    *unmeasured reading as zero* would make an unbuilt organ look like a perfectly healthy one
    (`d6c88e385`, 2026-07-23, shefa facing — unmeasured != zero).
    """
    path = Path(calibration_path) if calibration_path else find_calibration_ledger(repo_root)
    if path is None or not path.is_file():
        return {"status": "unreachable",
                "reason": f"no calibration ledger at {CALIBRATION_LEDGER_REL} "
                          f"(the forecast leg mints it; see _lib/seam_forecast.py)"}

    by_disposition: dict[str, int] = {}
    dispositioned: dict[str, str] = {}
    canon_growth_signals = 0
    for row in _load_jsonl(path):
        outcome = row.get("outcome")
        fp = str(row.get("residual_fp") or "").strip()
        if outcome == "miss-of-canon":
            canon_growth_signals += 1
            if fp:
                dispositioned.setdefault(fp, "new-concern-class")
            continue
        if outcome != "residual-disposition" or not fp:
            continue
        d = str(row.get("disposition") or "").strip()
        if d not in DISPOSITIONS:
            by_disposition[f"unrecognized:{d or '∅'}"] = \
                by_disposition.get(f"unrecognized:{d or '∅'}", 0) + 1
            continue
        dispositioned.setdefault(fp, d)

    for fp, d in dispositioned.items():
        by_disposition[d] = by_disposition.get(d, 0) + 1

    live_open = {str(e.get("fp")) for e in residual_entries(repo_root, ledger_path)}
    novel = set(dispositioned) | live_open
    return {
        "status": "present",
        "path": str(path),
        "novel": len(novel),
        "dispositioned": len(dispositioned),
        "outstanding": len(live_open - set(dispositioned)),
        "rate": (len(dispositioned) / len(novel)) if novel else 0.0,
        "by_disposition": by_disposition or {},
        "canon_growth_signals": canon_growth_signals,
    }


# ───────────────────────────── dispatch framing (used by runtime-harvest) ─────────────────────

#: Per-class triage framing for the runtime-harvest dispatch directive. A residual is NOT a
#: self-reported exhaustion and must not be described as one — mislabeling the class at the
#: dispatch boundary is C4 (honest absence) and C7 (advertise/serve) failing in the harvester
#: itself, and it would send the triage agent looking for a circuit breaker that never opened.
CLASS_FRAMING = {
    CLASS: (
        "A decision point WITNESSED a residual its outcome set does not classify (concern canon "
        "C14). Read the capsule's context census in the line: clocks/lags/peer-states are what "
        "unstack a distributed cause — a capsule showing `ctx: none` is a thin witness and the "
        "FIRST fix is usually at the seam that emitted it. Disposition the residual — cure, "
        "declared variance, or a NEW concern class via the calibration ledger's miss-of-canon "
        "intake. Recovery is restored capability (Mishpat), never punishment: the node that "
        "surfaced this did the fleet a service."
    ),
}

DEFAULT_FRAMING = ("The node self-REPORTED exhaustion; scope the cause, canonicalize by concern, "
                   "fix if bounded or document the blocker.")


def framing_for(cls: str) -> str:
    """Triage framing for a ledger class (falls back to the exhaustion framing)."""
    return CLASS_FRAMING.get(cls, DEFAULT_FRAMING)


def normalize_provenance(prov: str) -> str:
    """Trim a provenance to the ledger's practical width without touching its shape."""
    return re.sub(r"\s+", " ", str(prov or "")).strip()[:200]


__all__ = [
    "CLASS", "CLASS_FRAMING", "DEFAULT_FRAMING", "DISPOSITIONS", "MALFORMED_KIND",
    "CALIBRATION_LEDGER_REL", "LEDGER_REL",
    "capsule_line", "capsule_provenance", "exception_metabolism",
    "find_calibration_ledger", "findings_from_capsules", "framing_for",
    "normalize_provenance", "record_residual_disposition", "render_residual_panel",
    "residual_entries",
]
