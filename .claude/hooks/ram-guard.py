#!/usr/bin/env python3
"""
RAM Guard hook — the agent-facing half of genesis/agentic/bin/ram-guard.

Events (registered in .claude/settings.json):
  --event session   SessionStart: ensure the daemon is running (idempotent `ram-guard start`)
                    and land one status line via the hookSpecificOutput JSON wrapper.
  --event prompt    UserPromptSubmit: plain stdout reaches model context. Prints a RAM-GUARD
                    banner ONLY when there is something to say — a shed event this session has
                    not yet seen (per-session cursor), pressure at soft or above, or a stale
                    state (daemon dead). Hot path = one stat + one small JSON read + a tail of
                    events.jsonl; no subprocess.
  --event pretool   PreToolUse Bash: at high/hard DENY new heavy work (heavy cargo — parser
                    borrowed from cargo-disk-guard.py — JS builders/test runners, `just gate|test`);
                    at soft an advisory additionalContext. Stale/no state never denies (fail-open).
  --event posttool  PostToolUse Bash: surface a shed that landed since this session's last look,
                    so a command that died with `signal: 15` is explained in the same turn.

Why per-session cursors: three or four sessions share one workspace; the shed that killed
session A's cargo must reach A even if B already read the ledger. Cursor = line count of
events.jsonl seen, at <store>/seen/<session_id>.
"""

# Counted by _lib/intervenor_census.py. A condition, never a date.
RETIRE_WHEN = (
    "with genesis/agentic/bin/ram-guard — when the devworkspace platform sets memory.oom.group=0 "
    "and a memory.high band itself and a quarter passes with no shed event; the per-turn banner "
    "and the deny arm have nothing to report once the daemon has nothing to shed."
)

import importlib.machinery
import importlib.util
import json
import os
import re
import shlex
import subprocess
import sys
import time

PROJECT_DIR = os.environ.get("CLAUDE_PROJECT_DIR", "/projects/elohim")
HOOK_DIR = os.path.dirname(os.path.abspath(__file__))
DAEMON = os.path.join(PROJECT_DIR, "genesis", "agentic", "bin", "ram-guard")
STALE_SECONDS = 30  # the daemon writes state every poll (2s); 30s of silence = not running


def _load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, path)
    spec = importlib.util.spec_from_file_location(name, path, loader=loader)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _disk_guard():
    try:
        return _load(os.path.join(HOOK_DIR, "cargo-disk-guard.py"), "cargo_disk_guard")
    except Exception:
        return None


def store_dir():
    return (os.environ.get("RAM_GUARD_DIR")
            or os.path.join(os.environ.get("CLAUDE_CONFIG_DIR", "/projects/.claude-config"), "ram-guard"))


def load_policy():
    try:
        rg = _load(DAEMON, "ram_guard_daemon")
        return rg.load_policy()
    except Exception:
        return {"soft_pct": 70, "high_pct": 80, "hard_pct": 88}


def read_state(d=None):
    try:
        with open(os.path.join(d or store_dir(), "state.json")) as f:
            return json.load(f)
    except Exception:
        return None


def is_stale(state):
    return not state or (time.time() - float(state.get("ts", 0))) > STALE_SECONDS


# ----------------------------------------------------------------- heavy detection

JS_HEAVY_TOKENS = {
    "vitest", "vitest.mjs", "jest", "tsc", "ng", "cucumber-js", "cucumber", "playwright", "esbuild", "storybook",
    "eslint", "karma", "webpack", "ng-packagr", "build", "build:umd", "build:storybook", "test", "test:browser",
    "test:e2e", "lint", "a2o", "e2e",
}
PKG_HEAVY_VERBS = {"test", "build", "lint", "e2e", "run", "exec", "dlx", "start"}
_OPERATORS = {"&&", "||", ";", "|", "&"}
_ENV = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
_WRAPPERS = {"timeout", "nice", "env", "command", "sudo", "time"}


def _segments(command):
    try:
        toks = shlex.split(command.replace("\n", " ; "))
    except ValueError:
        toks = command.split()
    segs, cur = [], []
    for t in toks:
        if t in _OPERATORS:
            if cur:
                segs.append(cur)
            cur = []
        elif t.endswith(";"):
            cur.append(t.rstrip(";"))
            segs.append(cur)
            cur = []
        else:
            cur.append(t)
    if cur:
        segs.append(cur)
    return segs


def _strip(toks):
    i = 0
    while i < len(toks) and _ENV.match(toks[i]):
        i += 1
    while i < len(toks) and os.path.basename(toks[i]) in _WRAPPERS:
        i += 1
        if i < len(toks) and re.match(r"^[\d.]+[smhd]?$", toks[i]):
            i += 1
    return toks[i:]


def is_heavy(command):
    """True when the command starts new memory-heavy work: heavy cargo, `just gate|test`,
    a JS builder / test runner, or a package-manager script that runs one."""
    if not command:
        return False
    dg = _disk_guard()
    if dg and "cargo" in command and dg.heavy_cargo_segments(command):
        return True
    for seg in _segments(command):
        toks = _strip(seg)
        if not toks:
            continue
        head = os.path.basename(toks[0])
        rest = [os.path.basename(t) for t in toks[1:]]
        if head == "just" and rest and rest[0] in ("gate", "test"):
            return True
        if head in ("ng", "vitest", "jest", "tsc", "cucumber-js", "playwright", "esbuild", "storybook", "eslint", "karma", "webpack", "ng-packagr"):
            return True
        if head in ("pnpm", "npm", "npx", "yarn", "bun", "bunx", "node", "tsx"):
            words = [t for t in rest if not t.startswith("-")]
            if head in ("node", "tsx"):
                if any(w in JS_HEAVY_TOKENS for w in words[:1]):
                    return True
                continue
            # skip `--filter x` values
            filtered = []
            skip = False
            for t in rest:
                if skip:
                    skip = False
                    continue
                if t in ("--filter", "-F", "-C", "--dir", "-w", "--workspace"):
                    skip = True
                    continue
                if not t.startswith("-"):
                    filtered.append(t)
            if not filtered:
                continue
            if filtered[0] in ("run", "exec", "dlx", "start"):
                if len(filtered) > 1 and filtered[1] in JS_HEAVY_TOKENS:
                    return True
            elif filtered[0] in PKG_HEAVY_VERBS or filtered[0] in JS_HEAVY_TOKENS:
                return True
    return False


# ----------------------------------------------------------------- decisions

def _pressure_line(state):
    return (f"committed {state['committed_bytes'] / 2**30:.1f}G / {state['max_bytes'] / 2**30:.1f}G "
            f"({state['pct']}%, level {state['level']}; soft/high/hard "
            f"{state.get('soft_pct', '?')}/{state.get('high_pct', '?')}/{state.get('hard_pct', '?')}%)")


def decide_pretool(command, state, policy):
    """None (silent) | {'additionalContext': …} | {'permissionDecision': 'deny', …}. Only heavy
    commands are ever gated; stale or missing state never denies."""
    if not is_heavy(command):
        return None
    if is_stale(state):
        return {"additionalContext": (
            "RAM-GUARD: not running (no fresh state) — heavy work is ungated. Start it: "
            "`genesis/agentic/bin/ram-guard start` (the workspace OOM-group-kills at memory.max; "
            "the guard sheds builds first)."
        )}
    level = state.get("level", "ok")
    if level in ("high", "hard"):
        return {
            "permissionDecision": "deny",
            "permissionDecisionReason": (
                f"RAM {level.upper()}: {_pressure_line(state)}. New heavy work is refused until the "
                f"workspace drops below {policy['high_pct']}% — the guard is shedding "
                f"{'compile trees' if level == 'high' else 'compile trees, then JS builders, then dev servers'} "
                f"(genesis/agentic/pool-policy.json `ram`). Check `genesis/agentic/bin/ram-guard status`, "
                f"wait for a running build to finish, or use fewer jobs (CARGO_BUILD_JOBS=4). "
                f"Light commands are never gated."
            ),
        }
    if level == "soft":
        return {"additionalContext": (
            f"RAM SOFT: {_pressure_line(state)}. Heavy work still allowed; at {policy['high_pct']}% the guard "
            f"sheds the newest compile tree and refuses new builds. Prefer CARGO_BUILD_JOBS=4 / one build at a time."
        )}
    return None


def _cursor_path(d, session_id):
    safe = re.sub(r"[^A-Za-z0-9_.-]", "_", session_id or "default")
    return os.path.join(d, "seen", safe)


def unseen_events(d, session_id):
    """Events appended since this session last looked; advances the cursor."""
    path = os.path.join(d, "events.jsonl")
    try:
        with open(path) as f:
            lines = f.readlines()
    except Exception:
        lines = []
    cp = _cursor_path(d, session_id)
    try:
        with open(cp) as f:
            seen = int(f.read().strip() or 0)
    except Exception:
        seen = 0
    if seen > len(lines):  # ledger rotated
        seen = 0
    new = []
    for line in lines[seen:]:
        try:
            new.append(json.loads(line))
        except Exception:
            pass
    try:
        os.makedirs(os.path.dirname(cp), exist_ok=True)
        with open(cp, "w") as f:
            f.write(str(len(lines)))
    except Exception:
        pass
    return new


def _fmt_event(e):
    return (f"  - {time.strftime('%H:%M:%S', time.localtime(e['ts']))} shed {e['tier']} {e['comm']} "
            f"({e.get('rss_kb', 0) // 1024}MB, age {e.get('age_s', '?')}s) at {e['pct']}% — {e['cmd'][:100]}")


def prompt_banner(state, d, session_id, only_events=False):
    lines = []
    new = unseen_events(d, session_id)
    if new:
        lines.append(f"RAM-GUARD shed {len(new)} process tree(s) since your last look — a command that died with "
                     f"'signal: 15' / 'Terminated' was shed to keep the workspace under its memory limit:")
        lines.extend(_fmt_event(e) for e in new[-6:])
    if is_stale(state):
        if not only_events:
            lines.append("RAM-GUARD: not running (state stale or missing) — `genesis/agentic/bin/ram-guard start`.")
    elif state.get("level") != "ok" and (not only_events or state.get("level") in ("high", "hard")):
        lines.append(f"RAM-GUARD {state['level'].upper()}: {_pressure_line(state)}"
                     + (" — new heavy builds are refused until pressure drops." if state["level"] in ("high", "hard") else ""))
    return "\n".join(lines) if lines else None


# ----------------------------------------------------------------- events

def _emit(event_name, text, deny=None):
    out = {"hookEventName": event_name}
    if deny:
        out.update(deny)
    elif text:
        out["additionalContext"] = text
    else:
        return
    print(json.dumps({"hookSpecificOutput": out}))


def ensure_daemon():
    try:
        r = subprocess.run([sys.executable, DAEMON, "start"], capture_output=True, text=True, timeout=6)
        return (r.stdout or r.stderr).strip()
    except Exception as e:
        return f"ram-guard start failed: {e!r}"


IO_GUARD = os.path.join(PROJECT_DIR, "genesis", "agentic", "bin", "io-guard")
BERTH = os.path.join(PROJECT_DIR, "genesis", "agentic", "bin", "berth")


def berth_touch(session):
    """Liveness heartbeat for this session's mooring — one small file rewrite, no subprocess."""
    try:
        bdir = os.environ.get("BERTH_DIR") or os.path.join(os.environ.get("CLAUDE_CONFIG_DIR", "/projects/.claude-config"), "berth")
        safe = re.sub(r"[^A-Za-z0-9_.-]", "_", session or "default")
        path = os.path.join(bdir, "moorings", f"{safe}.json")
        with open(path) as f:
            m = json.load(f)
        m["last_seen"] = round(time.time(), 3)
        with open(path, "w") as f:
            json.dump(m, f, indent=1, sort_keys=True)
    except Exception:
        pass


def io_guard_and_berth_lines(session):
    """SessionStart: ensure io-guard (sibling daemon, write budget) and moor this session on the berth
    so the workspace knows who is here. Model/lab are unknown to a hook — the agent completes its own
    mooring with `berth moor --model <id> --lab <vendor> --task '<lane>'`."""
    out = []
    try:
        subprocess.run([sys.executable, IO_GUARD, "start"], capture_output=True, text=True, timeout=6)
        r = subprocess.run([sys.executable, IO_GUARD, "status", "--brief"], capture_output=True, text=True, timeout=6)
        out.append((r.stdout or r.stderr).strip() or "IO-GUARD: no status")
    except Exception as e:
        out.append(f"IO-GUARD: unavailable ({e!r})")
    try:
        subprocess.run([sys.executable, BERTH, "moor", "--session", session, "--runtime", "claude-code",
                        "--principal", os.environ.get("USER_EMAIL") or os.environ.get("USER") or "operator"],
                       capture_output=True, text=True, timeout=6)
        r = subprocess.run([sys.executable, BERTH, "status"], capture_output=True, text=True, timeout=6)
        st = (r.stdout or "").strip().splitlines()
        held = [l.strip() for l in st if " held by " in l or "/" in l.split()[1:2]]
        out.append(f"BERTH: moored as {session} (claude-code; complete it: `berth moor --model <id> --lab <vendor> --task '<lane>'`)"
                   + (" · leases: " + "; ".join(held) if held else " · no leases held")
                   + " · claim before mesh/cargo/disk-heavy work: `berth claim <resource>`; `berth status|ledger`")
    except Exception as e:
        out.append(f"BERTH: unavailable ({e!r})")
    return "\n".join(out)


def main(argv):
    event = argv[argv.index("--event") + 1] if "--event" in argv else "prompt"
    try:
        data = json.load(sys.stdin) if not sys.stdin.isatty() else {}
    except Exception:
        data = {}
    session = data.get("session_id") or os.environ.get("CLAUDE_SESSION_ID") or "default"
    d = store_dir()

    if event == "session":
        started = ensure_daemon()
        time.sleep(0.5)
        state = read_state(d)
        if is_stale(state):
            line = f"RAM-GUARD: {started} — no fresh state yet; `genesis/agentic/bin/ram-guard status`"
        else:
            line = (f"RAM-GUARD {state['level'].upper()}: {_pressure_line(state)} · memory.oom.group={state.get('oom_group')} "
                    f"memory.high={state.get('memory_high')} · sheds compile trees at high, the full ladder at hard; "
                    f"`genesis/agentic/bin/ram-guard status|plan`")
        line += "\n" + io_guard_and_berth_lines(session)
        _emit("SessionStart", line)
        return

    if event == "prompt":
        berth_touch(session)
        text = prompt_banner(read_state(d), d, session)
        if text:
            print(text)  # UserPromptSubmit: plain stdout lands in context
        return

    if data.get("tool_name") != "Bash":
        return
    command = (data.get("tool_input") or {}).get("command", "")

    if event == "pretool":
        dec = decide_pretool(command, read_state(d), load_policy())
        if dec and dec.get("permissionDecision") == "deny":
            _emit("PreToolUse", None, deny=dec)
        elif dec:
            _emit("PreToolUse", dec["additionalContext"])
        return

    if event == "posttool":
        text = prompt_banner(read_state(d), d, session, only_events=True)
        if text:
            _emit("PostToolUse", text)
        return


if __name__ == "__main__":
    try:
        main(sys.argv)
    except Exception:
        pass  # fail-open: a guard bug must never block a turn
    sys.exit(0)
