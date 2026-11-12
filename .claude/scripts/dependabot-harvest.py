#!/usr/bin/env python3
"""Poll Dependabot into a live findings ledger and append-only closure history.

Offline: --from-file PAGE.json --data-dir DIR. Hook failures are unmeasured,
never a zero reading. Retire this local sensor when native sense owns its poll,
live-set reconciliation and closure history.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _lib.dependabot_harvest import clusters, normalize, pressure, reconcile, summary_line
from _lib.findings_ledger import _flock

PROJECT = Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path(__file__).resolve().parents[2])
THROTTLE_SECONDS = 6 * 3600


def parse_alerts(text):
    """gh --paginate emits consecutive JSON arrays, not one JSON document.

    Accept a normal fixture array or --slurp arrays too. Validate the ENTIRE
    response before reconciling: a truncated later page cannot close findings.
    """
    decoder = json.JSONDecoder()
    alerts = []
    remaining = text.strip()
    if not remaining:
        raise ValueError("empty alert response")
    while remaining:
        page, end = decoder.raw_decode(remaining)
        if not isinstance(page, list):
            raise ValueError("expected an alert array")
        pages = page if page and all(isinstance(p, list) for p in page) else [page]
        for batch in pages:
            for alert in batch:
                if not isinstance(alert, dict) or alert.get("state") != "open":
                    raise ValueError("expected an open alert object")
                normalize(alert)
                alerts.append(alert)
        remaining = remaining[end:].lstrip()
    return alerts


def gh(*args, timeout=60):
    try:
        result = subprocess.run(["gh", *args], cwd=PROJECT, capture_output=True,
                                text=True, timeout=timeout)
    except FileNotFoundError as exc:
        raise RuntimeError("gh not found") from exc
    if result.returncode:
        reason = " ".join(result.stderr.split())[:200] or f"gh exited {result.returncode}"
        raise RuntimeError(reason)
    return result.stdout


def repository(override):
    repo = override or json.loads(gh("repo", "view", "--json", "nameWithOwner", timeout=10))["nameWithOwner"]
    if not isinstance(repo, str) or not re.fullmatch(r"[\w.-]+/[\w.-]+", repo):
        raise ValueError("expected --repo owner/repo")
    return repo


def read_json(path, default):
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else default


def read_ledger(path):
    # Unlike a best-effort advisory read, reconciliation must not silently drop
    # corrupt history. Refuse the poll and leave its previous evidence intact.
    if not path.exists():
        return []
    rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    if any(not isinstance(row, dict) or "fp" not in row for row in rows):
        raise ValueError(f"invalid ledger: {path.name}")
    return rows


def atomic_write(path, text):
    fd, name = tempfile.mkstemp(prefix=f".{path.name}-", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(text)
            fh.flush()
            os.fsync(fh.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def cached(cursor, ledger_path, repo):
    if not ledger_path.exists() or not cursor.get("last_poll"):
        return False
    if repo and repo != cursor.get("repo"):
        return False
    last = datetime.fromisoformat(cursor["last_poll"].replace("Z", "+00:00"))
    age = (datetime.now(timezone.utc) - last).total_seconds()
    return 0 <= age < THROTTLE_SECONDS


def harvest(args):
    data = Path(args.data_dir)
    data.mkdir(parents=True, exist_ok=True)
    live_path = data / "dependabot-findings.jsonl"
    closed_path = data / "dependabot-closed.jsonl"
    cursor_path = data / "dependabot-cursor.json"
    # One stable lock covers cache check, network and all persistence, preventing
    # concurrent SessionStarts from overwriting a newer snapshot or double-filing.
    with (data / "dependabot-findings.jsonl.lock").open("a") as lock:
        if not _flock(lock):
            raise RuntimeError("another harvest is running")
        cursor = read_json(cursor_path, {})
        if args.hook and not args.from_file and cached(cursor, live_path, args.repo):
            return read_ledger(live_path), cursor
        repo = args.repo or cursor.get("repo")
        if args.from_file:
            alerts = parse_alerts(Path(args.from_file).read_text(encoding="utf-8"))
        else:
            repo = repository(args.repo)
            if cursor.get("repo") and repo != cursor["repo"]:
                raise ValueError("data directory belongs to another repository; use --data-dir")
            alerts = parse_alerts(gh("api", "-X", "GET",
                                    f"repos/{repo}/dependabot/alerts?state=open&per_page=100",
                                    "--paginate"))
        if cursor.get("repo") and repo != cursor["repo"]:
            raise ValueError("data directory belongs to another repository; use --data-dir")
        now = datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z")
        new, still_open, closed = reconcile(read_ledger(live_path), alerts, now)
        live = sorted(new + still_open, key=lambda row: row["fp"])
        # History first. If live replacement fails, retry must not append the same
        # episode twice. first_seen + last_seen disambiguate later recurrences.
        history = read_ledger(closed_path)
        episode = lambda row: (row["fp"], row["first_seen"], row["last_seen"])
        recorded = {episode(row) for row in history}
        additions = [row for row in closed if episode(row) not in recorded]
        if additions:
            with closed_path.open("a", encoding="utf-8") as fh:
                for row in additions:
                    fh.write(json.dumps(row, ensure_ascii=False) + "\n")
                fh.flush()
                os.fsync(fh.fileno())
        atomic_write(live_path, "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in live))
        cursor = {"last_poll": now, "repo": repo, "counts": pressure(live),
                  "opened": len(new), "closed": len(closed)}
        atomic_write(cursor_path, json.dumps(cursor, indent=2) + "\n")
        return live, cursor


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--hook", action="store_true")
    mode.add_argument("--clusters", action="store_true")
    mode.add_argument("--json", action="store_true")
    parser.add_argument("--repo", help="owner/repo (otherwise gh repo view)")
    parser.add_argument("--from-file", help="complete open-alert snapshot, bypassing network/throttle")
    parser.add_argument("--data-dir", default=str(PROJECT / ".claude/data"))
    args = parser.parse_args(argv)
    try:
        entries, cursor = harvest(args)
        if args.json:
            print(json.dumps({"entries": entries, "clusters": clusters(entries), "cursor": cursor}, indent=2))
        elif args.clusters:
            print("ecosystem\tpackage\topen\tcritical\thigh\tmedium\tlow\tfirst patched\tunpatchable\tmanifests")
            for group in clusters(entries):
                print("\t".join(map(str, [group["ecosystem"], group["package"], group["count"],
                    *group["severities"].values(), group["first_patched"] or "—",
                    "yes" if group["unpatchable"] else "no", ", ".join(group["manifests"])])))
        else:
            print(summary_line(entries))
        return 0
    except Exception as exc:  # hook transport must never fail SessionStart
        reason = " ".join(str(exc).split())[:240] or type(exc).__name__
        print(f"dependabot: unmeasured ({reason})")
        return 0 if args.hook else 1


if __name__ == "__main__":
    sys.exit(main())
