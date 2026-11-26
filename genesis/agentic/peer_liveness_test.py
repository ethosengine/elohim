"""peer-liveness: a stalled exchange is restarted once, by pid, as the daemon runs; a dry run does nothing.

Run (from the repository root): python3 -m unittest genesis/agentic/peer_liveness_test.py
Every test uses a fake status and a fake process table; none reads this device's daemon.
"""
import contextlib
import json
import os
import runpy
import signal
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
P = runpy.run_path(str(HERE / "bin/peer-liveness"))
main = P["main"]

LAPTOP = "did:key:z6MkgHjuD5WZ3JFJFv51NZwN61LYaDowFfm3REjiqSuYxMbu"
ROOT_DEVICE = "did:key:z6MkpZRhMwWJpv7HeZyArTUkbnG6DUyukQp18MxUs2fHVMZB"
BTIME = 1_793_000_000
TICK = os.sysconf("SC_CLK_TCK")
NOW = BTIME + 100_000.0
PID = 4242


def status(laptop_age=7, laptop_seq=24455, root_age=7):
    return {"ownSeq": 95826, "devices": [
        {"device": LAPTOP, "seq": laptop_seq, "ageS": laptop_age, "acked": 95826},
        {"device": ROOT_DEVICE, "seq": 92711, "ageS": root_age, "acked": 95826},
    ]}


class Fixture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        base = Path(self.tmp.name)
        self.root = base / "device"
        self.root.mkdir()
        self.proc = base / "proc"
        self.proc.mkdir()
        (self.proc / "stat").write_text(f"cpu 1 2 3\nbtime {BTIME}\n")
        self.exe = base / "store" / "epr"
        self.exe.parent.mkdir()
        self.exe.write_text("#!/bin/sh\n")
        self.exe.chmod(0o755)
        self.serve_log = base / "serve.log"
        self.serve_log.write_text("earlier line\n")
        self.lines = []
        self.signals = []
        self.spawned = []
        self.sleeps = []
        self.mono = [0.0]
        self.statuses = []

    def process(self, pid, argv, up_s=50_000, env=None, cwd=None):
        entry = self.proc / str(pid)
        (entry / "fd").mkdir(parents=True)
        (entry / "cmdline").write_bytes(b"\0".join(a.encode() for a in argv) + b"\0")
        env = {"EPR_PEER_RELAYS": "https://relay.example", "XDG_DATA_HOME": "/nix/xdg/data", "HOME": "/h"} if env is None else env
        (entry / "environ").write_bytes(b"\0".join(f"{k}={v}".encode() for k, v in env.items()) + b"\0")
        ticks = int((NOW - up_s - BTIME) * TICK)
        (entry / "stat").write_text(f"{pid} (epr peer) S 1 {pid} {pid} 0 -1 4194560 1 0 0 0 1 1 0 0 20 0 9 0 {ticks} 1 1\n")
        os.symlink(cwd or str(self.root), entry / "cwd")
        os.symlink(str(self.serve_log), entry / "fd" / "1")
        os.symlink(str(self.serve_log), entry / "fd" / "2")
        return entry

    def daemon(self, pid=PID, **kw):
        return self.process(pid, [str(self.exe), "peer", "serve", "--root", str(self.root),
                                  "--handle", "matthew", "--listen", "0.0.0.0:7777"], **kw)

    def kill(self, pid, sig):
        self.signals.append((pid, sig))
        entry = self.proc / str(pid)
        for path in sorted(entry.rglob("*"), reverse=True):
            path.unlink() if path.is_symlink() or path.is_file() else path.rmdir()
        entry.rmdir()

    def spawn(self, argv, **kw):
        self.spawned.append({"argv": argv, **{k: kw[k] for k in ("cwd", "env", "start_new_session")},
                             "stdout": getattr(kw["stdout"], "name", kw["stdout"])})
        kw["stdout"].write(b"restarted\n")
        return type("Child", (), {"pid": 5151})()

    def sleep(self, seconds):
        self.sleeps.append(seconds)
        self.mono[0] += seconds

    def run_main(self, *argv, statuses):
        self.statuses = list(statuses)

        def status_of():
            value = self.statuses.pop(0) if len(self.statuses) > 1 else self.statuses[0]
            if isinstance(value, Exception):
                raise value
            return value
        return main(["--root", str(self.root), *argv], out=self.lines.append, proc=str(self.proc), now=lambda: NOW,
                    status_of=status_of, kill=self.kill, spawn=self.spawn, sleep=self.sleep, clock=lambda: self.mono[0])

    def log_rows(self):
        log = self.root / "genesis/a2o/reports/recovery/peer-liveness.jsonl"
        return [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []


class PeerLiveness(Fixture):
    def test_all_well_is_silent_and_exit_zero(self):
        self.daemon()
        for flags in ((), ("--apply",)):
            self.assertEqual(self.run_main(*flags, statuses=[status()]), 0)
        self.assertEqual(self.lines, [])
        self.assertEqual((self.signals, self.spawned, self.log_rows()), ([], [], []))

    def test_dry_run_names_the_stall_and_the_restart_and_touches_nothing(self):
        self.daemon()
        self.assertEqual(self.run_main(statuses=[status(laptop_age=8540)]), 3)
        self.assertEqual(self.lines, [
            "[peer-liveness] no head folded within 900 s from did:key:z6MkgHju…xMbu (8540 s); would restart this device's "
            f"peer daemon (pid {PID}) as it runs (dry run: --apply does it)",
            f"[peer-liveness]   argv: {self.exe} peer serve --root {self.root} --handle matthew --listen 0.0.0.0:7777",
            f"[peer-liveness]   cwd {self.root}  EPR_PEER_RELAYS=https://relay.example  output {self.serve_log}",
        ])
        self.assertEqual((self.signals, self.spawned, self.log_rows()), ([], [], []))
        self.assertTrue((self.proc / str(PID)).exists())
        self.assertFalse((self.root / "genesis").exists(), "a dry run writes no file")

    def test_apply_restarts_by_pid_as_it_ran_logs_one_line_and_says_the_head_moved(self):
        self.daemon()
        code = self.run_main("--apply", statuses=[status(laptop_age=8540), status(laptop_age=8545), status(laptop_age=3, laptop_seq=24460)])
        self.assertEqual(code, 0)
        self.assertEqual(self.signals, [(PID, signal.SIGTERM)], "one signal, to the numeric pid, never SIGKILL")
        self.assertEqual(len(self.spawned), 1)
        child = self.spawned[0]
        self.assertEqual(child["argv"], [str(self.exe), "peer", "serve", "--root", str(self.root),
                                         "--handle", "matthew", "--listen", "0.0.0.0:7777"])
        self.assertEqual(child["env"], {"EPR_PEER_RELAYS": "https://relay.example", "XDG_DATA_HOME": "/nix/xdg/data", "HOME": "/h"},
                         "the daemon's own environment, not the caller's")
        self.assertEqual((child["cwd"], child["stdout"], child["start_new_session"]), (str(self.root), str(self.serve_log), True))
        self.assertEqual(self.serve_log.read_text(), "earlier line\nrestarted\n", "its output file is appended to")
        self.assertEqual(self.lines[-1], "[peer-liveness] restarted as pid 5151; head moved for did:key:z6MkgHju…xMbu")
        rows = self.log_rows()
        self.assertEqual(len(rows), 1)
        self.assertEqual((rows[0]["outcome"], rows[0]["oldPid"], rows[0]["newPid"], rows[0]["moved"]), ("moved", PID, 5151, [LAPTOP]))
        self.assertEqual(self.sleeps, [5, 5], "the status is re-read every 5 s until a head moves")

    def test_a_restart_that_moves_nothing_says_so_and_is_not_repeated_for_the_same_stall(self):
        self.daemon()
        self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=8540)]), 4)
        self.assertRegex(self.lines[-1], r"restarted as pid 5151; no stale head moved within 60 s: .* not retried until a head moves$")
        self.assertEqual(sum(self.sleeps), 60, "the settle wait is bounded at 60 s")
        self.assertEqual(self.log_rows()[0]["outcome"], "not-moved")
        # Two hours on, past the interval, the same head sequence: no second restart.
        global NOW
        self.daemon(pid=5151, up_s=-7200 + 7200)
        before, NOW = NOW, NOW + 7200
        try:
            self.lines.clear()
            self.signals.clear()
            self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=15740)]), 3)
            self.assertRegex(self.lines[0], r"a restart 7200 s ago moved no head and none has moved since: the sibling is likely off")
            self.assertEqual((self.signals, len(self.spawned), len(self.log_rows())), ([], 1, 1))
            # The head moved and went stale again: a new stall, a new restart is allowed.
            self.assertEqual(self.run_main(statuses=[status(laptop_age=1000, laptop_seq=24999)]), 3)
            self.assertRegex(self.lines[-3], r"would restart this device's peer daemon \(pid 5151\)")
        finally:
            NOW = before

    def test_at_most_one_restart_per_interval(self):
        self.daemon()
        log = self.root / "genesis/a2o/reports/recovery/peer-liveness.jsonl"
        log.parent.mkdir(parents=True)
        log.write_text(json.dumps({"at": NOW - 1200, "outcome": "moved", "stale": []}) + "\n")
        self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=1000)]), 3)
        self.assertRegex(self.lines[0], r"this daemon was restarted 1200 s ago \(at most one per 3600 s\)$")
        self.assertEqual((self.signals, self.spawned, len(self.log_rows())), ([], [], 1))

    def test_a_daemon_younger_than_the_bound_is_given_time(self):
        self.daemon(up_s=120)
        self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=None)]), 3)
        self.assertRegex(self.lines[0], r"\(never folded\); this daemon \(pid 4242\) has been up 120 s, under the bound: not yet$")
        self.assertEqual((self.signals, self.spawned), ([], []))

    def test_none_or_two_daemons_or_another_root_is_said_and_nothing_is_signalled(self):
        self.process(77, ["/store/epr/x/epr", "peer", "serve", "--root", "/some/other/checkout"])
        self.process(78, ["/usr/bin/python3", "epr", "peer", "serve", "--root", str(self.root)])
        self.process(79, ["/store/epr/x/epr", "peer", "status", "--root", str(self.root)])
        self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=8540)]), 3)
        self.assertRegex(self.lines[-1], r"no peer daemon of this user serves this root: nothing to restart safely$")
        self.daemon(pid=10)
        self.daemon(pid=11)
        self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=8540)]), 3)
        self.assertRegex(self.lines[-1], r"2 peer daemons serve this root \(pids 10, 11\): nothing to restart safely$")
        self.assertEqual((self.signals, self.spawned, self.log_rows()), ([], [], []))

    def test_a_daemon_that_ignores_sigterm_is_left_running_and_nothing_is_started(self):
        self.daemon()
        self.kill = lambda pid, sig: self.signals.append((pid, sig))
        self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=8540)]), 3)
        self.assertEqual(self.signals, [(PID, signal.SIGTERM)])
        self.assertEqual(self.spawned, [])
        self.assertRegex(self.lines[-1], r"pid 4242 did not exit within 10 s of SIGTERM; left running, nothing started$")
        self.assertEqual(self.log_rows()[0]["outcome"], "not-restarted")
        self.assertLessEqual(sum(self.sleeps), 10.4)

    def test_a_reused_pid_is_never_signalled(self):
        self.daemon()
        captured = P["capture"](str(self.proc), P["find_daemons"](str(self.proc), self.root)[0])
        (self.proc / str(PID) / "stat").write_text(f"{PID} (bash) S 1 1 1 0 -1 0 1 0 0 0 1 1 0 0 20 0 1 0 999999999 1 1\n")
        result = P["restart"](str(self.proc), captured, kill=self.kill, spawn=self.spawn, sleep=self.sleep, clock=lambda: self.mono[0])
        self.assertEqual(result, {"error": f"pid {PID} is no longer the daemon that was read; nothing signalled"})
        self.assertEqual((self.signals, self.spawned), ([], []))

    def test_an_unreadable_status_is_one_line_and_does_nothing(self):
        self.daemon()
        self.assertEqual(self.run_main("--apply", statuses=[RuntimeError("no answer within 5 s")]), 1)
        self.assertEqual(self.lines, ["[peer-liveness] `epr peer status` could not be read (no answer within 5 s): "
                                      "the exchange is not known; nothing done"])
        self.assertEqual((self.signals, self.spawned, self.log_rows()), ([], [], []))

    def test_the_status_read_is_bounded_and_goes_through_the_named_epr(self):
        seen = {}

        def run(argv, **kw):
            seen.update(argv=argv, timeout=kw["timeout"])
            return type("R", (), {"returncode": 0, "stdout": json.dumps(status()), "stderr": ""})()
        self.assertEqual(P["read_status"]("/head/epr", "/device", run=run)["ownSeq"], 95826)
        self.assertEqual(seen, {"argv": ["/head/epr", "peer", "status", "--root", "/device", "--json"], "timeout": 5})

    def test_a_daemon_that_could_not_be_started_again_is_never_signalled(self):
        """The bin-store head moves under a running daemon: stopping it would leave no daemon."""
        cases = []
        self.daemon()
        self.exe.unlink()
        cases.append(r"its executable .*/store/epr is gone or not executable")
        for expect in list(cases) + ["cwd", "output"]:
            if expect == "cwd":
                self.exe.write_text("#!/bin/sh\n"); self.exe.chmod(0o755)
                os.unlink(self.proc / str(PID) / "cwd")
                os.symlink(str(self.root) + "/gone (deleted)", self.proc / str(PID) / "cwd")
                expect = r"its working directory .*gone \(deleted\) is not a directory"
            elif expect == "output":
                os.unlink(self.proc / str(PID) / "cwd"); os.symlink(str(self.root), self.proc / str(PID) / "cwd")
                for fd in ("1", "2"):
                    os.unlink(self.proc / str(PID) / "fd" / fd)
                    os.symlink("/nonexistent-dir/serve.log", self.proc / str(PID) / "fd" / fd)
                expect = r"its output directory /nonexistent-dir is not writable"
            for flags in ((), ("--apply",)):
                self.lines.clear()
                self.assertEqual(self.run_main(*flags, statuses=[status(laptop_age=8540)]), 5)
                self.assertEqual(len(self.lines), 1)
                self.assertRegex(self.lines[0], r"could not be started again: " + expect + r".*; nothing signalled")
        self.assertEqual((self.signals, self.spawned, self.log_rows()), ([], [], []))
        self.assertTrue((self.proc / str(PID)).exists())
        # The last check stands again inside restart(), after the plan and just before the signal.
        captured = P["capture"](str(self.proc), P["find_daemons"](str(self.proc), self.root)[0])
        result = P["restart"](str(self.proc), captured, kill=self.kill, spawn=self.spawn, sleep=self.sleep, clock=lambda: self.mono[0])
        self.assertTrue(result.get("unsafe"))
        self.assertEqual(self.signals, [])

    def test_an_ignored_sigterm_counts_toward_the_interval(self):
        self.daemon()
        self.kill = lambda pid, sig: self.signals.append((pid, sig))
        self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=8540)]), 3)
        self.lines.clear()
        self.assertEqual(self.run_main("--apply", statuses=[status(laptop_age=8540)]), 3)
        self.assertEqual(self.signals, [(PID, signal.SIGTERM)], "no second SIGTERM inside the interval")
        self.assertRegex(self.lines[0], r"a restart was attempted and failed 0 s ago \(at most one per 3600 s\)$")
        self.assertEqual(len(self.log_rows()), 1)


if __name__ == "__main__":
    unittest.main()
