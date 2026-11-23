#!/usr/bin/env python3
"""
Cargo Disk Guard — deterministic PVC-pressure enforcement.

Hook Type: PreToolUse
Matcher: Bash

Two deterministic rules (policy: genesis/agentic/pool-policy.json):

1. HARD CEILING — when /projects volume usage >= volume_hard_pct, DENY any
   heavy cargo invocation (build/test/check/clippy/run/nextest/bench/doc/
   install). Agents physically cannot starve the PVC; the deny reason names
   the reclaim command (`cargo-pool enforce --yes`).

2. POOL DISCIPLINE — heavy cargo in a NATIVE workspace without an explicit
   CARGO_TARGET_DIR is DENIED with the exact pool-slot export to use.
   In-tree target/ dirs can never balloon again (measured 2026-06-04:
   20.7GB of non-pooled in-tree targets). DNA/WASM workspaces are exempt
   (hc dna pack canonicalizes ./target — they MUST stay plain cargo).

3. I/O PRESSURE — when the HOST's /proc/pressure/io (readable in-container,
   mirrors the node) shows `full avg10` >= io.psi_full_avg10_deny_pct or
   `some avg60` >= io.psi_some_avg60_deny_pct, DENY new heavy work. Evidence
   2026-08-29: the control-plane host hard-reset twice with psi_io some=77%,
   45 procs in D-state, md2 queue depth 218-277 at 155ms write latency —
   two worn QLC NVMes (past rated TBW) that cannot absorb concurrent builds.
   Starting another build into that is the proximate trigger.

4. BUILD SEATS — a heavy cargo (or `just gate`) while `io.max_concurrent_heavy`
   builds are already running in this container is DENIED, naming each
   build and what to wait for. A build is a cargo driver or gate-runner that
   has no other driver below it or is itself compiling, together with its
   rustc/rust-lld/cc1 children: a gate is one build, not many processes, and
   two cargos compiling at once are two, nested or side by side
   (`group_builds`). Too many builds at once on a box
   whose storage is the bottleneck is what the operator asked us to stop
   doing; the deny replaces "remember not to". Marked LSP flychecks
   yield to builds: stop their wrappers and let their cleanup release leases.

5. JOBS CAP — heavy cargo with no `-j/--jobs` and no CARGO_BUILD_JOBS (in the
   command or the environment) is DENIED with the exact prefix to use
   (io.default_jobs). The devfile sets CARGO_BUILD_JOBS for new workspaces;
   this rule covers sessions born before that env existed, and retires
   itself the moment the env is present.

`just gate …` segments count as heavy for rules 1, 3, 4, 5 (gate-runner sets
its own CARGO_TARGET_DIR, so rule 2 does not apply to them).

Soft watermark (volume_soft_pct) emits an advisory additionalContext only.

PARSING MODEL (adversarial-review hardened): commands are split into shell
segments (on && || ; | and newlines) and a segment is "heavy cargo" only
when, after stripping leading env-assignments and common wrappers
(timeout/nice/env/command), its HEAD token is `cargo` and the following verb
is heavy. This kills the substring false-positives (cargo verbs inside git
commit messages / echo strings) and the 'cargo-pool'-anywhere ceiling
bypass: `cargo-pool status && cargo build` gates the second segment, while
`bash .../cargo-pool enforce --yes` has no cargo-headed segment at all.

Fail-open by design: any internal error exits 0 silently — a guard bug must
never block development. Fast path only: statvfs + parsing (no du, no
network; one short git subprocess only when composing a deny hint).
"""

# The intervenor's removal condition (Meadows' shifting-the-burden trap;
# counted by _lib/intervenor_census.py). A condition, never a date.
RETIRE_WHEN = (
    "when the cargo-pool policy reconciler holds the volume under the soft watermark for a full "
    "quarter with no deny path firing — the guard exists because reclamation lags allocation, "
    "so it retires when that lag closes, not when the disk happens to be quiet."
)
import json
import os
import re
import shlex
import signal
import stat as stat_mod
import subprocess
import sys
import time

POOL_ROOT = os.environ.get("CARGO_TARGET_POOL_ROOT", "/projects/.cargo-target-pool")
PROJECT_DIR = os.environ.get("CLAUDE_PROJECT_DIR", "/projects/elohim")
POLICY_FILE = os.path.join(PROJECT_DIR, "genesis", "agentic", "pool-policy.json")
VOLUME = "/projects"

# Native workspaces whose builds must land in the pool (mirror of
# pool-lib.sh NATIVE_WORKSPACES). 'crates' is cwd-matched only — the bare
# word is too generic for command-string matching.
NATIVE_WS = [
    "elohim/elohim-storage",
    "doorway/doorway-service",
    "steward/node",
    "elohim/holochain/tests/sweettest",
    "elohim/holochain/tools/hc-dbtool",
    "crates",
]

# WASM/DNA contexts — exempt entirely (plain cargo required; small artifacts).
WASM_MARKERS = ["holochain/dna/", "elohim-wasm", "wasm32-unknown-unknown", "rust-ipfs"]

HEAVY_VERBS = {
    "build", "b", "test", "t", "check", "c", "clippy", "run", "r",
    "nextest", "bench", "doc", "d", "install",
}
# Compiler and linker processes a build runs; rule 4 groups them under their cargo.
COMPILER_HEADS = (
    "rustc", "rustdoc", "clippy-driver", "rust-lld", "ld.lld", "cc1", "cc1plus", "clang",
)
WRAPPERS = {"timeout", "nice", "env", "command", "sudo"}
ENV_ASSIGN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
SEGMENT_SPLIT = re.compile(r"\|\||&&|;|\||\n")


def read_policy():
    try:
        with open(POLICY_FILE) as f:
            p = json.load(f)
        return int(p.get("volume_hard_pct", 85)), int(p.get("volume_soft_pct", 75))
    except Exception:
        return 85, 75


IO_DEFAULTS = {
    "psi_full_avg10_deny_pct": 20.0,
    "psi_some_avg60_deny_pct": 50.0,
    "max_concurrent_heavy": 1,
    "default_jobs": 4,
}


def read_io_policy():
    try:
        with open(POLICY_FILE) as f:
            io_pol = json.load(f).get("io") or {}
    except Exception:
        io_pol = {}
    out = dict(IO_DEFAULTS)
    for k in IO_DEFAULTS:
        if k in io_pol:
            out[k] = io_pol[k]
    return out


PSI_FILE = os.environ.get("CARGO_GUARD_PSI_FILE", "/proc/pressure/io")
PROC_DIR = os.environ.get("CARGO_GUARD_PROC_DIR", "/proc")


def host_psi_io():
    """{'some_avg10','some_avg60','full_avg10','full_avg60'} floats, or None.
    /proc/pressure/io inside the pod reports the HOST's global PSI (verified
    2026-08-29 against the operator's sar on the same node)."""
    try:
        out = {}
        with open(PSI_FILE) as f:
            for line in f:
                parts = line.split()
                if not parts or parts[0] not in ("some", "full"):
                    continue
                for kv in parts[1:]:
                    k, _, v = kv.partition("=")
                    if k in ("avg10", "avg60"):
                        out[f"{parts[0]}_{k}"] = float(v)
        return out or None
    except Exception:
        return None


def proc_argv(pid):
    try:
        with open(os.path.join(PROC_DIR, str(pid), "cmdline"), "rb") as f:
            return [a.decode("utf-8", "replace") for a in f.read().split(b"\0") if a]
    except OSError:
        return []


def flycheck_marked(pid):
    try:
        with open(os.path.join(PROC_DIR, str(pid), "environ"), "rb") as f:
            return b"ELOHIM_LSP_FLYCHECK=1" in f.read().split(b"\0")
    except OSError:
        # An unreadable environment is never evidence that a build may be killed.
        return False


def proc_stat(pid):
    """State, parent and start time; comm may itself contain spaces or ')'."""
    try:
        with open(os.path.join(PROC_DIR, str(pid), "stat")) as f:
            fields = f.read().rsplit(")", 1)[1].split()
        return fields[0], int(fields[1]), fields[19]
    except (OSError, ValueError, IndexError):
        return None


def flycheck_wrapper(pid):
    argv = proc_argv(pid)
    if not argv:
        return False
    head = os.path.basename(argv[0])
    return head == "lsp-flycheck" or (
        head == "bash" and len(argv) > 1 and os.path.basename(argv[1]) == "lsp-flycheck"
    )


def flycheck_root(pid):
    """Climb only the marked tree, including its bash wrapper (bash exports
    need not appear in the wrapper's own /proc environ until the next exec)."""
    seen = {pid}
    while (stat := proc_stat(pid)) is not None:
        parent = stat[1]
        if parent <= 1 or parent in seen:
            break
        if not (flycheck_marked(parent) or flycheck_wrapper(parent)):
            break
        seen.add(parent)
        pid = parent
    return pid


def yield_flychecks(live):
    """Signal each proven flycheck root once; share one three-second wait."""
    roots = {flycheck_root(pid) for pid, _, yieldable in live if yieldable}
    pending = {}
    messages = []
    for pid in sorted(roots):
        before = proc_stat(pid)
        if before is None or before[0] == "Z":
            continue
        # Revalidate immediately before signalling: a departed/reused PID must
        # never turn a flycheck cancellation into another session's lost build.
        if not (flycheck_marked(pid) or flycheck_wrapper(pid)):
            continue
        current = proc_stat(pid)
        if current is None or current[2] != before[2]:
            continue
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            continue
        except OSError as error:
            messages.append(f"flycheck yield: could not stop pid {pid}: {error}")
            continue
        pending[pid] = before[2]
    deadline = time.monotonic() + 3
    while pending:
        for pid, started in list(pending.items()):
            stat = proc_stat(pid)
            if stat is None or stat[0] == "Z" or stat[2] != started:
                messages.append(f"yielded: stopped lsp flycheck pid {pid} for this build")
                del pending[pid]
        remaining = deadline - time.monotonic()
        if not pending or remaining <= 0:
            break
        time.sleep(min(0.05, remaining))
    messages.extend(f"flycheck yield: pid {pid} still stopping after 3 s" for pid in pending)
    return messages


CARGO_VALUE_OPTIONS = {"--color", "--config", "--explain", "-C", "-Z"}


def cargo_verb(argv):
    """The subcommand of a cargo argv: the first argument that is neither a
    `+toolchain`, an option, nor the value of an option cargo takes before
    its subcommand (`cargo --color always build`)."""
    skip = False
    for arg in argv[1:]:
        if skip:
            skip = False
        elif arg in CARGO_VALUE_OPTIONS:
            skip = True
        elif not arg.startswith(("+", "-")):
            return arg
    return None


def running_heavy():
    """Live cargo drivers (heavy verb) and compiler/linker children in this
    container, excluding this hook and parent. [(pid, short cmd, yieldable)].
    Also read by lsp-flycheck before it attempts berth admission."""
    me = {os.getpid(), os.getppid()}
    found = []
    try:
        for name in os.listdir(PROC_DIR):
            if not name.isdigit() or int(name) in me:
                continue
            argv = proc_argv(name)
            if not argv:
                continue
            head = os.path.basename(argv[0])
            if head == "cargo":
                verb = cargo_verb(argv)
                if verb in HEAVY_VERBS:
                    found.append((int(name), " ".join(argv[:4]), flycheck_marked(name)))
            elif head in COMPILER_HEADS or head == "gate-runner.mjs":
                found.append((int(name), head, flycheck_marked(name)))
            elif head in ("node", "nodejs"):
                script = next((a for a in argv[1:] if not a.startswith("-")), "")
                if os.path.basename(script) == "gate-runner.mjs":
                    found.append((int(name), "gate-runner.mjs", flycheck_marked(name)))
    except Exception:
        return []
    return found


MAX_ANCESTRY_DEPTH = 64


def proc_parent(pid):
    """Parent pid, or None when the process is gone or unreadable; never raises."""
    try:
        stat = proc_stat(pid)
        return stat[1] if stat else None
    except Exception:
        return None


def owning_cargo(pid):
    """The nearest ancestor that is a cargo of ANY subcommand, or None."""
    seen, cur = {pid}, pid
    for _ in range(MAX_ANCESTRY_DEPTH):
        cur = proc_parent(cur)
        if cur is None or cur <= 1 or cur in seen:
            return None
        seen.add(cur)
        argv = proc_argv(cur)
        if argv and os.path.basename(argv[0]) == "cargo":
            return cur
    return None


def group_builds(live):
    """Count the builds among the non-yieldable heavy processes.

    `live` is [(pid, cmd)]. A DRIVER is a cargo with a heavy verb or a
    gate-runner; everything else is a compiler or linker. Each of these holds
    a seat:
      - a driver with no driver below it (a gate-runner between cargo steps,
        or the cargo it is running);
      - a driver that is the nearest driver above a live compiler or linker,
        so a cargo still compiling while a child cargo also compiles is two
        builds, as are two cargos side by side under one parent: ancestry
        alone does not make concurrent compiles one;
      - a cargo of any subcommand with a live compiler under it, listed by
        running_heavy or not, so an unrecognised cargo spelling cannot hide
        two builds behind the gate-runner above them;
      - a compiler with no driver above it (its cargo died), at its highest
        compiler ancestor;
      - any process whose ancestry walk was cut short by a parent loop or by
        MAX_ANCESTRY_DEPTH, counted on its own.
    Every ancestor up to pid 1 is walked, since bash, just and
    run-local-gate.sh sit between a gate-runner and its cargo.

    Returns [(pid, cmd, root_pid, root_cmd)] sorted by pid, where the root is
    the highest heavy ancestor found: the thing to wait for. A vanished or
    unreadable stat ends that walk, which can only split one build into more.
    If the grouping fails, or finds no build among live processes, every
    process counts as its own build: an error here must refuse work, never
    admit it. Never raises."""
    try:
        cmds = dict(live)
        # A cargo that running_heavy did not list (a subcommand it does not know)
        # is still a build while a compiler runs under it.
        for pid, cmd in live:
            if cmd in COMPILER_HEADS:
                owner = owning_cargo(pid)
                if owner is not None and owner not in cmds:
                    cmds[owner] = " ".join(proc_argv(owner)[:4]) or "cargo"
        drivers = {pid for pid, cmd in cmds.items() if cmd not in COMPILER_HEADS}

        def heavy_ancestors(pid):  # ([nearest first], walk reached the top)
            found, seen, cur = [], {pid}, pid
            for _ in range(MAX_ANCESTRY_DEPTH):
                cur = proc_parent(cur)
                if cur is None or cur <= 1:
                    return found, True
                if cur in seen:
                    return found, False
                seen.add(cur)
                if cur in cmds:
                    found.append(cur)
            return found, False

        walks = {pid: heavy_ancestors(pid) for pid in cmds}
        above = {pid: walk[0] for pid, walk in walks.items()}
        cut = {pid for pid, walk in walks.items() if not walk[1]}
        has_driver_below = {a for pid in drivers - cut for a in above[pid] if a in drivers}
        builds = (drivers - has_driver_below) | cut
        for pid in set(cmds) - drivers - cut:
            nearest = next((a for a in above[pid] if a in drivers), None)
            if nearest is not None:
                builds.add(nearest)
            else:
                builds.add(above[pid][-1] if above[pid] else pid)
        if cmds and not builds:
            raise ValueError("live processes but no build")
        rows = []
        for pid in builds:
            root = above[pid][-1] if above[pid] else pid
            rows.append((pid, cmds[pid], root, cmds[root]))
        return sorted(rows)
    except Exception:
        return sorted((pid, cmd, pid, cmd) for pid, cmd in set(live))


def has_jobs_cap(command: str, heavy: list) -> bool:
    if os.environ.get("CARGO_BUILD_JOBS", "").strip():
        return True
    if re.search(r"(^|[\s;&|])CARGO_BUILD_JOBS=\S+", command):
        return True
    for seg in heavy:
        toks = seg.split()
        if any(t == "-j" or t.startswith("-j") and t[2:].isdigit() or t == "--jobs" or t.startswith("--jobs=") for t in toks):
            return True
    return False


def disk_pct():
    """df-exact percentage: used/(used+avail), ceil — NOT used/blocks, which
    reads 1-3% low because of reserved blocks and would drift from every
    other consumer of these thresholds (pool-lib, pre-push) that uses df."""
    try:
        st = os.statvfs(VOLUME)
        used = st.f_blocks - st.f_bfree
        denom = used + st.f_bavail
        if denom <= 0:
            return None
        return int((used * 100 + denom - 1) // denom)
    except Exception:
        return None


OPERATORS = {"&&", "||", ";", "|", "&", "\n"}


def token_segments(command: str):
    """Tokenize the WHOLE command with shlex (so operators inside quoted
    arguments stay inside their token — a git-commit message or JSON blob
    containing '&& cargo build' cannot fake a command boundary), then split
    the token stream on real operator tokens. Falls back to a textual split
    only when shlex cannot parse."""
    try:
        toks = shlex.split(command.replace("\n", " ; "))
    except ValueError:
        return [seg.split() for seg in SEGMENT_SPLIT.split(command)]
    segs, cur = [], []
    for t in toks:
        if t in OPERATORS:
            if cur:
                segs.append(cur)
            cur = []
            continue
        # glued trailing separator: `foo;`
        if t.endswith(";") and t not in OPERATORS:
            cur.append(t.rstrip(";"))
            segs.append(cur)
            cur = []
            continue
        cur.append(t)
    if cur:
        segs.append(cur)
    return segs


def segment_head_and_verb(toks):
    """Return (head_basename, verb) for a token list, after stripping
    leading env assignments and common wrappers. (None, None) if empty."""
    i = 0
    while i < len(toks) and ENV_ASSIGN.match(toks[i]):
        i += 1
    while i < len(toks) and os.path.basename(toks[i]) in WRAPPERS:
        wrapper = os.path.basename(toks[i])
        i += 1
        if wrapper == "timeout" and i < len(toks) and re.match(r"^[\d.]+[smhd]?$", toks[i]):
            i += 1
    if i >= len(toks):
        return None, None
    head = os.path.basename(toks[i])
    # verb: first following token that isn't a +toolchain selector
    j = i + 1
    while j < len(toks) and toks[j].startswith("+"):
        j += 1
    verb = toks[j] if j < len(toks) else None
    return head, verb


def heavy_cargo_segments(command: str):
    """Command segments whose head is `cargo` with a heavy verb."""
    out = []
    for toks in token_segments(command):
        head, verb = segment_head_and_verb(toks)
        if head == "cargo" and verb in HEAVY_VERBS:
            out.append(" ".join(toks))
        elif head == "just" and verb == "gate":
            out.append(" ".join(toks))
    return out


def native_ws_for(heavy_text: str, command: str, cwd: str):
    """Which native workspace is this build running in, if any. The
    text arm matches only inside the HEAVY SEGMENTS (token-anchored, so
    quoted prose can't reach here) on real build-context markers
    (--manifest-path / cd) — never on a bare substring (review C4)."""
    for ws in NATIVE_WS:
        if ws == "crates":
            continue  # too generic; cwd-only below
        esc = re.escape(ws)
        if re.search(r"--manifest-path[= ]\S*" + esc, heavy_text):
            return ws
        if re.search(r"\bcd\s+\S*" + esc + r"(\s|/|$)", command):
            return ws
    for ws in NATIVE_WS:
        marker = "/" + ws
        if cwd.endswith(marker) or (marker + "/") in cwd:
            if ws == "crates" and "holochain/crates" in cwd:
                continue  # not the repo-root crates workspace
            return ws
    return None


def worktree_root(cwd: str):
    d = cwd or PROJECT_DIR
    while d and d != "/":
        if os.path.exists(os.path.join(d, ".git")):
            return d
        d = os.path.dirname(d)
    return PROJECT_DIR


# Mirror of pool-lib.sh pool_component_ok: one path component under the pool.
# fullmatch, so a trailing newline fails as it does under bash `[[ =~ ]]`.
_POOL_COMPONENT = re.compile(r"[A-Za-z0-9_][A-Za-z0-9._-]{0,199}")
_FAMILY_REJECTED = set()


def pool_component_ok(value) -> bool:
    return isinstance(value, str) and _POOL_COMPONENT.fullmatch(value) is not None


def _family_reject(value: str, source: str) -> None:
    """A source offered a family it may not have: said once on stderr, never used."""
    if (value, source) in _FAMILY_REJECTED:
        return
    _FAMILY_REJECTED.add((value, source))
    print(f"cargo-pool: ignoring family {value!r} from {source} "
          f"(expected one path component of [A-Za-z0-9._-])", file=sys.stderr)


def _read_trimmed(path: str) -> str:
    with open(path) as fh:
        return fh.read().strip()


def family_for(cwd: str):
    """Mirror pool-lib.sh detect_family exactly (review C8): env override →
    .family file → the worktree's recorded family (<git-dir>/cargo-pool-family,
    written on first derivation from a real branch; backlog row 18) → branch
    with feat/fix/chore/worktree- prefix strip → first [-/] token, lowercased.
    Every source must be one valid path component; an invalid one is reported
    and skipped. Where bash falls back to the directory basename (detached head,
    or a branch yielding no valid family) this returns None."""
    env = os.environ.get("CARGO_TARGET_POOL_FAMILY")
    if env:
        v = env.strip()
        if pool_component_ok(v):
            return v
        _family_reject(env, "CARGO_TARGET_POOL_FAMILY")
    wt = worktree_root(cwd)
    fam_file = os.path.join(wt, ".family")
    try:
        if os.path.isfile(fam_file):
            v = _read_trimmed(fam_file)
            if pool_component_ok(v):
                return v
            if v:
                _family_reject(v, fam_file)
    except Exception:
        pass
    record = None
    try:
        gitdir = subprocess.run(
            ["git", "-C", wt, "rev-parse", "--absolute-git-dir"],
            capture_output=True, text=True, timeout=2,
        ).stdout.strip()
        if gitdir and os.path.isdir(gitdir):
            record = os.path.join(gitdir, "cargo-pool-family")
            # Only a regular file that is not a symlink is read.
            try:
                st = os.lstat(record)
            except FileNotFoundError:
                st = None
            if st is not None and stat_mod.S_ISREG(st.st_mode):
                v = _read_trimmed(record)
                if pool_component_ok(v):
                    return v
                if v:
                    _family_reject(v, record)
    except Exception:
        pass
    try:
        branch = subprocess.run(
            ["git", "-C", wt, "rev-parse", "--abbrev-ref", "HEAD"],
            capture_output=True, text=True, timeout=2,
        ).stdout.strip()
        if not branch or branch == "HEAD":
            return None
        for prefix in ("feat/", "feature/", "fix/", "chore/"):
            if branch.startswith(prefix):
                branch = branch[len(prefix):]
                break
        else:
            if branch.startswith("worktree-"):
                branch = branch[len("worktree-"):]
        fam = re.split(r"[-/]", branch, 1)[0].lower()
    except Exception:
        return None
    if not pool_component_ok(fam):
        if fam:
            _family_reject(fam, f"branch {branch}")
        return None
    if record:
        # Create exclusively: O_EXCL never follows or replaces an existing
        # entry (an invalid record, a symlink, a fifo, a directory).
        try:
            fd = os.open(record, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o644)
            with os.fdopen(fd, "w") as fh:
                fh.write(fam + "\n")
        except Exception:
            pass
    return fam


def slot_for(ws: str, cwd: str, release: bool):
    fam = family_for(cwd)
    if not fam:
        return None
    # exact mirror of pool-lib flatten_path: '/'->'_' then runs of '_'->'__'
    flat = re.sub(r"_+", "__", ws.replace("/", "_"))
    profile = "release" if release else "dev"
    # Containment: three valid components, so exactly three levels under family/.
    if not all(pool_component_ok(part) for part in (fam, flat, profile)):
        return None
    family_root = os.path.join(POOL_ROOT, "family")
    slot = f"{family_root}/{fam}/{flat}/{profile}"
    if os.path.dirname(os.path.dirname(os.path.dirname(os.path.normpath(slot)))) != os.path.normpath(family_root):
        return None
    return slot


def deny(reason: str):
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }))
    sys.exit(0)


def advise(text: str):
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": text,
        }
    }))
    sys.exit(0)


def main():
    data = json.load(sys.stdin)
    if data.get("tool_name") != "Bash":
        return
    command = (data.get("tool_input") or {}).get("command", "")
    if "cargo" not in command and "just" not in command:
        return

    heavy = heavy_cargo_segments(command)
    if not heavy:
        return

    cwd = data.get("cwd") or os.getcwd()
    for m in WASM_MARKERS:
        if m in command or m in cwd:
            return

    hard, soft = read_policy()
    pct = disk_pct()

    # Rule 1 — hard ceiling: the PVC-starvation gate.
    if pct is not None and pct >= hard:
        deny(
            f"DISK HARD CEILING: /projects at {pct}% >= {hard}% "
            f"(genesis/agentic/pool-policy.json volume_hard_pct). Heavy cargo "
            f"commands are blocked until space is reclaimed — run:\n"
            f"  bash {PROJECT_DIR}/genesis/agentic/bin/cargo-pool enforce --yes\n"
            f"then retry. (cargo-pool status shows the families table; the "
            f"enforce ladder never touches the active family or any slot a "
            f"live cargo holds.)"
        )

    # Rule 2 — pool discipline: native builds must target a pool slot.
    cargo_heavy = [h for h in heavy if not h.startswith("just ")]
    ws = native_ws_for(" ; ".join(cargo_heavy), command, cwd) if cargo_heavy else None
    if ws and "CARGO_TARGET_DIR" not in command:
        release = bool(re.search(r"\s--release\b", command))
        slot = slot_for(ws, cwd, release)
        pointer = (
            f"(Slot: `bash {PROJECT_DIR}/genesis/agentic/bin/cargo-pool key` prints this "
            f"directory's slot; `cargo-pool status` holds the full reference. "
            f"DNA/WASM workspaces are exempt and must keep plain cargo.)"
        )
        if slot:
            body = (
                f"Re-run with the pool slot inline, e.g.:\n  CARGO_TARGET_DIR={slot} {command}\n"
            )
        else:
            body = (
                f"No pool family could be derived here (detached HEAD or no valid family). "
                f"Run `bash {PROJECT_DIR}/genesis/agentic/bin/cargo-pool key` from the "
                f"workspace directory to get the slot, then re-run with "
                f"CARGO_TARGET_DIR=<that slot> {command}\n"
            )
        deny(
            f"POOL DISCIPLINE: heavy cargo in native workspace '{ws}' without "
            f"CARGO_TARGET_DIR — in-tree target/ dirs balloon the PVC (this is "
            f"deterministic policy, not advice). {body}"
            f"{pointer}"
        )

    io_pol = read_io_policy()

    # Rule 3 — host I/O pressure: don't start a build into a stalled disk.
    psi = host_psi_io()
    if psi:
        full10, some60 = psi.get("full_avg10", 0.0), psi.get("some_avg60", 0.0)
        if full10 >= float(io_pol["psi_full_avg10_deny_pct"]) or some60 >= float(io_pol["psi_some_avg60_deny_pct"]):
            deny(
                f"HOST I/O PRESSURE: /proc/pressure/io full avg10={full10:.0f}% some avg60={some60:.0f}% "
                f"(deny at full10>={io_pol['psi_full_avg10_deny_pct']} / some60>={io_pol['psi_some_avg60_deny_pct']}, "
                f"genesis/agentic/pool-policy.json `io`). The control-plane host's NVMe mirror is "
                f"saturated — starting another build now is the pattern that preceded the 2026-08-29 "
                f"hard resets. Wait and re-check with `cat /proc/pressure/io`; light commands are never gated."
            )

    # Rule 4 — build seats: io.max_concurrent_heavy concurrent builds (not processes).
    processes = running_heavy()
    live = [(pid, cmd) for pid, cmd, yieldable in processes if not yieldable]
    seats = int(io_pol["max_concurrent_heavy"])
    builds = group_builds(live) if live else []
    if builds and len(builds) >= seats:
        named = "; ".join(
            f"pid {p}: {c}" + ("" if r == p else f" (under pid {r}: {rc})") for p, c, r, rc in builds)
        waits = " && ".join(f"[ -d /proc/{r} ]" for r in sorted({r for _, _, r, _ in builds}))
        deny(
            f"CARGO SEATS FULL: {len(builds)} of {seats} builds running in this workspace — {named} "
            f"({len(live)} heavy processes in all; seats = pool-policy `io.max_concurrent_heavy`). "
            f"Too many concurrent builds saturate the host's worn NVMe mirror (2026-08-29 resets). "
            f"Wait until fewer than {seats} remain, then retry: `while {waits}; do sleep 15; done` "
            f"returns when one of them has ended, and the retry is checked again "
            f"— never kill another session's build."
        )

    # Rule 5 — jobs cap.
    if not has_jobs_cap(command, heavy):
        n = int(io_pol["default_jobs"])
        deny(
            f"JOBS CAP: heavy cargo with no `-j`/`--jobs` and no CARGO_BUILD_JOBS in the environment. "
            f"This box's storage, not its {os.cpu_count() or 24} cores, is the build bottleneck "
            f"(pool-policy `io.default_jobs`). Re-run with the cap inline:\n"
            f"  CARGO_BUILD_JOBS={n} {command}\n"
            f"(New workspaces get CARGO_BUILD_JOBS from devfile.yaml; this rule retires itself once the env is present.)"
        )

    # Only cancel flychecks once all deny rules have passed: this build will run.
    context = yield_flychecks(processes)

    # Soft watermark — advisory only, sharing the hook's single context response.
    if pct is not None and soft <= pct < hard:
        context.append(
            f"DISK SOFT WATERMARK: /projects at {pct}% (soft {soft}%, hard {hard}%). "
            f"Heavy builds still allowed; `cargo-pool enforce --yes` reclaims "
            f"per policy before the hard ceiling blocks builds."
        )
    if context:
        advise("\n".join(context))


if __name__ == "__main__":
    try:
        main()
        sys.exit(0)
    except Exception:
        # Fail-open: a guard bug must never block development.
        sys.exit(0)
