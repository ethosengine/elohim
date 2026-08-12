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


# Mirrors `UnknownReason` (elohim/epr/src/measure.rs, serde kebab-case) — WHY an interval is
# unknown, spec Q17. Absence is not one thing, and this side of the mirror is where the
# distinction pays: `drift_score.measure` below is the repo's live `incommensurable`, and it must
# be excludable from any "which edge should we measure better?" queue by a machine, not by a
# reader noticing a paragraph in its basis string.
#
# ORDER IS LOAD-BEARING, not alphabetical taste: a list position here is the same tie-break
# `UnknownReason::stable_index` provides in Rust, and `reduce_reasons` below depends on the two
# agreeing. The vocabulary test asserts this sequence against the Rust enum body IN ORDER.
UNKNOWN_REASONS = ("no-observations", "not-yet-instrumented", "undefined-division",
                   "indeterminate-form", "zero-base", "malformed-input", "incommensurable")

# Mirrors `UnknownReason::tightenable`. Three responses, and the middle one is the trap: a
# zero-crossing denominator LOOKS like missing data and is not.
TIGHTENABLE = {
    "no-observations": "by-measurement",
    "not-yet-instrumented": "by-measurement",
    "undefined-division": "by-structure",
    "indeterminate-form": "by-structure",
    "zero-base": "by-structure",
    "malformed-input": "by-structure",
    "incommensurable": "never",
}


def tightenable(reason: str) -> str:
    """`by-measurement` (queue it) | `by-structure` (queue it as a MODEL fix, never as 'sample
    harder') | `never` (must not enter the queue at all)."""
    if reason not in TIGHTENABLE:
        raise ValueError(f"unknown reason {reason!r} not in {UNKNOWN_REASONS}")
    return TIGHTENABLE[reason]


_STUCKNESS = {"by-measurement": 0, "by-structure": 1, "never": 2}


def reduce_reasons(reasons) -> str | None:
    """Mirrors `UnknownReason::reduce` — the ONE rule for combining reasons.

    **Least tightenable wins; ties break on position in `UNKNOWN_REASONS`.** A derived quantity is
    no more improvable than the worst thing that fed it, so an `incommensurable` operand makes the
    result incommensurable no matter what the operation's own guard found. The tie-break is
    arbitrary in meaning and load-bearing in effect: `tightenable()` maps seven reasons onto three
    responses, so distinct reasons TIE, and without a total order the answer would depend on
    argument order — two callers reducing the same multiset would emit different measures.

    `None` in, `None` out: nothing contributed a reason, so nothing is claimed. Never invent one.
    """
    present = [r for r in reasons if r is not None]
    for r in present:
        if r not in TIGHTENABLE:
            raise ValueError(f"unknown reason {r!r} not in {UNKNOWN_REASONS}")
    if not present:
        return None
    return max(present, key=lambda r: (_STUCKNESS[TIGHTENABLE[r]], UNKNOWN_REASONS.index(r)))


def unknown_interval() -> dict:
    """`Interval::unknown()` — the interval that admits everything. Honest absence, and note the
    SIGN: `[+inf, +inf]` is a maximal claim, not an absence (spec Q12).

    Unchanged, and deliberately so: the reason rides on the `confidence` block beside the
    interval, never inside it, because the Rust `Interval` is hash-bearing (spec Q17)."""
    return {"lo": -_INF, "hi": _INF}


def measure(value, kind, *, basis, claim="modelled", per=None, interval=None,
            unknown_reason=None) -> dict:
    """Wrap a computed signal as a declared measure. `value=None` means unknowable — and it stays
    `None` rather than becoming a sentinel number, so a naive `value > threshold` test raises
    instead of quietly reading an absence as a breach (spec Q13).

    `unknown_reason` (spec Q17) says WHY the interval is unknown. It is omitted from the emitted
    dict when absent — same `skip_serializing_if` shape as the Rust `Confidence.unknownReason`,
    so a measure that carries no reason serializes byte-for-byte as it always did. Refused on a
    BOUNDED interval, because "here are the bounds, and here is why there are none" is not a
    thing anyone means."""
    if kind not in KINDS:
        raise ValueError(f"kind {kind!r} not in {KINDS}")
    if kind == "rate" and per not in PERIODS:
        raise ValueError(f"kind: rate requires per in {PERIODS} (got {per!r}) — a rate cannot "
                         f"forget its denominator")
    if interval is None:
        interval = unknown_interval() if value is None else {"lo": value, "hi": value}
    confidence = {"claim": claim, "interval": interval, "basis": basis}
    if unknown_reason is not None:
        if unknown_reason not in UNKNOWN_REASONS:
            raise ValueError(f"unknown_reason {unknown_reason!r} not in {UNKNOWN_REASONS}")
        if not is_unknown(interval):
            raise ValueError("unknown_reason explains an ABSENCE; this interval has bounds")
        confidence["unknownReason"] = unknown_reason
    out = {
        "value": value,
        "kind": kind,
        "confidence": confidence,
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

    **Mint-site parity (spec Q17).** This is the module's only arithmetic that can lose its
    bounds, so it is the only place a reason gets minted, and it mints the same one
    `Interval::div_because` + `Reasoned::attributed_to` would:

    - a denominator that is zero **or absent** → `undefined-division`. Both are a denominator band
      that admits zero (an unmeasured rate's band is `(-inf, +inf)`, a witnessed zero's is
      `[0,0]`), which is exactly `div_because`'s guard. `by-structure` reads correctly here — its
      remedy is "the model, the instrument, or an upstream operand must change first", and with no
      response rate at all that is precisely the work. Sampling the numerator harder is not.
    - a numerator that is absent, over a good denominator → `no-observations`. Rust mints nothing
      at this point (`unknown / [3,3]` trips no guard) and relies on the operand's own declared
      reason; Python has the extra bit Rust's bare interval threw away — a computed rate that
      returned `value: None` had its instrument and got nothing — so it declares the default
      rather than emitting the unclassified unknown that started this. An operand that knows
      better says so, and `reduce_reasons` prefers whichever is less tightenable.
    - operands' own `confidence.unknownReason` are threaded in either way, reduced against the
      guard by the shared least-tightenable rule. No precedence, one reduction.
    """
    if numerator["kind"] != "rate" or denominator["kind"] != "rate":
        raise ValueError("respite/response divides two RATES; a level in either position is the "
                         "unit error this vocabulary exists to refuse")
    if numerator.get("per") != denominator.get("per"):
        raise ValueError(f"different periods ({numerator.get('per')} vs "
                         f"{denominator.get('per')}) — this module will not guess a conversion")
    num, den = numerator["value"], denominator["value"]
    operand_reasons = [numerator.get("confidence", {}).get("unknownReason"),
                       denominator.get("confidence", {}).get("unknownReason")]
    if den is None or den == 0.0:
        guard = "undefined-division"
    elif num is None:
        guard = "no-observations"
    else:
        # Bounded both sides: no absence to explain, and an operand reason cannot apply because a
        # reason only ever accompanies an unknown interval.
        return measure(num / den, "ratio", basis=basis, claim="modelled")
    return measure(None, "ratio", basis=basis, claim="modelled",
                   unknown_reason=reduce_reasons([guard, *operand_reasons]))
