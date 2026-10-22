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
from unittest.mock import patch

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

    def launch(self, runtime='codex', *, instrumentation='', wait_for_controller=True):
        cp = self.checkpoint()
        cp['runtime'] = runtime
        state.atomic(self.directory / 'checkpoint.json', cp)
        output = open(self.directory / 'guardian.log', 'ab')
        process = subprocess.Popen([sys.executable, '-c',
            'import os,signal,sys; from pathlib import Path; import shift_supervisor as supervisor\n'
            + instrumentation + '\nsys.exit(supervisor.guardian(Path(sys.argv[1])))',
            str(self.directory)], cwd=self.driver, env=self.env, stdout=output, stderr=subprocess.STDOUT)
        output.close()
        self.processes.append(process)
        if wait_for_controller:
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

    def assert_controller_disconnect_at_ack(self, stage, runtime):
        self.env['FIXTURE_AGENT_SECONDS'] = '2' if stage == 'active-worker' else '0.25'
        instrumentation = f'''
import time
directory=Path(sys.argv[1]); stage={stage!r}; injected=False
original_checkpoint=supervisor.checkpoint
receipt=directory/'operations/implement/receipt.json'
def checkpoint_before_ack(path, **updates):
    global injected
    result=original_checkpoint(path, **updates)
    if stage=='completed-worker' and updates.get('worker'):
        expires=time.monotonic()+5
        while not receipt.exists() and time.monotonic()<expires:
            time.sleep(.005)
        if not receipt.exists():
            raise RuntimeError('fixture worker did not persist its receipt')
    candidate=updates.get('controller')
    worker=supervisor.read(directory/'checkpoint.json').get('worker')
    ready=(stage=='before-dispatch' or
           (stage=='active-worker' and supervisor.alive(worker)) or
           (stage=='completed-worker' and receipt.exists()))
    if candidate and not injected and ready:
        injected=True
        supervisor.atomic(directory/'ack-disconnect-injection.json',{{
            'stage':stage,'controller':candidate,'worker':worker,
            'ready_existed':(directory/'guardian-ready.json').exists(),
            'dispatches':len(list((directory/'operations').glob('*/dispatch-*.json'))),
            'receipt_sha256':supervisor.digest(receipt) if receipt.exists() else None,
        }})
        supervisor.signal_handle(candidate,signal.SIGKILL)
        expires=time.monotonic()+3
        while supervisor.alive(candidate) and time.monotonic()<expires:
            time.sleep(.005)
        if supervisor.alive(candidate):
            raise RuntimeError('fixture controller did not die before ACK')
    return result
supervisor.checkpoint=checkpoint_before_ack
'''
        process = self.launch(runtime, instrumentation=instrumentation)
        process.wait(timeout=15)
        self.assertEqual(process.returncode, 0, [state.read(p) for p in self.directory.glob('refusal-*.json')])
        injection = state.read(self.directory/'ack-disconnect-injection.json')
        operation = self.directory/'operations/implement'
        receipt = operation/'receipt.json'
        self.assertEqual(state.read(receipt)['exit'], 0)
        self.assertEqual(len(list(operation.glob('dispatch-*.json'))), 1)
        self.assertNotEqual(state.read(self.directory/'guardian-ready.json')['controller'], injection['controller'])
        self.assertFalse(list(self.directory.glob('refusal-*.json')))
        if stage == 'before-dispatch':
            self.assertFalse(injection['ready_existed'])
            self.assertEqual(injection['dispatches'], 0)
        else:
            self.assertIsNotNone(injection['worker'])
            self.wait_for(lambda: not state.alive(injection['worker']))
        if stage == 'completed-worker':
            self.assertEqual(state.digest(receipt), injection['receipt_sha256'])

    def test_initial_controller_death_before_ack_never_authorizes_early_dispatch(self):
        self.assert_controller_disconnect_at_ack('before-dispatch', 'claude')

    def test_controller_ack_disconnect_preserves_active_worker(self):
        self.assert_controller_disconnect_at_ack('active-worker', 'codex')

    def test_controller_ack_disconnect_preserves_completed_worker_receipt(self):
        self.assert_controller_disconnect_at_ack('completed-worker', 'claude')

    def assert_controller_disappears_after_alive(self, stage, runtime):
        self.env['FIXTURE_AGENT_SECONDS'] = '2'
        instrumentation = f'''
import time
directory=Path(sys.argv[1]); stage={stage!r}; injected=False
original_alive=supervisor.alive
def alive_before_controller_disappears(handle):
    global injected
    result=original_alive(handle)
    saved=supervisor.read(directory/'checkpoint.json')
    worker=saved.get('worker')
    dispatches=len(list((directory/'operations').glob('*/dispatch-*.json')))
    ready=stage=='before-dispatch' or (original_alive(worker) and dispatches>0)
    if result and handle and saved.get('controller')==handle and ready and not injected:
        injected=True
        supervisor.atomic(directory/'stat-disappearance-injection.json',{{
            'controller':handle,'worker':worker,'stage':stage,
            'dispatches':dispatches,
        }})
        supervisor.signal_handle(handle,signal.SIGKILL)
        expires=time.monotonic()+3
        stat=Path('/proc/'+str(handle['pid'])+'/stat')
        while stat.exists() and time.monotonic()<expires:
            time.sleep(.005)
        if stat.exists():
            raise RuntimeError('fixture controller was not reaped before status read')
    return result
supervisor.alive=alive_before_controller_disappears
'''
        process=self.launch(runtime, instrumentation=instrumentation)
        process.wait(timeout=15)
        self.assertEqual(process.returncode,0,[state.read(p) for p in self.directory.glob('refusal-*.json')])
        injection=state.read(self.directory/'stat-disappearance-injection.json')
        operation=self.directory/'operations/implement'
        self.assertEqual(state.read(operation/'receipt.json')['exit'],0)
        self.assertEqual(len(list(operation.glob('dispatch-*.json'))),1)
        self.assertNotEqual(state.read(self.directory/'guardian-ready.json')['controller'],injection['controller'])
        self.assertFalse(list(self.directory.glob('refusal-*.json')))
        self.assertEqual(injection['dispatches'],0 if stage=='before-dispatch' else 1)
        if stage=='active-worker':
            self.assertIsNotNone(injection['worker'])
            self.wait_for(lambda:not state.alive(injection['worker']))

    def test_controller_disappearance_after_alive_recovers_before_dispatch(self):
        self.assert_controller_disappears_after_alive('before-dispatch','claude')

    def test_controller_disappearance_after_alive_preserves_active_worker(self):
        self.assert_controller_disappears_after_alive('active-worker','codex')

    def test_controller_status_requires_matching_incarnation_and_live_phase(self):
        handle=state.incarnation(os.getpid())
        record=Path(f"/proc/{handle['pid']}/stat").read_text()
        prefix,tail=record.rsplit(')',1)
        fields=tail.split()
        with patch.object(supervisor,'alive',return_value=True):
            for phase in ['R','T','t','Z','X']:
                with self.subTest(phase=phase):
                    fields[0]=phase
                    with patch.object(Path,'read_text',return_value=prefix+') '+' '.join(fields)):
                        self.assertEqual(supervisor.controller_status(handle),None if phase in {'Z','X'} else phase)
            fields[0]='T'
            fields[19]=str(int(handle['start'])+1)
            with patch.object(Path,'read_text',return_value=prefix+') '+' '.join(fields)):
                self.assertIsNone(supervisor.controller_status(handle))

    def test_controller_status_tolerates_only_departed_process_read_errors(self):
        handle=state.incarnation(os.getpid())
        with patch.object(supervisor,'alive',return_value=True):
            for error in [FileNotFoundError('departed'),ProcessLookupError('departed')]:
                with patch.object(Path,'read_text',side_effect=error):
                    self.assertIsNone(supervisor.controller_status(handle))
            with patch.object(Path,'read_text',side_effect=OSError('host status I/O failure')):
                with self.assertRaisesRegex(OSError,'host status I/O failure'):
                    supervisor.controller_status(handle)
            with patch.object(Path,'read_text',return_value='malformed'):
                with self.assertRaisesRegex(state.Refused,'malformed controller process status'):
                    supervisor.controller_status(handle)

    def test_controller_status_io_failure_retains_refusal_and_traceback(self):
        instrumentation='''
import errno
original_alive=supervisor.alive; original_read=Path.read_text; fail_path=None
def alive_before_status_io_failure(handle):
    global fail_path
    result=original_alive(handle)
    if result and supervisor.read(Path(sys.argv[1])/'checkpoint.json').get('controller')==handle:
        fail_path=Path('/proc/'+str(handle['pid'])+'/stat')
    return result
def status_io_failure(path,*args,**kwargs):
    global fail_path
    if path==fail_path:
        fail_path=None
        raise OSError(errno.EIO,'fixture controller status I/O failure')
    return original_read(path,*args,**kwargs)
supervisor.alive=alive_before_status_io_failure
Path.read_text=status_io_failure
'''
        process=self.launch(instrumentation=instrumentation)
        process.wait(timeout=8)
        self.assertEqual(process.returncode,3)
        refusals=list(self.directory.glob('refusal-*.json'))
        self.assertEqual(len(refusals),1)
        refusal=state.read(refusals[0])
        self.assertEqual(refusal['exception_type'],'OSError')
        self.assertIn('fixture controller status I/O failure',refusal['reason'])
        self.assertIn('controller_status',refusal['traceback'])
        self.assertFalse(list((self.directory/'operations').glob('*/dispatch-*.json')))

    def test_unexpected_checkpoint_io_error_retains_exception_type_and_traceback(self):
        instrumentation = '''
import errno
original_checkpoint=supervisor.checkpoint
def host_checkpoint_failure(path, **updates):
    if updates.get('controller'):
        raise OSError(errno.EIO,'fixture host checkpoint I/O failure')
    return original_checkpoint(path, **updates)
supervisor.checkpoint=host_checkpoint_failure
'''
        process=self.launch(instrumentation=instrumentation, wait_for_controller=False)
        process.wait(timeout=8)
        self.assertEqual(process.returncode,3)
        refusals=list(self.directory.glob('refusal-*.json'))
        self.assertEqual(len(refusals),1)
        refusal=state.read(refusals[0])
        self.assertEqual(refusal['exception_type'],'OSError')
        self.assertIn('fixture host checkpoint I/O failure',refusal['reason'])
        self.assertIn('host_checkpoint_failure',refusal['traceback'])
        self.assertFalse((self.directory/'guardian-ready.json').exists())

    def test_reply_tolerates_departed_peers_but_propagates_other_socket_errors(self):
        left,right=socket.socketpair()
        with left:
            right.close()
            self.assertFalse(supervisor.reply(left,b'ack\n'))
        class Fault:
            def __init__(self,error): self.error=error
            def sendall(self,_payload): raise self.error
        for error in [ConnectionResetError('peer reset'),socket.timeout('peer stalled')]:
            self.assertFalse(supervisor.reply(Fault(error),b'refused\n'))
        with self.assertRaisesRegex(OSError,'unrelated socket failure'):
            supervisor.reply(Fault(OSError('unrelated socket failure')),b'ack\n')

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

    def assert_signal_during_refusal(self, stage):
        import shift
        self.env['FIXTURE_AGENT_SECONDS'] = '120'
        instrumentation = f'''
directory = Path(sys.argv[1])
stage = {stage!r}
original_atomic = supervisor.atomic
original_wait = supervisor.subprocess.Popen.wait
refusal_written = False
injected = False
def inject():
    global injected
    if injected:
        return
    injected = True
    original_atomic(directory / 'signal-injection.json',
                    {{'stage': stage, 'timer_remaining': signal.getitimer(signal.ITIMER_REAL)[0]}})
    os.kill(os.getpid(), signal.SIGTERM)
    os.kill(os.getpid(), signal.SIGINT)
def observe_atomic(path, value, **kwargs):
    global refusal_written
    result = original_atomic(path, value, **kwargs)
    if Path(path).name.startswith('refusal-'):
        refusal_written = True
        if stage == 'after-refusal':
            inject()
    return result
def observe_wait(process, *args, **kwargs):
    if refusal_written and stage == 'during-reap':
        inject()
    return original_wait(process, *args, **kwargs)
supervisor.atomic = observe_atomic
supervisor.subprocess.Popen.wait = observe_wait
'''
        process = self.launch(instrumentation=instrumentation)
        worker = self.wait_for(lambda: self.checkpoint().get('worker'))
        wrapper = self.checkpoint()['wrapper']
        # The stop marker is real. Delay only its signal until the child reaches
        # the exact persistence/reap boundary; there is no timing-based injection.
        with patch.object(shift, 'signal_handle', return_value=False):
            shift.stop(self.directory)
        original_stop = (self.directory / 'stop.json').read_bytes()
        process.wait(timeout=8)
        self.assertEqual(process.returncode, 3, (self.directory / 'guardian.log').read_text())
        self.wait_for(lambda: not state.alive(worker) and not state.alive(wrapper))
        self.assertEqual((self.directory / 'stop.json').read_bytes(), original_stop)
        refusals = list(self.directory.glob('refusal-*.json'))
        self.assertEqual(len(refusals), 1)
        self.assertIn('stop marker', state.read(refusals[0])['reason'])
        injection = state.read(self.directory / 'signal-injection.json')
        self.assertEqual(injection['stage'], stage)
        if stage == 'during-reap':
            self.assertEqual(injection['timer_remaining'], 0)
        self.assertNotIn('Traceback', (self.directory / 'guardian.log').read_text())

    def test_late_termination_cannot_interrupt_durable_refusal(self):
        self.assert_signal_during_refusal('after-refusal')

    def test_late_termination_cannot_interrupt_child_reaping(self):
        self.assert_signal_during_refusal('during-reap')

    def test_stop_between_handshake_checkpoint_and_timer_is_recorded_without_dispatch(self):
        instrumentation = '''
directory = Path(sys.argv[1])
original_checkpoint = supervisor.checkpoint
def stop_after_checkpoint(path, **updates):
    result = original_checkpoint(path, **updates)
    if updates.get('phase') == 'handshake':
        supervisor.atomic(directory / 'stop.json', {'reason': 'fixture pre-loop stop'})
    return result
supervisor.checkpoint = stop_after_checkpoint
'''
        process = self.launch(instrumentation=instrumentation, wait_for_controller=False)
        process.wait(timeout=8)
        self.assertEqual(process.returncode, 3, (self.directory / 'guardian.log').read_text())
        refusals = list(self.directory.glob('refusal-*.json'))
        self.assertEqual(len(refusals), 1)
        self.assertIn('stop marker', state.read(refusals[0])['reason'])
        self.assertTrue((self.directory / 'stop.json').exists())
        self.assertIsNone(self.checkpoint().get('worker'))
        self.assertIsNone(self.checkpoint().get('controller'))
        self.assertFalse((self.directory / 'guardian-ready.json').exists())

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
