#!/usr/bin/env python3
"""Warn when a visible shell call has a shape that can stall or conceal its effect."""
import importlib.util
import json
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
PARSER_ERROR = None
try:
    spec = importlib.util.spec_from_file_location(
        "command_parser", ROOT / ".claude/hooks/capability-tier-gate.py"
    )
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
except Exception as error:  # Fail open; main reports the coverage gap.
    parser = None
    PARSER_ERROR = error


SHELL_TOOLS = frozenset({"Bash", "exec_command", "functions.exec_command", "shell_command"})
MUTATION_COMMANDS = {
    "git": frozenset({
        ("add",), ("commit",), ("push",), ("merge",), ("rebase",), ("reset",),
        ("checkout",), ("switch",), ("worktree", "add"), ("worktree", "remove"),
    }),
    "brit": frozenset({
        ("add",), ("commit",), ("push",), ("merge",), ("rebase",), ("reset",),
        ("checkout",), ("switch",), ("worktree", "add"), ("worktree", "remove"),
    }),
    "rm": frozenset({()}),
    "mv": frozenset({()}),
    "cp": frozenset({()}),
    "mkdir": frozenset({()}),
    "ln": frozenset({()}),
    "berth": frozenset({("claim",), ("release",), ("say",), ("moor",)}),
    "epr": frozenset({("flow", "note"), ("flow", "intend"), ("covenant", "record"),
                      ("session", "open")}),
}
INTERPRETERS = frozenset({"python", "python3", "node", "bash", "sh", "zsh", "ruby",
                          "perl", "deno"})
CODE_SUFFIXES = (".py", ".js", ".mjs", ".ts", ".sh", ".rs", ".rb", ".pl")
DIRECTORY_COMMANDS = frozenset({"cd", "pushd", "popd"})
HEREDOC = re.compile(r"(?<!<)<<-?\s*(['\"]?)([A-Za-z_][A-Za-z0-9_]*)\1(?!<)")
WRITE_REDIRECT = re.compile(r"^(?:\d?(?:>>|>)|&>>?)$")
COUNTER_BOUND = re.compile(
    r"(?:\(\([^)]*(?:<=|>=|<|>)\s*\d+[^)]*\)\)|(?:-lt|-le|-gt|-ge)\s+\d+)", re.S
)
COMPARISON = r"(?:-lt|-le|-gt|-ge|<=|>=|<|>)"
SECONDS_BOUND = re.compile(
    rf"(?:\$(?:\{{)?SECONDS(?:\}})?[^;\n]{{0,80}}{COMPARISON}|"
    rf"{COMPARISON}[^;\n]{{0,80}}\$(?:\{{)?SECONDS(?:\}})?)"
)
DATE_BOUND = re.compile(rf"date \+%s[^;\n]{{0,120}}{COMPARISON}")
BRIT_FALLBACK = "# Brit fallback:"
FULL_COMMAND_MATCH = re.compile(r"\bp(?:grep|kill)\b(?:\s+-[A-Za-z]+)*?\s+-[A-Za-z]*f[A-Za-z]*\b")


def _command_words(argv):
    """Return a normalized command path used by the single mutation table above."""
    if not argv:
        return "", []
    head = Path(argv[0]).name
    rest = argv[1:]
    if head == "git":
        subcommand, args = parser._split_git_global_opts(rest)
        return head, ([subcommand, *args] if subcommand else [])
    if head == "brit":
        index = 0
        while index < len(rest):
            token = rest[index]
            if token == "-C":
                index += 2
            elif token.startswith("-"):
                index += 1
            else:
                break
        return head, rest[index:]
    return head, rest


def _stage_mutates(stage):
    argv = parser._strip_wrappers(stage)
    head, words = _command_words(argv)
    forms = MUTATION_COMMANDS.get(head, ())
    return any(not form or tuple(words[:len(form)]) == form for form in forms)


def _write_redirect_chains(command):
    """Indexes of parser chains carrying a file write redirect, using its tokenizer."""
    tokens = parser._tokenize(command)
    flags = []
    current = False
    has_tokens = False
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token in parser._TOP_OPERATORS:
            if has_tokens:
                flags.append(current)
            current = False
            has_tokens = False
            index += 1
            continue
        has_tokens = True
        if WRITE_REDIRECT.match(token):
            target = tokens[index + 1] if index + 1 < len(tokens) else ""
            if target != "/dev/null" and not target.isdigit():
                current = True
            index += 2
            continue
        index += 1
    if has_tokens:
        flags.append(current)
    return flags


def _heredoc_headers(command):
    return [(line, match) for line in command.splitlines() if (match := HEREDOC.search(line))]


def _without_heredoc_bodies(command):
    output = []
    delimiter = None
    strip_tabs = False
    for line in command.splitlines():
        if delimiter is not None:
            candidate = line.lstrip("\t") if strip_tabs else line
            if candidate.strip() == delimiter:
                delimiter = None
            continue
        output.append(line)
        match = HEREDOC.search(line)
        if match:
            delimiter = match.group(2)
            strip_tabs = line[match.start():].startswith("<<-")
    return "\n".join(output)


def _executable_text(command):
    return "\n".join(
        line for line in _without_heredoc_bodies(command).splitlines()
        if not line.lstrip().startswith("#")
    )


def _one_operation(command):
    executable = _executable_text(command)
    chains = parser._chains(executable)
    if len(chains) < 2:
        return None
    redirect_flags = _write_redirect_chains(executable)
    kinds = []
    for index, chain in enumerate(chains):
        mutation = any(_stage_mutates(stage) for stage in chain)
        mutation = mutation or (index < len(redirect_flags) and redirect_flags[index])
        kinds.append("mutation" if mutation else "read")
    if "mutation" in kinds and "read" in kinds:
        return "command-shape: one-operation — read and mutation segments are chained → split them into separate tool calls"
    return None


def _named_replacement(command):
    for chain in parser._chains(_executable_text(command)):
        for stage in chain:
            argv = parser._strip_wrappers(stage)
            head, words = _command_words(argv)
            if head != "git" or not words:
                continue
            if words[0] == "cherry-pick":
                return "genesis/agentic/bin/landing-pick"
            if words[0] == "rebase" and any(
                word == "--exec" or word.startswith("--exec=") for word in words[1:]
            ):
                return "genesis/agentic/bin/landing-retrailer"
            if words[0] == "commit-tree":
                return "genesis/agentic/bin/landing-tagged-tip"
    return None


def _leading_comment(command):
    nonblank = [line.strip() for line in command.splitlines() if line.strip()]
    if not nonblank or not nonblank[0].startswith("#"):
        return None
    first = nonblank[0]
    if first.startswith(BRIT_FALLBACK) and first[len(BRIT_FALLBACK):].strip():
        replacement = _named_replacement(command)
        if replacement:
            return ("command-shape: leading-comment — Brit fallback precedes a Git mutation with "
                    f"a named replacement → run {replacement} -C <worktree> … without a comment prefix")
        if len(nonblank) == 1 or not nonblank[1].startswith("#"):
            return None
    return ("command-shape: leading-comment — the command starts with a comment line → put the "
            "explanation in commentary; only one # Brit fallback: <reason> line is exempt")


def _directory_change(command):
    for chain in parser._chains(_executable_text(command)):
        for stage in chain:
            argv = parser._strip_wrappers(stage)
            if argv and Path(argv[0]).name in DIRECTORY_COMMANDS:
                return ("command-shape: no-cd — cd/pushd/popd changes shell-local location → use an "
                        "absolute path or the tool's working-directory option (git -C, --manifest-path, -C <dir>)")
    return None


def _code_heredoc(command):
    for header, match in _heredoc_headers(command):
        prefix = header[:match.start()]
        chains = parser._chains(prefix)
        if not chains:
            continue
        argv = parser._strip_wrappers(chains[-1][-1])
        head = Path(argv[0]).name if argv else ""
        if head in INTERPRETERS:
            return ("command-shape: no-code-heredoc — a heredoc feeds an interpreter → write the "
                    "script with the editor tool, then run the file")
        if head == "cat":
            tokens = parser._tokenize(header)
            for index, token in enumerate(tokens[:-1]):
                if WRITE_REDIRECT.match(token):
                    target = tokens[index + 1].rstrip(";,)")
                    if target.endswith(CODE_SUFFIXES):
                        return ("command-shape: no-code-heredoc — cat writes heredoc code to a file → "
                                "write the script with the editor tool, then run the file")
    return None


def _unbounded_wait(command):
    executable = _executable_text(command)
    # The shell that runs an agent's command carries the whole command text in its own command
    # line, so `pgrep -f <pattern>` (or pkill) matches that shell whatever the pattern is, also
    # inside `$(…)`. Matched as text, because the parser does not descend into substitutions.
    if FULL_COMMAND_MATCH.search(executable):
        return ("command-shape: bounded-wait — pgrep/pkill -f matches the shell running this command → "
                "record a PID or done-file and use timeout <s> bash -c 'while kill -0 <pid>; do sleep 1; done'")
    loop = re.search(r"\b(?:while|until)\b", executable)
    if loop and re.search(r"\bsleep\b", executable):
        bounded = (
            re.search(r"\btimeout\b", executable)
            or SECONDS_BOUND.search(executable)
            or DATE_BOUND.search(executable)
            or COUNTER_BOUND.search(executable)
        )
        if not bounded:
            return ("command-shape: bounded-wait — a sleep loop has no deadline or counter bound → "
                    "name a PID or done-file and wrap the check in timeout <s>")
    return None


def _env_unset(command):
    for chain in parser._chains(_executable_text(command)):
        for stage in chain:
            if len(stage) > 1 and Path(stage[0]).name == "env" and stage[1] == "-u":
                return ("command-shape: no-env-u — a segment starts with env -u → unset it in the "
                        "session or pass the variable explicitly")
    return None


def findings(command):
    checks = (_one_operation, _leading_comment, _directory_change, _code_heredoc,
              _unbounded_wait, _env_unset)
    return [finding for check in checks if (finding := check(command))][:3]


def evaluate(event):
    if PARSER_ERROR is not None:
        raise RuntimeError("shell parser unavailable") from PARSER_ERROR
    if not isinstance(event, dict) or event.get("tool_name", "") not in SHELL_TOOLS:
        return None
    tool_input = event.get("tool_input", {})
    if isinstance(tool_input, str):
        command = tool_input
    elif isinstance(tool_input, dict):
        command = tool_input.get("command", tool_input.get("cmd", ""))
    else:
        command = ""
    if not isinstance(command, str):
        return None
    messages = findings(command)
    return {"additionalContext": "\n".join(messages)} if messages else None


def main():
    try:
        raw = sys.stdin.buffer.read(1572865)
        if len(raw) > 1572864:
            raise ValueError("event too large")
        result = evaluate(json.loads(raw))
    except Exception:
        result = {"additionalContext":
                  "command-shape: coverage-gap — shell event could not be decoded or checked"}
    if result:
        try:
            print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", **result}}))
        except OSError:
            pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
