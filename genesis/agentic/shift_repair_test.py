"""Actual CLI stand-ins, Git commits and failed oracles exercise bounded repair replay."""
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import shift_ops as ops
import shift_repair as repair
import shift_state as state
import shift_test


class RepairLoop(unittest.TestCase):
    def setUp(self):
        self.fixture = shift_test.ContractFixture(methodName='runTest')
        self.fixture.setUp()
        self.addCleanup(self.fixture.tearDown)
        self.root, self.directory, self.contract = self.fixture.root, self.fixture.directory, self.fixture.contract
        state.run(['git', 'config', 'user.name', 'Fixture'], self.root)
        state.run(['git', 'config', 'user.email', 'fixture@example.invalid'], self.root)
        self.bin = self.directory / 'bin'; self.bin.mkdir()
        self.context_path = self.directory / 'native.json'
        self.native = {'reconciliation': {'scope_cid': 'scope-existing', 'assertions': [{
            'intent': 'intent-existing', 'commitments': [{'commitment': 'promise-existing', 'fulfillments': []}]}]}}
        state.atomic(self.context_path, self.native)
        self.executable('epr', f'from pathlib import Path\nprint(Path({str(self.context_path)!r}).read_text())\n')
        adapter = '''import json,os,sys,uuid
from pathlib import Path
args=sys.argv[1:]; name=Path(sys.argv[0]).name
session=args[args.index('resume')+1] if 'resume' in args else (args[args.index('--resume')+1] if '--resume' in args else (args[args.index('--session-id')+1] if '--session-id' in args else str(uuid.uuid4())))
print(json.dumps({'type':'thread.started','thread_id':session} if name=='codex' else {'type':'system','session_id':session}),flush=True)
attempt=int(os.environ['SHIFT_ATTEMPT']); text=sys.stdin.read()
Path('.claude/shifts/test-shift/prompt-'+str(attempt)+'.txt').write_text(text)
if attempt==1 or not os.environ.get('REPAIR_NO_CHANGE'):
 Path('source.txt').write_text('broken-'+str(attempt) if attempt==1 or os.environ.get('REPAIR_ALWAYS_BAD') else 'fixed')
print(json.dumps({'type':'turn.completed'} if name=='codex' else {'type':'result','session_id':session,'is_error':False}),flush=True)
'''
        self.executable('codex', adapter); self.executable('claude', adapter)
        self.executable('just', "from pathlib import Path\nimport sys\nprint('fixture oracle observes source.txt')\nsys.exit(0 if sys.argv[-1]=='earlier' or Path('source.txt').read_text()=='fixed' else 1)\n")
        self.enterContext(patch.dict(os.environ, PATH=str(self.bin)+os.pathsep+os.environ['PATH']))
        self.agent = {'id': 'implement', 'kind': 'agent', 'prompt': 'Implement the bounded fixture.'}
        self.commit = {'id': 'commit', 'kind': 'commit', 'message': 'fixture delivery'}
        self.gate = {'id': 'verify', 'kind': 'gate', 'project': 'fixture'}
        self.contract.update(steps=[self.agent, self.commit, self.gate], verification=['fixture'],
                             repair={'agent': 'implement', 'commit': 'commit', 'prompt': 'Repair only source.txt from the failed oracle.'})
        self.freeze()

    def executable(self, name, source):
        path = self.bin/name; path.write_text(f'#!{sys.executable}\n'+source); path.chmod(0o700)

    def freeze(self):
        self.fixture.freeze(); self.checkpoint = self.fixture.checkpoint

    def select(self):
        return ops.next_operation(self.directory, self.contract, self.checkpoint)

    def perform(self, expected=None):
        step = self.select()
        if expected:
            self.assertEqual(step['id'], expected)
        pending = ops.prepare(self.directory, self.contract, self.checkpoint, step)
        ops.execute(self.directory, step['id'])
        return step, pending

    def fail_oracle(self, runtime='codex'):
        self.checkpoint['runtime'] = runtime
        state.atomic(self.directory/'checkpoint.json', self.checkpoint)
        for name in ['implement', 'commit', 'verify']:
            self.perform(name)
        path = self.directory/'operations/verify/receipt.json'
        self.assertEqual(state.read(path)['exit'], 1)
        return path

    def test_real_both_adapters_repair_commit_and_verify_with_exact_saved_session(self):
        for runtime in ['codex', 'claude']:
            with self.subTest(runtime=runtime):
                case = RepairLoop(methodName='runTest'); case.setUp()
                try:
                    failed = case.fail_oracle(runtime); original = failed.read_bytes()
                    initial = state.read(case.directory/'operations/implement/receipt.json')
                    step, pending = case.perform('implement')
                    self.assertEqual(pending['resume_session'], initial['session'])
                    self.assertEqual(step['repair_for']['sha256'], state.digest(failed))
                    self.assertIn('Frozen repair instruction', (case.directory/'prompt-2.txt').read_text())
                    case.perform('commit'); case.perform('verify')
                    self.assertIsNone(case.select())
                    self.assertEqual(failed.read_bytes(), original)
                    self.assertEqual(state.read(case.directory/'operations/verify/receipt-2.json')['exit'], 0)
                    self.assertIn('test-shift/commit/2', state.run(['git','show','-s','--format=%B'],case.root))
                finally:
                    case.doCleanups()

    def test_repair_receipt_survives_missing_checkpoint_and_commit_effect_is_reconciled_once(self):
        self.fail_oracle()
        self.perform('implement')  # no checkpoint update is needed to select the commit
        step = self.select(); pending = ops.prepare(self.directory, self.contract, self.checkpoint, step)
        with patch.object(ops, 'save_receipt', side_effect=RuntimeError('crash after commit')):
            with self.assertRaisesRegex(RuntimeError, 'crash after commit'):
                ops.execute(self.directory, 'commit')
        delivered = state.revision(self.root)
        self.checkpoint['runtime'] = 'claude'
        state.atomic(self.directory/'checkpoint.json', self.checkpoint)
        self.assertEqual(self.select()['id'], 'verify')
        recovered = state.read(self.directory/'operations/commit/receipt-2.json')
        self.assertTrue(recovered['recovered'])
        self.assertEqual(recovered['repair_for'], pending['repair_for'])
        self.perform('verify'); self.assertIsNone(self.select())
        self.assertEqual(state.revision(self.root), delivered)

    def test_runtime_switch_starts_fresh_repair_conversation(self):
        self.fail_oracle('codex')
        self.checkpoint['runtime'] = 'claude'
        state.atomic(self.directory/'checkpoint.json', self.checkpoint)
        step, pending = self.perform('implement')
        self.assertIsNone(pending['resume_session'])
        self.assertIsNotNone(pending['session'])
        self.assertEqual(step['repair_for']['attempt'], 1)

    def test_no_source_change_refuses_instead_of_blind_gate_loop(self):
        self.fail_oracle()
        with patch.dict(os.environ, REPAIR_NO_CHANGE='1'):
            self.perform('implement')
        with self.assertRaisesRegex(state.Refused, 'no source change'):
            self.select()
        self.assertFalse((self.directory/'operations/verify/receipt-2.json').exists())

    def test_new_failure_cycles_consume_original_agent_commit_and_oracle_budgets(self):
        with patch.dict(os.environ, REPAIR_ALWAYS_BAD='1'):
            for _ in range(3):
                for name in ['implement', 'commit', 'verify']:
                    self.perform(name)
            with self.assertRaisesRegex(state.Refused, 'iteration budget'):
                self.select()
        for step in [self.agent, self.commit, self.gate]:
            self.assertEqual(len(ops.receipt_history(self.directory,self.contract,step)), 3)

    def test_every_earlier_gate_refreshes_after_repair_commit(self):
        earlier = {'id':'earlier','kind':'gate','project':'earlier'}
        self.contract['steps'].insert(2,earlier); self.contract['verification'].append('earlier'); self.freeze()
        for name in ['implement','commit','earlier','verify','implement','commit','earlier','verify']:
            self.perform(name)
        self.assertIsNone(self.select())
        current=state.revision(self.root)
        self.assertEqual(state.read(self.directory/'operations/earlier/receipt-2.json')['revision'],current)

    def test_missing_or_changed_failure_receipt_refuses_before_repair_execution(self):
        failed=self.fail_oracle(); original=failed.read_bytes()
        step=self.select(); ops.prepare(self.directory,self.contract,self.checkpoint,step)
        for corruption in ['changed','missing']:
            with self.subTest(corruption=corruption):
                if corruption=='changed': failed.write_bytes(original+b'\n')
                else: failed.unlink()
                with self.assertRaises((state.Refused,OSError)):
                    ops.execute(self.directory,'implement')
                failed.write_bytes(original)
        self.assertFalse((self.directory/'operations/implement/dispatch-2.json').exists())

    def test_invalid_prepared_source_digest_refuses_before_agent_dispatch_or_source_change(self):
        self.fail_oracle(); step=self.select(); pending=ops.prepare(self.directory,self.contract,self.checkpoint,step)
        original=(self.root/'source.txt').read_bytes()
        for value in [None, 42, [], 'bad-digest', 'z'*64]:
            altered={**pending,'source_before':value}
            if value is None: del altered['source_before']
            state.atomic(self.directory/'operations/implement/pending.json',altered)
            with self.subTest(value=value), self.assertRaisesRegex(state.Refused,'source_before'):
                ops.execute(self.directory,'implement')
            self.assertFalse((self.directory/'operations/implement/dispatch-2.json').exists())
            self.assertEqual((self.root/'source.txt').read_bytes(),original)

    def test_interrupted_repair_preserves_original_source_digest_and_exact_session(self):
        self.fail_oracle(); step=self.select(); first=ops.prepare(self.directory,self.contract,self.checkpoint,step)
        with patch.object(ops,'save_receipt',side_effect=RuntimeError('crash after repair')):
            with self.assertRaisesRegex(RuntimeError,'crash after repair'):
                ops.execute(self.directory,'implement')
        selected=self.select(); resumed=ops.prepare(self.directory,self.contract,self.checkpoint,selected)
        self.assertEqual(resumed['source_before'],first['source_before'])
        self.assertEqual(resumed['resume_session'],first['resume_session'])
        ops.execute(self.directory,'implement')
        self.perform('commit');self.perform('verify');self.assertIsNone(self.select())

    def test_legacy_first_attempt_commit_trailer_remains_recoverable(self):
        from shift_reconcile import commit_argv
        self.perform('implement'); step=self.select(); pending=ops.prepare(self.directory,self.contract,self.checkpoint,step)
        ops.host_argv(self.directory,self.contract,self.checkpoint,step)  # scoped staging
        argv=commit_argv(self.contract,step,pending['attempt'],legacy=True)
        operation=self.directory/'operations/commit'
        state.atomic(operation/'dispatch-1.json',{'operation':'commit','attempt':1,'argv':argv,
                     'contract_sha256':pending['contract_sha256'],'revision':pending['revision']})
        subprocess.run(argv,cwd=self.root,check=True,capture_output=True)
        self.assertEqual(self.select()['id'],'verify')
        self.assertTrue(state.read(operation/'receipt.json')['recovered'])

    def test_native_fulfillment_and_uncertain_household_or_publication_close_repair(self):
        self.fail_oracle()
        self.native['reconciliation']['assertions'][0]['commitments'][0]['fulfillments']=['native-fulfilled']
        state.atomic(self.context_path,self.native)
        with self.assertRaisesRegex(state.Refused,'native fulfillment'):
            self.select()
        self.native['reconciliation']['assertions'][0]['commitments'][0]['fulfillments']=[]
        state.atomic(self.context_path,self.native)
        for step in [{'id':'boundary','kind':'household'},{'id':'boundary','kind':'lifecycle'},
                     {'id':'boundary','kind':'land'},{'id':'boundary','kind':'measure','artifact':'built'},
                     {'id':'boundary','kind':'native','argv':['epr','flow','fulfill']}]:
            with self.subTest(kind=step['kind']):
                # The frozen declaration in this fixture is read-only to the helper;
                # model a declared boundary without rewriting existing receipt identities.
                contract={**self.contract,'steps':[*self.contract['steps'],step]}
                path=self.directory/'operations/boundary/dispatch-1.json'; state.atomic(path,{'uncertain':True})
                with self.assertRaisesRegex(state.Refused,'repair closed'):
                    repair.require_open(self.directory,contract,self.checkpoint)
                path.unlink()

    def test_live_revocation_stop_and_changed_repair_identity_refuse_dispatch(self):
        self.fail_oracle(); step=self.select(); pending=ops.prepare(self.directory,self.contract,self.checkpoint,step)
        state.atomic(self.directory/'revocation.json',{'revoked':True})
        with self.assertRaisesRegex(state.Refused,'revoked'):
            ops.execute(self.directory,'implement')
        state.atomic(self.directory/'revocation.json',{'revoked':False})
        state.atomic(self.directory/'stop.json',{'reason':'stop'})
        with self.assertRaisesRegex(state.Refused,'stop marker'):
            ops.execute(self.directory,'implement')
        (self.directory/'stop.json').unlink()
        pending['repair_for']['operation']='implement'
        state.atomic(self.directory/'operations/implement/pending.json',pending)
        with self.assertRaisesRegex(state.Refused,'failed host verification'):
            ops.execute(self.directory,'implement')
        self.assertFalse((self.directory/'operations/implement/dispatch-2.json').exists())

    def test_pre_first_commit_objective_failure_repairs_then_returns_to_normal_sequence(self):
        oracle={'id':'objective','kind':'measure','objective':True,'readonly':True}
        self.contract['steps']=[self.agent,oracle,self.commit,self.gate]
        self.contract['objective'].update(measure={'run':"python3 -c \"from pathlib import Path; print(int(Path('source.txt').read_text()=='fixed'))\""},
                                          baseline={'predicate':'>=','value':0},
                                          target={'predicate':'>=','value':1,'stability':{'consecutive':1}})
        self.freeze()
        for name in ['implement','objective','implement','objective','commit','objective','verify']:
            self.perform(name)
        self.assertIsNone(self.select())
        self.assertEqual(len(ops.receipt_history(self.directory,self.contract,self.commit)),1)

    def test_household_failure_keeps_frontier_and_does_not_reopen_agent(self):
        household={'id':'household','kind':'household','argv':[sys.executable,'-c','raise SystemExit(1)'],
                   'oracle':'fixture-only','proof':'.claude/shifts/test-shift/proof.json','binary':{'fixture':True}}
        self.contract['steps']=[self.agent,self.commit,household];self.freeze()
        for name in ['implement','commit','household']:
            self.perform(name)
        with self.assertRaisesRegex(state.Refused,'side effects require independent inspection'):
            self.select()
        self.assertEqual(len(ops.receipt_history(self.directory,self.contract,self.agent)),1)

    def test_refusal_without_executed_verification_marker_is_not_a_repair_instruction(self):
        path=self.fail_oracle(); value=state.read(path)
        value.pop('verification_failure');value['refusal']='fixture authority refusal'
        state.atomic(path,value)
        with self.assertRaisesRegex(state.Refused,'not an authorized repair signal'):
            self.select()
        self.assertFalse((self.directory/'operations/implement/dispatch-2.json').exists())

    def test_frozen_repair_spec_requires_existing_writable_agent_commit_and_prompt(self):
        for spec in [{}, {'agent':'missing','commit':'commit','prompt':'repair'},
                     {'agent':'implement','commit':'verify','prompt':'repair'},
                     {'agent':'implement','commit':'commit','prompt':' '}]:
            with self.subTest(spec=spec),self.assertRaises(state.Refused):
                ops.validate_steps({**self.contract,'repair':spec})
        ops.validate_steps(self.contract)


if __name__=='__main__':
    unittest.main()
