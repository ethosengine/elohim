"""Run: python3 -m unittest genesis.agentic.lsp_attach_test"""
import io
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import tempfile
import unittest


BIN = Path(__file__).parent / 'bin'
read_message = runpy.run_path(str(BIN / 'lsp-null.py'))['read_message']


class AttachTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        (self.root / '.git').mkdir()
        scripts = self.root / 'genesis/agentic/bin'
        scripts.mkdir(parents=True)
        for name in ('pool-lib.sh', 'lsp-null.py'):
            shutil.copyfile(BIN / name, scripts / name)
        self.launcher = self.root / 'bin-launch'
        self.attach = scripts / 'lsp-attach'
        # Isolate the one absolute deployment path; all resolution logic stays unchanged.
        self.attach.write_text((BIN / 'lsp-attach').read_text().replace(
            '/projects/.claude-config/tools/bin-store/bin/bin-launch', str(self.launcher)))
        self.attach.chmod(0o755)
        self.path_bin = self.root / 'path-bin'
        self.path_bin.mkdir()
        self.pool_binary = self.root / 'pool/family/fixture/elohim__eprfs/dev/debug/epr'
        self.env = {**os.environ, 'PATH': f'{self.path_bin}:/usr/bin:/bin',
                    'EPR_BIN': str(self.root / 'override epr'),
                    'CARGO_TARGET_POOL_ROOT': str(self.root / 'pool'),
                    'CARGO_TARGET_POOL_FAMILY': 'fixture'}

    def executable(self, path):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('#!/usr/bin/env python3\nimport json, sys\n'
                        'print(json.dumps(dict(executable=sys.argv[0], args=sys.argv[1:])))\n')
        path.chmod(0o755)

    def test_resolution_order_and_argument_boundaries(self):
        candidates = [Path(self.env['EPR_BIN']), self.launcher,
                      self.pool_binary, self.path_bin / 'epr']
        for path in candidates:
            self.executable(path)
        for path in candidates:
            with self.subTest(candidate=path):
                result = subprocess.run([str(self.attach), 'rust', '--root', 'a root with spaces'],
                                        env=self.env, cwd=self.root, text=True,
                                        capture_output=True, timeout=3)
                self.assertEqual(result.returncode, 0, result.stderr)
                row = json.loads(result.stdout)
                self.assertEqual(row['executable'], str(path))
                prefix = ['epr'] if path == self.launcher else []
                self.assertEqual(row['args'], prefix + ['lsp', 'attach', 'rust', '--root', 'a root with spaces'])
                path.unlink()

    def fallback_messages(self):
        messages = [
            {'jsonrpc': '2.0', 'id': 1, 'method': 'initialize'},
            {'jsonrpc': '2.0', 'method': 'textDocument/didOpen',
             'params': {'textDocument': {'uri': 'file:///workspace/lib.rs'}}},
        ]
        frames = []
        for message in messages:
            body = json.dumps(message).encode()
            frames.append(f'Content-Length: {len(body)}\r\n\r\n'.encode() + body)
        result = subprocess.run([str(self.attach), 'rust'], env=self.env, cwd=self.root,
                                input=b''.join(frames),
                                capture_output=True, timeout=3)
        self.assertEqual(result.returncode, 0, result.stderr)
        stream = io.BytesIO(result.stdout)
        self.assertEqual(read_message(stream)['result'], {'capabilities': {}})
        warning = read_message(stream)['params']
        diagnostic = read_message(stream)['params']['diagnostics'][0]
        self.assertIsNone(read_message(stream))
        return warning, diagnostic

    def test_unresolved_epr_preserves_fallback_reason(self):
        warning, diagnostic = self.fallback_messages()
        self.assertEqual(warning, {'type': 2, 'message': 'no diagnostics: epr unresolved'})
        self.assertEqual(diagnostic['message'], warning['message'])

    def test_attach_passes_through_device_supervisor_unavailable_reason(self):
        epr = Path(self.env['EPR_BIN'])
        # The attach client chooses the reason and speaks LSP through lsp-null.
        # Any attempt to serve or supervise here must fail the fixture.
        epr.write_text('#!/usr/bin/env python3\nimport os, sys\n'
                       'assert sys.argv[1:] == ["lsp", "attach", "rust"], sys.argv\n'
                       f'os.execv(sys.executable, [sys.executable, {str(BIN / "lsp-null.py")!r}, '
                       '"device supervisor not running — epr device supervise"])\n')
        epr.chmod(0o755)
        warning, diagnostic = self.fallback_messages()
        self.assertEqual(warning, {
            'type': 2,
            'message': 'no diagnostics: device supervisor not running — epr device supervise'})
        self.assertEqual(diagnostic['message'], warning['message'])

    def test_device_resident_has_an_exact_named_daemon_class(self):
        berth = runpy.run_path(str(BIN / 'berth'))['Berth'](
            str(self.root / 'berth'), policy_path=str(BIN.parent / 'pool-policy.json'))
        self.assertEqual(berth.capacities['toolchain:lsp'], 1)
        self.assertEqual(berth.class_spec('toolchain:lsp', 'lsp'), {'daemon': True})
        self.assertIsNone(berth.claim_preflight('toolchain:lsp', klass='lsp'))


if __name__ == '__main__':
    unittest.main()
