#!/usr/bin/env python3
"""mempalace-currency — the deterministic staleness tripwire for the MemPalace semantic index.

MemPalace embeddings are FROZEN at mine-time (they do not auto-update when a source file changes), so the index
silently drifts behind the cleaned surface. This is the 7th store's tripwire: it measures whether the index is
current by comparing the last-mine timestamp against the newest mtime on the mined surface — pure stdlib, no
MemPalace CLI dependency (so it runs anywhere, including subagent contexts). The actual re-mine (--remine) does
need the CLI; the staleness CHECK never does.

  --status [--json]   one-line currency for the SessionStart headline (fresh vs N changes since last mine)
  --record            stamp .mempalace/.last-mine = now (call after a successful mine)
  --remine [--wait]   sync (prune deleted) + mine the surface + record  (needs the `mempalace` CLI)
                      Records freshness ONLY if the run was not lock-blocked; --wait retries.

The mined surface = the durable, cleaned multi-destination surface the front-link recalls from (canonical
architecture seeds + curated history + working memory + canonical stories). NEVER the transient pile / raw
code / junk drawer — index only the clean surface (the no-dumping-grounds rule applied to the index).
"""
import sys, os, json, subprocess, datetime, time

# the durable cleaned surface MemPalace indexes (relative to repo root)
SURFACE = [
    "genesis/docs/content/elohim-protocol",   # canonical seeds + curated history (incl. MAP.md)
    ".claude/memory",                          # working memory (the cites-linked lessons)
    "genesis/data/stories",                    # canonical stories (storyteller graduations)
]
GRACE_SECONDS = 120  # ignore sub-2-minute skew between a write and a mine


def _root():
    p = os.path.abspath(__file__)
    while p != "/":
        if os.path.isdir(os.path.join(p, ".mempalace")):
            return p
        p = os.path.dirname(p)
    return os.getcwd()


def _marker(root):
    return os.path.join(root, ".mempalace", ".last-mine")


def last_mine(root):
    """epoch of the last recorded mine; fall back to the palace config mtime if never recorded."""
    mk = _marker(root)
    if os.path.exists(mk):
        try:
            return float(open(mk).read().strip())
        except (OSError, ValueError):
            pass
    cfg = os.path.join(root, ".mempalace", "config.json")
    return os.path.getmtime(cfg) if os.path.exists(cfg) else 0.0


def newest_surface(root):
    """(newest_mtime, count_changed_since_last_mine) across the mined surface .md files."""
    lm = last_mine(root)
    newest = 0.0
    changed = 0
    for rel in SURFACE:
        base = os.path.join(root, rel)
        if not os.path.isdir(base):
            continue
        for dp, _dn, fn in os.walk(base):
            if "/.git" in dp:
                continue
            for f in fn:
                if not f.endswith(".md"):
                    continue
                try:
                    mt = os.path.getmtime(os.path.join(dp, f))
                except OSError:
                    continue
                if mt > newest:
                    newest = mt
                if mt > lm + GRACE_SECONDS:
                    changed += 1
    return newest, changed


def status(root, as_json=False):
    lm = last_mine(root)
    newest, changed = newest_surface(root)
    stale = changed > 0
    mined = datetime.date.fromtimestamp(lm).isoformat() if lm else "—"
    if as_json:
        return {"stale": stale, "changed_since_mine": changed, "last_mine": mined}
    if stale:
        return (f"mempalace: ⚠ {changed} surface file(s) changed since last mine ({mined}) "
                f"— re-mine due (index is behind the front-link)")
    return f"mempalace: fresh ✅ (mined {mined})"


def record(root):
    mk = _marker(root)
    with open(mk, "w") as fh:
        fh.write(str(datetime.datetime.now().timestamp()))
    print(f"mempalace-currency: recorded mine @ {datetime.date.today().isoformat()}")


# The CLI prints this (and still exits 0) when another process holds the palace.
# Treating it as success is what let a fully-blocked run report `fresh ✅`.
LOCK_MARKER = "is held by PID"


def _run_step(argv, timeout):
    """Run one mempalace step. Return (ok, blocked). Echoes output as it would have."""
    proc = subprocess.run(argv, timeout=timeout, check=False,
                          capture_output=True, text=True)
    out = (proc.stdout or "") + (proc.stderr or "")
    if out.strip():
        print(out.rstrip())
    blocked = LOCK_MARKER in out
    return (proc.returncode == 0 and not blocked), blocked


def remine(root, wait=False):
    """Sync + mine, recording freshness ONLY on evidence that work actually happened.

    The freshness marker gates the memory-stasis loop and the ceremony's Phase-4d
    exit criteria. Recording it after a lock-blocked run is self-concealing: the
    next run also reads "fresh" and skips the work, so the index silently falls
    behind the front-link while every dashboard reads green. A tool that reports
    green on blocked work is worse than one that fails, because it removes the
    signal that would have triggered a retry.
    """
    palace = os.path.join(root, ".mempalace", "palace")
    base = ["mempalace", "--palace", palace]
    print("mempalace re-mine: sync (prune) + mine surface ...")
    attempts = 12 if wait else 1
    try:
        for attempt in range(attempts):
            blocked_any = False
            ok, blocked = _run_step(base + ["sync", "--root", root, "--apply"], 600)
            blocked_any |= blocked
            for rel in SURFACE:
                d = os.path.join(root, rel)
                if os.path.isdir(d):
                    _ok, _blocked = _run_step(base + ["mine", d], 900)
                    ok &= _ok
                    blocked_any |= _blocked
            if not blocked_any:
                break
            if attempt < attempts - 1:
                print(f"mempalace re-mine: palace busy — retrying in 30s "
                      f"({attempt + 1}/{attempts - 1})")
                time.sleep(30)
        if blocked_any:
            print("mempalace re-mine: ABORTED — palace was held for the whole run; "
                  "freshness NOT recorded (previous mine date left intact). "
                  "Stop the holder or re-run with --wait.")
            return 1
        if not ok:
            print("mempalace re-mine: ABORTED — a step failed; freshness NOT recorded.")
            return 1
        record(root)
        print(status(root))
        return 0
    except FileNotFoundError:
        print("mempalace CLI not found — re-mine skipped and freshness NOT recorded "
              "(run on a host with the CLI)")
        return 1
    except Exception as e:  # noqa: BLE001 — best-effort; never crash the loop
        print(f"mempalace re-mine error: {e} — freshness NOT recorded")
        return 1


def main():
    root = _root()
    if "--record" in sys.argv:
        record(root)
    elif "--remine" in sys.argv:
        return remine(root, wait="--wait" in sys.argv)
    else:  # --status (default)
        print(json.dumps(status(root, True)) if "--json" in sys.argv else status(root))


if __name__ == "__main__":
    sys.exit(main() or 0)
