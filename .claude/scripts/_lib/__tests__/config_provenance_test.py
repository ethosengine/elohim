#!/usr/bin/env python3
"""Tests for _lib.config_provenance — the read-vs-set cross-reference that makes the
`ELOHIM_IROH_RELAY_URL` class of gap (shipped `dual` fleet-wide, relay knob read by
code, set by nothing) computable instead of grep-luck.

Run: python3 .claude/scripts/_lib/__tests__/config_provenance_test.py (exit 0 = pass)

All fixtures below are synthetic trees under `tempfile.TemporaryDirectory()` — a test
that asserts against live repo content breaks the moment unrelated content moves, which
is itself one of the findings this module exists to prevent. The ONE exception is the
LIVE-REPO block near the end, deliberately singular and clearly marked, covering the
exact ground-truth case (`ELOHIM_IROH_RELAY_URL`) this module was built for; it
skips cleanly if run against a checkout where that file has moved.
"""
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
REPO_ROOT = here

from _lib import config_provenance as cp  # noqa: E402

_failures = []
_passed = 0


def check(label, condition, detail=""):
    global _passed
    if condition:
        _passed += 1
        print(f"  ✅ {label}")
    else:
        _failures.append(label)
        print(f"  ❌ {label}{(' — ' + detail) if detail else ''}")


def write(tmp: Path, rel: str, content: str) -> Path:
    p = tmp / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content)
    return p


def reads_for(reads_by_key: dict, key: str):
    return reads_by_key.get(key, [])


def writes_for(writes_by_key: dict, key: str):
    return writes_by_key.get(key, [])


# ───────────────────────────── Rust reads ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "src/main.rs",
        '\n'.join([
            "use std::env;",
            "",
            'fn a() {',
            '    if let Ok(v) = std::env::var("ELOHIM_TEST_PLAIN") {',
            '        println!("{}", v);',
            '    }',
            '    let b = env::var("ELOHIM_TEST_DEFAULTED").unwrap_or("fallback".to_string());',
            '    let c = env::var_os("ELOHIM_TEST_VAR_OS");',
            '    let d = option_env!("ELOHIM_TEST_OPTION_ENV").unwrap_or("x");',
            '    let e = env!("ELOHIM_TEST_ENV_MACRO");',
            '    let f = env::var("ELOHIM_TEST_OK").ok();',
            '    let g = env::var("ELOHIM_TEST_UNWRAP_ELSE").unwrap_or_else(|_| "z".into());',
            '    let h = env::var("ELOHIM_TEST_UNWRAP_DEFAULT").unwrap_or_default();',
            '}',
            '',
        ]),
    )
    reads = cp.scan_reads(tmp)
    check("rust plain std::env::var read found", "ELOHIM_TEST_PLAIN" in reads)
    check("rust plain read is not defaulted", reads_for(reads, "ELOHIM_TEST_PLAIN")[0].defaulted is False)
    check("rust plain read line points at its source line", reads_for(reads, "ELOHIM_TEST_PLAIN")[0].line == 4)
    check("rust plain read path is repo-relative posix", reads_for(reads, "ELOHIM_TEST_PLAIN")[0].path == "src/main.rs")
    check("rust .unwrap_or read is defaulted", reads_for(reads, "ELOHIM_TEST_DEFAULTED")[0].defaulted is True)
    check("rust env::var_os read found, not defaulted", reads_for(reads, "ELOHIM_TEST_VAR_OS")[0].defaulted is False)
    check("rust option_env! + .unwrap_or is defaulted", reads_for(reads, "ELOHIM_TEST_OPTION_ENV")[0].defaulted is True)
    check("rust env! macro read found, not defaulted", reads_for(reads, "ELOHIM_TEST_ENV_MACRO")[0].defaulted is False)
    check("rust .ok() chain is defaulted", reads_for(reads, "ELOHIM_TEST_OK")[0].defaulted is True)
    check("rust .unwrap_or_else chain is defaulted", reads_for(reads, "ELOHIM_TEST_UNWRAP_ELSE")[0].defaulted is True)
    check("rust .unwrap_or_default() chain is defaulted", reads_for(reads, "ELOHIM_TEST_UNWRAP_DEFAULT")[0].defaulted is True)


# ───────────────────────────── TS/JS reads ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "src/config.ts",
        '\n'.join([
            'const a = process.env.ELOHIM_TEST_JS_PLAIN;',
            'const b = process.env.ELOHIM_TEST_JS_DEFAULTED || "fallback";',
            'const c = process.env["ELOHIM_TEST_JS_BRACKET"];',
            'const d = process.env.ELOHIM_TEST_JS_NULLISH ?? "x";',
            'const e = import.meta.env.ELOHIM_TEST_JS_IMPORT_META;',
            '',
        ]),
    )
    reads = cp.scan_reads(tmp)
    check("js process.env.KEY read found, not defaulted", reads_for(reads, "ELOHIM_TEST_JS_PLAIN")[0].defaulted is False)
    check("js process.env.KEY || fallback is defaulted", reads_for(reads, "ELOHIM_TEST_JS_DEFAULTED")[0].defaulted is True)
    check("js process.env['KEY'] bracket form found", "ELOHIM_TEST_JS_BRACKET" in reads)
    check("js process.env.KEY ?? fallback is defaulted", reads_for(reads, "ELOHIM_TEST_JS_NULLISH")[0].defaulted is True)
    check("js import.meta.env.KEY read found", "ELOHIM_TEST_JS_IMPORT_META" in reads)


# ───────────────────────────── Python reads ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "tool.py",
        '\n'.join([
            'import os',
            'a = os.environ["ELOHIM_TEST_PY_BRACKET"]',
            'b = os.environ.get("ELOHIM_TEST_PY_GET_PLAIN")',
            'c = os.environ.get("ELOHIM_TEST_PY_GET_DEFAULT", "fallback")',
            'd = os.getenv("ELOHIM_TEST_PY_GETENV_PLAIN")',
            'e = os.getenv("ELOHIM_TEST_PY_GETENV_DEFAULT", "fallback")',
            '',
        ]),
    )
    reads = cp.scan_reads(tmp)
    check("python os.environ[...] read, not defaulted", reads_for(reads, "ELOHIM_TEST_PY_BRACKET")[0].defaulted is False)
    check("python os.environ.get(K) with no default, not defaulted", reads_for(reads, "ELOHIM_TEST_PY_GET_PLAIN")[0].defaulted is False)
    check("python os.environ.get(K, default) is defaulted", reads_for(reads, "ELOHIM_TEST_PY_GET_DEFAULT")[0].defaulted is True)
    check("python os.getenv(K) with no default, not defaulted", reads_for(reads, "ELOHIM_TEST_PY_GETENV_PLAIN")[0].defaulted is False)
    check("python os.getenv(K, default) is defaulted", reads_for(reads, "ELOHIM_TEST_PY_GETENV_DEFAULT")[0].defaulted is True)


# ───────────────────────────── shell reads: scoped to scripts/, self-assign exempt ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "scripts/ci/deploy.sh",
        '\n'.join([
            '#!/bin/bash',
            'echo "${ELOHIM_TEST_SHELL_PLAIN}"',
            'echo "$ELOHIM_TEST_SHELL_BARE"',
            'VALUE="${ELOHIM_TEST_SHELL_DEFAULTED:-fallback}"',
            'export ELOHIM_TEST_SHELL_SELFSET=hello',
            'echo "${ELOHIM_TEST_SHELL_SELFSET}"',
            '',
        ]),
    )
    write(
        tmp,
        "deploy/outside.sh",
        '\n'.join([
            'echo "${ELOHIM_TEST_SHELL_OUTSIDE_SCOPE}"',
            'export ELOHIM_TEST_SHELL_OUTSIDE_WRITE=1',
            '',
        ]),
    )
    reads = cp.scan_reads(tmp)
    writes = cp.scan_writes(tmp)
    check("shell braced read found in scripts/ scope", "ELOHIM_TEST_SHELL_PLAIN" in reads)
    check("shell braced read is not defaulted", reads_for(reads, "ELOHIM_TEST_SHELL_PLAIN")[0].defaulted is False)
    check("shell bare $KEY read found in scripts/ scope", "ELOHIM_TEST_SHELL_BARE" in reads)
    check("shell ${KEY:-default} read is defaulted", reads_for(reads, "ELOHIM_TEST_SHELL_DEFAULTED")[0].defaulted is True)
    check(
        "shell var assigned+read in the SAME file is not double-counted as a read",
        "ELOHIM_TEST_SHELL_SELFSET" not in reads,
        str(reads_for(reads, "ELOHIM_TEST_SHELL_SELFSET")),
    )
    check("shell export writes are recorded", "ELOHIM_TEST_SHELL_SELFSET" in writes)
    check(
        "shell read OUTSIDE scripts/ scope is not counted as a read",
        "ELOHIM_TEST_SHELL_OUTSIDE_SCOPE" not in reads,
    )
    check(
        "shell write OUTSIDE scripts/ scope is still counted (writes are unscoped)",
        "ELOHIM_TEST_SHELL_OUTSIDE_WRITE" in writes,
    )


# ───────────────────────────── k8s manifest (YAML) writes ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "manifests/deploy.yaml",
        '\n'.join([
            "apiVersion: v1",
            "kind: ConfigMap",
            "metadata:",
            "  name: test-cm",
            "---",
            "apiVersion: apps/v1",
            "kind: Deployment",
            "metadata:",
            "  name: ELOHIM_TEST_NOT_AN_ENV_VAR",
            "spec:",
            "  template:",
            "    spec:",
            "      containers:",
            "        - name: app",
            "          env:",
            "            - name: ELOHIM_TEST_YAML_SET",
            "              value: \"hello\"",
            "            - name: ELOHIM_TEST_YAML_SET2",
            "              value: \"world\"",
            "",
        ]),
    )
    writes = cp.scan_writes(tmp)
    check("yaml env: list entry recorded as a write", "ELOHIM_TEST_YAML_SET" in writes)
    check("yaml second env: list entry recorded", "ELOHIM_TEST_YAML_SET2" in writes)
    check(
        "yaml bare 'name:' OUTSIDE an env: list is NOT a write (metadata.name is not env)",
        "ELOHIM_TEST_NOT_AN_ENV_VAR" not in writes,
    )
    check(
        "yaml write line number points at the '- name:' line",
        writes_for(writes, "ELOHIM_TEST_YAML_SET")[0].line == 16,
        str(writes_for(writes, "ELOHIM_TEST_YAML_SET")),
    )
    check("yaml write kind is tagged k8s-env", writes_for(writes, "ELOHIM_TEST_YAML_SET")[0].kind == "k8s-env")

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "manifests/malformed.yaml",
        '\n'.join([
            "apiVersion: apps/v1",
            "kind: Deployment",
            "spec:",
            "  template:",
            "    spec:",
            "      containers:",
            "        - name: app",
            "          env:",
            "            - name: ELOHIM_TEST_YAML_FALLBACK",
            "              value: {{ .Values.unterminated",
            "",
        ]),
    )
    try:
        writes = cp.scan_writes(tmp)
        raised = False
    except Exception as e:  # noqa: BLE001
        writes = {}
        raised = True
    check("malformed yaml document never raises out of scan_writes", raised is False)
    check(
        "malformed yaml falls back to the line-scanner and still finds the env entry",
        "ELOHIM_TEST_YAML_FALLBACK" in writes,
        str(writes),
    )


# ───────────────────────────── Dockerfile writes ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "docker/Dockerfile",
        '\n'.join([
            "FROM node:24",
            "ENV ELOHIM_TEST_DOCKER_SINGLE=value1",
            "ENV ELOHIM_TEST_DOCKER_MULTI1=a ELOHIM_TEST_DOCKER_MULTI2=b",
            "ENV ELOHIM_TEST_DOCKER_OLDSTYLE oldvalue",
            "",
        ]),
    )
    writes = cp.scan_writes(tmp)
    check("dockerfile single ENV KEY=value found", "ELOHIM_TEST_DOCKER_SINGLE" in writes)
    check("dockerfile multi KEY=value on one ENV line, first key found", "ELOHIM_TEST_DOCKER_MULTI1" in writes)
    check("dockerfile multi KEY=value on one ENV line, second key found", "ELOHIM_TEST_DOCKER_MULTI2" in writes)
    check("dockerfile old-style 'ENV KEY value' found", "ELOHIM_TEST_DOCKER_OLDSTYLE" in writes)
    check("dockerfile write kind is tagged dockerfile-env", writes_for(writes, "ELOHIM_TEST_DOCKER_SINGLE")[0].kind == "dockerfile-env")


# ───────────────────────────── .env-style file writes ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "config/.env",
        '\n'.join([
            "# comment line, should be skipped",
            "ELOHIM_TEST_ENVFILE_PLAIN=value",
            "export ELOHIM_TEST_ENVFILE_EXPORT=value2",
            "",
        ]),
    )
    write(tmp, "config/.env.production", "ELOHIM_TEST_ENVFILE_PROD=value3\n")
    writes = cp.scan_writes(tmp)
    check(".env plain KEY=value found", "ELOHIM_TEST_ENVFILE_PLAIN" in writes)
    check(".env export KEY=value found", "ELOHIM_TEST_ENVFILE_EXPORT" in writes)
    check(".env.production (named-variant) file is recognized", "ELOHIM_TEST_ENVFILE_PROD" in writes)


# ───────────────────────────── Jenkinsfile (Groovy) writes ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "pipeline/Jenkinsfile",
        '\n'.join([
            "pipeline {",
            "    agent any",
            "    environment {",
            "        ELOHIM_TEST_JENKINS_ENV_BLOCK = 'value'",
            '        OTHER_JENKINS_VAR = "value2"',
            "    }",
            "    stages {",
            "        stage('Deploy') {",
            "            steps {",
            "                script {",
            '                    withEnv(["ELOHIM_TEST_JENKINS_WITHENV=${params.FOO}", "ELOHIM_TEST_JENKINS_WITHENV2=bar"]) {',
            "                        sh 'echo hi'",
            "                    }",
            "                }",
            "            }",
            "        }",
            "    }",
            "}",
            "",
        ]),
    )
    writes = cp.scan_writes(tmp)
    check("jenkinsfile environment{} block entry found", "ELOHIM_TEST_JENKINS_ENV_BLOCK" in writes)
    check("jenkinsfile environment{} second entry found", "OTHER_JENKINS_VAR" in writes)
    check("jenkinsfile withEnv([...]) first entry found", "ELOHIM_TEST_JENKINS_WITHENV" in writes)
    check("jenkinsfile withEnv([...]) second entry found", "ELOHIM_TEST_JENKINS_WITHENV2" in writes)


# ───────────────────────────── ignore-list + key-shape filtering ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "src/ignored.rs",
        '\n'.join([
            'fn ignored() {',
            '    let _ = std::env::var("PATH");',
            '    let _ = std::env::var("HOME");',
            '    let _ = std::env::var("RUSTFLAGS");',
            '    let _ = std::env::var("CARGO_HOME");',
            '    let _ = std::env::var("NODE_ENV");',
            '    let _ = std::env::var("X");',
            '    let _ = std::env::var("someLowerCaseThing");',
            '    let _ = std::env::var("ELOHIM_TEST_KEEP_ME");',
            '}',
            '',
        ]),
    )
    reads = cp.scan_reads(tmp)
    check("PATH is ignored", "PATH" not in reads)
    check("HOME is ignored", "HOME" not in reads)
    check("RUSTFLAGS is ignored", "RUSTFLAGS" not in reads)
    check("CARGO_* prefix is ignored", "CARGO_HOME" not in reads)
    check("NODE_ENV is ignored", "NODE_ENV" not in reads)
    check("single-char key is ignored", "X" not in reads)
    check("lowercase (non-SCREAMING_SNAKE_CASE) key is ignored", "someLowerCaseThing" not in reads)
    check("a real ELOHIM_ key survives the filter", "ELOHIM_TEST_KEEP_ME" in reads)


# ───────────────────────────── directory pruning ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(tmp, "node_modules/pkg/index.js", "const a = process.env.ELOHIM_TEST_SHOULD_BE_PRUNED;\n")
    write(tmp, "target/debug/build/out.rs", 'let a = std::env::var("ELOHIM_TEST_ALSO_PRUNED");\n')
    write(tmp, "src/keep.rs", 'let a = std::env::var("ELOHIM_TEST_NOT_PRUNED");\n')
    # .claude/worktrees/ holds sibling git worktrees — duplicate checkouts of this same
    # repo. Unpruned, every site in the main tree is reported N+1 times (once per live
    # worktree), which reads as "N deployment surfaces set this" for a single manifest.
    write(tmp, ".claude/worktrees/agent-deadbeef/src/keep.rs",
          'let a = std::env::var("ELOHIM_TEST_WORKTREE_DUPE");\n')
    reads = cp.scan_reads(tmp)
    check("node_modules/ is pruned from the walk", "ELOHIM_TEST_SHOULD_BE_PRUNED" not in reads)
    check("target/ is pruned from the walk", "ELOHIM_TEST_ALSO_PRUNED" not in reads)
    check(".claude/worktrees/ (sibling checkouts) is pruned from the walk",
          "ELOHIM_TEST_WORKTREE_DUPE" not in reads)
    check("a real source file outside vendor dirs is scanned", "ELOHIM_TEST_NOT_PRUNED" in reads)


# ───────────────────────────── provenance() verdicts + unset_keys() ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(
        tmp,
        "src/app.rs",
        '\n'.join([
            'fn f() {',
            '    let a = std::env::var("ELOHIM_TEST_UNSET_KEY");',
            '    let b = std::env::var("ELOHIM_TEST_SET_KEY");',
            '    let c = std::env::var("ELOHIM_TEST_DEFAULTED_KEY").unwrap_or("d".to_string());',
            '}',
            '',
        ]),
    )
    write(
        tmp,
        "manifests/deploy.yaml",
        '\n'.join([
            "apiVersion: v1",
            "kind: Pod",
            "spec:",
            "  containers:",
            "    - name: app",
            "      env:",
            "        - name: ELOHIM_TEST_SET_KEY",
            "          value: \"1\"",
            "        - name: ELOHIM_TEST_ORPHAN_KEY",
            "          value: \"2\"",
            "",
        ]),
    )
    prov = cp.provenance(tmp)
    check("unset: read by code, set by nothing", prov["ELOHIM_TEST_UNSET_KEY"].verdict == "unset")
    check("set: read AND written", prov["ELOHIM_TEST_SET_KEY"].verdict == "set")
    check("set: reads list is populated", len(prov["ELOHIM_TEST_SET_KEY"].reads) >= 1)
    check("set: writes list is populated", len(prov["ELOHIM_TEST_SET_KEY"].writes) >= 1)
    check("orphan-set: written, read by no code", prov["ELOHIM_TEST_ORPHAN_KEY"].verdict == "orphan-set")
    check("defaulted: read only in a form supplying its own default, set nowhere", prov["ELOHIM_TEST_DEFAULTED_KEY"].verdict == "defaulted")

    unset = cp.unset_keys(prov)
    check("unset_keys() returns exactly the unset verdicts", [kp.key for kp in unset] == ["ELOHIM_TEST_UNSET_KEY"])

    narrowed = cp.provenance(tmp, keys=["ELOHIM_TEST_SET_KEY"])
    check("provenance(keys=...) returns only the requested key", set(narrowed) == {"ELOHIM_TEST_SET_KEY"}, str(set(narrowed)))
    check("provenance(keys=...) result is otherwise identical to the full scan", narrowed["ELOHIM_TEST_SET_KEY"].verdict == "set")


# ───────────────────────────── malformed input never raises ─────────────────────────────

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    write(tmp, "docker/Dockerfile", "\x00\x01 not really a dockerfile \xff\xfe\n" * 5)
    write(tmp, "manifests/garbage.yaml", ": : : not : yaml : at : all : {{{{ [[[[ \n" * 3)
    try:
        cp.provenance(tmp)
        raised = False
    except Exception:  # noqa: BLE001
        raised = True
    check("provenance() never raises on garbage input files", raised is False)


# ───────────────────────────── LIVE-REPO ground truth (the ONE non-synthetic case) ─────────────────────────────
# This is the exact defect this module exists to make computable: ELOHIM_TRANSPORT_BACKEND
# shipped `dual` fleet-wide while the iroh relay knob it depends on, ELOHIM_IROH_RELAY_URL,
# was read by elohim-storage's p2p_iroh config and set by nothing. Skips cleanly (does not
# fail) if this checkout's layout has moved the file.
#
# LIVE TRUTH MOVED 2026-08-30, and this expectation moved with it: commit 7234b6ff0
# ("manifests(alpha): storage-side iroh relay env + manifest board on doorway-alpha")
# wired the knob at genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
# (a k8s-env write), so the verdict is now `set` — read by p2p_iroh/config.rs, written by
# the edgenode template. The instrument stays pointed at live truth: it asserts the gap is
# CLOSED and by which surface, so a regression that drops the env re-reds this check.

_iroh_config_rs = REPO_ROOT / "elohim" / "elohim-storage" / "src" / "p2p_iroh" / "config.rs"
if _iroh_config_rs.is_file():
    live = cp.provenance(REPO_ROOT, keys=["ELOHIM_IROH_RELAY_URL"])
    kp = live.get("ELOHIM_IROH_RELAY_URL")
    check("LIVE: ELOHIM_IROH_RELAY_URL is discovered as a read", kp is not None, str(live))
    if kp is not None:
        check(
            "LIVE: the read site is the real p2p_iroh config source",
            any(r.path.endswith("elohim/elohim-storage/src/p2p_iroh/config.rs") for r in kp.reads),
            str(kp.reads),
        )
        check(
            "LIVE: the edgenode manifest template is the deployment surface that sets it",
            any(
                w.path.endswith(
                    "genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml"
                )
                and w.kind == "k8s-env"
                for w in kp.writes
            ),
            str(kp.writes),
        )
        check(
            "LIVE: exactly one surface sets it (a second write site means a duplicate "
            "checkout leaked into the walk, or two manifests disagree)",
            len(kp.writes) == 1,
            str(kp.writes),
        )
        check(
            "LIVE: verdict is 'set' (read by p2p_iroh config, written by the edgenode template)",
            kp.verdict == "set",
            kp.verdict,
        )
else:
    print("  ⏭  SKIP live-repo ground-truth case (p2p_iroh/config.rs not found in this checkout)")


if _failures:
    print(f"\n{len(_failures)} failure(s): {', '.join(_failures)}")
    raise SystemExit(1)

print(f"\n{_passed} checks passed ✅")
