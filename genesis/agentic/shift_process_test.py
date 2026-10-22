"""Fault injection through real ark, Linux namespaces and host process incarnations.

The runtime executables are deterministic stand-ins; separate launch canaries
exercise the authenticated Claude/Codex CLIs under their actual approval policy.
"""
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import struct
import subprocess
import sys
import time
import unittest

import shift_test
from shift_test import iso
import shift_state as state
import shift_supervisor as supervisor


class ProcessRecovery(unittest.TestCase):
    def setUp(self):
        self.fixture = shift_test.ContractFixture()
        self.fixture.setUp()
        self.root, self.directory = self.fixture.root, self.fixture.directory
        self.contract = self.fixture.contract
        self.processes = []
        self.ark = os.environ.get('ARK_BIN') or shutil.which('ark')
        if not self.ark:
            pool = Path('/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark')
            self.ark = str(pool) if pool.is_file() else None
        self.assertTrue(self.ark, 'ARK_BIN is required for shift containment acceptance; build elohim-ark')
        self.driver = self.directory / 'driver'
        self.driver.mkdir()
        for source in Path(__file__).parent.glob('shift*.py'):
            if not source.name.endswith('_test.py'):
                data = source.read_text()
                # Accelerate only the disposable test driver. Production constants have a separate assertion.
                data = data.replace('HEARTBEAT = 15\n', 'HEARTBEAT = 0.2\n')
                data = data.replace('STOPPED_RECOVERY = 60\n', 'STOPPED_RECOVERY = 0.6\n')
                (self.driver / source.name).write_text(data)
        self.bin = self.directory / 'fixture-bin'
        self.bin.mkdir()
        context = {'reconciliation': {'scope_cid': 'scope-existing', 'assertions': [{
            'intent': 'intent-existing', 'commitments': [{'commitment': 'promise-existing'}]}]}}
        self.executable('epr', f'print({json.dumps(json.dumps(context))})\n')
        adapter = '''import json, os, sys, time, uuid
args = sys.argv[1:]
runtime = os.path.basename(sys.argv[0])
session = args[args.index('resume')+1] if 'resume' in args else (args[args.index('--resume')+1] if '--resume' in args else (args[args.index('--session-id')+1] if '--session-id' in args else str(uuid.uuid4())))
event = {'type':'thread.started','thread_id':session} if runtime == 'codex' else {'type':'system','session_id':session}
print(json.dumps(event), flush=True)
if os.environ.get('FIXTURE_DAEMON_SOCKET'):
 import subprocess
 child = "import socket,time; s=socket.socket(socket.AF_UNIX); s.connect(bytes([0])+"+repr(os.environ['FIXTURE_DAEMON_SOCKET'])+".encode()); s.send(b'daemon'); time.sleep(120)"
 subprocess.Popen([sys.executable, '-c', child], start_new_session=True)
time.sleep(float(os.environ.get('FIXTURE_AGENT_SECONDS', '1')))
print(json.dumps({'type':'turn.completed'} if runtime == 'codex' else {'type':'result','session_id':session,'is_error':False}), flush=True)
'''
        self.executable('codex', adapter)
        self.executable('claude', adapter)
        self.contract['ark'] = self.ark
        self.contract['deadline'] = iso(time.time()+25)
        grant = state.read(self.directory / 'authority.json')
        grant['deadline'] = self.contract['deadline']
        state.atomic(self.directory / 'authority.json', grant)
        self.contract['authority']['sha256'] = state.digest(self.directory / 'authority.json')
        self.fixture.freeze()
        self.env = dict(os.environ, PATH=str(self.bin)+os.pathsep+os.environ['PATH'])

    def executable(self, name, source):
        path = self.bin / name
        path.write_text(f'#!{sys.executable}\n'+source)
        path.chmod(0o700)

    def wait_for(self, predicate, timeout=10):
        deadline = time.monotonic()+timeout
        while time.monotonic() < deadline:
            result = predicate()
            if result:
                return result
            time.sleep(0.05)
        logs = '\n'.join(p.read_text()[-2500:] for p in self.directory.glob('*.log'))
        refusals = [state.read(p) for p in self.directory.glob('refusal-*.json')]
        self.fail(f'timed out; refusals={refusals}; logs={logs}')

    def checkpoint(self):
        return state.read(self.directory / 'checkpoint.json')

    def launch(self, runtime='codex'):
        cp = self.checkpoint()
        cp['runtime'] = runtime
        state.atomic(self.directory / 'checkpoint.json', cp)
        output = open(self.directory / 'guardian.log', 'ab')
        process = subprocess.Popen([sys.executable, '-c',
            'import sys; from pathlib import Path; from shift_supervisor import guardian; sys.exit(guardian(Path(sys.argv[1])))',
            str(self.directory)], cwd=self.driver, env=self.env, stdout=output, stderr=subprocess.STDOUT)
        output.close()
        self.processes.append(process)
        self.wait_for(lambda: self.checkpoint().get('controller'))
        return process

    def tearDown(self):
        for process in self.processes:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=4)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        self.fixture.tearDown()

    def test_defaults_are_fifteen_second_heartbeat_sixty_second_stopped_recovery(self):
        self.assertEqual(supervisor.HEARTBEAT, 15)
        self.assertEqual(supervisor.STOPPED_RECOVERY, 60)

    def test_both_adapters_finish_once_after_controller_death_during_dispatch(self):
        for name in ['codex', 'claude']:
            with self.subTest(runtime=name):
                self.contract['steps'] = [{'id': f'agent-{name}', 'kind': 'agent', 'prompt': 'fixture'}]
                self.fixture.freeze()
                process = self.launch(name)
                worker = self.wait_for(lambda: self.checkpoint().get('worker'))
                controller = self.checkpoint()['controller']
                self.assertTrue(state.signal_handle(controller, signal.SIGKILL))
                process.wait(timeout=15)
                self.assertEqual(process.returncode, 0, list(self.directory.glob('refusal-*.json')))
                operation = self.directory / 'operations' / f'agent-{name}'
                self.assertTrue((operation / 'receipt.json').exists())
                self.assertEqual(len(list(operation.glob('dispatch-*.json'))), 1)
                self.assertFalse(state.alive(worker))

    def test_measurement_finishes_while_controller_is_absent(self):
        marker = self.directory / 'measurement.txt'
        self.contract['steps'] = [{'id': 'measure', 'kind': 'measure', 'argv': [sys.executable, '-c',
            f"import time,pathlib;time.sleep(1);pathlib.Path({str(marker)!r}).write_text('measured')"]}]
        self.fixture.freeze()
        process = self.launch()
        self.wait_for(lambda: self.checkpoint().get('worker'))
        controller = self.checkpoint()['controller']
        state.signal_handle(controller, signal.SIGSTOP)
        self.wait_for(marker.exists)
        process.wait(timeout=15)
        self.assertEqual(process.returncode, 0)
        self.assertEqual(marker.read_text(), 'measured')

    def test_guardian_kill_reaps_daemonized_agent_descendant(self):
        name = 'shift-test-'+str(os.getpid())+'-'+str(time.time_ns())
        self.env.update(FIXTURE_DAEMON_SOCKET=name, FIXTURE_AGENT_SECONDS='120')
        with socket.socket(socket.AF_UNIX) as server:
            server.bind('\0'+name)
            server.listen(1)
            server.settimeout(10)
            process = self.launch()
            connection, _ = server.accept()
            with connection:
                host_pid = struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[0]
                descendant = state.incarnation(host_pid)
                self.assertEqual(connection.recv(20), b'daemon')
            process.kill()
            process.wait()
            self.wait_for(lambda: not state.alive(descendant), timeout=8)

    def test_wrapper_death_recovers_without_repeating_active_measurement(self):
        self.env['FIXTURE_AGENT_SECONDS'] = '2'
        process = self.launch()
        self.wait_for(lambda: self.checkpoint().get('worker'))
        wrapper = self.checkpoint()['wrapper']
        state.signal_handle(wrapper, signal.SIGKILL)
        self.wait_for(lambda: self.checkpoint().get('wrapper') != wrapper)
        process.wait(timeout=15)
        self.assertEqual(process.returncode, 0)
        self.assertEqual(len(list((self.directory/'operations/implement').glob('dispatch-*.json'))), 1)

    def test_expiry_stops_agent_outside_conversation(self):
        self.env['FIXTURE_AGENT_SECONDS'] = '120'
        self.contract['deadline'] = iso(time.time()+3)
        grant = state.read(self.directory / 'authority.json')
        grant['deadline'] = self.contract['deadline']
        state.atomic(self.directory / 'authority.json', grant)
        self.contract['authority']['sha256'] = state.digest(self.directory / 'authority.json')
        self.fixture.freeze()
        process = self.launch()
        worker = self.wait_for(lambda: self.checkpoint().get('worker'))
        process.wait(timeout=8)
        self.assertEqual(process.returncode, 3)
        self.wait_for(lambda: not state.alive(worker))
        with self.assertRaisesRegex(state.Refused, 'expired'):
            state.load(self.directory)

    def test_revocation_and_corruption_stop_active_execution(self):
        for reason in ['revoked', 'corrupt']:
            with self.subTest(reason=reason):
                self.env['FIXTURE_AGENT_SECONDS'] = '120'
                self.contract['steps'] = [{'id': reason, 'kind': 'agent', 'prompt': 'fixture'}]
                self.fixture.freeze()
                state.atomic(self.directory / 'revocation.json', {'revoked': False})
                process = self.launch()
                worker = self.wait_for(lambda: self.checkpoint().get('worker'))
                if reason == 'revoked':
                    state.atomic(self.directory / 'revocation.json', {'revoked': True})
                else:
                    (self.directory / 'checkpoint.json').write_text('{corrupt')
                process.wait(timeout=8)
                self.assertEqual(process.returncode, 3)
                deadline = time.monotonic() + 5
                while state.alive(worker) and time.monotonic() < deadline:
                    time.sleep(.05)
                self.assertFalse(state.alive(worker))

    def test_explicit_stop_is_durable_and_boot_recovery_refuses_it(self):
        import shift
        self.env['FIXTURE_AGENT_SECONDS'] = '120'
        process = self.launch()
        worker = self.wait_for(lambda: self.checkpoint().get('worker'))
        shift.stop(self.directory)
        process.wait(timeout=8)
        self.assertEqual(process.returncode, 3)
        self.wait_for(lambda: not state.alive(worker))
        recovered = shift.recover(self.root)
        self.assertIn('stop marker', recovered[0]['refused'])
        self.assertTrue((self.directory / 'stop.json').exists())

    def test_expired_boot_recovery_never_launches(self):
        import shift
        self.contract['deadline'] = iso(time.time() - 1)
        grant = state.read(self.directory / 'authority.json')
        grant['deadline'] = self.contract['deadline']
        state.atomic(self.directory / 'authority.json', grant)
        self.contract['authority']['sha256'] = state.digest(self.directory / 'authority.json')
        self.fixture.freeze()
        result = shift.recover(self.root)
        self.assertIn('expired', result[0]['refused'])
        self.assertFalse((self.directory / 'guardian-ready.json').exists())

    def test_concurrent_recovery_refuses_the_second_owner(self):
        import shift
        self.env['FIXTURE_AGENT_SECONDS'] = '3'
        process = self.launch()
        with self.assertRaisesRegex(state.Refused, 'already owned'):
            shift.resume(self.directory, 'claude')
        process.wait(timeout=15)
        self.assertEqual(process.returncode, 0)
        self.assertEqual(len(list((self.directory/'operations/implement').glob('dispatch-*.json'))), 1)

    def test_durable_receipt_survives_guardian_death_before_checkpoint(self):
        for runtime in ['codex', 'claude']:
            with self.subTest(runtime=runtime):
                step = {'id': f'receipt-{runtime}', 'kind': 'agent', 'prompt': 'fixture'}
                self.contract['steps'] = [step]
                self.fixture.freeze()
                process = self.launch(runtime)
                self.wait_for(lambda: self.checkpoint().get('worker'))
                # Stop the controller so the guardian cannot declare completion until a new handshake.
                state.signal_handle(self.checkpoint()['controller'], signal.SIGSTOP)
                receipt = self.directory / 'operations' / step['id'] / 'receipt.json'
                self.wait_for(receipt.exists)
                original = receipt.read_bytes()
                process.kill()
                process.wait()
                self.wait_for(lambda: not state.alive(self.checkpoint().get('worker')))
                restarted = self.launch(runtime)
                restarted.wait(timeout=15)
                self.assertEqual(restarted.returncode, 0)
                self.assertEqual(receipt.read_bytes(), original)
                self.assertEqual(len(list(receipt.parent.glob('dispatch-*.json'))), 1)

    def test_ark_termination_recovers_its_contained_controller(self):
        self.env['FIXTURE_AGENT_SECONDS'] = '3'
        process = self.launch()
        self.wait_for(lambda: self.checkpoint().get('worker'))
        controller = self.checkpoint()['controller']
        fields = Path(f"/proc/{controller['pid']}/stat").read_text().rsplit(')', 1)[1].split()
        ark = state.incarnation(int(fields[1]))
        self.assertTrue(supervisor.descendant(ark['pid'], self.checkpoint()['wrapper']['pid']))
        self.assertTrue(state.signal_handle(ark, signal.SIGKILL))
        self.wait_for(lambda: self.checkpoint().get('controller') != controller)
        process.wait(timeout=15)
        self.assertEqual(process.returncode, 0)


if __name__ == '__main__':
    unittest.main()
