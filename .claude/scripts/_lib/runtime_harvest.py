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
DEGEN_MIN_EVENTS = 3    # ... and >= N NEW degenerate renders (a rate needs a base)
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


def _cum_render(s):
    """A sample's LIFETIME render counters as (total, degenerate), or None.

    Prefers the /admin/render-stats shape (`stalled` + `timedOut`); falls back to
    /admin/self-healing's smaller `{total, degenerateRate}` render sub-object so the
    predicate keeps working off either endpoint. Field-presence-tolerant."""
    r = s.get("render")
    if not isinstance(r, dict) or not isinstance(r.get("total"), (int, float)):
        return None
    total = r["total"]
    if isinstance(r.get("stalled"), (int, float)) and isinstance(r.get("timedOut"), (int, float)):
        return total, r["stalled"] + r["timedOut"]
    if isinstance(r.get("degenerateRate"), (int, float)):
        return total, r["degenerateRate"] * total
    return None


def _render_degenerate(node, samples):
    """SSR saturation happening NOW: the degenerate share of the renders that are
    NEW across the window (a DELTA), >= DEGEN_RATE.

    Why a delta and not the served `degenerateRate` scalar: /admin/render-stats
    counters are LIFETIME-cumulative by construction (elohim-render
    `src/stats.rs:10-11` — "Cumulative (lifetime) counters for the MVP"), so
    `degenerateRate` is a since-boot ratio, not a current condition. Reading it
    "sustained >= N polls" carries ZERO currency: once the lifetime ratio crosses
    DEGEN_RATE it stays crossed on every later poll until diluted by a large
    volume of clean renders, so the finding can never close by disappearance (D5)
    and an idle node keeps re-asserting a stall burst that ended hours ago.
    Measured 2026-09-11 on alpha-b: three consecutive polls carried byte-identical
    counters (total=11, stalled=4 — no renders at all in the window) and the
    cumulative read still filed "sustained >= 3 polls (SSR saturation)".

    Deltas are what the plan specified all along ("`stalled`/`timedOut` deltas",
    elevate-arm plan D2) and what the sibling `_admission_shed` already does to
    the equally-cumulative `shedTotal`. The delta baseline spans the whole stored
    ring buffer (WINDOW) so low-volume saturation is still visible, but at least
    DEGEN_POLLS observations AND DEGEN_MIN_EVENTS new degenerate renders are
    required before the predicate may fire — a ratio over a 4-render base is noise,
    not a condition (see the floor rationale inline below)."""
    win = samples[-WINDOW:]
    if len(win) < DEGEN_POLLS:
        return None
    cums = [c for c in (_cum_render(s) for s in win) if c is not None]
    if len(cums) < DEGEN_POLLS:
        return None
    d_total = cums[-1][0] - cums[0][0]
    d_degen = cums[-1][1] - cums[0][1]
    # d_total == 0: nothing rendered in the window — silence, not saturation.
    # Either delta < 0: the counters reset (pod restart); the window refills.
    if d_total <= 0 or d_degen < 0:
        return None
    # A RATE NEEDS A BASE. DEGEN_RATE is 0.25, so on a 4-render delta a SINGLE slow
    # fetch crossing the soft budget scores exactly at threshold — and on a 1-render
    # delta it scores 1.00. One stall is not saturation: it is the fetch budget doing
    # its job once. Measured 2026-09-12 on alpha (fp da8bb3bdd7e1): five byte-identical
    # samples at total=9/stalled=0 then one at total=13/stalled=1 filed "1/4 of NEW
    # renders = 0.25", while alpha-b — the node with 4x the stall history and 5x the
    # avgWallMs — was correctly silent. The delta cured the lifetime-ratio defect but
    # left the predicate with no floor under its denominator.
    #
    # So require DEGEN_MIN_EVENTS NEW degenerate renders as well as the rate. Three is
    # the same "3 observations before we believe it" discipline OPEN_POLLS / SHED_POLLS
    # / LAG_POLLS already carry, applied to events instead of polls. Deliberately NOT a
    # floor on d_total: 3-of-3 degenerate on a quiet node IS saturation, and a volume
    # floor would blind the predicate to exactly the low-traffic peer that needs it.
    if d_degen < DEGEN_MIN_EVENTS:
        return None
    rate = d_degen / d_total
    if rate < DEGEN_RATE:
        return None
    # Count the polls that actually SAW new renders. `len(cums)` is the stored window,
    # which on a mostly-idle node is 5 parts staleness to 1 part evidence — reporting it
    # as "across last 6 polls" overstated the base in the very finding that exposed this.
    moving = sum(1 for a, b in zip(cums, cums[1:]) if b[0] > a[0])
    return {
        "node": node,
        "class": CLASS,
        "provenance": "render-degenerate",
        "line": f"render degenerate {int(round(d_degen))}/{int(d_total)} of NEW renders "
                f"= {rate:.2f} over {moving} of the last {len(cums)} polls "
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
