"""Host lease delegation, including a real worker PID namespace and private berth store."""
import datetime as dt
import json
import os
from pathlib import Path
import runpy
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch
import uuid

sys.path.insert(0, str(Path(__file__).parent))
import shift_lease as lease
import shift_supervisor as supervisor
from shift_state import Refused
import shift_state as state

ROOT = Path(__file__).resolve().parents[2]
BERTH = ROOT / 'genesis/agentic/bin/berth'

# No device in these fixtures: berth binds a mooring to the host device's session envelope when
# a device key exists, and the enclosing agent session's envelope would refuse every fixture
# session (envelope-owner-mismatch). Point the key at a path that does not exist.
_NO_DEVICE = tempfile.TemporaryDirectory(prefix='shift-lease-no-device-')
_device_env = patch.dict(os.environ, {'ELOHIM_DEVICE_KEY_FILE': os.path.join(_NO_DEVICE.name, 'ed25519.seed')})


def setUpModule():
    _device_env.start()


def tearDownModule():
    _device_env.stop()
    _NO_DEVICE.cleanup()
Berth = runpy.run_path(str(BERTH), run_name='berth_fixture')['Berth']


class ShiftLease(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='shift-lease-test-')
        self.addCleanup(self.temporary.cleanup)
        self.store = Path(self.temporary.name)
        self.environment = patch.dict(os.environ, {'BERTH_DIR': str(self.store)})
        self.environment.start()
        self.addCleanup(self.environment.stop)
        for name in lease.PROXY_ENV:
            os.environ.pop(name, None)
        self.session = 'shift-lease-fixture'
        self.contract = {
            'id': 'lease-fixture', 'worktree': str(ROOT),
            'deadline': dt.datetime.fromtimestamp(time.time() + 60, dt.timezone.utc).isoformat(),
        }
        self.berth = Berth(d=str(self.store), policy_path='/nonexistent/shift-fixture-policy')
        self.berth.moor(self.session, pid=os.getpid(), runtime='shift')
        self.berth.claim('mesh', self.session, ttl_s=1800, klass='verify')
        self.started = time.monotonic()

    def request(self, command, *options, resource='mesh', session=None, held='mesh'):
        return lease.handle_request(self.contract, held, self.started,
                                    [command, resource, '--session', session or self.session, *options])

    def files(self):
        return {str(p.relative_to(self.store)): p.read_bytes() for p in self.store.rglob('*') if p.is_file()}

    def test_nested_checks_preserve_lease_and_mooring_byte_for_byte(self):
        before = self.files()
        result = self.request('claim', '--class', 'verify', '--ttl', '1800', '--note', 'nested verification')
        self.assertEqual(result['exit'], 0, result)
        self.assertIn('renewed', result['stdout'])
        self.assertEqual(self.request('release')['exit'], 0)
        self.assertEqual(self.request('owner-check', '--action', 'storage-restart')['exit'], 0)
        who = self.request('who')
        self.assertEqual(who['exit'], 0, who)
        self.assertEqual(json.loads(who['stdout'])['holder'], self.session)
        self.assertEqual(self.files(), before)

    def test_resource_forms_are_unique_known_names_in_deterministic_order(self):
        self.assertEqual(lease.resources(None), [])
        self.assertEqual(lease.resources([]), [])
        self.assertEqual(lease.resources('mesh'), ['mesh'])
        self.assertEqual(lease.resources(['mesh', 'disk-heavy', 'cargo']), ['cargo', 'disk-heavy', 'mesh'])
        for value in ['unknown', True, 1, {}, ('cargo',), ['mesh', 'mesh'], ['cargo', None], [['cargo']]]:
            with self.subTest(value=value), self.assertRaises(Refused):
                lease.resources(value)

    def test_validation_checks_step_and_measurement_lists_without_changing_declaration(self):
        import shift_ops
        spec = {'measurement_resource': ['mesh', 'cargo'], 'steps': [
            {'id': 'measure', 'kind': 'measure', 'argv': ['true'], 'resource': ['mesh', 'cargo']}]}
        original = json.dumps(spec)
        shift_ops.validate_steps(spec)
        self.assertEqual(json.dumps(spec), original)
        for value in [['cargo', 'cargo'], ['mesh', 'unknown'], {'cargo': True}]:
            for field in ['measurement_resource', 'resource']:
                candidate = json.loads(original)
                (candidate if field == 'measurement_resource' else candidate['steps'][0])[field] = value
                with self.subTest(field=field, value=value), self.assertRaises(Refused):
                    shift_ops.validate_steps(candidate)

    def test_readiness_flattens_resources_and_bounds_mesh_baseline_before_releasing(self):
        import shift as launcher
        root = self.store / 'readiness-root'
        root.mkdir()
        (root / '.git').touch()
        (root / 'ark').touch()
        spec = {'id': 'fixture', 'worktree': str(root), 'ark': str(root / 'ark'), 'allowed_tools': [],
                'verification': ['fixture'], 'measurement_resource': ['mesh', 'cargo'],
                'deadline': 'original-deadline', 'steps': [
                    {'id': 'gate', 'kind': 'gate', 'project': 'fixture', 'resource': ['mesh', 'disk-heavy']}]}
        elapsed = [0]
        def bounded(argv, cwd, timeout, log, text=None, *, env=None, check=None):
            if 'genesis/agentic/readiness.mjs' in argv:
                self.assertEqual(timeout, 1800)
                elapsed[0] = 1801
                self.assertEqual(check(), -1)
                raise Refused('mesh readiness lease expired')
        with patch.object(launcher.shutil, 'which', return_value='/fixture/tool'), \
                patch.object(launcher, 'initialization_remaining', return_value=3000), \
                patch.object(launcher, 'session_from', return_value='fixture-session'), \
                patch.object(launcher, 'completed', return_value=True), \
                patch.object(launcher, 'native_context'), \
                patch.object(launcher, 'bounded', side_effect=bounded), \
                patch.object(launcher, 'run', return_value='fixture\tfixture'), \
                patch.object(launcher.time, 'monotonic', side_effect=lambda: elapsed[0]), \
                patch.object(launcher, 'lease') as claim:
            with self.assertRaisesRegex(Refused, 'mesh readiness lease expired'):
                launcher.preflight(spec, root / '.claude/shifts/fixture')
            self.assertEqual(claim.call_args_list, [
                unittest.mock.call(spec, ['cargo', 'disk-heavy', 'mesh']),
                unittest.mock.call(spec, ['cargo', 'disk-heavy', 'mesh'], release=True),
                unittest.mock.call(spec, ['mesh', 'cargo']),
                unittest.mock.call(spec, ['mesh', 'cargo'], release=True)])

    def test_multiple_resources_preserve_each_original_lease_and_refuse_expansion(self):
        self.berth.claim('cargo', self.session)
        before = self.files()
        held = ['mesh', 'cargo']
        for resource in held:
            for command in ['claim', 'release', 'owner-check', 'who']:
                result = self.request(command, resource=resource, held=held)
                self.assertEqual(result['exit'], 0, result)
        self.assertEqual(self.request('claim', resource='disk-heavy', held=held)['exit'], 3)
        self.assertEqual(self.request('claim', '--ttl', '1801', held=held)['exit'], 3)
        self.assertEqual(self.request('claim', '--class', 'mesh', held=held)['exit'], 3)
        self.assertEqual(self.files(), before)

    def test_requested_member_still_requires_its_live_exact_host_lease(self):
        held = ['cargo', 'mesh']
        self.assertEqual(self.request('claim', resource='cargo', held=held)['exit'], 3)
        self.berth.moor('other-session', pid=os.getpid())
        self.berth.claim('cargo', 'other-session')
        self.assertEqual(self.request('owner-check', resource='cargo', held=held)['exit'], 3)
        self.assertEqual(self.request('owner-check', held=held)['exit'], 0)
        expired = lease.handle_request(self.contract, held, self.started - 1801,
                                       ['owner-check', 'mesh', '--session', self.session])
        self.assertEqual(expired['exit'], 3, expired)

    def test_host_acquires_in_order_with_one_mooring_and_releases_every_member(self):
        self.berth.release('mesh', self.session)
        original = supervisor.host_berth
        calls = []
        def observed(contract, *argv):
            calls.append(argv)
            return original(contract, *argv)
        with patch.object(supervisor, 'host_berth', side_effect=observed), patch.dict(os.environ, {
                lease.PROXY_ENV[0]: '@absent-enclosing-guardian', lease.PROXY_ENV[1]: 'must-not-recurse'}):
            supervisor.lease(self.contract, ['mesh', 'cargo'])
            mesh_before = self.berth.who('mesh')
            self.assertEqual(mesh_before['ttl_s'], 1800)
            self.assertEqual(mesh_before['class'], 'verify')
            cargo = self.berth.who('cargo')
            self.assertEqual((cargo[0] if isinstance(cargo, list) else cargo)['holder'], self.session)
            supervisor.lease(self.contract, ['mesh', 'cargo'], release=True)
        self.assertEqual([args[:2] for args in calls], [('moor', '--session'), ('claim', 'cargo'),
                         ('claim', 'mesh'), ('release', 'mesh'), ('release', 'cargo')])
        self.assertIsNone(self.berth.who('cargo'))
        self.assertIsNone(self.berth.who('mesh'))

    def test_partial_claim_rolls_back_our_resources_preserving_other_holder(self):
        self.berth.release('mesh', self.session)
        self.berth.moor('other-session', pid=os.getpid())
        self.berth.claim('mesh', 'other-session', ttl_s=1800, klass='verify')
        incumbent = self.berth.who('mesh')
        with self.assertRaisesRegex(Refused, 'claim exited 3'):
            supervisor.lease(self.contract, ['mesh', 'cargo'])
        self.assertIsNone(self.berth.who('cargo'))
        self.assertEqual(self.berth.who('mesh'), incumbent)

    def test_lost_claim_response_also_rolls_back_the_uncertain_own_lease(self):
        self.berth.release('mesh', self.session)
        original = supervisor.host_berth
        def lost_response(contract, *argv):
            result = original(contract, *argv)
            if argv[:2] == ('claim', 'mesh'):
                raise Refused('lost claim response')
            return result
        with patch.object(supervisor, 'host_berth', side_effect=lost_response):
            with self.assertRaisesRegex(Refused, 'lost claim response'):
                supervisor.lease(self.contract, ['mesh', 'cargo'])
        self.assertIsNone(self.berth.who('cargo'))
        self.assertIsNone(self.berth.who('mesh'))

    def test_release_failure_still_attempts_other_resources(self):
        self.berth.claim('cargo', self.session)
        original = supervisor.host_berth
        def failed_release(contract, *argv):
            if argv[:2] == ('release', 'mesh'):
                raise Refused('fixture mesh release unavailable')
            return original(contract, *argv)
        with patch.object(supervisor, 'host_berth', side_effect=failed_release):
            with self.assertRaisesRegex(Refused, 'resource release unresolved'):
                supervisor.lease(self.contract, ['cargo', 'mesh'], release=True)
        self.assertIsNone(self.berth.who('cargo'))
        self.assertEqual(self.berth.who('mesh')['holder'], self.session)

    def test_requests_cannot_expand_authority_or_change_lease_terms(self):
        before = self.files()
        denied = [
            self.request('claim', '--force'),
            self.request('claim', '--class', 'measure'),
            self.request('claim', '--ttl', '1801'),
            self.request('claim', '--ttl', '0'),
            self.request('claim', '--ttl', 'invalid'),
            self.request('claim', '--session', self.session),
            self.request('claim', resource='cargo'),
            self.request('claim', session='some-other-session'),
            self.request('release', '--force'),
            self.request('owner-check', '--force'),
            self.request('moor'),
            self.request('touch'),
            self.request('unmoor'),
        ]
        self.assertTrue(all(result['exit'] == 3 for result in denied), denied)
        self.assertEqual(self.files(), before)

    def test_no_current_resource_or_missing_store_never_allows_unleased_run(self):
        argv = ['claim', 'mesh', '--session', self.session]
        self.assertEqual(lease.handle_request(self.contract, None, self.started, argv)['exit'], 3)
        (self.store / 'berth.lock').unlink()
        self.assertEqual(self.request('claim')['exit'], 3)

    def test_expired_or_replaced_lease_and_deadline_refuse(self):
        self.assertEqual(lease.handle_request(self.contract, 'mesh', self.started - 1801,
                         ['claim', 'mesh', '--session', self.session])['exit'], 3)
        self.contract['deadline'] = dt.datetime.fromtimestamp(time.time() - 1, dt.timezone.utc).isoformat()
        self.assertEqual(self.request('claim')['exit'], 3)
        self.contract['deadline'] = dt.datetime.fromtimestamp(time.time() + 60, dt.timezone.utc).isoformat()
        self.berth.release('mesh', self.session)
        self.berth.moor('other-session', pid=os.getpid())
        self.berth.claim('mesh', 'other-session', ttl_s=1800, klass='verify')
        self.assertEqual(self.request('claim')['exit'], 3)

    def test_stale_guardian_incarnation_and_corrupt_mooring_refuse(self):
        path = self.store / 'moorings' / f'{self.session}.json'
        original = path.read_text()
        row = json.loads(original)
        row['pid_start'] -= 1
        path.write_text(json.dumps(row))
        self.assertEqual(self.request('claim')['exit'], 3)
        path.write_text('{broken')
        self.assertEqual(self.request('claim')['exit'], 3)
        path.write_text(original)
        self.assertEqual(self.request('claim')['exit'], 0)

    def test_host_inspection_strips_proxy_environment(self):
        with patch.dict(os.environ, {lease.PROXY_ENV[0]: '@missing-host', lease.PROXY_ENV[1]: 'unused'}):
            self.assertEqual(self.request('claim')['exit'], 0)

    def test_missing_proxy_credentials_or_host_refuse_instead_of_exit_four(self):
        for credentials in [
            {lease.PROXY_ENV[0]: '@missing-host'},
            {lease.PROXY_ENV[1]: 'token'},
            {lease.PROXY_ENV[0]: '@missing-host', lease.PROXY_ENV[1]: 'token'},
        ]:
            with self.subTest(credentials=list(credentials)):
                env = dict(os.environ, BERTH_SESSION=self.session, **credentials)
                result = subprocess.run([sys.executable, str(BERTH), 'claim', 'mesh'], env=env,
                                        text=True, capture_output=True, timeout=10)
                self.assertEqual(result.returncode, 3, result.stderr)
                self.assertIn('REFUSED', result.stderr)

    def test_ordinary_berth_still_reads_host_store_without_proxy(self):
        result = subprocess.run([sys.executable, str(BERTH), 'who', 'mesh'], env=dict(os.environ),
                                text=True, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['holder'], self.session)

    def proxy_server(self, reply=None, held='mesh'):
        address = '@shift-lease-test-' + str(uuid.uuid4())
        server = socket.socket(socket.AF_UNIX)
        server.bind('\0' + address[1:])
        server.listen(1)
        server.settimeout(10)
        errors = []

        def serve():
            try:
                with server, server.accept()[0] as stream:
                    stream.settimeout(5)
                    data = bytearray()
                    while b'\n' not in data:
                        chunk = stream.recv(4096)
                        if not chunk or len(data) + len(chunk) > lease.MAX_MESSAGE:
                            raise ValueError('invalid test request framing')
                        data.extend(chunk)
                    request = json.loads(data)
                    _pid, uid, _gid = struct.unpack('3i', stream.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                    if uid != os.getuid() or request.get('kind') != 'berth' or request.get('token') != 'fixture-token':
                        result = lease.refused('test peer authentication failed')
                    elif reply is not None:
                        result = reply
                    else:
                        result = lease.handle_request(self.contract, held, self.started, request['argv'])
                    stream.sendall(json.dumps(result).encode() + b'\n')
            except Exception as error:
                errors.append(error)

        thread = threading.Thread(target=serve, daemon=True)
        thread.start()
        return address, thread, errors

    def test_proxy_rejects_unleased_or_malformed_host_response(self):
        for reply in [{'exit': 4, 'stdout': '', 'stderr': ''}, {'exit': 0}]:
            address, thread, errors = self.proxy_server(reply=reply)
            env = {'BERTH_SESSION': self.session, lease.PROXY_ENV[0]: address, lease.PROXY_ENV[1]: 'fixture-token'}
            self.assertEqual(lease.proxy_request(['claim', 'mesh'], env)['exit'], 3)
            thread.join(timeout=10)
            self.assertFalse(thread.is_alive())
            self.assertFalse(errors, errors)

    def test_real_pid_namespace_delegates_to_live_host_lease_without_mutation(self):
        # The primitive must work here; an unavailable namespace is a failed
        # prerequisite, not a skipped acceptance check.
        namespace = ['unshare', '--user', '--map-root-user', '--pid', '--fork', '--kill-child=KILL', '--mount-proc']
        code = ('import runpy; b=runpy.run_path(' + repr(str(BERTH)) + ',run_name="fixture")["Berth"](); '
                'print(b.is_live(b.mooring(' + repr(self.session) + ')))')
        invisible = subprocess.run(namespace + [sys.executable, '-c', code], env=dict(os.environ),
                                   text=True, capture_output=True, timeout=10)
        self.assertEqual(invisible.returncode, 0, invisible.stderr)
        self.assertEqual(invisible.stdout.strip(), 'False')
        self.assertTrue(self.berth.is_live(self.berth.mooring(self.session)))
        before = self.files()
        for command in [('claim', 'mesh', '--class', 'verify', '--ttl', '1800'), ('release', 'mesh'), ('who', 'mesh')]:
            address, thread, errors = self.proxy_server()
            env = dict(os.environ, BERTH_SESSION=self.session,
                       ELOHIM_SHIFT_BERTH_ADDRESS=address, ELOHIM_SHIFT_BERTH_TOKEN='fixture-token')
            result = subprocess.run(namespace + [sys.executable, str(BERTH), *command], env=env,
                                    text=True, capture_output=True, timeout=10)
            thread.join(timeout=10)
            self.assertFalse(thread.is_alive())
            self.assertFalse(errors, errors)
            self.assertEqual(result.returncode, 0, result.stderr)
            if command[0] == 'claim':
                self.assertIn('renewed', result.stdout)
        self.assertEqual(self.files(), before)

    def test_real_pid_namespace_nested_mesh_and_cargo_requests_keep_both_host_leases(self):
        self.berth.claim('cargo', self.session)
        before = self.files()
        namespace = ['setpriv', '--pdeathsig', 'KILL', 'unshare', '--user', '--map-root-user',
                     '--pid', '--fork', '--kill-child=KILL', '--mount-proc']
        for command in [('owner-check', 'mesh', '--action', 'build'), ('claim', 'cargo'),
                        ('release', 'cargo'), ('claim', 'mesh', '--class', 'verify', '--ttl', '1800'),
                        ('release', 'mesh')]:
            address, thread, errors = self.proxy_server(held=['cargo', 'mesh'])
            env = dict(os.environ, BERTH_SESSION=self.session,
                       ELOHIM_SHIFT_BERTH_ADDRESS=address, ELOHIM_SHIFT_BERTH_TOKEN='fixture-token')
            result = subprocess.run(namespace + [sys.executable, str(BERTH), *command], env=env,
                                    text=True, capture_output=True, timeout=10)
            thread.join(timeout=10)
            self.assertFalse(thread.is_alive())
            self.assertFalse(errors, errors)
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.files(), before)

    def test_real_guardian_holds_and_releases_the_set_around_a_contained_worker(self):
        from shift_process_test import ProcessRecovery
        fixture = ProcessRecovery('runTest')
        fixture.setUp()
        self.addCleanup(fixture.tearDown)
        fixture.env['BERTH_DIR'] = str(self.store / 'guardian-berth')
        (fixture.root / 'genesis').mkdir()
        (fixture.root / 'genesis/agentic').symlink_to(ROOT / 'genesis/agentic', target_is_directory=True)
        marker, finish = fixture.directory / 'nested-lease-result.json', fixture.directory / 'finish-worker'
        commands = [['owner-check', 'mesh', '--action', 'build'], ['claim', 'cargo'],
                    ['release', 'cargo'], ['release', 'mesh']]
        script = ('import json,pathlib,subprocess,sys,time\n'
                  f'rows=[subprocess.run([{str(BERTH)!r},*args],capture_output=True,text=True) for args in {commands!r}]\n'
                  f'pathlib.Path({str(marker)!r}).write_text(json.dumps([dict(exit=r.returncode,out=r.stdout,error=r.stderr) for r in rows]))\n'
                  'until=time.monotonic()+10\n'
                  f'while not pathlib.Path({str(finish)!r}).exists() and time.monotonic()<until: time.sleep(.02)\n'
                  'sys.exit(any(r.returncode for r in rows))\n')
        fixture.contract['steps'] = [{'id': 'nested-leases', 'kind': 'measure', 'resource': ['mesh', 'cargo'],
                                      'argv': [sys.executable, '-c', script]}]
        fixture.fixture.freeze()
        process = fixture.launch('claude')
        fixture.wait_for(marker.exists)
        result = json.loads(marker.read_text())
        self.assertTrue(all(row['exit'] == 0 for row in result), result)
        host = Berth(d=fixture.env['BERTH_DIR'], policy_path='/nonexistent/shift-fixture-policy')
        for resource in ['cargo', 'mesh']:
            held = host.who(resource)
            self.assertEqual((held[0] if isinstance(held, list) else held)['holder'], 'shift-' + fixture.contract['id'])
        finish.touch()
        process.wait(timeout=15)
        self.assertEqual(process.returncode, 0, list(fixture.directory.glob('refusal-*.json')))
        for resource in ['cargo', 'mesh']:
            self.assertIsNone(host.who(resource))


class ReadinessCapacity(unittest.TestCase):
    """Real berth CLI and private capacity policy; runtime/native evidence is mocked."""

    def setUp(self):
        import shift as launcher
        self.launcher = launcher
        temporary = tempfile.TemporaryDirectory(prefix='shift-readiness-capacity-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / '.git').touch()
        (self.root / 'ark').touch()
        agentic = self.root / 'genesis/agentic'
        (agentic / 'bin').mkdir(parents=True)
        shutil.copy2(BERTH, agentic / 'bin/berth')
        # berth imports _lib.session at startup (resolved from the repo root it runs in).
        scripts_lib = self.root / '.claude/scripts/_lib'
        scripts_lib.mkdir(parents=True)
        for name in ('__init__.py', 'session.py'):
            shutil.copy2(ROOT / '.claude/scripts/_lib' / name, scripts_lib / name)
        policy = agentic / 'pool-policy.json'
        state.atomic(policy, {'berth': {'resources': {'cargo': 2, 'mesh': 1, 'disk-heavy': 1}}})
        self.store = self.root / 'berth'
        env = patch.dict(os.environ, {'BERTH_DIR': str(self.store), 'CLAUDE_PROJECT_DIR': str(self.root),
                                     lease.PROXY_ENV[0]: '@absent-parent', lease.PROXY_ENV[1]: 'fixture'})
        env.start()
        self.addCleanup(env.stop)
        self.berth = Berth(d=str(self.store), policy_path=str(policy))
        self.directory = state.home(self.root, 'capacity-fixture')
        self.directory.mkdir(parents=True)
        self.spec = {'id': 'capacity-fixture', 'worktree': str(self.root), 'ark': str(self.root / 'ark'),
                     'allowed_tools': [], 'verification': ['fixture'], 'measurement_resource': 'cargo',
                     'started_at': dt.datetime.fromtimestamp(time.time() - 1, dt.timezone.utc).isoformat(),
                     'deadline': dt.datetime.fromtimestamp(time.time() + 120, dt.timezone.utc).isoformat(),
                     'objective_path': 'objective.json',
                     'steps': [{'id': 'agent', 'kind': 'agent', 'prompt': 'Fixture only', 'resource': 'cargo'}]}
        state.atomic(self.root / 'objective.json', {'budget': {'iterations': 3}, 'measure': {'run': 'true'}})
        state.atomic(self.directory / 'objective.json', state.read(self.root / 'objective.json'))
        self.spec['objective_sha256'] = state.digest(self.directory / 'objective.json')
        grant = self.directory / 'authority.json'
        state.atomic(grant, {'build': True, 'source': 'fixture authorization', 'deadline': self.spec['deadline']})
        state.atomic(self.directory / 'revocation.json', {'revoked': False})
        self.spec['authority'] = {'path': str(grant.relative_to(self.root)), 'sha256': state.digest(grant),
                                  'revocation': str((self.directory / 'revocation.json').relative_to(self.root))}
        self.freeze()
        self.baseline_holders = []
        def bounded(argv, cwd, timeout, log, *args, **kwargs):
            if 'genesis/agentic/readiness.mjs' in argv:
                self.baseline_holders = self.berth.who('cargo')
                state.atomic(log, {'ready': True, 'baseline': 1})
        for name, replacement in [('native_context', None), ('session_from', 'session'),
                                   ('completed', True), ('run', 'fixture\tfixture'), ('revision', 'a' * 40)]:
            patcher = patch.object(launcher, name, return_value=replacement)
            patcher.start()
            self.addCleanup(patcher.stop)
        patcher = patch.object(launcher.shutil, 'which', return_value='/fixture/tool')
        patcher.start()
        self.addCleanup(patcher.stop)
        patcher = patch.object(launcher, 'bounded', side_effect=bounded)
        self.bounded = patcher.start()
        self.addCleanup(patcher.stop)

    def freeze(self):
        state.atomic(self.directory / 'initialization.json', {
            'version': 1, 'spec': self.spec,
            'objective_source_sha256': state.digest(self.root / 'objective.json')})
        state.atomic(self.directory / 'initialization-checkpoint.json', {
            'version': 1, 'phase': 'preflight', 'attempt': 1, 'last_wall': time.time(),
            'initialization_sha256': state.digest(self.directory / 'initialization.json')})

    def incumbent(self, name, resource='cargo'):
        self.berth.moor(name, pid=os.getpid())
        self.assertTrue(self.berth.claim(resource, name)[0])
        return self.berth.who(resource)

    def test_one_of_two_slots_allows_readiness_and_preserves_incumbent(self):
        before = self.incumbent('incumbent')
        mooring = (self.store / 'moorings/incumbent.json').read_bytes()
        result = self.launcher.preflight(self.spec, self.directory)
        self.assertEqual(result['measured'], 1)
        self.assertEqual({row['holder'] for row in self.baseline_holders},
                         {'incumbent', 'shift-capacity-fixture'})
        self.assertEqual(self.berth.who('cargo'), before)
        self.assertEqual((self.store / 'moorings/incumbent.json').read_bytes(), mooring)

    def test_full_capacity_refuses_before_canaries_or_baseline(self):
        self.incumbent('first')
        before = self.incumbent('second')
        with self.assertRaisesRegex(Refused, 'claim exited 3'):
            self.launcher.preflight(self.spec, self.directory)
        self.bounded.assert_not_called()
        self.assertEqual(self.berth.who('cargo'), before)

    def test_later_resource_refusal_rolls_back_probe_without_releasing_incumbents(self):
        cargo = self.incumbent('cargo-holder')
        mesh = self.incumbent('mesh-holder', 'mesh')
        self.spec['steps'][0]['resource'] = ['mesh', 'cargo']
        self.freeze()
        with self.assertRaisesRegex(Refused, 'claim exited 3'):
            self.launcher.preflight(self.spec, self.directory)
        self.bounded.assert_not_called()
        self.assertEqual(self.berth.who('cargo'), cargo)
        self.assertEqual(self.berth.who('mesh'), mesh)
        events = self.berth.ledger(0)
        self.assertTrue(any(e['kind'] == 'claim' and e.get('session') == 'shift-capacity-fixture'
                            and e.get('resource') == 'cargo' for e in events), events)

    def test_probe_release_failure_refuses_before_canaries_or_baseline(self):
        original = self.launcher.lease
        def failed_release(contract, resource, release=False):
            original(contract, resource, release=release)
            if release:
                raise Refused('fixture release acknowledgement lost')
        with patch.object(self.launcher, 'lease', side_effect=failed_release):
            with self.assertRaisesRegex(Refused, 'release acknowledgement lost'):
                self.launcher.preflight(self.spec, self.directory)
        self.bounded.assert_not_called()
        self.assertIsNone(self.berth.who('cargo'))

    def test_missing_or_corrupt_initialization_refuses_before_any_lease_or_runtime(self):
        for name in ['initialization.json', 'initialization-checkpoint.json', 'authority.json', 'revocation.json']:
            path = self.directory / name
            original = path.read_bytes()
            for missing in [True, False]:
                with self.subTest(name=name, missing=missing):
                    if missing:
                        path.unlink()
                    else:
                        path.write_text('{corrupt')
                    with patch.object(self.launcher, 'lease') as claim, self.assertRaises((Refused, OSError)):
                        self.launcher.preflight(self.spec, self.directory)
                    claim.assert_not_called()
                    self.bounded.assert_not_called()
                    path.write_bytes(original)

    def test_authority_refusal_after_claim_still_releases_probe(self):
        remaining = self.launcher.initialization_remaining
        def revoked(directory):
            left = remaining(directory)
            held = self.berth.who('cargo') or []
            if any(row['holder'] == 'shift-capacity-fixture' for row in held):
                raise Refused('fixture authority revoked during probe')
            return left
        with patch.object(self.launcher, 'initialization_remaining', side_effect=revoked):
            with self.assertRaisesRegex(Refused, 'authority revoked during probe'):
                self.launcher.preflight(self.spec, self.directory)
        self.bounded.assert_not_called()
        self.assertIsNone(self.berth.who('cargo'))

    def test_shared_capacity_does_not_weaken_exclusive_owner_check(self):
        incumbent = self.incumbent('incumbent')
        supervisor.lease(self.spec, 'cargo')
        try:
            result = lease.handle_request(self.spec, 'cargo', time.monotonic(),
                ['owner-check', 'cargo', '--session', 'shift-capacity-fixture', '--action', 'build'])
            self.assertEqual(result['exit'], 3, result)
            self.assertIn('another host session', result['stderr'])
        finally:
            supervisor.lease(self.spec, 'cargo', release=True)
        self.assertEqual(self.berth.who('cargo'), incumbent)


if __name__ == '__main__':
    unittest.main()
