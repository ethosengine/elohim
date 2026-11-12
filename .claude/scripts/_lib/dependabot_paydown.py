"""Offline security ratchet sketch; no ledger writes or vulnerability discovery.

Retire when native epr flow sense owns polling, reconciliation and paydown
judgement. The current ledger bounds knowledge; absence is never a live audit.
"""
import json
from pathlib import Path, PurePosixPath
import re
import shlex
import subprocess
import tomllib

from _lib.dependabot_harvest import clusters

DEPENDENCY_FILES = {"Cargo.toml", "Cargo.lock", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"}
ROOT = Path(__file__).resolve().parents[3]


def read_ledger(path):
    rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    return [row for row in rows if row.get("status", "open") == "open"]


def remediation(group):
    package, version = group["package"], group["first_patched"]
    if not version:
        return "no patched version; inspect upstream mitigation"
    # GitHub's Dependabot API names the Rust ecosystem "rust"; accept "cargo" too.
    if group["ecosystem"].lower() in ("rust", "cargo"):
        directories = sorted({str(PurePosixPath(p).parent) for p in group["manifests"]})
        command = f"cargo update -p {shlex.quote(package)} --precise {shlex.quote(version)}"
        return " ; ".join(f"(cd {shlex.quote(d)} && {command})" for d in directories)
    return (f'root package.json pnpm.overrides[{json.dumps(package)}]={json.dumps(version)}, '
            f'mirror overrides in pnpm-workspace.yaml; pnpm install --lockfile-only '
            f'(all affected workspaces: {", ".join(group["manifests"])})')


def parse_lock(path, content):
    """Read actual package resolutions, never importer specifiers or snapshots."""
    if PurePosixPath(path).name == "Cargo.lock":
        document = tomllib.loads(content)
        if "package" not in document or not isinstance(document["package"], list):
            raise ValueError(f"invalid Cargo lockfile: {path}")
        records = [(p["name"], p["version"]) for p in document["package"]]
    else:
        import yaml
        document = yaml.load(content, Loader=getattr(yaml, "CSafeLoader", yaml.SafeLoader))
        if not isinstance(document, dict) or "lockfileVersion" not in document:
            raise ValueError(f"invalid pnpm lockfile: {path}")
        records = []
        for key in (document.get("packages") or {}):
            # v9 name@version, v6 /name@version, v5 /name/version;
            # scoped names and peer/patch suffixes are preserved until split.
            key = key.lstrip("/")
            match = re.fullmatch(r"(@[^/]+/[^@/]+|[^@/]+)(?:@|/)(\d[^(/_]*)(?:[(/_].*)?", key)
            if not match:
                raise ValueError(f"unmeasured pnpm package key: {key}")
            records.append(match.groups())
    result = {}
    for name, version in records:
        result.setdefault(name, set()).add(version)
    return result


def version_states(requests):
    """Batch the installed node-semver implementation; no install/network calls.

    Include prereleases in advisory ranges conservatively. A patch prerelease
    still sorts below its final patch release. Invalid input is unmeasured.
    """
    if not requests:
        return []
    source = r'''
const semver = require('semver');
const fs = require('node:fs');
const rows = JSON.parse(fs.readFileSync(0, 'utf8'));
const result = rows.map(({versions, patched, range}) => {
  if (!semver.valid(patched) || !range || semver.validRange(range.replaceAll(',', ' ')) === null)
    throw new Error('invalid patched version or advisory range');
  if (versions.some(v => !semver.valid(v))) throw new Error('invalid locked version');
  return {
    resolved: versions.every(v => semver.gte(v, patched)),
    vulnerable: versions.filter(v => semver.satisfies(v, range.replaceAll(',', ' '), {includePrerelease: true}))
  };
});
process.stdout.write(JSON.stringify(result));
'''
    proc = subprocess.run(["node", "-e", source], cwd=ROOT, input=json.dumps(requests),
                          text=True, capture_output=True, timeout=10)
    if proc.returncode:
        raise ValueError("semver evaluation unavailable or invalid version/range (requires installed node-semver)")
    return json.loads(proc.stdout)


def git(root, *args):
    proc = subprocess.run(["git", *args], cwd=root, text=True, capture_output=True, timeout=10)
    if proc.returncode:
        raise ValueError("cannot read committed range: " + " ".join(proc.stderr.split())[:180])
    return proc.stdout


class Tree:
    def __init__(self, root, revision):
        self.root, self.revision = root, revision
        self.paths = set(git(root, "ls-tree", "-r", "--name-only", "-z", revision).strip("\0").split("\0"))
        self.cache = {}

    def lock_for(self, manifest):
        path = PurePosixPath(manifest)
        name = "Cargo.lock" if path.name in {"Cargo.toml", "Cargo.lock"} else "pnpm-lock.yaml"
        if path.name == name:
            return manifest if manifest in self.paths else None
        for directory in (path.parent, *path.parent.parents):
            candidate = str(directory / name)
            if candidate in self.paths:
                return candidate
        return None

    def versions(self, manifest):
        lock = self.lock_for(manifest)
        if lock is None:
            # A removed entire dependency scope has no remaining resolutions.
            directory = PurePosixPath(manifest).parent
            names = {"Cargo.toml", "Cargo.lock"} if PurePosixPath(manifest).name.startswith("Cargo.") else DEPENDENCY_FILES - {"Cargo.toml", "Cargo.lock"}
            if not any(str(directory / name) in self.paths for name in names):
                return {}
            raise ValueError(f"no committed lockfile for {manifest}")
        if lock not in self.cache:
            self.cache[lock] = parse_lock(lock, git(self.root, "show", f"{self.revision}:{lock}"))
        return self.cache[lock]


def same_scope(tree, changed, manifest):
    cargo = {"Cargo.toml", "Cargo.lock"}
    if (PurePosixPath(changed).name in cargo) != (PurePosixPath(manifest).name in cargo):
        return False
    if PurePosixPath(changed).parent == PurePosixPath(manifest).parent:
        return True
    lock = tree.lock_for(changed)
    return lock is not None and lock == tree.lock_for(manifest)


def check(root, base, head, entries):
    # Resolve refs to object IDs before reading; a moving ref cannot mix trees.
    base = git(root, "rev-parse", "--verify", f"{base}^{{commit}}").strip()
    head = git(root, "rev-parse", "--verify", f"{head}^{{commit}}").strip()
    base = git(root, "merge-base", base, head).strip()
    changed = [p for p in git(root, "diff", "--no-renames", "--name-only", "-z", base, head, "--").split("\0")
               if PurePosixPath(p).name in DEPENDENCY_FILES]
    if not changed:
        return []
    before, after = Tree(root, base), Tree(root, head)
    groups = {}
    for entry in entries:
        manifest = entry["manifest_path"]
        if any(same_scope(t, p, manifest) for p in changed for t in (before, after)):
            groups.setdefault(manifest, []).append(entry)
    # Also report touched scopes for which the ledger has no applicable findings.
    for path in changed:
        if not any(same_scope(t, path, m) for m in groups for t in (before, after)):
            groups[path] = []
    plans, requests = [], []
    for manifest, rows in sorted(groups.items()):
        patchable = [r for r in rows if r.get("first_patched")]
        plan = {"manifest": manifest, "rows": patchable, "pairs": []}
        plans.append(plan)
        if not patchable:
            continue
        try:
            old, new = before.versions(manifest), after.versions(manifest)
            for row in patchable:
                plan["pairs"].append((len(requests), len(requests) + 1))
                for versions in (old, new):
                    requests.append({"versions": sorted(versions.get(row["package"], [])),
                                     "patched": row["first_patched"], "range": row["vulnerable_range"]})
        except (OSError, ValueError, KeyError) as exc:
            plan["error"] = str(exc)
    states = version_states(requests)
    results = []
    for plan in plans:
        rows = plan["rows"]
        command = remediation(clusters(rows)[0]) if rows else "dependabot-harvest.py --clusters"
        introduced = paid = debt = 0
        for a, b in plan["pairs"]:
            old, new = states[a], states[b]
            # Alert identity is advisory/package/manifest, not each version.
            # Moving within inherited debt is not a new alert; it still cannot
            # count as paydown until every resolved copy reaches the patch.
            introduced += bool(new["vulnerable"]) and not old["vulnerable"]
            debt += not old["resolved"]
            paid += not old["resolved"] and new["resolved"]
        reason = plan.get("error")
        if reason:
            reason = "unmeasured: " + reason
        elif introduced:
            reason = f"{introduced} new vulnerable resolution(s)"
        elif debt and not paid:
            reason = f"{debt} patchable alert(s), no paydown"
        ok = reason is None
        reason = reason or (f"resolved {paid} patchable alert(s)" if paid else "no outstanding patchable debt at base")
        # The ledger is a snapshot from before the push, so on an ok line its top cluster may be
        # the one just paid; name the remediation only where a push is refused.
        suffix = "" if ok else f" → {command}"
        results.append((ok, f'{"ok" if ok else "REFUSED"} {plan["manifest"]}: {reason}{suffix}'))
    return results
