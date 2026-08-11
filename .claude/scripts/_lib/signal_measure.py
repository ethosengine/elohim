"""One place where a memory-kit signal becomes a declared measure.

The kit computes a lot of numbers — drift scores, cleanup pressure, stasis composites, leverage
scores — and until now not one of them declared what kind of quantity it was. That matters more
here than almost anywhere else in the repo, because these are the signals that decide **when the
memory ceremony fires**. A miscalibrated firing signal is Meadows' third cause of overshoot (a
delay or error in perceiving the limit) sitting directly on the loop that exists to prevent it.

Wire vocabulary mirrors `elohim/epr/src/measure.rs` — the same `kind` / `confidence` shape the
`.epr-meta` L6 gate already enforces on rule-level measures, and the same one `doc_dynamics.py`
emits. Kept in ONE module because four call sites hand-rolling the same dict is how the wire
shape drifts from the Rust it is supposed to mirror.

## On `ClaimKind` for a computed score

`modelled` is the honest claim for a weighted composite, and it is not a hedge. A witnessed count
is an observation; an estimate is an observation with a stated band; a **modelled** value is the
output of a formula whose weights someone chose. `drift_score`'s weights (`age 1.0`,
`scope_edits 0.8`, `direct_edits 2.0`, `structural_edits 5.0`) are re-tunable by design — the
docstring says so — which means the number moves when a human changes their mind, not when the
world does. Calling that `witnessed` would be the false precision this ontology exists to
prevent, and `estimated` would imply a band around a true value that no observation defines.
"""
from __future__ import annotations

_INF = float("inf")

# Mirrors `MeasureKind` (elohim/epr/src/measure.rs, serde lowercase) and
# `epr_meta.MEASURE_KIND_VOCAB` — the L6 gate's vocabulary.
KINDS = ("level", "rate", "ratio")
PERIODS = ("second", "minute", "hour", "day", "week", "month", "year")


def unknown_interval() -> dict:
    """`Interval::unknown()` — the interval that admits everything. Honest absence, and note the
    SIGN: `[+inf, +inf]` is a maximal claim, not an absence (spec Q12)."""
    return {"lo": -_INF, "hi": _INF}


def measure(value, kind, *, basis, claim="modelled", per=None, interval=None) -> dict:
    """Wrap a computed signal as a declared measure. `value=None` means unknowable — and it stays
    `None` rather than becoming a sentinel number, so a naive `value > threshold` test raises
    instead of quietly reading an absence as a breach (spec Q13)."""
    if kind not in KINDS:
        raise ValueError(f"kind {kind!r} not in {KINDS}")
    if kind == "rate" and per not in PERIODS:
        raise ValueError(f"kind: rate requires per in {PERIODS} (got {per!r}) — a rate cannot "
                         f"forget its denominator")
    if interval is None:
        interval = unknown_interval() if value is None else {"lo": value, "hi": value}
    out = {
        "value": value,
        "kind": kind,
        "confidence": {"claim": claim, "interval": interval, "basis": basis},
    }
    if per:
        out["per"] = per
    return out


def is_unknown(interval: dict) -> bool:
    """Mirrors `Interval::is_unknown` INCLUDING the sign check it was missing (Q12)."""
    return interval["lo"] == -_INF and interval["hi"] == _INF


def ratio_of_rates(numerator: dict, denominator: dict, *, basis) -> dict:
    """Meadows' controllability index: problem growth rate ÷ response rate.

    Mirrors `elohim_epr_rea::stock::respite_response`, including its two refusals — flows of
    different tempo do not divide, and a zero (or unknown) denominator yields honest absence
    rather than infinity.

    > *"Any system in which the rate of growth of a problem is significantly faster than the rate
    > of response is, quite simply, out of control. There are only two ways to bring it back:
    > either quicken the response rate (if possible) or slow the growth rate of the problem."*

    There is no third option, and in particular *being more disciplined* is not one — that is an
    attempt to raise the response rate by will, which is the confuse-effort-with-result trap.
    """
    if numerator["kind"] != "rate" or denominator["kind"] != "rate":
        raise ValueError("respite/response divides two RATES; a level in either position is the "
                         "unit error this vocabulary exists to refuse")
    if numerator.get("per") != denominator.get("per"):
        raise ValueError(f"different periods ({numerator.get('per')} vs "
                         f"{denominator.get('per')}) — this module will not guess a conversion")
    num, den = numerator["value"], denominator["value"]
    if num is None or den is None or den == 0.0:
        return measure(None, "ratio", basis=basis, claim="modelled")
    return measure(num / den, "ratio", basis=basis, claim="modelled")
