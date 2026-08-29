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

4. ONE BUILD AT A TIME — a heavy cargo (or `just gate`) while another cargo
   build/test/check (or its rustc/rust-lld children) is already running in
   this container is DENIED, naming the pid to wait for. Two gates at once
   on a box whose storage is the bottleneck is what the operator asked us to
   stop doing; the deny replaces "remember not to".

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
import subprocess
import sys

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
    "crates",
]

# WASM/DNA contexts — exempt entirely (plain cargo required; small artifacts).
WASM_MARKERS = ["holochain/dna/", "elohim-wasm", "wasm32-unknown-unknown", "rust-ipfs"]

HEAVY_VERBS = {
    "build", "b", "test", "t", "check", "c", "clippy", "run", "r",
    "nextest", "bench", "doc", "install",
}
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


def running_heavy():
    """Live cargo drivers (heavy verb) and compiler/linker children in this
    container, excluding this hook's own process tree. [(pid, short cmd)]."""
    me = {os.getpid(), os.getppid()}
    found = []
    try:
        for name in os.listdir(PROC_DIR):
            if not name.isdigit() or int(name) in me:
                continue
            try:
                with open(os.path.join(PROC_DIR, name, "cmdline"), "rb") as f:
                    argv = f.read().split(b"\0")
            except Exception:
                continue
            argv = [a.decode("utf-8", "replace") for a in argv if a]
            if not argv:
                continue
            head = os.path.basename(argv[0])
            if head == "cargo":
                verb = next((a for a in argv[1:] if not a.startswith("+")), None)
                if verb in HEAVY_VERBS:
                    found.append((int(name), " ".join(argv[:4])))
            elif head in ("rustc", "rust-lld", "ld.lld", "cc1", "gate-runner.mjs"):
                found.append((int(name), head))
    except Exception:
        return []
    return found


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


def family_for(cwd: str):
    """Mirror pool-lib.sh detect_family exactly (review C8): env override →
    .family file → branch with feat/fix/chore/worktree- prefix strip →
    first [-/] token, lowercased."""
    env = os.environ.get("CARGO_TARGET_POOL_FAMILY")
    if env:
        return env
    wt = worktree_root(cwd)
    fam_file = os.path.join(wt, ".family")
    try:
        if os.path.isfile(fam_file):
            v = open(fam_file).read().strip()
            if v:
                return v
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
        return re.split(r"[-/]", branch, 1)[0].lower()
    except Exception:
        return None


def slot_for(ws: str, cwd: str, release: bool):
    fam = family_for(cwd)
    if not fam:
        return None
    # exact mirror of pool-lib flatten_path: '/'->'_' then runs of '_'->'__'
    flat = re.sub(r"_+", "__", ws.replace("/", "_"))
    profile = "release" if release else "dev"
    return f"{POOL_ROOT}/family/{fam}/{flat}/{profile}"


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
        hint = (
            f"CARGO_TARGET_DIR={slot} " if slot
            else "CARGO_TARGET_DIR=<pool slot from the SessionStart preflight block> "
        )
        deny(
            f"POOL DISCIPLINE: heavy cargo in native workspace '{ws}' without "
            f"CARGO_TARGET_DIR — in-tree target/ dirs balloon the PVC (this is "
            f"deterministic policy, not advice). Re-run with the pool slot "
            f"inline, e.g.:\n  {hint}{command}\n"
            f"(Slot map: SessionStart 'ELOHIM CARGO TARGET POOL' block. "
            f"DNA/WASM workspaces are exempt and must keep plain cargo.)"
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

    # Rule 4 — one build at a time in this container.
    live = running_heavy()
    if live and len({p for p, _ in live}) >= int(io_pol["max_concurrent_heavy"]):
        drivers = [c for _, c in live if c.startswith("cargo") or c == "gate-runner.mjs"] or [live[0][1]]
        pids = ", ".join(str(p) for p, c in live if c.startswith("cargo") or c == "gate-runner.mjs") or str(live[0][0])
        deny(
            f"ONE BUILD AT A TIME: a build is already running in this workspace — pid {pids}: "
            f"{drivers[0]} ({len(live)} compiler/linker processes). Two concurrent builds saturate "
            f"the host's worn NVMe mirror (2026-08-29 resets). Wait for it: "
            f"`while pgrep -x rustc >/dev/null || pgrep -f 'cargo (build|test|check|clippy)' >/dev/null; do sleep 15; done` "
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

    # Soft watermark — advisory only.
    if pct is not None and soft <= pct < hard:
        advise(
            f"DISK SOFT WATERMARK: /projects at {pct}% (soft {soft}%, hard {hard}%). "
            f"Heavy builds still allowed; `cargo-pool enforce --yes` reclaims "
            f"per policy before the hard ceiling blocks builds."
        )


if __name__ == "__main__":
    try:
        main()
        sys.exit(0)
    except Exception:
        # Fail-open: a guard bug must never block development.
        sys.exit(0)
