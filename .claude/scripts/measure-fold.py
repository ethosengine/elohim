#!/usr/bin/env python3
"""measure-fold — executor + fold cache for the middot Measure primitive (MVP slice: the clippy family).

  measure-fold.py --measure clippy-pedantic@1                       measure the changed crates vs merge-base
  measure-fold.py --measure clippy-pedantic@1 --dry-run             print derivation keys + cache hit/miss only
  measure-fold.py --measure clippy-pedantic@1 --crate steward/node  measure a named workspace (skip detection)
  measure-fold.py --measure clippy-pedantic@1 --diff-only           print only findings on lines changed base..head

A *measure* is a named, versioned observation procedure (registry: `.claude/epr-meta/measures.yaml`); applying
it to a subject under an environment memoizes as a *fold* keyed by the derivation triple
(subject-CID × measure@version × env). This script is the dev-tier instance of that shape: subject = a native
Rust workspace at a git tree hash, measure = `clippy-pedantic@1`, env = toolchain + lockfile + lint-config +
RUSTFLAGS. A cache hit is a free read of somebody else's compute; a miss spends the compute and publishes the
result so the next reader (human or agent) gets the hit.

**Advisory, always.** Findings NEVER make this script exit non-zero — the measure carries no teeth (teeth live
only in a lens contract half). A non-zero exit means the *measurement itself* failed (missing registry, cargo
compile error, missing CARGO_TARGET_DIR), and in that case NO fold is written: unmeasured must render
differently from measured-zero (honest absence).

**Known dev-tier approximation (documented, not hidden).** The subject component of the derivation key is
`git rev-parse <head>:<crate-path>` — the crate directory's tree hash. Source changes in a *path-dependency*
that lives outside the crate directory are therefore captured only indirectly, via the workspace's `Cargo.lock`
hash — and a path-dep edit that does not move the lock will not move the key, so a workspace-sibling edit can
produce a stale cache hit. Acceptable for slice 1 (the lifted version keys on the resolved source-set CID);
delete the fold cache directory if you suspect a stale hit.

Spec: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md (§3 Measure, §4 Fold, §8 MVP).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = next(p for p in Path(__file__).resolve().parents if (p / ".git").exists() or (p / ".claude").is_dir())
REGISTRY = ROOT / ".claude" / "epr-meta" / "measures.yaml"
FOLD_ROOT = ROOT / ".claude" / "data" / "measure-folds"
CARGO_POOL = ROOT / "genesis" / "agentic" / "bin" / "cargo-pool"

# Native Rust workspaces that are legitimate measure subjects. DNA/WASM workspaces
# (elohim/holochain/dna/**) are deliberately OUT of scope — they must stay plain cargo.
FIXED_WORKSPACES = ("doorway/doorway-service", "steward/node", "elohim/elohim-storage")
STORAGE_RUSTFLAGS = '--cfg getrandom_backend="custom"'  # hard repo rule: storage needs the custom getrandom backend
HUNK = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")


# ── shell / git ───────────────────────────────────────────────────────────────
def run(cmd, cwd=None, env=None):
    """Run a command with no shell; return (returncode, stdout, stderr)."""
    p = subprocess.run(cmd, cwd=str(cwd or ROOT), env=env, capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


def git(*args, ok=False):
    rc, out, err = run(["git", *args])
    if rc != 0 and not ok:
        raise RuntimeError(f"git {' '.join(args)} failed: {err.strip()}")
    return out.strip() if rc == 0 else ""


def default_base():
    for ref in ("dev", "origin/dev"):
        mb = git("merge-base", "HEAD", ref, ok=True)
        if mb:
            return mb
    return git("rev-parse", "HEAD")


# ── registry ──────────────────────────────────────────────────────────────────
def load_registry(path=None):
    path = Path(path) if path else REGISTRY  # resolved at call time so REGISTRY stays overridable
    if not path.exists():
        raise SystemExit(
            f"measure registry not found: {path}\n"
            "  The measure definition is the authority — this executor refuses to invent one.\n"
            "  Author the row (measures-version: 1, measures: [- id: clippy-pedantic, version: 1, ...]) first."
        )
    try:
        import yaml  # noqa: PLC0415 — optional dep, resolved at call time
    except ImportError:
        raise SystemExit("PyYAML is required to read the measure registry (pip install pyyaml)") from None
    data = yaml.safe_load(path.read_text()) or {}
    if not isinstance(data, dict) or "measures" not in data:
        raise SystemExit(f"malformed measure registry (no `measures:` key): {path}")
    return data


def resolve_measure(registry, ref):
    """`<id>@<version>` (version optional when the registry holds exactly one)."""
    mid, _, ver = ref.partition("@")
    cands = [m for m in registry.get("measures") or [] if m.get("id") == mid]
    if ver:
        cands = [m for m in cands if str(m.get("version")) == ver]
    if not cands:
        known = ", ".join(f"{m.get('id')}@{m.get('version')}" for m in registry.get("measures") or []) or "(none)"
        raise SystemExit(f"measure {ref!r} not in registry. Known: {known}")
    if len(cands) > 1:
        raise SystemExit(f"measure {ref!r} is ambiguous — pin a version (<id>@<version>)")
    m = dict(cands[0])
    m["ref"] = f"{m['id']}@{m['version']}"
    return m


# ── subjects (workspaces) ─────────────────────────────────────────────────────
def known_workspaces():
    ws = [w for w in FIXED_WORKSPACES if (ROOT / w / "Cargo.toml").exists()]
    ws += sorted(f"crates/{d.name}" for d in (ROOT / "crates").glob("*") if (d / "Cargo.toml").exists())
    return ws


def changed_workspaces(base, head):
    files = git("diff", "--name-only", f"{base}..{head}").splitlines()
    ws, hit = known_workspaces(), []
    for f in files:
        match = max((w for w in ws if f.startswith(w + "/")), key=len, default=None)
        if match and match not in hit:
            hit.append(match)
    return hit


def rustflags_for(crate):
    return STORAGE_RUSTFLAGS if crate == "elohim/elohim-storage" else ""


def sha256_file(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def crate_env(crate):
    _, rustc, _ = run(["rustc", "-V"])
    _, clippy, _ = run(["cargo", "clippy", "-V"])
    return {
        "rustc": rustc.strip(),
        "clippy": clippy.strip(),
        "cargo_lock": sha256_file(ROOT / crate / "Cargo.lock"),
        "clippy_toml": sha256_file(ROOT / crate / "clippy.toml"),
        "rustflags": rustflags_for(crate),
    }


def subject_of(crate, head):
    return {"crate": crate, "tree": git("rev-parse", f"{head}:{crate}")}


# ── derivation key + fold cache (measure-generic) ─────────────────────────────
def canonical(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"))


def derivation_key(measure_ref, subject, env):
    return hashlib.sha256(canonical({"measure": measure_ref, "subject": subject, "env": env}).encode()).hexdigest()


def fold_path(measure_id, key):
    return FOLD_ROOT / measure_id / f"{key[:16]}.json"


def load_fold(measure_id, key):
    p = fold_path(measure_id, key)
    if not p.exists():
        return None
    try:
        fold = json.loads(p.read_text())
    except (OSError, json.JSONDecodeError):
        return None
    return fold if fold.get("derivationKey") == key else None


def write_fold(measure_id, fold):
    p = fold_path(measure_id, fold["derivationKey"])
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(fold, indent=2, sort_keys=True) + "\n")
    return p


def make_fold(measure_ref, subject, env, key, findings):
    by_code = {}
    for f in findings:
        by_code[f["code"]] = by_code.get(f["code"], 0) + 1
    return {
        "schema": "measure-fold@1",
        "measure": measure_ref,
        "subject": subject,
        "env": env,
        "derivationKey": key,
        "computedBy": git("config", "user.name", ok=True) or "unknown",
        "computedAt": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "evidence": "claimed",  # one agent computed it; a matching independent recompute raises it to witnessed
        "findings": findings,
        "counts": {"total": len(findings), "byCode": dict(sorted(by_code.items()))},
    }


# ── the clippy family: procedure + canonicalization ───────────────────────────
def canonicalize_clippy(stdout):
    """cargo --message-format=json stream -> the canonical finding set (deterministic order)."""
    findings = []
    for line in stdout.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-message":
            continue  # drop compiler-artifact / build-finished / build-script-executed
        d = msg.get("message") or {}
        level = d.get("level") or ""
        code = ((d.get("code") or {}) or {}).get("code") or ""
        if not (code.startswith("clippy::") or level == "warning"):
            continue
        spans = d.get("spans") or []
        span = next((s for s in spans if s.get("is_primary")), spans[0] if spans else {})
        rendered = (d.get("rendered") or "").splitlines()[:10]
        findings.append(
            {
                "file": span.get("file_name") or "",
                "line": span.get("line_start") or 0,
                "endLine": span.get("line_end") or span.get("line_start") or 0,
                "code": code or f"rustc::{level}",
                "level": level,
                "message": d.get("message") or "",
                "rendered": "\n".join(rendered),
            }
        )
    findings.sort(key=lambda f: (f["file"], f["line"], f["code"]))
    return findings


def target_dir_for(crate):
    """CARGO_TARGET_DIR from the pool. A native cargo run without it is DENIED by the repo's disk-guard hook."""
    if not CARGO_POOL.exists():
        raise SystemExit(f"cargo-pool not found at {CARGO_POOL}; cannot resolve CARGO_TARGET_DIR — refusing to run cargo")
    rc, out, err = run([str(CARGO_POOL), "key"], cwd=ROOT / crate)
    slot = next((ln.split("=", 1)[1].strip() for ln in out.splitlines() if ln.startswith("slot_dev=")), "")
    if rc != 0 or not slot:
        raise SystemExit(
            f"`cargo-pool key` failed for {crate} (rc={rc}): {err.strip() or out.strip()}\n"
            "  Native cargo MUST run with CARGO_TARGET_DIR set to a pool slot (the PreToolUse disk-guard hook\n"
            "  denies native-workspace cargo without it). Fix cargo-pool, or export the slot and retry."
        )
    return slot


def run_clippy(crate, measure):
    allows = measure.get("allows") or []
    cmd = ["cargo", "clippy", "--message-format=json", "--", "-W", "clippy::pedantic"]
    for a in allows:
        cmd += ["-A", f"clippy::{a}"]
    env = dict(os.environ, RUSTFLAGS=rustflags_for(crate), CARGO_TARGET_DIR=target_dir_for(crate))
    rc, out, err = run(cmd, cwd=ROOT / crate, env=env)
    if rc != 0:
        raise SystemExit(
            f"\n✗ measurement FAILED for {crate}: cargo clippy exited {rc}.\n"
            f"  No fold written — an unmeasured subject must not render as measured-zero.\n"
            f"  --- cargo stderr (tail) ---\n{chr(10).join(err.strip().splitlines()[-25:])}"
        )
    return canonicalize_clippy(out)


PROCEDURES = {"clippy": run_clippy}


# ── --diff-only: changed-line filter (printing only; the fold stays full-crate) ─
def changed_lines(base, head, crate):
    diff = git("diff", "-U0", f"{base}..{head}", "--", crate)
    lines, cur = {}, None
    for ln in diff.splitlines():
        if ln.startswith("+++ b/"):
            cur = ln[6:]
            lines.setdefault(cur, set())
        elif ln.startswith("@@") and cur:
            m = HUNK.match(ln)
            if m:
                start, count = int(m.group(1)), int(m.group(2) or 1)
                lines[cur].update(range(start, start + count))
    return lines


def on_changed_lines(findings, crate, touched):
    keep = []
    for f in findings:
        repo_path = f"{crate}/{f['file']}"
        hits = touched.get(repo_path)
        if hits and any(n in hits for n in range(f["line"], (f["endLine"] or f["line"]) + 1)):
            keep.append(f)
    return keep


# ── reporting ─────────────────────────────────────────────────────────────────
def print_findings(findings, tier):
    for f in findings:
        print(f"    {f['file']}:{f['line']}  {f['code']}  [{tier}]")
        print(f"      {f['message']}")


def main(argv=None):
    ap = argparse.ArgumentParser(description="Compute or reuse a measure fold over the changed Rust workspaces.")
    ap.add_argument("--measure", required=True, help="measure ref, e.g. clippy-pedantic@1")
    ap.add_argument("--base", help="base ref (default: merge-base HEAD dev)")
    ap.add_argument("--head", default="HEAD")
    ap.add_argument("--crate", action="append", default=[], help="workspace path (repeatable); overrides detection")
    ap.add_argument("--diff-only", action="store_true", help="print only findings on lines changed base..head")
    ap.add_argument("--dry-run", action="store_true", help="print derivation key + hit/miss; run no cargo")
    args = ap.parse_args(argv)

    measure = resolve_measure(load_registry(), args.measure)
    procedure = PROCEDURES.get(measure.get("family"))
    if procedure is None and not args.dry_run:
        raise SystemExit(f"no executor implemented for measure family {measure.get('family')!r} (v1 ships `clippy`)")

    base = args.base or default_base()
    head = git("rev-parse", args.head)
    known = known_workspaces()
    crates = args.crate or changed_workspaces(base, head)
    unknown = [c for c in crates if c not in known]
    if unknown:
        raise SystemExit(f"not a native Rust workspace subject: {', '.join(unknown)}\n  known: {', '.join(known)}")

    print(f"measure: {measure['ref']}  (family {measure.get('family')}, authority {measure.get('default-authority')})")
    print(f"base:    {base[:12]}  head: {head[:12]}")
    if not crates:
        print("subjects: none — no changed native Rust workspace in this range (nothing to measure)")
        return 0

    tier = "advisory" if measure.get("default-authority", "observation") == "observation" else measure["ref"]
    for crate in crates:
        subject = subject_of(crate, head)
        env = crate_env(crate)
        key = derivation_key(measure["ref"], subject, env)
        fold = load_fold(measure["id"], key)
        print(f"\n── {crate}  (tree {subject['tree'][:12]})")
        print(f"   key:   {key}")
        print(f"   cache: {'hit' if fold else 'miss'}  ({fold_path(measure['id'], key).relative_to(ROOT)})")
        if args.dry_run:
            continue
        if fold is None:
            fold = make_fold(measure["ref"], subject, env, key, procedure(crate, measure))
            write_fold(measure["id"], fold)
            print("   fold:  written")

        findings = fold["findings"]
        print(f"   counts: total={fold['counts']['total']}", end="")
        for code, n in fold["counts"]["byCode"].items():
            print(f"  {code}={n}", end="")
        print()
        if args.diff_only:
            findings = on_changed_lines(findings, crate, changed_lines(base, head, crate))
            print(f"   diff-only: {len(findings)} finding(s) on changed lines (fold stays full-crate)")
        print_findings(findings, tier)
    return 0


if __name__ == "__main__":
    sys.exit(main())
