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
import base64
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path


# A hung evaluator must never hang an author's keystroke.
_TIMEOUT_SECONDS = 10

# `epr bin status` costs ~2.3 s on a loaded host (measured 2026-10-30, load 54), and every hook
# that resolves the binary paid it on every call. That alone overran the 2 s and 3 s hook
# timeouts: 201 cancelled hooks in one session. A resolved path is content-addressed, so a
# cached one stays runnable. The TTL only bounds how long a newly landed pin goes unnoticed.
# The cache is keyed on the launcher and BIN_STORE, never on the store's layout; it holds
# positive answers only; it applies only to the real environment (env=None); and
# EPR_CLIENT_CACHE_TTL=0 turns it off.
_CACHE_TTL_SECONDS = 120


def _cache_file() -> Path:
    import tempfile
    return Path(tempfile.gettempdir()) / f"epr-client-resolve-{os.getuid()}.json"


def _cache_identity() -> dict:
    launcher = shutil.which("epr")
    return {"launcher": os.path.realpath(launcher) if launcher else None,
            "binStore": os.environ.get("BIN_STORE", "")}


def _cache_ttl() -> float:
    try:
        return float(os.environ.get("EPR_CLIENT_CACHE_TTL", _CACHE_TTL_SECONDS))
    except ValueError:
        return _CACHE_TTL_SECONDS


# Past the TTL, the stale path is still served (it is content-addressed and checked executable)
# while ONE detached refresher re-resolves it in the background. Without this, the first hook
# after each expiry paid the full lookup (3.4 s measured) and overran a 2 s hook timeout, as did
# every hook racing with it. A stale entry older than this bound is no longer served.
_CACHE_MAX_STALE_SECONDS = 86400


def _refresh_in_background() -> None:
    import time
    lock = _cache_file().with_name(_cache_file().name + ".refreshing")
    try:
        if lock.exists() and time.time() - lock.stat().st_mtime < 60:
            return  # a refresher is already running
        lock.write_text(str(os.getpid()))
        lib = str(Path(__file__).resolve().parents[1])
        subprocess.Popen(
            [os.environ.get("PYTHON", "python3"), "-c",
             "import sys; sys.path.insert(0, sys.argv[1]); from _lib import epr_client; "
             "epr_client._refresh_now(); import os; os.unlink(sys.argv[2])", lib, str(lock)],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
    except Exception:  # noqa: BLE001 — a failed refresh leaves the stale entry until the next try
        pass


def _refresh_now() -> None:
    """Re-resolve through `epr bin status` and rewrite the cache (the detached refresher)."""
    os.environ["EPR_CLIENT_CACHE_TTL"] = "0"  # bypass the cache read in this one process
    path, cid = resolve()
    if path and cid:
        os.environ.pop("EPR_CLIENT_CACHE_TTL", None)
        _store_resolution(path, cid)


def _cached_resolution() -> tuple[str, str] | None:
    import time
    ttl = _cache_ttl()
    if ttl <= 0:
        return None
    try:
        entry = json.loads(_cache_file().read_text())
        path, cid = entry["path"], entry["cid"]
        if entry.get("identity") != _cache_identity() or not os.access(path, os.X_OK):
            return None
        age = time.time() - entry["at"]
        if age < ttl:
            return path, cid
        if age < _CACHE_MAX_STALE_SECONDS:
            _refresh_in_background()
            return path, cid
    except Exception:  # noqa: BLE001 — a missing or torn cache is a miss, never an answer
        pass
    return None


def _store_resolution(path: str, cid: str) -> None:
    import time
    if _cache_ttl() <= 0:
        return
    try:
        target = _cache_file()
        tmp = target.with_name(f"{target.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps({"identity": _cache_identity(), "at": time.time(),
                                   "path": path, "cid": cid}))
        os.replace(tmp, target)
    except Exception:  # noqa: BLE001 — failing to cache only costs the next caller a lookup
        pass


def resolve(env: dict[str, str] | None = None) -> tuple[str | None, str | None]:
    """Resolve one executable path/CID pair, never the stable launcher itself."""
    try:
        environment = os.environ if env is None else env
        candidate = environment.get("EPR_BIN", "").strip()
        if candidate:
            return _candidate(candidate)
        if env is None:
            cached = _cached_resolution()
            if cached:
                return cached
        result = subprocess.run(
            ["epr", "bin", "status", "--tool", "epr", "--json"],
            env=dict(environment), capture_output=True, text=True, timeout=_TIMEOUT_SECONDS,
            check=False,
        )
        if result.returncode != 0:
            if _arrival_owed(result.returncode, environment):
                bootstrap(environment)
            return None, None
        status = json.loads(result.stdout)
        path, cid = status.get("resolved"), status.get("resolvedCid")
        # A release is addressed bafkrei… (one binary, raw) or bafyrei… (a DAG-CBOR sums manifest:
        # epr with bin-launch); anything else is a legacy sha256 identity, an absence.
        if isinstance(path, str) and isinstance(cid, str) and cid.startswith(("bafkrei", "bafyrei")):
            if env is None:
                _store_resolution(path, cid)
            return path, cid
        return None, None
    except Exception:  # noqa: BLE001 — resolution is best-effort; absence is a valid answer
        return None, None


def _arrival_owed(returncode: int, environment) -> bool:
    """Whether a failed resolution is a device that has not arrived, rather than an old release.

    Behind the native launcher a session may stay pinned to a release older than `epr bin`; it
    refuses the subcommand, and that is the session's pin, not a pre-native device. Only a
    legacy launcher, or a native one that cannot resolve at all (127), owes the bootstrap.
    """
    launcher = shutil.which("epr", path=environment.get("PATH"))
    native = launcher is not None and os.path.basename(os.path.realpath(launcher)) == "bin-launch"
    return not native or returncode == 127


def _candidate(value: str) -> tuple[str | None, str | None]:
    """An explicit `EPR_BIN` is the answer itself: never asked to read a store, never a bootstrap.

    The candidate is not executed to learn its identity (a test stub need not speak `bin
    status`), and a refused or absent candidate is an absence: it must not queue a bootstrap
    that ingests into the live device store from whatever checkout the caller runs in.
    """
    path = os.path.realpath(os.path.expanduser(value))
    if not os.path.isfile(path) or not os.access(path, os.X_OK):
        return None, None
    if os.path.basename(path) == "bin-launch":
        return None, None
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    # CIDv1, codec raw (0x55), sha2-256 (0x12, 32 bytes): the store's spelling of one binary.
    raw = bytes([0x01, 0x55, 0x12, 0x20]) + digest.digest()
    return path, "b" + base64.b32encode(raw).decode("ascii").lower().rstrip("=")


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
