#!/usr/bin/env python3
"""deprecation-sentinel — PostToolUse(Bash) hook.

Watches Bash tool output for deprecation warnings AND security-
vulnerability reports emitted in flight (Vitest "DEPRECATED ...",
npm "npm warn deprecated", Node DeprecationWarning, Rust "use of
deprecated"; pnpm/npm install + audit vulnerability summaries, GitHub
push banners "found N vulnerabilities", CVE-/GHSA-/RUSTSEC advisories)
against two stores with crisp roles:

  1. Ledger = the EXISTING-POSITIVES CHECK SURFACE (decides firing):
     .claude/data/deprecations.jsonl — one line per LIVE fingerprint
     {ts, fp, line, cmd, status, backlog?}. Status: open → triaged →
     blocked. A fingerprint PRESENT here suppresses dispatch; a
     fingerprint ABSENT fires the dev. FIXED items are DELETED from
     the ledger at close (full memory decomposition — the git commit
     is the record), so a reintroduced deprecation reads as NEW and
     correctly re-fires: regression handling for free.
  2. Canonical backlog = the CLOSE-OF-TRIAGE DECISION SURFACE:
     genesis/data/timeline/backlog/deprecation-*.md — timeline-
     CONVENTIONS-conformant entries holding the live trajectory
     ("Current decision" is the citation line for blocked items).
     Fixed-no-work-left entries are DELETED (rarely graduated to
     timeline/chronicle/ when genuinely meaningful) — everything in
     the backlog has a trajectory or a status, or it's not there.

Behavior:
  * NEW fingerprint  → append to ledger + inject a dispatch directive:
    the session launches the `deprecation-triage` agent (Opus) in the
    BACKGROUND — flag → scope → canonicalize → fix|block — and carries
    on with its current task.
  * KNOWN fingerprint (live: open/triaged/blocked) → once per session,
    inject a one-line deterministic citation of the current decision
    (backlog path + status). No agent dispatch — blocked-and-
    canonicalized items never re-fire automation; the
    deprecation-stasis sweep re-checks blockers deliberately.
  * Command itself mentions deprecation (greps, ledger edits, this
    hook's own tooling) → skip entirely (false-positive guard).

Fail-safe: any internal error exits 0 silently — the sentinel must
never break a session.
"""

import hashlib
import json
import os
import re
import sys
from datetime import datetime, timezone

MAX_SCAN_BYTES = 200_000  # bound the regex scan on huge outputs
MAX_NEW_PER_CALL = 5  # bound dispatch-directive noise from one command
MAX_CITED_PER_CALL = 3  # bound re-encounter citation noise
LINE_TRUNC = 300

# Deprecation signatures across the toolchains in this repo
# (Vitest/Vite, npm/pnpm, Node, Angular, Rust, Python, generic).
DEPRECATION_PATTERNS = re.compile(
    r"(?:"
    r"\bDEPRECATED\b"
    r"|\bDeprecationWarning\b"
    r"|npm warn deprecated"
    r"|\buse of deprecated\b"
    r"|\b(?:is|are|has been|was|were) deprecated\b"
    r"|\bdeprecated (?:and|API|option|in|since)\b"
    r"|\bwill be removed in (?:a future|the next|version|v?\d)"
    r")",
    re.IGNORECASE,
)

# Security-vulnerability signatures from dependency pull/install/audit stages
# (pnpm/npm install + audit summaries, GitHub push banners, cargo-audit
# RUSTSEC advisories, GHSA/CVE identifiers).
SECURITY_PATTERNS = re.compile(
    r"(?:"
    r"\b\d+ vulnerabilit(?:y|ies)\b"
    r"|\bCVE-\d{4}-\d+\b"
    r"|\bGHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}\b"
    r"|\bRUSTSEC-\d{4}-\d+\b"
    r"|\bsecurity advisor(?:y|ies)\b"
    r"|\b(?:critical|high|moderate|low) severity vulnerab"
    r")",
    re.IGNORECASE,
)

# Count-bearing security SUMMARY lines ("found 191 vulnerabilities (1
# critical, 113 high...)"): counts drift run-to-run, so their fingerprint is
# digit-normalized — one live concern, stable across count churn. Advisory-ID
# lines (CVE-/GHSA-/RUSTSEC-) are NOT normalized: distinct advisories must
# stay distinct fingerprints.
SECURITY_SUMMARY = re.compile(
    r"\d+ vulnerabilit|\(\d+ (?:critical|high|moderate|low)", re.IGNORECASE
)

# Commands that themselves talk about these signals (greps, ledger edits,
# this tooling) are not new in-flight findings.
GUARD_TOKENS = ("deprecat", "vulnerab", "cve-", "ghsa-", "rustsec", "advisor")

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")

# ── Anti-echo guards (three deterministic false-positive classes) ─────────────
#
# Guard A — ledger self-capture:
#   A grep that recurses into the repo may hit the ledger file itself and
#   re-fingerprint already-triaged entries.  A grep-with-filename result line
#   looks like "/path/to/deprecations.jsonl:42:{...json...}" so we match any
#   line that contains "deprecations.jsonl" as a source path prefix.
ECHO_LEDGER_PATH = re.compile(r"deprecations\.jsonl", re.IGNORECASE)

# Guard B — managed decision-record / museum / planning prose:
#   Lines sourced from the project's deprecation-narrating managed surfaces:
#     - genesis/docs/content/elohim-protocol/history/**  (museum / arc records)
#     - genesis/data/timeline/backlog/**                 (the canonical backlog
#       decision records THIS sentinel itself writes — every deprecation-*.md
#       carries "deprecated" in its title/tags/author frontmatter by design)
#     - genesis/data/timeline/chronicle/**               (graduated-lesson records)
#     - genesis/docs/superpowers/**                      (plan / spec / brief
#       docs whose task headers narrate work — "### Task 1: Retire the dead
#       Groovy helpers + fix the inverted DEPRECATED label")
#     - .superpowers/**                                  (the SDD working tree:
#       progress.md task-checkbox ledgers, per-task reports, review diffs —
#       "- [x] Task 1: complete … DEPRECATED comment removed …")
#   These narrate (or canonicalize) deprecations; reading them back is never a
#   NEW in-flight finding.  fp-dedupe cannot suppress the class: each new
#   backlog/chronicle/plan/progress edit mints a fresh fingerprint on the next
#   cat/grep/tail, so only a structural guard collapses it (cf. the 2026-06-17
#   + 2026-06-20 backlog-frontmatter captures, and the 2026-06-25 SDD-ledger
#   c4c3eccc7e05 + plan-header d09bbe4004a6 captures during the P6 sprint).
#   They appear in grep-with-filename output as a leading file path, or the
#   command itself directly reads such a file (cat/head/tail/sed output has no
#   path prefix — the command gate below covers that case).
ECHO_HISTORY_PATH_RE = re.compile(
    r"genesis/docs/content/elohim-protocol/history/"
    r"|genesis/data/timeline/(?:backlog|chronicle)/"
    r"|genesis/docs/superpowers/"
    r"|\.superpowers/",
    re.IGNORECASE,
)

# Guard C — commit-message prose:
#   git log --oneline produces lines like "8f0cb4122 chore(deprecation): …"
#   git show --stat/--format produces commit subject lines verbatim.
#   Heuristic: a line that starts with a short hex run followed by a space is a
#   git log oneline entry.  Additionally, bare commit-subject lines beginning
#   with conventional-commit prefixes (chore(deprecat…)/fix(deprecat…) etc.)
#   are commit message text, not live runtime output.
#   We do NOT gate on the command being a git command alone — a `git show` DIFF
#   hunk (lines starting with + or -) can legitimately add a `#[deprecated]`
#   attribute and should still be captured.  The shape guards below are
#   line-level and do not suppress diff hunks.
ECHO_GIT_ONELINE_RE = re.compile(r"^[0-9a-f]{7,12} ")
ECHO_COMMIT_SUBJECT_RE = re.compile(
    r"^(?:chore|fix|feat|refactor|docs|test|perf|ci|build|revert)"
    r"\((?:deprecat|security|sentinel)[^)]*\)\s*:",
    re.IGNORECASE,
)

# Guard D — ephemeral-script self-capture:
#   A DeprecationWarning whose SOURCE marker is literally `<string>:N:` or
#   `<stdin>:N:` was emitted by code run via `python3 -c "…"`, `exec()`, or
#   stdin — by Python's own convention these markers NEVER name a checked-in
#   source file (a real file warning carries its path: "foo.py:12:"). These
#   are an agent's own ad-hoc inline scripts (e.g. parsing an MCP tool-result
#   .txt with datetime.utcfromtimestamp), scrolled past in Bash output and
#   re-fingerprinted line-by-line. Every one of the 14 priors of this exact
#   shape (utcfromtimestamp / PIL getdata) was dispositioned false-positive,
#   and because the marker's line number differs each run, fp-dedupe can never
#   suppress the class — only this shape guard can. The marker may sit at
#   line-start OR after a grep line-number prefix ("1110: <string>:1: …"), so
#   we search rather than anchor. Zero true-positive risk: a genuine codebase
#   deprecation is always sourced from a named path or a toolchain prefix.
ECHO_EPHEMERAL_SOURCE_RE = re.compile(r"<(?:string|stdin)>:\d+:")

# Command-level gates for echo classes B and C: if the command itself is a pure
# git history read (git log / git show without a -p / --patch flag) or reads
# directly from the history / planning / SDD prose trees, ALL lines from that
# output are echo candidates — the command gate is a cheap early exit before
# per-line work (cat/tail/grep of a single doc carries no path prefix per line,
# so the command string is the only signal of source).
_CMD_GIT_HISTORY_RE = re.compile(
    r"\bgit\s+(?:log|show)\b(?!.*(?:\s-p\b|\s--patch\b|\s--diff\b))",
    re.IGNORECASE,
)
_CMD_HISTORY_TREE_RE = re.compile(
    r"genesis/docs/content/elohim-protocol/history/"
    r"|genesis/data/timeline/(?:backlog|chronicle)/"
    r"|genesis/docs/superpowers/"
    r"|\.superpowers/",
    re.IGNORECASE,
)


def _is_echo_line(line: str, cmd_is_git_history: bool, cmd_is_history_tree: bool) -> bool:
    """Return True if *line* is a known echo false-positive and must be skipped.

    Called after classify() returns non-None to prevent false echo entries
    from being fingerprinted and appended to the ledger.

    The four guards:
      A) Line sourced from the ledger file (self-capture via recursive grep).
      B) Line sourced from the history/museum prose tree.
      C) Line that is git commit-message text (log oneline entry, or any
         subject/body line from a pure git history read), NOT a diff hunk
         (diff hunks start with + or - so carry real signal).
      D) DeprecationWarning sourced from `<string>:N:`/`<stdin>:N:` — an
         agent's own ephemeral `python3 -c`/stdin script, never a real file.
    """
    # Guard A — ledger self-capture
    if ECHO_LEDGER_PATH.search(line):
        return True

    # Guard D — ephemeral-script self-capture (<string>:N: / <stdin>:N:).
    # Placed early: cheapest deterministic skip, no command-flag dependency.
    if ECHO_EPHEMERAL_SOURCE_RE.search(line):
        return True

    # Guard B — history prose (grep output carries file path in the line;
    # if the command itself reads from the tree, every line is echo output)
    if ECHO_HISTORY_PATH_RE.search(line) or cmd_is_history_tree:
        return True

    # Guard C — commit-message text (both git log oneline entries and all
    # lines of commit message bodies from git log/show history reads).
    # Preserve diff hunks: lines starting with + or - are hunk content that
    # can legitimately add a real #[deprecated] attribute and must still capture.
    if not line.startswith(("+", "-")):
        # git log --oneline shape: "8f0cb4122 commit subject…" — always echo.
        if ECHO_GIT_ONELINE_RE.match(line):
            return True
        # Any non-hunk line from a pure git history read (git log/git show
        # without -p/--patch) is commit message prose — subject OR body.
        # ECHO_COMMIT_SUBJECT_RE was too narrow (subject-only); the command-
        # level flag is the authoritative gate for the whole message body.
        if cmd_is_git_history:
            return True

    return False


def collect_strings(node, out, budget):
    """Walk the tool_response JSON collecting string values, bounded."""
    if budget[0] <= 0:
        return
    if isinstance(node, str):
        out.append(node[: budget[0]])
        budget[0] -= len(node)
    elif isinstance(node, dict):
        for v in node.values():
            collect_strings(v, out, budget)
    elif isinstance(node, list):
        for v in node:
            collect_strings(v, out, budget)


def classify(line: str) -> str | None:
    """Return 'deprecation' | 'security' | None for a stripped line."""
    if DEPRECATION_PATTERNS.search(line):
        return "deprecation"
    if SECURITY_PATTERNS.search(line):
        return "security"
    return None


def fingerprint(line: str, cls: str) -> str:
    norm = re.sub(r"\s+", " ", line).strip().lower()
    if cls == "security" and SECURITY_SUMMARY.search(line):
        # Count-churn stability: digit runs collapse so "191 vulnerabilities
        # (1 critical, 113 high)" and next week's counts share one concern.
        norm = re.sub(r"\d+", "#", norm)
    return hashlib.sha256(norm.encode()).hexdigest()[:12]


def session_cited_path(session_id: str) -> str:
    sid = re.sub(r"[^A-Za-z0-9]", "", session_id)[:12] or "nosession"
    return f"/tmp/claude-dep-cited-{sid}"


def load_session_cited(path: str) -> set:
    try:
        with open(path, encoding="utf-8") as fh:
            return set(fh.read().split())
    except OSError:
        return set()


def main() -> None:
    payload = json.load(sys.stdin)
    if payload.get("tool_name") != "Bash":
        return

    command = (payload.get("tool_input") or {}).get("command", "")
    # Guard the grep/echo false-positive class: a command that itself
    # talks about deprecation/vulnerabilities (searching the codebase,
    # reading this ledger, editing this tooling) is not a NEW in-flight
    # finding. Real sources (pnpm install, pnpm audit, cargo audit, git
    # push banners) don't carry these tokens in the command string.
    cmd_lower = command.lower()
    if any(tok in cmd_lower for tok in GUARD_TOKENS):
        return

    texts: list = []
    collect_strings(payload.get("tool_response"), texts, [MAX_SCAN_BYTES])
    if not texts:
        return

    project = os.environ.get("CLAUDE_PROJECT_DIR", ".")
    ledger = os.path.join(project, ".claude", "data", "deprecations.jsonl")

    known = {}  # fp -> entry
    if os.path.exists(ledger):
        with open(ledger, encoding="utf-8", errors="replace") as fh:
            for raw in fh:
                try:
                    entry = json.loads(raw)
                    known[entry["fp"]] = entry
                except (json.JSONDecodeError, KeyError):
                    continue

    # Pre-compute command-level echo flags (cheap, done once per call).
    cmd_is_git_history = bool(_CMD_GIT_HISTORY_RE.search(command))
    cmd_is_history_tree = bool(_CMD_HISTORY_TREE_RE.search(command))

    new_entries = []
    reencountered = []  # known, live
    matched_this_call = set()
    for text in texts:
        for line in ANSI.sub("", text).splitlines():
            line = line.strip()
            if not line:
                continue
            cls = classify(line)
            if cls is None:
                continue
            # Anti-echo guards: skip deterministic false-positive line shapes
            # before fingerprinting so they never enter the ledger.
            if _is_echo_line(line, cmd_is_git_history, cmd_is_history_tree):
                continue
            fp = fingerprint(line, cls)
            if fp in matched_this_call:
                continue
            matched_this_call.add(fp)
            entry = known.get(fp)
            if entry is None:
                if len(new_entries) < MAX_NEW_PER_CALL:
                    new_entries.append(
                        {
                            "ts": datetime.now(timezone.utc).isoformat(timespec="seconds"),
                            "fp": fp,
                            "class": cls,
                            "line": line[:LINE_TRUNC],
                            "cmd": command[:160],
                            "status": "open",
                        }
                    )
            else:
                # Any ledger presence is a LIVE positive (fixed items are
                # deleted at close, never parked) — cite, don't re-fire.
                reencountered.append(entry)

    context_parts = []
    system_parts = []

    if new_entries:
        os.makedirs(os.path.dirname(ledger), exist_ok=True)
        with open(ledger, "a", encoding="utf-8") as fh:
            for entry in new_entries:
                fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        fps = ", ".join(e["fp"] for e in new_entries)
        lines = " | ".join(f'{e["fp"]} [{e["class"]}]: "{e["line"][:110]}"' for e in new_entries)
        context_parts.append(
            f"[deprecation-sentinel] {len(new_entries)} NEW finding(s) "
            f"captured to .claude/data/deprecations.jsonl — {lines}. "
            f"DISPATCH NOW (do not derail the current task): launch the "
            f"`deprecation-triage` agent via the Agent tool with "
            f"run_in_background: true and the prompt 'Triage ledger "
            f"fingerprint(s) {fps} per your agent definition "
            f"(.claude/agents/deprecation-triage.md). Your goal is the "
            f"largest genuine step toward stasis this run supports — "
            f"canonicalize by concern, land what is bounded, document "
            f"live trajectories for the rest.' "
            f"If the Agent tool lacks the deprecation-triage type this "
            f"session, use general-purpose with the same prompt. Then "
            f"continue your current task."
        )
        system_parts.append(f"+{len(new_entries)} new → deprecation-triage dispatch")

    if reencountered:
        # Deterministic backlog citation: once per session per fingerprint.
        cited_path = session_cited_path(str(payload.get("session_id", "")))
        cited = load_session_cited(cited_path)
        fresh = [e for e in reencountered if e["fp"] not in cited][:MAX_CITED_PER_CALL]
        if fresh:
            try:
                with open(cited_path, "a", encoding="utf-8") as fh:
                    for e in fresh:
                        fh.write(e["fp"] + "\n")
            except OSError:
                pass
            cites = "; ".join(
                f'{e["fp"]} status={e.get("status", "open")}'
                + (f' decision={e["backlog"]}' if e.get("backlog") else " (untriaged)")
                for e in fresh
            )
            context_parts.append(
                f"[deprecation-sentinel] known deprecation(s) re-encountered — "
                f"current decision(s): {cites}. No action needed; the "
                f"deprecation-stasis sweep owns re-checks."
            )

    if not context_parts:
        return

    print(
        json.dumps(
            {
                "systemMessage": "deprecation-sentinel: " + "; ".join(system_parts or ["known re-encounter cited"]),
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": " ".join(context_parts),
                },
            }
        )
    )


if __name__ == "__main__":
    try:
        main()
    except Exception:  # noqa: BLE001 — sentinel must never break a session
        pass
    sys.exit(0)
