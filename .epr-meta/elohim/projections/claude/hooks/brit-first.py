#!/usr/bin/env python3
"""Brit habit friction for visible shell commands; not a security boundary."""
import importlib.util
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("command_parser", ROOT / ".claude/hooks/capability-tier-gate.py")
parser = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parser)


# One table, read from the installed brit's own `--help` (not from memory). When the
# installed brit differs from BRIT_TABLE_VERSION the table is stale: re-read the help.
# A session keeps the brit it started with, so the brit on PATH is asked for its version
# on a refusal (never on a pass) and BRIT_TABLE_PRIOR supplies the rows that differ there.
# kind HAS: (form,)   PARTIAL: (what exists, the absent form)   ABSENT: ()
BRIT_TABLE_VERSION = "0.3.5"
BRIT_TABLE = {
    "status": ("HAS", "brit status --no-write (-f porcelain-v2, -u; another checkout: brit -C <dir> status)"),
    "log": ("HAS", "brit log [-n N] [--oneline] [--first-parent] [-S <string>] [A..B] [-- paths]"),
    "show": ("HAS", "brit show [--stat|--name-only|-s] <rev>[:path] (a file at a ref: brit show <rev>:<path>)"),
    "diff": ("HAS", "brit diff [--cached] [--stat|--name-only] [A..B] [-- paths]; brit diff tree A B"),
    "add": ("HAS", "brit add <literal paths> | -A | -u | --gitlink <full commit> <submodule path> (no pathspec patterns)"),
    "commit": ("HAS", "brit commit -m <msg> | -F <file> [--amend --no-edit] [--allow-empty] [-n]; no -a, use brit add -u"),
    "branch": ("HAS", "brit branch list | create <name> [start] | delete <name> | move <old> <new>"),
    "fetch": ("HAS", "brit fetch [<remote> [<refspec>]]"),
    "merge-base": ("HAS", "brit merge-base [--is-ancestor] <A> <B>"),
    "rebase": ("HAS", "brit rebase <upstream> | -i | --continue | --abort | --status"),
    "rev-parse": ("HAS", "brit rev-parse [--show-toplevel|--git-dir|--abbrev-ref|--verify|--short] <rev>"),
    "ls-remote": ("HAS", "brit ls-remote [-h|-t|--refs] <remote> [pattern]"),
    "tag": ("HAS", "brit tag [-a -m <msg>] [-f] <name> [commit] | list | delete <name>"),
    "blame": ("HAS", "brit blame [-L a,b] <file>"),
    "clean": ("HAS", "brit clean [-d] [-x] [-e] (dry run until -e)"),
    "check-ignore": ("HAS", "brit exclude query <paths>"),
    "describe": ("HAS", "brit commit describe [rev]"),
    "for-each-ref": ("HAS", "brit for-each-ref"),
    "archive": ("HAS", "brit archive <out-file> [treeish]"),
    "switch": ("PARTIAL", "brit switch <existing branch>", "switch -c (use brit branch create <n> [start], then brit switch)"),
    "checkout": ("PARTIAL", "brit switch <branch>; brit restore -- <paths>", "checkout -b, <ref> -- <path>, detaching (unverified: read blobs with brit show <ref>:<path>)"),
    "pull": ("PARTIAL", "brit pull --ff-only <remote> <branch> | --status | --continue | --abort", "a merging or rebasing pull"),
    "push": ("HAS", "brit push [<remote> <refspec>] [-f|--force-with-lease|-u|-d|-n]; dev landing: GATE_EVIDENCE=reuse brit -c core.hooksPath=<worktree>/.husky/_ push origin HEAD:dev"),
    "merge": ("PARTIAL", "brit merge --ff-only <ref> (a local commit; the ref is required); object plumbing: brit merge file|tree|commit", "a merge that is not a fast-forward"),
    "rev-list": ("PARTIAL", "brit rev-list --count [--left-right] A...B; listing is brit log --oneline", "other rev-list output"),
    "ls-files": ("PARTIAL", "brit index entries [pathspec]", "git's exact output format (unverified)"),
    "ls-tree": ("PARTIAL", "brit tree entries [-r] <treeish>", "git's exact output format (unverified)"),
    "cat-file": ("PARTIAL", "brit cat <rev>[:path]", "-t, -s, --batch (unverified)"),
    "worktree": ("PARTIAL", "brit worktree list | add <path> [-b new] [commit] | remove <path>", "prune, move, lock"),
    "submodule": ("PARTIAL", "brit submodule list | update [--init] [path]", "add, status, sync, foreach"),
    "restore": ("PARTIAL", "brit restore [--staged] [--worktree] [--source HEAD] -- <literal files>", "--source other than HEAD, directories, patterns"),
    "remote": ("PARTIAL", "brit remote -n <name> url | refs", "add, remove, rename, set-url"),
    "config": ("PARTIAL", "brit config show [filter] | set <k> <v> | unset <k>", "--global, --add, --get-all (unverified)"),
    "cherry-pick": ("PARTIAL", "brit cherry-pick <commit> (one commit per call)", "a range, -n, -x, --continue, --abort"),
    "notes": ("PARTIAL", "brit notes [--ref <notes ref>] show [object]; refs/notes/* move by refspec: brit fetch|push <remote> refs/notes/<x>:refs/notes/<x>", "add, append, remove, copy, list"),
    "grep": ("PARTIAL", "brit grep [-n] [-i] [-l] [-F|-E] <rev> -- <pattern> (or <pattern> <rev> [-- pathspecs]; -e <pattern> repeats)", "searching the working tree or index without a revision"),
    "update-index": ("PARTIAL", "brit add --gitlink <full commit> <submodule path> (for --cacheinfo 160000,<commit>,<path>; no checked-out submodule needed)", "every other update-index form"),
    "stash": ("ABSENT",),
    "reset": ("ABSENT",),
    "apply": ("ABSENT",),
    "format-patch": ("ABSENT",),
    "am": ("ABSENT",),
    "bisect": ("ABSENT",),
    "reflog": ("ABSENT",),
    "gc": ("ABSENT",),
    "repack": ("ABSENT",),
}
# The rows a brit one release back answers differently; every other row is shared.
BRIT_TABLE_PRIOR_VERSION = "0.3.4"
BRIT_TABLE_PRIOR = {
    "log": ("HAS", "brit log [-n N] [--oneline] [--first-parent] [A..B] [-- paths]"),
    "add": ("HAS", "brit add <literal paths> | -A | -u (no pathspec patterns)"),
    "merge": ("PARTIAL", "brit merge file|tree|commit (object-database plumbing only; fast-forward: brit pull --ff-only)", "merging into the worktree (unverified)"),
    "cherry-pick": ("ABSENT",),
    "notes": ("ABSENT",),
    "grep": ("ABSENT",),
    "update-index": ("ABSENT",),
}
VERSION_PROBE_SECONDS = 0.5
# git global options that consume the next token.
GIT_GLOBAL_WITH_VALUE = {"-C", "-c", "--git-dir", "--work-tree", "--namespace", "--super-prefix", "--config-env"}
MAX_REASON = 600
GENERIC = ("Use Brit for this repository operation. Check brit <command> --help first. "
    "If a specific capability is missing or failed, explain it in commentary and retry "
    "with a leading '# Brit fallback: <specific reason>' comment on this command. "
    "Inspect state before retrying a failed mutation. No human approval is needed.")


def git_stages(command, depth=0):
    """argv of every stage whose program is git, including inside nested bash -c."""
    found = []
    if depth > 4:
        return found
    for chain in parser._chains(command):
        for stage in chain:
            argv = parser._strip_wrappers(stage)
            if not argv:
                continue
            head = Path(argv[0]).name
            if head == "git":
                found.append(argv)
            elif head in ("bash", "sh", "zsh", "dash"):
                index = parser._find_dash_c(argv[1:])
                if index is not None and index + 2 < len(argv):
                    found.extend(git_stages(argv[index + 2], depth + 1))
    return found


def uses_git(command, depth=0):
    return bool(git_stages(command, depth))


def git_subcommand(argv):
    i = 1
    while i < len(argv):
        token = argv[i]
        if token in GIT_GLOBAL_WITH_VALUE:
            i += 2
        elif token.startswith("-"):
            i += 1
        else:
            return token
    return None


def installed_brit_version():
    """Version of the brit on PATH, or None when it cannot be read within the budget."""
    try:
        import os
        import re
        import subprocess
        out = subprocess.run(["brit", "--version"], capture_output=True, text=True, stdin=subprocess.DEVNULL,
            timeout=VERSION_PROBE_SECONDS, env={**os.environ, "BRIT_QUIET": "1"}).stdout
        found = re.match(r"brit (\d+\.\d+\.\d+)", out)
        return found.group(1) if found else None
    except Exception:
        return None


def table_for(installed):
    """(version label, row lookup, note) for the brit this session runs."""
    if installed == BRIT_TABLE_PRIOR_VERSION:
        return installed, lambda sub: BRIT_TABLE_PRIOR.get(sub) or BRIT_TABLE.get(sub), (
            " (this session's brit; %s is landed, restart to get its forms)" % BRIT_TABLE_VERSION)
    if installed == BRIT_TABLE_VERSION:
        return installed, BRIT_TABLE.get, ""
    if installed is None:
        return BRIT_TABLE_VERSION, BRIT_TABLE.get, (
            " (brit %s form; a session still on %s falls back with a stated reason)"
            % (BRIT_TABLE_VERSION, BRIT_TABLE_PRIOR_VERSION))
    return BRIT_TABLE_VERSION, BRIT_TABLE.get, " (installed brit is %s: the table is stale, read its help)" % installed


def steer(command):
    """Message naming the brit form per git subcommand, or None when unparseable."""
    subs = []
    for argv in git_stages(command):
        sub = git_subcommand(argv)
        if sub is None:
            return None
        if sub not in subs:
            subs.append(sub)
    if not subs:
        return None
    tail = " Else retry with a first-line '# Brit fallback: <reason>' comment."
    version, lookup, note = table_for(installed_brit_version())
    head = "Brit %s table%s. " % (version, note)
    parts = []
    for sub in subs:
        row = lookup(sub)
        if row is None:
            text = "git %s: not in the %s table; run brit %s --help." % (sub, version, sub)
        elif row[0] == "HAS":
            text = "git %s: brit has this: %s." % (sub, row[1])
        elif row[0] == "PARTIAL":
            text = "git %s: %s; fallback only for %s." % (sub, row[1], row[2])
        else:
            text = "git %s: no brit form in %s; a fallback is expected, state it." % (sub, version)
        parts.append(text)
    message = head + " ".join(parts) + tail
    while len(message) > MAX_REASON and len(parts) > 1:
        parts.pop()
        message = head + " ".join(parts) + " (+more git verbs)." + tail
    if len(message) > MAX_REASON:
        return None
    return message


def deny_reason(command):
    try:
        return steer(command) or GENERIC
    except Exception:
        return GENERIC


def evaluate(event):
    config = json.loads((ROOT / ".epr-meta/brit-first.json").read_text())
    if config.get("enabled") is not True:
        return None
    name = event.get("tool_name", "")
    if name not in ("Bash", "exec_command", "functions.exec_command", "shell_command"):
        return None
    args = event.get("tool_input", {})
    command = args.get("command", args.get("cmd", ""))
    if not isinstance(command, str) or not uses_git(command):
        return None
    # A leading comment belongs only to this invocation, never ambient environment.
    first = command.splitlines()[0].strip()
    prefix = "# Brit fallback: "
    if first.startswith(prefix) and len(first[len(prefix):].strip()) >= 12:
        return {"additionalContext": "Brit fallback recorded in this tool call: " + first[len(prefix):]}
    return {"permissionDecision": "deny", "permissionDecisionReason": deny_reason(command)}


def main():
    try:
        raw = sys.stdin.buffer.read(1572865)
        if len(raw) > 1572864:
            raise ValueError("event too large")
        result = evaluate(json.loads(raw))
    except Exception:
        result = {"additionalContext": "Brit reminder coverage gap: shell event could not be decoded."}
    if result:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", **result}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
