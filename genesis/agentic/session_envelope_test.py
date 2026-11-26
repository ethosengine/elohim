"""Isolated native envelope proof; the a2o steps drive these same state transitions."""
import argparse
from datetime import datetime, timezone
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]


def resolve_caller_epr(env=os.environ):
    """Use the executable selected by the caller; selecting a device head belongs to recipes."""
    try:
        selected = env.get('EPR_BIN')
        if not selected:
            raise ValueError('EPR_BIN was not supplied by the caller')
        binary = Path(selected).resolve()
        if not binary.is_file() or not os.access(binary, os.X_OK):
            raise ValueError(f'EPR_BIN is not an executable file: {selected}')
        return binary
    except (OSError, ValueError) as error:
        raise RuntimeError(f'UNMEASURED: caller-selected epr is unavailable ({error})') from error


# Chosen when this file's own tests run, never at import: shift_lease_test imports this module for its
# Fixture and patches BINARY itself, and an import must not raise for a caller that set no EPR_BIN.
BINARY = None


def setUpModule():
    global BINARY
    BINARY = resolve_caller_epr()
RUNTIMES = {'claude': ('claude-code', 4101), 'codex': ('codex', 4102)}


class CallerEprResolutionTest(unittest.TestCase):
    def test_explicit_epr_bin_wins(self):
        with tempfile.TemporaryDirectory(prefix='caller-epr-') as base:
            selected = Path(base) / 'epr'
            selected.write_text('#!/bin/sh\nexit 0\n')
            selected.chmod(0o755)
            self.assertEqual(resolve_caller_epr(env={'EPR_BIN': str(selected)}), selected)

    def test_missing_epr_bin_is_unmeasured(self):
        with self.assertRaisesRegex(RuntimeError, 'UNMEASURED.*EPR_BIN was not supplied'):
            resolve_caller_epr(env={})


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def link_or_copy(source, dest):
    """`dest` names `source`'s bytes: a hard link where one filesystem holds both, else a copy.

    The tested epr is the run's one shared (stripped) file; linking it into every fixture keeps
    setup from writing it again. A shared inode must never be written in place: nothing writes a
    fixture's candidate or launcher, and the rebuilt-target case replaces the candidate's path.
    """
    try:
        os.link(source, dest)
    except OSError:
        shutil.copy2(source, dest)


class Fixture:
    def __init__(self, base):
        self.base = Path(base)
        self.root = self.base / 'repo'

    def env(self, runtime='claude'):
        _, pid = RUNTIMES[runtime]
        return {**{k: v for k, v in os.environ.items() if not k.startswith(('EPR_', 'ELOHIM_', 'GIT_', 'CLAUDE_', 'CODEX_', 'BIN_STORE', 'BERTH_', 'BRIT_'))},
                'EPR_BIN': str(self.base / 'hook-bin/epr'), 'EPR_BIN_TOOLS': str(self.base / 'registry.json'),
                'EPR_UNDER_TEST': '1', 'EPR_AGENT_RUNTIMES': str(REPO / 'genesis/agentic/agent-runtimes.json'),
                'BIN_STORE': str(self.base / 'stores'), 'EPR_PROC_ROOT': str(self.base / 'proc'),
                'HOME': str(self.base / 'home'), 'XDG_DATA_HOME': str(self.base / 'data-home'),
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

    def setup(self, human="matthew"):
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
        self.run([BINARY, 'actor','witness','--subject',f'human:{human}','--as','agent:reader@fixture','--session','enrollment','--basis','Fixture person present','--json'])
        candidate = self.base / 'cargo-target/debug/epr'
        candidate.parent.mkdir(parents=True)
        link_or_copy(BINARY, candidate)
        cid = json.loads(self.run([BINARY,'bin','ingest',candidate,'--tool','epr','--source-commit',self.run(['git','rev-parse','HEAD'],native=False).strip(),'--tree-clean','true','--gate-run','fixture','--repo',self.root,'--json']))['cid']
        write(self.base / 'cid', cid)
        (self.base / 'launch').mkdir()
        link_or_copy(BINARY.with_name('bin-launch'), self.base / 'launch/epr')
        write(self.base / 'hook-bin/epr', f'#!/bin/sh\nexec {sys.executable} {self.base}/exec.py {self.base}/launch/epr "$@"\n')
        (self.base / 'hook-bin/epr').chmod(0o755)
        for runtime in RUNTIMES:
            self.run([BINARY,'bin','pin','--tool','epr','--cid',cid,'--json'], runtime)
        # Unpinned context projection is set up before the launcher lazily opens an envelope.
        self.run([BINARY,'flow','project','--root',self.root,'--json'])

    def runtime_child(self, session):
        # Claude assigns its id after exec: the runtime's exec environment is empty,
        # and its child exports the id. This is same-principal resolution, not authentication.
        fields = ['0']*20; fields[0] = 'S'; fields[19] = '88'
        write(self.base/'proc/4201/stat', '4201 (claude) '+' '.join(fields))
        write(self.base/'proc/4201/environ', '')
        return {'FIXTURE_OWNER': '4201', 'CLAUDE_SESSION_ID': session}

    def resume_schema(self):
        session = self.id('claude')
        shutil.rmtree(self.base/'proc/4101')
        env = self.runtime_child(session)
        opened = json.loads(self.run([BINARY, 'session', 'open', '--json'], extra_env=env))
        assert opened['generation'] == 2 and opened['predecessor'] is not None
        write(self.base/'open-successor.json', json.dumps(opened))
        view = self.run([BINARY, 'session', 'show', '--json'], extra_env=env)
        write(self.base/'view-successor.json', view)

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
        # A fresh file of its own (the candidate may be a link to the shared, read-only epr),
        # mode 0755 as cargo writes it.
        rebuilt = self.base/'cargo-target/debug/rebuilt'
        shutil.copyfile(BINARY,rebuilt); rebuilt.chmod(0o755)
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


class ResumeEnvelopeTest(unittest.TestCase):
    def test_memory_history_keeps_the_writing_generation(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory(prefix='generation-history-') as base:
            f = Fixture(base); f.setup(human='alice')
            for rel in ('.epr-meta/collective.json', '.eprfs/status/affiliations.jsonl'):
                write(f.root/rel, (REPO/rel).read_text())
            spec = importlib.util.spec_from_file_location('memory_projection', REPO/'.claude/hooks/memory-index-projection.py')
            hook = importlib.util.module_from_spec(spec); spec.loader.exec_module(hook)
            session = f.epr('session', 'open', '--json')['identity']['session']
            env = {}
            def run(*args, expected=0):
                out = f.run([BINARY, *args], extra_env=env, expected=expected)
                return json.loads(out) if expected == 0 and out.strip() else out
            def entry(name):
                path = f.root/f'.claude/memory/{name}.md'
                write(path, f'---\nname: {name}\ndescription: Generation history\ntype: feedback\n---\n{name}\n')
                # The hook's production witness boundary must freeze the current envelope key.
                # Deliberately date both witnesses after both claims: history must choose
                # by the stored generation label even when time alone cannot disambiguate.
                with patch.dict(os.environ, {**f.env(), **env}, clear=True), \
                     patch.object(hook, '_now', return_value='2099-01-01T00:00:00.000Z'):
                    hook.witness(f.root, path, hook.generation_key(session))
                return str(path.relative_to(f.root))
            def resume(pid):
                nonlocal env
                run('session', 'close', '--json')
                shutil.rmtree(f.base/f'proc/{pid}')
                env = f.runtime_child(session)
                if pid == 4201:
                    stat = f.base/'proc/4201/stat'
                    stat.write_text(stat.read_text().rsplit(' ', 1)[0] + ' 89')
                return run('session', 'open', '--json')
            bob = run('actor', 'claim', '--as', 'human:bob', '--session', session, '--json')
            run('flow', 'claim', '--on', 'envelope#1', '--session', session, '--json')
            a = entry('generation-a')
            assert resume(4101)['generation'] == 2
            assert run('session', 'show', '--json')['claims'] == []
            alice = run('actor', 'claim', '--as', 'human:alice', '--session', session, '--json')
            run('flow', 'claim', '--on', 'envelope#2', '--session', session, '--json')
            claims = run('session', 'show', '--json')['claims']
            assert len(claims) == 1 and f'session:{session}/generations/2' in json.dumps(claims)
            b = entry('generation-b')
            # Actor attestations also retain the generation, independently of claim attribution.
            attestation = run('actor', 'witness', '--subject', 'human:alice', '--as',
                              'agent:reader@fixture', '--session', session, '--basis',
                              'Generation two witness', '--again', '--json')
            with self.subTest(writer='actor witness'):
                assert attestation['session'] == f'{session}/generations/2', attestation
            assert resume(4201)['generation'] == 3
            assert run('session', 'show', '--json')['claims'] == []
            closed_rows = (f.root/'.eprfs/status/flows.jsonl').read_text()
            assert f'session {session}/generations/2 closed;' in closed_rows
            c = entry('generation-unclaimed')
            rows = run('flow', 'memory', 'attribution', '.claude/memory', '--json')['entries']
            by = {r['path']: r for r in rows}
            for path, claim, key in ((a, bob, session), (b, alice, f'{session}/generations/2')):
                with self.subTest(writer=path):
                    row = by[path]
                    assert row['session'] == key and row['claim'] == claim['claimed'], row
                    imported = run('flow', 'memory', 'import', path, '--session', key,
                                   '--as-of', row['writtenAt'], '--json')
                    assert imported['counts']['contributed'] == 1, imported
                    flows = [json.loads(line) for line in (f.root/'.eprfs/status/flows.jsonl').read_text().splitlines()]
                    assert f"actor-claim:{claim['recordCid']}" in json.dumps(flows[-1]), flows[-1]
                    # Even a historical lookup AFTER both claims must match the exact key.
                    request = f".eprfs/status/memory/contributions/{Path(path).stem}.json"
                    later = run('flow', 'memory', 'contribute', '--input', request,
                                '--session', key, '--as-of', datetime.now(timezone.utc).isoformat(), '--json')
                    assert later['actorClaim'] == claim['recordCid'], later
            assert by[c]['session'] == f'{session}/generations/3' and not by[c]['attributable'], by[c]
            f.run([BINARY, 'flow', 'memory', 'import', c, '--json'], extra_env=env, expected=2)
            assert not (f.root/'.eprfs/status/memory/contributions/generation-unclaimed.json').exists()

    def test_runtime_child_resumes_without_inheriting_authority(self):
        for death in ('absent-pid', 'new-boot'):
            with self.subTest(death=death), tempfile.TemporaryDirectory(prefix='envelope-resume-') as base:
                f = Fixture(base); f.setup(human='alice')
                for rel in ('.epr-meta/collective.json', '.eprfs/status/affiliations.jsonl'):
                    write(f.root/rel, (REPO/rel).read_text())
                # A stateful brit adapter exercises real native covenant admission and reads.
                write(f.base/'brit-stub', '''#!/usr/bin/env python3
import datetime,hashlib,json,pathlib,sys
args=sys.argv[1:]; verb=args[3]; args=args[4:]
opts=dict(zip(args[::2],args[1::2]))
path=pathlib.Path(__file__).parent/'clauses'/opts['--session']/'rows.json'
rows=json.loads(path.read_text()) if path.exists() else []
if verb=='put':
 clause={'kind':opts['--kind'],'session':opts['--session'],'party':opts['--party'],
         'quote':opts['--quote'],'payload':json.loads(opts['--payload-json']),
         'validFrom':datetime.datetime.now(datetime.timezone.utc).isoformat(),'validUntil':None}
 row={'cid':'fixture-'+hashlib.sha256(json.dumps(clause).encode()).hexdigest(), 'clause':clause}
 rows.append(row); path.parent.mkdir(parents=True,exist_ok=True); path.write_text(json.dumps(rows))
 print(json.dumps(row))
else: print(json.dumps(rows))
''')
                opened = f.epr('session', 'open', '--json')
                session = opened['identity']['session']
                assert f.epr('actor', 'current', '--device', '--json')['standing']['subject'] == 'human:alice'
                f.epr('actor', 'claim', '--as', 'human:matthew', '--session', session, '--json')
                f.epr('flow', 'memory', 'affiliate', '--session', session,
                      '--member', 'human:bob', '--kind', 'person', '--role', 'steward', '--json')
                first_claim = f.epr('actor', 'claim', '--as', 'human:bob', '--session', session, '--json')
                assert first_claim['session'] == session
                actors_path = f.root/'.eprfs/status/actors.jsonl'
                actor_bytes = actors_path.read_bytes()
                history_at = datetime.now(timezone.utc).isoformat()
                f.epr('flow', 'claim', '--on', 'envelope#1', '--session', session, '--json')
                write(f.root/'genesis/evidence.json', '{"observed":"isolated succession"}')
                collective = f.epr('flow', 'memory', 'pin', '--input', '.epr-meta/collective.json', '--json')['resource']
                evidence = f.epr('flow', 'memory', 'pin', '--input', 'genesis/evidence.json', '--json')['resource']
                write(f.root/'genesis/claim.json', json.dumps({
                    'version': 1, 'collective': collective, 'author': 'human:bob',
                    'steward': 'repo:fixture/envelope', 'scope': 'workspace', 'reach': 'workspace',
                    'concern': 'session authority', 'claim': 'This isolated run has evidence',
                    'uncertainty': ['No live proof'], 'sources': [{'resource': evidence, 'reach': 'workspace'}],
                    'supersedes': [], 'contradicts': []}))
                f.epr('flow', 'memory', 'contribute', '--input', 'genesis/claim.json', '--session', session, '--json')
                write(f.root/'area/.epr-meta', '''---
epr-meta-version: 1
id: successor-variance
rules:
  - id: fresh-read
    class: inject
    parameters:
      variance:
        floor: one fresh read
        requires: steward-of-record
---
''')
                # Manifest discovery uses tracked paths, just like a real checked-in rule.
                f.run(['git', 'add', 'area/.epr-meta'], native=False)
                (f.base/'berth').mkdir(exist_ok=True)
                f.epr('session', 'prepare-berth', '--session', session, '--store', str(f.base/'berth'))
                f.epr('covenant', 'record', '--kind', 'affirmsStewardOfRecord',
                      '--party', 'human:bob', '--quote', 'I stand for this work', '--json')
                assert f.epr('covenant', 'show', '--json')['steward'] == 'human:bob'
                f.epr('covenant', 'record', '--kind', 'variesRule', '--party', 'human:bob',
                      '--quote', 'One fresh read for this run', '--rule', 'fresh-read',
                      '--paths', 'area/**', '--json')
                assert f.epr('covenant', 'check', '--rule', 'fresh-read',
                             '--path', 'area/story.feature', '--json')['status'] == 'held'
                clause_path = f.base/f'clauses/{session}/rows.json'
                clause_bytes = clause_path.read_bytes()
                assert len(json.loads(clause_bytes)) == 2
                if death == 'absent-pid':
                    shutil.rmtree(f.base/'proc/4101')
                else:
                    write(f.base/'proc/sys/kernel/random/boot_id', 'next boot')
                collected = json.loads(f.run([BINARY, 'session', 'gc', '--json']))
                assert len(collected['closed']) == 1
                path = f.base/f'config-home/elohim/device/sessions/{session}.json'
                before = path.read_bytes()
                predecessor = json.loads(before)
                seal = path.parent/f"seals/{predecessor['covenantSeal']}.json"
                seal_bytes = seal.read_bytes()
                resumed_env = f.runtime_child(session)
                def run(*args):
                    out = f.run([BINARY, *args], extra_env=resumed_env)
                    return json.loads(out) if out.strip() else None
                # Retained dead-owner facets are governed by berth expiry/takeover, not resume.
                write(f.base/'berth/leases.json', '{"old-lease":{"holder":"'+session+'","pid":4101}}')
                write(f.base/f'berth/moorings/{session}.json', '{"pid":4101}')
                berth_before = {p: p.read_bytes() for p in (f.base/'berth').rglob('*') if p.is_file()}
                resumed = run('session', 'open', '--json')
                successor = json.loads(path.read_bytes())
                assert successor['berthStore'] is None
                assert predecessor['berthStore'] == str((f.base/'berth').resolve())
                assert clause_path.read_bytes() == clause_bytes
                assert not (f.base/f'clauses/{session}/generations/2/rows.json').exists()
                assert {p: p.read_bytes() for p in (f.base/'berth').rglob('*') if p.is_file()} == berth_before
                assert resumed['generation'] == 2 and resumed['identity']['ownerPid'] == 4201
                assert resumed['predecessor']['covenantSeal'] == predecessor['covenantSeal']
                archive = path.parent/f'{session}.gen/1.json'
                assert archive.read_bytes() == before and seal.read_bytes() == seal_bytes
                run('session', 'prepare-berth', '--session', session, '--store', str(f.base/'berth'))
                run('session', 'validate', '--session', session)
                assert all(p.read_bytes() == data for p, data in berth_before.items())
                with self.subTest(reader='actor current'):
                    assert run('actor', 'current', '--session', session, '--json')['claim'] is None
                assert run('actor', 'current', '--device', '--json')['standing']['subject'] == 'human:alice'
                # Each authority reader must refuse independently, without writing a facet.
                for args, reason in [
                    (['covenant', 'record', '--kind', 'affirmsStewardOfRecord', '--party', 'human:bob',
                      '--quote', 'Not yet claimed in this generation', '--json'], 'has no standing'),
                    (['flow', 'note', '--on', 'envelope#1', '--kind', 'verdict', '--verdict', 'approved',
                      '--purpose', 'acceptance', '--session', session, '--reason', 'Fresh acceptance', '--json'],
                     'acceptance requires a registered --session actor claim'),
                    (['flow', 'memory', 'contribute', '--input', 'genesis/claim.json',
                      '--session', session, '--json'], 'author differs from registered session claim'),
                    (['flow', 'memory', 'affiliate', '--session', session, '--member', 'human:carol',
                      '--kind', 'person', '--role', 'observer', '--json'], 'has no actor claim'),
                ]:
                    result = subprocess.run([sys.executable, str(f.base/'exec.py'), str(BINARY), *args],
                                            cwd=f.root, env={**f.env(), **resumed_env},
                                            text=True, capture_output=True, timeout=45)
                    with self.subTest(reader=args[:3]):
                        assert result.returncode != 0 and reason in result.stderr, result
                note = run('flow', 'note', '--on', 'docs/spec.md', '--kind', 'verdict',
                           '--verdict', 'approved', '--reason', 'Current review', '--session', session, '--json')
                assert note['actor'] == 'human:alice' and note['actor_claim'] != first_claim['recordCid'], note
                history = run('flow', 'memory', 'contribute', '--input', 'genesis/claim.json',
                              '--session', session, '--as-of', history_at, '--json')
                assert history['actorClaim'] == first_claim['recordCid'], history
                flow_rows = [json.loads(line) for line in (f.root/'.eprfs/status/flows.jsonl').read_text().splitlines()]
                historical_row = next(row for row in flow_rows if row['cid'] == history['event']['record_cid'])
                assert 'source:claim-signed' not in json.dumps(historical_row), historical_row
                assert actors_path.read_bytes() == actor_bytes
                assert run('covenant', 'show', '--json')['clauses'] == []
                offer = run('covenant', 'offer', '--json')
                assert any(row['kind'] == 'affirmsStewardOfRecord' for row in offer['pending'])
                second_claim = run('actor', 'claim', '--as', 'human:bob', '--session', session, '--json')
                assert second_claim['session'] == f'{session}/generations/2'
                assert second_claim['recordCid'] != first_claim['recordCid']
                assert actors_path.read_bytes().startswith(actor_bytes)
                assert run('actor', 'current', '--session', session, '--json')['claim']['session'] == f'{session}/generations/2'
                sponsored = run('flow', 'memory', 'affiliate', '--session', session,
                                '--member', 'human:carol', '--kind', 'person', '--role', 'observer', '--json')
                assert sponsored['sponsor'] == 'human:bob'
                contribution = run('flow', 'memory', 'contribute', '--input', 'genesis/claim.json',
                                   '--session', session, '--json')
                assert contribution['actorClaim'] == second_claim['recordCid'], contribution
                # Restore Bob's standing before checking: a bare-key check would now honour
                # the predecessor variance, so refusal cannot pass just because standing changed.
                check = run('covenant', 'check', '--rule', 'fresh-read', '--path', 'area/story.feature', '--json')
                assert check['status'] == 'not-held' and 'no variance clause' in check['reason'], check
                run('covenant', 'record', '--kind', 'affirmsStewardOfRecord',
                    '--party', 'human:bob', '--quote', 'I reaffirm for this generation', '--json')
                assert len(run('covenant', 'show', '--json')['clauses']) == 1
                assert clause_path.read_bytes() == clause_bytes
                assert len(json.loads((f.base/f'clauses/{session}/generations/2/rows.json').read_bytes())) == 1
                assert seal.read_bytes() == seal_bytes
                assert run('session', 'gc', '--json')['closed'] == []
                assert archive.read_bytes() == before
                # Arrival overwrites P1's existing pin at the same payload-id path.
                pin = f.base/f'stores/epr/sessions/{session}'
                assert json.loads(pin.read_text())['ownerPid'] == 4101
                run('bin', 'pin', '--tool', 'epr', '--cid', (f.base/'cid').read_text(), '--json')
                assert json.loads(pin.read_text())['ownerPid'] == 4201
                spec = importlib.util.spec_from_file_location('head_notice', REPO/'genesis/agentic/bin-head-notice.py')
                notice = importlib.util.module_from_spec(spec); spec.loader.exec_module(notice)
                assert pin == f.base/'stores/epr/sessions'/notice.pin_name(session)
                write(f.base/'stores/epr/current.json', json.dumps({'cid': 'new-head', 'landedAt': 'later'}))
                lines = notice.notices(str(f.base/'stores'), session)
                assert len(lines) == 1 and f"pinned {(f.base/'cid').read_text()}" in lines[0], lines
                successor_view = run('covenant', 'show', '--json')
                run('session', 'close', '--json')
                closed = json.loads(path.read_bytes())
                successor_seal = json.loads((path.parent/f"seals/{closed['covenantSeal']}.json").read_bytes())
                assert len(successor_seal['clauses']) == 1, successor_seal
                assert successor_seal['clauses'] == successor_view['clauses']
                assert successor_seal['clauses'][0]['quote'] == 'I reaffirm for this generation'
                assert archive.read_bytes() == before and seal.read_bytes() == seal_bytes
                assert clause_path.read_bytes() == clause_bytes

    def test_leaked_proc_fixture_cannot_admit_successor_over_live_owner(self):
        with tempfile.TemporaryDirectory(prefix='live-owner-') as base:
            f = Fixture(base); f.setup()
            session = 'real-live-owner'
            env = {'EPR_UNDER_TEST': '0', 'EPR_PROC_ROOT': '/proc',
                   'ELOHIM_SESSION_ID': session, 'ELOHIM_SESSION_OWNER_PID': str(os.getpid())}
            opened = json.loads(f.run([BINARY, 'session', 'open', '--json'], extra_env=env, native=False))
            assert opened['identity']['ownerPid'] == os.getpid()
            f.run([BINARY, 'session', 'close', '--json'], extra_env=env, native=False)
            sessions = f.base/'config-home/elohim/device/sessions'
            before = {p: p.read_bytes() for p in sessions.rglob('*') if p.is_file()}
            # A valid fixture successor sees a different boot and no live parent's PID.
            # Neither an absent switch nor 0 may use that evidence to admit takeover.
            child = {**f.env(), **f.runtime_child(session)}
            args = [sys.executable, str(f.base/'exec.py'), str(BINARY), 'session', 'open', '--json']
            for switch in (None, '0'):
                child.pop('EPR_UNDER_TEST', None)
                if switch is not None:
                    child['EPR_UNDER_TEST'] = switch
                result = subprocess.run(args, cwd=f.root, env=child,
                                        text=True, capture_output=True, timeout=45)
                assert result.returncode != 0 and 'envelope-owner-mismatch' in result.stderr, result
                assert {p: p.read_bytes() for p in sessions.rglob('*') if p.is_file()} == before
            # Positive control: only explicit fixture mode permits this simulated death.
            resumed = json.loads(f.run([BINARY, 'session', 'open', '--json'],
                                      extra_env=f.runtime_child(session)))
            assert resumed['generation'] == 2 and resumed['identity']['ownerPid'] == 4201

    def test_gc_honors_isolated_proc_root_without_test_switch(self):
        with tempfile.TemporaryDirectory(prefix='gc-isolation-') as base:
            f = Fixture(base); f.setup()
            env = {'EPR_UNDER_TEST': '0', 'EPR_PROC_ROOT': '/proc',
                   'ELOHIM_SESSION_ID': 'real-live-owner', 'ELOHIM_SESSION_OWNER_PID': str(os.getpid())}
            opened = json.loads(f.run([BINARY, 'session', 'open', '--json'], extra_env=env, native=False))
            assert opened['identity']['ownerPid'] == os.getpid()
            # GC retains the supplied view, even when it hides a real live owner.
            env['EPR_PROC_ROOT'] = str(f.base/'proc')
            result = json.loads(f.run([BINARY, 'session', 'gc', '--json'], extra_env=env, native=False))
            assert len(result['closed']) == 1

    def test_prepare_berth_invalid_destination_does_not_publish_successor(self):
        for destination in ('missing', 'regular-file'):
            with self.subTest(destination=destination), tempfile.TemporaryDirectory(prefix='berth-refuse-') as base:
                f = Fixture(base); f.setup()
                opened = f.epr('session', 'open', '--json')
                session = opened['identity']['session']
                f.epr('session', 'close', '--json')
                shutil.rmtree(f.base/'proc/4101')
                env = f.runtime_child(session)
                path = f.base/destination
                if destination == 'regular-file':
                    path.write_text('not a store')
                sessions = f.base/'config-home/elohim/device/sessions'
                before = {p: p.read_bytes() for p in sessions.rglob('*') if p.is_file()}
                args = [BINARY, 'session', 'prepare-berth', '--session', session, '--store', str(path)]
                result = subprocess.run([sys.executable, str(f.base/'exec.py'), *map(str, args)],
                    cwd=f.root, env={**f.env(), **env}, capture_output=True, text=True, timeout=45)
                assert result.returncode != 0 and 'envelope-berth-unreachable' in result.stderr, result
                assert {p: p.read_bytes() for p in sessions.rglob('*') if p.is_file()} == before
                # The same closed, dead predecessor is admissible once the destination is valid.
                if path.exists():
                    path.unlink()
                path.mkdir()
                f.run(args, extra_env=env)
                current = json.loads((sessions/f'{session}.json').read_bytes())
                assert current['generation'] == 2 and current['berthStore'] == str(path.resolve())

    def test_resume_refusals_write_nothing(self):
        for refusal in ('declared', 'live', 'unclosed', 'namespace', 'stat'):
            with self.subTest(refusal=refusal), tempfile.TemporaryDirectory(prefix='envelope-refuse-') as base:
                f = Fixture(base); f.setup()
                opened = f.epr('session', 'open', '--json')
                session = opened['identity']['session']
                if refusal not in ('live', 'unclosed'):
                    f.epr('session', 'close', '--json')
                # The namespace case keeps its predecessor's /proc entry: with it gone the
                # predecessor's holder exits within its one-second poll and releases its lock,
                # and a released holder is, by design, proof of death across a foreign pid
                # namespace (device-runtime envelope.rs, `holder_released`). The refusal asserted
                # below then held only for as long as the holder took to notice, and a loaded
                # device ran `session open` after it (2026-11-21, one run in eleven).
                if refusal in ('declared', 'unclosed'):
                    shutil.rmtree(f.base/'proc/4101')
                if refusal == 'namespace':
                    namespace = f.base/'proc/self/ns/pid'
                    namespace.rename(namespace.with_suffix('.old'))
                    namespace.write_text('different namespace')
                if refusal == 'stat':
                    stat = f.base/'proc/4101/stat'; stat.unlink(); stat.mkdir()
                fields = ['0']*20; fields[0] = 'S'; fields[19] = '88'
                write(f.base/'proc/4201/stat', '4201 (claude) '+' '.join(fields))
                write(f.base/'proc/4201/environ', '')
                env = {**f.env(), 'FIXTURE_OWNER': '4201', 'CLAUDE_SESSION_ID': session}
                if refusal == 'declared':
                    env.update(ELOHIM_SESSION_ID=session, ELOHIM_SESSION_OWNER_PID='4201')
                sessions = f.base/'config-home/elohim/device/sessions'
                before = {p: p.read_bytes() for p in sessions.rglob('*') if p.is_file()}
                result = subprocess.run([sys.executable, str(f.base/'exec.py'), str(BINARY), 'session', 'open'],
                    cwd=f.root, env=env, capture_output=True, text=True, timeout=45)
                assert result.returncode != 0 and 'envelope-owner-mismatch' in result.stderr, result
                assert {p: p.read_bytes() for p in sessions.rglob('*') if p.is_file()} == before
                # Positive control: the refusal is selective, not a blanket ban on resuming.
                if refusal == 'namespace':
                    namespace.with_suffix('.old').replace(namespace)
                shutil.rmtree(f.base/'proc/4101', ignore_errors=True)
                env.pop('ELOHIM_SESSION_ID', None); env.pop('ELOHIM_SESSION_OWNER_PID', None)
                f.run([BINARY, 'session', 'gc', '--json'], extra_env=env)
                successor = json.loads(f.run([BINARY, 'session', 'open', '--json'], extra_env=env))
                assert successor['generation'] == 2


class EnvelopeTest(unittest.TestCase):
    def test_commit_decoration_uses_mounted_model_and_session_without_claiming_authority(self):
        for runtime, suffix, address, key in [('codex', ' (Codex)', 'openai', 'Codex-Session'),
                                              ('claude', '', 'anthropic', 'Claude-Session')]:
            with self.subTest(runtime=runtime), tempfile.TemporaryDirectory(prefix='commit-decoration-') as base:
                f = Fixture(base); f.setup()
                opened = f.epr('session', 'open', '--model', 'fixture-model', '--json', runtime=runtime)
                if runtime == 'codex':
                    # Private, exact-thread metadata; never read the host's Codex store.
                    model_home = f.base / 'codex-model'
                    model_home.mkdir()
                    with sqlite3.connect(model_home / 'state_5.sqlite') as db:
                        db.execute('CREATE TABLE threads(id TEXT PRIMARY KEY, model TEXT)')
                        db.execute('INSERT INTO threads VALUES (?, ?)', (opened['identity']['session'], 'fixture-model'))
                    original_env = f.env
                    f.env = lambda runtime='claude': {**original_env(runtime), 'CODEX_HOME': str(model_home)}
                result = f.epr('session', 'commit-trailers', '--json', runtime=runtime)
                assert result == {'status': 'measured', 'trailers': [
                    ['Co-Authored-By', f'fixture-model{suffix} <noreply@{address}.com>'],
                    [key, opened['identity']['session']]]}, result
                assert f.epr('actor', 'current', '--session', opened['identity']['session'],
                             '--json', runtime=runtime)['claim'] is None
                # Exercise the actual Git entrypoint and native reader, not a Python runtime map.
                hooks = f.root / '.hooks'
                hooks.mkdir()
                shutil.copy2(REPO / '.husky/prepare-commit-msg', hooks / 'prepare-commit-msg')
                (hooks / 'prepare-commit-msg').chmod(0o755)
                (f.root / 'genesis/agentic/commit-attribution.py').symlink_to(REPO / 'genesis/agentic/commit-attribution.py')
                f.run(['git', 'config', 'core.hooksPath', str(hooks)], native=False)
                git = shutil.which('git')
                assert git is not None
                body = 'empty handoff [skip ci]\n\nCo-Authored-By: Existing <existing@example.test>\n'
                f.run([git, 'commit', '--allow-empty', '-qm', body], runtime=runtime)
                trailers = f.run(['git', 'show', '-s', '--format=%(trailers)'], native=False)
                assert 'Existing <existing@example.test>' in trailers, trailers
                assert trailers.count(f'{key}: {opened["identity"]["session"]}') == 1, trailers
                assert f'Co-Authored-By: fixture-model{suffix} <noreply@{address}.com>' in trailers, trailers
                # Reusing the message must preserve contributors and not duplicate our decoration.
                f.run([git, 'commit', '--allow-empty', '-q', '-C', 'HEAD'], runtime=runtime)
                assert f.run(['git', 'show', '-s', '--format=%(trailers)'], native=False) == trailers
                if runtime == 'codex':
                    with sqlite3.connect(model_home / 'state_5.sqlite') as db:
                        db.execute('UPDATE threads SET model=? WHERE id=?', ('fixture-next-model', opened['identity']['session']))
                    f.run([git, 'commit', '--allow-empty', '-qm', 'model changed'], runtime=runtime)
                    changed = f.run(['git', 'show', '-s', '--format=%(trailers)'], native=False)
                    assert 'fixture-next-model (Codex)' in changed and 'fixture-model (Codex)' not in changed, changed

    def test_a_live_owner_declares_its_model_once_and_commits_become_measured(self):
        # SessionStart opens the envelope before the runtime can name its model. The same live
        # owner then declares it; the CLI must record it, not answer with the old record.
        with tempfile.TemporaryDirectory(prefix='commit-late-model-') as base:
            f = Fixture(base); f.setup()
            first = f.epr('session', 'open', '--json')
            assert first['model'] is None, first
            late = f.epr('session', 'open', '--model', 'fixture-late-model', '--json')
            assert late['model'] == 'fixture-late-model', late
            assert late['generation'] == first['generation'], (first, late)
            assert f.epr('session', 'show', '--json')['envelope']['model'] == 'fixture-late-model'
            result = f.epr('session', 'commit-trailers', '--json')
            assert result['status'] == 'measured', result
            assert ['Co-Authored-By', 'fixture-late-model <noreply@anthropic.com>'] in result['trailers'], result
            # Declaring the same model again is a read; a different one is refused, never swapped.
            assert f.epr('session', 'open', '--model', 'fixture-late-model', '--json')['model'] == 'fixture-late-model'
            argv = [sys.executable, str(f.base / 'exec.py'), str(f.base / 'launch/epr'),
                    'session', 'open', '--model', 'another-model', '--json']
            refused = subprocess.run(argv, cwd=f.root, env=f.env('claude'), text=True, capture_output=True, timeout=45)
            assert refused.returncode != 0, refused
            assert 'envelope-model-conflict' in refused.stdout + refused.stderr, refused
            assert f.epr('session', 'show', '--json')['envelope']['model'] == 'fixture-late-model'

    def test_commit_decoration_missing_model_is_unmeasured_not_invented(self):
        with tempfile.TemporaryDirectory(prefix='commit-unmeasured-') as base:
            f = Fixture(base); f.setup()
            original_env = f.env
            f.env = lambda runtime='claude': {**original_env(runtime), 'CODEX_HOME': str(f.base / 'missing-codex-model')}
            f.epr('session', 'open', '--model', 'stale-mounted-model', '--json', runtime='codex')
            message = f.root / 'message'
            message.write_text('unchanged message\n')
            result = subprocess.run([sys.executable, str(REPO / 'genesis/agentic/commit-attribution.py'), str(message)],
                                    cwd=f.root, env=f.env('codex'), capture_output=True, text=True, timeout=15)
            assert result.returncode == 0, result
            assert 'UNMEASURED' in result.stderr and 'model' in result.stderr, result
            assert message.read_text() == 'unchanged message\n'
            registry = f.base / 'no-agent-runtimes.json'
            registry.write_text(json.dumps({'runtimes': [{'id': 'unused-runtime', 'comm': ['unused-runtime'], 'sessionEnv': []}]}))
            result = json.loads(f.run([BINARY, 'session', 'commit-trailers', '--json'],
                                     extra_env={'EPR_AGENT_RUNTIMES': str(registry)}))
            assert result == {'status': 'not-applicable', 'trailers': []}, result

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
            # The owner's /proc entry stays until this assertion has run: removing it first lets
            # the holder exit and release its lock, which across a foreign namespace is proof of
            # death by design, so gc would then be right to close the envelope and this line
            # would pass only while the holder had not yet noticed.
            assert json.loads(f.run([BINARY,'session','gc','--json']))['closed']==[]
            shutil.rmtree(stat.parent)
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
            for args in [('session','open'), ('session','show'), ('session','commit-trailers'), ('session','close'),
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
            # The lane refuses before its berth claim without the a2o workspace prerequisites.
            a2o=f.base/'a2o'
            write(a2o/'node_modules/.bin/cucumber-js', '#!/bin/sh\nexit 0\n')
            (a2o/'node_modules/.bin/cucumber-js').chmod(0o755)
            write(a2o/'node_modules/@elohim/storage-client/dist/index.js', '')
            template=(REPO/'genesis/agentic/compute/stage/stage-runner.template.sh').read_text()
            exports='\n'.join(line for line in template.splitlines()
                              if line.startswith(('export ELOHIM_SESSION_', 'export BERTH_CLASS=')))
            shutil.copy2(REPO/'justfile', f.root/'justfile')
            script=f.base/'stage.sh'
            write(script, 'STAGE_NAME=probe\n'+exports+'\n'+
                  f'just --justfile {f.root}/justfile app_dir={app} a2o_dir={a2o} test mesh fixture.feature\n')
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
                    # gc reports an unclosable record in `failed` and keeps sweeping the rest.
                    failed=json.loads(r.stdout)['failed']
                    assert r.returncode!=0 and [x['session'] for x in failed]==[f.id('claude')],r
                    assert 'envelope-berth-unreachable' in failed[0]['error'],r
                    assert path.read_bytes()==before
                    (f.base/'unreachable-berth').rename(store_a)
                    f.run([BINARY,'session','gc','--json'],extra_env={'BERTH_DIR':str(store_b)})
                else:
                    assert r.returncode==0,r
                assert 'fixture-claude' not in json.loads((store_a/'leases.json').read_text())
                assert 'fixture-codex' in json.loads((store_a/'leases.json').read_text())
                assert sentinel.read_text()=='{"unrelated":true}'

    def test_berth_from_a_linked_worktree_keeps_the_session(self):
        # Berth's store is device-scoped, so a lease taken from any checkout belongs to the same
        # envelope: pre-push runs berth from the worktree being pushed (2026-10-25, the push was
        # refused `session-conflict`). Checkout-scoped facets keep the bound checkout.
        with tempfile.TemporaryDirectory(prefix='envelope-worktree-') as base:
            f=Fixture(base); f.setup()
            opened=f.epr('session','open','--json')
            worktree=f.base/'worktree'
            f.run(['git','worktree','add','-q','--detach',worktree],native=False)
            def from_worktree(*args):
                return subprocess.run([sys.executable,str(f.base/'exec.py'),*map(str,args)],
                    cwd=worktree,env=f.env(),capture_output=True,text=True,timeout=45)
            r=from_worktree(REPO/'genesis/agentic/bin/berth','claim','fixture-worktree')
            assert r.returncode==0, r
            leases=json.loads((f.base/'berth/leases.json').read_text())
            assert leases['fixture-worktree']['holder']==f.id('claude'), leases
            record=json.loads((f.base/f'config-home/elohim/device/sessions/{f.id("claude")}.json').read_text())
            assert record['repository']==opened['repository'] and record['berthStore']==str(f.base/'berth')
            r=from_worktree(BINARY,'flow','claim','--on','envelope#1','--json')
            assert r.returncode!=0 and 'envelope-checkout-mismatch' in r.stderr, r
            f.epr('session','close','--json')
            assert 'fixture-worktree' not in json.loads((f.base/'berth/leases.json').read_text())

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


# Cases that assert on a hook or native call with a 5-10 s budget whose expiry the case reads
# as a product failure, not a tolerated degradation. They run one at a time after the parallel
# batch, so four-way load cannot spend those budgets. Remove one only on measured evidence.
SERIAL_TAIL = (
    # load-project-context.py's 5 s recall and run-projection.py's 6 s budgets: an expiry
    # prints `bootstrap: skipped`, which the case refuses.
    'EnvelopeTest.test_rebuilt_cargo_target_does_not_replace_hook_executor',
    # commit-attribution.py's 5 s `epr session commit-trailers`: an expiry leaves the message
    # UNMEASURED and undecorated, and the case asserts the trailers.
    'EnvelopeTest.test_commit_decoration_uses_mounted_model_and_session_without_claiming_authority',
    # _lib/session.py's 5 s `epr session show` in capability-tier-gate.py: an expiry reads as no
    # envelope, so the gate denies where the case expects it to pass.
    'EnvelopeTest.test_harness_model_caps_the_real_envelope_claim',
    # The 10 s rendezvous includes a launcher `epr flow note` that lazily opens the envelope.
    'EnvelopeTest.test_berth_direct_claim_racing_close_is_fenced_and_post_close_refuses',
)


def case_ids(exclude=()):
    """Every test case in this module, `Class.method`, longest source first.

    Each case builds its own Fixture under its own TemporaryDirectory: config (XDG_CONFIG_HOME),
    the bin store, berth, proc view, git checkout and flow stores all live under that base.
    Outside it, the cases read this repository's hooks, berth and tested epr, and write three
    things: epr_client's resolution and candidate caches and _observation's probe cache, all in
    `tempfile.gettempdir()`, and Python bytecode beside the hooks they import. `run_parallel`
    gives each case its own TMPDIR and PYTHONDONTWRITEBYTECODE=1, so no two cases share a
    writable path. The order is a balance heuristic only: long cases start first.
    """
    import inspect
    cases = [obj for obj in globals().values()
             if isinstance(obj, type) and issubclass(obj, unittest.TestCase)]
    ids, size = [], {}
    for case in cases:
        for name in unittest.TestLoader().getTestCaseNames(case):
            ids.append(f'{case.__name__}.{name}')
            size[ids[-1]] = len(inspect.getsourcelines(getattr(case, name))[0])
    unknown = sorted((set(exclude) | set(SERIAL_TAIL)) - set(ids))
    if unknown:
        raise SystemExit(f'session_envelope_test: --exclude or SERIAL_TAIL names no case: {unknown}')
    return sorted((i for i in ids if i not in exclude), key=lambda i: -size[i])


def run_parallel(jobs, exclude, case_timeout):
    """Run each case once, in its own process and its own TMPDIR, `jobs` at a time, then the
    SERIAL_TAIL one at a time. Results print as they complete; a case that fails, times out,
    is interrupted or cannot be run prints its whole output, and the last line names them all.

    Every exit path of a case removes its TMPDIR, which holds its fixture base: a detached
    `epr session hold` (its own session, out of reach of the group signal) then finds its fake
    owner gone and exits by its own one-second poll. An interrupt or an exception in the runner
    ends every running case, starts no more, and exits non-zero.
    """
    import signal
    import threading
    import time
    import traceback
    from concurrent.futures import ThreadPoolExecutor, as_completed
    me = str(Path(__file__).resolve())
    stopping = threading.Event()

    def end(proc):
        # SIGINT, not SIGTERM: Python's default SIGTERM exits without unwinding, so the case's
        # TemporaryDirectory would not be removed; SIGINT raises KeyboardInterrupt through it.
        # Then SIGKILL whatever of the group remains. Both waits are bounded.
        for sig in (signal.SIGINT, signal.SIGKILL):
            try:
                os.killpg(proc.pid, sig)
            except ProcessLookupError:
                pass
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                pass

    def collect(proc):
        try:
            return proc.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            for pipe in (proc.stdout, proc.stderr):
                pipe.close()
            return '', '(output lost: a descendant of the case kept its pipes open)'

    def one(case):
        started = time.monotonic()
        home = tempfile.mkdtemp(prefix='envelope-case-')
        proc = None
        try:
            if stopping.is_set():
                return case, 'NOT RUN (interrupted)', '', '', 0.0
            env = {**os.environ, 'TMPDIR': home, 'PYTHONDONTWRITEBYTECODE': '1'}
            proc = subprocess.Popen([sys.executable, me, case], stdout=subprocess.PIPE,
                                    stderr=subprocess.PIPE, text=True, start_new_session=True,
                                    env=env)
            deadline = started + case_timeout
            while True:
                try:
                    out, err = proc.communicate(timeout=1)
                    verdict = 'ok' if proc.returncode == 0 else 'FAIL'
                    break
                except subprocess.TimeoutExpired:
                    if stopping.is_set():
                        verdict = 'INTERRUPTED'
                    elif time.monotonic() >= deadline:
                        verdict = f'TIMEOUT after {case_timeout:.0f}s'
                    else:
                        continue
                    end(proc)
                    out, err = collect(proc)
                    break
            return case, verdict, out, err, time.monotonic() - started
        except Exception:  # a runner failure is this case's failure, reported with its text
            if proc is not None and proc.poll() is None:
                end(proc)
                collect(proc)
            return case, 'ERROR', '', traceback.format_exc(), time.monotonic() - started
        finally:
            shutil.rmtree(home, ignore_errors=True)

    ids = case_ids(exclude)
    # Every case's setup links (or copies) and ingests the tested epr; the 158 MB debug build
    # made that the heaviest step of each case and, at eight fixtures at once, outran the 45 s
    # command guard. (The Rust harness already hands over a stripped epr; stripping it again
    # writes one 66 MB file.)
    # The cases run one stripped copy instead (66 MB), with bin-launch beside it as setup looks
    # for it. It must be BINARY itself, not only the ingested candidate: cases mix direct BINARY
    # calls with launcher calls that run the candidate, and recall's executor digest
    # (`session_envelope::executor_cid`, the running file's CID) holds only while both are the
    # same bytes. Nothing the cases run reads symbols or debug info. Without `strip`, unchanged.
    binary_dir = tempfile.mkdtemp(prefix='envelope-epr-')
    binary = BINARY
    try:
        stripped = subprocess.run(['strip', '--strip-all', '-o', os.path.join(binary_dir, 'epr'),
                                   str(BINARY)], capture_output=True, timeout=120).returncode == 0
    except (OSError, subprocess.SubprocessError):
        stripped = False
    if stripped:
        shutil.copy2(BINARY.with_name('bin-launch'), os.path.join(binary_dir, 'bin-launch'))
        binary = Path(binary_dir) / 'epr'
    else:
        print('session_envelope_test: strip unavailable; the cases run the unstripped epr',
              file=sys.stderr, flush=True)
    # Child cases inherit the caller-selected executable, or this per-run stripped derivative.
    os.environ['EPR_BIN'] = str(binary)
    batches = ([i for i in ids if i not in SERIAL_TAIL], jobs), ([i for i in ids if i in SERIAL_TAIL], 1)
    failed, reported, interrupted = [], set(), None

    def report(future):
        if future in reported or future.cancelled() or not future.done():
            return
        reported.add(future)
        case, verdict, out, err, took = future.result()
        print(f'{verdict} {case} ({took:.1f}s)', file=sys.stderr, flush=True)
        if verdict != 'ok':
            failed.append(case)
            print(f'--- {case} stdout ---\n{out}--- {case} stderr ---\n{err}',
                  file=sys.stderr, flush=True)

    try:
        for cases, width in batches:
            pool = ThreadPoolExecutor(max_workers=max(1, width))
            futures = [pool.submit(one, case) for case in cases]
            try:
                for future in as_completed(futures):
                    report(future)
            except BaseException:
                stopping.set()
                raise
            finally:
                pool.shutdown(wait=True, cancel_futures=True)
                for future in futures:
                    report(future)
    except BaseException as error:  # KeyboardInterrupt included: every case above was ended
        interrupted = f'{type(error).__name__}: {error}'
        traceback.print_exc()
    finally:
        shutil.rmtree(binary_dir, ignore_errors=True)
        print(f'session_envelope_test: {len(ids)} cases ({len(batches[1][0])} serial), '
              f'{jobs} at a time; {len(failed)} failed'
              f'{": " + ", ".join(failed) if failed else ""}'
              f'{"; runner stopped: " + interrupted if interrupted else ""}',
              file=sys.stderr, flush=True)
    return 1 if failed or interrupted else 0


if __name__ == '__main__':
    # Run as a program (--stage and --parallel bypass unittest's setUpModule): choose the caller's epr now.
    setUpModule()
    if '--stage' in sys.argv:
        p=argparse.ArgumentParser();p.add_argument('--stage');p.add_argument('--base');a=p.parse_args()
        getattr(Fixture(a.base),a.stage)()
    elif '--parallel' in sys.argv:
        p = argparse.ArgumentParser()
        p.add_argument('--parallel', action='store_true')
        p.add_argument('--jobs', type=int, default=min(4, os.cpu_count() or 1))
        p.add_argument('--exclude', action='append', default=[],
                       help='a Class.method another runner already runs')
        p.add_argument('--case-timeout', type=float, default=600.0)
        a = p.parse_args()
        sys.exit(run_parallel(a.jobs, a.exclude, a.case_timeout))
    else:
        unittest.main()
