"""Positive implementation fixtures protect census sensitivity, including file selection."""
import importlib.machinery
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
loader = importlib.machinery.SourceFileLoader('dup_census', str(ROOT / 'genesis/agentic/bin/dup-census'))
spec = importlib.util.spec_from_loader(loader.name, loader)
census = importlib.util.module_from_spec(spec)
loader.exec_module(census)


class CensusTest(unittest.TestCase):
    def test_known_implementation_per_family(self):
        fixtures = {
            'session resolvers': ('.claude/hooks/ram-guard.py', 'session = data.get("session_id") or os.environ.get("CLAUDE_SESSION_ID") or "default"'),
            'proc session ancestry': ('.claude/hooks/ram-guard.py', 'def claude_pid():\n    pid = os.getppid()\n    for _ in range(3):\n        with open(f"/proc/{pid}/stat") as f:\n            pid = int(f.read().rsplit(")", 1)[1].split()[1])'),
            'epr/brit lookup paths': ('.claude/scripts/_lib/bin_store.py', 'def resolve_binary(tool, env):'),
            'arrival surfaces': ('genesis/agentic/bin/device-ready', 'roster bound'),
            'follow-set declarations': ('genesis/agentic/bin-tools.json', '"publishedRef": ["refs/heads/dev"]'),
            'runtime-config writers': ('elohim/eprfs/epr-cli/src/peer.rs', 'fn persist_runtime(config: Config)'),
            'head verifiers': ('elohim/eprfs/epr-cli/src/head.rs', 'fn verify_head(head: Head)'),
        }
        self.assertEqual(set(fixtures), set(census.SIGNATURES))
        for family, (path, body) in fixtures.items():
            with self.subTest(family=family):
                self.assertIn(family, census.matching_families(path, body))

    def test_repository_implementations_remain_visible(self):
        examples = {
            'session resolvers': '.claude/scripts/_lib/session.py',
            'proc session ancestry': 'genesis/agentic/bin/berth',
            'epr/brit lookup paths': '.claude/scripts/_lib/epr_client.py',
            'arrival surfaces': 'elohim/eprfs/epr-cli/src/device/arrival.rs',
            'follow-set declarations': 'genesis/agentic/bin-tools.json',
            'runtime-config writers': 'elohim/elohim-settings/src/runtime_config.rs',
            'head verifiers': 'elohim/holochain/dna/elohim/zomes/content_store/src/head_delegation.rs',
        }
        for family, path in examples.items():
            with self.subTest(family=family):
                self.assertIn(family, census.matching_families(path, (ROOT / path).read_text()))

    def test_mjs_selected_and_tests_excluded(self):
        self.assertIn('session resolvers', census.matching_families('.claude/scripts/_lib/session.mjs', 'function resolveFallback(env) {'))
        self.assertIn('follow-set declarations', census.matching_families('genesis/agentic/follow.mjs', 'const source = {publishedRef: []};'))
        self.assertEqual([], census.matching_families('genesis/agentic/follow.test.mjs', 'publishedRef'))


if __name__ == '__main__':
    unittest.main()
