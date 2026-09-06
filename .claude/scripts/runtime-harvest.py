#!/usr/bin/env python3
"""runtime-harvest — EXTERNAL elevate-arm poller (findings-sentinel pattern,
instantiation D). Reads each alpha node's admin read-endpoints, appends one
sample per poll to a per-node ring buffer, evaluates pure exhaustion predicates
over the window, and files NEW findings to .claude/data/runtime-findings.jsonl
+ dispatches a background runtime-triage agent. Mirrors ci-harvest.py.

THE NO-RUNTIME-WRITE RULE: runtime Rust NEVER writes .claude/data. This
external poller READS endpoints and WRITES the ledger. Zero Rust here.

Endpoints (per node, degrade-quiet — a missing one contributes no fields):
  GET /admin/self-healing   PRIMARY (PENDING — stability-surface plan C)
  GET /admin/render-stats   SECONDARY (LANDED)
  GET /admin/residuals      C14 residual capsules (PENDING — seam-concern plan P4.7)
  GET /p2p/status           dataplane self-report: provideLoop + replication (LANDED)
  GET /health               liveness

Stores:
  .claude/data/runtime-cursor.json    {poll_index, windows:{node:[sample,...]}}
  .claude/data/runtime-findings.jsonl one line per LIVE finding:
      {ts, fp, class, node, provenance, line, status,
       seen, first_poll, last_poll, backlog?}  (closure = DELETION, D5)

TWO CLASSES ON ONE LEDGER (seam-concern-contract plan P4.7, canon row C14). `class` was always an
opaque ledger field — `rh.fingerprint` takes it as a parameter and `rh.reconcile` keys on `fp`
alone — so the residual channel BINDS this pipeline instead of forking it, and the pure core
(_lib/runtime_harvest.py) needed ZERO change. Proven by fixture, not asserted:
_lib/__tests__/residual_channel_test.py reconciles a c14-class finding through the unmodified
core. What DID need changing is right here in the shell: `render` used to hardcode the class name
in its dispatch directive, which would have described a witnessed residual as a self-reported
exhaustion and sent runtime-triage hunting a circuit breaker that never opened — the harvester
committing C4 (honest absence) and C7 (advertise/serve) against its own findings.
  • self-heal-exhaustion                     — window predicates over /admin/self-healing + /admin/render-stats
  • concern:c14-witnessed-residual           — capsules served at /admin/residuals (see _lib/residual_channel.py)
  • provide-loop-dead-remaining-stuck        — /p2p/status provideLoop.deadRemainingStuck (see p2p_status_findings below;
                                                the persistence — 3 unchanged reanchor sweeps — is already computed
                                                server-side by elohim-storage's provide_loop_status.rs, so this classifier
                                                relays a self-report rather than re-deriving the window)
  • replication-caughtup-false-sustained     — /p2p/status replication.caughtUp == false for
                                                REPLICATION_LAG_POLLS consecutive polls (mirrors the ring-buffer /
                                                consecutive-observation pattern _lib/runtime_harvest.py already uses
                                                for projector lag; kept in this shell file rather than the pure core
                                                so multi-doorway B-side coverage lands without touching _lib)

Nodes (dict, node -> base URL): "alpha" -> doorway-alpha (storage peer matthew, A-side) and
"alpha-b" -> elohim.host (storage peer adam, B-side) — added so the B-side doorway is polled too;
fingerprints already key on `node`, so this never re-fires existing "alpha" entries.

Modes:
  (default)  poll all NODES, append sample, reconcile; human summary
  --hook     same; emit SessionStart-hook JSON (silent when nothing new)
  --nodes a,b  restrict to listed nodes
  --base URL   override the node base URL template (test/dev)

Fail-safe: in --hook mode every error exits 0 — a node outage must never
break session start.

Boundary (by design): this is the ELEVATE arm for SELF-REPORTED exhaustion —
a node serving /admin/self-healing that says "my circuit is open / I'm
shedding / projector can't catch up". A TOTAL outage (runtime wedged, nginx
503, host unreachable) returns no JSON to sample, so poll_node degrades quiet
and files NOTHING — a dead node cannot self-report. Catching total-down is
external uptime/liveness monitoring, a different system; it is intentionally
out of scope here.
"""
import argparse
import fcntl
import json
import os
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _lib import residual_channel as rc  # noqa: E402
from _lib import runtime_harvest as rh  # noqa: E402

# node -> base URL, explicit (not templated) because the B-side doorway does not follow the
# "doorway-{node}" convention. A/B are separate storage peers/doorways (matthew, adam) with no
# shared coherence today (project_doorway_ops_incidents) — the poller must see both to catch an
# exhaustion the other side can't observe.
NODE_BASES = {
    "alpha": "https://doorway-alpha.elohim.host",     # A-side: storage peer matthew
    "alpha-b": "https://elohim.host",                 # B-side: storage peer adam
}
NODES = list(NODE_BASES)  # extend per cluster-state
BASE_TMPL = "https://doorway-{node}.elohim.host"  # fallback for a node not in NODE_BASES
ENDPOINTS = {
    "self_healing": "/admin/self-healing",
    "render": "/admin/render-stats",
    "residuals": "/admin/residuals",
    "p2p_status": "/p2p/status",
    "health": "/health",
}
HTTP_TIMEOUT = 8

PROJECT = os.environ.get("CLAUDE_PROJECT_DIR") or os.path.dirname(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
)
CURSOR_PATH = os.path.join(PROJECT, ".claude", "data", "runtime-cursor.json")
LEDGER_PATH = os.path.join(PROJECT, ".claude", "data", "runtime-findings.jsonl")


def get_json(url):
    """Fetch JSON; None on ANY failure (degrade quiet — D3)."""
    try:
        req = urllib.request.Request(url, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
            return json.loads(resp.read().decode("utf-8", "replace"))
    except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError,
            OSError, ValueError):
        return None


def poll_node(node, base):
    """One poll: fetch every endpoint, merge into ONE sample. Returns the
    sample dict, or None if the node is wholly unreachable (skip its append)."""
    sample, reached = {}, False
    sh = get_json(base + ENDPOINTS["self_healing"])
    if isinstance(sh, dict):
        reached = True
        for k in ("upstreams", "admission", "projector"):
            if k in sh:
                sample[k] = sh[k]
        if isinstance(sh.get("render"), dict):
            sample["render"] = sh["render"]
    rs = get_json(base + ENDPOINTS["render"])
    if isinstance(rs, dict):
        reached = True
        sample["render"] = rs  # render-stats is authoritative for render
    # C14 capsules are EVENTS, not window state: they live under a key `harvest` pops before the
    # sample joins the ring buffer, so 8 polls of capsules never accumulate in the cursor.
    res = get_json(base + ENDPOINTS["residuals"])
    if isinstance(res, list):
        reached = True
        sample["residuals"] = res
    ps = get_json(base + ENDPOINTS["p2p_status"])
    if isinstance(ps, dict):
        reached = True
        sample["p2p_status"] = ps
    h = get_json(base + ENDPOINTS["health"])
    if isinstance(h, dict):
        reached = True
    return sample if reached else None


PROVIDE_LOOP_CLASS = "provide-loop-dead-remaining-stuck"
REPLICATION_LAG_CLASS = "replication-caughtup-false-sustained"
REPLICATION_LAG_POLLS = 3  # consecutive-observation window, mirrors rh.LAG_POLLS' pattern


def _provide_loop_stuck_finding(node, samples):
    """Self-heal exhaustion: elohim-storage's provide-loop watchdog already computed
    `deadRemainingStuck` after >= 3 unchanged reanchor sweeps
    (elohim/elohim-storage/src/services/provide_loop_status.rs:57-68,130-132) — the persistence
    happened server-side, so relaying the latest sample is sufficient; no window needed here."""
    if not samples:
        return None
    ps = samples[-1].get("p2p_status")
    pl = ps.get("provideLoop") if isinstance(ps, dict) else None
    if not isinstance(pl, dict) or pl.get("deadRemainingStuck") is not True:
        return None
    return {
        "node": node, "class": PROVIDE_LOOP_CLASS,
        "provenance": "p2p-status:provide-loop",
        "line": f"provideLoop.deadRemainingStuck reanchorDeadRemaining="
                f"{pl.get('reanchorDeadRemaining')} reanchorPending={pl.get('reanchorPending')} "
                f"stuckSweeps={pl.get('stuckSweeps')}",
    }


def _replication_caughtup_finding(node, samples):
    """Sustained replication.caughtUp == false across REPLICATION_LAG_POLLS consecutive polls.
    Uses the same ring-buffer / consecutive-observation notion _lib/runtime_harvest.py's
    predicates already rely on (window kept in cursor.windows[node]); kept here rather than in
    the pure core so B-side coverage lands without an _lib change."""
    win = samples[-REPLICATION_LAG_POLLS:] if len(samples) >= REPLICATION_LAG_POLLS else []
    if not win:
        return None
    states = []
    for s in win:
        ps = s.get("p2p_status")
        rep = ps.get("replication") if isinstance(ps, dict) else None
        states.append(isinstance(rep, dict) and rep.get("caughtUp") is False)
    if not all(states):
        return None
    return {
        "node": node, "class": REPLICATION_LAG_CLASS,
        "provenance": "p2p-status:replication",
        "line": f"replication.caughtUp == false sustained >= {REPLICATION_LAG_POLLS} polls",
    }


def p2p_status_findings(node, samples):
    """PURE-ish (reads only the sample window, no I/O): exhaustion predicates over /p2p/status,
    combined the same way rh.evaluate() combines its predicates."""
    findings = []
    for pred in (_provide_loop_stuck_finding, _replication_caughtup_finding):
        f = pred(node, samples)
        if f is not None:
            findings.append(f)
    return findings


def load_cursor():
    try:
        with open(CURSOR_PATH, encoding="utf-8") as fh:
            c = json.load(fh)
            c.setdefault("poll_index", 0)
            c.setdefault("windows", {})
            return c
    except (OSError, json.JSONDecodeError):
        return {"poll_index": 0, "windows": {}}


def load_jsonl(path):
    out = []
    if os.path.exists(path):
        with open(path, encoding="utf-8", errors="replace") as fh:
            for raw in fh:
                try:
                    out.append(json.loads(raw))
                except json.JSONDecodeError:
                    continue
    return out


def write_jsonl(path, entries):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        for e in entries:
            fh.write(json.dumps(e, ensure_ascii=False) + "\n")
    os.replace(tmp, path)


def harvest(nodes, base_override, as_hook):
    os.makedirs(os.path.dirname(LEDGER_PATH), exist_ok=True)
    lock = open(LEDGER_PATH + ".lock", "w")  # noqa: SIM115 — held for fn lifetime
    fcntl.flock(lock, fcntl.LOCK_EX)
    try:
        cursor = load_cursor()
        cursor["poll_index"] = cursor.get("poll_index", 0) + 1
        poll_index = cursor["poll_index"]
        entries = load_jsonl(LEDGER_PATH)
        active = []
        for node in nodes:
            base = base_override or NODE_BASES.get(node) or BASE_TMPL.format(node=node)
            sample = poll_node(node, base)
            if sample is None:
                continue  # wholly unreachable -> no append, no finding (D3)
            capsules = sample.pop("residuals", [])
            win = cursor["windows"].setdefault(node, [])
            win.append(sample)
            del win[: -rh.WINDOW]  # keep last WINDOW samples
            # Three producers, ONE fingerprint/reconcile core (C14 binds this pipeline, D5 closure
            # and the storm guard apply to residuals + p2p-status findings unchanged).
            produced = (rh.evaluate({"node": node, "samples": win})
                        + rc.findings_from_capsules(node, capsules)
                        + p2p_status_findings(node, win))
            for f in produced:
                f["fp"] = rh.fingerprint(f["node"], f["class"], f["provenance"])
                active.append(f)
        new, bumped, closed = rh.reconcile(entries, active, poll_index)
        for e in new:  # stamp wall-clock first-capture time
            e.setdefault("ts", datetime.now(timezone.utc).isoformat(timespec="seconds"))
        write_jsonl(LEDGER_PATH, entries)   # ledger BEFORE cursor (crash-safe)
        os.makedirs(os.path.dirname(CURSOR_PATH), exist_ok=True)
        tmp = CURSOR_PATH + ".tmp"
        with open(tmp, "w", encoding="utf-8") as fh:
            json.dump(cursor, fh, indent=1)
        os.replace(tmp, CURSOR_PATH)
    finally:
        fcntl.flock(lock, fcntl.LOCK_UN)
        lock.close()
    return new, bumped, closed


def _by_class(entries):
    """Group ledger entries by `class`, first-seen order. The ledger carries more than one class
    (D5 + P4.7), and ONE dispatch directive naming the wrong class is worse than two naming the
    right ones — a triage agent told "self-reported exhaustion" will look for a circuit breaker
    that never opened."""
    groups: dict[str, list] = {}
    for e in entries:
        groups.setdefault(e.get("class", "?"), []).append(e)
    return groups


def render(new, bumped, closed, as_hook):
    parts, sys_parts = [], []
    for cls, group in _by_class(new).items():
        fps = ", ".join(e["fp"] for e in group)
        lines = " | ".join(f'{e["fp"]} [{e["provenance"]}] {e["node"]}: "{e["line"][:80]}"'
                           for e in group[:5])
        parts.append(
            f"[runtime-harvest] {len(group)} NEW {cls} finding(s) "
            f"captured to .claude/data/runtime-findings.jsonl — {lines}. "
            f"DISPATCH (do not derail the current task): launch the `runtime-triage` "
            f"agent via the Agent tool with run_in_background: true and the prompt "
            f"'Triage runtime ledger fingerprint(s) {fps} (class {cls}) per your agent "
            f"definition (.claude/agents/runtime-triage.md). {rc.framing_for(cls)}' "
            f"Fall back to general-purpose with the same prompt if the "
            f"type is unavailable. Then continue your current task."
        )
        sys_parts.append(f"+{len(group)} new {cls} finding(s) -> runtime-triage dispatch")
    for label, bucket in (("recurred", bumped), ("self-resolved (closed by disappearance)", closed)):
        for cls, group in _by_class(bucket).items():
            sys_parts.append(f"{len(group)} known {cls} {label}")
    if not parts and not sys_parts:
        return None
    if as_hook:
        return json.dumps({
            "systemMessage": "runtime-harvest: " + "; ".join(sys_parts),
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": " ".join(parts) if parts else "; ".join(sys_parts),
            },
        })
    out = []
    if sys_parts:
        out.append("; ".join(sys_parts))
    out.extend(parts)
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--hook", action="store_true", help="SessionStart-hook JSON output")
    ap.add_argument("--nodes", help="comma-separated node subset")
    ap.add_argument("--base", help="base URL override (test/dev)")
    args = ap.parse_args()
    nodes = [n.strip() for n in args.nodes.split(",")] if args.nodes else NODES
    new, bumped, closed = harvest(nodes, args.base, args.hook)
    rendered = render(new, bumped, closed, args.hook)
    if rendered:
        print(rendered)
    elif not args.hook:
        print("runtime-harvest: nothing new")


if __name__ == "__main__":
    if "--hook" in sys.argv:
        try:
            main()
        except BaseException:  # noqa: BLE001 — never break session start
            pass
        sys.exit(0)
    main()
