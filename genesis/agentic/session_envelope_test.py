"""Isolated native envelope proof; the a2o steps drive these same state transitions."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
BINARY = Path(os.environ.get('EPR_BIN', '/var/tmp/eprfs-a2-target/debug/epr')).resolve()
RUNTIMES = {'claude': ('claude-code', 4101), 'codex': ('codex', 4102)}


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


class Fixture:
    def __init__(self, base):
        self.base = Path(base)
        self.root = self.base / 'repo'

    def env(self, runtime='claude'):
        _, pid = RUNTIMES[runtime]
        return {**{k: v for k, v in os.environ.items() if not k.startswith(('EPR_', 'ELOHIM_', 'GIT_', 'CLAUDE_', 'CODEX_', 'BIN_STORE', 'BERTH_', 'BRIT_'))},
                'EPR_BIN': str(self.base / 'hook-bin/epr'), 'EPR_BIN_TOOLS': str(self.base / 'registry.json'),
                'EPR_AGENT_RUNTIMES': str(REPO / 'genesis/agentic/agent-runtimes.json'),
                'BIN_STORE': str(self.base / 'stores'), 'EPR_PROC_ROOT': str(self.base / 'proc'),
                'XDG_CONFIG_HOME': str(self.base / 'config-home'),
                'CLAUDE_PROJECT_DIR': str(self.root), 'CLAUDE_CONFIG_DIR': str(self.base / 'config'),
                'BERTH_DIR': str(self.base / 'berth'), 'BERTH_UNDER_TEST': '1',
                'BRIT_BUILD_REF_BIN': str(self.base / 'brit-stub'), 'FIXTURE_OWNER': str(pid),
                'GIT_CONFIG_GLOBAL': '/dev/null', 'GIT_CONFIG_NOSYSTEM': '1',
                'GIT_AUTHOR_NAME': 'Fixture', 'GIT_COMMITTER_NAME': 'Fixture',
                'GIT_AUTHOR_EMAIL': 'fixture@example.test', 'GIT_COMMITTER_EMAIL': 'fixture@example.test'}

    def run(self, args, runtime='claude', expected=0, native=True, extra_env=None):
        argv = [sys.executable, str(self.base / 'exec.py'), *map(str, args)] if native else list(map(str, args))
        result = subprocess.run(argv, cwd=self.root, env={**self.env(runtime), **(extra_env or {})}, text=True, capture_output=True, timeout=45)
        assert result.returncode == expected, (argv, result.returncode, result.stdout, result.stderr)
        return result.stdout

    def epr(self, *args, runtime='claude', expected=0):
        out = self.run([self.base / 'launch/epr', *args], runtime, expected)
        return json.loads(out) if out.strip() else None

    def setup(self):
        self.root.mkdir(parents=True, exist_ok=True)
        # An exec wrapper supplies a fake registered ancestor to every native command/shim.
        write(self.base / 'exec.py', '''import os, pathlib, sys
root=pathlib.Path(os.environ['EPR_PROC_ROOT'])
pid=os.getpid(); parent=int(os.environ['FIXTURE_OWNER'])
fields=['0']*20; fields[0]='S'; fields[1]=str(parent); fields[19]='99'
p=root/str(pid); p.mkdir(parents=True,exist_ok=True)
(p/'stat').write_text(f'{pid} (fixture) '+ ' '.join(fields))
os.execv(sys.argv[1], sys.argv[1:])
''')
        write(self.base / 'proc/self/ns/pid', 'fixture namespace')
        write(self.base / 'proc/sys/kernel/random/boot_id', 'fixture boot')
        for runtime, (name, pid) in RUNTIMES.items():
            fields = ['0'] * 20
            fields[0], fields[19] = 'S', '77'
            write(self.base / f'proc/{pid}/stat', f'{pid} ({runtime}) ' + ' '.join(fields))
            write(self.base / f'proc/{pid}/environ', '')
        write(self.base / 'registry.json', json.dumps({'version': 1, 'tools': {'epr': {'bins': ['epr']}}}))
        write(self.base / 'brit-stub', '#!/usr/bin/env python3\nimport json\nprint("[]")\n')
        (self.base / 'brit-stub').chmod(0o755)
        write(self.root / '.epr-meta/repository.yaml', 'version: 1\nagent: repo:fixture/envelope\n')
        write(self.root / '.epr-meta/manifest.md', '---\nepr-meta-version: 1\nid: fixture\nroot: true\n---\n')
        write(self.root / 'docs/spec.md', '---\nid: envelope-fixture\ntitle: Session work\n---\nSeparate work for two sessions.\n')
        write(self.root / 'gap/spec.json', json.dumps({'doc':'docs/spec.md','slug':'envelope','items':[{'id':'envelope#1','state':'OPEN'},{'id':'envelope#2','state':'OPEN'}]}))
        write(self.root / '.claude/epr-meta/recipes.yaml', '''version: 1
recipes:
  - id: fixture
    version: 1
    description: fixture
    stages:
      - {name: spec, artifactKind: 'doc:spec', paths: ['docs/*.md']}
      - {name: intent, artifactKind: gap-item, paths: ['gap/*.json']}
    edges: []
''')
        contract = '.epr-meta/elohim/algorithms/recall-contract.json'
        write(self.root / contract, (REPO / contract).read_text())
        write(self.root / 'genesis/manifests/habits.yaml', 'habits: []\n')
        shutil.copytree(REPO/'.epr-meta/elohim/lenses', self.root/'.epr-meta/elohim/lenses', dirs_exist_ok=True)
        # Berth remains its existing authority; only the fixture supplies its installation path.
        (self.root / 'genesis/agentic/bin').mkdir(parents=True)
        (self.root / 'genesis/agentic/bin/berth').symlink_to(REPO / 'genesis/agentic/bin/berth')
        for args in [('init','-q'), ('add','.epr-meta','docs','gap','.claude'), ('commit','-qm','fixture')]:
            self.run(['git', *args], native=False)
        self.run([BINARY, 'actor','witness','--subject','human:matthew','--as','agent:reader@fixture','--session','enrollment','--basis','Fixture person present','--json'])
        candidate = self.base / 'cargo-target/debug/epr'
        candidate.parent.mkdir(parents=True)
        shutil.copy2(BINARY, candidate)
        cid = json.loads(self.run([BINARY,'bin','ingest',candidate,'--tool','epr','--source-commit',self.run(['git','rev-parse','HEAD'],native=False).strip(),'--tree-clean','true','--gate-run','fixture','--repo',self.root,'--json']))['cid']
        write(self.base / 'cid', cid)
        (self.base / 'launch').mkdir()
        shutil.copy2(BINARY.with_name('bin-launch'), self.base / 'launch/epr')
        write(self.base / 'hook-bin/epr', f'#!/bin/sh\nexec {sys.executable} {self.base}/exec.py {self.base}/launch/epr "$@"\n')
        (self.base / 'hook-bin/epr').chmod(0o755)
        for runtime in RUNTIMES:
            self.run([BINARY,'bin','pin','--tool','epr','--cid',cid,'--json'], runtime)
        # Unpinned context projection is set up before the launcher lazily opens an envelope.
        self.run([BINARY,'flow','project','--root',self.root,'--json'])

    def id(self, runtime):
        name, pid = RUNTIMES[runtime]
        return f'{name}-{pid}-77'

    def acquire(self):
        for n, runtime in enumerate(RUNTIMES, 1):
            first = self.epr('session','open','--model',f'fixture-{runtime}','--json',runtime=runtime)
            write(self.base/f'open-{runtime}.json', json.dumps(first))
            second = self.epr('session','open','--json',runtime=runtime)
            assert first == second
            self.run([REPO/'genesis/agentic/bin/berth','moor'],runtime)
            self.run([REPO/'genesis/agentic/bin/berth','claim',f'fixture-{runtime}'],runtime)
            self.epr('actor','claim','--as',f'agent:implementer@fixture-{runtime}','--json',runtime=runtime)
            self.epr('flow','claim','--on',f'envelope#{n}','--json',runtime=runtime)

    def joined(self):
        for runtime in RUNTIMES:
            view = self.epr('session','show','--json',runtime=runtime)
            assert view['envelope']['identity']['session'] == self.id(runtime)
            assert view['recallSession'] == self.id(runtime)
            assert list(view['berth']['leases']) == [f'fixture-{runtime}'], view
            assert len(view['claims']) == 1, view
            assert f'session:{self.id(runtime)}' in view['claims'][0]['commitment']['resourceSpec']['classifiedAs']
            assert view['pinnedTools']['epr'] == (self.base/'cid').read_text()
            write(self.base/f'view-{runtime}.json',json.dumps(view))

    def close_claude(self):
        first = self.epr('session','close','--json')
        write(self.base/'close.json', json.dumps(first))
        second = self.epr('session','close','--json')
        assert first == second and first['witness'] and first['covenantSeal']
        seal = self.base / f"config-home/elohim/device/sessions/seals/{first['covenantSeal']}.json"
        assert json.loads(seal.read_text())['session'] == self.id('claude')

    def claude_closed(self):
        leases = json.loads((self.base/'berth/leases.json').read_text())
        assert 'fixture-claude' not in leases and leases['fixture-codex']['holder'] == self.id('codex')
        self.witnesses(1)

    def collect(self):
        shutil.rmtree(self.base/'proc/4102')
        first = self.epr('session','gc','--json')
        write(self.base/'gc.json', json.dumps(first))
        assert len(first['closed']) == 1
        assert self.epr('session','gc','--json')['closed'] == []

    def all_closed(self):
        assert not json.loads((self.base/'berth/leases.json').read_text())
        for runtime in RUNTIMES:
            record = json.loads((self.base/f'config-home/elohim/device/sessions/{self.id(runtime)}.json').read_text())
            assert record['closedAt'] and record['covenantSeal'] and record['witness']
        self.witnesses(2)

    def witnesses(self, count):
        rows = (self.root/'.eprfs/status/flows.jsonl').read_text().splitlines()
        assert sum('session ' in row and ' closed; opened ' in row for row in rows) == count

    def hooks(self):
        self.epr('session','open','--json')
        script = '''import importlib.util,sys
sys.path.insert(0,sys.argv[1]+'/.claude/hooks')
def load(name,path):
 spec=importlib.util.spec_from_file_location(name,path); m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
obs=load('_observation',sys.argv[1]+'/.claude/hooks/_observation.py')
print(obs.bootstrap_session_id(sys.argv[2],{'session_id':'wrong-hook-id'}))
if sys.argv[3]=='start':
 hook=load('start',sys.argv[1]+'/.claude/hooks/load-project-context.py')
 hook._observation_module=lambda _:obs
 out=hook._epr_bootstrap_block(sys.argv[2],{'session_id':'wrong-hook-id'})
else:
 hook=load('prompt',sys.argv[1]+'/.claude/hooks/run-projection.py')
 hook._observation_module=lambda:obs
 from pathlib import Path
 out='\\n'.join(hook.run_plane_lines(Path(sys.argv[2]),{'session_id':'another-wrong-id'}))
print(out)
assert 'executor bytes changed' not in out,out
assert 'bootstrap: skipped' not in out,out
'''
        hook = self.base/'hook.py'; write(hook, script)
        first = self.run([sys.executable,hook,REPO,self.root,"start"])
        state_path = self.root/f'.eprfs/status/recall/{self.id("claude")}/continuation.json'
        before = json.loads(state_path.read_text())['executor_digest']
        # Cargo installs a freshly linked executable by replacement, not in-place writes.
        rebuilt = self.base/'cargo-target/debug/rebuilt'
        shutil.copy2(BINARY,rebuilt)
        with rebuilt.open('ab') as f: f.write(b'\nrebuilt cargo target\n')
        rebuilt.replace(self.base/'cargo-target/debug/epr')
        second = self.run([sys.executable,hook,REPO,self.root,"prompt"])
        assert json.loads(state_path.read_text())["executor_digest"] == before == (self.base/"cid").read_text()
        assert first.splitlines()[0] == second.splitlines()[0] == self.id('claude')
        states = list((self.root/f'.eprfs/status/recall/{self.id("claude")}').glob('*.json'))
        contents = '\n'.join(p.read_text() for p in states)
        assert (self.base/'cid').read_text() in contents, contents
        assert 'executor bytes changed' not in first+second
        write(self.base/'hook-proof.json',json.dumps({'session':self.id('claude'),'executor':(self.base/'cid').read_text()}))


class EnvelopeTest(unittest.TestCase):
    def test_two_runtimes_close_and_dead_owner_is_collected(self):
        with tempfile.TemporaryDirectory(prefix='envelope-') as base:
            f=Fixture(base); f.setup(); f.acquire(); f.joined(); f.close_claude(); f.claude_closed(); f.collect(); f.all_closed()

    def test_model_is_descriptive_and_later_actor_claim_is_authoritative(self):
        with tempfile.TemporaryDirectory(prefix='envelope-tier-') as base:
            f=Fixture(base); f.setup()
            f.epr('session','open','--model','human','--json')
            assert f.epr('session','show','--json')['capabilityTier'] == 'unknown'
            f.epr('actor','claim','--as','agent:implementer@haiku','--json')
            view=f.epr('session','show','--json')
            assert view['envelope']['model'] == 'human'
            assert view['capabilityTier'] == 'haiku'

    def test_gc_preserves_unknown_and_foreign_namespace_but_collects_a_previous_boot(self):
        with tempfile.TemporaryDirectory(prefix='envelope-gc-') as base:
            f=Fixture(base); f.setup()
            opened=f.epr('session','open','--json')
            assert opened['ownerBootId']=='fixture boot' and opened['ownerPidNamespace']
            stat=f.base/'proc/4101/stat'
            original=stat.read_text()
            for action in ['missing','malformed','unreadable']:
                stat.unlink(missing_ok=True)
                if action=='malformed': stat.write_text('malformed')
                if action=='unreadable': stat.mkdir()
                assert json.loads(f.run([BINARY,'session','gc','--json']))['closed']==[]
                if action=='unreadable': stat.rmdir()
            stat.write_text(original)
            namespace=f.base/'proc/self/ns/pid'
            namespace.rename(namespace.with_suffix('.original'))
            namespace.write_text('another namespace')
            shutil.rmtree(stat.parent)
            assert json.loads(f.run([BINARY,'session','gc','--json']))['closed']==[]
            write(f.base/'proc/sys/kernel/random/boot_id','another boot')
            closed=json.loads(f.run([BINARY,'session','gc','--json']))['closed']
            assert len(closed)==1 and closed[0]['identity']['session']==f.id('claude')
            f.witnesses(1)
            assert json.loads(f.run([BINARY,'session','gc','--json']))['closed']==[]

    def test_reused_session_owner_cannot_read_close_or_mutate(self):
        with tempfile.TemporaryDirectory(prefix='envelope-owner-') as base:
            f=Fixture(base); f.setup(); f.acquire()
            path=f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json'
            before=path.read_bytes()
            stat=f.base/'proc/4101/stat'
            stat.write_text(stat.read_text().removesuffix('77')+'78')
            for args in [('session','open'), ('session','show'), ('session','close'),
                         ('session','validate','--session',f.id('claude')),
                         ('flow','claim','--on','envelope#1'),
                         ('covenant','record'), ('actor','claim','--as','agent:reader@haiku')]:
                result=subprocess.run([sys.executable,str(f.base/'exec.py'),str(BINARY),*args,'--json'],
                    cwd=f.root,env={**f.env(),'CLAUDE_SESSION_ID':f.id('claude')},capture_output=True,text=True)
                assert result.returncode != 0 and 'envelope-owner-mismatch' in result.stderr, result
            assert path.read_bytes() == before

    def test_harness_model_caps_the_real_envelope_claim(self):
        with tempfile.TemporaryDirectory(prefix='envelope-harness-') as base:
            f=Fixture(base); f.setup()
            write(f.root/'.claude/epr-meta/policies.yaml', (REPO/'.claude/epr-meta/policies.yaml').read_text())
            f.epr('session','open','--model','human','--json')
            def gate(model=None, **markers):
                payload={'tool_name':'Bash','tool_input':{'command':'git reset --hard'},**markers}
                if model is not None: payload['model']=model
                r=subprocess.run([sys.executable,str(f.base/'exec.py'),sys.executable,
                    str(REPO/'.claude/hooks/capability-tier-gate.py')], input=json.dumps(payload),
                    cwd=f.root,env={**f.env(),'CLAUDE_MODEL':'human'},capture_output=True,text=True,timeout=15)
                assert r.returncode==0, r.stderr
                assert 'skipped' not in r.stderr, r.stderr
                return r.stdout
            # No claim: only the harness ceiling. Neither envelope nor env grants human.
            assert 'claude-haiku-4-5' in gate('claude-haiku-4-5')
            assert gate('claude-opus-5') == ''
            f.epr('actor','claim','--as','human:matthew','--json')
            assert f.epr('session','show','--json')['capabilityTier']=='human'
            assert gate() == '', 'main agent without model retains its own claim'
            for marker in ('agent_id', 'agent_type', 'subagent_id', 'subagent_type'):
                assert 'permissionDecision' in gate(**{marker:'child'}), marker

            assert 'claude-haiku-4-5' in gate('claude-haiku-4-5')
            assert gate('claude-opus-5') == ''
            path=f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json'
            record=json.loads(path.read_text()); record['model']='claude-haiku-4-5'
            path.write_text(json.dumps(record))
            assert gate('claude-opus-5') == '', 'descriptive envelope model cannot lower capability'
            # A lower claim remains a bound even when a higher model executes.
            f.epr('actor','claim','--as','agent:reader@claude-haiku-4-5','--json')
            assert 'claude-haiku-4-5' in gate('claude-opus-5')

    def test_facet_destination_must_match_bound_checkout(self):
        with tempfile.TemporaryDirectory(prefix='envelope-checkout-') as base:
            f=Fixture(base); f.setup()
            f.epr('session','open','--json')
            f.epr('actor','claim','--as','human:matthew','--json')
            f.run([BINARY,'actor','claim','--as','agent:reader@claude-haiku-4-5',
                   '--root',f.root/'docs','--json'])
            assert not (f.root/'docs/.eprfs').exists()
            assert f.epr('session','show','--json')['capabilityTier']=='claude-haiku-4-5'
            other=f.base/'other-repo'; other.mkdir()
            subprocess.run(['git','init','-q',str(other)],check=True,env=f.env())
            before={str(p.relative_to(other)):p.read_bytes() for p in other.rglob('*') if p.is_file()}
            for args in [('actor','claim','--as','agent:reader@haiku','--root',other),
                         ('covenant','record','--repo',other)]:
                r=subprocess.run([sys.executable,str(f.base/'exec.py'),str(BINARY),*map(str,args),'--json'],
                    cwd=f.root,env=f.env(),capture_output=True,text=True)
                assert r.returncode != 0 and 'envelope-checkout-mismatch' in r.stderr, r
            assert before=={str(p.relative_to(other)):p.read_bytes() for p in other.rglob('*') if p.is_file()}
            # A subdirectory and a symlink resolving to the same checkout remain valid.
            alias=f.base/'alias'; alias.symlink_to(f.root, target_is_directory=True)
            f.run([BINARY,'actor','claim','--as','agent:reader@haiku','--root',alias/'docs','--json'])
            assert not (f.root/'docs/.eprfs').exists()
            assert f.epr('session','show','--json')['capabilityTier']=='haiku'
            f.run([BINARY,'flow','claim','--on','envelope#1','--root',alias/'docs','--json'])
            assert len(f.epr('session','show','--json')['claims'])==1
            assert not (f.root/'docs/.eprfs').exists()

    def test_heartbeat_and_cleanup_refuse_reused_owner_without_mutating_berth(self):
        with tempfile.TemporaryDirectory(prefix='envelope-heartbeat-') as base:
            f=Fixture(base); f.setup(); f.acquire()
            script=f.base/'heartbeat.py'
            write(script, """import os,runpy,sys
ns=runpy.run_path(os.environ['BERTH_SOURCE'])
b=ns['Berth'](os.environ['BERTH_DIR'])
session=os.environ['CLAUDE_SESSION_ID']
verb=sys.argv[1]
if verb=='ram-guard':
 hook=runpy.run_path(os.environ['RAM_GUARD_SOURCE'])
 original=hook['_berth']().Berth.touch
 def checked(*args,**kwargs):
  try: return original(*args,**kwargs)
  except Exception as e:
   assert 'envelope-owner-mismatch' in str(e),str(e)
   from pathlib import Path
   Path(os.environ['HEARTBEAT_REFUSAL']).write_text(str(e))
   raise
 hook['_berth']().Berth.touch=checked
 hook['berth_touch'](session)
elif verb=='release' : b.release('fixture-claude',session)
elif verb=='unmoor': b.unmoor(session)
else: b.touch(session,pid=4101)
""")
            env={**f.env(),'BERTH_SOURCE':str(REPO/'genesis/agentic/bin/berth'),
                 'RAM_GUARD_SOURCE':str(REPO/'.claude/hooks/ram-guard.py'),
                 'CLAUDE_SESSION_ID':f.id('claude'), 'HEARTBEAT_REFUSAL':str(f.base/'heartbeat-refused')}
            # A legitimate heartbeat still succeeds through the guard.
            f.run([sys.executable,script,'touch'],extra_env=env)
            paths=list((f.base/'berth/moorings').glob('*.json'))+[f.base/'berth/leases.json']
            before={p:p.read_bytes() for p in paths}
            stat=f.base/'proc/4101/stat'
            stat.write_text(stat.read_text().removesuffix('77')+'78')
            for verb in ('touch','release','unmoor','ram-guard'):
                r=subprocess.run([sys.executable,str(f.base/'exec.py'),sys.executable,str(script),verb],
                    cwd=f.root,env=env,capture_output=True,text=True,timeout=45)
                if verb=='ram-guard':
                    assert r.returncode==0,r.stderr  # hook swallows refusal, never the guard
                    assert 'envelope-owner-mismatch' in (f.base/'heartbeat-refused').read_text()
                else:
                    assert r.returncode!=0 and 'envelope-owner-mismatch' in r.stderr,r
                assert all(p.read_bytes()==data for p,data in before.items()),verb

    def test_interrupted_close_is_not_live_and_refuses_elevation(self):
        with tempfile.TemporaryDirectory(prefix='envelope-sealed-') as base:
            f=Fixture(base); f.setup()
            write(f.root/'.claude/epr-meta/policies.yaml',(REPO/'.claude/epr-meta/policies.yaml').read_text())
            f.epr('session','open','--json')
            f.epr('actor','claim','--as','human:matthew','--json')
            assert f.epr('session','show','--json')['live'] is True
            f.epr('session','close','--json')
            path=f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json'
            record=json.loads(path.read_text()); record['closedAt']=None
            path.write_text(json.dumps(record))
            view=f.epr('session','show','--json')
            assert view['live'] is False and view['envelope']['covenantSeal']
            assert view['envelope']['closedAt'] is None and view['capabilityTier']=='human'
            r=subprocess.run([sys.executable,str(f.base/'exec.py'),sys.executable,
                str(REPO/'.claude/hooks/capability-tier-gate.py')],
                input=json.dumps({'tool_name':'Bash','tool_input':{'command':'git reset --hard'}}),
                cwd=f.root,env=f.env(),capture_output=True,text=True,timeout=15)
            assert r.returncode==0 and '"permissionDecision": "deny"' in r.stdout,(r.stdout,r.stderr)
            assert 'unknown' in r.stdout,r.stdout

    def test_hookless_open_completes_recall_and_charges_its_attempt(self):
        with tempfile.TemporaryDirectory(prefix='envelope-recall-') as base:
            f=Fixture(base); f.setup()
            f.epr('session','open','--json')
            path=f.root/f'.eprfs/status/recall/{f.id("claude")}/continuation.json'
            before=json.loads(path.read_text())
            assert before['ceremony'] and before['attempts']==1 and before['unmetered_attempts']==0,before
            write(f.root/'genesis/docs/session.md', '---\nid: recall-fixture\ntitle: Session\n---\nSession proof.\n')
            result=f.epr('flow','memory','recall','search','--search-scope','genesis/docs',
                         '--name','*.md','--query','Session','--json')
            assert result['cumulative']['unmetered_attempts']==0,result
            after=json.loads(path.read_text())
            assert after['attempts']==2 and after['unmetered_attempts']==0,after

    def test_headless_stage_claims_mesh_before_entering_lane(self):
        with tempfile.TemporaryDirectory(prefix='envelope-stage-') as base:
            f=Fixture(base); f.setup()
            app=f.base/'app'; snapshot=f.base/'stage-lease.json'
            write(app/'scripts/hc-mesh.sh', f'cp "$BERTH_DIR/leases.json" {snapshot}\nexit 42\n')
            template=(REPO/'genesis/agentic/compute/stage/stage-runner.template.sh').read_text()
            exports='\n'.join(line for line in template.splitlines()
                              if line.startswith(('export ELOHIM_SESSION_', 'export BERTH_CLASS=')))
            shutil.copy2(REPO/'justfile', f.root/'justfile')
            script=f.base/'stage.sh'
            write(script, 'STAGE_NAME=probe\n'+exports+'\n'+
                  f'just --justfile {f.root}/justfile app_dir={app} test mesh fixture.feature\n')
            write(f.base/'no-runtimes.json', '{"runtimes":[]}')
            out=f.run(['bash',script], native=False, expected=42, extra_env={
                'EPR_BIN':str(BINARY), 'EPR_PROC_ROOT':'/proc',
                'EPR_AGENT_RUNTIMES':str(f.base/'no-runtimes.json')})
            lease=json.loads(snapshot.read_text())['mesh']
            assert lease['holder'].startswith('stage-probe-'), lease
            assert 'mesh: claimed by stage-probe-' in out, out
            session=lease['holder']
            record=json.loads((f.base/f'config-home/elohim/device/sessions/{session}.json').read_text())
            assert record['identity']['ownerPid']==int(session.rsplit('-',1)[1])
            assert record['identity']['source']=='declared'

    def test_first_open_requires_complete_provable_owner_before_writes(self):
        for missing in ('owner', 'boot', 'namespace'):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory(prefix='envelope-first-') as base:
                f=Fixture(base); f.setup()
                extra={}
                if missing=='owner':
                    write(f.base/'empty-registry.json', '{"runtimes":[]}')
                    extra={'ELOHIM_SESSION_ID':'ownerless', 'EPR_AGENT_RUNTIMES':str(f.base/'empty-registry.json')}
                else:
                    p=f.base/('proc/sys/kernel/random/boot_id' if missing=='boot' else 'proc/self/ns/pid')
                    p.unlink()
                    if missing=='boot': p.mkdir()  # unreadable as boot-id text
                r=subprocess.run([sys.executable,str(f.base/'exec.py'),str(BINARY),'session','open','--json'],
                    cwd=f.root,env={**f.env(),**extra},capture_output=True,text=True)
                assert r.returncode != 0 and 'envelope-owner-unknown' in r.stderr, r
                assert not (f.base/'config-home/elohim/device/sessions').exists()
                assert not (f.root/'.eprfs/status/recall'/('ownerless' if missing=='owner' else f.id('claude'))).exists()

    def test_boot_and_namespace_mismatch_or_unknown_refuses_every_binding_operation(self):
        with tempfile.TemporaryDirectory(prefix='envelope-context-') as base:
            f=Fixture(base); f.setup(); f.acquire()
            path=f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json'
            before=path.read_bytes()
            boot=f.base/'proc/sys/kernel/random/boot_id'
            namespace=f.base/'proc/self/ns/pid'
            for field in [boot,namespace]:
                original=field.with_suffix('.original'); field.rename(original)
                for state in ['different','unknown']:
                    if state=='different': field.write_text('different context')
                    else: field.unlink()
                    for args in [('session','open'),('session','show'),('session','close'),
                                 ('session','validate','--session',f.id('claude')),
                                 ('flow','claim','--on','envelope#1'),('covenant','record'),
                                 ('actor','claim','--as','agent:reader@haiku')]:
                        r=subprocess.run([sys.executable,str(f.base/'exec.py'),str(BINARY),*args],
                            cwd=f.root,env=f.env(),capture_output=True,text=True)
                        reason='envelope-owner-mismatch' if state=='different' else 'envelope-owner-unknown'
                        assert r.returncode!=0 and reason in r.stderr, (state,args,r)
                        assert path.read_bytes()==before
                original.rename(field)

    def test_gc_uses_bound_store_and_missing_store_refuses_without_sealing(self):
        for missing in [False,True]:
            with tempfile.TemporaryDirectory(prefix='envelope-store-') as base:
                f=Fixture(base); f.setup(); f.acquire()
                store_a=f.base/'berth'; store_b=f.base/'berth-b'; store_b.mkdir()
                sentinel=store_b/'leases.json'; sentinel.write_text('{"unrelated":true}')
                path=f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json'
                before=path.read_bytes()
                assert json.loads(before)['berthStore']==str(store_a.resolve())
                r=subprocess.run([sys.executable,str(f.base/'exec.py'),str(REPO/'genesis/agentic/bin/berth'),
                    'claim','wrong-store'],cwd=f.root,env={**f.env(),'BERTH_DIR':str(store_b)},capture_output=True,text=True)
                assert r.returncode!=0 and 'envelope-berth-mismatch' in r.stderr,r
                view=json.loads(f.run([BINARY,'session','show','--json'],extra_env={'BERTH_DIR':str(store_b)}))
                assert list(view['berth']['leases'])==['fixture-claude']
                shutil.rmtree(f.base/'proc/4101')
                if missing: store_a.rename(f.base/'unreachable-berth')
                r=subprocess.run([sys.executable,str(f.base/'exec.py'),str(BINARY),'session','gc','--json'],
                    cwd=f.root,env={**f.env(),'BERTH_DIR':str(store_b)},capture_output=True,text=True)
                if missing:
                    assert r.returncode!=0 and 'envelope-berth-unreachable' in r.stderr and f.id('claude') in r.stderr,r
                    assert path.read_bytes()==before
                    (f.base/'unreachable-berth').rename(store_a)
                    f.run([BINARY,'session','gc','--json'],extra_env={'BERTH_DIR':str(store_b)})
                else:
                    assert r.returncode==0,r
                assert 'fixture-claude' not in json.loads((store_a/'leases.json').read_text())
                assert 'fixture-codex' in json.loads((store_a/'leases.json').read_text())
                assert sentinel.read_text()=='{"unrelated":true}'

    def test_hookless_berth_initializes_before_shared_guard(self):
        with tempfile.TemporaryDirectory(prefix='envelope-first-berth-') as base:
            f=Fixture(base); f.setup()
            f.epr('session','open','--json',runtime='codex')
            path=f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json'
            assert not path.exists()
            # A steward resource exercises the peer query that used to lazily open
            # the envelope while the parent held its shared guard.
            shutil.copy2(REPO/'genesis/agentic/pool-policy.json',f.root/'genesis/agentic/pool-policy.json')
            f.run([REPO/'genesis/agentic/bin/berth','claim','dev-integration'])
            assert json.loads(path.read_text())['berthStore']==str(f.base/'berth')
            f.epr('session','close','--json')
            assert 'dev-integration' not in json.loads((f.base/'berth/leases.json').read_text())

    def test_class_refusal_without_session_precedes_guard_and_resolution(self):
        with tempfile.TemporaryDirectory(prefix='envelope-refusal-') as base:
            f=Fixture(base); f.setup(); f.epr('session','open','--json',runtime='codex')
            shutil.copy2(REPO/'genesis/agentic/pool-policy.json',f.root/'genesis/agentic/pool-policy.json')
            # Any native call is a failure: the exact review command needs neither
            # P1 resolution nor envelope preparation, even with sessions present.
            fail=f.base/'must-not-run'; write(fail,'#!/bin/sh\necho unexpected-native-call >&2\nexit 99\n'); fail.chmod(0o755)
            for args,code in [(['mesh','--class','measure'],3),(['mesh','--class','bogus'],2),
                              (['mesh','--ttl','7200'],3),(['mesh','--ttl','0'],2)]:
                r=subprocess.run([str(REPO/'genesis/agentic/bin/berth'),'claim',*args],cwd=f.root,
                    env={**f.env(),'EPR_BIN':str(fail),'EPR_PROC_ROOT':str(f.base/'absent-proc')},capture_output=True,text=True)
                assert r.returncode==code and 'unexpected-native-call' not in r.stderr,r
                if args[-1]=='measure': assert 'just measure <scope>' in r.stderr,r
            assert not (f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json').exists()

    def test_home_fallback_and_home_alias_write_no_envelope(self):
        with tempfile.TemporaryDirectory(prefix='envelope-home-') as base:
            f=Fixture(base); f.setup()
            home=f.base/'home'; home.mkdir()
            shutil.copytree(f.base/'config-home',home/'.config')
            alias=f.base/'alias'; alias.symlink_to(home/'.config',target_is_directory=True)
            for config in ['', 'relative', str(home/'.config'), str(alias)]:
                result=subprocess.run([sys.executable,str(f.base/'exec.py'),str(BINARY),'session','open','--json'],
                    cwd=f.root,env={**f.env(),'HOME':str(home),'XDG_CONFIG_HOME':config},capture_output=True,text=True)
                assert result.returncode != 0 and 'persistent-config-required' in result.stderr, result
                assert not (home/'.config/elohim/device/sessions').exists()
            # Explicit home keys receive the same remedy, before the generic key guard.
            for key in [home/'.config/elohim/device/ed25519.seed', home/'missing/ed25519.seed']:
                result=subprocess.run([sys.executable,str(f.base/'exec.py'),str(BINARY),'session','open','--json'],
                    cwd=f.root,env={**f.env(),'HOME':str(home),'ELOHIM_DEVICE_KEY_FILE':str(key)},capture_output=True,text=True)
                assert result.returncode != 0 and 'persistent-config-required' in result.stderr, result
                assert not (key.parent/'sessions').exists()
            assert not (f.root/'.eprfs/status/recall').exists()

    def test_berth_direct_claim_racing_close_is_fenced_and_post_close_refuses(self):
        with tempfile.TemporaryDirectory(prefix='envelope-race-') as base:
            f=Fixture(base); f.setup(); f.acquire()
            # Pause a direct library caller inside its guarded mutation, proving close waits.
            script=f.base/'claim.py'
            write(script, """import os, runpy, pathlib, time
ns=runpy.run_path(os.environ['BERTH_SOURCE'])
b=ns['Berth'](os.environ['BERTH_DIR'])
original=b._claim_live
def held(*args, **kwargs):
 # A fold emitted through the launcher lazily opens the existing envelope. It must
 # share this guard rather than deadlock trying to upgrade to an exclusive lock.
 import subprocess
 result=subprocess.run([os.environ['EPR_BIN'], 'flow', 'note', '--on', '.epr-meta/manifest.md', '--kind', 'observation', '--reason', 'guarded fold', '--json'], capture_output=True, text=True)
 assert result.returncode == 0, (result.stdout, result.stderr)
 p=pathlib.Path(os.environ['RENDEZVOUS']); p.write_text('locked')
 while not p.with_suffix('.release').exists(): time.sleep(.01)
 return original(*args, **kwargs)
b._claim_live=held
assert b.claim('racing', os.environ['SESSION'])[0]
""")
            rendezvous=f.base/'rendezvous'
            env={**f.env(),'BERTH_SOURCE':str(REPO/'genesis/agentic/bin/berth'),'RENDEZVOUS':str(rendezvous),'SESSION':f.id('claude')}
            claimant=subprocess.Popen([sys.executable,str(f.base/'exec.py'),sys.executable,str(script)],cwd=f.root,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
            import time
            try:
                deadline=time.monotonic()+10
                while not rendezvous.exists() and claimant.poll() is None and time.monotonic()<deadline: time.sleep(.01)
                assert rendezvous.exists(), claimant.communicate(timeout=2) if claimant.poll() is not None else 'guard timed out'
                closer=subprocess.Popen([sys.executable,str(f.base/'exec.py'),str(BINARY),'session','close','--json'],cwd=f.root,env=f.env(),stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
                try:
                    time.sleep(.15)
                    assert closer.poll() is None, 'close crossed a live shared guard'
                    write(rendezvous.with_suffix('.release'),'go')
                    out,err=claimant.communicate(timeout=15); assert claimant.returncode==0,(out,err)
                    out,err=closer.communicate(timeout=15); assert closer.returncode==0,(out,err)
                finally:
                    if closer.poll() is None: closer.kill(); closer.wait()
            finally:
                if claimant.poll() is None: claimant.kill(); claimant.wait()
            assert 'racing' not in json.loads((f.base/'berth/leases.json').read_text())
            f.witnesses(1)
            before=(f.root/'.eprfs/status/flows.jsonl').read_bytes()
            for verb in ['moor','claim','touch','release','unmoor']:
                args=[str(REPO/'genesis/agentic/bin/berth'),verb]
                if verb in ('claim','release'): args+=['after-close']
                result=subprocess.run([sys.executable,str(f.base/'exec.py'),*args],cwd=f.root,env=f.env(),capture_output=True,text=True)
                assert result.returncode != 0 and 'envelope-closed: '+f.id('claude') in result.stderr, result
                write(script,"import os,runpy\nns=runpy.run_path(os.environ['BERTH_SOURCE'])\nb=ns['Berth'](os.environ['BERTH_DIR'])\nb."+verb+"("+("'after-close', " if verb in ('claim','release') else '')+"os.environ['SESSION'])\n")
                result=subprocess.run([sys.executable,str(f.base/'exec.py'),sys.executable,str(script)],cwd=f.root,env=env,capture_output=True,text=True)
                assert result.returncode != 0 and 'envelope-closed: '+f.id('claude') in result.stderr,result
            for args in [('flow','claim','--on','envelope#1'),('covenant','record')]:
                result=subprocess.run([sys.executable,str(f.base/'exec.py'),str(BINARY),*args],cwd=f.root,env=f.env(),capture_output=True,text=True)
                assert result.returncode != 0 and 'envelope-closed: '+f.id('claude') in result.stderr,result
            f.epr('session','close','--json')
            assert (f.root/'.eprfs/status/flows.jsonl').read_bytes()==before
            assert not (f.base/f'berth/moorings/{f.id("claude")}.json').exists()

    def test_hookless_first_flow_call_opens_once_and_old_hex_requires_adoption(self):
        with tempfile.TemporaryDirectory(prefix='envelope-lazy-') as base:
            f=Fixture(base); f.setup()
            record=f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json'
            assert not record.exists()
            f.epr('flow','status','--json')
            assert json.loads(record.read_text())['identity']['session']==f.id('claude')
            state=f.root/f'.eprfs/status/recall/{f.id("claude")}/continuation.json'
            before=state.read_bytes()
            f.epr('session','open','--json')
            assert state.read_bytes()==before, 'idempotent open cannot charge recall again'
            continuation=json.loads(before); continuation['executor_digest']='a'*64
            state.write_text(json.dumps(continuation))
            out=f.run([f.base/'launch/epr','flow','memory','recall','open','--need','context','--json'],expected=2)
            assert 'adopt' in out and 'executor bytes changed' in out
            assert json.loads(state.read_text())['executor_digest']=='a'*64

    def test_close_retry_after_witness_append_survives_a_new_git_head(self):
        with tempfile.TemporaryDirectory(prefix='envelope-retry-') as base:
            f=Fixture(base); f.setup(); f.acquire(); f.close_claude()
            path=f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json'
            record=json.loads(path.read_text()); witness=record['witness']
            record['witness']=None; record['closedAt']=None; path.write_text(json.dumps(record))
            f.run(['git','commit','--allow-empty','-qm','new head'],native=False)
            closed=f.epr('session','close','--json')
            assert closed['witness']==witness
            f.witnesses(1)

    def test_rebuilt_cargo_target_does_not_replace_hook_executor(self):
        with tempfile.TemporaryDirectory(prefix='envelope-pin-') as base:
            f=Fixture(base); f.setup(); f.hooks()


if __name__ == '__main__':
    if '--stage' in sys.argv:
        p=argparse.ArgumentParser();p.add_argument('--stage');p.add_argument('--base');a=p.parse_args()
        getattr(Fixture(a.base),a.stage)()
    else:
        unittest.main()
