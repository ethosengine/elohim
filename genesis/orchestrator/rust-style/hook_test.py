"""Contract tests for the shared Claude/Codex edit adapter; no files are edited."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("rust_style_hook", ROOT / ".claude/hooks/rust-style.py")
HOOK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HOOK)


def install_wrapper(root, name):
    directory = root / "genesis/orchestrator"
    directory.mkdir(parents=True, exist_ok=True)
    script = directory / name
    script.write_text((ROOT / "genesis/orchestrator" / name).read_text())
    helper = directory / "rust-style-build-env.sh"
    helper.write_text((ROOT / "genesis/orchestrator/rust-style-build-env.sh").read_text())
    return script


class Adapter(unittest.TestCase):
    def invoke(self, tool, data, code=0, failures=None, stderr=""):
        result = SimpleNamespace(returncode=code, stdout=json.dumps({"failures": failures or []}), stderr=stderr)
        with patch.object(HOOK.subprocess, "run", return_value=result) as native:
            answer = HOOK.inspect(ROOT, {"tool_name": tool, "tool_input": data})
        return answer, native

    def test_native_denial_reaches_both_write_names(self):
        for name in ("Write", "write_file"):
            result, native = self.invoke(name, {"file_path": "fixture.rs", "content": "#[allow(unused)] fn f() {}"}, 1, ["RS-GOV-04 new finding"])
            self.assertIn("RS-GOV-04", result["deny"])
            self.assertEqual(native.call_count, 1)

    def test_unavailable_is_explicitly_unmeasured(self):
        result, _ = self.invoke("Write", {"path": "fixture.rs", "content": ""}, 2, stderr="checker stale")
        self.assertIn("UNMEASURED", result["context"])
        self.assertNotIn("deny", result)

    def test_unrelated_tools_and_opaque_edits_make_no_clean_claim(self):
        result, native = self.invoke("Read", {"path": "README.md"})
        self.assertIsNone(result)
        self.assertFalse(native.called)

    def test_codex_freeform_events_surface_the_active_rule(self):
        for tool, source in (
            ("apply_patch", "*** Begin Patch\n*** Add File: src/lib.rs\n+fn f() {}\n*** End Patch"),
            ("functions.exec", 'await tools.exec_command({cmd: "cargo test"})'),
        ):
            result = HOOK.inspect(ROOT, {"tool_name": tool, "tool_input": source})
            self.assertIn("RS-GOV-04", result["context"])
            self.assertIn("not been certified clean", result["context"])
        result, native = self.invoke("apply_patch", {"path": "fixture.rs"})
        self.assertIn("not been measured", result["context"])
        self.assertFalse(native.called)

    def test_edit_reconstructs_the_complete_candidate(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "genesis/orchestrator/rust-style") as tmp:
            path = Path(tmp) / "fixture.rs"
            path.write_text("pub fn old() {}\n")
            result, native = self.invoke("Edit", {"file_path": str(path), "old_string": "old", "new_string": "new"})
            self.assertIn("pass", result["context"])
            self.assertEqual(native.call_args.kwargs["input"], "pub fn new() {}\n")

    def test_registered_in_both_runtimes(self):
        for config in (".claude/settings.json", ".codex/hooks.json"):
            self.assertIn("rust-style.py", (ROOT / config).read_text())


class BuildEnvironment(unittest.TestCase):
    VARIABLES = (
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS",
        "CARGO_REGISTRIES_ELOHIM_TOKEN",
    )

    def invoke(self, root, token=None):
        helper = ROOT / "genesis/orchestrator/rust-style-build-env.sh"
        script = """
source "$1"
rust_style_build_environment "$2" || exit $?
for style_variable in "${@:3}"; do
  printf '%s\\n' "${!style_variable}"
done
"""
        env = os.environ.copy()
        env.pop("CARGO_REGISTRIES_ELOHIM_TOKEN", None)
        if token is not None:
            env["CARGO_REGISTRIES_ELOHIM_TOKEN"] = token
        return subprocess.run(
            ["bash", "-c", script, "fixture", str(helper), str(root), *self.VARIABLES],
            env=env,
            text=True,
            capture_output=True,
        )

    def test_wrappers_pin_compiler_and_registry_environment(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "genesis/orchestrator/rust-style").mkdir(parents=True)
            result = self.invoke(root)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                result.stdout.splitlines(),
                ["rustc", "", "", "", "", "cargo:token", "anonymous"],
            )
            explicit = self.invoke(root, "fixture-private-token")
            self.assertEqual(explicit.returncode, 0, explicit.stderr)
            self.assertEqual(explicit.stdout.splitlines()[-1], "fixture-private-token")

        for wrapper in ("rust-style.sh", "rust-style-ci.sh"):
            source = (ROOT / "genesis/orchestrator" / wrapper).read_text()
            self.assertLess(source.index("rust_style_build_environment"), source.index("cargo build"))
            self.assertLess(source.index('cd "$style_root"'), source.index("cargo build"))

    def test_legacy_config_at_every_checker_ancestor_refuses_unmeasured(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            checker = root / "genesis/orchestrator/rust-style"
            checker.mkdir(parents=True)
            for directory in (root, root / "genesis", root / "genesis/orchestrator", checker):
                with self.subTest(directory=directory):
                    legacy = directory / ".cargo/config"
                    legacy.parent.mkdir(parents=True, exist_ok=True)
                    legacy.write_text("[build]\nrustc='unreviewed'\n")
                    result = self.invoke(root)
                    self.assertEqual(result.returncode, 2)
                    self.assertIn("Rust style UNMEASURED", result.stderr)
                    self.assertIn(str(legacy), result.stderr)
                    legacy.unlink()

    def test_nested_config_toml_refuses_but_root_inventory_is_allowed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            checker = root / "genesis/orchestrator/rust-style"
            checker.mkdir(parents=True)
            root_config = root / ".cargo/config.toml"
            root_config.parent.mkdir()
            root_config.write_text("[build]\nincremental=false\n")
            self.assertEqual(self.invoke(root).returncode, 0)
            for directory in (root / "genesis", root / "genesis/orchestrator", checker):
                with self.subTest(directory=directory):
                    nested = directory / ".cargo/config.toml"
                    nested.parent.mkdir(parents=True, exist_ok=True)
                    nested.write_text("[build]\nrustc='unreviewed'\n")
                    result = self.invoke(root)
                    self.assertEqual(result.returncode, 2)
                    self.assertIn("Rust style UNMEASURED", result.stderr)
                    self.assertIn("below the repository root", result.stderr)
                    self.assertIn(str(nested), result.stderr)
                    nested.unlink()


class WrapperCargoConfig(unittest.TestCase):
    def fixture(self, root):
        script = install_wrapper(root, "rust-style.sh")
        checker = root / "genesis/orchestrator/rust-style"
        (checker / "src").mkdir(parents=True)
        (checker / "Cargo.toml").write_text(
            '[package]\nname="epr-rust-style"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n'
        )
        (checker / "Cargo.lock").write_text(
            'version = 4\n\n[[package]]\nname = "epr-rust-style"\nversion = "0.0.0"\n'
        )
        (checker / "src/main.rs").write_text(
            'fn main() { println!("{}", std::env::current_dir().unwrap().display()); }\n'
        )
        pool = root / "genesis/agentic/bin/pool-lib.sh"
        pool.parent.mkdir(parents=True)
        pool.write_text(
            'detect_family() { echo fixture; }\n'
            'slot_path() { echo "$STYLE_TEST_SLOT"; }\n'
        )
        (pool.parent.parent / "pool-policy.json").write_text(json.dumps({
            "cargo_env_overrides": {"*": {"RUSTUP_TOOLCHAIN": "1.96.1"}}
        }))
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        env = {**os.environ, "STYLE_TEST_SLOT": str(root / "pool")}
        return script, checker, env

    def test_nested_config_refuses_before_marker_linker_can_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script, checker, env = self.fixture(root)
            marker = root / "linker-ran"
            linker = root / "marker-linker"
            linker.write_text(
                '#!/usr/bin/env bash\n'
                'printf reached > "$STYLE_LINKER_MARKER"\n'
                'exec cc "$@"\n'
            )
            linker.chmod(0o755)
            host = next(
                line.removeprefix("host: ")
                for line in subprocess.check_output(
                    ["rustc", "-vV"], env={**env, "RUSTUP_TOOLCHAIN": "1.96.1"}, text=True
                ).splitlines()
                if line.startswith("host: ")
            )
            nested = checker / ".cargo/config.toml"
            nested.parent.mkdir()
            nested.write_text(
                f'[target.{json.dumps(host)}]\nlinker = {json.dumps(str(linker))}\n'
            )
            result = subprocess.run(
                ["bash", str(script)],
                cwd=checker,
                env={**env, "STYLE_LINKER_MARKER": str(marker)},
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("Rust style UNMEASURED", result.stderr)
            self.assertIn(str(nested), result.stderr)
            self.assertFalse(marker.exists(), "refused nested config must never execute its linker")

    def test_nested_invocation_without_config_normalizes_to_repository_root(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script, checker, env = self.fixture(root)
            result = subprocess.run(
                ["bash", str(script)],
                cwd=checker,
                env=env,
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), str(root))


class CiBaseline(unittest.TestCase):
    def test_first_multicommit_branch_uses_integration_and_missing_evidence_refuses(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = install_wrapper(root, "rust-style-ci.sh")
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, text=True).strip()
            git("init", "-q")
            def commit(source):
                (root / "fixture.rs").write_text(source)
                git("add", "fixture.rs")
                git("-c", "user.name=Style fixture", "-c", "user.email=style@example.invalid", "commit", "-qm", "fixture")
                return git("rev-parse", "HEAD")
            base = commit("pub fn clean() {}")
            git("update-ref", "refs/remotes/origin/dev", base)
            commit("#[allow(dead_code)] fn new_violation() {}")
            head = commit("#[allow(dead_code)] fn new_violation() {} // later edit")
            def selected(branch, previous=""):
                env = {**os.environ, "BRANCH_NAME": branch, "CHANGE_TARGET": "", "GIT_PREVIOUS_SUCCESSFUL_COMMIT": previous}
                return subprocess.run(["bash", str(script), "--print-base"], cwd=root, env=env, text=True, capture_output=True)
            first = selected("feature/style")
            self.assertEqual(first.returncode, 0, first.stderr)
            self.assertEqual(first.stdout.strip(), base)
            self.assertEqual(selected("dev").returncode, 2)
            self.assertEqual(selected("dev", base).stdout.strip(), base)
            self.assertEqual(selected("dev", head).stdout.strip(), git("rev-parse", "HEAD^"))
            git("update-ref", "-d", "refs/remotes/origin/dev")
            self.assertEqual(selected("feature/style").returncode, 2)


class GateGit(unittest.TestCase):
    def test_hook_refuses_changed_method_build_inputs_without_running_cargo(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = install_wrapper(root, "rust-style.sh")
            pool = root / "genesis/agentic/bin/pool-lib.sh"
            pool.parent.mkdir(parents=True)
            pool.write_text('detect_family() { echo fixture; }\nslot_path() { echo "$STYLE_TEST_SLOT"; }\n')
            (pool.parent.parent / "pool-policy.json").write_text(json.dumps({
                "cargo_env_overrides": {"*": {"RUSTUP_TOOLCHAIN": "1.96.1"}}
            }))
            binary = root / "pool/debug/epr-rust-style"
            binary.parent.mkdir(parents=True)
            binary.write_text('#!/bin/sh\necho "checker invoked"\n')
            binary.chmod(0o755)
            os.utime(binary, (100, 100))
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            env = {**os.environ, "STYLE_TEST_SLOT": str(binary.parent.parent)}
            for relative in (
                "genesis/orchestrator/rust-style/Cargo.toml",
                "genesis/orchestrator/rust-style/Cargo.lock",
                "genesis/orchestrator/rust-style/src/discipline.rs",
                ".cargo/config.toml",
            ):
                with self.subTest(relative=relative):
                    path = root / relative
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text("changed method input\n")
                    os.utime(path, (200, 200))
                    result = subprocess.run(["bash", str(script), "--hook"], cwd=root,
                                            env=env, text=True, capture_output=True)
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertIn("checker inputs changed", result.stderr)
                    self.assertNotIn("checker invoked", result.stdout)
                    path.unlink()

    def test_hook_does_not_treat_registry_dependency_sources_as_checker_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = install_wrapper(root, "rust-style.sh")
            pool = root / "genesis/agentic/bin/pool-lib.sh"
            pool.parent.mkdir(parents=True)
            pool.write_text('detect_family() { echo fixture; }\nslot_path() { echo "$STYLE_TEST_SLOT"; }\n')
            (pool.parent.parent / "pool-policy.json").write_text(json.dumps({
                "cargo_env_overrides": {"*": {"RUSTUP_TOOLCHAIN": "1.96.1"}}
            }))
            binary = root / "pool/debug/epr-rust-style"
            binary.parent.mkdir(parents=True)
            binary.write_text('#!/bin/sh\necho "checker invoked"\n')
            binary.chmod(0o755)
            os.utime(binary, (100, 100))
            dependency = root / "elohim/eprfs/eprfs-meta/src/lib.rs"
            dependency.parent.mkdir(parents=True)
            dependency.write_text("changed registry dependency source\n")
            os.utime(dependency, (200, 200))
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            env = {**os.environ, "STYLE_TEST_SLOT": str(binary.parent.parent)}
            result = subprocess.run(
                ["bash", str(script), "--hook"],
                cwd=root,
                env=env,
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("checker invoked", result.stdout)

    def test_ownership_failure_names_cure_before_pool_or_cargo(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = install_wrapper(root, "rust-style.sh")
            # No pool or Cargo executable is provided: the refusal must precede both.
            env = {
                "PATH": os.environ["PATH"],
                "GIT_CONFIG_NOSYSTEM": "1",
                "GIT_CONFIG_GLOBAL": "/dev/null",
                "GIT_TEST_ASSUME_DIFFERENT_OWNER": "1",
            }
            subprocess.run(["git", "init", "-q", str(root)], env=env, check=True)
            for args in ([], ["--hook"]):
                with self.subTest(args=args):
                    result = subprocess.run(["bash", str(script), *args], cwd=root,
                                            env=env, text=True, capture_output=True)
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertIn("Rust style UNMEASURED", result.stderr)
                    self.assertIn("dubious ownership", result.stderr)
                    self.assertIn("git config --global --add safe.directory '*'", result.stderr)
                    self.assertIn("setup-git", result.stderr)


if __name__ == "__main__":
    unittest.main()
