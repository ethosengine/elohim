#!/usr/bin/env python3
"""Brit habit friction for visible shell commands; not a security boundary.

It still fails closed: a command it cannot read with certainty is refused when a git token is in it,
and the one exemption (a call into a scratch repository) is a narrow allow-list.
"""
import importlib.util
import json
import os
import re
import shlex
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


class Unparseable(Exception):
    """The command could not be read with certainty; a git token in it is then refused."""


# A git token anywhere in the raw text: what fails closed when the command cannot be read.
GIT_TOKEN = re.compile(r"(?<![A-Za-z0-9_.-])git(?![A-Za-z0-9_-])")
SHELLS = ("bash", "sh", "zsh", "dash")
# Characters that end a simple command outside quotes, for deciding which command owns a heredoc.
SEGMENT_END = ";&|\n()`"
# The operators a qualifying heredoc's command may sit between: a list, never a pipe or a subshell.
LIST_OPERATORS = (None, ";", "&&", "||", "&", "\n")

# The outside-repository exemption is a narrow allow-list (ruling 2026-11-27): a git call is let
# through only when it names a scratch repository by one literal absolute -C and runs a verb that
# stays inside that repository. Verbs that reach another repository (push, fetch, pull, clone,
# remote, submodule, ls-remote, config, worktree, archive, bundle, the pack and transport verbs)
# are absent from it, so they are always policed.
LOCAL_VERBS = {
    "init", "status", "add", "rm", "mv", "commit", "log", "show", "diff", "rev-parse", "rev-list",
    "branch", "tag", "checkout", "switch", "restore", "reset", "merge", "rebase", "cherry-pick",
    "revert", "stash", "apply", "am", "format-patch", "ls-files", "ls-tree", "cat-file",
    "hash-object", "update-index", "write-tree", "read-tree", "commit-tree", "update-ref",
    "symbolic-ref", "for-each-ref", "show-ref", "merge-base", "describe", "blame", "grep", "notes",
    "reflog", "fsck", "gc", "prune", "repack", "pack-refs", "clean", "shortlog", "name-rev",
    "count-objects", "check-ignore", "check-attr", "diff-tree", "diff-index", "diff-files",
    "range-diff", "mktree", "mktag", "verify-commit", "verify-tag", "var",
}
# The most entries a scratch repository's git directory may hold for the symlink walk to vouch for it.
SCRATCH_GIT_ENTRIES = 20000
REPOSITORY_ENV = re.compile(r"(?<![A-Za-z0-9_])GIT_[A-Z_]+")
URL_LIKE = re.compile(r"://|^[^/\s:@]+@[^/\s:]+:")
PARENT_SEGMENT = re.compile(r"(^|[/=])\.\.(/|$)")
# What the shell would expand in a word: a parameter or command, a glob, a leading tilde, a brace list.
EXPANDS = re.compile(r"[$`*?\[]|(^|[=:])~|\{[^}]*,[^}]*\}")
# The variables that move where git finds a repository, its config or its objects.
LOCATING_ENV = ("GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_INDEX_FILE", "GIT_OBJECT_DIRECTORY",
                "GIT_ALTERNATE_OBJECT_DIRECTORIES", "GIT_NAMESPACE", "GIT_CONFIG", "GIT_CONFIG_GLOBAL",
                "GIT_CONFIG_SYSTEM", "GIT_CONFIG_COUNT", "GIT_CONFIG_PARAMETERS", "GIT_EXEC_PATH",
                "GIT_TEMPLATE_DIR")


def _segment_bounds(text, start, end):
    """(operator before, operator after) a simple command spanning text[start:end]."""
    before = None
    j = start - 1
    while j >= 0 and text[j] in " \t":
        j -= 1
    if j >= 0:
        before = text[j]
        if before in "&|" and j > 0 and text[j - 1] == before:
            before = before * 2
    after = None
    if end < len(text):
        after = text[end]
        if after in "&|" and end + 1 < len(text) and text[end + 1] in (after, "&"):
            after = text[end:end + 2]
    return before, after


def _writes_only_to_file(segment, before, after):
    """Names of the files a `cat > file <<'X'` or `tee file <<'X'` writes, or None when it does not qualify.

    The command must be cat or tee and nothing else: no environment, no wrapper, no pipe on either
    side, no process substitution, no other redirection, exactly one heredoc."""
    if before not in LIST_OPERATORS or after not in LIST_OPERATORS:
        return None
    try:
        tokens = shlex.split(segment)
    except ValueError:
        return None
    if not tokens or tokens[0] not in ("cat", "tee"):
        return None
    heredocs = 0
    targets = []
    args = []
    i = 1
    while i < len(tokens):
        token = tokens[i]
        if token.startswith("<<"):
            heredocs += 1
            i += 2 if token in ("<<", "<<-") else 1
            continue
        if token in (">", ">>"):
            if i + 1 >= len(tokens):
                return None
            targets.append(tokens[i + 1])
            i += 2
            continue
        if token.startswith(">"):
            targets.append(token.lstrip(">"))
            i += 1
            continue
        if re.match(r"^\d*[<>]", token) or "<" in token or "&" in token or "(" in token or ")" in token:
            return None
        args.append(token)
        i += 1
    if heredocs != 1:
        return None
    if tokens[0] == "cat":
        if args or len(targets) != 1:
            return None
        files = targets
    else:
        if targets:
            return None
        files = [a for a in args if a not in ("-a", "--append")]
        if not files or any(a.startswith("-") for a in files):
            return None
    for name in files:
        if not name or EXPANDS.search(name) or name.startswith(("&", "|")) or "(" in name:
            return None
    return files


def _heredoc_word(text, j):
    """(delimiter, quoted, index after the word) of a heredoc word starting at text[j]."""
    k = j
    parts = []
    quoted = False
    while k < len(text) and text[k] not in " \t\n;&|()<>":
        ch = text[k]
        if ch in ("'", '"'):
            end = text.find(ch, k + 1)
            if end < 0 or "\n" in text[k + 1:end]:
                raise Unparseable("heredoc delimiter")
            parts.append(text[k + 1:end])
            quoted = True
            k = end + 1
        elif ch == "\\":
            if k + 1 >= len(text):
                raise Unparseable("heredoc delimiter")
            parts.append(text[k + 1])
            quoted = True
            k += 2
        else:
            parts.append(ch)
            k += 1
    delimiter = "".join(parts)
    if not delimiter:
        raise Unparseable("empty heredoc delimiter")
    return delimiter, quoted, k


def _scan(command):
    """The command with each heredoc body that is only data blanked; raises Unparseable.

    A body is data only when its delimiter is quoted and its command is cat or tee writing to a
    file with nothing piped onward, and that file is not named again anywhere else in the command
    (so a script written and then run stays scanned). Every other body is scanned as on dev.
    Quotes are tracked across lines; a comment ends the line before any << in it is read."""
    text = command
    n = len(text)
    heredocs = []
    pending = []
    quote = None
    segment_start = 0
    segment_heredocs = []
    i = 0

    def close_segment(end):
        nonlocal segment_heredocs
        if segment_heredocs:
            before, after = _segment_bounds(text, segment_start, end)
            files = _writes_only_to_file(text[segment_start:end], before, after)
            for record in segment_heredocs:
                record["files"] = files if record["quoted"] and len(segment_heredocs) == 1 else None
        segment_heredocs = []

    while i < n:
        ch = text[i]
        if quote == "'":
            if ch == "'":
                quote = None
            i += 1
            continue
        if quote == "$'":
            # ANSI-C quoting: a backslash escapes the next character, so \' does not end it.
            if ch == "\\":
                i += 2
                continue
            if ch == "'":
                quote = None
            i += 1
            continue
        if quote == '"':
            if ch == "\\":
                i += 2
                continue
            if ch == '"':
                quote = None
            i += 1
            continue
        if ch == "\\":
            i += 2
            continue
        if text.startswith("$'", i):
            quote = "$'"
            i += 2
            continue
        # $"…" (locale translation) quotes as "…" does: the $ is an ordinary character here and the
        # double quote that follows opens a double-quoted string.
        if ch in ("'", '"'):
            quote = ch
            i += 1
            continue
        if ch == "#" and (i == 0 or text[i - 1] in " \t\n;&|()"):
            end = text.find("\n", i)
            i = n if end < 0 else end
            continue
        if text.startswith("<<<", i):
            i += 3
            continue
        if text.startswith("<<", i):
            j = i + 2
            strip_tabs = j < n and text[j] == "-"
            if strip_tabs:
                j += 1
            while j < n and text[j] in " \t":
                j += 1
            delimiter, quoted, k = _heredoc_word(text, j)
            record = {"delimiter": delimiter, "strip": strip_tabs, "quoted": quoted, "files": None}
            heredocs.append(record)
            pending.append(record)
            segment_heredocs.append(record)
            i = k
            continue
        if ch in SEGMENT_END:
            close_segment(i)
            segment_start = i + 1
            if ch == "\n" and pending:
                i += 1
                for record in pending:
                    record["body_start"] = i
                    while True:
                        if i >= n:
                            raise Unparseable("unterminated heredoc")
                        end = text.find("\n", i)
                        end = n if end < 0 else end
                        line = text[i:end]
                        candidate = line.lstrip("\t") if record["strip"] else line
                        if candidate == record["delimiter"]:
                            record["body_end"] = i
                            i = min(end + 1, n)
                            break
                        i = end + 1
                pending = []
                segment_start = i
                continue
            i += 1
            continue
        i += 1
    close_segment(n)
    if quote or pending:
        raise Unparseable("open quote or heredoc")
    blank = []
    for record in heredocs:
        files = record.get("files")
        if not files or "body_start" not in record:
            continue
        elsewhere = text[:record["body_start"]] + text[record["body_end"]:]
        # Its own command names the file once; any other mention (a later bash, chmod, source,
        # cat or ./ of it) keeps the body scanned.
        if any(elsewhere.count(os.path.basename(name)) > 1 for name in files):
            continue
        blank.append((record["body_start"], record["body_end"]))
    # ANSI-C quoting is tracked above, but shlex, which reads the result, does not know it: when the
    # two readers could disagree about where a quote ends, no body is treated as data.
    if not blank or "$'" in text:
        return text
    out = []
    cursor = 0
    for start, end in sorted(blank):
        out.append(text[cursor:start])
        out.append("\n" * text.count("\n", start, end))
        cursor = end
    out.append(text[cursor:])
    return "".join(out)


def _tokens_or_raise(text):
    tokens = parser._tokenize(text)
    if not tokens and text.strip():
        raise Unparseable("tokenize")
    return tokens


def git_stages(command, depth=0):
    """argv of every stage whose program is git, including inside nested bash -c; raises Unparseable."""
    found = []
    if depth > 4:
        if GIT_TOKEN.search(command):
            raise Unparseable("nesting")
        return found
    _tokens_or_raise(command)
    for chain in parser._chains(command):
        for stage in chain:
            argv = parser._strip_wrappers(stage)
            if not argv:
                continue
            head = Path(argv[0]).name
            if head == "git":
                found.append(argv)
            elif head in SHELLS:
                index = parser._find_dash_c(argv[1:])
                if index is not None and index + 2 < len(argv):
                    found.extend(git_stages(argv[index + 2], depth + 1))
    return found


def _workspace_repo_root():
    root = ROOT.resolve()
    if root.parent.name == ".worktrees":
        return root.parent.parent
    return root


def _under(path, root):
    return path == root or path.startswith(root.rstrip("/") + "/")


def _scratch_roots():
    roots = ["/tmp"]
    tmpdir = os.environ.get("TMPDIR")
    if tmpdir and os.path.isabs(tmpdir):
        roots.append(tmpdir)
    return [os.path.realpath(root) for root in roots]


def _names_workspace(value, workspace):
    """Whether an argument names the workspace, a URL, a parent segment, or anything not literal."""
    if EXPANDS.search(value) or URL_LIKE.search(value) or PARENT_SEGMENT.search(value):
        return True
    if workspace in value or str(ROOT) in value:
        return True
    for piece in [value] + value.split("=")[1:]:
        if piece.startswith("/") and _under(os.path.realpath(piece), workspace):
            return True
    return False


def _scratch_repository(real, root, workspace):
    """Whether the repository at or above `real` (inside `root`) stands apart from the workspace."""
    path = real
    while True:
        marker = os.path.join(path, ".git")
        if os.path.lexists(marker):
            break
        if path == root or not _under(path, root):
            return False
        parent = os.path.dirname(path)
        if parent == path:
            return False
        path = parent
    if os.path.islink(marker) and _under(os.path.realpath(marker), workspace):
        return False
    if os.path.isfile(marker):
        with open(marker, encoding="utf-8") as handle:
            line = handle.readline().strip()
        if not line.startswith("gitdir:"):
            return False
        gitdir = line[len("gitdir:"):].strip()
        gitdir = os.path.realpath(os.path.join(path, gitdir))
        if _under(gitdir, workspace) or not any(_under(gitdir, r) for r in _scratch_roots()):
            return False
        marker = gitdir
    gitdir = os.path.realpath(marker)
    if _under(gitdir, workspace) or not any(_under(gitdir, r) for r in _scratch_roots()):
        return False
    # A commondir file makes this git directory a worktree of the repository it names, and an
    # alternates file borrows another repository's objects: either can tie a scratch path to the
    # workspace, so both are refused outright (a guard must not run git to ask where they lead).
    for tie in ("commondir", os.path.join("objects", "info", "alternates")):
        if os.path.lexists(os.path.join(gitdir, tie)):
            return False
    # A symlink anywhere in the git directory (objects, refs/heads, HEAD, packed-refs, any name, at
    # any depth) that resolves outside the scratch roots, the workspace included, makes a write here a
    # write there. The walk is bounded; a git directory too large to walk is refused, never trusted.
    seen = 0
    for base, dirs, files in os.walk(gitdir, followlinks=False):
        for name in dirs + files:
            seen += 1
            if seen > SCRATCH_GIT_ENTRIES:
                return False
            path = os.path.join(base, name)
            if os.path.islink(path):
                target = os.path.realpath(path)
                if _under(target, workspace) or not any(_under(target, r) for r in _scratch_roots()):
                    return False
    config = os.path.join(gitdir, "config")
    if os.path.exists(config):
        with open(config, encoding="utf-8", errors="replace") as handle:
            body = handle.read().lower()
        if "worktree" in body or "include" in body:
            return False
    return True


def _exempt(argv):
    """Whether one git stage is let through as a call into a scratch repository."""
    i = 1
    target = None
    while i < len(argv) and argv[i].startswith("-"):
        if argv[i] == "-C" and target is None and i + 1 < len(argv):
            target = argv[i + 1]
            i += 2
            continue
        return False  # any other global option, or a second -C
    if target is None or i >= len(argv):
        return False
    verb = argv[i]
    rest = argv[i + 1:]
    if verb not in LOCAL_VERBS:
        return False
    if verb == "rebase" and any(a == "--exec" or a.startswith("--exec=") or a.startswith("-x") for a in rest):
        return False
    if not os.path.isabs(target) or EXPANDS.search(target):
        return False
    workspace = os.path.realpath(_workspace_repo_root())
    real = os.path.realpath(target)
    if not os.path.isdir(real) or _under(real, workspace):
        return False
    roots = [r for r in _scratch_roots() if _under(real, r) and not _under(r, workspace)]
    if not roots:
        return False
    if any(_names_workspace(a, workspace) for a in rest):
        return False
    if verb == "init":
        return True  # nothing exists yet to point anywhere; its arguments were checked above
    return _scratch_repository(real, roots[0], workspace)


def policed_stages(command):
    """The git stages the hook refuses; raises Unparseable when the command cannot be read."""
    text = _scan(command)
    stages = git_stages(text)
    if not stages:
        return stages
    if REPOSITORY_ENV.search(command) or any(os.environ.get(k) for k in LOCATING_ENV):
        return stages
    if all(Path(argv[0]).name == "git" and argv[0] in ("git", "/usr/bin/git") and _exempt(argv)
           for argv in stages):
        return []
    return stages


def uses_git(command):
    return bool(policed_stages(command))


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
    for argv in policed_stages(command):
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
    if not isinstance(command, str):
        return None
    try:
        stages = policed_stages(command)
    except Exception:
        stages = None  # unreadable: refused below when a git token is in it, never passed
    if stages == [] or (stages is None and not GIT_TOKEN.search(command)):
        return None
    # A leading comment belongs only to this invocation, never ambient environment.
    first = command.splitlines()[0].strip()
    prefix = "# Brit fallback: "
    if first.startswith(prefix) and len(first[len(prefix):].strip()) >= 12:
        return {"additionalContext": "Brit fallback recorded in this tool call: " + first[len(prefix):]}
    return {"permissionDecision": "deny", "permissionDecisionReason": deny_reason(command)}


def main():
    raw = b""
    try:
        raw = sys.stdin.buffer.read(1572865)
        if len(raw) > 1572864:
            raise ValueError("event too large")
        result = evaluate(json.loads(raw))
    except Exception:
        # An event that cannot be read is refused when a git token is in it; only one without passes.
        try:
            text = raw.decode("utf-8", "replace")
        except Exception:
            text = ""
        if GIT_TOKEN.search(text):
            result = {"permissionDecision": "deny", "permissionDecisionReason": GENERIC}
        else:
            result = {"additionalContext": "Brit reminder coverage gap: shell event could not be decoded."}
    if result:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", **result}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
