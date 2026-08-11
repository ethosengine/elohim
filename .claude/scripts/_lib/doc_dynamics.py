"""The doc corpus as a modelled STOCK — the dynamics of our own development discipline.

Meadows' rule: overshoot is indicated when a rate ratio crosses 1.0, not when the stock is
visibly gone. *"Deforestation is indicated not when the forest is gone, but when the rate of
harvest first exceeds the rate of regrowth."*

# What changed in slice 2, and why the shape matters more than the number

Slice 1 emitted a bare `kind: "ratio"` — generation over absorption, and nothing else. That is
half a model. A ratio with no level behind it cannot answer the question that actually
distinguishes a healthy corpus from a silting one: **turnover time**. A corpus authoring 10 docs
a week and retiring 10 is in dynamic equilibrium; one doing zero of both has the same net change
and is dead. Only the flows tell them apart, and only a level makes turnover computable at all.

So this module now emits a STOCK — level, inflow, outflow — mirroring
`elohim/epr-rea/src/stock.rs` field for field, with the derived quantities computed the same way
and refusing the same things. The parity gate is `doc_dynamics_test.py`.

# The corpus is a SINK, and that decides the direction of the index

Documents are *emitted*; compaction, decompose-to-zero-residue, and moves to `held/` are the
*absorption*. So the index is `emission / absorption` (inflow ÷ outflow), above 1.0 = the sink is
filling faster than anything drains it. The reciprocal reading — harvest/regeneration — applies
to a resource being drawn down and is the wrong way up for this corpus. Getting that backwards
reports overshoot as health, which is why `stock.rs` names both directions rather than offering
one "Meadows index."

# The window is part of the measure's IDENTITY

Slice 1's 28-day window was an argument default, and in the first live run **that default alone
decided whether the headline read `unknown` or `3.2`.** A measure whose conclusion flips on an
undeclared parameter is not yet honest. The window is now declared, named in every `basis`, and
reported at more than one span — because absorption in this repo is bursty (a large sweep
clustered in early June, then nothing for a month), so any single window is a *claim about what
counts as now* rather than a neutral setting.
"""
from __future__ import annotations

import subprocess

DOC_GLOBS = ["genesis/docs/superpowers/specs", "genesis/docs/superpowers/plans"]

# Windows reported side by side. One window is a claim; several make the burstiness visible
# instead of letting whichever one is listed first decide the finding.
DEFAULT_WINDOWS_DAYS = (28, 90, 365)

# A hung `git log` must never stall session start.
GIT_TIMEOUT_SECONDS = 5

# Absorption we cannot see, expressed as a multiplier on what we can. Applied ONLY when the
# counted base is non-zero: multiplying a zero base yields a zero-width band at exactly the
# moment the count is least trustworthy (spec Q10 — `Interval::multiplier_widen` is the general
# cure, mirrored here).
ABSORPTION_UNCOUNTED_MULTIPLIER = (1.0, 3.0)

_INF = float("inf")


def _git(args: list[str]) -> str:
    return subprocess.run(
        ["git"] + args, capture_output=True, text=True, check=False,
        timeout=GIT_TIMEOUT_SECONDS,
    ).stdout


def _count_added(window_days: int) -> int:
    """Docs authored into the corpus in the window — witnessed from git."""
    if window_days <= 0:
        return 0
    out = _git(["log", f"--since={window_days} days ago", "--diff-filter=A", "--name-only",
                "--pretty=format:", "--"] + DOC_GLOBS)
    return len({line for line in out.splitlines() if line.strip()})


def _count_absorbed(window_days: int) -> tuple[int, int]:
    """Absorption in the window → (absorbed, renamed_in_place).

    THE DISCRIMINATION SLICE 1 COULD NOT MAKE. Its `--diff-filter=DR` counted a move to `held/`
    (real absorption — the doc left the plate) and an in-place rename (not absorption at all —
    same doc, same directory, new filename) as the same event, which is why its estimate had to
    stay wide.

    With `-M` git reports renames as `R<score>\\told\\tnew`, and the destination decides: a
    rename that lands back inside `DOC_GLOBS` never left the corpus. This mirrors
    `derive_absorption` in `elohim/eprfs/epr-cli/src/flow/project.rs`, which makes the same call
    against the recipe's stage globs — same judgment, two implementations, because the live
    session-start instrument cannot depend on a Rust binary being built.

    Returns the excluded in-place count too, so the basis can say what was NOT counted rather
    than quietly improving the number.
    """
    if window_days <= 0:
        return 0, 0
    out = _git(["log", f"--since={window_days} days ago", "--diff-filter=DR", "--name-status",
                "-M", "--pretty=format:", "--"] + DOC_GLOBS)
    absorbed: set[str] = set()
    in_place: set[str] = set()
    for line in out.splitlines():
        cols = line.split("\t")
        if len(cols) < 2 or not cols[0]:
            continue
        status, path = cols[0], cols[1]
        if status.startswith("R") and len(cols) >= 3:
            dest = cols[2]
            if any(dest.startswith(g + "/") for g in DOC_GLOBS):
                in_place.add(path)   # moved within the corpus — not absorption
                continue
            absorbed.add(path)       # moved out (held/, archive, elsewhere)
        elif status.startswith("D"):
            absorbed.add(path)
    return len(absorbed), len(in_place)


def _level() -> int:
    """The stock itself: docs currently live in the corpus. `git ls-files`, not a running
    difference — a level is a state, and deriving it from flow arithmetic would let any gap in
    the flow history silently become a wrong level."""
    out = _git(["ls-files", "--"] + DOC_GLOBS)
    return len([ln for ln in out.splitlines() if ln.strip().endswith(".md")])


def _weeks(days: int) -> float:
    return days / 7.0


def _div(num_lo: float, num_hi: float, den_lo: float, den_hi: float) -> dict:
    """Interval division mirroring `Interval::div` — a denominator band admitting zero yields
    honest absence, NEVER `[+inf, +inf]`. That shape asserts *exact infinity*, the most confident
    claim expressible, at the precise moment the data is most absent (spec Q12/Q13)."""
    if den_lo <= 0.0 <= den_hi:
        return {"lo": -_INF, "hi": _INF}
    q = [num_lo / den_lo, num_lo / den_hi, num_hi / den_lo, num_hi / den_hi]
    return {"lo": min(q), "hi": max(q)}


def _is_unknown(interval: dict) -> bool:
    """Mirrors `Interval::is_unknown` INCLUDING its sign check. Slice 1's version tested only
    `isinf` on both bounds, so `[+inf, +inf]` reported as honest absence."""
    return interval["lo"] == -_INF and interval["hi"] == _INF


def doc_corpus_stock(window_days: int = 28, *, count_fns=None) -> dict:
    """The doc corpus as a stock over `window_days`, on the wire vocabulary of
    `elohim/epr/src/measure.rs` and the shape of `elohim/epr-rea/src/stock.rs`.

    `count_fns` is an injectable `(added_fn, absorbed_fn, level_fn)` triple for tests, so results
    are deterministic rather than dependent on what the last N days of this repo's history
    happen to contain.
    """
    added_fn, absorbed_fn, level_fn = count_fns or (_count_added, _count_absorbed, _level)
    generated = added_fn(window_days)
    absorbed, in_place = absorbed_fn(window_days)
    level = level_fn()
    weeks = _weeks(window_days)
    window_label = f"{window_days}d window ({weeks:.1f} week-periods)"

    inflow = generated / weeks
    outflow = absorbed / weeks
    # Q10 mirrored: widen only from a non-zero base. At zero counted absorption the multiplier
    # carries no information, so the band is honest absence rather than a confident zero.
    lo_m, hi_m = ABSORPTION_UNCOUNTED_MULTIPLIER
    if absorbed:
        out_lo, out_hi = outflow * lo_m, outflow * hi_m
    else:
        out_lo, out_hi = 0.0, _INF

    index_iv = _div(inflow, inflow, out_lo, out_hi)
    turnover_iv = _div(float(level), float(level), out_lo, out_hi)
    # Q13 mirrored: a value the interval declares unknowable must not compare. `None` here is
    # Python's honest NaN — a naive `value > 1.0` overshoot test raises instead of quietly
    # returning True off an absence.
    index_value = None if _is_unknown(index_iv) or outflow == 0.0 else inflow / outflow
    turnover_value = None if _is_unknown(turnover_iv) or outflow == 0.0 else level / outflow

    uncounted = (
        f"in-place compaction and decompose-to-zero-residue remain uncounted (no ledger sees "
        f"them), so absorption is widened x[{lo_m},{hi_m}]"
    )
    return {
        "window_days": window_days,
        "level": {
            "value": float(level),
            "kind": "level",
            "confidence": {
                "claim": "witnessed",
                "interval": {"lo": float(level), "hi": float(level)},
                "basis": f"{level} live .md under {' + '.join(DOC_GLOBS)} (git ls-files)",
            },
        },
        "inflow": {
            "value": inflow,
            "kind": "rate", "per": "week",
            "confidence": {
                "claim": "witnessed",
                "interval": {"lo": inflow, "hi": inflow},
                "basis": f"{generated} docs added over the {window_label}, git-witnessed",
            },
        },
        "outflow": {
            "value": outflow,
            "kind": "rate", "per": "week",
            "confidence": {
                "claim": "estimated",
                "interval": {"lo": out_lo, "hi": out_hi},
                "basis": (
                    f"{absorbed} docs deleted or moved OUT of the corpus over the "
                    f"{window_label} ({in_place} in-place renames excluded — they never left "
                    f"the plate); {uncounted}"
                ),
            },
        },
        "emission_absorption": {
            "value": index_value,
            "kind": "ratio",
            "confidence": {
                "claim": "estimated",
                "interval": index_iv,
                "basis": (
                    f"emission/absorption over the {window_label}: {generated} authored vs "
                    f"{absorbed} absorbed. The corpus is a SINK — above 1.0 it fills faster "
                    f"than it drains"
                ),
            },
        },
        "turnover_time": {
            "value": turnover_value,
            "kind": "level",  # a duration, denominated in weeks (spec Q15)
            "confidence": {
                "claim": "estimated",
                "interval": turnover_iv,
                "basis": (
                    f"weeks for the {level}-doc corpus to fully turn over at the "
                    f"{window_label} absorption rate; unbounded turnover reports as UNKNOWN, "
                    f"never as a number"
                ),
            },
        },
        "counts": {"generated": generated, "absorbed": absorbed, "in_place_renames": in_place},
    }


def corpus_dynamics(windows=DEFAULT_WINDOWS_DAYS, *, count_fns=None) -> list[dict]:
    """The stock at several windows. Absorption here is BURSTY, so a single window is a claim
    about what counts as "now" — reporting a few makes that claim arguable instead of invisible."""
    return [doc_corpus_stock(w, count_fns=count_fns) for w in windows]


# --------------------------------------------------------------------------- back-compat

def generation_absorption_ratio(window_days: int = 28, *, count_fn=None) -> dict:
    """Slice 1's bare-ratio entry point, preserved so no caller breaks mid-slice.

    Returns the `emission_absorption` leg of the full stock, plus the flat `generated` /
    `absorbed_counted` keys the session-start line reads. New callers should take
    `doc_corpus_stock` — a ratio with no level behind it cannot answer the turnover question,
    which is the one that actually separates equilibrium from silting.
    """
    fns = None
    if count_fn is not None:
        # Slice 1's injectable took a single `(window, diff_filter)` callable. Adapt it so the
        # old test shape keeps working against the new internals.
        fns = (
            lambda w: count_fn(w, "A"),
            lambda w: (count_fn(w, "DR"), 0),
            lambda: 0,
        )
    stock = doc_corpus_stock(window_days, count_fns=fns)
    idx = stock["emission_absorption"]
    return {
        "value": _INF if idx["value"] is None else idx["value"],
        "kind": "ratio",
        "generated": stock["counts"]["generated"],
        "absorbed_counted": stock["counts"]["absorbed"],
        "confidence": idx["confidence"],
    }
