#!/usr/bin/env python3
"""memkit-retention — comet-shape the .claude/memory-kit/ report tier so it can't grow forever.

These are PROCESS artifacts (ceremony snapshots, cleanup proposals, audits) — unlike plans/specs they do
NOT decompose into the documentation. Their only durable value is the TRAJECTORY: the dated sequence of what
happened when. So they follow the comet shape (project_memory_lifecycle_comet_shape), not decompose-to-zero:

  HEAD  (<= HEAD_DAYS)            dated dirs kept FULL — the live working window
  TAIL  (HEAD_DAYS..TAIL_DAYS)    dated dirs COMPACTED to a one-file _digest.md (bulk dropped)
  CORE  (> TAIL_DAYS)             dated dir DROPPED, leaving a one-line entry in TRAJECTORY.md (memorialized)

LOOSE top-level reports get filed into a dated dir (by mtime) so everything is on the comet spine.
LIVE generated state (*.json, gap-items/) and the dir guide (CLAUDE.md) + the spine (TRAJECTORY.md) are kept.

TRAJECTORY.md is the comet's permanent spine: one dated line per cycle, so the arc stays inferable forever
even after the content is gone. Bulk content is recoverable from git history; the trajectory never is lost.

Usage:
  memkit-retention.py            # dry-run: show what comet-shaping would do
  memkit-retention.py --apply    # apply it
  memkit-retention.py --status   # one-line currency for the SessionStart headline
  memkit-retention.py --status --json
"""
import sys, os, json, datetime, re, shutil

HEAD_DAYS = 30          # full-retention working window
TAIL_DAYS = 90          # compacted-to-digest window; older than this is memorialized + dropped
SIZE_CAP_MB = 8         # soft cap for the whole tier; over this the headline flags overflow

KEEP_LOOSE = {"CLAUDE.md", "TRAJECTORY.md", "next-ceremony-inputs.md"}  # guide + spine + live handoff
KEEP_DIRS = {"gap-items", "balance-sheets", "horizon-scans"}            # live budget + periodic series (own cadence)
DATED = re.compile(r"^(\d{4})-(\d{2})-(\d{2})$")


def _root():
    p = os.path.abspath(__file__)
    while p != "/":
        if os.path.isdir(os.path.join(p, ".claude", "memory-kit")):
            return p
        p = os.path.dirname(p)
    return os.getcwd()


def _kit(root):
    return os.path.join(root, ".claude", "memory-kit")


def _today():
    return datetime.date.today()


def _dir_size(path):
    total = 0
    for dp, _dn, fn in os.walk(path):
        for f in fn:
            try:
                total += os.path.getsize(os.path.join(dp, f))
            except OSError:
                pass
    return total


def _first_line(path):
    try:
        with open(path, encoding="utf-8", errors="ignore") as fh:
            for ln in fh:
                ln = ln.strip().lstrip("#").strip()
                if ln:
                    return ln[:120]
    except OSError:
        pass
    return ""


def plan(kit):
    """Return the comet-shaping plan without mutating anything."""
    today = _today()
    actions = {"file_loose": [], "compact_tail": [], "memorialize_core": []}
    # loose top-level reports -> dated dir by mtime
    for name in sorted(os.listdir(kit)):
        full = os.path.join(kit, name)
        if os.path.isfile(full) and name.endswith(".md") and name not in KEEP_LOOSE:
            d = datetime.date.fromtimestamp(os.path.getmtime(full)).isoformat()
            actions["file_loose"].append((name, d))
    # dated dirs -> head/tail/core by age
    for name in sorted(os.listdir(kit)):
        m = DATED.match(name)
        full = os.path.join(kit, name)
        if not (m and os.path.isdir(full)):
            continue
        ddate = datetime.date(int(m.group(1)), int(m.group(2)), int(m.group(3)))
        age = (today - ddate).days
        has_digest = os.path.exists(os.path.join(full, "_digest.md"))
        if age > TAIL_DAYS:
            actions["memorialize_core"].append((name, age))
        elif age > HEAD_DAYS and not has_digest:
            actions["compact_tail"].append((name, age))
    return actions


def apply(kit, actions):
    log = []
    # 1. file loose reports into their dated dir
    for name, d in actions["file_loose"]:
        dest_dir = os.path.join(kit, d)
        os.makedirs(dest_dir, exist_ok=True)
        shutil.move(os.path.join(kit, name), os.path.join(dest_dir, name))
        log.append(f"filed {name} -> {d}/")
    # 2. compact tail dirs to a single _digest.md
    for name, _age in actions["compact_tail"]:
        full = os.path.join(kit, name)
        files = sorted(f for f in os.listdir(full) if os.path.isfile(os.path.join(full, f)))
        lines = [f"# {name} — digest (compacted by memkit-retention)\n",
                 f"> {len(files)} report(s) compacted to this one-line index; bodies in git history.\n"]
        for f in files:
            lines.append(f"- **{f}** — {_first_line(os.path.join(full, f))}")
        for f in files:
            os.remove(os.path.join(full, f))
        with open(os.path.join(full, "_digest.md"), "w", encoding="utf-8") as fh:
            fh.write("\n".join(lines) + "\n")
        log.append(f"compacted {name}/ ({len(files)} files -> _digest.md)")
    # 3. memorialize core dirs to TRAJECTORY.md, then drop them
    traj = os.path.join(kit, "TRAJECTORY.md")
    if actions["memorialize_core"]:
        existing = ""
        if os.path.exists(traj):
            with open(traj, encoding="utf-8") as fh:
                existing = fh.read()
        else:
            existing = ("# memory-kit trajectory — the comet spine\n\n"
                        "One line per retired cycle. Bodies are in git history; this is the permanent arc.\n\n")
        new_lines = []
        for name, _age in actions["memorialize_core"]:
            full = os.path.join(kit, name)
            files = sorted(f for f in os.listdir(full) if os.path.isfile(os.path.join(full, f)))
            topics = ", ".join(f.replace(".md", "").replace("-", " ")[:40] for f in files[:4])
            new_lines.append(f"- **{name}** — {len(files)} report(s): {topics}")
            shutil.rmtree(full)
            log.append(f"memorialized + dropped {name}/ ({len(files)} files)")
        with open(traj, "w", encoding="utf-8") as fh:
            fh.write(existing.rstrip() + "\n" + "\n".join(new_lines) + "\n")
    return log


def status(kit, as_json=False):
    size = _dir_size(kit)
    mb = size / (1024 * 1024)
    dated = sorted(n for n in os.listdir(kit) if DATED.match(n) and os.path.isdir(os.path.join(kit, n)))
    loose = [n for n in os.listdir(kit)
             if os.path.isfile(os.path.join(kit, n)) and n.endswith(".md") and n not in KEEP_LOOSE]
    over = mb > SIZE_CAP_MB or len(loose) > 0
    if as_json:
        return {"size_mb": round(mb, 1), "cap_mb": SIZE_CAP_MB, "dated_cycles": len(dated),
                "loose_unfiled": len(loose), "overflow": over}
    if over:
        why = []
        if mb > SIZE_CAP_MB:
            why.append(f"{mb:.1f}MB > {SIZE_CAP_MB}MB cap")
        if loose:
            why.append(f"{len(loose)} loose unfiled report(s)")
        return f"memkit: ⚠ comet-prune due ({'; '.join(why)})   [{len(dated)} cycles]"
    return f"memkit: {mb:.1f}MB / {len(dated)} cycles   (comet shape ✅)"


def main():
    root = _root()
    kit = _kit(root)
    if "--status" in sys.argv:
        print(json.dumps(status(kit, True)) if "--json" in sys.argv else status(kit))
        return
    actions = plan(kit)
    n = sum(len(v) for v in actions.values())
    if "--apply" in sys.argv:
        log = apply(kit, actions)
        print(f"memkit-retention applied: {len(log)} action(s)")
        for ln in log:
            print(f"  {ln}")
        print(status(kit))
    else:
        print(f"memkit-retention DRY-RUN — {n} action(s) (use --apply):")
        for k, items in actions.items():
            for it in items:
                print(f"  {k}: {it[0]}")
        print(status(kit))


if __name__ == "__main__":
    main()
