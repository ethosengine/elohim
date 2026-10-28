"""Real gate observations must survive a shift without entering product commits."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import shift_ops as ops
import shift_state as state


SOURCE = Path(__file__).resolve().parents[2]
LEDGER = '.claude/data/architecture-findings.jsonl'


class GateObserverTests(unittest.TestCase):
    def test_gate_findings_preserve_source_scope_but_never_enter_the_index(self):
        with tempfile.TemporaryDirectory(prefix='shift-gate-observer-') as temporary:
            root = Path(temporary)
            ledger = root / LEDGER
            ledger.parent.mkdir(parents=True)
            ledger.write_text('')
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            subprocess.run(['git', '-C', str(root), 'add', LEDGER], check=True)
            subprocess.run(['git', '-C', str(root), '-c', 'user.name=Fixture', '-c',
                            'user.email=fixture@example.invalid', 'commit', '-qm', 'baseline'],
                           check=True)
            contract = {
                'worktree': str(root), 'baseline_revision': state.revision(root),
                'objective': {'scope': {'paths': ['product.txt']}},
                'observer_paths': state.read(SOURCE / 'genesis/agentic/shift-observer-paths.json'),
            }
            producer = """
                const { recordCycle } = await import(process.argv[1]);
                recordCycle({name:'fixture', dir:'fixture'}, Number(process.argv[5]), 0,
                  {GATE_RUN_KIND:'full-gate'}, {
                    root:process.argv[2], measuresPath:process.argv[3],
                    ledgerPath:process.argv[4], now:()=>new Date().toISOString(),
                    runEpr:()=>{}, print:()=>{}
                  });
            """
            # Exercise the actual producer: a hard finding, a warm run, then closure.
            for seconds, status, streak in [(1442, 'open', 0), (500, 'open', 1),
                                            (500, 'closed', 2)]:
                subprocess.run([
                    'node', '--input-type=module', '-e', producer,
                    (SOURCE / 'genesis/orchestrator/gate-cycle.mjs').as_uri(), str(root),
                    str(SOURCE / '.claude/epr-meta/measures.yaml'), str(ledger), str(seconds),
                ], check=True, capture_output=True, text=True)
                before = ledger.read_bytes()
                rows = [json.loads(line) for line in before.splitlines()]
                self.assertEqual(len(rows), 1)
                self.assertEqual((rows[0]['status'], rows[0]['under_streak']), (status, streak))
                self.assertTrue(ops.clean(root, contract), 'gate observation became source dirt')
                self.assertEqual(ops.scoped_changes(contract), [])
                self.assertEqual(ledger.read_bytes(), before)

            # The exact exception cannot hide a neighboring source or enter a commit.
            neighbor = ledger.with_name('unexpected-source.json')
            neighbor.write_text('{}')
            self.assertFalse(ops.clean(root, contract))
            with self.assertRaisesRegex(state.Refused, 'outside frozen scope'):
                ops.scoped_changes(contract)
            neighbor.unlink()
            subprocess.run(['git', '-C', str(root), 'add', LEDGER], check=True)
            self.assertFalse(ops.clean(root, contract))
            with self.assertRaisesRegex(state.Refused, 'observer state is staged'):
                ops.scoped_changes(contract)
            self.assertEqual(ledger.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
