"""berth: capacity transitions across lease shapes, and the mesh lease as the dev berth's router
(lease class + ttl, measure refused with a declared override, ledger locking, overrun pain).

Run: python3 -m unittest genesis.agentic.berth_test
"""
import json
import os
import runpy
import tempfile
import unittest
from pathlib import Path

_NS = runpy.run_path(str(Path(__file__).parent / "bin/berth"))
Berth = _NS["Berth"]
BerthUsage = _NS["BerthUsage"]
render_status = _NS["render_status"]
MISSING_POLICY = "/nonexistent/pool-policy.json"
CLASSES = {"mesh": {"verify": {"default_ttl_s": 1800, "max_ttl_s": 3600},
                    "measure": {"allowed": False, "alternative": "just measure <scope>"},
                    "mesh": {"default_ttl_s": 1800}}}


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


if __name__ == "__main__":
    unittest.main()
