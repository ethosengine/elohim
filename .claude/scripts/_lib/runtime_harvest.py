"""runtime_harvest — pure core of the elevate-arm runtime poller (findings-
sentinel pattern, instantiation D). NO I/O, NO network: exhaustion predicates
(`evaluate`) over a persisted sample window + ledger `reconcile`. The thin I/O
shell is .claude/scripts/runtime-harvest.py. NEVER imported by runtime Rust —
the no-runtime-write rule means only this external poller touches .claude/data."""
import hashlib
import re

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")

# ── exhaustion thresholds (rationale in the plan Decisions / Task 3) ──
OPEN_POLLS = 3          # circuit Open across >= N consecutive polls
SHED_POLLS = 3          # admission/upstream shed delta > 0 across >= N polls
LAG_POLLS = 3           # projector caughtUp=false / lag rising across >= N polls
LAG_SECONDS = 30        # projector lagSeconds threshold (seconds)
DEGEN_RATE = 0.25       # render degenerateRate sustained threshold
DEGEN_POLLS = 3         # ... across >= N consecutive polls
WINDOW = 8              # ring-buffer length per node (>= max predicate window)
CLASS = "self-heal-exhaustion"

CLOSE_STREAK = 3        # fp absent for >= N consecutive polls => closed (deleted)
MAX_NEW_FINDINGS = 12   # ledger appends per run (storm guard, mirror ci-harvest)


def normalize(line):
    """Build-invariant normalization so poll-count / duration / timestamp churn
    produces ONE fingerprint (mirror ci-harvest.normalize)."""
    line = ANSI.sub("", line)
    line = re.sub(r"\s+", " ", line).strip()
    line = re.sub(r"\b\d{1,3}(?:\.\d{1,3}){3}\b", "#", line)          # IPv4
    line = re.sub(r"\b\d+(?:\.\d+)?(?:ms|s|m|h)\b", "#", line)        # durations
    line = re.sub(r"\b20\d{2}-\d{2}-\d{2}[T ][\d:.]+\S*", "#", line)  # timestamps
    line = re.sub(r"\b\d+\b", "#", line)                              # bare counts/polls
    return line[:300]


def fingerprint(node, cls, provenance):
    """fp(node + class + provenance), 12-hex. node/class are developer
    vocabulary (lowered); provenance is normalized for count-churn invariance."""
    norm = f"{node.lower()}|{cls.lower()}|{normalize(provenance)}"
    return hashlib.sha256(norm.encode()).hexdigest()[:12]


def _tail(samples, n):
    """Last n samples (the predicate window). Fewer than n => empty (a predicate
    needs n consecutive observations before it can fire)."""
    return samples[-n:] if len(samples) >= n else []


def _render_degenerate(node, samples):
    """SSR saturation: degenerateRate >= DEGEN_RATE for >= DEGEN_POLLS
    consecutive polls. degenerateRate = (stalled+timedOut)/total, the LANDED
    /admin/render-stats headline (stats.rs:50). Field-presence-tolerant."""
    win = _tail(samples, DEGEN_POLLS)
    rates = [s["render"]["degenerateRate"] for s in win
             if isinstance(s.get("render"), dict) and "degenerateRate" in s["render"]]
    if len(rates) < DEGEN_POLLS or any(r < DEGEN_RATE for r in rates):
        return None
    worst = max(rates)
    return {
        "node": node,
        "class": CLASS,
        "provenance": "render-degenerate",
        "line": f"render.degenerateRate {worst:.2f} sustained >= {DEGEN_POLLS} polls "
                f"(SSR stalled/timedOut saturation)",
    }


def _circuit_open(node, samples):
    """An upstream circuit Open for OPEN_POLLS consecutive polls. Until
    /admin/self-healing lands, `upstreams` is absent => no signal. Per-endpoint:
    fires for any endpoint open across the whole window."""
    win = _tail(samples, OPEN_POLLS)
    if len(win) < OPEN_POLLS:
        return None
    endpoints = {u.get("endpoint", "?")
                 for s in win for u in (s.get("upstreams") or [])}
    for ep in endpoints:
        states = []
        for s in win:
            ups = {u.get("endpoint"): u for u in (s.get("upstreams") or [])}
            u = ups.get(ep, {})
            # circuit field is authoritative; fall back to errorStreak+skipped
            if "circuit" in u:
                states.append(u["circuit"] == "open")
            else:
                states.append(bool(u.get("skipped")) and u.get("errorStreak", 0) > 0)
        if len(states) == OPEN_POLLS and all(states):
            return {
                "node": node, "class": CLASS, "provenance": f"circuit:{ep}",
                "line": f"upstream {ep} circuit Open >= {OPEN_POLLS} consecutive polls",
            }
    return None


def _admission_shed(node, samples):
    """Sustained shed-storm: shedTotal strictly rising across SHED_POLLS, OR
    available==0 sustained. Absent `admission` => no signal."""
    win = _tail(samples, SHED_POLLS)
    sheds = [s["admission"]["shedTotal"] for s in win
             if isinstance(s.get("admission"), dict) and "shedTotal" in s["admission"]]
    if len(sheds) == SHED_POLLS and all(b > a for a, b in zip(sheds, sheds[1:])):
        return {"node": node, "class": CLASS, "provenance": "admission-shed",
                "line": f"admission.shedTotal rising {sheds[0]}->{sheds[-1]} over "
                        f"{SHED_POLLS} polls (shed-storm)"}
    avail = [s["admission"]["available"] for s in win
             if isinstance(s.get("admission"), dict) and "available" in s["admission"]]
    if len(avail) == SHED_POLLS and all(a == 0 for a in avail):
        return {"node": node, "class": CLASS, "provenance": "admission-shed",
                "line": f"admission.available == 0 sustained >= {SHED_POLLS} polls"}
    return None


def _projector_lag(node, samples):
    """Projector not caught up / lagSeconds over threshold for LAG_POLLS polls.
    Absent `projector` => no signal."""
    win = _tail(samples, LAG_POLLS)
    projs = [s["projector"] for s in win if isinstance(s.get("projector"), dict)]
    if len(projs) < LAG_POLLS:
        return None
    not_caught = all(p.get("caughtUp") is False for p in projs)
    lag_high = all(isinstance(p.get("lagSeconds"), (int, float))
                   and p["lagSeconds"] >= LAG_SECONDS for p in projs)
    if not_caught or lag_high:
        why = "caughtUp=false" if not_caught else f"lagSeconds>={LAG_SECONDS}"
        return {"node": node, "class": CLASS, "provenance": "projector:reconcile",
                "line": f"projector {why} sustained >= {LAG_POLLS} polls"}
    return None


def evaluate(window):
    """PURE: exhaustion predicates over a node's persisted sample window.
    Returns a list of Finding dicts (possibly empty). NO I/O. Each predicate is
    field-presence-tolerant: an endpoint field absent from the samples (e.g.
    /admin/self-healing not yet landed) contributes NO signal."""
    node = window.get("node", "?")
    samples = window.get("samples", [])
    findings = []
    for pred in (_render_degenerate, _circuit_open, _admission_shed, _projector_lag):
        f = pred(node, samples)
        if f is not None:
            findings.append(f)
    return findings


def reconcile(entries, active, poll_index):
    """PURE: fold this tick's active findings into the ledger list (MUTATES
    `entries` in place, like ci-harvest). Returns (new, bumped, closed).
    - unknown fp        -> append open/seen=1 (NEW; dispatch-eligible)
    - known fp active   -> bump seen/last_poll, reset clean_poll_streak (BUMPED)
    - known fp inactive -> increment clean_poll_streak; at >= CLOSE_STREAK DELETE
                           (CLOSED by disappearance, ANY status — runtime
                           exhaustions self-resolve without triage; D5)
    Structural suppression: a fp already on the ledger (any status) is NEVER
    returned as NEW."""
    by_fp = {e["fp"]: e for e in entries}
    active_fps = set()
    new, bumped = [], []
    for f in active:
        fp = f["fp"]
        active_fps.add(fp)
        e = by_fp.get(fp)
        if e is None:
            if len(new) >= MAX_NEW_FINDINGS:
                continue
            e = {"fp": fp, "class": f["class"], "node": f["node"],
                 "provenance": f["provenance"], "line": f["line"][:300],
                 "status": "open", "seen": 1,
                 "first_poll": poll_index, "last_poll": poll_index,
                 "clean_poll_streak": 0}
            by_fp[fp] = e
            entries.append(e)
            new.append(e)
        else:
            e["seen"] = e.get("seen", 1) + 1
            e["last_poll"] = poll_index
            e["clean_poll_streak"] = 0
            bumped.append(e)
    closed, kept = [], []
    for e in entries:
        if e["fp"] in active_fps:
            kept.append(e)
            continue
        e["clean_poll_streak"] = e.get("clean_poll_streak", 0) + 1
        if e["clean_poll_streak"] >= CLOSE_STREAK:
            closed.append(e)          # decomposed — line NOT kept (D5)
        else:
            kept.append(e)
    entries[:] = kept
    return new, bumped, closed
