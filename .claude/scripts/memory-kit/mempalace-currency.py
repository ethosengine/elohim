#!/usr/bin/env python3
"""mempalace-currency — the deterministic staleness tripwire for the MemPalace semantic index.

MemPalace embeddings are FROZEN at mine-time (they do not auto-update when a source file changes), so the index
silently drifts behind the cleaned surface. This is the 7th store's tripwire: it measures whether the index is
current by comparing the last-mine timestamp against the newest mtime on the mined surface — pure stdlib, no
MemPalace CLI dependency (so it runs anywhere, including subagent contexts). The actual re-mine (--remine) does
need the CLI; the staleness CHECK never does.

  --status [--json]   one-line currency for the SessionStart headline (fresh vs N changes since last mine)
  --record            stamp .mempalace/.last-mine = now (call after a successful mine)
  --remine            sync (prune deleted) + mine the surface + record  (needs the `mempalace` CLI)

The mined surface = the durable, cleaned multi-destination surface the front-link recalls from (canonical
architecture seeds + curated history + working memory + canonical stories). NEVER the transient pile / raw
code / junk drawer — index only the clean surface (the no-dumping-grounds rule applied to the index).
"""
import sys, os, json, subprocess, datetime

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


def remine(root):
    palace = os.path.join(root, ".mempalace", "palace")
    base = ["mempalace", "--palace", palace]
    print("mempalace re-mine: sync (prune) + mine surface ...")
    try:
        subprocess.run(base + ["sync", "--root", root, "--apply"], timeout=600, check=False)
        for rel in SURFACE:
            d = os.path.join(root, rel)
            if os.path.isdir(d):
                subprocess.run(base + ["mine", d], timeout=900, check=False)
        record(root)
        print(status(root))
    except FileNotFoundError:
        print("mempalace CLI not found — staleness recorded but re-mine skipped (run on a host with the CLI)")
    except Exception as e:  # noqa: BLE001 — best-effort; never crash the loop
        print(f"mempalace re-mine error: {e}")


def main():
    root = _root()
    if "--record" in sys.argv:
        record(root)
    elif "--remine" in sys.argv:
        remine(root)
    else:  # --status (default)
        print(json.dumps(status(root, True)) if "--json" in sys.argv else status(root))


if __name__ == "__main__":
    main()
