"""SessionStart caches project headlines and bounds synchronous bootstrap recall."""
import importlib.util
import json
import os
import signal
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from concurrent.futures import ThreadPoolExecutor
from types import SimpleNamespace
from unittest.mock import patch

HOOK = Path(__file__).resolve().parents[1] / 'load-project-context.py'
SPEC = importlib.util.spec_from_file_location('context_hook', HOOK)
HOOK_MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HOOK_MODULE)


class ProjectContextTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.cache = self.root / 'context.json'
        self.identity = {'session': 'session-one', 'ownerPid': 123}

    def cached(self):
        with patch.object(HOOK_MODULE, '_context_cache_path', return_value=self.cache):
            return HOOK_MODULE._cached_context(str(self.root), self.identity)

    def store(self, age=0, text='native context'):
        self.cache.write_text(json.dumps({'at': time.time() - age, 'context': text}))

    def test_fresh_cache_needs_no_subprocess(self):
        self.store()
        with patch.object(subprocess, 'Popen') as spawn:
            self.assertIn('native context', self.cached())
        spawn.assert_not_called()

    def test_stale_cache_lands_with_age_and_detached_owner_bound_refresh(self):
        self.store(age=180)
        with patch.object(subprocess, 'Popen') as spawn:
            text = self.cached()
        self.assertIn('last completed pass 180s ago; refresh pending', text)
        self.assertIn('native context', text)
        kwargs = spawn.call_args.kwargs
        self.assertTrue(kwargs['start_new_session'])
        for stream in ('stdin', 'stdout', 'stderr'):
            self.assertEqual(kwargs[stream], subprocess.DEVNULL)
        self.assertEqual(kwargs['env']['ELOHIM_SESSION_ID'], 'session-one')
        self.assertEqual(kwargs['env']['ELOHIM_SESSION_OWNER_PID'], '123')

    def test_absent_corrupt_and_expired_cache_are_honest_absence(self):
        for content in (None, '{', '[]', json.dumps({'at': 0, 'context': 'too old'})):
            with self.subTest(content=content):
                if content is not None:
                    self.cache.write_text(content)
                with patch.object(subprocess, 'Popen'):
                    text = self.cached()
                self.assertIn('refresh pending (no completed pass)', text)
                self.assertNotIn('too old', text)

    def test_failed_refresh_preserves_previous_report(self):
        self.store(age=180)
        before = self.cache.read_bytes()
        with patch.object(HOOK_MODULE, '_render_context', side_effect=OSError('unavailable')):
            HOOK_MODULE._refresh_context(str(self.root), self.cache)
        self.assertEqual(self.cache.read_bytes(), before)
        with patch.object(HOOK_MODULE, '_render_context', return_value=''):
            HOOK_MODULE._refresh_context(str(self.root), self.cache)
        self.assertEqual(self.cache.read_bytes(), before)

    def test_new_session_reuses_project_headlines_without_another_sessions_recall(self):
        self.store(age=60, text='SAGA: measured frontier')
        self.identity['session'] = 'session-two'
        with patch.object(subprocess, 'Popen') as spawn:
            text = self.cached()
        spawn.assert_not_called()
        self.assertIn('last completed pass 60s ago', text)
        self.assertIn('SAGA: measured frontier', text)
        self.assertNotIn('BOOTSTRAP:', text)

    def test_cache_identity_separates_project_and_executor(self):
        key = HOOK_MODULE._context_cache_path
        a = key('/project/a')
        self.assertNotEqual(a, key('/project/b'))
        with patch.dict(os.environ, {'ELOHIM_SESSION_ID': 'session-two'}):
            self.assertEqual(a, key('/project/a'))
        with patch.dict(os.environ, {'EPR_BIN': '/new/executor'}):
            self.assertNotEqual(a, key('/project/a'))

    def test_recall_budget_kills_the_group_including_inherited_pipes(self):
        native = self.root / 'slow-epr'
        native.write_text('''#!/usr/bin/env python3
import subprocess, sys, time
subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)'])
time.sleep(30)
''')
        native.chmod(0o755)
        obs = SimpleNamespace(resolve_bin=lambda: str(native),
                              bootstrap_session_id=lambda root, data: 'timeout-session')
        with patch.object(HOOK_MODULE, '_observation_module', return_value=obs), \
                patch.object(HOOK_MODULE, 'RECALL_TIMEOUT_S', 0.2), \
                patch.object(subprocess, 'Popen', wraps=subprocess.Popen) as spawn, \
                patch.object(os, 'killpg', wraps=os.killpg) as kill_group:
            started = time.monotonic()
            text = HOOK_MODULE._epr_bootstrap_block(str(self.root), {})
            elapsed = time.monotonic() - started
        self.assertEqual(text, 'bootstrap: skipped — recall open exceeded the 0.2s budget')
        self.assertLess(elapsed, 2)
        self.assertTrue(spawn.call_args.kwargs['start_new_session'])
        kill_group.assert_called_once()
        self.assertEqual(kill_group.call_args.args[1], signal.SIGKILL)

    def test_slow_cold_refresh_is_single_flight_and_next_hook_receives_all_context(self):
        hooks = self.root / '.claude/hooks'
        hooks.mkdir(parents=True)
        (hooks / '_observation.py').write_text('''import os
def resolve_bin(): return os.environ['FAKE_EPR']
def bootstrap_session_id(root, data): return os.environ['ELOHIM_SESSION_ID']
''')
        scripts = self.root / '.claude/scripts'
        scripts.mkdir()
        (scripts / 'saga-status.py').write_text("print('SAGA: measured frontier')\n")
        (self.root / '.claude/file-relationships.json').write_text('{"schemas": {}}')
        native = self.root / 'fake-epr'
        native.write_text('''#!/usr/bin/env python3
import os, sys, time
from pathlib import Path
with open(os.environ['CALL_LOG'], 'a') as log: log.write(' '.join(sys.argv[1:])+'\\n')
while 'report' in sys.argv and Path(os.environ['REFRESH_HOLD']).exists(): time.sleep(.02)
time.sleep(.6)
if 'report' in sys.argv: print('MEMORY BUDGET: measured headline')
else: print('Bootstrap: native session '+sys.argv[sys.argv.index('--session')+1])
''')
        native.chmod(0o755)
        registry = self.root / 'runtimes.json'
        registry.write_text('{"runtimes": []}')
        log = self.root / 'calls'
        hold = self.root / 'hold-refresh'
        hold.touch()
        env = {**os.environ, 'CLAUDE_PROJECT_DIR': str(self.root), 'TMPDIR': str(self.root),
               'ELOHIM_SESSION_ID': 'session-fixture', 'ELOHIM_SESSION_OWNER_PID': '',
               'EPR_AGENT_RUNTIMES': str(registry), 'FAKE_EPR': str(native), 'CALL_LOG': str(log),
               'REFRESH_HOLD': str(hold)}

        def invoke(_=None):
            started = time.monotonic()
            result = subprocess.run([sys.executable, str(HOOK)], env=env, input='{}',
                                    capture_output=True, text=True, timeout=3)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertLess(time.monotonic() - started, 4)
            output = json.loads(result.stdout)['hookSpecificOutput']
            self.assertEqual(output['hookEventName'], 'SessionStart')
            return output['additionalContext']

        try:
            with ThreadPoolExecutor(max_workers=4) as pool:
                cold = list(pool.map(invoke, range(4)))
        finally:
            hold.unlink()
        self.assertTrue(all('refresh pending' in text for text in cold))
        self.assertTrue(all('BOOTSTRAP:\nBootstrap: native session session-fixture' in text
                            for text in cold))
        deadline = time.monotonic() + 8
        while not list(self.root.glob('claude-project-context-*.json')) and time.monotonic() < deadline:
            time.sleep(.05)
        env['ELOHIM_SESSION_ID'] = 'session-next'
        warm = invoke()
        self.assertIn('MEMORY BUDGET: measured headline', warm)
        self.assertIn('SAGA: measured frontier', warm)
        self.assertIn('Bootstrap: native session session-next', warm)
        self.assertNotIn('session-fixture', warm)
        self.assertIn('SYNC HOOKS ACTIVE', warm)
        calls = log.read_text().splitlines()
        self.assertEqual(sum('report' in call for call in calls), 1,
                         'only one detached headline render may run')
        self.assertEqual(sum('recall open' in call for call in calls), 5,
                         'every hook invocation must render bootstrap synchronously')
        cache = next(self.root.glob('claude-project-context-*.json'))
        self.assertNotIn('Bootstrap:', cache.read_text())


if __name__ == '__main__':
    unittest.main()
