#!/usr/bin/env python3
"""Expose native Rust style findings before edits; reach gates cover every writer."""
import json
from pathlib import Path
import re
import subprocess
import sys

RETIRE_WHEN = "all supported runtimes expose the native style gate as a trusted built-in edit check"


SHELL_TOOLS = {"Bash", "shell", "exec_command", "local_shell", "functions.shell", "functions.exec_command"}
# Default-emit rule: listing the ways a command can write a .rs file is unbounded, so a shell command
# that names cargo or a .rs token speaks unless it is provably read-only. Silence is the exception,
# and the allowlist holds only commands that cannot write a file or run a program the caller names.
# This is an advisory notice for a cooperating agent, not a security boundary: the reach gates
# cover every writer. So the proof is deliberately crude and refuses anything it cannot read
# plainly (any expansion, escape, grouping, or quoting combined with a separator).
# The default-value class excludes "$" and "{" so a run of "${CARGO:-" cannot backtrack quadratically.
CARGO_WORD = re.compile(r"(?<![\w.-])cargo(?=[\"']?\s)|\$\{?CARGO(?::-[^}\s${]*)?\}?[\"']?\s")
RS_TOKEN = re.compile(r"\.rs(?!\w)")
# Whole-word fd plumbing that holds a ">" without writing a file; any other ">" disqualifies silence.
HARMLESS = re.compile(r"(?<!\S)(?:2>&1|1>&2|>&2|2>/dev/null)(?=$|[\s;&|])")
SINGLE_QUOTED = re.compile(r"'[^']*'")
# Expansions, escapes and command substitution: the words the shell runs are not the words written.
OPAQUE = ("\\", "$", "`")
# After quoted text is blanked: redirect, heredoc, grouping, a function body or brace expansion.
DISQUALIFYING = (">", "<<", "(", "{", "}")
SEPARATOR = re.compile(r"[;&|\n]")
SEGMENT = re.compile(r"\n|;|&&|\|\||\||&")
ASSIGNMENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*=")
READ_ONLY = frozenset({
    "grep", "egrep", "fgrep", "rg", "ls", "cat", "head", "tail", "wc", "find", "stat", "diff",
    "cmp", "cut", "nl", "tac", "basename", "dirname", "realpath", "readlink", "echo", "true",
    "test", "[", "cd", "pwd", "du", "sha256sum", "md5sum", "strings", "brit", "git", "sed",
})
# Word prefixes that make an allowlisted reader write a file or run a program.
FIND_UNSAFE = ("-exec", "--exec", "-ok", "-fprint", "-fls", "-delete", "-rm")
RG_UNSAFE = ("--pre", "--hostname-bin")
# git accepts any unambiguous long-option prefix and bundled short flags, hence "--op" and GIT_BUNDLED_O.
GIT_UNSAFE = ("--output", "--op", "--ext-diff", "--textconv", "--config-env", "-c")
GIT_BUNDLED_O = re.compile(r"-[^-]*O")
SED_PRINT = re.compile(r"'?[0-9,$]+p'?")  # the one sed script known to be read-only: address + print
GIT_READS = frozenset({
    "diff", "log", "show", "status", "grep", "blame", "ls-files", "ls-tree", "cat-file", "rev-parse",
    "shortlog", "describe", "branch",
})
BRIT_READS = frozenset({"status", "log", "diff", "show", "remote", "exclude", "blame"})


def command_word(words):
    """Index and name of a segment's command word, past one leading ( { or !.

    A leading NAME=value is not skipped: an environment variable can choose the program a reader
    runs (GIT_EXTERNAL_DIFF, RIPGREP_CONFIG_PATH), so such a segment is not provably read-only."""
    for index, word in enumerate(words):
        if word[:1] in ("(", "{", "!"):
            word = word[1:]
        if not word:
            continue
        if ASSIGNMENT.match(word):
            return index, None
        return index, word
    return None, ""


def git_verb(rest):
    """Position and name of git's subcommand, past options and a -C <dir> pair."""
    skip = False
    for position, word in enumerate(rest):
        if skip:
            skip = False
        elif word == "-C":
            skip = True
        elif not word.startswith("-"):
            return position, word
    return None, None


def segment_reads_only(segment):
    words = [word.replace("'", "").replace('"', "") for word in segment.split()]
    index, name = command_word(words)
    if name == "":
        return True  # an empty segment is ignored
    if name not in READ_ONLY:
        return False
    rest = words[index + 1:]
    if name == "find":
        return not any(word.startswith(FIND_UNSAFE) for word in rest)
    if name == "rg":
        return not any(word.startswith(RG_UNSAFE) for word in rest)
    if name == "sed":
        # Only `sed -n <address>p <files>`: no -f, no in-place, no w/e commands can hide in it.
        return len(rest) >= 2 and rest[0] == "-n" and bool(SED_PRINT.fullmatch(rest[1])) and not any(
            word.startswith("-") for word in rest[2:])
    if name == "git":
        if any(word.startswith(GIT_UNSAFE) or GIT_BUNDLED_O.match(word) for word in rest):
            return False
        position, verb = git_verb(rest)
        if verb not in GIT_READS:
            return False
        return verb != "branch" or all(word.startswith("-") for word in rest[position + 1:])
    if name == "brit":
        return bool(rest) and rest[0] in BRIT_READS
    return True


def provably_read_only(command):
    if any(mark in command for mark in OPAQUE):
        return False
    quoted = "'" in command or '"' in command
    if quoted and SEPARATOR.search(command):
        return False  # a separator may sit inside the quotes; splitting on it would mislead
    # A ">" inside single quotes is text (grep -c 'a -> b'); blank those spans when nothing else quotes.
    scan = command if '"' in command else SINGLE_QUOTED.sub("''", command)
    scan = HARMLESS.sub(" ", scan)
    if any(mark in scan for mark in DISQUALIFYING):
        return False
    return all(segment_reads_only(segment) for segment in SEGMENT.split(HARMLESS.sub(" ", command)))


def shell_runs_rust(command):
    """True when a shell command names cargo, or names a .rs token and is not provably read-only.

    Linear: plain replacements, one split, set membership and simple regexes. It defaults to
    emitting; only a command whose every segment is an allowlisted reader stays silent."""
    if "*** Begin Patch" in command:
        return "cargo " in command or ".rs" in command
    if CARGO_WORD.search(command):
        return True
    if not RS_TOKEN.search(command):
        return False
    return not provably_read_only(command)


def inspect(root, event):
    tool = event.get("tool_name", "")
    data = event.get("tool_input", {})
    narrowed = tool in SHELL_TOOLS and isinstance(data, dict) and "code" not in data
    if isinstance(data, str):
        # Codex freeform apply_patch and exec events carry a string, not a dict.
        data = {"command": data}
    if not isinstance(data, dict):
        return None
    path = data.get("file_path") or data.get("path")
    if not isinstance(path, str) or not path.endswith(".rs"):
        # Shell and opaque orchestration calls cannot be reconstructed safely.
        # Their committed output is checked independently by pre-push and CI.
        command = data.get("command", data.get("cmd", data.get("code", "")))
        if isinstance(command, str):
            if narrowed:
                hit = shell_runs_rust(command)
            else:
                # Not a real shell tool call (apply_patch body, opaque exec string): the original broad match.
                hit = "cargo " in command or ".rs" in command
            if hit:
                return {"context": "Rust style RS-GOV-04 is active: no new unexplained suppressions; touched crates with debt must reduce it. Run just gate rust-style before reach. This tool's prospective source is unavailable; it has not been certified clean."}
        return None
    target = Path(path)
    if not target.is_absolute():
        target = root / target
    try:
        relative = target.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return None
    if tool in ("Write", "write_file"):
        content = data.get("content")
    elif tool in ("Edit", "edit_file"):
        before = target.read_text()
        old, new = data.get("old_string"), data.get("new_string")
        if not isinstance(old, str) or not old or not isinstance(new, str):
            return {"context": "Rust style: prospective edit is unmeasured; run just gate rust-style."}
        count = before.count(old)
        if count == 0 or (count > 1 and not data.get("replace_all")):
            return {"context": "Rust style: edit context is ambiguous; native tool and reach gate will validate it."}
        content = before.replace(old, new, -1 if data.get("replace_all") else 1)
    else:
        return {"context": "Rust style RS-GOV-04 applies to this crate. Run just gate rust-style; this tool's prospective source has not been measured."}
    if not isinstance(content, str):
        return {"context": "Rust style: prospective content is unmeasured."}
    result = subprocess.run(
        ["bash", str(root / "genesis/orchestrator/rust-style.sh"), "--hook", "--path", relative, "--stdin"],
        input=content, text=True, capture_output=True, timeout=4,
    )
    if result.returncode not in (0, 1):
        return {"context": "Rust style UNMEASURED: " + result.stderr.strip()[:700]}
    payload = json.loads(result.stdout)
    failures = payload.get("failures")
    if not isinstance(failures, list):
        return {"context": "Rust style UNMEASURED: checker returned no verdict."}
    if failures:
        return {"deny": "\n".join(failures[:8]) + "\nSee genesis/docs/rust-style-guide.md#rs-gov-04."}
    return {"context": "Rust style: proposed attributes pass RS-GOV-04 (all cfg branches; cached checker binary passed freshness checks). Crate debt/paydown and compiler checks remain required at reach; just gate rust-style."}


def main():
    root = Path(__file__).resolve().parents[2]
    try:
        event = json.loads(sys.stdin.buffer.read(1572864))
        result = inspect(root, event)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        result = {"context": f"Rust style UNMEASURED ({type(error).__name__}); run just gate rust-style before reach."}
    if result:
        output = {"hookEventName": "PreToolUse"}
        if "deny" in result:
            output.update(permissionDecision="deny", permissionDecisionReason=result["deny"])
        else:
            output["additionalContext"] = result["context"]
        print(json.dumps({"hookSpecificOutput": output}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
