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

FIX ROUND 1: substring matching -> TOKENISED subcommand classification; contentHash pin
verification BEFORE trusting the row (fail-closed on tamper); every internal error spoken;
tier resolution drops `ANTHROPIC_MODEL`; remedy carries no self-claim hint.

FIX ROUND 2: DENY ON AMBIGUITY — a wider wrapper/keyword strip set, flexible shell `-c`
detection, and an unconditional deny (bypassing tier) for a chain that is still indirect after
unwrapping (command substitution, `xargs`, an interpreter carrying a git/rm word). Session key
fixed to `CLAUDE_CODE_SESSION_ID` (this harness's real var), `CLAUDE_SESSION_ID` as fallback.

FIX ROUND 3 (third adversarial pass, 25/25 tests still green — two problems, one in each
direction). STILL ALLOWED: a quote-split word (`g""it reset --hard` — the RAW-STRING pre-filter
missed it although shlex merges it to `git`); `git${IFS}reset${IFS}--hard` / `git reset
$'--hard'` (shell variable-expansion / ANSI-C-quote gluing); `GIT=git; $GIT reset --hard`
(variable head); `f(){ git reset --hard; }; f` (function body); `bash <<< "…"` / `printf … | sh`
/ `echo … | sh` (a bare shell as a heredoc/here-string target or pipeline SINK wasn't treated as
indirect); `find . -exec git checkout -- . ;` (find -exec is xargs-equivalent, terminator
escaped in real usage). NOW
OVER-BLOCKED (denied for every tier — load-bearing, would block routine work): `cd
$(git rev-parse --show-toplevel) && ls`, `git log --oneline $(git merge-base HEAD main)`,
`export SHA=$(git rev-parse HEAD)`, `rm -rf $(mktemp -d)`, `git commit -m 'fix rm handling'`,
`find . -name '*.rs' | xargs grep -l git`, `python3 script.py --git-dir x` (`\bgit\b` hit
`--git-dir`) — round 2's blanket "an indirection marker anywhere + a git/rm word anywhere in the
chain" denied everyone the moment a command substitution held ANY git mention, destructive or
not.

The fix: (1) the pre-filter runs on TOKENS (shlex-merged), never the raw string, so a
quote-split word is never missed. (2) A stage's stripped HEAD starting with `$` or a backtick is
unconditionally indirect (a variable/substitution used AS THE COMMAND — we cannot know what it
resolves to); a `NAME(){ BODY; }` function-definition preamble is stripped so the BODY still
classifies directly. A raw-text scan catches `${...}`/`$'...'` gluing adjacent to a git/rm word
(`git${IFS}reset`, `git reset $'--hard'`) as indirect too. (3) A `$( )`/backtick substitution used
as an ARGUMENT (not the head) is SCOPED: its inner text is extracted and itself classified —
ambiguous (deny) only when that inner text is destructive, or is git/rm-headed but not one of a
small read-only allowlist (`rev-parse`, `merge-base`, `log`, `status`, `describe`, `ls-files`,
`diff`, `show`, `remote`, `rev-list`, `config --get`, `branch --show-current`, a read-form
`symbolic-ref`); a non-git/rm inner head (`mktemp`, `date`, `pwd`, …) is never ambiguous. `xargs`
and `find -exec/-execdir` resolve their REAL trailing command and classify it the SAME way a
plain invocation would (`xargs grep -l git` is allowed — grep is not git/rm; `xargs git reset
--hard` denies via the ordinary destructive-git reason, not the generic indirect one); only when
the resolved command is itself a shell/`eval` (the "run whatever came down the pipe" shape) do we
fall back to inspecting the upstream payload (an `echo`/`printf` stage's own arguments, or a
`<<<` here-string's text) and deny with the indirect reason if THAT payload is destructive — an
unrecoverable upstream (anything else) defaults to deny, since we cannot verify it's safe. The
interpreter word-scan now skips dash-prefixed tokens entirely (a flag like `--git-dir` is never
"free text"), so `\bgit\b` no longer fires on an option name.

FIX ROUND 4 (fourth adversarial pass — three misses, one over-block class). MISSES, all plainly
typed and all ALLOWED before this round: (a) UNSPACED SHELL OPERATORS — `git reset --hard;echo x`,
`...&&echo x`, `...||true`, `...&`, `...>log`, `cd /dir&&git reset --hard`: shlex only splits on
whitespace, so `--hard;echo` never equalled `--hard`. Fixed by a quote-aware pre-tokenisation pass
(`_space_operators`) that inserts spaces around `;`/`&&`/`||`/`|`/`&`/`>`/`>>`/`<`/`2>`/… OUTSIDE
quotes and outside backslash escapes, plus redirect-token dropping (`_drop_redirects`) so a
redirect and its target never count as a subcommand's positional arguments. (b) `exec git reset
--hard`, `exec -a foo git …`, `builtin eval git …`: `exec`/`builtin` joined the wrapper set, with a
value-consuming option table (`exec -a NAME`, `env -u`, `nice -n`, `sudo -u`, …) so an option's
separate VALUE is never mistaken for the real head. (c) `xargs -n 1 git reset --hard`, `xargs -a
list.txt git …`: `_xargs_real_command` skipped only dash tokens, so an option's value became the
head — now a declared xargs option table consumes values (`-n -a -I -i -L -P -s -d -E --max-args
--arg-file --replace --max-procs --delimiter …`).

OVER-BLOCK, denied at EVERY tier before this round with the INDIRECT reason (which bypasses tier
resolution entirely): `git log --oneline ${SHA}`, `git commit -m "${MSG}"`, `git checkout
${BRANCH}`, `git add -- ${FILES}`, `cd "${PROJECT_DIR}" && git status`, `rm -rf
"${TMPDIR}/scratch"`, `git log --format=$'%h %s'` — round 3's `_has_glued_var_obfuscation` denied
any `${…}`/`$'…'` within a 20-character WINDOW of a git/rm word. Replaced by `_glue_affected` /
`_deglue`: a hit requires the `$`-construct to be fused INSIDE ONE TOKEN (its immediate neighbour
in that token is a letter/digit/underscore — never a quote, never `/`, never the token boundary),
or to be an ANSI-C `$'…'` quote (which shlex silently corrupts into a literal `$` prefix). A hit
no longer short-circuits to INDIRECT: the de-glued form is scanned through the NORMAL tier path,
so `git${IFS}reset${IFS}--hard` denies at haiku exactly like `git reset --hard` does, and a
fable-tier controller is not blocked either way. `${VAR}` standing alone as an argument, quoted,
or used as a path component is never a hit.

FIX ROUND 5 (fifth and final adversarial pass — two pre-existing misses). (a) BACKSLASH-NEWLINE
LINE CONTINUATION, load-bearing because a wrapped multi-line command is ordinary typing: `_tokenize`
substituted newline -> ` ; ` BEFORE anything handled backslash escapes, so a trailing backslash
left `--hard ` (trailing space, matching no declared flag) and a command wrapped between `reset`
and `--hard` split into two separate chains. Continuation lines are now JOINED (`_JOIN_CONTINUATION_RE`,
backslash + newline -> one space) before the newline substitution. (b) `rm -rf "$PWD"` / `$PWD` /
`${PWD}` / `$HOME` / `${HOME}` / `rm -rf -- "$PWD"`: a bare `$VAR` is not in-token gluing and none
of these appeared in the declared `rm-force-recursive-targets`, so every one was ALLOWED. A
code-level target set (`_RM_DESTRUCTIVE_VAR_TARGETS`) now names the shell variables whose loss IS
the incident this gate exists for, unioned with the declared literal list (the policy row is
unchanged this round); a trailing slash is normalised away (`$PWD/`, `../`) while a path BELOW one
of them (`rm -rf "$PWD/target"`) stays allowed. Leading `VAR=value` assignments in the same command
are also resolved into a substituted variant scanned alongside the original, so `MODE=--hard git
reset ${MODE}` classifies with the value — an ADDITIONAL scan through the ordinary tier path, never
a short-circuit.

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

INDIRECT = "__INDIRECT__"
INDIRECT_REASON = (
    "indirect invocation carrying a destructive token; run it plainly or ask the controller"
)

_MAX_UNWRAP_DEPTH = 4

# ── tokenisation ──────────────────────────────────────────────────────────────────────────────

_TOP_OPERATORS = {"&&", "||", ";", "&"}
_PIPE_OP = "|"
_ENV_ASSIGN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")

_WRAPPERS = {"timeout", "nice", "env", "command", "sudo", "nohup", "setsid", "stdbuf", "time",
             "exec", "builtin"}
# Round 4: a wrapper option that consumes a SEPARATE value — skipping only the dash token leaves
# the value standing where the real head should be (`exec -a foo git reset --hard` -> head `foo`).
_WRAPPER_VALUE_OPTS = {
    "exec": {"-a"},
    "env": {"-u", "--unset", "-C", "--chdir", "-S", "--split-string"},
    "nice": {"-n", "--adjustment"},
    "sudo": {"-u", "--user", "-g", "--group", "-p", "--prompt", "-C", "--close-from"},
    "stdbuf": {"-i", "-o", "-e", "--input", "--output", "--error"},
}
_NOOP_LEADING = {"(", ")", "{", "}", "if", "then", "else", "elif", "fi", "do", "done", "while", "until"}
_SHELLS = {"bash", "sh", "dash", "zsh"}
_NEVER_CLASSIFY_HEADS = {"echo", "printf", "grep", "cat", "rg"}
_INTERPRETER_RE = re.compile(r"^(python\d*(\.\d+)?|perl\d?|ruby|node|nodejs)$")
_GIT_RM_WORD_RE = re.compile(r"\bgit\b|\brm\b")

# `(`/`)`/`{`/`}` are spaced out so shlex yields them as their own tokens (bare shell no-ops); a
# `(` immediately preceded by `$` is left GLUED so a `$(` command-substitution marker survives
# as one detectable token instead of being torn into `$` and `(`.
_PAREN_BRACE_RE = re.compile(r"(?<!\$)([(){}])")

# Round 4: shell OPERATORS need no surrounding whitespace — `git reset --hard;echo x` is two
# commands to the shell but one `--hard;echo` token to shlex. Longest-first so `&&` never reads as
# two `&`, `>>` never as two `>`, `<<<` never as `<<` + `<`.
_OPERATOR_UNITS = ("2>&1", "&>>", "<<<", ">>", "<<", "&&", "||", "&>", ">&", "2>",
                   ";", "|", "&", ">", "<")
_TOKEN_BOUNDARY_BEFORE = set(" \t\n|&;<>()")

# A redirection and its target are never part of the command's own arguments (`git symbolic-ref
# HEAD > out` has ONE positional arg, not three). `<<<` is deliberately NOT matched here — the
# bare-shell here-string branch needs it.
_REDIR_RE = re.compile(r"^(?:\d?(?:>>|>|<)&?\d*|&>>?)$")
_REDIR_CARRIES_ITS_TARGET_RE = re.compile(r"&\d+$")

# Round 4 (replacing round 3's 20-character WINDOW scan, which denied every `${…}` near a git/rm
# word): the `$`-construct shapes that do not survive tokenisation as the shell would read them.
_VAR_BRACE_RE = re.compile(r"\$\{[^}]{1,200}\}")
_ANSI_C_RE = re.compile(r"\$'(?:[^'\\]|\\.){0,200}'")
_GLUE_WORD_CHAR_RE = re.compile(r"[A-Za-z0-9_]")

# Round 5: a backslash-newline is a LINE CONTINUATION — the shell joins the lines into one command
# before it ever splits words. Joining must happen before the newline -> `;` substitution below, or
# `git reset --hard\` + newline tokenises as `--hard ` (trailing space, matching no declared flag)
# and a wrapped `git reset \<NL> --hard` reads as two unrelated chains.
_JOIN_CONTINUATION_RE = re.compile(r"\\[ \t]*\r?\n")

# A `${…}` expansion span — its closing `}` belongs to the expansion, never to a shell block.
_DOLLAR_BRACE_SPAN_RE = re.compile(r"\$\{[^}]*\}")

# Round 5: shell variables naming a directory whose recursive removal IS the incident this gate
# exists for. A bare `$VAR` is neither in-token gluing nor a declared literal target, so `rm -rf
# "$PWD"` was allowed at every tier. Code-level (the declared row is unchanged this round) and
# unioned with `parameters.rm-force-recursive-targets`.
_RM_DESTRUCTIVE_VAR_TARGETS = {
    "$PWD", "${PWD}", "$OLDPWD", "${OLDPWD}", "$HOME", "${HOME}",
    "$CLAUDE_PROJECT_DIR", "${CLAUDE_PROJECT_DIR}", "$REPO_ROOT", "${REPO_ROOT}",
}

# Round 5: a leading `VAR=value` assignment is part of the SAME command — `MODE=--hard git reset
# ${MODE}` runs a hard reset. Resolved into a substituted variant scanned ALONGSIDE the original.
_ASSIGNMENT_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)=(.*)$", re.S)


def _space_operators(text: str) -> str:
    """Insert spaces around shell operators found OUTSIDE quotes and outside backslash escapes,
    so shlex's whitespace-only splitting sees them as their own tokens. A backslash-escaped
    character is copied through untouched, so a find-exec terminator keeps behaving as before."""
    out = []
    i = 0
    n = len(text)
    quote = None
    while i < n:
        ch = text[i]
        if quote:
            out.append(ch)
            if ch == "\\" and quote == '"' and i + 1 < n:
                out.append(text[i + 1])
                i += 2
                continue
            if ch == quote:
                quote = None
            i += 1
            continue
        if ch == "\\":
            out.append(ch)
            if i + 1 < n:
                out.append(text[i + 1])
            i += 2
            continue
        if ch in ("'", '"'):
            quote = ch
            out.append(ch)
            i += 1
            continue
        matched = None
        for op in _OPERATOR_UNITS:
            if not text.startswith(op, i):
                continue
            if op[0].isdigit() and not (i == 0 or text[i - 1] in _TOKEN_BOUNDARY_BEFORE):
                continue  # `foo2>x`: the digit belongs to the word, not to a `2>` redirect
            matched = op
            break
        if matched:
            out.append(" " + matched + " ")
            i += len(matched)
            continue
        out.append(ch)
        i += 1
    return "".join(out)


def _drop_redirects(stage: list) -> list:
    out = []
    i = 0
    while i < len(stage):
        t = stage[i]
        if _REDIR_RE.match(t):
            i += 1 if _REDIR_CARRIES_ITS_TARGET_RE.search(t) else 2
            continue
        out.append(t)
        i += 1
    return out


def _space_parens_braces(text: str) -> str:
    """Space out bare parens/braces, but never INSIDE a `${…}` span: the closing `}` of `${PWD}` is
    the expansion's own syntax, not a shell block terminator, and splitting it left `rm -rf ${PWD}`
    tokenised as `${PWD` + `}` — matching no declared target (round 5)."""
    out = []
    pos = 0
    for m in _DOLLAR_BRACE_SPAN_RE.finditer(text):
        out.append(_PAREN_BRACE_RE.sub(r" \1 ", text[pos:m.start()]))
        out.append(m.group(0))
        pos = m.end()
    out.append(_PAREN_BRACE_RE.sub(r" \1 ", text[pos:]))
    return "".join(out)


def _tokenize(text: str) -> list:
    text = _JOIN_CONTINUATION_RE.sub(" ", text)
    text = text.replace("\n", " ; ")
    text = _space_operators(text)
    text = _space_parens_braces(text)
    try:
        return shlex.split(text)
    except ValueError:
        return []


def _chains(command: str) -> list:
    """List of chains; each chain is a list of stages; each stage is a list of tokens. Chains
    are independent units separated by `;`/`&&`/`||`/`&`/newline; stages within one chain are
    `|`-piped (so a pipeline's stages travel together)."""
    toks = _tokenize(command)
    chains, cur_chain, cur_stage = [], [], []

    def _close_stage(stage):
        stage = _drop_redirects(stage)
        if stage:
            cur_chain.append(stage)

    for t in toks:
        if t in _TOP_OPERATORS:
            if cur_stage:
                _close_stage(cur_stage)
            if cur_chain:
                chains.append(cur_chain)
            cur_chain, cur_stage = [], []
            continue
        if t == _PIPE_OP:
            if cur_stage:
                _close_stage(cur_stage)
            cur_stage = []
            continue
        if t.endswith(";") and t not in _TOP_OPERATORS:
            cur_stage.append(t.rstrip(";"))
            _close_stage(cur_stage)
            chains.append(cur_chain)
            cur_chain, cur_stage = [], []
            continue
        cur_stage.append(t)
    if cur_stage:
        _close_stage(cur_stage)
    if cur_chain:
        chains.append(cur_chain)
    return chains


def _strip_wrappers(toks: list) -> list:
    """Strip leading `VAR=val` assignments, a `NAME(){` function-definition preamble, bare
    shell-keyword/paren/brace no-ops, and wrapper heads (their own dash-prefixed options
    included) so the REAL head is toks[0] afterward. `eval` and shells recurse elsewhere."""
    i = 0
    while i < len(toks):
        if _ENV_ASSIGN.match(toks[i]):
            i += 1
            continue
        if i + 3 < len(toks) and toks[i + 1] == "(" and toks[i + 2] == ")" and toks[i + 3] == "{":
            i += 4  # `NAME(){` -- classify the function BODY, not the definition
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
                value_opts = _WRAPPER_VALUE_OPTS.get(base, ())
                while i < len(toks) and toks[i].startswith("-"):
                    i += 2 if toks[i] in value_opts else 1
            continue
        break
    return toks[i:]


def _find_dash_c(tokens: list):
    """Index of the shell `-c`-equivalent option (combined clusters like `-lc` count), or None."""
    for i, t in enumerate(tokens):
        if t == "-c":
            return i
        if t.startswith("-") and not t.startswith("--") and len(t) > 1 and "c" in t[1:]:
            return i
    return None


def _is_interpreter(head: str) -> bool:
    return bool(_INTERPRETER_RE.match(head))


def _text_carries_git_or_rm_word(text: str) -> bool:
    return bool(_GIT_RM_WORD_RE.search(text))


def _interpreter_args_carry_destructive_word(argv: list) -> bool:
    """FREE-TEXT scan of an interpreter's positional args (its `-c` script string, typically).
    Dash-prefixed tokens are never "free text" — `--git-dir` is an option name, not prose, and a
    naive word-boundary scan would otherwise fire on it (a hyphen satisfies `\\b` on both sides)."""
    for t in argv:
        if t.startswith("-"):
            continue
        if _text_carries_git_or_rm_word(t):
            return True
    return False


def _raw_words(command: str) -> list:
    """The RAW text split on unquoted whitespace, quotes and escapes preserved — one entry per
    shell WORD, which is the unit in-token gluing is defined against."""
    words, cur, quote = [], [], None
    i = 0
    n = len(command)
    while i < n:
        ch = command[i]
        if quote:
            cur.append(ch)
            if ch == quote:
                quote = None
            i += 1
            continue
        if ch == "\\" and i + 1 < n:
            cur.append(ch)
            cur.append(command[i + 1])
            i += 2
            continue
        if ch in ("'", '"'):
            quote = ch
            cur.append(ch)
            i += 1
            continue
        if ch.isspace():
            if cur:
                words.append("".join(cur))
                cur = []
            i += 1
            continue
        cur.append(ch)
        i += 1
    if cur:
        words.append("".join(cur))
    return words


def _glue_affected(command: str) -> bool:
    """True iff some `$`-construct is fused INSIDE one shell word in a way shlex cannot reproduce:
    a `${…}` whose immediate neighbour within that word is a letter/digit/underscore
    (`git${IFS}reset`, `g${X}t`), or ANY ANSI-C `$'…'` quote (shlex leaves its `$` as a literal
    character, so `$'--hard'` tokenises as `$--hard` and matches no declared flag).

    A `${VAR}` standing alone as an argument (`git log --oneline ${SHA}`), quoted (`git commit -m
    "${MSG}"`), or used as a path component (`rm -rf "${TMPDIR}/scratch"`) is NEVER a hit — that
    over-block was round 3's 20-character window scan, and it denied routine work at every tier."""
    for word in _raw_words(command):
        if _ANSI_C_RE.search(word):
            return True
        for m in _VAR_BRACE_RE.finditer(word):
            before = word[m.start() - 1] if m.start() > 0 else ""
            after = word[m.end()] if m.end() < len(word) else ""
            if (before and _GLUE_WORD_CHAR_RE.match(before)) or (
                after and _GLUE_WORD_CHAR_RE.match(after)
            ):
                return True
    return False


def _resolve_leading_assignments(command: str) -> "str | None":
    """The command with any `VAR=value` assignment it carries substituted into later `$VAR` /
    `${VAR}` references, or None when it carries no assignment that is actually referenced. The
    result is scanned IN ADDITION to the original, through the ordinary tier path."""
    assignments = {}
    for chain in _chains(command):
        for stage in chain:
            for tok in stage:
                m = _ASSIGNMENT_RE.match(tok)
                if m and m.group(2):
                    assignments.setdefault(m.group(1), m.group(2))
    if not assignments:
        return None
    out = command
    changed = False
    for name, value in assignments.items():
        pattern = re.compile(r"\$\{" + re.escape(name) + r"\}|\$" + re.escape(name) + r"\b")
        out, n = pattern.subn(value.replace("\\", "\\\\"), out)
        changed = changed or bool(n)
    return out if changed else None


def _deglue(command: str) -> str:
    """The command as it reads once the `$`-constructs resolve away: `${…}` becomes the word break
    it was hiding, `$'X'` becomes the ordinary `'X'` the shell would hand the command. Scanned IN
    ADDITION to the original text, through the normal tier path — never as a short-circuit deny."""
    out = _ANSI_C_RE.sub(lambda m: m.group(0)[1:], command)
    return _VAR_BRACE_RE.sub(" ", out)


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
    tokens is present, this rule does NOT match. Multiple rows sharing one `sub` OR together."""
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
    friends consume the option AND their following value; git subcommands never start with `-`."""
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
        i += 1
    return None, []


def _classify_argv(argv: list, row: dict) -> "str | None":
    """Classify an already-split argv (head = argv[0]) exactly like a plain top-level
    invocation — shared by the direct stage path, `xargs`'s real trailing command, and
    `find -exec`'s exec'd command."""
    if not argv:
        return None
    head = os.path.basename(argv[0])
    if head == "git":
        sub, sub_args = _split_git_global_opts(argv[1:])
        if not sub:
            return None
        rule = _git_rule_matches(sub, sub_args, row["rules"])
        if rule:
            return f"git {sub}" + (f" {' '.join(sub_args)}" if sub_args else "")
        return None
    if head == "rm":
        target = _rm_matches(argv[1:], row["rm_targets"])
        if target:
            return f"rm {' '.join(argv[1:])}"
        return None
    return None


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
    # `$PWD/` and `../` name the same directory as `$PWD` and `..`; a path BELOW one of them
    # (`$PWD/target`) is a different, ordinary thing and stays allowed.
    normalised = target[:-1] if len(target) > 1 and target.endswith("/") else target
    if target in literal_targets or normalised in literal_targets:
        return True
    if target in _RM_DESTRUCTIVE_VAR_TARGETS or normalised in _RM_DESTRUCTIVE_VAR_TARGETS:
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


# ── xargs / find -exec real-command resolution ───────────────────────────────────────────────

# Round 4: xargs options that consume a SEPARATE value. Skipping only the dash token leaves the
# value standing where the real command should be — `xargs -n 1 git reset --hard` read its head as
# `1` and allowed the reset.
_XARGS_VALUE_OPTS = {
    "-n", "-a", "-I", "-i", "-L", "-P", "-s", "-d", "-E",
    "--max-args", "--arg-file", "--replace", "--max-lines", "--max-procs", "--max-chars",
    "--delimiter", "--eof", "--process-slot-var",
}


def _xargs_real_command(args: list) -> list:
    """Skip xargs's own options — value-consuming ones (`-n 1`, `-a list.txt`, `-I {}`) take their
    value with them — AND stray `{`/`}` tokens, since the paren/brace spacer (`_PAREN_BRACE_RE`)
    splits a glued placeholder like `-I{}` into `-I`, `{`, `}` and those braces are xargs's own
    replacement-string syntax, never the start of the real command."""
    i = 0
    while i < len(args):
        t = args[i]
        if t in ("{", "}"):
            i += 1
            continue
        if t.startswith("-"):
            i += 2 if t in _XARGS_VALUE_OPTS else 1
            continue
        break
    return args[i:]


def _find_exec_commands(args: list) -> list:
    """Token lists for each `-exec`/`-execdir ... {} ;|+` clause in a `find` stage's argv
    (argv AFTER the `find` head). The `;` terminator is almost always already consumed as a
    chain-separator by `_chains` (an escaped `\\;` unescapes to a bare `;` token), so this simply
    collects to the end of the available tokens and trims a literal trailing `;`/`+`."""
    out = []
    i = 0
    n = len(args)
    while i < n:
        if args[i] in ("-exec", "-execdir"):
            cmd = args[i + 1:]
            if cmd and cmd[-1] in (";", "+"):
                cmd = cmd[:-1]
            if cmd:
                out.append(cmd)
            break
        i += 1
    return out


def _stage_payload_text(stage_toks: list) -> "str | None":
    """Best-effort literal text a stage would emit downstream: `echo`/`printf`'s own arguments,
    joined. None for any other head — we cannot see what it would actually output."""
    toks = _strip_wrappers(stage_toks)
    if not toks:
        return None
    head = os.path.basename(toks[0])
    if head in ("echo", "printf"):
        return " ".join(toks[1:])
    return None


# ── $()/backtick argument-position substitutions, SCOPED to their inner text ─────────────────

_INNER_SAFE_GIT_SUBS = {
    "rev-parse", "merge-base", "log", "status", "describe", "ls-files",
    "diff", "show", "remote", "rev-list",
}


def _extract_paren_substitutions(chain: list) -> list:
    """Inner-text strings for each `$(...)` span found across a chain's tokens (token-based,
    depth-tracked via '(' / ')' occurrence counts across the collected tokens)."""
    flat = [t for stage in chain for t in stage]
    spans = []
    i = 0
    n = len(flat)
    while i < n:
        idx = flat[i].find("$(")
        if idx == -1:
            i += 1
            continue
        remainder = flat[i][idx + 2:]
        depth = 1 + remainder.count("(") - remainder.count(")")
        collected = [remainder]
        j = i
        while depth > 0 and j + 1 < n:
            j += 1
            seg = flat[j]
            depth += seg.count("(") - seg.count(")")
            collected.append(seg)
        if collected:
            collected[-1] = collected[-1].rstrip(")")
        spans.append(" ".join(p for p in collected if p))
        i = j + 1
    return spans


def _inner_text_is_safe(inner_text: str) -> bool:
    """True iff an argument-position substitution's inner text is definitely non-ambiguous:
    every stage it contains is either NOT git/rm-headed, or a git invocation matching the small
    read-only allowlist. Any git subcommand outside that allowlist (destructive-declared or
    simply unknown), or a bare `rm`, is NOT safe — this is what keeps `cd
    $(git rev-parse --show-toplevel)` allowed while still treating an unrecognised or
    destructive inner git/rm invocation as ambiguous."""
    chains = _chains(inner_text)
    if not chains:
        return True
    for chain in chains:
        for stage in chain:
            toks = _strip_wrappers(stage)
            if not toks:
                continue
            head = os.path.basename(toks[0])
            if head == "git":
                sub, sub_args = _split_git_global_opts(toks[1:])
                if not sub:
                    return False
                if sub in _INNER_SAFE_GIT_SUBS:
                    continue
                if sub == "config" and "--get" in sub_args:
                    continue
                if sub == "branch" and "--show-current" in sub_args:
                    continue
                if sub == "symbolic-ref":
                    if (
                        _positional_count(sub_args) < 2
                        and not _flag_present(sub_args, "-d")
                        and not _flag_present(sub_args, "--delete")
                    ):
                        continue
                return False
            if head == "rm":
                return False
    return True


# ── policy row loading + pin verification ────────────────────────────────────────────────────

def _load_policy_row():
    """(`{raw, rules, rm_targets, tier_order, tier_floor, unknown_tier, remedy}`, None) on
    success, or (None, reason) — never raises."""
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
    declared = row.get("contentHash")
    if not declared:
        return False, "row carries no contentHash pin"
    try:
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
    return os.environ.get("CLAUDE_CODE_SESSION_ID") or os.environ.get("CLAUDE_SESSION_ID") or ""


# ── classification entry point (DENY ON AMBIGUITY, scoped) ──────────────────────────────────

def _scan(command: str, row: dict, depth: int = 0) -> "str | None":
    if depth > _MAX_UNWRAP_DEPTH:
        return None
    for chain in _chains(command):
        if len(chain) == 1:
            toks0 = _strip_wrappers(chain[0])
            if toks0 and os.path.basename(toks0[0]) in _NEVER_CLASSIFY_HEADS:
                continue  # prose, single stage, no pipe/xargs downstream: exempt

        # A stage's stripped head starting with `$` or a backtick is a variable/substitution
        # USED AS THE COMMAND — unconditionally indirect, regardless of what it resolves to.
        head_is_indirect = False
        for stage in chain:
            toks = _strip_wrappers(stage)
            if toks and toks[0][:1] in ("$", "`"):
                head_is_indirect = True
                break
        if head_is_indirect:
            return INDIRECT

        # Argument-position $(...) substitutions: SCOPED — ambiguous only when the inner text
        # itself is unsafe (destructive, or git/rm-headed outside the read-only allowlist).
        for inner in _extract_paren_substitutions(chain):
            if not _inner_text_is_safe(inner):
                return INDIRECT

        for k, stage in enumerate(chain):
            toks = _strip_wrappers(stage)
            if not toks:
                continue
            head = os.path.basename(toks[0])

            if head in _NEVER_CLASSIFY_HEADS:
                continue

            if head == "find":
                for cmd in _find_exec_commands(toks[1:]):
                    result = _classify_argv(cmd, row)
                    if result:
                        return result
                continue

            if head == "xargs":
                real = _xargs_real_command(toks[1:])
                real_head = os.path.basename(real[0]) if real else ""
                if real_head in ("git", "rm"):
                    result = _classify_argv(real, row)
                    if result:
                        return result
                    continue
                if real_head in ("grep", "rg"):
                    continue
                if real_head in _SHELLS or real_head == "eval":
                    payload = _stage_payload_text(chain[k - 1]) if k > 0 else None
                    if payload is None:
                        return INDIRECT  # unverifiable upstream feeding an executor: deny
                    if _scan(payload, row, depth + 1):
                        return INDIRECT
                    continue
                continue

            if head in _SHELLS:
                idx = _find_dash_c(toks[1:])
                if idx is not None and idx + 1 < len(toks[1:]):
                    result = _scan(toks[1:][idx + 1], row, depth + 1)
                    if result:
                        return result
                    continue
                # bare shell (no -c): here-string, or a piped sink
                here_idx = None
                for hi, t in enumerate(toks[1:]):
                    if t == "<<<":
                        here_idx = hi
                        break
                if here_idx is not None:
                    payload = " ".join(toks[1:][here_idx + 1:])
                    if _scan(payload, row, depth + 1):
                        return INDIRECT
                    continue
                if k > 0:
                    payload = _stage_payload_text(chain[k - 1])
                    if payload is None:
                        return INDIRECT
                    if _scan(payload, row, depth + 1):
                        return INDIRECT
                    continue
                continue

            if head == "eval":
                rest = toks[1:]
                if rest:
                    result = _scan(" ".join(rest), row, depth + 1)
                    if result:
                        return result
                continue

            if _is_interpreter(head):
                if _interpreter_args_carry_destructive_word(toks[1:]):
                    return INDIRECT
                continue

            result = _classify_argv(toks, row)
            if result:
                return result
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


def _worth_investigating(command: str) -> bool:
    """The pre-filter, fixed round 3: runs on TOKENS (shlex-merged, so a quote-split word like
    `g""it` reads as `git`), never the raw string. Also checks the raw text for the in-token
    `${…}`/`$'…'` gluing shape, which by design does not survive as a clean token."""
    for chain in _chains(command):
        for stage in chain:
            for t in stage:
                if "git" in t or "rm" in t:
                    return True
    return _glue_affected(command)


def main():
    data = json.load(sys.stdin)
    if data.get("tool_name") != "Bash":
        return
    command = (data.get("tool_input") or {}).get("command", "")
    if not command or not _worth_investigating(command):
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

    # Round 4: in-token `${…}`/`$'…'` gluing (`git${IFS}reset${IFS}--hard`, `git reset $'--hard'`,
    # `g$'i't reset --hard`) survives no clean token for the scan above to recognise. Scan the
    # DE-GLUED form too — as an ADDITION, and through the ordinary classification path, so the
    # declared tier floor still decides. Round 3 short-circuited this class to INDIRECT, which
    # denied `git commit -m "${MSG}"` at every tier including the controller's.
    if not matched and _glue_affected(command):
        matched = _scan(_deglue(command), row, depth=0)

    # Round 5: a leading `VAR=value` assignment belongs to the same command — `MODE=--hard git
    # reset ${MODE}` runs a hard reset. Again an ADDITION, classified through the normal path.
    if not matched:
        resolved = _resolve_leading_assignments(command)
        if resolved:
            matched = _scan(resolved, row, depth=0)

    if not matched:
        return

    if matched == INDIRECT:
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

    deny(
        f"DESTRUCTIVE GIT ({POLICY_REF}): command matches `{matched}`. "
        f"Resolved tier: {tier} (floor: {floor}; tier-order: {tier_order}). {row['remedy']}"
    )


if __name__ == "__main__":
    try:
        main()
        sys.exit(0)
    except Exception as e:
        try:
            print(f"capability-tier-gate: skipped — internal error: {type(e).__name__}",
                  file=sys.stderr)
        except Exception:
            pass
        sys.exit(0)
