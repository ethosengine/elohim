#!/usr/bin/env python3
"""run-projection.py — the per-turn run-plane projection emitter (read-only, fail-open).

Spec: genesis/docs/superpowers/specs/2026-08-13-run-plane-projection-observation-events-design.md
§4 (Mechanism 2, the emitter's contract) and §5 (the write-path discipline this block's footer
carries). Plan: genesis/docs/superpowers/plans/2026-08-13-agentic-harness-borrows-implementation-plan.md
Task 3.

THE PROJECTION CONTRACT — what this is, and what it must never become:

  * It is a DERIVATION, never a register. Every line is re-derived from state that is already
    durable somewhere else: `genesis/manifests/habits.yaml` (the top red + the max-2 active WIP
    fence), the resiliency-saga chapters (the frontier), and `.eprfs/status/flows.jsonl` (the
    commitments stock and the run-scale notes `epr flow note` appends). Nothing here is authored
    state; re-deriving twice from unchanged inputs yields byte-identical output, which is exactly
    what makes the cache legitimate. A projection that acquired storage would have become the
    fifth register the survey (LEAVE-11) declines.
  * It is injected PER TURN so it cannot drift deeper into context as tokens accumulate (Arize's
    PlanMessage, survey §1.5 — the corpus's only per-call injection precedent). That is the whole
    borrow: not polling a tracker, not refreshing an in-memory work object, but placing current
    state into the next inference request (survey WATCH-10's category guard).
  * It ends with the WRITE-PATH footer. The footer is what makes this a loop rather than a
    broadcast: a projection over state nobody writes to projects nothing, so the block teaches
    `epr flow note` every turn, at the moment it is needed.

DELIVERY DIFFERS BY EVENT and getting it wrong ships a hook that never lands:

  --event prompt    UserPromptSubmit. PLAIN STDOUT reaches model context directly — no wrapper
                    (`pickup-semantic-surfacing.py:205-206`, with the reason recorded in its
                    docstring at :182-188: the tool-path injection was print()-based and never
                    landed). This path EMITS THE BLOCK.
  --event session   SessionStart. Only the `hookSpecificOutput` JSON form lands
                    (`delivery-gate.py:138-147`). This path REFRESHES THE CACHE — including the
                    one expensive leg (`epr flow stocks --json`, ~21s on a debug binary over
                    ~4.9k CID-verified records) — and emits ONE line, never the block: the
                    SessionStart headline (`load-project-context.py`) already carries habits,
                    saga and the memory budget, and duplicating it would cost tokens for nothing.

CACHE DISCIPLINE (the per-turn path must never spawn a subprocess chain):

  Cache: .claude/data/run-projection-cache.json, keyed on (habits.yaml mtime, flows.jsonl size,
  saga-dir mtime, git HEAD sha). On a HIT the prompt path is one file read and a print. On a MISS
  it re-derives only the CHEAP legs inline — habits line-scan (~2ms), saga fold (~70ms, in-process
  import, never a subprocess), flows.jsonl TAIL scan (last 64KB, never the whole 2.4MB file) — and
  SKIPS the stocks line with an honest `refresh pending` marker rather than blocking the turn on a
  20s cargo binary. The stocks leg carries its OWN sub-key so a cheap refresh never masks a stale
  equilibrium reading as fresh.

  Measured budget (2026-08-13, this container): cached prompt path ~0.04s, fully-stale prompt path
  ~0.15s. Registered at `timeout: 5` in .claude/settings.json — a budget the work actually fits
  inside, deliberately unlike the live `load-project-context.py` mismatch the plan names (declared
  5s, internal 25s, unreachable by construction).

DEGRADATION: any unreadable input drops ITS line and the rest still prints; any unexpected error
exits 0 silently. A hook that blocks a turn is worse than a hook that says nothing.
"""
from __future__ import annotations

import importlib.util
import json
import os
import shutil
import subprocess
import sys
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

# ── The intervenor's exit (Meadows' Way Out; counted as population 3 of the intervenor census,
# `.claude/scripts/_lib/intervenor_census.py:19-21`). Stated as a MODEL-TIER claim because that
# is the axis this scaffold actually lives or dies on — Anthropic's sprint scaffold was
# load-bearing on one tier and dead weight four months later (survey §1.4, §4.6) — and re-asked
# at every model-tier landing (WATCH-8).
RETIRE_WHEN = (
    "when a paired run shows the block adds nothing: two consecutive model-tier landings in "
    "which a session run WITHOUT this hook holds the WIP fence, the top red and the saga "
    "frontier across a full window as well as a session run WITH it — or earlier, when the "
    "harness itself injects durable run state per turn and this hook is competing with it "
    "rather than supplying it"
)

MAX_LINES = 20            # the block's hard cap (spec §4)
TAIL_BYTES = 64 * 1024    # flows.jsonl is read O(tail), never O(file)
MAX_NOTES = 3
LINE_WIDTH = 190
STOCKS_TIMEOUT_S = 45     # SessionStart only; sits inside the registered 60s budget
STOCK_NAME = "commitments"
WINDOW_DAYS = 7
CACHE_VERSION = 1

FOOTER = ("corrections that must outlive this turn: epr flow note --on <target> "
          "--kind correction --reason '...'")


def project_dir() -> Path:
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env:
        return Path(env)
    return Path(__file__).resolve().parents[2]


def read_payload() -> dict:
    try:
        return json.loads(sys.stdin.read() or "{}")
    except (json.JSONDecodeError, OSError, ValueError):
        return {}


def _clip(text: str, width: int = LINE_WIDTH) -> str:
    """One physical line, indentation preserved (the block's shape carries meaning), inner
    whitespace folded (habits.yaml checks are wrapped block scalars)."""
    raw = str(text)
    indent = raw[: len(raw) - len(raw.lstrip(" "))]
    body = " ".join(raw.split())
    line = indent + body
    return line if len(line) <= width else line[: width - 1] + "…"


# ───────────────────────────── cache key ─────────────────────────────

def git_head(root: Path) -> str:
    """HEAD sha WITHOUT shelling out to git — the per-turn path spawns no subprocess."""
    try:
        gitdir = root / ".git"
        if gitdir.is_file():  # worktree: `gitdir: <path>`
            gitdir = Path(gitdir.read_text(encoding="utf-8").split(":", 1)[1].strip())
        head = (gitdir / "HEAD").read_text(encoding="utf-8").strip()
        if head.startswith("ref:"):
            ref = head.split(":", 1)[1].strip()
            loose = gitdir / ref
            if loose.exists():
                return loose.read_text(encoding="utf-8").strip()[:40]
            packed = gitdir / "packed-refs"
            if packed.exists():
                for line in packed.read_text(encoding="utf-8", errors="replace").splitlines():
                    if line.endswith(" " + ref):
                        return line.split(" ", 1)[0][:40]
            return ""
        return head[:40]
    except (OSError, ValueError, IndexError):
        return ""


def _mtime(path: Path) -> float:
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def _size(path: Path) -> int:
    try:
        return path.stat().st_size
    except OSError:
        return 0


def cache_key(root: Path) -> dict:
    return {
        "habits_mtime": _mtime(root / "genesis" / "manifests" / "habits.yaml"),
        "flows_size": _size(root / ".eprfs" / "status" / "flows.jsonl"),
        "saga_mtime": _mtime(root / "genesis" / "a2o" / "features" / "dataplane"
                             / "resiliency-saga"),
        "head": git_head(root),
    }


def stocks_key(root: Path, window: tuple[str, str]) -> dict:
    """The stocks leg's OWN key. Kept separate so a cheap prompt-path refresh cannot advance the
    main key past a stocks reading that was never re-folded — the shape that would let a stale
    equilibrium verdict read as fresh."""
    return {"flows_size": _size(root / ".eprfs" / "status" / "flows.jsonl"),
            "head": git_head(root), "window": list(window), "stock": STOCK_NAME}


def cache_path(root: Path) -> Path:
    return root / ".claude" / "data" / "run-projection-cache.json"


def read_cache(root: Path) -> dict:
    try:
        cached = json.loads(cache_path(root).read_text(encoding="utf-8"))
        return cached if isinstance(cached, dict) and cached.get("version") == CACHE_VERSION else {}
    except (OSError, json.JSONDecodeError, ValueError):
        return {}


def write_cache(root: Path, payload: dict) -> None:
    path = cache_path(root)
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_suffix(".json.tmp")
        tmp.write_text(json.dumps(payload, indent=1, sort_keys=True), encoding="utf-8")
        os.replace(tmp, path)  # atomic: a torn cache would be read as a miss, never as truth
    except OSError:
        pass  # the cache is an optimization; never fail a turn over it


# ───────────────────────────── habits ─────────────────────────────

def parse_habits(path: Path) -> dict:
    """Line-scan `habits.yaml` for (id, status, active, first check) per habit.

    Deliberately NOT a YAML parse and deliberately not a `habits-status.py` subprocess: PyYAML is
    optional in this tree (habits-status.py exits 0 when the import fails) and its --headline path
    costs ~0.7s because it also shells out to git for doc dynamics. Keys are read at exactly
    4-space indent, which cannot collide with the block scalars (`invariant: >`, `evidence: >`)
    whose bodies are indented deeper."""
    out: dict = {"habits": []}
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return out
    cur: dict | None = None
    in_habits = False
    in_checks = False
    for raw in text.splitlines():
        if not in_habits:
            in_habits = raw.rstrip() == "habits:"
            continue
        if raw[:1].isalpha() and ":" in raw.split(" ", 1)[0]:
            break  # a new top-level key ends the habits block (a comment/blank does not)
        if raw.startswith("  - id:"):
            cur = {"id": raw.split(":", 1)[1].strip(), "status": "", "active": False, "check": ""}
            out["habits"].append(cur)
            in_checks = False
            continue
        if cur is None:
            continue
        if raw.startswith("    ") and not raw.startswith("     "):
            in_checks = raw.strip() == "checks:"
            key, _, val = raw.strip().partition(":")
            val = val.strip()
            if key == "status":
                cur["status"] = val
            elif key == "active":
                cur["active"] = val.lower() == "true"
            continue
        if in_checks and not cur["check"] and raw.strip().startswith("- "):
            cur["check"] = raw.strip()[2:].strip().strip('"')
    return out


def habit_lines(path: Path) -> list[str]:
    """The top red (habits-status.py:136-139's selection rule, reused not re-invented) and the
    max-2 `active` WIP fence."""
    habits = parse_habits(path).get("habits") or []
    if not habits:
        return []
    reds = [h for h in habits if h["status"] == "red"]
    unwired = [h for h in habits if h["status"] == "unwired"]
    active = [h for h in habits if h["active"]]
    top = next((h for h in reds if h["active"]), None) or (reds[0] if reds else None) \
        or next((h for h in unwired if h["active"]), None)
    lines = []
    if top is None:
        lines.append("  habits: all green — pull the next habit from the covenant queue")
    elif top["status"] == "red":
        lines.append(_clip(f"  top red: {top['id']} → {top['check'] or '(check missing)'}"))
    else:
        lines.append(_clip(f"  top move: {top['id']} is unwired → write its red"))
    fence = ", ".join(h["id"] for h in active) or "none"
    lines.append(_clip(f"  WIP fence (max 2 active): {fence} · "
                       f"{len(reds)} red / {len(unwired)} unwired of {len(habits)}"))
    return lines


# ───────────────────────────── saga frontier ─────────────────────────────

def _report_age(report) -> str:
    """Human-scale age of the banked saga report ('2.1h old' / '3d old'), or '' when
    unparseable. Pure arithmetic on the already-loaded report dict — no I/O."""
    try:
        raw = (report or {}).get("generatedAt") or ""
        ts = datetime.fromisoformat(raw.replace("Z", "+00:00"))
        hours = max(0.0, (datetime.now(timezone.utc) - ts).total_seconds() / 3600.0)
        return f"{hours / 24:.0f}d old" if hours >= 48 else f"{hours:.1f}h old"
    except (ValueError, TypeError, AttributeError):
        return ""


def saga_line(root: Path) -> str:
    """The saga frontier, from the SAME source the SessionStart headline uses
    (`.claude/scripts/saga-status.py`, called by `load-project-context.py:115-128`) — but IMPORTED
    in-process rather than spawned, because the per-turn path spawns nothing. ~70ms."""
    script = root / ".claude" / "scripts" / "saga-status.py"
    if not script.exists():
        return ""
    try:
        spec = importlib.util.spec_from_file_location("_saga_status", script)
        if spec is None or spec.loader is None:
            return ""
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        rows, report, flows_present = module.assemble()
        if not rows:
            return ""
        line = module.headline(rows, report, flows_present)
        # Report age rides the headline (2026-08-14): a saga verdict quoted without the age of
        # the banked report behind it reads as "now" when it may be days old. Computed HERE
        # (projection layer), never in saga-status.py — that script is a shift measure oracle.
        age = _report_age(report)
        if age:
            line += f" · report {age}"
        return _clip("  " + line)
    except Exception:  # noqa: BLE001 — a projection line is never worth a traceback
        return ""


# ───────────────────────────── run notes (tail-only) ─────────────────────────────

def run_notes(flows: Path, limit: int = MAX_NOTES) -> list[str]:
    """The last `limit` UNRESOLVED `run:*` notes, newest first, from the TAIL of flows.jsonl.

    "Unresolved" here is the cheap, honest v1 predicate: a note is superseded by any LATER note on
    the same target resource, so only the newest note per target survives. It deliberately does not
    claim to know whether the underlying correction was acted on — that would require the whole
    file and a commitment-discharge fold, and a per-turn hook may not read 2.4MB.

    Only the last TAIL_BYTES are read; the first (possibly torn) line is discarded. A note older
    than the tail window is out of scope by construction, which is correct for a RUN-scale surface.
    """
    if not flows.exists():
        return []
    try:
        size = flows.stat().st_size
        with flows.open("rb") as fh:
            if size > TAIL_BYTES:
                fh.seek(size - TAIL_BYTES)
                fh.readline()  # discard the partial first line
            chunk = fh.read()
    except OSError:
        return []

    seen: set[str] = set()
    out: list[str] = []
    for raw in reversed(chunk.decode("utf-8", errors="replace").splitlines()):
        raw = raw.strip()
        if not raw or "run:" not in raw:
            continue  # cheap pre-filter before the json parse
        try:
            row = json.loads(raw)
            rec = row.get("record") or {}
            if rec.get("kind") != "event":
                continue
            tags = rec.get("classifiedAs") or []
            if not tags or not str(tags[0]).startswith("run:"):
                continue
            target = str(tags[1]) if len(tags) > 1 else "?"
            if target in seen:
                continue  # a later note on the same target supersedes this one
            seen.add(target)
            body = next((str(t)[len("reason:"):] for t in tags[2:]
                         if str(t).startswith("reason:")), "")
            switched = next((str(t)[len("switched-to:"):] for t in tags[2:]
                             if str(t).startswith("switched-to:")), "")
            note = f"    {tags[0]} · {Path(target).name} · {body}"
            if switched:
                note += f" → switched to {switched}"
            out.append(_clip(note))
            if len(out) >= limit:
                break
        except (json.JSONDecodeError, ValueError, TypeError, AttributeError):
            continue
    return out


# ───────────────────────────── commitments stock (cached) ─────────────────────────────

def window_for(today: date | None = None) -> tuple[str, str]:
    """Canonical stocks window: WINDOW_DAYS periods, **end-exclusive at tomorrow's midnight**
    so every event that already happened TODAY is inside the window. With `end = today`
    (the pre-2026-08-14 shape) the fold's end-exclusive predicate dropped same-day events,
    so identical flows read REFUSED (no observations yet today) or FILLING (yesterday's
    inflow visible, today's outflow not) depending on the wall-clock day the check ran —
    the verdict flap that disqualified equilibrium as a stasis termination criterion."""
    end = (today or date.today()) + timedelta(days=1)
    return ((end - timedelta(days=WINDOW_DAYS)).isoformat(), end.isoformat())


def epr_candidates(root: Path) -> list[Path]:
    """Where an `epr` carrying the `stocks` leg may live, most-likely first. The eprfs workspace is
    excluded from the monorepo workspace, so its gate target is /tmp/eprfs-gate-target
    (.husky/pre-push.bash:330-336); the pool slot is the branch-family form."""
    cands: list[Path] = []
    env = os.environ.get("ELOHIM_EPR_BIN")
    if env:
        cands.append(Path(env))
    pool = os.environ.get("CARGO_TARGET_POOL_ROOT")
    if pool:
        try:
            cands.extend(sorted(Path(pool).glob("family/*/elohim__eprfs/dev/release/epr")))
            cands.extend(sorted(Path(pool).glob("family/*/elohim__eprfs/dev/debug/epr")))
        except OSError:
            pass
    cands.append(Path("/tmp/eprfs-gate-target/release/epr"))
    cands.append(Path("/tmp/eprfs-gate-target/debug/epr"))
    found = shutil.which("epr")
    if found:
        cands.append(Path(found))
    return [c for c in cands if c.is_file() and os.access(c, os.X_OK)]


def git_readable(root: Path) -> bool:
    """Can git read this repo RIGHT NOW? Guards the stocks fold against a fail-OPEN defect
    proven 2026-08-14: with git unreadable (the `dubious ownership` state a fresh container
    hits before `safe.directory` is set), `epr flow stocks` returns **inflow 0.0** while the
    outflow arm survives — so `outflow >= inflow` holds trivially and the verdict reads
    DRAINING. That is a FABRICATED equilibrium: the check reports "drained" precisely when it
    cannot see inflow, which is the one reading a stasis loop must never trust. Reproduced:
    same window/stock/log bytes, `HOME=/nonexistent` → inflow 0.0/draining, control →
    inflow 23.0/filling. The typed refusal belongs in epr's own verdict arm (eprfs is outside
    this hook's scope — filed as a backlog item); until then this gate keeps the hook honest."""
    try:
        proc = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=str(root),
                              capture_output=True, text=True, timeout=10)
        return proc.returncode == 0 and bool(proc.stdout.strip())
    except (OSError, subprocess.SubprocessError):
        return False


def refresh_stocks(root: Path, window: tuple[str, str]) -> dict:
    """Run `epr flow stocks --json` ONCE (SessionStart only). Returns the parsed reading, or an
    honest absence record — never a fabricated equilibrium. A `stocks` leg that is missing from an
    older binary is recorded as absent rather than retried forever."""
    if not git_readable(root):
        # Fail CLOSED: no reading at all beats a false DRAINING (see `git_readable`).
        return {"ok": False, "reason": "git unreadable — inflow arm would silently read 0 "
                                       "(fabricated equilibrium); refusing the fold",
                "window": list(window)}
    for binary in epr_candidates(root):
        try:
            proc = subprocess.run(
                [str(binary), "flow", "stocks", "--window", f"{window[0]}..{window[1]}",
                 "--per", "week", "--stock", STOCK_NAME, "--json", "--root", str(root)],
                capture_output=True, text=True, timeout=STOCKS_TIMEOUT_S,
            )
            parsed = json.loads(proc.stdout)
            for stock in parsed.get("stocks") or []:
                if stock.get("stock") == STOCK_NAME:
                    m = stock.get("measures") or {}
                    return {"ok": True, "binary": str(binary),
                            "level": m.get("level"), "inflow": m.get("inflow"),
                            "outflow": m.get("outflow"), "net": m.get("net"),
                            "verdict": (stock.get("verdict") or {}).get("verdict"),
                            "window": list(window)}
        except (OSError, subprocess.SubprocessError, json.JSONDecodeError, ValueError, TypeError):
            continue
    return {"ok": False, "reason": "no epr binary with a `stocks` leg", "window": list(window)}


def stock_line(stocks: dict | None, fresh: bool) -> str:
    if not stocks or not stocks.get("ok"):
        reason = (stocks or {}).get("reason", "not yet folded")
        return f"  {STOCK_NAME}: stocks unavailable ({reason}) — honest absence, not equilibrium"
    if not fresh:
        try:
            level = f"{float(stocks['level']):.0f}"
        except (KeyError, TypeError, ValueError):
            level = "?"
        return (f"  {STOCK_NAME}: {level} open at last fold (stocks: refresh pending — "
                f"equilibrium re-folds at next SessionStart)")
    win = stocks.get("window") or ["?", "?"]
    try:
        return (f"  {STOCK_NAME}: {stocks['level']:.0f} open · inflow {stocks['inflow']:.2f}/wk · "
                f"outflow {stocks['outflow']:.2f}/wk · {str(stocks['verdict']).upper()} "
                f"({win[0]}..{win[1]})")
    except (KeyError, TypeError, ValueError):
        return f"  {STOCK_NAME}: stocks reading malformed — honest absence, not equilibrium"


# ───────────────────────────── derivation + render ─────────────────────────────

def derive(root: Path) -> dict:
    """The cheap legs: habits · saga · run notes. Never the stocks fold (that is SessionStart's)."""
    return {
        "habits": habit_lines(root / "genesis" / "manifests" / "habits.yaml"),
        "saga": saga_line(root),
        "notes": run_notes(root / ".eprfs" / "status" / "flows.jsonl"),
    }


def render(parts: dict, stocks: dict | None, stocks_fresh: bool) -> list[str]:
    notes = parts.get("notes") or []
    # An "honest absence" stocks line is a caveat, never content: a block whose ONLY line says a
    # measure is missing is noise, so it must not by itself keep the block alive.
    if not (parts.get("habits") or parts.get("saga") or notes or (stocks or {}).get("ok")):
        return []
    body: list[str] = list(parts.get("habits") or [])
    if parts.get("saga"):
        body.append(parts["saga"])
    body.append(stock_line(stocks, stocks_fresh))
    if notes:
        body.append("  run notes (newest per target):")
        body.extend(notes)
    head = "[run-plane] re-derived this turn from habits.yaml · resiliency-saga · flows.jsonl"
    return ([head] + body)[: MAX_LINES - 1] + [f"  {FOOTER}"]


def handle_prompt(root: Path) -> None:
    cached = read_cache(root)
    key = cache_key(root)
    s_key = stocks_key(root, window_for())
    stocks = cached.get("stocks")
    stocks_fresh = cached.get("stocks_key") == s_key

    if cached.get("key") == key and cached.get("lines"):
        print("\n".join(cached["lines"]))      # HIT: one read, one print
        return

    parts = derive(root)                        # MISS: cheap legs only, stocks skipped honestly
    lines = render(parts, stocks, stocks_fresh)
    if not lines:
        return
    write_cache(root, {"version": CACHE_VERSION, "key": key, "parts": parts, "lines": lines,
                       "stocks": stocks, "stocks_key": cached.get("stocks_key")})
    print("\n".join(lines))


def handle_session(root: Path) -> None:
    """Refresh the FULL cache (stocks included) and emit ONE line — never the block. The
    SessionStart headline already carries habits + saga + the memory budget; what it does not
    carry is the write path, so that is what this line teaches."""
    cached = read_cache(root)
    window = window_for()
    s_key = stocks_key(root, window)
    stocks = cached.get("stocks") if cached.get("stocks_key") == s_key else refresh_stocks(
        root, window)
    parts = derive(root)
    lines = render(parts, stocks, True)
    write_cache(root, {"version": CACHE_VERSION, "key": cache_key(root), "parts": parts,
                       "lines": lines, "stocks": stocks, "stocks_key": s_key})
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": ("[run-plane] per-turn projection armed (top red · WIP fence · saga "
                              "frontier · commitments equilibrium). " + FOOTER),
    }}))


def main() -> int:
    event = sys.argv[sys.argv.index("--event") + 1] if "--event" in sys.argv else ""
    read_payload()  # drained so the writing end never blocks on a full pipe
    try:
        root = project_dir()
        if event == "prompt":
            handle_prompt(root)
        elif event == "session":
            handle_session(root)
    except Exception:  # noqa: BLE001 — a projection must never block a turn or a session start
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
