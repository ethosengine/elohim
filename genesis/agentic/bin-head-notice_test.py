"""Prompt I/O bounds and landing-transition semantics, with isolated stores only."""
import importlib.util
import io
import json
import os
from pathlib import Path
import runpy
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('notice', Path(__file__).with_name('bin-head-notice.py'))
notice = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notice)


class NoticeTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.tool = self.root / 'epr'
        (self.tool / 'sessions').mkdir(parents=True)
        self.session = 'this-session'
        self.pin = self.tool / 'sessions' / notice.pin_name(self.session)
        self.pin.write_text(json.dumps({'session': self.session, 'cid': 'A'}))
        self.head('B', 'landing-1')

    def head(self, cid, at):
        (self.tool / 'current.json').write_text(json.dumps({'cid': cid, 'landedAt': at}))

    def test_return_to_earlier_head_is_a_new_landing_not_a_repeat_prompt(self):
        for cid, at in [('B', 'landing-1'), ('C', 'landing-2'), ('B', 'landing-3')]:
            self.head(cid, at)
            self.assertEqual(len(notice.notices(self.root, self.session)), 1)
            self.assertEqual(notice.notices(self.root, self.session), [])
        self.assertEqual(list((self.tool / 'sessions').iterdir()), [self.pin])
        self.assertTrue((self.tool / 'notices' / self.pin.name).is_file())

    def test_a_return_between_prompts_is_identified_by_landing_time(self):
        self.assertEqual(len(notice.notices(self.root, self.session)), 1)
        self.head('C', 'landing-2')
        # The session did not prompt while C was current.
        self.head('B', 'landing-3')
        self.assertEqual(len(notice.notices(self.root, self.session)), 1)
        self.assertEqual(notice.notices(self.root, self.session), [])

    def test_reads_only_head_and_addressed_pin_without_session_or_tool_scan(self):
        reads = []
        original = Path.read_text
        def read(path, *args, **kwargs):
            reads.append(path)
            return original(path, *args, **kwargs)
        with patch.object(Path, 'iterdir', side_effect=AssertionError('directory scan')), patch.object(Path, 'read_text', read):
            self.assertEqual(len(notice.notices(self.root, self.session, ('epr',))), 1)
        self.assertEqual(reads, [self.tool / 'current.json', self.pin])

    def test_hook_agrees_with_native_explicit_session_before_payload(self):
        import builtins
        original = builtins.__import__
        def guarded(name, *args, **kwargs):
            if name.startswith('_lib.session') or name == 'subprocess':
                raise AssertionError('runtime discovery or spawned process')
            return original(name, *args, **kwargs)
        cases = [
            ({'ELOHIM_SESSION_ID': 'ownerless-X'}, self.session),
            ({'ELOHIM_SESSION_ID': self.session, 'CLAUDE_SESSION_ID': 'wrong'}, ''),
            ({'ELOHIM_SESSION_ID': self.session, 'ELOHIM_SESSION_OWNER_PID': str(os.getppid()), 'PPID': 'wrong'}, 'payload-Y'),
            ({'ELOHIM_SESSION_ID': self.session, 'ELOHIM_SESSION_OWNER_PID': '123', 'ELOHIM_HOOK_RUNTIME_PID': '123'}, 'payload-Y'),
            ({'ELOHIM_SESSION_ID': 'wrong', 'ELOHIM_SESSION_OWNER_PID': '999', 'ELOHIM_HOOK_RUNTIME_PID': '123'}, self.session),
            ({'ELOHIM_SESSION_ID': 'wrong', 'ELOHIM_SESSION_OWNER_PID': 'invalid'}, self.session),
            ({'ELOHIM_SESSION_ID': '   ', 'CLAUDE_SESSION_ID': 'wrong'}, self.session),
            ({'CLAUDE_SESSION_ID': self.session}, ''),
        ]
        for env, payload in cases:
            with self.subTest(env=env):
                for marker in (self.tool / 'notices').glob('*'):
                    marker.unlink()
                with patch.dict(os.environ, {'BIN_STORE': str(self.root), **env}, clear=True), patch('sys.stdin', io.StringIO(json.dumps({'session_id': payload}))), patch('sys.stdout', new_callable=io.StringIO) as output, patch('builtins.__import__', guarded), patch.object(Path, 'iterdir', side_effect=AssertionError('ancestry or directory scan')):
                    runpy.run_path(str(ROOT / '.claude/hooks/device-head-notice.py'), run_name='__main__')
                self.assertIn('pinned A; landed B', output.getvalue())

    def test_hook_agrees_with_native_fixture_cases(self):
        cases = json.loads((ROOT / 'genesis/agentic/fixtures/session-cases.json').read_text())
        covered = set()
        for case in cases:
            expected = case['expected']
            # These are the native declaration/runtime cases representable by a
            # prompt hook. TTY fallback and no-runtime results have no hook payload.
            if expected['source'] not in ('declared', 'runtimeAncestor'):
                continue
            runtime = next((p for p in case['processes']
                            if any(p[2] in r['comm'] for r in (case.get('registry') or {}).get('runtimes', []))), None)
            owner = case['env'].get('ELOHIM_SESSION_OWNER_PID')
            if expected['source'] == 'declared' and owner and (not runtime or int(owner) != runtime[0]):
                continue  # plain CI/self declaration is not a prompt-hook runtime
            payload = expected['session'] if expected['source'] == 'runtimeAncestor' else ('payload-Y' if runtime else '')
            env = {**case['env'], 'BIN_STORE': str(self.root)}
            if runtime:
                env['ELOHIM_HOOK_RUNTIME_PID'] = str(runtime[0])
            for pin in (self.tool / 'sessions').glob('*'):
                pin.unlink()
            key = notice.pin_name(expected['session'])
            (self.tool / 'sessions' / key).write_text(json.dumps({'session': expected['session'], 'cid': 'A'}))
            with self.subTest(case=case['name']):
                for marker in (self.tool / 'notices').glob('*'):
                    marker.unlink()
                with patch.dict(os.environ, env, clear=True), patch('sys.stdin', io.StringIO(json.dumps({'session_id': payload}))), patch('sys.stdout', new_callable=io.StringIO) as output, patch.object(Path, 'iterdir', side_effect=AssertionError('runtime discovery')):
                    runpy.run_path(str(ROOT / '.claude/hooks/device-head-notice.py'), run_name='__main__')
                self.assertIn('pinned A; landed B', output.getvalue())
            covered.add(case['name'])
        self.assertTrue({'declared wins', 'ownerless declared', 'ownerless declaration cannot cross runtime'} <= covered)

    def test_installed_command_passes_the_shell_parent_as_runtime(self):
        import subprocess
        settings = json.loads((ROOT / '.claude/settings.json').read_text())
        command = next(hook['command'] for group in settings['hooks']['UserPromptSubmit']
                       for hook in group['hooks'] if 'device-head-notice.py' in hook['command'])
        payload_session = 'payload-Y'
        (self.tool / 'sessions' / notice.pin_name(payload_session)).write_text(
            json.dumps({'session': payload_session, 'cid': 'Y'}))
        for owner, expected in [(os.getpid(), 'A'), (os.getpid() + 100000, 'Y')]:
            with self.subTest(owner=owner):
                for marker in (self.tool / 'notices').glob('*'):
                    marker.unlink()
                env = {key: value for key, value in os.environ.items()
                       if key not in ('PPID', 'ELOHIM_HOOK_RUNTIME_PID')}
                env.update(BIN_STORE=str(self.root), CLAUDE_PROJECT_DIR=str(ROOT),
                           ELOHIM_SESSION_ID=self.session, ELOHIM_SESSION_OWNER_PID=str(owner))
                # Keep sh alive: an exec-optimized shell would make getppid()
                # accidentally equal the runtime and hide a missing installation prefix.
                invocation = command + '\nstatus=$?\nexit "$status"'
                result = subprocess.run(['sh', '-c', invocation], env=env,
                                        input=json.dumps({'session_id': payload_session}),
                                        text=True, capture_output=True, check=True)
                self.assertIn(f'pinned {expected}; landed B', result.stdout)

    def test_long_and_dot_session_keys_are_addressed_without_scanning(self):
        for session in ['.', '..', 'this/session', 'session-' * 40]:
            pin = self.tool / 'sessions' / notice.pin_name(session)
            pin.write_text(json.dumps({'session': session, 'cid': 'A'}))
            with patch.object(Path, 'iterdir', side_effect=AssertionError('directory scan')):
                self.assertEqual(len(notice.notices(self.root, session)), 1)

if __name__ == '__main__':
    unittest.main()
