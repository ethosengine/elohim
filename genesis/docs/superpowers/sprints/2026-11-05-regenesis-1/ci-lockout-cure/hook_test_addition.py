# Add to class CiBaseline in genesis/orchestrator/rust-style/hook_test.py
# (same helpers: install_wrapper, tempfile, subprocess, os, json, Path).

    def test_integration_baseline_moves_forward_to_the_method_bootstrap(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = install_wrapper(root, "rust-style-ci.sh")
            policy = root / ".epr-meta/rust-style-policy.json"
            policy.parent.mkdir()
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, text=True).strip()
            git("init", "-q")
            def commit(method):
                policy.write_text(json.dumps({"discipline": {"method": {"cid": method}}}))
                git("add", ".epr-meta/rust-style-policy.json")
                git("-c", "user.name=Style fixture", "-c", "user.email=style@example.invalid",
                    "commit", "-qm", "fixture", "--allow-empty")
                return git("rev-parse", "HEAD")
            base = commit("bafkrei-old-method")
            bootstrap = commit("bafkrei-new-method")
            later = commit("bafkrei-new-method")
            commit("bafkrei-new-method")
            def selected(previous):
                env = {**os.environ, "BRANCH_NAME": "dev", "CHANGE_TARGET": "", "GIT_PREVIOUS_SUCCESSFUL_COMMIT": previous}
                return subprocess.run(["bash", str(script), "--print-base"], cwd=root, env=env, text=True, capture_output=True)
            moved = selected(base)
            self.assertEqual(moved.returncode, 0, moved.stderr)
            self.assertEqual(moved.stdout.strip(), bootstrap)
            self.assertIn("Rust style baseline moved", moved.stderr)
            self.assertIn(bootstrap, moved.stderr)
            kept = selected(later)
            self.assertEqual(kept.stdout.strip(), later)
            self.assertNotIn("baseline moved", kept.stderr)
            # A HEAD that pins no method keeps the trusted baseline as it is.
            git("rm", "-q", ".epr-meta/rust-style-policy.json")
            git("-c", "user.name=Style fixture", "-c", "user.email=style@example.invalid", "commit", "-qm", "no policy")
            plain = selected(base)
            self.assertEqual(plain.stdout.strip(), base)
            self.assertNotIn("baseline moved", plain.stderr)
