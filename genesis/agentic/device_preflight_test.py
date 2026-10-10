"""device-preflight: one line per arrival condition, REFUSED names the fix, exit 1 on any REFUSED.

Run: python3 -m unittest genesis.agentic.device_preflight_test
"""
import json
import os
import runpy
import stat
import sys
import tempfile
import unittest
from pathlib import Path

_NS = runpy.run_path(str(Path(__file__).parent / "bin/device-preflight"))
preflight = _NS["preflight"]
expand_resolve_entry = _NS["expand_resolve_entry"]
check_fold = _NS["check_fold"]
check_roster = _NS["check_roster"]

MANIFEST_REL = _NS["MODEL_MANIFEST_REL"]
MEASURE_REL = _NS["INDEX_MEASURE_REL"]
PARTICIPANTS_REL = _NS["PARTICIPANTS_REL"]


def fake_epr(bin_dir: Path, current: dict, status: dict) -> str:
    """A stand-in `epr` answering `actor current --device --json` and `flow memory index status --json`."""
    script = bin_dir / "epr"
    script.write_text(
        "#!/usr/bin/env python3\n"
        "import json, sys\n"
        f"CURRENT = {json.dumps(current)!r}\n"
        f"STATUS = {json.dumps(status)!r}\n"
        "a = sys.argv[1:]\n"
        "if a[:2] == ['actor', 'current']: print(CURRENT)\n"
        "elif a[:4] == ['flow', 'memory', 'index', 'status']: print(STATUS)\n"
        "else: sys.exit(2)\n"
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    return str(script)


class ArrivalTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name) / "repo"
        (self.root / PARTICIPANTS_REL).mkdir(parents=True)
        (self.root / PARTICIPANTS_REL / "matthew.jsonl").write_text("{}\n")
        (self.root / MANIFEST_REL).parent.mkdir(parents=True)
        self.model_dir = Path(self.tmp.name) / "models"
        (self.root / MANIFEST_REL).write_text(json.dumps({"resolve": ["$EPR_EMBED_MODEL_DIR", "~/.cache/none"]}))
        (self.root / MEASURE_REL).write_text(json.dumps({"foldLag": {"limit": 25}}))
        self.config = Path(self.tmp.name) / "config"
        (self.config / "berth" / "moorings").mkdir(parents=True)
        self.bin = Path(self.tmp.name) / "bin"
        self.bin.mkdir()
        self.key = Path(self.tmp.name) / "device" / "ed25519.seed"

    def env(self, **extra):
        base = {
            # The fake `epr` is a python3 script: keep the real PATH behind the fixture bin so
            # `/usr/bin/env python3` resolves while `epr` still resolves to the fake first.
            "PATH": f"{self.bin}{os.pathsep}{os.path.dirname(sys.executable)}",
            # The pool glob must not find the workspace's real `epr` when the test removes the fake.
            "CARGO_TARGET_POOL_ROOT": str(Path(self.tmp.name) / "no-pool"),
            "HOME": str(Path(self.tmp.name) / "home"),
            "CLAUDE_CONFIG_DIR": str(self.config),
            "ELOHIM_DEVICE_KEY_FILE": str(self.key),
            "EPR_EMBED_MODEL_DIR": str(self.model_dir),
        }
        base.update(extra)
        return base

    def arrived(self):
        self.key.parent.mkdir(parents=True, exist_ok=True)
        self.key.write_bytes(b"\0" * 32)
        self.model_dir.mkdir(parents=True, exist_ok=True)
        (self.model_dir / "model.onnx").write_bytes(b"m")
        (self.model_dir / "tokenizer.json").write_text("{}")
        (self.config / "berth" / "moorings" / "sess.json").write_text("{}")
        fake_epr(
            self.bin,
            {"standing": {"subject": "matthew", "via": "bound"}},
            {"lag": 3, "last": {"state": {"state": "complete"}}},
        )

    def test_every_condition_met_prints_six_ok_lines(self):
        self.arrived()
        lines = preflight(self.root, self.env(), None, "sess")
        self.assertEqual([l.name for l in lines], ["epr binary", "device key", "roster bound", "embed model", "fold attested", "berth moored"])
        self.assertTrue(all(l.ok for l in lines), [l.render() for l in lines])
        self.assertIn("human:matthew", lines[2].render())

    def test_no_key_refuses_and_names_enroll(self):
        self.arrived()
        self.key.unlink()
        line = preflight(self.root, self.env(), None, "sess")[1]
        self.assertFalse(line.ok)
        self.assertIn("epr actor device enroll --handle matthew", line.render())

    def test_unbound_device_names_the_three_commands(self):
        self.arrived()
        fake_epr(self.bin, {"standing": None}, {"lag": 0, "last": {"state": {"state": "complete"}}})
        line = preflight(self.root, self.env(), None, "sess")[2]
        self.assertFalse(line.ok)
        for verb in ("enroll", "authorize", "bind"):
            self.assertIn(verb, line.render())

    def test_pending_enrollment_names_authorize_then_bind(self):
        self.arrived()
        fake_epr(self.bin, {"standing": None}, {"lag": 0, "last": {"state": {"state": "complete"}}})
        (self.root / PARTICIPANTS_REL / ".enroll-matthew.json").write_text(json.dumps({"controller": "did:key:z6Mkabcdefgh"}))
        line = preflight(self.root, self.env(), None, "sess")[2]
        self.assertIn("pending", line.render())
        self.assertIn("authorize", line.render())
        self.assertIn("matthew.jsonl", line.render())

    def test_missing_model_names_the_provisioner(self):
        self.arrived()
        (self.model_dir / "model.onnx").unlink()
        line = preflight(self.root, self.env(), None, "sess")[3]
        self.assertFalse(line.ok)
        self.assertIn("embed-model-provision", line.render())

    def test_failed_fold_names_why_and_the_model_line(self):
        self.arrived()
        fake_epr(self.bin, {"standing": {"subject": "matthew"}}, {"lag": 2942, "last": {"state": {"state": "failed", "why": "unavailable: no model directory resolves"}}})
        line = preflight(self.root, self.env(), None, "sess")[4]
        self.assertFalse(line.ok)
        self.assertIn("no model directory resolves", line.render())
        self.assertIn("embed model", line.render())

    def test_lag_past_limit_names_a_catch_up_fold(self):
        self.arrived()
        fake_epr(self.bin, {"standing": {"subject": "matthew"}}, {"lag": 400, "last": {"state": {"state": "degraded", "retried": 0}}})
        line = preflight(self.root, self.env(), None, "sess")[4]
        self.assertFalse(line.ok)
        self.assertIn("--max-files 450", line.render())

    def test_no_epr_binary_refuses_the_dependent_reads_too(self):
        self.arrived()
        (self.bin / "epr").unlink()
        lines = preflight(self.root, self.env(), None, "sess")
        self.assertFalse(lines[0].ok)
        self.assertFalse(lines[2].ok)
        self.assertFalse(lines[4].ok)
        self.assertIn("epr binary", lines[2].render())

    def test_unmoored_session_names_berth_moor(self):
        self.arrived()
        line = preflight(self.root, self.env(), None, "other")[5]
        self.assertFalse(line.ok)
        self.assertIn("berth moor --session other", line.render())

    def test_resolve_entry_expansion_mirrors_the_embedder(self):
        env = {"HOME": "/h", "SET": "/s"}
        self.assertEqual(expand_resolve_entry("$SET/rest", env), Path("/s/rest"))
        self.assertIsNone(expand_resolve_entry("$UNSET/rest", env))
        self.assertEqual(expand_resolve_entry("~/.cache/x", env), Path("/h/.cache/x"))
        self.assertEqual(expand_resolve_entry("/abs", env), Path("/abs"))


if __name__ == "__main__":
    unittest.main()
