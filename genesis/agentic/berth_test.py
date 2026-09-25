"""berth: capacity transitions across lease shapes, and the mesh lease as the dev berth's router
(lease class + ttl, measure refused with a declared override, ledger locking, overrun pain).

Run: python3 -m unittest genesis.agentic.berth_test
"""
import json
import os
import runpy
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

_NS = runpy.run_path(str(Path(__file__).parent / "bin/berth"))
Berth = _NS["Berth"]
BerthUsage = _NS["BerthUsage"]
render_status = _NS["render_status"]
MISSING_POLICY = "/nonexistent/pool-policy.json"
CLASSES = {"mesh": {"verify": {"default_ttl_s": 1800, "max_ttl_s": 3600},
                    "measure": {"allowed": False, "alternative": "just measure <scope>"},
                    "mesh": {"daemon": True}}}


class CapacityTransitionTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.berth = Berth(self.directory.name, now=lambda: 100, policy_path=MISSING_POLICY)
        for session in ("first", "second", "third"):
            self.berth.moor(session)
        self.berth.capacities["cargo"] = 1

    def holders(self):
        result = self.berth.who("cargo")
        rows = [result] if isinstance(result, dict) else result or []
        return {row["holder"] for row in rows}

    def test_capacity_increase_and_reduction_preserve_incumbents(self):
        self.assertTrue(self.berth.claim("cargo", "first")[0])
        original = dict(self.berth.leases()["cargo"])
        self.berth.capacities["cargo"] = 2
        self.assertTrue(self.berth.claim("cargo", "second")[0])
        self.assertFalse(self.berth.claim("cargo", "third")[0])
        self.assertEqual(self.holders(), {"first", "second"})
        self.assertEqual(self.berth.leases()["cargo"], original)
        self.berth.capacities["cargo"] = 1
        self.assertFalse(self.berth.claim("cargo", "third")[0])
        self.assertTrue(self.berth.claim("cargo", "second")[0])
        self.assertEqual(self.holders(), {"first", "second"})
        self.assertTrue(self.berth.release("cargo", "first"))
        self.assertFalse(self.berth.claim("cargo", "third")[0])
        self.assertTrue(self.berth.release("cargo", "second"))
        self.assertTrue(self.berth.claim("cargo", "third")[0])
        self.assertEqual(self.holders(), {"third"})

    def test_unmoor_releases_slot_without_removing_other_holder(self):
        self.berth.claim("cargo", "first")
        self.berth.capacities["cargo"] = 2
        self.berth.claim("cargo", "second")
        self.berth.unmoor("second")
        self.assertEqual(self.holders(), {"first"})
        self.assertTrue(self.berth.claim("cargo", "third")[0])

    def test_expired_legacy_holder_does_not_consume_a_slot(self):
        self.berth.claim("cargo", "first", ttl_s=1)
        self.berth.now = lambda: 102
        self.berth.capacities["cargo"] = 2
        self.assertTrue(self.berth.claim("cargo", "second")[0])
        self.assertTrue(self.berth.claim("cargo", "third")[0])
        self.assertEqual(self.holders(), {"second", "third"})



class FakeObservation:
    """Stands in for .claude/hooks/_observation: records emits, never spawns `epr`."""

    def __init__(self, available=True):
        self._available = available
        self.emits = []

    def available(self):
        return self._available

    def emit(self, measure, subject, value, *, reason, env=None, root=None):
        self.emits.append({"measure": measure, "subject": subject, "value": value, "reason": reason, "env": env})
        return True


class LeaseClassTest(unittest.TestCase):
    """The mesh lease routes: verify is bounded, measure is refused with its alternative."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.clock = [1000.0]
        self.obs = FakeObservation()
        self.berth = self.make()
        self.berth.classes = json.loads(json.dumps(CLASSES))
        for session in ("dev", "other"):
            self.berth.moor(session)

    def make(self):
        b = Berth(self.directory.name, now=lambda: self.clock[0], obs=self.obs, policy_path=MISSING_POLICY)
        b.classes = json.loads(json.dumps(CLASSES))
        return b

    def rows(self, kind=None):
        return [r for r in self.berth.ledger(1000) if kind is None or r["kind"] == kind]

    def test_policy_file_declares_the_mesh_classes_and_mooring_ttl(self):
        b = Berth(self.directory.name, now=lambda: 0)
        self.assertEqual(b.classes["mesh"]["verify"], {"default_ttl_s": 1800, "max_ttl_s": 3600})
        self.assertIs(b.classes["mesh"]["measure"]["allowed"], False)
        self.assertEqual(b.mooring_ttl_s, 10800)
        self.assertNotIn("cargo", b.classes)

    def test_defaults_on_mesh_not_cargo(self):
        ok, lease, _ = self.berth.claim("mesh", "dev")
        self.assertTrue(ok)
        self.assertEqual((lease["class"], lease["ttl_s"], lease["claimed_at"]), ("verify", 1800, 1000.0))
        ok, lease, _ = self.berth.claim("cargo", "dev")
        self.assertTrue(ok)
        self.assertEqual(lease["class"], "verify")
        self.assertIsNone(lease["ttl_s"])

    def test_measure_refused_exit_3_naming_the_alternative(self):
        ok, lease, reason = self.berth.claim("mesh", "dev", klass="measure", ttl_s=7200)
        self.assertFalse(ok)
        self.assertIsNone(lease)
        self.assertIn("measure-class work does not run on the dev berth", reason)
        self.assertIn("`just measure <scope>`", reason)
        self.assertIn("rung A: --on adam", reason)
        refuse = self.rows("refuse")[-1]
        self.assertEqual((refuse["reason"], refuse["class"]), ("class", "measure"))
        self.assertIsNone(self.berth.raw_lease("mesh"))
        self.assertEqual(self.obs.emits, [])

    def test_measure_refused_through_the_cli_with_exit_3(self):
        main = _NS["main"]
        env = {"BERTH_DIR": self.directory.name, "BERTH_SESSION": "dev"}
        old = {k: os.environ.get(k) for k in (*env, "MEASURE_ON_DEV_BERTH")}
        os.environ.update(env)
        os.environ.pop("MEASURE_ON_DEV_BERTH", None)
        try:
            self.assertEqual(main(["berth", "claim", "mesh", "--class", "measure", "--ttl", "600"]), 3)
            self.assertEqual(main(["berth", "claim", "mesh", "--class", "bogus"]), 2)
        finally:
            for k, v in old.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v

    def test_verify_ttl_past_the_cap_is_refused(self):
        ok, _, reason = self.berth.claim("mesh", "dev", klass="verify", ttl_s=7200)
        self.assertFalse(ok)
        self.assertIn("declare it measure", reason)
        self.assertEqual(self.rows("refuse")[-1]["reason"], "ttl")
        self.assertTrue(self.berth.claim("mesh", "dev", klass="verify", ttl_s=3600)[0])

    def test_unknown_class_is_a_usage_error(self):
        with self.assertRaises(BerthUsage):
            self.berth.claim("mesh", "dev", klass="profile")

    def test_live_verify_refuses_another_session(self):
        self.assertTrue(self.berth.claim("mesh", "dev")[0])
        ok, lease, reason = self.berth.claim("mesh", "other")
        self.assertFalse(ok)
        self.assertEqual(lease["holder"], "dev")
        self.assertIn("held by dev", reason)

    def test_expired_verify_renders_expired_and_is_taken_over(self):
        self.berth.claim("mesh", "dev", ttl_s=60, note="test mesh x.feature")
        self.clock[0] += 61
        self.berth.touch("dev")
        self.berth.touch("other")
        status = render_status(self.berth)
        self.assertIn("mesh        EXPIRED dev — held 61s of a 60s verify ttl (takeover on next claim)", status)
        ok, lease, reason = self.berth.claim("mesh", "other")
        self.assertTrue(ok)
        self.assertEqual(reason, "taken over from a stale holder")
        row = self.rows("claim")[-1]
        self.assertEqual((row["took_over_from"], row["expired_after_s"]), ("dev", 60))
        self.assertEqual(lease["claimed_at"], self.clock[0])

    def test_renew_resets_since_not_claimed_at_and_carries_the_seat(self):
        self.berth.claim("mesh", "dev", ttl_s=600)
        self.clock[0] += 300
        ok, lease, reason = self.berth.claim("mesh", "dev", seat="che-a")
        self.assertTrue(ok)
        self.assertEqual(reason, "renewed")
        stored = self.berth.raw_lease("mesh")
        self.assertEqual((stored["claimed_at"], stored["since"]), (1000.0, 1300.0))
        renew = self.rows("renew")[-1]
        self.assertEqual((renew["seat"], renew["claimed_at"], renew["class"]), ("che-a", 1000.0, "verify"))

    def test_renew_without_ttl_stops_at_the_class_cap(self):
        self.berth.claim("mesh", "dev", ttl_s=1800)
        self.clock[0] += 1500
        self.berth.touch("dev")
        self.assertTrue(self.berth.claim("mesh", "dev", ttl_s=1800)[0])
        self.clock[0] += 1500
        self.berth.touch("dev")
        self.assertTrue(self.berth.claim("mesh", "dev")[0])
        self.assertEqual(self.berth.raw_lease("mesh")["ttl_s"], 600)  # 1000 + 3600 - 4000
        self.clock[0] += 599
        self.berth.touch("dev")
        self.clock[0] += 2
        self.berth.touch("dev")
        # past the cap: the lease lapsed; a bare renew cannot resurrect the old hold
        ok, _, reason = self.berth.claim("mesh", "dev")
        self.assertTrue(ok)
        self.assertEqual(reason, "re-claimed after its own lease lapsed")

    def test_renew_refused_at_the_cap_without_explicit_ttl(self):
        self.berth.claim("mesh", "dev", ttl_s=3600)
        self.clock[0] += 3500
        self.berth.touch("dev")
        self.berth.claim("mesh", "dev", ttl_s=3600)  # explicit: capped at 3600 from now
        self.clock[0] += 200  # 3700 s since claimed_at, lease still live
        self.berth.touch("dev")
        ok, _, reason = self.berth.claim("mesh", "dev")
        self.assertFalse(ok)
        self.assertIn("reached its 3600s cap", reason)
        self.assertEqual(self.rows("refuse")[-1]["reason"], "max_ttl")
        self.assertFalse(self.berth.claim("mesh", "dev", ttl_s=99999)[0])  # an explicit ttl is itself capped
        ok, _, _ = self.berth.claim("mesh", "dev", ttl_s=3600)
        self.assertTrue(ok)
        self.assertEqual(self.berth.raw_lease("mesh")["ttl_s"], 3600)

    def test_override_writes_a_row_and_emits_exactly_once(self):
        ok, lease, reason = self.berth.claim("mesh", "dev", klass="measure", ttl_s=5400,
                                             note="K0 A/B window", override=True)
        self.assertTrue(ok)
        self.assertEqual(lease["class"], "measure")
        override = self.rows("override")
        self.assertEqual(len(override), 1)
        self.assertEqual((override[0]["holder"], override[0]["ttl_s"], override[0]["reason"]),
                         ("dev", 5400, "K0 A/B window"))
        self.assertEqual(len(self.obs.emits), 1)
        self.assertEqual((self.obs.emits[0]["measure"], self.obs.emits[0]["value"]),
                         ("dev-berth-held-by-measure@1", 5400))
        self.clock[0] += 6000
        self.assertIsNone(self.berth.overrun_check("mesh"))  # one pain per lease
        self.assertEqual(len(self.obs.emits), 1)

    def test_override_needs_a_declared_ttl(self):
        with self.assertRaises(BerthUsage):
            self.berth.claim("mesh", "dev", klass="measure", override=True)

    def test_overrun_check_emits_once_per_lease(self):
        self.berth.claim("mesh", "dev", ttl_s=1800)
        self.clock[0] += 1800
        self.assertIsNone(self.berth.overrun_check())
        self.clock[0] += 100
        self.assertEqual(self.berth.overrun_check(), 1900)
        self.assertEqual(self.obs.emits[-1]["value"], 1900)
        self.clock[0] += 100
        self.assertIsNone(self.berth.overrun_check())
        self.assertEqual(len(self.obs.emits), 1)
        self.assertEqual(len(self.rows("overrun")), 1)
        # a new lease is a new pain
        self.berth.release("mesh", "dev")
        self.berth.touch("dev")
        self.berth.claim("mesh", "dev", ttl_s=60)
        self.clock[0] += 61
        self.assertEqual(self.berth.overrun_check(), 61)
        self.assertEqual(len(self.obs.emits), 2)

    def test_overrun_check_never_emits_without_an_emitter(self):
        self.obs._available = False
        self.berth.claim("mesh", "dev", ttl_s=60)
        self.clock[0] += 120
        self.assertIsNone(self.berth.overrun_check())
        self.assertEqual(self.obs.emits, [])
        self.assertFalse(os.path.isdir(self.berth.emitted_dir) and os.listdir(self.berth.emitted_dir))
        self.obs._available = True  # the emitter returns: the unrecorded overrun lands then
        self.assertEqual(self.berth.overrun_check(), 120)


class LedgerSafetyTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.berth = Berth(self.directory.name, now=lambda: 5, policy_path=MISSING_POLICY)

    def test_ledger_tolerates_a_nul_prefixed_and_a_torn_line(self):
        self.berth.record("note", session="a", text="before")
        with open(self.berth.ledger_path, "ab") as f:
            f.write(b"\x00\x00\x00" + json.dumps({"at": 5, "kind": "note", "text": "padded"}).encode() + b"\n")
            f.write(b'{"at": 5, "kind": "no\n')
        self.berth.record("note", session="a", text="after")
        texts = [ev.get("text") for ev in self.berth.ledger(10)]
        self.assertEqual(texts, ["before", "padded", "after"])

    def test_forked_writers_never_interleave(self):
        writers, per = 6, 200
        pids = []
        for w in range(writers):
            pid = os.fork()
            if pid == 0:
                try:
                    b = Berth(self.directory.name, now=lambda: 5, policy_path=MISSING_POLICY)
                    for i in range(per):
                        b.record("note", session=f"w{w}", text=f"{w}:{i}:" + "x" * 512)
                        b.claim("cargo", f"w{w}") if i % 50 == 0 else None
                finally:
                    os._exit(0)
            pids.append(pid)
        for pid in pids:
            os.waitpid(pid, 0)
        with open(self.berth.ledger_path, "rb") as f:
            raw = f.read()
        self.assertNotIn(b"\x00", raw)
        lines = raw.decode().splitlines()
        parsed = [json.loads(line) for line in lines]  # every line whole
        notes = [ev for ev in parsed if ev["kind"] == "note"]
        self.assertEqual(len(notes), writers * per)
        with open(self.berth.leases_path) as f:
            json.load(f)


class AncestryResolutionTest(unittest.TestCase):
    """Runtime-asserted identity: a Claude session moors with the Claude pid; a shell it spawned
    (a Bash-tool command, `just test mesh`) resolves to that session through its process ancestry."""

    CLAUDE, OTHER_CLAUDE = 4242, 5151

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.clock = [1000.0]
        self.berth = Berth(self.directory.name, now=lambda: self.clock[0], policy_path=MISSING_POLICY)
        # fake pids: liveness by kill(0) would fail, so is_live consults last_seen only
        self.berth.is_live = lambda m: bool(m) and (self.clock[0] - m.get("last_seen", 0)) < m.get("ttl_s", 10800)

    def run_main(self, ancestry, *args):
        main = _NS["main"]
        g = main.__globals__
        saved = (g["proc_ancestry"], g["Berth"])
        berth = self.berth
        g["proc_ancestry"] = lambda: list(ancestry)
        g["Berth"] = lambda *a, **k: berth
        env_saved = {k: os.environ.pop(k, None) for k in ("BERTH_SESSION", "CLAUDE_SESSION_ID")}
        try:
            return main(["berth", *args])
        finally:
            g["proc_ancestry"], g["Berth"] = saved
            for k, v in env_saved.items():
                if v is not None:
                    os.environ[k] = v

    def test_moored_pid_is_an_ancestor_resolves(self):
        self.berth.moor("sess-claude", pid=self.CLAUDE)
        self.assertEqual(self.berth.session_by_ancestry([777, 778, self.CLAUDE, 3753]), "sess-claude")
        rc = self.run_main([777, 778, self.CLAUDE, 3753], "claim", "mesh", "--ttl", "60", "--note", "probe")
        self.assertEqual(rc, 0)
        row = [r for r in self.berth.ledger(100) if r["kind"] == "claim"][-1]
        self.assertEqual((row["session"], row["session_source"]), ("sess-claude", "ancestry"))

    def test_no_ancestor_mooring_is_exit_2(self):
        self.berth.moor("sess-claude", pid=self.CLAUDE)
        self.assertIsNone(self.berth.session_by_ancestry([777, 3753]))
        self.assertEqual(self.run_main([777, 3753], "claim", "mesh"), 4)
        self.assertIsNone(self.berth.raw_lease("mesh"))

    def test_the_ancestor_wins_not_the_newest(self):
        self.berth.moor("sess-mine", pid=self.CLAUDE)
        self.clock[0] += 50
        self.berth.moor("sess-newer", pid=self.OTHER_CLAUDE)
        self.assertEqual(self.berth.session_by_ancestry([777, self.CLAUDE, 1]), "sess-mine")

    def test_a_stale_ancestor_mooring_does_not_resolve(self):
        self.berth.moor("sess-claude", pid=self.CLAUDE)
        self.clock[0] += 20000
        self.assertIsNone(self.berth.session_by_ancestry([self.CLAUDE]))

    def test_explicit_env_wins_and_is_recorded(self):
        self.berth.moor("sess-claude", pid=self.CLAUDE)
        os.environ["BERTH_SESSION"] = "sess-env"
        try:
            main = _NS["main"]
            g = main.__globals__
            saved = g["Berth"]
            berth = self.berth
            g["Berth"] = lambda *a, **k: berth
            try:
                self.assertEqual(main(["berth", "claim", "mesh"]), 0)
            finally:
                g["Berth"] = saved
        finally:
            os.environ.pop("BERTH_SESSION", None)
        row = [r for r in self.berth.ledger(100) if r["kind"] == "claim"][-1]
        self.assertEqual((row["session"], row["session_source"]), ("sess-env", "env"))

    def test_real_proc_walk_reaches_this_process_parent(self):
        chain = _NS["proc_ancestry"]()
        if os.getppid() > 1:
            self.assertEqual(chain[0], os.getppid())
        self.assertLessEqual(len(chain), 32)



class DaemonClassTest(unittest.TestCase):
    """`mesh` is the daemon lease: no ttl, never taken over by expiry; lanes restricted; transitions explicit."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.clock = [1000.0]
        self.berth = Berth(self.directory.name, now=lambda: self.clock[0], obs=FakeObservation(),
                           policy_path=MISSING_POLICY)
        self.berth.classes = json.loads(json.dumps(CLASSES))
        for session in ("dev", "other"):
            self.berth.moor(session)

    def test_daemon_lease_takes_no_ttl(self):
        with self.assertRaises(BerthUsage):
            self.berth.claim("mesh", "dev", klass="mesh", ttl_s=1800)
        ok, lease, _ = self.berth.claim("mesh", "dev", klass="mesh")
        self.assertTrue(ok)
        self.assertIsNone(lease["ttl_s"])

    def test_daemon_lease_never_expires_while_its_holder_lives(self):
        self.berth.claim("mesh", "dev", klass="mesh")
        self.clock[0] += 100000
        self.berth.touch("dev")
        self.berth.touch("other")
        self.assertFalse(self.berth.claim("mesh", "other")[0])
        self.assertIsNone(self.berth.overrun_check())
        self.clock[0] += 20000  # dev's mooring goes stale (3 h ttl); other keeps heartbeating
        self.berth.touch("other")
        ok, _, reason = self.berth.claim("mesh", "other")
        self.assertTrue(ok)
        self.assertEqual(reason, "taken over from a stale holder")

    def test_a_lane_under_the_holders_own_daemon_lease_is_covered(self):
        self.berth.claim("mesh", "dev", klass="mesh")
        ok, _, reason = self.berth.claim("mesh", "dev", klass="verify", ttl_s=1800)
        self.assertTrue(ok)
        self.assertIn("renewed — covered by your mesh daemon lease", reason)
        self.assertEqual(self.berth.raw_lease("mesh")["class"], "mesh")
        self.assertIsNone(self.berth.raw_lease("mesh")["ttl_s"])

    def test_class_transition_on_a_held_lease_is_refused(self):
        self.berth.claim("mesh", "dev", klass="verify")
        ok, _, reason = self.berth.claim("mesh", "dev", klass="mesh")
        self.assertFalse(ok)
        self.assertIn("class transition", reason)
        ok, _, _ = self.berth.claim("mesh", "dev", klass="measure", ttl_s=600, override=True)
        self.assertFalse(ok)
        self.assertEqual(self.berth.raw_lease("mesh")["class"], "verify")
        self.assertTrue(self.berth.release("mesh", "dev"))
        self.assertTrue(self.berth.claim("mesh", "dev", klass="mesh")[0])

    def test_owner_check_refuses_a_stop_under_another_holder_and_force_is_on_the_record(self):
        self.berth.claim("mesh", "other", klass="mesh")
        ok, _, reason = self.berth.owner_check("mesh", "dev", "stop")
        self.assertFalse(ok)
        self.assertIn("held by other", reason)
        self.assertTrue(self.berth.owner_check("mesh", "other", "stop")[0])
        ok, _, _ = self.berth.owner_check("mesh", "dev", "stop", force=True, note="operator")
        self.assertTrue(ok)
        row = [r for r in self.berth.ledger(100) if r["kind"] == "override"][-1]
        self.assertEqual((row["action"], row["holder"]), ("stop", "other"))
        self.assertTrue(self.berth.release("mesh", "dev", force=True))
        self.assertIsNone(self.berth.raw_lease("mesh"))
        self.assertEqual([r for r in self.berth.ledger(100) if r["kind"] == "release"][-1]["forced_from"], "other")


class IncarnationTest(unittest.TestCase):
    """A pid alone is not an identity: the mooring carries the process's start time."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.starts = {4242: 111}
        self.berth = Berth(self.directory.name, now=lambda: 1000.0, policy_path=MISSING_POLICY)
        self.berth.pid_alive = lambda pid: int(pid) in self.starts
        self.berth.pid_start = lambda pid: self.starts.get(int(pid))

    def test_mooring_records_pid_start_and_a_reused_pid_is_dead(self):
        m = self.berth.moor("sess", pid=4242)
        self.assertEqual((m["pid"], m["pid_start"]), (4242, 111))
        self.assertTrue(self.berth.is_live(self.berth.mooring("sess")))
        self.assertEqual(self.berth.session_by_ancestry([9, 4242]), "sess")
        self.starts[4242] = 222  # the Claude process died; the pid now names another process
        self.assertFalse(self.berth.is_live(self.berth.mooring("sess")))
        self.assertIsNone(self.berth.session_by_ancestry([9, 4242]))
        self.berth.touch("sess", pid=4242)  # the hook re-asserts: new incarnation recorded
        self.assertTrue(self.berth.is_live(self.berth.mooring("sess")))

    def test_two_moorings_on_the_nearest_ancestor_are_ambiguous(self):
        self.berth.moor("a", pid=4242)
        self.berth.moor("b", pid=4242)
        with self.assertRaises(_NS["BerthAmbiguous"]):
            self.berth.session_by_ancestry([9, 4242])
        main = _NS["main"]
        g = main.__globals__
        saved = (g["proc_ancestry"], g["Berth"])
        berth = self.berth
        g["proc_ancestry"], g["Berth"] = (lambda: [9, 4242]), (lambda *a, **k: berth)
        env_saved = {k: os.environ.pop(k, None) for k in ("BERTH_SESSION", "CLAUDE_SESSION_ID")}
        try:
            self.assertEqual(main(["berth", "claim", "mesh"]), 4)
        finally:
            g["proc_ancestry"], g["Berth"] = saved
            for k, v in env_saved.items():
                if v is not None:
                    os.environ[k] = v
        self.assertIsNone(self.berth.raw_lease("mesh"))


class _FileObservation:
    """A cross-process fake emitter: every emit is one line in a file, slowly (widens the race)."""

    def __init__(self, path):
        self.path = path

    def available(self):
        return True

    def emit(self, measure, subject, value, *, reason, env=None, root=None):
        time.sleep(0.2)
        with open(self.path, "a") as f:
            f.write(f"{measure} {value}\n")
        return True


class ConcurrencyTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.dir = self.directory.name

    def _fork(self, fn):
        pid = os.fork()
        if pid == 0:
            try:
                fn()
            finally:
                os._exit(0)
        return pid

    def test_concurrent_heartbeats_never_read_as_a_dead_holder(self):
        holder = Berth(self.dir, policy_path=MISSING_POLICY)
        holder.moor("holder", pid=os.getpid())
        self.assertTrue(holder.claim("cargo", "holder", ttl_s=600)[0])
        from importlib.machinery import SourceFileLoader
        from importlib.util import module_from_spec, spec_from_file_location
        path = str(REPO / ".claude/hooks/ram-guard.py")
        spec = spec_from_file_location("rg_test", path, loader=SourceFileLoader("rg_test", path))
        ram_guard = module_from_spec(spec)
        spec.loader.exec_module(ram_guard)  # the hook's own heartbeat path, not a copy of it
        os.environ["BERTH_DIR"] = self.dir
        self.addCleanup(os.environ.pop, "BERTH_DIR", None)

        def beat():
            b = Berth(self.dir, policy_path=MISSING_POLICY)
            for i in range(400):
                b.touch("holder", pid=os.getppid())
                if i % 4 == 0:
                    ram_guard.berth_touch("holder")

        kids = [self._fork(beat) for _ in range(2)]
        rival = Berth(self.dir, policy_path=MISSING_POLICY)
        rival.moor("rival")
        outcomes = [rival.claim("cargo", "rival")[0] for _ in range(300)]
        for kid in kids:
            os.waitpid(kid, 0)
        self.assertFalse(any(outcomes), "a heartbeat mid-write read as a dead holder and was taken over")
        self.assertEqual(rival.leases()["cargo"]["holder"], "holder")

    def test_two_overrun_checks_emit_once(self):
        emits = os.path.join(self.dir, "emits.txt")
        seed = Berth(self.dir, policy_path=MISSING_POLICY)
        now = time.time()
        seed._write_leases({"mesh": {"resource": "mesh", "holder": "h", "since": now - 100,
                                     "claimed_at": now - 100, "ttl_s": 10, "class": "verify"}})
        kids = [self._fork(lambda: Berth(self.dir, obs=_FileObservation(emits),
                                         policy_path=MISSING_POLICY).overrun_check()) for _ in range(4)]
        for kid in kids:
            os.waitpid(kid, 0)
        with open(emits) as f:
            self.assertEqual(len(f.read().splitlines()), 1)


@unittest.skipUnless(shutil.which("just") and shutil.which("timeout"), "needs just + timeout")
class JustWrapperTest(unittest.TestCase):
    """The real justfile arms against a stub app_dir: refusals never reach the mesh, exit 4 proceeds,
    and a verify lane is killed at its ttl so a competing claimant can take the berth safely."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        root = Path(self.directory.name)
        self.berth_dir = root / "berth"
        self.app = root / "app"
        (self.app / "scripts").mkdir(parents=True)
        self.reached = root / "reached"
        self.lane_pid = root / "lane.pid"
        stub = f"""#!/usr/bin/env bash
# stub hc-mesh.sh: sourced by the test lane, executed by the mesh arms — records that it was reached
echo "$*" >> {self.reached}
if [[ "${{BASH_SOURCE[0]}}" != "$0" ]]; then
  if [[ "${{STUB_MODE:-}}" == "overlong" ]]; then
    mesh_seed_env() {{ echo "$BASHPID" > {self.lane_pid}; sleep 60; }}
  else
    exit 42
  fi
else
  exit 0
fi
"""
        for name in ("hc-mesh.sh", "hc-mesh-transport-matrix.sh"):
            (self.app / "scripts" / name).write_text(stub)
            (self.app / "scripts" / name).chmod(0o755)
        self.env = {k: v for k, v in os.environ.items() if k not in (
            "BERTH_SESSION", "CLAUDE_SESSION_ID", "BERTH_CLASS", "BERTH_TTL", "MEASURE_ON_DEV_BERTH",
            "MESH_STOP_FORCE", "BERTH_FENCED")}
        self.env.update({"BERTH_DIR": str(self.berth_dir), "EPR_BIN": "/nonexistent-epr",
                         "PATH": f"{REPO / 'genesis/agentic/bin'}:/usr/local/bin:/usr/bin:/bin",
                         "MESH_DIR": str(root / "no-mesh")})

    def just(self, *args, **env):
        e = dict(self.env, **env)
        return subprocess.run(["just", "--justfile", str(REPO / "justfile"), f"app_dir={self.app}", *args],
                              env=e, capture_output=True, text=True, timeout=120)

    def berth(self, *args, **env):
        return subprocess.run([str(REPO / "genesis/agentic/bin/berth"), *args], env=dict(self.env, **env),
                              capture_output=True, text=True, timeout=30)

    def test_refusals_never_reach_the_mesh(self):
        cases = [({"BERTH_CLASS": "measure"}, 3), ({"BERTH_CLASS": "bogus"}, 2), ({"BERTH_CLASS": "mesh"}, 2),
                 ({"BERTH_TTL": "0"}, 2), ({"BERTH_TTL": "7200"}, 3), ({"BERTH_TTL": "soon"}, 2)]
        for env, want in cases:
            r = self.just("test", "mesh", "x.feature", BERTH_SESSION="me", **env)
            self.assertEqual(r.returncode, want, (env, r.stderr[-400:]))
        self.assertFalse(self.reached.exists())

    def test_no_session_is_exit_4_and_the_lane_proceeds_unleased(self):
        self.assertEqual(self.berth("claim", "mesh", "--ttl", "60").returncode, 4)
        r = self.just("test", "mesh", "x.feature")
        self.assertEqual(r.returncode, 42, r.stderr[-400:])  # the stub was sourced: the lane ran
        self.assertTrue(self.reached.exists())

    def test_measure_recipes_are_refused_on_the_dev_berth(self):
        for action in ("matrix", "recovery", "recovery-matrix"):
            r = self.just("mesh", action, BERTH_SESSION="me")
            self.assertEqual(r.returncode, 3, (action, r.stderr[-300:]))
            self.assertIn("just measure <scope>", r.stderr)
        self.assertFalse(self.reached.exists())

    def test_stop_is_refused_under_another_live_holder_unless_forced(self):
        self.assertEqual(self.berth("claim", "mesh", "--class", "mesh", "--session", "rival").returncode, 0)
        r = self.just("mesh", "stop", BERTH_SESSION="me")
        self.assertEqual(r.returncode, 3, r.stderr[-300:])
        self.assertIn("held by rival", r.stderr)
        self.assertFalse(self.reached.exists())
        r = self.just("mesh", "stop", BERTH_SESSION="me", MESH_STOP_FORCE="1")
        self.assertEqual(r.returncode, 0, r.stderr[-300:])
        self.assertTrue(self.reached.exists())
        self.assertIsNone(json.loads(self.berth("who", "mesh").stdout))

    def test_an_overlong_verify_lane_is_killed_at_its_ttl_then_the_berth_is_free(self):
        proc = subprocess.Popen(["just", "--justfile", str(REPO / "justfile"), f"app_dir={self.app}",
                                 "test", "mesh", "x.feature"],
                                env=dict(self.env, BERTH_SESSION="me", BERTH_TTL="3", STUB_MODE="overlong"),
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        deadline = time.time() + 10
        while not self.lane_pid.exists() and time.time() < deadline:
            time.sleep(0.1)
        self.assertTrue(self.lane_pid.exists(), "the lane never started")
        rival = self.berth("claim", "mesh", "--session", "rival", "--ttl", "60")
        self.assertEqual(rival.returncode, 3, "a live verify lane must refuse a competing claimant")
        out, err = proc.communicate(timeout=60)
        self.assertEqual(proc.returncode, 3, err[-400:])
        self.assertIn("BUDGET-EXCEEDED: verify lane ran past its 3s lease", err)
        lane = int(self.lane_pid.read_text())
        self.assertFalse(os.path.exists(f"/proc/{lane}"), "the fenced lane outlived its lease")
        rival = self.berth("claim", "mesh", "--session", "rival", "--ttl", "60")
        self.assertEqual(rival.returncode, 0, rival.stderr)


if __name__ == "__main__":
    unittest.main()
