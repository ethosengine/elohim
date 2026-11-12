#!/usr/bin/env python3
"""Offline fixtures; retire with native sense ownership of poll/reconcile/paydown judgement."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SCRIPTS = HERE.parents[1]
ROOT = SCRIPTS.parents[1]
FIXTURES = HERE / "fixtures/dependabot-paydown"
sys.path.insert(0, str(SCRIPTS))
from _lib import dependabot_paydown as dp

spec = importlib.util.spec_from_file_location("paydown_cli", SCRIPTS / "dependabot-paydown.py")
cli = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cli)


def row(**kwargs):
    return {"status": "open", "package": "fixture-crate", "ecosystem": "rust",
            "manifest_path": "crate/Cargo.lock", "first_patched": "0.2.5",
            "vulnerable_range": ">= 0.2.0, < 0.2.5", "severity": "critical", **kwargs}


class FixtureRange:
    """Object-reader fixture: no commits, dirty-tree reads or network operations."""
    def __init__(self, base, head, changed):
        self.trees = {"base": base, "head": head}
        self.changed = changed
        self.calls = []

    def git(self, root, *args):
        self.calls.append(args)
        if args[0] == "rev-parse":
            return args[-1].removesuffix("^{commit}")
        if args[0] == "merge-base":
            return "base"
        if args[0] == "diff":
            return "\0".join(self.changed) + "\0"
        if args[0] == "ls-tree":
            return "\0".join(self.trees[args[-1]]) + "\0"
        if args[0] == "show":
            rev, path = args[-1].split(":", 1)
            return self.trees[rev][path]
        raise AssertionError(args)


class PaydownTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix=".paydown-test-", dir=HERE)
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.ledger = self.root / "ledger.jsonl"
        self.old = (FIXTURES / "Cargo.before.lock").read_text()
        self.new = (FIXTURES / "Cargo.after.lock").read_text()

    def run_check(self, old=None, new=None, rows=None, mode="strict", changed=None,
                  manifest="crate/Cargo.lock", extra=None):
        old = self.old if old is None else old
        new = old if new is None else new
        trees = FixtureRange({manifest: old, **(extra or {})}, {manifest: new, **(extra or {})},
                             changed or [manifest])
        self.ledger.write_text("".join(json.dumps(r) + "\n" for r in (rows if rows is not None else [row()])))
        output = io.StringIO()
        with patch.object(dp, "git", side_effect=trees.git), patch.dict(os.environ, {"SECURITY_PAYDOWN": mode}), contextlib.redirect_stdout(output):
            code = cli.main(["--base", "base", "--head", "head", "--root", str(self.root), "--ledger", str(self.ledger)])
        return code, output.getvalue(), trees

    def test_new_vulnerable_resolution_refused(self):
        code, text, _ = self.run_check(old=self.new, new=self.old)
        self.assertEqual(code, 1, text)
        self.assertIn("REFUSED crate/Cargo.lock: 1 new vulnerable", text)

    def test_no_paydown_strict_and_warn(self):
        for mode, expected in (("strict", 1), ("warn", 0)):
            code, text, _ = self.run_check(mode=mode)
            self.assertEqual(code, expected, text)
            self.assertIn("no paydown", text)
            self.assertIn("cargo update -p fixture-crate --precise 0.2.5", text)

    def test_paydown(self):
        code, text, tree = self.run_check(new=self.new)
        self.assertEqual(code, 0, text)
        self.assertIn("ok crate/Cargo.lock: resolved 1", text)
        self.assertIn(("show", "head:crate/Cargo.lock"), tree.calls)

    def test_partial_duplicate_upgrade_is_not_closure(self):
        code, text, _ = self.run_check(new=self.new + '\n[[package]]\nname="fixture-crate"\nversion="0.2.3"\n')
        self.assertEqual(code, 1, text)
        self.assertIn("no paydown", text)

    def test_new_vulnerable_copy_cannot_hide_behind_paydown(self):
        rows = [row(), row(package="other", first_patched="1.2.0", vulnerable_range="<1.2.0")]
        code, text, _ = self.run_check(new=self.new + '\n[[package]]\nname="other"\nversion="1.0.0"\n', rows=rows)
        self.assertEqual(code, 1, text)
        self.assertIn("new vulnerable", text)

    def test_inherited_alert_version_change_is_not_a_new_alert(self):
        other = '\n[[package]]\nname="other"\nversion="1.0.0"\n'
        rows = [row(), row(package="other", first_patched="1.2.0", vulnerable_range="<1.2.0")]
        code, text, _ = self.run_check(old=self.old + other,
                                      new=self.old.replace("0.2.3", "0.2.4") + other.replace("1.0.0", "1.2.0"), rows=rows)
        self.assertEqual(code, 0, text)
        self.assertIn("resolved 1", text)

    def test_unpatchable_and_closed_ignored(self):
        code, text, _ = self.run_check(rows=[row(first_patched=None), row(status="closed")])
        self.assertEqual(code, 0, text)
        self.assertNotIn("REFUSED", text)

    def test_already_patched_stale_ledger_does_not_demand_more(self):
        code, text, _ = self.run_check(old=self.new)
        self.assertEqual(code, 0, text)
        self.assertIn("no outstanding patchable debt", text)

    def test_workspace_manifest_selects_shared_lock(self):
        code, text, _ = self.run_check(changed=["crate/member/Cargo.toml"],
                                      extra={"crate/member/Cargo.toml": "[package]\n"})
        self.assertEqual(code, 1, text)
        self.assertIn("REFUSED crate/Cargo.lock", text)

    def test_other_workspace_and_non_dependency_changes_do_not_select(self):
        for path in ("independent/Cargo.toml", "crate/src/lib.rs"):
            code, text, _ = self.run_check(changed=[path])
            self.assertEqual(code, 0, text)
            self.assertNotIn("REFUSED", text)

    def test_pnpm_manifest_and_lock_paydown(self):
        for change in ("package.json", "pnpm-workspace.yaml", "pnpm-lock.yaml", "app/package.json"):
            code, text, _ = self.run_check(
                old=(FIXTURES / "pnpm.before.yaml").read_text(),
                new=(FIXTURES / "pnpm.after.yaml").read_text(),
                manifest="pnpm-lock.yaml", changed=[change],
                rows=[row(manifest_path="pnpm-lock.yaml", ecosystem="npm", package="@scope/fixture")])
            self.assertEqual(code, 0, text)
            self.assertIn("resolved 1", text)
            # A paid-down line names no remediation (the pre-push ledger is a stale snapshot);
            # the pnpm hint is covered by test_remediation_uses_cargo_for_the_api_rust_ecosystem.
            self.assertNotIn("→", text)

    def test_each_manifest_must_pay(self):
        fixture = FixtureRange({"a/Cargo.lock": self.old, "b/Cargo.lock": self.old},
                               {"a/Cargo.lock": self.new, "b/Cargo.lock": self.old},
                               ["a/Cargo.lock", "b/Cargo.toml"])
        with patch.object(dp, "git", side_effect=fixture.git):
            results = dp.check(self.root, "base", "head", [row(manifest_path=f"{p}/Cargo.lock") for p in "ab"])
        self.assertEqual([ok for ok, _ in results], [True, False])

    def test_deleted_lock_with_remaining_manifest_refuses(self):
        # A parent lock must not mask a deleted independent workspace lock.
        fixture = FixtureRange({"Cargo.lock": self.new, "crate/Cargo.lock": self.old, "crate/Cargo.toml": ""},
                               {"Cargo.lock": self.new, "crate/Cargo.toml": ""}, ["crate/Cargo.lock"])
        with patch.object(dp, "git", side_effect=fixture.git):
            results = dp.check(self.root, "base", "head", [row()])
        self.assertFalse(results[0][0])
        self.assertIn("no committed lockfile", results[0][1])

    def test_removed_dependency_scope_pays_down(self):
        fixture = FixtureRange({"crate/Cargo.lock": self.old}, {}, ["crate/Cargo.lock"])
        with patch.object(dp, "git", side_effect=fixture.git):
            results = dp.check(self.root, "base", "head", [row()])
        self.assertTrue(results[0][0], results)

    def test_remediation_uses_cargo_for_the_api_rust_ecosystem(self):
        # The GitHub Dependabot API spells the Rust ecosystem "rust" (not "cargo").
        for ecosystem in ("rust", "cargo"):
            hint = dp.remediation({"package": "zip", "first_patched": "9.0.5", "ecosystem": ecosystem,
                                   "manifests": ["doorway/doorway-service/Cargo.lock"]})
            self.assertIn("cargo update -p zip --precise 9.0.5", hint)
            self.assertNotIn("pnpm", hint)
        npm = dp.remediation({"package": "axios", "first_patched": "1.20.0", "ecosystem": "npm",
                              "manifests": ["pnpm-lock.yaml"]})
        self.assertIn("pnpm.overrides", npm)

    def test_missing_ledger_is_unmeasured(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output), patch.object(dp, "git", side_effect=AssertionError("must not read Git")), patch.dict(os.environ, {"SECURITY_PAYDOWN": "strict"}):
            self.assertEqual(cli.main(["--base", "base", "--head", "head", "--root", str(self.root)]), 0)
        self.assertEqual(output.getvalue(), "security-paydown: unmeasured (no ledger)\n")

    def test_off_is_silent_and_default_is_warn(self):
        code, text, tree = self.run_check(mode="off")
        self.assertEqual((code, text, tree.calls), (0, "", []))
        with patch.dict(os.environ, {}, clear=True):
            output = io.StringIO()
            fixture = FixtureRange({"crate/Cargo.lock": self.old}, {"crate/Cargo.lock": self.old}, ["crate/Cargo.lock"])
            with patch.object(dp, "git", side_effect=fixture.git), contextlib.redirect_stdout(output):
                self.assertEqual(cli.main(["--base", "base", "--head", "head", "--ledger", str(self.ledger)]), 0)
            self.assertIn("REFUSED", output.getvalue())

    def test_invalid_lock_refuses_and_ref_read_failure_is_unmeasured(self):
        for bad in ("not toml = [", "", "version = 4\n"):
            code, text, _ = self.run_check(new=bad)
            self.assertEqual(code, 1, text)
            self.assertIn("unmeasured", text)
        with patch.object(dp, "git", side_effect=ValueError("unreadable ref")), patch.dict(os.environ, {"SECURITY_PAYDOWN": "strict"}), contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(cli.main(["--base", "bad", "--head", "head", "--ledger", str(self.ledger)]), 1)
        self.assertIn("unmeasured", out.getvalue())

    def test_pre_push_wiring_is_before_rust_build(self):
        source = (ROOT / ".husky/pre-push.bash").read_text()
        self.assertIn('dependabot-paydown.py" --base "$STYLE_BASE" --head "$LOCAL_SHA"', source)
        self.assertLess(source.index('dependabot-paydown.py'), source.index('if ! bash "$(git rev-parse --show-toplevel)/genesis/orchestrator/rust-style.sh"'))


class VersionTests(unittest.TestCase):
    def test_semver_edges(self):
        cases = [
            ("1.10.0", "1.9.0", "<1.9.0", True, False),
            ("1.2.0-rc.2", "1.2.0-rc.10", "<1.2.0", False, True),
            ("1.2.0-rc.10", "1.2.0", "<1.2.0", False, True),
            ("1.2.0+build.1", "1.2.0", "<1.2.0", True, False),
            ("0.2.9", "0.3.0", "^0.2.3", False, True),
            ("0.3.0", "0.3.0", "^0.2.3", True, False),
            ("0.0.3", "0.0.4", "^0.0.3", False, True),
            ("0.0.4", "0.0.4", "^0.0.3", True, False),
            ("2.0.2", "2.0.3", ">=1.0.0 <1.1.0 || >=2.0.0 <2.0.3", False, True),
        ]
        states = dp.version_states([{"versions": [v], "patched": p, "range": r} for v, p, r, _, _ in cases])
        for case, state in zip(cases, states):
            with self.subTest(case=case):
                self.assertEqual((state["resolved"], bool(state["vulnerable"])), case[-2:])

    def test_invalid_semver_is_not_silently_clean(self):
        for version, patched, range_ in (("garbage", "1.2.0", "<1.2.0"), ("1.0.0", "1.2.0", "unknown")):
            with self.assertRaises(ValueError):
                dp.version_states([{"versions": [version], "patched": patched, "range": range_}])

    def test_pnpm_key_generations_scopes_and_duplicate_versions(self):
        for keys in (("/fixture/1.2.3", "/@scope/fixture/1.2.3_peer@1.0.0"),
                     ("/fixture@1.2.3", "/@scope/fixture@1.2.3(peer@1.0.0)"),
                     ("fixture@1.2.3", "@scope/fixture@1.2.3(patch_hash=abc)")):
            content = "lockfileVersion: '9.0'\npackages:\n" + "".join(f"  '{key}': {{}}\n" for key in keys)
            self.assertEqual(dp.parse_lock("pnpm-lock.yaml", content), {"fixture": {"1.2.3"}, "@scope/fixture": {"1.2.3"}})


if __name__ == "__main__":
    unittest.main(verbosity=2)
