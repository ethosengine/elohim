#!/usr/bin/env python3
"""Every compile-time include a crate reaches outside its own src/ must be COPY'd into the
Dockerfile stage that compiles it.

`include_str!` / `include_bytes!` resolve against the source file at compile time. The local tree
always has the target, so `cargo test` cannot see a Dockerfile that forgot to copy it; the image
build then fails an hour into the pipeline (edge #1137, edge #1541).

Non-test includes must be covered by the release stage's COPY chain; test-only includes by the
check stage's. Text-level on purpose: no docker, no cargo.

usage: check-image-include-targets.py [--crate elohim/elohim-storage] [--dockerfile PATH]
                                      [--release-stage builder] [--check-stage check]
exit 0 = covered, 1 = a target is missing from a stage, 2 = usage / unreadable input.
"""
import argparse
import os
import re
import sys

INCLUDE = re.compile(r'include_(?:str|bytes)!\(\s*"([^"]+)"')


def stage_copies(dockerfile_text):
    """stage name -> (parent stage or None, [COPY sources from the build context])."""
    stages, current = {}, None
    logical, buf = [], ""
    for raw in dockerfile_text.splitlines():
        line = raw.rstrip()
        if line.endswith("\\"):
            buf += line[:-1] + " "
            continue
        logical.append(buf + line)
        buf = ""
    for line in logical:
        s = line.strip()
        m = re.match(r"(?i)^FROM\s+(\S+)(?:\s+AS\s+(\S+))?", s)
        if m:
            current = m.group(2) or "stage%d" % len(stages)
            stages[current] = (m.group(1), [])
            continue
        if current and re.match(r"(?i)^COPY\s", s):
            parts = s.split()[1:]
            if any(p.startswith("--from=") for p in parts):
                continue
            parts = [p for p in parts if not p.startswith("--")]
            stages[current][1].extend(p.rstrip("/") for p in parts[:-1])
    return stages


def chain_sources(stages, name):
    out, seen = [], set()
    while name in stages and name not in seen:
        seen.add(name)
        parent, sources = stages[name]
        out.extend(sources)
        name = parent
    return out


def covered(target, sources):
    return any(s in (".", "") or target == s or target.startswith(s + "/") for s in sources)


def includes(crate_dir):
    """(repo-relative target, source file, line number, is_test) for includes leaving src/."""
    src = os.path.join(crate_dir, "src")
    for root, _dirs, files in os.walk(src):
        for name in files:
            if not name.endswith(".rs"):
                continue
            path = os.path.join(root, name)
            test_file = name.endswith("tests.rs") or os.sep + "tests" + os.sep in path
            in_test = test_file
            with open(path, encoding="utf-8") as fh:
                for lineno, line in enumerate(fh, 1):
                    if "#[cfg(test)]" in line:
                        in_test = True
                    for rel in INCLUDE.findall(line):
                        target = os.path.normpath(os.path.join(os.path.dirname(path), rel))
                        if target == src or target.startswith(src + os.sep):
                            continue
                        yield target, path, lineno, in_test


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--crate", default="elohim/elohim-storage")
    ap.add_argument("--dockerfile")
    ap.add_argument("--release-stage", default="builder")
    ap.add_argument("--check-stage", default="check")
    args = ap.parse_args()
    # COPY sources are relative to the build context, the repository root; work there
    # so a crate given by absolute path resolves to the same spelling.
    repo_root = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
    dockerfile = args.dockerfile and os.path.abspath(args.dockerfile)
    crate = os.path.relpath(os.path.abspath(args.crate), repo_root)
    os.chdir(repo_root)
    dockerfile = dockerfile or os.path.join(crate, "Dockerfile")
    try:
        stages = stage_copies(open(dockerfile, encoding="utf-8").read())
    except OSError as e:
        print(f"check-image-include-targets: cannot read {dockerfile}: {e}", file=sys.stderr)
        return 2
    for stage in (args.release_stage, args.check_stage):
        if stage not in stages:
            print(f"check-image-include-targets: {dockerfile} has no stage '{stage}'", file=sys.stderr)
            return 2
    release = chain_sources(stages, args.release_stage)
    check = chain_sources(stages, args.check_stage)
    missing, total = [], 0
    for target, path, lineno, is_test in sorted(set(includes(crate))):
        total += 1
        stage, sources = (args.check_stage, check) if is_test else (args.release_stage, release)
        if not covered(target, sources):
            missing.append((stage, target, path, lineno))
    if missing:
        for stage, target, path, lineno in missing:
            print(
                f"MISSING in stage '{stage}': {target}  (included at {path}:{lineno}) — "
                f"add `COPY {target} ...` to that stage of {dockerfile}",
                file=sys.stderr,
            )
        return 1
    print(f"check-image-include-targets: {total} include target(s) outside src/ covered in {dockerfile}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
