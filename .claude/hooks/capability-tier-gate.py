#!/usr/bin/env python3
"""
Capability Tier Gate — destructive git needs a declared tier or a team check.

Hook Type: PreToolUse   Matcher: Bash

Why (operator, 2026-09-11): a Haiku subagent of another session ran `git reset --hard` on the
shared `dev` checkout — three commits left the branch pointer and ~27 tracked files' uncommitted
work across several lanes was wiped; recovery took two sessions an hour (see
.claude/memory/feedback_tiered_agent_capabilities_destructive_git.md). Haiku- and Sonnet-tier
agents must not hold `git reset`-level capability without checking with the team first.

The table lives in ONE place: `.claude/epr-meta/policies.yaml` row
`destructive-git-requires-tier@1` (`parameters.rules` — a tokenised subcommand classification,
`parameters.rm-force-recursive-targets` — the `rm -rf` literal target list, tier-order,
tier-floor, unknown-tier, remedy). This hook is a thin reader of that declared table, never a
second copy of it — see the `capability-tier-gate-owns-destructive-git` rule in
`.claude/hooks/.epr-meta`.

FIX ROUND 1 (adversarial review, measured with 30 payloads — spec was met, quality was not:
the gate was bypassable). What changed from the first pass:

  1. Substring matching -> TOKENISED CLASSIFICATION. A command is split into shell segments
     (`;`/`&&`/`||`/`|`/newlines), each segment is unwrapped (`bash -c "…"` / `sh -c '…'` /
     `command …` / `env …` / an absolute path to a binary all resolve to the REAL head), git
     invocations have their GLOBAL options stripped (`-C <p>`, `--git-dir=`, `--work-tree=`,
     `-c k=v`, ...) before the subcommand is read, and classification is subcommand + flags, not
     a string search. This closes the whole bypass class a literal-pattern scan cannot see:
     `git reset -q --hard` (an inserted flag broke the substring), `git -C /other/checkout reset
     --hard` (global options moved the pattern's position — and this MUST still deny: a hook
     running in one worktree has to catch a reset aimed at ANY checkout, not just its own cwd),
     `git clean -xfd` (combined short flags), `git checkout HEAD -- .` / `git restore --staged
     --worktree .` (extra tokens before the destructive marker). It also adds a whole class that
     was never in the pattern set at all: `rebase`, `filter-branch`, `checkout -B`,
     `switch -C`, `reflog expire`, `gc --prune=now`.
  2. The row's `contentHash` PIN is now VERIFIED (reusing `_lib.epr_meta.policy_content_hash`,
     the SAME canonicalization `epr-meta-pin.py` writes) BEFORE the row is trusted for anything.
     A mismatch denies every git/rm candidate outright — fail-CLOSED, because a tampered table
     cannot be trusted to say what ISN'T destructive either. This is distinct from a merely
     malformed row (missing/wrong-shaped fields), which stays the honest `skipped —` path.
  3. Every internal error is now SPOKEN: `capability-tier-gate: skipped — internal error:
     <ExceptionType>` on stderr, exit 0 — never a bare, silent `except Exception: exit(0)`.
  4. Tier resolution consults ONLY `CLAUDE_MODEL` (dropped `ANTHROPIC_MODEL` — a process-wide
     config value would be a blanket bypass available to every session, not a per-actor
     declaration), then the actor sidecar, then `unknown`. The remedy text no longer hands out
     the self-claim command — it says only to ask the controller/operator. Read plainly: the
     tier is SELF-ASSERTED either way (an honesty fence, not a control); the worktree-per-plan
     discipline (`.claude/worktrees/<plan>`) is the structural half — a reset in a worktree
     cannot wipe another lane's checkout — this row is the behavioral half.

Deny shape still copies `cargo-disk-guard.py`'s exact convention: one `hookSpecificOutput` JSON
object on stdout (`permissionDecision: deny`), process exit 0 — the harness reads the JSON, not
the exit code, for a structured PreToolUse deny.
"""

# The intervenor's removal condition (counted by _lib/intervenor_census.py). A condition, never
# a date.
RETIRE_WHEN = (
    "when the harness enforces agent-tier command capability natively — a first-class permission "
    "scope keyed on the acting model, not a repo-side pattern table — at which point this hook "
    "is a redundant second enforcement of a guarantee the platform already holds. Until then an "
    "unscoped git-reset-class command is unrecoverable, cross-session data loss on a shared "
    "checkout, and that risk does not retire on a quiet quarter."
)

import json
import os
import re
import shlex
import sys

PROJECT_DIR = os.environ.get("CLAUDE_PROJECT_DIR", "/projects/elohim")
POLICY_FILE = os.path.join(PROJECT_DIR, ".claude", "epr-meta", "policies.yaml")
ACTORS_FILE = os.path.join(PROJECT_DIR, ".eprfs", "status", "actors.jsonl")

POLICY_ID = "destructive-git-requires-tier"
POLICY_VERSION = 1
POLICY_REF = f"{POLICY_ID}@{POLICY_VERSION}"

# Cheap pre-filter: only a Bash command that mentions `git` or `rm` can possibly contain a
# destructive-git or destructive-rm invocation, so everything else returns before touching
# policies.yaml (or the actor sidecar, or the pin-hashing lib) at all — mirrors
# cargo-disk-guard.py's `"cargo" not in command` early bailout.
_PRE_FILTER = re.compile(r"git|rm")

# ── tokenisation ──────────────────────────────────────────────────────────────────────────────

_OPERATORS = {"&&", "||", ";", "|", "&"}
_ENV_ASSIGN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
_WRAPPERS = {"timeout", "nice", "env", "command", "sudo"}
_SHELLS = {"bash", "sh", "dash", "zsh"}
_NEVER_CLASSIFY_HEADS = {"echo", "printf", "grep", "cat"}
_MAX_UNWRAP_DEPTH = 4


def _segments(command: str) -> list:
    """Split the WHOLE command into shell segments on real operator tokens, quote-aware (an
    operator character embedded in a quoted argument stays inside its token — a commit message
    or an inner `bash -c` string containing `&&` can't fake a segment boundary)."""
    try:
        toks = shlex.split(command.replace("\n", " ; "))
    except ValueError:
        return []
    segs, cur = [], []
    for t in toks:
        if t in _OPERATORS:
            if cur:
                segs.append(cur)
            cur = []
            continue
        if t.endswith(";") and t not in _OPERATORS:
            cur.append(t.rstrip(";"))
            segs.append(cur)
            cur = []
            continue
        cur.append(t)
    if cur:
        segs.append(cur)
    return segs


def _strip_wrappers(toks: list) -> list:
    """Strip leading `VAR=val` env assignments and wrapper heads (`env`, `command`, `timeout`,
    `nice`, `sudo`) so the REAL head is toks[0] afterward."""
    i = 0
    while i < len(toks) and _ENV_ASSIGN.match(toks[i]):
        i += 1
    while i < len(toks) and os.path.basename(toks[i]) in _WRAPPERS:
        wrapper = os.path.basename(toks[i])
        i += 1
        while i < len(toks) and _ENV_ASSIGN.match(toks[i]):
            i += 1
        if wrapper == "timeout" and i < len(toks) and re.match(r"^[\d.]+[smhd]?$", toks[i]):
            i += 1
    return toks[i:]


def _leaf_commands(command: str, depth: int = 0):
    """Yield (head_basename, args_after_head) for every unwrapped, non-shell invocation found in
    `command` — recursing into `bash -c "…"` / `sh -c '…'` strings, after stripping wrappers. An
    absolute path to a binary resolves via basename (`/usr/bin/git` -> head `git`). A segment
    whose head is `echo`/`printf`/`grep`/`cat` is never classified — falls out by construction
    (only shell/`git`/`rm` heads are ever inspected further; anything else is simply not git or
    rm and is skipped, exactly like `pnpm test` would be)."""
    if depth > _MAX_UNWRAP_DEPTH:
        return
    for seg in _segments(command):
        toks = _strip_wrappers(seg)
        if not toks:
            continue
        head = os.path.basename(toks[0])
        if head in _NEVER_CLASSIFY_HEADS:
            continue
        if head in _SHELLS:
            if len(toks) >= 3 and toks[1] == "-c":
                yield from _leaf_commands(toks[2], depth + 1)
            continue
        yield head, toks[1:]


_GIT_GLOBAL_ARG_OPTS = {"-C", "--git-dir", "--work-tree", "-c", "--namespace", "--exec-path"}
_GIT_GLOBAL_ARG_OPTS_EQ = ("--git-dir=", "--work-tree=", "--namespace=", "--exec-path=")


def _split_git_global_opts(args: list):
    """(subcommand, subcommand_args) after skipping git GLOBAL options — `-C <path>` and
    friends consume the option AND their following value; git subcommands never start with `-`,
    so the first non-dash token is always the subcommand. This is what makes `git -C
    /other/checkout reset --hard` classify as `reset` regardless of the `-C` target — the gate
    denies a destructive command aimed at ANY checkout, not only the cwd's."""
    i = 0
    while i < len(args):
        t = args[i]
        if not t.startswith("-"):
            return t, args[i + 1:]
        if t in _GIT_GLOBAL_ARG_OPTS:
            i += 2
            continue
        if t.startswith(_GIT_GLOBAL_ARG_OPTS_EQ):
            i += 1
            continue
        i += 1  # an unrecognised global flag: skip one token, keep looking for the subcommand
    return None, []


# ── rule evaluation (git subcommand rules) ───────────────────────────────────────────────────

def _flag_present(argv: list, flag: str) -> bool:
    return any(t == flag or t.startswith(flag + "=") for t in argv)


def _flag_letters_present(argv: list, letters: list) -> bool:
    wanted = set(letters)
    for t in argv:
        if t.startswith("-") and not t.startswith("--") and len(t) > 1:
            if wanted & set(t[1:]):
                return True
    return False


def _git_rule_matches(sub: str, args: list, rules: list):
    """The first declared rule (in registry order) whose `sub` matches and whose predicate keys
    (`any_flags`/`any_args`/`flag_letters` — each OR-within, ALL keys present AND-across) all
    hold. A rule with none of those keys matches the bare subcommand unconditionally
    (`rebase`/`filter-branch`/`update-ref`)."""
    for rule in rules:
        if not isinstance(rule, dict) or rule.get("sub") != sub:
            continue
        ok = True
        if "any_flags" in rule:
            ok = ok and any(_flag_present(args, f) for f in rule["any_flags"])
        if "any_args" in rule:
            ok = ok and any(a in args for a in rule["any_args"])
        if "flag_letters" in rule:
            ok = ok and _flag_letters_present(args, rule["flag_letters"])
        if ok:
            return rule
    return None


# ── rm -rf / -fr / -r -f classification ──────────────────────────────────────────────────────

def _rm_force_and_recursive(args: list) -> bool:
    has_r = has_f = False
    for t in args:
        if t == "--recursive":
            has_r = True
        elif t == "--force":
            has_f = True
        elif t.startswith("--"):
            continue
        elif t.startswith("-") and len(t) > 1:
            letters = set(t[1:])
            if "r" in letters or "R" in letters:
                has_r = True
            if "f" in letters:
                has_f = True
    return has_r and has_f


def _rm_target_is_destructive(target: str, literal_targets: set) -> bool:
    if target in literal_targets:
        return True
    if target.startswith("/"):
        try:
            return os.path.exists(os.path.join(target, ".git"))
        except Exception:
            return False
    return False


def _rm_matches(args: list, literal_targets: set) -> "str | None":
    if not _rm_force_and_recursive(args):
        return None
    for t in args:
        if t.startswith("-"):
            continue
        if _rm_target_is_destructive(t, literal_targets):
            return t
    return None


# ── policy row loading + pin verification ────────────────────────────────────────────────────

def _load_policy_row():
    """(`{raw, rules, rm_targets, tier_order, tier_floor, unknown_tier, remedy}`, None) on
    success, or (None, reason) — never raises. Reads the row directly (not the full `.epr-meta`
    compose-gate machinery in `_lib.epr_meta.load_policies`, which validates fields this
    Bash-command predicate does not carry, e.g. a file-write `scope`) — this hook IS the
    consumer, so it owns its own minimal, honest validation."""
    try:
        import yaml
    except Exception as e:  # pragma: no cover - PyYAML is vendored in this workspace
        return None, f"PyYAML unavailable ({e!r})"
    try:
        with open(POLICY_FILE) as f:
            data = yaml.safe_load(f) or {}
    except Exception as e:
        return None, f"cannot read/parse {POLICY_FILE}: {e!r}"
    if not isinstance(data, dict) or data.get("epr-meta-policies-version") != 1:
        return None, f"{POLICY_FILE} missing/invalid `epr-meta-policies-version`"
    row = None
    for candidate in data.get("policies") or []:
        if (
            isinstance(candidate, dict)
            and candidate.get("id") == POLICY_ID
            and candidate.get("version") == POLICY_VERSION
            and candidate.get("status") != "superseded"
        ):
            row = candidate
            break
    if row is None:
        return None, f"no active row `{POLICY_REF}` in {POLICY_FILE}"
    if row.get("class") != "deny":
        return None, f"policy `{POLICY_REF}` is not `class: deny` (got {row.get('class')!r})"
    params = row.get("parameters")
    if not isinstance(params, dict):
        return None, f"policy `{POLICY_REF}` missing `parameters` block"
    rules = params.get("rules")
    rm_targets = params.get("rm-force-recursive-targets")
    tier_order = params.get("tier-order")
    tier_floor = params.get("tier-floor")
    if not isinstance(rules, list) or not rules:
        return None, f"policy `{POLICY_REF}` `parameters.rules` missing or empty"
    if not isinstance(rm_targets, list) or not rm_targets:
        return None, f"policy `{POLICY_REF}` `parameters.rm-force-recursive-targets` missing or empty"
    if not isinstance(tier_order, list) or not tier_order:
        return None, f"policy `{POLICY_REF}` `parameters.tier-order` missing or empty"
    if not isinstance(tier_floor, str) or tier_floor not in tier_order:
        return None, (
            f"policy `{POLICY_REF}` `parameters.tier-floor` missing or not present in "
            f"`parameters.tier-order`"
        )
    return {
        "raw": row,
        "rules": rules,
        "rm_targets": set(rm_targets),
        "tier_order": tier_order,
        "tier_floor": tier_floor,
        "unknown_tier": params.get("unknown-tier", "deny"),
        "remedy": params.get("remedy", ""),
    }, None


def _verify_pin(row: dict):
    """(True, None) if the row's `contentHash` matches the SAME canonicalization
    `epr-meta-pin.py` writes (`_lib.epr_meta.policy_content_hash`); else (False, reason). Never
    raises."""
    declared = row.get("contentHash")
    if not declared:
        return False, "row carries no contentHash pin"
    try:
        # The hashing CODE resolves relative to THIS hook's own installed location (sibling
        # `.claude/scripts/_lib/`), never relative to CLAUDE_PROJECT_DIR — the row's DATA comes
        # from the (test-variable) project dir, but the canonicalization logic is fixed code
        # that must be found regardless of which registry is under test.
        lib_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "scripts")
        if lib_dir not in sys.path:
            sys.path.insert(0, lib_dir)
        from _lib import epr_meta as _em  # noqa: PLC0415
        computed = _em.policy_content_hash(row)
    except Exception as e:
        return False, f"cannot compute contentHash ({e!r})"
    if computed != declared:
        return False, f"pinned {declared[:14]}… != computed {computed[:14]}…"
    return True, None


def _latest_claim(session_id: str) -> "str | None":
    """The `claimed` string (`agent:<role>@<model>`) of the LAST claim record in the actor
    sidecar for `session_id`, append-order (claims stack; latest wins) — mirrors
    `ActorStore::current_for` (elohim/epr-rea/src/actor.rs). Missing/unreadable file or no claim
    for this session -> None (honest absence, never an error)."""
    try:
        with open(ACTORS_FILE) as f:
            lines = f.readlines()
    except Exception:
        return None
    latest = None
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except Exception:
            continue
        record = obj.get("record") if isinstance(obj, dict) else None
        if not isinstance(record, dict) or record.get("kind") != "claim":
            continue
        if record.get("session") != session_id:
            continue
        claimed = record.get("claimed")
        if isinstance(claimed, str) and claimed:
            latest = claimed
    return latest


def _resolve_tier(session_id: str) -> str:
    """`CLAUDE_MODEL` env ONLY (fix round 1 dropped `ANTHROPIC_MODEL` — a process-wide config
    value would be a blanket bypass) -> the actor sidecar's latest claim for `session_id` ->
    `unknown`."""
    env_tier = os.environ.get("CLAUDE_MODEL")
    if env_tier and env_tier.strip():
        return env_tier.strip()
    if session_id:
        claimed = _latest_claim(session_id)
        if claimed:
            _, _, model = claimed.partition("@")
            if model:
                return model
    return "unknown"


# ── classification entry point ───────────────────────────────────────────────────────────────

def _classify(command: str, row: dict) -> "str | None":
    """The first destructive invocation found, described as a short human-readable string, or
    None."""
    for head, args in _leaf_commands(command):
        if head == "git":
            sub, sub_args = _split_git_global_opts(args)
            if not sub:
                continue
            rule = _git_rule_matches(sub, sub_args, row["rules"])
            if rule:
                return f"git {sub}" + (f" {' '.join(sub_args)}" if sub_args else "")
        elif head == "rm":
            target = _rm_matches(args, row["rm_targets"])
            if target:
                return f"rm {' '.join(args)}"
    return None


# ── hook contract ─────────────────────────────────────────────────────────────────────────────

def deny(reason: str):
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }))
    sys.exit(0)


def skip(reason: str):
    print(f"capability-tier-gate: skipped — {reason}", file=sys.stderr)
    sys.exit(0)


def main():
    data = json.load(sys.stdin)
    if data.get("tool_name") != "Bash":
        return
    command = (data.get("tool_input") or {}).get("command", "")
    if not command or not _PRE_FILTER.search(command):
        return

    row, err = _load_policy_row()
    if err:
        skip(err)
        return  # unreachable (skip() exits); kept for readability under test/import

    pin_ok, pin_reason = _verify_pin(row["raw"])
    if not pin_ok:
        deny(
            f"policy row {POLICY_REF} fails its pin ({pin_reason}) — refusing every git/rm "
            f"candidate until the registry is re-pinned. Fail-closed: a tampered table cannot "
            f"be trusted to say what ISN'T destructive either."
        )
        return

    matched = _classify(command, row)
    if not matched:
        return

    session_id = os.environ.get("CLAUDE_SESSION_ID", "")
    tier = _resolve_tier(session_id)

    tier_order = row["tier_order"]
    floor = row["tier_floor"]
    floor_idx = tier_order.index(floor)
    tier_idx = tier_order.index(tier) if tier in tier_order else None

    if tier_idx is not None and tier_idx >= floor_idx:
        return  # at/above the declared floor: allowed, silently

    # Below the floor, OR an unresolved/unrecognised tier — `unknown-tier: deny` is fail-closed
    # by design: an unclaimed or unrecognised actor is treated as below the floor, never assumed
    # safe.
    deny(
        f"DESTRUCTIVE GIT ({POLICY_REF}): command matches `{matched}`. "
        f"Resolved tier: {tier} (floor: {floor}; tier-order: {tier_order}). {row['remedy']}"
    )


if __name__ == "__main__":
    try:
        main()
        sys.exit(0)
    except Exception as e:
        # Fail-open, but never silent: a bare `except: exit(0)` swallows a hook bug where a
        # deny should have fired. This must never itself crash, so it names only the TYPE.
        try:
            print(f"capability-tier-gate: skipped — internal error: {type(e).__name__}",
                  file=sys.stderr)
        except Exception:
            pass
        sys.exit(0)
