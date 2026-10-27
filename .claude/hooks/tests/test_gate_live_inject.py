"""gate-live-inject: a gate red reaches every agent at its next tool call, with no subscription."""
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest


REPO = Path(__file__).resolve().parents[3]
HOOK = REPO / '.claude/hooks/gate-live-inject.py'
GATE_LIVE = REPO / 'genesis/orchestrator/gate-live.mjs'
MODULE = runpy.run_path(str(HOOK))


class GateLiveInjectTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        clean = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
        self.env = {**clean, 'CLAUDE_PROJECT_DIR': str(self.root), 'ELOHIM_SESSION_ID': '',
                    'EPR_BIN': str(self.root / 'missing-epr')}

    def live(self, *args):
        result = subprocess.run(['node', str(GATE_LIVE), *args], cwd=self.root, env=self.env,
                                text=True, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def open_run(self, session='s-main'):
        return self.live('open', '--sha', '0' * 40, '--target', 'dev', '--session', session)

    def red(self, run, test='adds'):
        self.live('event', '--run', run, '--json', json.dumps(
            {'kind': 'failure', 'project': 'storage', 'class': 'test', 'test': test,
             'excerpt': f'test {test} ... FAILED'}))

    def hook(self, payload):
        result = subprocess.run([sys.executable, str(HOOK)], cwd=self.root, env=self.env,
                                input=json.dumps(payload), text=True, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        if not result.stdout.strip():
            return None
        out = json.loads(result.stdout)['hookSpecificOutput']
        self.assertEqual(out['hookEventName'], 'PostToolUse')
        return out['additionalContext']

    def test_silent_without_a_run(self):
        self.assertIsNone(self.hook({'session_id': 's-main', 'tool_name': 'Read', 'tool_input': {}}))

    def test_a_red_arrives_once_at_the_next_tool_call_of_any_tool(self):
        run = self.open_run()
        self.red(run)
        context = self.hook({'session_id': 's-main', 'tool_name': 'Read', 'tool_input': {}})
        self.assertIn('[gate-live] RED storage test adds', context)
        self.assertIsNone(self.hook({'session_id': 's-main', 'tool_name': 'Grep', 'tool_input': {}}))
        self.red(run, 'subtracts')
        context = self.hook({'session_id': 's-main', 'tool_name': 'Edit', 'tool_input': {}})
        self.assertIn('subtracts', context)
        self.assertNotIn('test adds', context)

    def test_a_subagent_and_its_parent_each_hear_the_red(self):
        self.red(self.open_run())
        self.assertIn('RED', self.hook({'session_id': 's-main', 'agent_id': 'a1', 'tool_name': 'Read'}))
        self.assertIn('RED', self.hook({'session_id': 's-main', 'tool_name': 'Read'}))

    def test_another_sessions_run_is_not_this_sessions_news(self):
        self.red(self.open_run('s-other'))
        self.assertIsNone(self.hook({'session_id': 's-main', 'tool_name': 'Read'}))

    def test_a_background_push_gets_the_monitor_command_and_a_foreground_one_does_not(self):
        bash = lambda command, background: {'session_id': 's-main', 'tool_name': 'Bash',
                                            'tool_input': {'command': command, 'run_in_background': background}}
        context = self.hook(bash('cd x && GATE_EVIDENCE=reuse git -C w push origin abc:refs/heads/dev', True))
        self.assertIn('gate-live.mjs follow --session s-main', context)
        self.assertIn('select:Monitor', context)
        self.assertIsNone(self.hook(bash('git push origin abc:refs/heads/dev', False)))
        self.assertIsNone(self.hook(bash('git stash push -m tag -- f', True)))

    def test_gate_run_recognizer(self):
        starts = MODULE['starts_gate_run']
        for command in ('git push', 'git -C /w push origin x', 'X=1 git push', 'echo a; just gate eprfs',
                        'node genesis/orchestrator/gate-runner.mjs --target x', '(cd w && git push o s)'):
            self.assertTrue(starts(command), command)
        for command in ('git stash push -m t', 'git log --grep push', 'echo git push', 'just test mesh', ''):
            self.assertFalse(starts(command), command)


if __name__ == '__main__':
    unittest.main()
