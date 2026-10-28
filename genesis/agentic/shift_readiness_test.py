"""Live readiness authority and finite clock tests with real contained descendants."""
import datetime as dt
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import shift
import shift_inputs as inputs
import shift_state as state


def iso(value):
    return dt.datetime.fromtimestamp(value, dt.timezone.utc).isoformat()


class ClockValidation(unittest.TestCase):
    def setUp(self):
        self.now = time.time()
        self.contract = {'started_at': iso(self.now - 10), 'deadline': iso(self.now + 100)}

    def test_missing_non_numeric_boolean_and_nonfinite_durable_floors_refuse(self):
        for invalid in [None, True, False, '100', float('nan'), float('inf'), -float('inf'), 10**1000]:
            with self.subTest(value=invalid), self.assertRaisesRegex(state.Refused, 'clock floor'):
                state.check_clock(self.contract, {'last_wall': invalid}, self.now)
        with self.assertRaisesRegex(state.Refused, 'clock floor'):
            state.check_clock(self.contract, {}, self.now)

    def test_current_clock_and_absolute_time_inputs_are_validated(self):
        for invalid in [True, '100', float('nan'), float('inf'), -float('inf'), 10**1000]:
            with self.subTest(value=invalid), self.assertRaisesRegex(state.Refused, 'current wall clock'):
                state.check_clock(self.contract, {'last_wall': self.now}, invalid)
        with patch.object(state.time, 'time', return_value=None), self.assertRaises(state.Refused):
            state.check_clock(self.contract, {'last_wall': self.now})
        for field in ['started_at', 'deadline']:
            for invalid in [None, True, float('nan'), 'NaN', '2026-10-22T00:00:00']:
                with self.subTest(field=field, value=invalid), self.assertRaisesRegex(state.Refused, 'absolute time'):
                    state.check_clock({**self.contract, field: invalid}, {'last_wall': self.now}, self.now)

    def test_valid_floor_preserves_deadline_and_refuses_regression(self):
        checkpoint = {'last_wall': self.now}
        remaining = state.check_clock(self.contract, checkpoint, self.now)
        self.assertAlmostEqual(remaining, 100, places=5)
        with self.assertRaisesRegex(state.Refused, 'clock regression'):
            state.check_clock(self.contract, checkpoint, self.now - 1)
        with self.assertRaisesRegex(state.Refused, 'deadline expired'):
            state.check_clock(self.contract, checkpoint, self.now + 101)
        with self.assertRaisesRegex(state.Refused, 'deadline must follow'):
            state.check_clock({**self.contract, 'deadline': self.contract['started_at']}, checkpoint, self.now)


class LiveReadiness(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='shift-readiness-live-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.directory = state.home(self.root, 'readiness-fixture')
        self.directory.mkdir(parents=True)
        self.objective = self.root / 'objective.json'
        state.atomic(self.objective, {'measure': {'run': 'fixture'}, 'budget': {'iterations': 3}})
        state.atomic(self.directory / 'objective.json', state.read(self.objective))
        self.spec = {'worktree': str(self.root), 'started_at': iso(time.time() - 1),
                     'deadline': iso(time.time() + 30), 'objective_path': self.objective.name,
                     'objective_sha256': state.digest(self.directory / 'objective.json'),
                     'authority': {'path': '.claude/shifts/readiness-fixture/authority.json',
                                   'revocation': '.claude/shifts/readiness-fixture/revocation.json'}}
        state.atomic(self.directory / 'revocation.json', {'revoked': False})
        self.freeze()

    def freeze(self):
        state.atomic(self.directory / 'authority.json', {'source': 'fixture original authorization',
                     'build': True, 'deadline': self.spec['deadline']})
        self.spec['authority']['sha256'] = state.digest(self.directory / 'authority.json')
        state.atomic(self.directory / 'initialization.json', {'version': 1, 'spec': self.spec,
                     'objective_source_sha256': state.digest(self.objective)})
        state.atomic(self.directory / 'initialization-checkpoint.json', {
            'version': 1, 'phase': 'preflight', 'attempt': 1, 'last_wall': time.time(),
            'initialization_sha256': state.digest(self.directory / 'initialization.json')})

    def test_stop_revocation_expiry_and_clock_corruption_kill_readiness_daemons(self):
        for reason in ['stop', 'revocation', 'expiry', 'clock', 'input']:
            with self.subTest(reason=reason):
                fixture = LiveReadiness(methodName='runTest')
                fixture.setUp()
                process = None
                try:
                    if reason == 'expiry':
                        fixture.spec['deadline'] = iso(time.time() + 2)
                        fixture.freeze()
                    if reason == 'input':
                        helper = fixture.directory / 'helper.py'
                        helper.write_text('print(1)\n')
                        fixture.spec['input_files'] = inputs.normalize_inputs(fixture.root, [helper.relative_to(fixture.root).as_posix()])
                        fixture.freeze()
                    frozen = (fixture.directory / 'initialization.json').read_bytes()
                    name = 'shift-readiness-' + str(time.time_ns())
                    finished = fixture.directory / 'completed-after-refusal'
                    daemon = '\n'.join([
                        'import socket,time,pathlib',
                        's=socket.socket(socket.AF_UNIX)',
                        f's.connect({chr(0) + name!r})',
                        "s.sendall(b'daemon')",
                        'time.sleep(30)',
                        f'pathlib.Path({str(finished)!r}).write_text("unauthorized continuation")',
                    ])
                    workload = '\n'.join([
                        'import subprocess,sys,time',
                        f'subprocess.Popen([sys.executable,"-c",{daemon!r}],start_new_session=True)',
                        'time.sleep(30)',
                    ])
                    runner = '\n'.join([
                        'import sys', 'from pathlib import Path', 'import shift',
                        f'directory=Path({str(fixture.directory)!r})',
                        'try:',
                        f' shift.bounded([sys.executable,"-c",{workload!r}], directory, 20, directory/"workload.log",',
                        '   check=lambda: shift.initialization_remaining(directory))',
                        'except shift.Refused as error:',
                        ' print(str(error), flush=True)', ' sys.exit(3)',
                    ])
                    with socket.socket(socket.AF_UNIX) as server:
                        server.bind('\0' + name)
                        server.listen(1)
                        server.settimeout(6)
                        process = subprocess.Popen([sys.executable, '-c', runner],
                                                   cwd=Path(__file__).parent, stdout=subprocess.PIPE,
                                                   stderr=subprocess.STDOUT, text=True)
                        connection, _ = server.accept()
                        with connection:
                            host_pid = struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[0]
                            handle = state.incarnation(host_pid)
                            self.assertEqual(connection.recv(20), b'daemon')
                        start = time.monotonic()
                        if reason == 'stop':
                            self.assertTrue(shift.stop(fixture.directory)['stopped'])
                        elif reason == 'revocation':
                            state.atomic(fixture.directory / 'revocation.json', {'revoked': True})
                        elif reason == 'clock':
                            # Serialize fault injection with the controller's floor
                            # update, so its in-flight write cannot replace our fault.
                            with state.lock(fixture.directory / 'state.lock'):
                                checkpoint = state.read(fixture.directory / 'initialization-checkpoint.json')
                                checkpoint['last_wall'] = float('nan')
                                state.atomic(fixture.directory / 'initialization-checkpoint.json', checkpoint)
                        elif reason == 'input':
                            helper.write_text('print(0)\n')
                        output, _ = process.communicate(timeout=5)
                        self.assertEqual(process.returncode, 3, output)
                        self.assertLess(time.monotonic() - start, 4)
                        self.assertRegex(output, 'stop marker|revoked|deadline expired|corrupt frozen initialization|private execution input changed')
                        until = time.monotonic() + 2
                        while state.alive(handle) and time.monotonic() < until:
                            time.sleep(0.02)
                        self.assertFalse(state.alive(handle), f'{reason}: daemon survived readiness refusal')
                        self.assertFalse(finished.exists())
                        self.assertEqual((fixture.directory / 'initialization.json').read_bytes(), frozen)
                finally:
                    if process is not None and process.poll() is None:
                        process.kill()
                        process.communicate(timeout=5)
                    fixture.doCleanups()

    def test_refused_authority_prevents_spawn(self):
        shift.stop(self.directory)
        with patch.object(shift, 'spawn_owned') as spawn:
            with self.assertRaisesRegex(state.Refused, 'stop marker'):
                shift.bounded(['true'], self.root, 10, self.directory / 'never.log',
                              check=lambda: shift.initialization_remaining(self.directory))
        spawn.assert_not_called()

    def test_polling_never_renews_original_monotonic_timeout(self):
        started = time.monotonic()
        with self.assertRaisesRegex(state.Refused, 'exceeded remaining time'):
            shift.bounded([sys.executable, '-c', 'import time;time.sleep(30)'],
                          self.root, 0.3, self.directory / 'timeout.log', check=lambda: 3600)
        self.assertLess(time.monotonic() - started, 3)

    def test_boolean_or_missing_initialization_floor_refuses(self):
        path = self.directory / 'initialization-checkpoint.json'
        original = state.read(path)
        for value in [True, False, None]:
            with self.subTest(value=value):
                state.atomic(path, {**original, 'last_wall': value})
                with self.assertRaisesRegex(state.Refused, 'corrupt frozen initialization'):
                    shift.initialization_remaining(self.directory)


if __name__ == '__main__':
    unittest.main()
