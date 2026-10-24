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
without betting the authoring loop on a mutable build artifact: the stable PATH
launcher resolves a content-addressed store and the cargo pool is never a
fallback. `EPR_BIN` remains only the explicit candidate-under-test override.
Referring is reserved for the case where NEITHER host can answer, which is the
genuine absence the law is about.
"""
import json
import os
import subprocess
from pathlib import Path


# A hung evaluator must never hang an author's keystroke.
_TIMEOUT_SECONDS = 10


def resolve(env: dict[str, str] | None = None) -> tuple[str | None, str | None]:
    """Resolve one executable path/CID pair, never the stable launcher itself."""
    try:
        environment = os.environ if env is None else env
        result = subprocess.run(
            [environment.get("EPR_BIN") or "epr", "bin", "status", "--tool", "epr", "--json"],
            env=dict(environment), capture_output=True, text=True, timeout=_TIMEOUT_SECONDS,
            check=False,
        )
        if result.returncode != 0:
            bootstrap(environment)
            return None, None
        status = json.loads(result.stdout)
        path, cid = status.get("resolved"), status.get("resolvedCid")
        if isinstance(path, str) and isinstance(cid, str) and cid.startswith("bafkrei"):
            return path, cid
        return None, None
    except Exception:  # noqa: BLE001 — resolution is best-effort; absence is a valid answer
        return None, None


def bootstrap(environment: dict[str, str]) -> None:
    """Queue the shared pre-native worker without retaining the evaluator's pipes."""
    global DEGRADED_NOTICE
    DEGRADED_NOTICE = (
        "[.epr-meta] native evaluator bootstrapping — the Python evaluator answers alone "
        "until the native store is ready; the cross-host check did not run."
    )
    root = Path(__file__).resolve().parents[3]
    subprocess.Popen(
        ["node", str(root / "genesis/agentic/native-epr-bootstrap.mjs"), str(root)],
        env=dict(environment), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, start_new_session=True,
    )


def resolve_binary(env: dict[str, str] | None = None) -> str | None:
    """Compatibility accessor for callers that need only the path."""
    return resolve(env)[0]


def resolved_cid(env: dict[str, str] | None = None) -> str | None:
    """Compatibility accessor for callers that need only the CID."""
    return resolve(env)[1]


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


def covenant(root: Path, verb: str, session: str | None, *args: str) -> dict:
    """`epr covenant <verb> --repo R --session S … --json`, the session covenant's native reader.

    Transport only: whether a clause is honoured (standing, paths, expiry, revocation, the rule's
    current bytes) is decided by `epr`, never re-derived here. An absent binary, a stale `epr`
    without the subcommand, a timeout or unparseable output all return
    `{"status": "unavailable", "reason": …}`, and callers apply the full rule: a covenant that
    cannot be read never lowers an obligation.
    """
    if not session:
        return {"status": "unavailable", "reason": "no session id"}
    binary = resolve_binary()
    if not binary:
        return {"status": "unavailable", "reason": "`epr` not found"}
    argv = [binary, "covenant", verb, "--repo", str(root), "--session", session, *args, "--json"]
    try:
        proc = subprocess.run(argv, capture_output=True, text=True,
                              timeout=_COVENANT_TIMEOUT_SECONDS)
    except Exception as e:  # noqa: BLE001 — timeout / spawn failure is an absence
        return {"status": "unavailable", "reason": type(e).__name__}
    if proc.returncode != 0:
        first = (proc.stderr or proc.stdout).strip().splitlines()[:1]
        return {"status": "unavailable", "reason": first[0][:160] if first else f"exit {proc.returncode}"}
    try:
        body = json.loads(proc.stdout)
    except Exception:  # noqa: BLE001
        return {"status": "unavailable", "reason": "unparseable `epr covenant` output"}
    return body if isinstance(body, dict) else {"status": "unavailable", "reason": "non-object output"}


# Covenant reads sit on the author's keystroke and on SessionStart/SubagentStart, beside the 0.8 s
# actor read — not on govern's 10 s budget.
_COVENANT_TIMEOUT_SECONDS = 1.5


DEGRADED_NOTICE = (
    "[.epr-meta] the native evaluator (`epr`) could not be resolved — this decision came from "
    "the Python evaluator alone, so the cross-host check did not run. Build it "
    "(`cargo build -p elohim-epr-cli` in elohim/eprfs) or set EPR_BIN. Governance was still "
    "evaluated; what is missing is the second opinion."
)
