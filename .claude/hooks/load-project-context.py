#!/usr/bin/env python3
"""
Load Project Context Hook

DELIVERY: SessionStart receives one hookSpecificOutput.additionalContext JSON
wrapper. PROJECT CONTEXT serves age-labelled, cached headlines refreshed by a
detached worker. BOOTSTRAP is rendered synchronously on every invocation through
`epr flow memory recall open --purpose bootstrap --lens minimal`.

The bootstrapping-head-is-recall-open contract permits only that native rendering
or one honest `bootstrap: skipped — <reason>` line. Recall has its own five-second
budget and process group; neither its output nor its session links are cached.

Hook Type: SessionStart
"""
import json
import sys
import os

RECALL_TIMEOUT_S = 5  # foreground recall budget within the harness's 8-second hook timeout

def load_relationships(project_dir: str) -> dict:
    """Load the file relationships configuration."""
    rel_path = os.path.join(project_dir, '.claude', 'file-relationships.json')
    if not os.path.exists(rel_path):
        return {}

    with open(rel_path, 'r') as f:
        return json.load(f)

def format_schema_summary(schemas: dict) -> str:
    """Format schemas for context injection."""
    if not schemas:
        return ""

    lines = ["ELOHIM PROJECT SCHEMAS:"]
    lines.append("")

    for name, info in schemas.items():
        lines.append(f"• {name}: {info.get('description', 'No description')}")
        lines.append(f"  Defined in: {info.get('definedIn', 'Unknown')}")
        fields = info.get('fields', [])
        if fields:
            lines.append(f"  Fields: {', '.join(fields[:8])}")
            if len(fields) > 8:
                lines.append(f"          ...and {len(fields) - 8} more")
        lines.append("")

    return "\n".join(lines)

def get_sync_relationships_summary(relationships: dict) -> str:
    """Summarize file relationships for context."""
    lines = ["FILE SYNC RELATIONSHIPS:"]
    lines.append("")

    for group_name, group in relationships.get('relationships', {}).items():
        desc = group.get('description', group_name)
        lines.append(f"• {group_name}: {desc}")

        if 'skill' in group:
            lines.append(f"  Skill: {group['skill']}")
        if 'cli' in group:
            lines.append(f"  CLI: {group['cli']}")

        sync_rules = group.get('sync_rules', [])
        if sync_rules:
            lines.append(f"  Sync rules: {len(sync_rules)} patterns tracked")

        lines.append("")

    return "\n".join(lines)

def _headline_cache_path(project_dir: str) -> str:
    """A per-project /tmp path both this hook and delivery-gate.py agree on, so the heavy
    `epr flow report --headline` runs ONCE per SessionStart instead of once per consumer."""
    import re
    slug = re.sub(r'[^A-Za-z0-9]+', '-', project_dir).strip('-')
    return f"/tmp/claude-headline-{slug}.txt"


def _observation_module(project_dir: str):
    """The hooks' shared emitter, imported by path so this works however the hook was launched."""
    sys.path.insert(0, os.path.join(project_dir, '.claude', 'hooks'))
    try:
        import _observation
        return _observation
    except Exception:
        return None


def _epr_headline(project_dir: str) -> str:
    """`epr flow report --headline` — the native projection of the declared bounds.

    The bounds themselves live in .claude/epr-meta/{measures,policies}.yaml; this verb folds
    each one against the observations the drift hooks append and prints the same headline
    lines the SessionStart headline is made of. Returns "" when the verb is absent or fails;
    there is no second producer to fall back to. Binary resolution: $EPR_BIN, then PATH.
    """
    import subprocess
    obs = _observation_module(project_dir)
    binary = obs.resolve_bin() if obs else None
    if not binary:
        return ""
    try:
        r = subprocess.run([binary, 'flow', 'report', '--headline', '--root', project_dir],
                           capture_output=True, text=True, timeout=25)
        return r.stdout.strip() if r.returncode == 0 else ""
    except Exception:
        return ""


def get_memory_budget(project_dir: str) -> str:
    """Always-on memory budget headline — `epr flow report --headline` is the OWNER.

    The last-resort kit re-run went at station six round (a); the PRODUCER BRIDGE went at
    round (b) (2026-09-11), when the last two bridged values — `memkit-report-tier-mb@1` and
    `mempalace-surfaces-changed@1` — got native producers (the first `status: superseded` with
    the report tier it measured, the second `derive: files-newer-than` over a declared surface
    walk). Every one of the five slots is now derived, so a bridge would DOUBLE a value rather
    than supply one. Caches its stdout to a per-project /tmp file so delivery-gate.py (same
    SessionStart) reuses it instead of recomputing. Fail-open: any failure returns ''.
    """
    out = _epr_headline(project_dir)
    if out:
        try:
            with open(_headline_cache_path(project_dir), 'w', encoding='utf-8') as fh:
                fh.write(out)
        except OSError:
            pass  # cache is a bonus; never fail the budget fetch on a write error
    return out


def _epr_bootstrap_block(project_dir: str, data: dict) -> str:
    """The SessionStart BOOTSTRAP block: the `minimal` lens of
    `epr flow memory recall open --purpose bootstrap` — the bootstrapping head declared in
    `.claude/hooks/.epr-meta` (rule `bootstrapping-head-is-recall-open`).

    This is the ONLY orientation this hook renders. The former `get_habits_status` (a bespoke
    `habits-status.py --headline` re-scan) is RETIRED, not kept beside this: the `open` call's
    `minimal` lens already carries a `Bootstrap: top red: <id> — <check>` line drawn from the
    same habits register, and keeping both would be exactly the "a hook renders it at a lens
    and never derives a second orientation" drift the rule exists to catch.

    Landing: this hook is SessionStart and SYNCHRONOUS, so its `hookSpecificOutput.
    additionalContext` is the documented landing path (verified empirically — see
    `main()`'s comment). The caller folds this block's return value INTO that same JSON
    wrapper alongside the memory-budget headline; nothing here is printed on its own.

    Session-id and binary resolution both come from `_observation` (`resolve_bin`,
    `bootstrap_session_id`) — the same shared helper `run-projection.py` uses, so the two
    hooks' fallback session ids always agree. When `_observation` itself cannot be imported
    (a missing/broken copy in this project dir), there is no binary to resolve either, so the
    same "no epr binary resolved" absence line covers both cases without a separate check.

    Honest absence, never a fallback renderer: a missing binary, a non-zero exit, or a run past
    the 5-second budget all return exactly one line, `bootstrap: skipped — <reason>`.
    """
    import signal
    import subprocess
    obs = _observation_module(project_dir)
    binary = obs.resolve_bin() if obs else None
    if not binary:
        return "bootstrap: skipped — no epr binary resolved ($EPR_BIN or PATH)"
    session_id = obs.bootstrap_session_id(project_dir, data)
    try:
        # Kill the whole group at the deadline: a grandchild holding a pipe must not
        # stretch recall past the hook timeout (same shape as run-projection.py).
        proc = subprocess.Popen(
            [binary, 'flow', 'memory', 'recall', 'open',
             '--purpose', 'bootstrap', '--lens', 'minimal',
             '--session', session_id, '--root', project_dir],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True,
        )
        try:
            stdout, stderr = proc.communicate(timeout=RECALL_TIMEOUT_S)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except OSError:
                pass
            # Reap without waiting on inherited pipe EOF or spending the remaining
            # hook budget if the OS has not completed the kill yet.
            proc.stdout.close()
            proc.stderr.close()
            try:
                proc.wait(timeout=0.1)
            except subprocess.TimeoutExpired:
                pass
            return f"bootstrap: skipped — recall open exceeded the {RECALL_TIMEOUT_S}s budget"
    except Exception as exc:  # noqa: BLE001 — a SessionStart hook must never crash the session
        return f"bootstrap: skipped — {exc}"
    if proc.returncode != 0:
        reason_lines = (stderr or stdout or "").strip().splitlines()
        reason = reason_lines[0] if reason_lines else f"exit {proc.returncode}"
        return f"bootstrap: skipped — {reason}"
    return stdout.rstrip("\n") or "bootstrap: skipped — empty rendering"


def get_saga_status(project_dir: str) -> str:
    """Resiliency-saga headline (deterministic; from saga-status.py, no args = one-liner).
    A DIFFERENT derivation from the bootstrap block (the ten dataplane chapters' green/frontier
    state, not the habits register's top red) — same subprocess/timeout/silent-failure posture:
    a failed or slow saga-status.py emits nothing rather than blocking session start."""
    import subprocess
    saga = os.path.join(project_dir, '.claude', 'scripts', 'saga-status.py')
    if not os.path.exists(saga):
        return ""
    try:
        r = subprocess.run([sys.executable, saga],
                           capture_output=True, text=True, timeout=10)
        return r.stdout.strip()
    except Exception:
        return ""


def seed_memory_injection_flag(session_id: str) -> None:
    """Pre-seed pre-tool-memory.py's flag for the MAIN session tree.

    The harness injects MEMORY.md natively for the main session (claudeMd
    system-reminder), so the PreToolUse injector would only duplicate ~22KB.
    SessionStart runs in the same process tree as the main loop's hooks, so
    touching the (session_id, ppid) flag here suppresses the duplicate while
    leaving subagent trees (different ppid) to get their injection.
    """
    try:
        from pathlib import Path
        Path(f"/tmp/claude-memory-loaded-{session_id[:64]}-{os.getppid()}").touch()
    except OSError:
        pass


def _render_context(project_dir: str) -> str:
    """Render only project headlines for the detached, shared cache."""
    context_parts = []

    # Always-on memory budget (deterministic) — fetched UNCONDITIONALLY (before any
    # schema/relationship gating) so every session opens knowing the state of the memory
    # surface + what's testable, per genesis/docs/PLACEMENT.md.
    budget = get_memory_budget(project_dir)
    if budget:
        context_parts.append(budget)
        context_parts.append("")

    # Resiliency-saga headline — the ten dataplane chapters' green/frontier state
    # (genesis/a2o/features/dataplane/resiliency-saga/), joined from flows.jsonl +
    # the (often-absent-locally) dataplane sprint-report. See saga-sync.sh to pull
    # the report down and refresh the flow state.
    saga = get_saga_status(project_dir)
    if saga:
        context_parts.append(saga)
        context_parts.append("")

    # Schema + sync-relationship data lives in .claude/file-relationships.json and is
    # re-delivered at edit time by the sync-check hook; a one-line pointer replaces the
    # ~2.5KB static dump every session paid before 2026-07-02.
    relationships = load_relationships(project_dir)
    if relationships:
        context_parts.append(
            "SYNC HOOKS ACTIVE: edit-time hooks surface related-file reminders from "
            ".claude/file-relationships.json (schemas + sync rules; read it when you need the map)."
        )
        context_parts.append("")

    # As before, a failed native headline pass must not replace the shared cache
    # with only the static sync pointer.
    return "\n".join(context_parts) if budget or saga else ""


# Measured 2026-10-31 at load 28: headline 1.32 s + saga 0.19 s + recall 3.47 s;
# the reported loaded run took 13.87 s against an 8 s hook timeout. Only project
# headlines render off-path, single-flight, and land with their age. The cached path
# measured 0.091–0.206 s; foreground recall now has a separate 5 s budget.
# After the split, three live runs took 5.088 / 3.376 / 3.772 s (2026-10-31);
# recall honestly skipped because the sandbox's /projects/elohim root was read-only.
CONTEXT_TTL_S = 120
CONTEXT_MAX_STALE_S = 86400


def _context_cache_path(project_dir: str):
    import hashlib
    import tempfile
    from pathlib import Path
    # Retain the existing shared headline key; session-specific bootstrap caches
    # are no longer read or written.
    identity = [os.path.realpath(project_dir), None,
                os.environ.get('EPR_BIN', ''), os.environ.get('BIN_STORE', '')]
    key = hashlib.sha256(json.dumps(identity).encode()).hexdigest()[:24]
    return Path(tempfile.gettempdir()) / f"claude-project-context-{os.getuid()}-{key}.json"


def _read_context_cache(cache):
    import time
    try:
        entry = json.loads(cache.read_text())
        age = time.time() - entry['at']
        if 0 <= age < CONTEXT_MAX_STALE_S and isinstance(entry['context'], str) and entry['context']:
            return entry['context'], age
    except (OSError, ValueError, KeyError, TypeError):
        pass
    return None


def _store_context_cache(cache, context: str) -> None:
    import time
    tmp = cache.with_name(f"{cache.name}.{os.getpid()}.tmp")
    try:
        tmp.write_text(json.dumps({'at': time.time(), 'context': context}))
        os.replace(tmp, cache)
    finally:
        tmp.unlink(missing_ok=True)


def _refresh_context(project_dir: str, cache) -> None:
    import fcntl
    try:
        with cache.with_suffix('.lock').open('a') as lock:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                return
            entry = _read_context_cache(cache)
            if entry and entry[1] < CONTEXT_TTL_S:
                return
            context = _render_context(project_dir)
            if context:
                _store_context_cache(cache, context)
    except OSError:
        pass  # the previous completed context survives a failed refresh


def _cached_context(project_dir: str, identity: dict) -> str:
    import subprocess
    session_id = identity['session'] or ''
    cache = _context_cache_path(project_dir)
    entry = _read_context_cache(cache)
    refreshing = not entry or entry[1] >= CONTEXT_TTL_S
    state = 'refresh pending'
    if refreshing:
        # Detachment loses process ancestry. Carry the shared adapter's canonical identity
        # and owner into the worker so binary pins still belong to THIS session.
        env = dict(os.environ)
        if session_id:
            env['ELOHIM_SESSION_ID'] = session_id
            env['ELOHIM_SESSION_OWNER_PID'] = str(identity['ownerPid'] or '')
        try:
            subprocess.Popen(
                [sys.executable, os.path.abspath(__file__), '--refresh', project_dir,
                 str(cache)], env=env,
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
        except OSError:
            state = 'refresh unavailable'
    if entry:
        context, age = entry
        pending = f'; {state}' if refreshing else ''
        return f"PROJECT CONTEXT (last completed pass {int(age)}s ago{pending}):\n{context}"
    return (f"PROJECT CONTEXT: {state} (no completed pass).\n"
            "SYNC HOOKS: .claude/file-relationships.json carries schemas + sync rules.")


def main():
    try:
        data = json.load(sys.stdin)
        project_dir = os.environ.get('CLAUDE_PROJECT_DIR', '/projects/elohim')
        # The shared dependency-free adapter avoids a native session-id subprocess on the
        # foreground path; its owner-bound result is also passed to the detached renderer.
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'scripts'))
        from pathlib import Path
        from _lib.session import load_registry, resolve_fallback
        identity = resolve_fallback(registry=load_registry(os.environ, Path(project_dir)))
        seed_memory_injection_flag(identity['session'] or '')
        context = _cached_context(project_dir, identity)
        # BOOTSTRAP must land in this invocation, even when headline refresh is pending.
        context += "\nBOOTSTRAP:\n" + _epr_bootstrap_block(project_dir, data)
        print(json.dumps({'hookSpecificOutput': {'hookEventName': 'SessionStart',
                                                'additionalContext': context}}))
    except json.JSONDecodeError:
        sys.exit(0)
    except Exception as e:
        print(f"load-project-context hook error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == '--refresh':
        from pathlib import Path
        _refresh_context(sys.argv[2], Path(sys.argv[3]))
    else:
        main()
