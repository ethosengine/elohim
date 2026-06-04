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
    if "cargo" not in command:
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
    ws = native_ws_for(" ; ".join(heavy), command, cwd)
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
