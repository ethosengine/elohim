"""Pure Dependabot normalization, live-set reconciliation and remediation groups.

The shell owns I/O. A closed finding disappears from the live ledger; a later
reappearance starts a new episode, leaving the previous closure in history.
"""
import hashlib
import json
import re

SEVERITIES = ("critical", "high", "medium", "low")
SEVERITY_WEIGHTS = {"critical": 8, "high": 4, "medium": 2, "low": 1}


def normalize(alert):
    """Extract the affected dependency's vulnerability, not an advisory sibling."""
    dependency = alert["dependency"]
    advisory = alert["security_advisory"]
    vulnerability = alert["security_vulnerability"]
    package = dependency["package"]
    patched = vulnerability.get("first_patched_version")
    row = {
        "number": alert["number"],
        "severity": vulnerability["severity"],
        "ecosystem": package["ecosystem"],
        "package": package["name"],
        "manifest_path": dependency["manifest_path"],
        "ghsa_id": advisory["ghsa_id"],
        "cve_id": advisory.get("cve_id"),
        "summary": advisory["summary"],
        "vulnerable_range": vulnerability["vulnerable_version_range"],
        "first_patched": patched["identifier"] if patched else None,
        "created_at": alert["created_at"],
        "html_url": alert["html_url"],
    }
    if type(row["number"]) is not int or row["number"] <= 0:
        raise ValueError("invalid alert number")
    for key, value in row.items():
        if key in ("number", "cve_id", "first_patched"):
            continue
        if not isinstance(value, str) or not value:
            raise ValueError(f"invalid alert {key}")
    for key in ("cve_id", "first_patched"):
        if row[key] is not None and (not isinstance(row[key], str) or not row[key]):
            raise ValueError(f"invalid alert {key}")
    if row["severity"] not in SEVERITIES:
        raise ValueError("unknown alert severity")
    return row


def fingerprint(n):
    """12-hex identity independent of alert numbering and changing metadata."""
    identity = [n[k] for k in ("ghsa_id", "ecosystem", "package", "manifest_path")]
    return hashlib.sha256(json.dumps(identity, separators=(",", ":")).encode()).hexdigest()[:12]


def reconcile(ledger_entries, open_alerts, now):
    """Return (new, still_open, closed), without mutating either input.

    open_alerts are raw API alerts. A complete open snapshot is required. Duplicate
    fingerprints count once per poll; first_seen/seen describe this live episode.
    """
    previous = {e["fp"]: e for e in ledger_entries}
    active = {}
    for alert in open_alerts:
        n = normalize(alert)
        active[fingerprint(n)] = n
    new, still_open = [], []
    for fp, n in sorted(active.items()):
        old = previous.get(fp)
        row = {**(old or {}), **n, "fp": fp, "status": "open",
               "first_seen": old["first_seen"] if old else now,
               "last_seen": now, "seen": old.get("seen", 1) + 1 if old else 1}
        row.pop("closed_at", None)
        (still_open if old else new).append(row)
    closed = [{**e, "status": "closed", "closed_at": now}
              for fp, e in sorted(previous.items()) if fp not in active]
    return new, still_open, closed


def _version_key(version):
    """Numeric release ordering (1.10 > 1.9), prereleases before final releases.

    Natural token ordering is a display aid for non-semver ecosystem suffixes;
    this patch floor is not a dependency solver or a compatible upgrade promise.
    """
    release, _, suffix = version.lstrip("v").split("+", 1)[0].partition("-")
    match = re.match(r"(\d+(?:\.\d+)*)(.*)", release)
    numbers = tuple(int(n) for n in match[1].split(".")) if match else ()
    while numbers and numbers[-1] == 0:
        numbers = numbers[:-1]
    suffix = ((match[2] if match else release) + suffix).lower()
    tokens = tuple((0, int(t)) if t.isdigit() else (1, t)
                   for t in re.findall(r"\d+|[^\d.]+", suffix))
    return numbers, not bool(suffix), tokens


def clusters(entries):
    """Group by ecosystem/package; manifests are unique, counts are findings."""
    groups = {}
    for e in entries:
        key = e["ecosystem"], e["package"]
        group = groups.setdefault(key, {
            "ecosystem": key[0], "package": key[1], "count": 0,
            "severities": dict.fromkeys(SEVERITIES, 0), "manifests": set(),
            "first_patched": None, "unpatchable": False,
        })
        group["count"] += 1
        group["severities"][e["severity"]] += 1
        group["manifests"].add(e["manifest_path"])
        patched = e["first_patched"]
        if patched is None:
            group["unpatchable"] = True
        elif group["first_patched"] is None or _version_key(patched) > _version_key(group["first_patched"]):
            group["first_patched"] = patched
    return [{**g, "manifests": sorted(g["manifests"])} for g in sorted(
        groups.values(), key=lambda g: (-g["count"], g["ecosystem"], g["package"]))]


def pressure(entries):
    """Shared delivery weight: critical/high/medium/low cost 8/4/2/1."""
    counts = {s: sum(e.get("severity") == s for e in entries) for s in SEVERITIES}
    return {"open": len(entries), "critical": counts["critical"], "severities": counts,
            "weighted": sum(counts[s] * SEVERITY_WEIGHTS[s] for s in SEVERITIES)}


def summary_line(entries):
    counts = pressure(entries)
    unpatchable = sum(e["first_patched"] is None for e in entries)
    top = ", ".join(f'{g["package"]} {g["count"]}' for g in clusters(entries)[:3]) or "none"
    return (f'dependabot: {counts["open"]} open ({counts["critical"]} critical, '
            f'{counts["severities"]["high"]} high) · {unpatchable} unpatchable · '
            f'top: {top} → /deprecation-stasis')
