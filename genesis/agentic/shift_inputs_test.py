"""Private executable dependencies remain frozen even though Git ignores their files."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
import shift
import shift_inputs as inputs
import shift_ops as ops
import shift_state as state
import shift_initialization_test as initialization_fixture
import shift_test as execution_fixture


class PrivateInputs(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='shift-inputs-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.helper = self.root / '.claude/shifts/fixture/measure.py'
        self.helper.parent.mkdir(parents=True)
        self.helper.write_text('print(1)\n')
        self.label = self.helper.relative_to(self.root).as_posix()
        self.contract = {'worktree': str(self.root),
                         'objective': {'measure': {'run': f'python3 {self.label}'}}, 'steps': []}

    def test_direct_private_script_requires_explicit_pin_in_objective_and_host_argv(self):
        for command in [f'python3 {self.label}', f'FLAG=yes python3 -u {self.label}',
                        f'env FLAG=yes node {self.label}', f'sh -c "python3 {self.label}"', self.label,
                        'python3 .claude//shifts/fixture/measure.py']:
            with self.subTest(command=command), self.assertRaisesRegex(state.Refused, 'absent from input_files'):
                inputs.verify_inputs({**self.contract, 'objective': {'measure': {'run': command}}})
        for argv in [[self.label], ['python3', self.label], ['node', self.label], ['bash', self.label]]:
            with self.subTest(argv=argv), self.assertRaisesRegex(state.Refused, 'absent from input_files'):
                inputs.verify_inputs({'worktree': str(self.root), 'steps': [{'argv': argv}]})
        self.contract['input_files'] = inputs.normalize_inputs(self.root, [self.label])
        inputs.verify_inputs(self.contract)

    def test_declared_private_imports_are_pinned_with_the_direct_helper(self):
        imported = self.helper.with_name('oracle_support.py')
        imported.write_text('EXPECTED = 1\n')
        labels = [self.label, imported.relative_to(self.root).as_posix()]
        self.contract['input_files'] = inputs.normalize_inputs(self.root, labels)
        inputs.verify_inputs(self.contract)
        imported.write_text('EXPECTED = 0\n')
        with self.assertRaisesRegex(state.Refused, 'private execution input changed'):
            inputs.verify_inputs(self.contract)

    def test_duplicate_absolute_outside_private_and_missing_input_paths_refuse(self):
        tracked = self.root / 'source.py'
        tracked.write_text('print(1)\n')
        for paths in [[self.label, './' + self.label], [str(self.helper)], ['../escape.py'],
                      ['source.py'], ['.claude/shifts/fixture/missing.py']]:
            with self.subTest(paths=paths), self.assertRaises(state.Refused):
                inputs.normalize_inputs(self.root, paths)

    def test_outputs_native_reports_and_editable_source_scripts_are_not_pinned_as_inputs(self):
        source = self.root / 'source.py'
        source.write_text('print(1)\n')
        contract = {'worktree': str(self.root), 'objective': {'measure': {'run': 'python3 source.py'}},
                    'steps': [{'argv': ['epr', 'flow', 'fulfill', '--report', self.label]},
                              {'argv': ['python3', 'source.py', '--output', self.label]}]}
        inputs.verify_inputs(contract)
        source.write_text('print(2)\n')
        inputs.verify_inputs(contract)

    def test_corrupt_or_removed_frozen_pin_refuses(self):
        self.contract['input_files'] = inputs.normalize_inputs(self.root, [self.label])
        for pins in [None, ['wrong'], [{'path': self.label}], [],
                     [{'path': self.label, 'sha256': 'wrong'}]]:
            with self.subTest(pins=pins), self.assertRaises(state.Refused):
                inputs.verify_inputs({**self.contract, 'input_files': pins})
        self.helper.unlink()
        with self.assertRaisesRegex(state.Refused, 'input missing'):
            inputs.verify_inputs(self.contract)

    def test_changed_private_helper_refuses_initialization_retry_and_readiness(self):
        fixture = initialization_fixture.Initialization(methodName='runTest')
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        helper = fixture.directory / 'measure.py'
        helper.write_text('print(1)\n')
        label = helper.relative_to(fixture.root).as_posix()
        fixture.spec['input_files'] = [label]
        objective = state.read(fixture.objective)
        objective['measure'] = {'run': f'python3 {label}'}
        state.atomic(fixture.objective, objective)
        state.atomic(fixture.declaration, fixture.spec)
        fixture.fail_readiness()
        frozen = (fixture.directory / 'initialization.json').read_bytes()
        self.assertEqual(state.read(fixture.directory / 'initialization.json')['spec']['input_files'],
                         [{'path': label, 'sha256': state.digest(helper)}])
        helper.write_text('print(0)\n')
        with self.assertRaisesRegex(state.Refused, 'private execution input changed'):
            shift.initialization_remaining(fixture.directory)
        with self.assertRaisesRegex(state.Refused, 'initialization declaration'):
            shift.start(fixture.declaration, 'codex')
        self.assertEqual((fixture.directory / 'initialization.json').read_bytes(), frozen)
        self.assertEqual(len(fixture.attempts), 1)

    def test_changed_private_helper_refuses_recovery_and_dispatch_despite_clean_git_head(self):
        fixture = execution_fixture.ContractFixture(methodName='runTest')
        fixture.setUp()
        self.addCleanup(fixture.tearDown)
        helper = fixture.directory / 'measure.py'
        helper.write_text('print(1)\n')
        label = helper.relative_to(fixture.root).as_posix()
        fixture.contract['input_files'] = inputs.normalize_inputs(fixture.root, [label])
        fixture.freeze()
        original_revision = state.revision(fixture.root)
        self.assertTrue(ops.clean(fixture.root))
        helper.write_text('print(0)\n')
        self.assertTrue(ops.clean(fixture.root))
        self.assertEqual(state.revision(fixture.root), original_revision)
        with self.assertRaisesRegex(state.Refused, 'private execution input changed'):
            shift.resume(fixture.directory)
        with self.assertRaisesRegex(state.Refused, 'private execution input changed'):
            ops.prepare(fixture.directory, fixture.contract, fixture.checkpoint, fixture.contract['steps'][0])
        with self.assertRaisesRegex(state.Refused, 'private execution input changed'):
            state.load(fixture.directory)


if __name__ == '__main__':
    unittest.main()
