"""Client for the native governance evaluator (`epr govern`).

THE COLLAPSE, AND WHY IT IS NOT A DELETION. The repo ran two governance
evaluators that never met: the Python one (live, in the PreToolUse hook and the
git gate) and the Rust one (`epr`, parity-tested and invoked by nothing). Four
divergences survived in that gap — two making Rust more blocking than Python,
two less — and none of them could surface, because agreeing in a test suite is
not the same as running.

This module makes the Rust evaluator the DECISION AUTHORITY at both surfaces
while keeping the Python evaluator running beside it. That is deliberate, and it
is not a half-measure:

  * DISAGREEMENT IS THE ARTIFACT. Agreement between two evaluators is the
    trivial half of the claim; what makes the plane trustworthy is that a second
    host can re-derive a decision and record a dispute BY NAME. Running only one
    evaluator would delete the dispute path and, with it, the only mechanism by
    which the next divergence gets noticed. Disagreements are witnessed to the
    governance ledger with both verdicts named.

  * ADVICE IS HOST-LOCAL BY DECLARATION. Three validators are scoped `python` in
    governance-validators.json (heal-fills-never-moves, bounded-work,
    dna-hash-neutrality — all class `inject`, all firing on elohim-storage/src).
    They are Python's to run by design, not by accident. A decision-only
    collapse would silently drop their advisories; single decision authority
    must not quietly become less advice.

FALLBACK POLICY. When `epr` cannot be resolved the Python evaluator decides, and
the degradation is announced rather than swallowed. This satisfies the polarity
law — the gate never SILENTLY permits when governance could not be evaluated —
without betting the authoring loop on a build artifact: `epr` is not on PATH by
default, it is compiled into a branch-scoped cargo-pool slot, so "refer when the
binary is missing" would fire on essentially every governed write rather than in
some rare outage. Referring is reserved for the case where NEITHER host can
answer, which is the genuine absence the law is about.
"""
import json
import os
import shutil
import subprocess
from pathlib import Path

# A hung evaluator must never hang an author's keystroke.
_TIMEOUT_SECONDS = 10

# Where a compiled `epr` plausibly lives, in resolution order. `EPR_BIN` wins so
# an operator (or CI) can pin one explicitly; PATH is next; the cargo target pool
# is the local-dev reality, globbed rather than derived because the family
# segment is branch-scoped and a wrong guess is indistinguishable from absence.
_POOL_GLOBS = (
    "/projects/.cargo-target-pool/family/*/elohim__eprfs/*/release/epr",
    "/projects/.cargo-target-pool/family/*/elohim__eprfs/*/debug/epr",
)


def resolve_binary() -> str | None:
    """Path to a usable `epr`, or None. Never raises."""
    try:
        pinned = os.environ.get("EPR_BIN")
        if pinned and Path(pinned).is_file() and os.access(pinned, os.X_OK):
            return pinned
        found = shutil.which("epr")
        if found:
            return found
        newest, newest_mtime = None, -1.0
        for pattern in _POOL_GLOBS:
            root = Path(pattern.split("*")[0])
            if not root.is_dir():
                continue
            for candidate in root.parent.glob(pattern[len(str(root.parent)) + 1:]):
                try:
                    if not (candidate.is_file() and os.access(candidate, os.X_OK)):
                        continue
                    mtime = candidate.stat().st_mtime
                except OSError:
                    continue
                if mtime > newest_mtime:
                    newest, newest_mtime = str(candidate), mtime
        return newest
    except Exception:  # noqa: BLE001 — resolution is best-effort; absence is a valid answer
        return None


def govern(root: Path, path: str, content, is_new: bool, is_new_subdir: bool, *,
           session: str | None = None) -> dict | None:
    """Ask the native evaluator about a PROSPECTIVE write.

    Returns the decision payload, or None when the evaluator did not run. The
    distinction is load-bearing: `epr govern` puts its decision in the payload
    and reserves a nonzero exit for "the evaluator did not run", precisely so a
    client can tell a refusal from an absence. Collapsing the two is how a gate
    ends up permitting whenever its evaluator breaks.
    """
    binary = resolve_binary()
    if not binary:
        return None
    args = [binary, "govern", "--repo", str(root), "--path", path]
    if is_new:
        args.append("--new")
    if is_new_subdir:
        args.append("--new-subdir")
    if content is not None:
        args.append("--content-stdin")
    if session:
        # A stale `epr` that doesn't know --session exits nonzero, which the
        # returncode != 0 -> None path below already treats as absence — deliberate degradation.
        args.extend(["--session", session])
    try:
        proc = subprocess.run(
            args,
            input=content if content is not None else "",
            capture_output=True,
            text=True,
            timeout=_TIMEOUT_SECONDS,
        )
    except Exception:  # noqa: BLE001 — timeout / spawn failure is an absence, not a verdict
        return None
    if proc.returncode != 0:
        return None
    try:
        return json.loads(proc.stdout)
    except Exception:  # noqa: BLE001 — unparseable output is an absence
        return None


def disagreement(py_result: dict, rs_payload: dict) -> str | None:
    """Describe a DECISION-level dispute between the two hosts, else None.

    Only the decision is compared. Differences in winning class or rule id when
    the decision matches are the DECLARED asymmetry, not drift: a validator
    scoped to one host produces a verdict there and none here, so the permitting
    host attributes the permit to no rule. Both still permit. Reporting that as
    a dispute would cry wolf on the one behaviour the scope registry exists to
    make legitimate — and an alarm that cries wolf gets deleted, not obeyed.
    """
    py_decision = py_result.get("decision")
    rs_decision = rs_payload.get("decision")
    if py_decision == rs_decision:
        return None
    return (
        f"python={py_decision} (class={py_result.get('cls')}, rule={py_result.get('rule_id')}) "
        f"vs rust={rs_decision} (class={rs_payload.get('winningClass')}, "
        f"rule={rs_payload.get('ruleId')})"
    )


DEGRADED_NOTICE = (
    "[.epr-meta] the native evaluator (`epr`) could not be resolved — this decision came from "
    "the Python evaluator alone, so the cross-host check did not run. Build it "
    "(`cargo build -p elohim-epr-cli` in elohim/eprfs) or set EPR_BIN. Governance was still "
    "evaluated; what is missing is the second opinion."
)
