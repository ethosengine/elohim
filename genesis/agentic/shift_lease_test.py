"""Host lease delegation, including a real worker PID namespace and private berth store."""
import datetime as dt
import json
import os
from pathlib import Path
import runpy
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

ROOT = Path(__file__).resolve().parents[2]
BERTH = ROOT / 'genesis/agentic/bin/berth'
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

    def request(self, command, *options, resource='mesh', session=None):
        return lease.handle_request(self.contract, 'mesh', self.started,
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

    def proxy_server(self, reply=None):
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
                        result = lease.handle_request(self.contract, 'mesh', self.started, request['argv'])
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


if __name__ == '__main__':
    unittest.main()
