#!/usr/bin/env python3
"""
Capability Tier Gate — destructive git needs a declared tier or a team check.

Hook Type: PreToolUse   Matcher: Bash

Why (operator, 2026-09-11): a Haiku subagent of another session ran `git reset --hard` on the
shared `dev` checkout — three commits left the branch pointer and ~27 tracked files' uncommitted
work across several lanes was wiped; recovery took two sessions an hour (see
.claude/memory/feedback_tiered_agent_capabilities_destructive_git.md).

The table lives in ONE place: `.claude/epr-meta/policies.yaml` row
`destructive-git-requires-tier@1` (`parameters.rules` — a tokenised subcommand classification,
`parameters.rm-force-recursive-targets` — the `rm -rf` literal target list, tier-order,
tier-floor, unknown-tier, remedy). This hook is a thin reader of that declared table — see the
`capability-tier-gate-owns-destructive-git` rule in `.claude/hooks/.epr-meta`.

FIX ROUND 1: substring matching -> TOKENISED subcommand classification (unwrap bash -c/sh -c/
command/env/an absolute path to git; strip git global options before reading the subcommand);
contentHash pin verification BEFORE trusting the row (fail-closed on tamper); every internal
error spoken, never a bare silent exit; tier resolution drops `ANTHROPIC_MODEL`; remedy carries
no self-claim hint.

FIX ROUND 2 (adversarial re-review, 30+ new payloads: round 1's tokeniser traded a substring
bypass for a HEAD-TOKEN bypass). Round 1 only classified an invocation when the unwrapped HEAD
token literally equaled `git`/`rm`/a shell — so `eval "git reset --hard"`, `$(echo git) reset
--hard`, `xargs git reset --hard`, `echo … | xargs -I{} bash -c '{}'`, `(git reset --hard)`,
`{ git reset --hard; }`, `if …; then git reset --hard; fi`, `bash -lc "…"` / `bash -x -c "…"`,
`nohup|setsid|stdbuf|time git reset --hard`, and `python3 -c "…git reset --hard…"` all resolved
to a non-git/rm/shell head and were silently allowed. This is DENY ON AMBIGUITY: a candidate is
now any CHAIN (a `;`/`&&`/`||`/newline-separated unit, itself possibly multiple `|`-piped
stages) containing a `git` or `rm` word anywhere. A wider wrapper set is stripped before
classification (`eval`, `command`, `env`+assignments, `nohup`, `setsid`, `stdbuf`, `time`,
`timeout <n>`, `nice`, `sudo`, and the bare shell-keyword/paren/brace no-ops `( ) { } if then
else elif fi do done while until`); a shell (`bash`/`sh`/`zsh`) is unwrapped whenever ANY of its
leading option tokens carries the letter `c` (`-c`, `-lc`, `-x -c`, `-euo pipefail -c`),
recursing into its script string. If, after all that, the chain is STILL indirect — command
substitution (`$(`/backtick) inside a token, `xargs` as any stage's head, or an interpreter
(`python*`/`perl`/`ruby`/`node`) whose argument string carries a `git`/`rm` word — the WHOLE
chain denies with the reason `indirect invocation carrying a destructive token; run it plainly
or ask the controller`, unconditionally (no tier can clear it: an ambiguous invocation is not
something a floor can bless). `echo`/`printf`/`grep`/`cat`/`rg` prose stays exempt ONLY as a
single-stage chain (no `|`, no `xargs` downstream) — piping prose into `xargs`/`bash -c` voids
the exemption. Session key: this harness's real session-id env var is `CLAUDE_CODE_SESSION_ID`
(round 1 read only `CLAUDE_SESSION_ID`, which this harness never sets, so the sidecar lookup
never matched and every actor silently resolved `unknown`) — read `CLAUDE_CODE_SESSION_ID`
first, `CLAUDE_SESSION_ID` as a fallback. `CLAUDE_MODEL` is not exported by this harness either;
in practice the ladder is honour-system via `epr actor claim`, and absent a claim it is a
uniform deny.

Deny shape copies `cargo-disk-guard.py`'s exact convention: one `hookSpecificOutput` JSON object
on stdout (`permissionDecision: deny`), process exit 0.
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

INDIRECT_REASON = (
    "indirect invocation carrying a destructive token; run it plainly or ask the controller"
)

# Cheap pre-filter: only a Bash command that mentions `git` or `rm` can possibly contain a
# destructive invocation, so everything else returns before touching policies.yaml (or the
# actor sidecar, or the pin-hashing lib) at all.
_PRE_FILTER = re.compile(r"git|rm")

_MAX_UNWRAP_DEPTH = 4

# ── tokenisation ──────────────────────────────────────────────────────────────────────────────

_TOP_OPERATORS = {"&&", "||", ";", "&"}
_PIPE_OP = "|"
_ENV_ASSIGN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")

# Wrappers whose head is stripped and whose own (dash-prefixed) options are skipped; `timeout`
# additionally consumes its one numeric duration argument. `eval` is handled separately (its
# remaining tokens are REJOINED into a string and re-scanned, like a shell's `-c` argument).
_WRAPPERS = {"timeout", "nice", "env", "command", "sudo", "nohup", "setsid", "stdbuf", "time"}
# Bare shell-keyword / paren / brace no-ops: skip ONE token, no argument consumed.
_NOOP_LEADING = {"(", ")", "{", "}", "if", "then", "else", "elif", "fi", "do", "done", "while", "until"}
_SHELLS = {"bash", "sh", "dash", "zsh"}
_NEVER_CLASSIFY_HEADS = {"echo", "printf", "grep", "cat", "rg"}
_INTERPRETER_RE = re.compile(r"^(python\d*(\.\d+)?|perl\d?|ruby|node|nodejs)$")
_GIT_RM_WORD_RE = re.compile(r"\bgit\b|\brm\b")

# `(` and `)`/`{`/`}` are spaced out so shlex yields them as their own tokens (it has no shell
# syntax awareness and otherwise glues them to an adjacent word, e.g. "(git" / "hard)"). A `(`
# immediately preceded by `$` is left GLUED on purpose — that is what keeps a `$(` command-
# substitution marker intact as a single detectable token instead of being torn apart into `$`
# and `(`.
_PAREN_BRACE_RE = re.compile(r"(?<!\$)([(){}])")


def _tokenize(text: str) -> list:
    text = text.replace("\n", " ; ")
    text = _PAREN_BRACE_RE.sub(r" \1 ", text)
    try:
        return shlex.split(text)
    except ValueError:
        return []


def _chains(command: str) -> list:
    """List of chains; each chain is a list of stages; each stage is a list of tokens. Chains
    are independent units separated by `;`/`&&`/`||`/`&`/newline; stages within one chain are
    `|`-piped (so a pipeline's stages travel together for indirection analysis)."""
    toks = _tokenize(command)
    chains, cur_chain, cur_stage = [], [], []
    for t in toks:
        if t in _TOP_OPERATORS:
            if cur_stage:
                cur_chain.append(cur_stage)
            if cur_chain:
                chains.append(cur_chain)
            cur_chain, cur_stage = [], []
            continue
        if t == _PIPE_OP:
            if cur_stage:
                cur_chain.append(cur_stage)
            cur_stage = []
            continue
        if t.endswith(";") and t not in _TOP_OPERATORS:
            cur_stage.append(t.rstrip(";"))
            cur_chain.append(cur_stage)
            chains.append(cur_chain)
            cur_chain, cur_stage = [], []
            continue
        cur_stage.append(t)
    if cur_stage:
        cur_chain.append(cur_stage)
    if cur_chain:
        chains.append(cur_chain)
    return chains


def _strip_wrappers(toks: list) -> list:
    """Strip leading `VAR=val` env assignments, bare shell-keyword/paren/brace no-ops, and
    wrapper heads (`env`, `command`, `timeout <n>`, `nice`, `sudo`, `nohup`, `setsid`, `stdbuf`,
    `time` + their own dash-prefixed options) so the REAL head is toks[0] afterward. `eval` and
    shells are handled separately by the caller (they recurse into a re-joined/`-c` string,
    rather than simply continuing with the same token list)."""
    i = 0
    while i < len(toks):
        if _ENV_ASSIGN.match(toks[i]):
            i += 1
            continue
        base = os.path.basename(toks[i])
        if base in _NOOP_LEADING:
            i += 1
            continue
        if base in _WRAPPERS:
            i += 1
            if base == "timeout":
                if i < len(toks) and re.match(r"^[\d.]+[smhd]?$", toks[i]):
                    i += 1
            else:
                while i < len(toks) and toks[i].startswith("-"):
                    i += 1
            continue
        break
    return toks[i:]


def _find_dash_c(tokens: list):
    """Index of the shell `-c`-equivalent option among a shell's argv (combined short clusters
    like `-lc` count — any leading dash-option token carrying the letter `c`), or None."""
    for i, t in enumerate(tokens):
        if t == "-c":
            return i
        if t.startswith("-") and not t.startswith("--") and len(t) > 1 and "c" in t[1:]:
            return i
    return None


def _is_interpreter(head: str) -> bool:
    return bool(_INTERPRETER_RE.match(head))


# ── indirection (DENY ON AMBIGUITY) ──────────────────────────────────────────────────────────

def _chain_is_indirect(chain: list) -> bool:
    for stage in chain:
        for t in stage:
            if "$(" in t or "`" in t:
                return True
        toks = _strip_wrappers(stage)
        if not toks:
            continue
        head = os.path.basename(toks[0])
        if head == "xargs":
            return True
        if _is_interpreter(head):
            rest_text = " ".join(toks[1:])
            if _GIT_RM_WORD_RE.search(rest_text):
                return True
    return False


def _chain_has_git_or_rm_word(chain: list) -> bool:
    text = " ".join(t for stage in chain for t in stage)
    return bool(_GIT_RM_WORD_RE.search(text))


# ── git subcommand rule evaluation ───────────────────────────────────────────────────────────

def _flag_present(argv: list, flag: str) -> bool:
    return any(t == flag or t.startswith(flag + "=") for t in argv)


def _flag_letters_present(argv: list, letters: list) -> bool:
    wanted = set(letters)
    for t in argv:
        if t.startswith("-") and not t.startswith("--") and len(t) > 1:
            if wanted & set(t[1:]):
                return True
    return False


def _arg_prefix_present(argv: list, prefixes: list) -> bool:
    return any(t.startswith(p) for t in argv for p in prefixes)


def _positional_count(argv: list) -> int:
    return sum(1 for t in argv if not t.startswith("-"))


def _git_rule_matches(sub: str, args: list, rules: list):
    """The first declared rule (registry order) whose `sub` matches and whose predicate keys
    all hold (AND across keys present on ONE rule; a rule with no predicate keys matches the
    bare subcommand unconditionally). `exempt_if_any_args` is a final override: if any of its
    tokens is present, this rule does NOT match, regardless of everything else. Multiple rows
    sharing one `sub` OR together (each is checked independently)."""
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
        if "any_arg_prefix" in rule:
            ok = ok and _arg_prefix_present(args, rule["any_arg_prefix"])
        if "min_positional_args" in rule:
            ok = ok and _positional_count(args) >= rule["min_positional_args"]
        if ok and "exempt_if_any_args" in rule:
            if any(a in args for a in rule["exempt_if_any_args"]):
                ok = False
        if ok:
            return rule
    return None


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


# ── rm -rf / -fr / -r -f (the SHELL command) ─────────────────────────────────────────────────

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
    """`CLAUDE_MODEL` env (not exported by this harness in practice; the ladder is honour-
    system via `epr actor claim`) -> the actor sidecar's latest claim for `session_id` ->
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


def _session_id_from_env() -> str:
    """`CLAUDE_CODE_SESSION_ID` first (this harness's real session-id env var — round 1 read
    only `CLAUDE_SESSION_ID`, which this harness never sets, so the sidecar lookup never
    matched), `CLAUDE_SESSION_ID` as a fallback for any harness that does use it."""
    return os.environ.get("CLAUDE_CODE_SESSION_ID") or os.environ.get("CLAUDE_SESSION_ID") or ""


# ── classification entry point (DENY ON AMBIGUITY) ───────────────────────────────────────────

INDIRECT = "__INDIRECT__"


def _scan(command: str, row: dict, depth: int = 0) -> "str | None":
    """None (allowed) | INDIRECT sentinel | a short match description, for the first destructive
    or indirect invocation found in `command`."""
    if depth > _MAX_UNWRAP_DEPTH:
        return None
    for chain in _chains(command):
        if len(chain) == 1:
            toks = _strip_wrappers(chain[0])
            if toks and os.path.basename(toks[0]) in _NEVER_CLASSIFY_HEADS:
                continue  # prose, single stage, no pipe/xargs downstream: exempt

        if _chain_is_indirect(chain) and _chain_has_git_or_rm_word(chain):
            return INDIRECT

        for stage in chain:
            toks = _strip_wrappers(stage)
            if not toks:
                continue
            head = os.path.basename(toks[0])
            if head in _NEVER_CLASSIFY_HEADS:
                continue
            if head in _SHELLS:
                idx = _find_dash_c(toks[1:])
                if idx is not None and idx + 1 < len(toks[1:]):
                    result = _scan(toks[1:][idx + 1], row, depth + 1)
                    if result:
                        return result
                continue
            if head == "eval":
                rest = toks[1:]
                if rest:
                    result = _scan(" ".join(rest), row, depth + 1)
                    if result:
                        return result
                continue
            if head == "git":
                sub, sub_args = _split_git_global_opts(toks[1:])
                if sub:
                    rule = _git_rule_matches(sub, sub_args, row["rules"])
                    if rule:
                        return f"git {sub}" + (f" {' '.join(sub_args)}" if sub_args else "")
                continue
            if head == "rm":
                target = _rm_matches(toks[1:], row["rm_targets"])
                if target:
                    return f"rm {' '.join(toks[1:])}"
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

    matched = _scan(command, row, depth=0)
    if not matched:
        return

    if matched == INDIRECT:
        # DENY ON AMBIGUITY: an indirect invocation bypasses tier resolution entirely — we
        # cannot confirm what it actually runs, so no declared floor can clear it.
        deny(INDIRECT_REASON)
        return

    session_id = _session_id_from_env()
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
