"""Deterministic host operations and immutable evidence; arbitrary effects are not exactly-once."""
import fnmatch
import json
import os
from pathlib import Path
import re
import subprocess
import time
import uuid

from shift_runtime import command, completed, prompt, session_from
from shift_evidence import oracle_proof, require_binary_evidence, require_measurements, satisfies
from shift_reconcile import dispatch_binding, native_argv, native_effect, observe_effect
from shift_state import (Refused, atomic, authority, digest, load, lock, native_context,
                         read, require_acceptance, revision, run, under)


KINDS = {'agent', 'measure', 'gate', 'household', 'lifecycle', 'native', 'commit', 'land', 'push', 'ci'}


def validate_steps(spec):
    if spec.get('measurement_resource') not in (None, 'cargo', 'mesh', 'disk-heavy'):
        raise Refused('unknown readiness measurement resource lease')
    env = spec.get('workload_env', {})
    if (not isinstance(env, dict) or not set(env) <= {'MESH_DIR', 'MESH_TRANSPORT_BACKEND', 'CARGO_TARGET_POOL_FAMILY'}
            or not all(isinstance(v, str) for v in env.values())):
        raise Refused('workload environment may only pin mesh root/transport and cargo family')
    seen = set()
    for step in spec['steps']:
        ident = step.get('id', '')
        if not ident or not ident.replace('-', '').isalnum() or ident in seen:
            raise Refused('step ids must be unique alphanumeric/hyphen identifiers')
        seen.add(ident)
        kind = step.get('kind')
        if kind not in KINDS:
            raise Refused(f'unknown operation kind: {kind}')
        if kind == 'agent':
            if not isinstance(step.get('prompt'), str) or not step['prompt'].strip():
                raise Refused('agent step requires a frozen prompt')
        elif kind in {'measure', 'household', 'lifecycle', 'native', 'ci'}:
            if kind == 'measure' and step.get('objective') is True:
                continue
            argv = step.get('argv')
            if not isinstance(argv, list) or not argv or not all(isinstance(v, str) for v in argv):
                raise Refused('host operation requires a frozen argv array')
            if kind == 'native' and argv[:2] != ['epr', 'flow']:
                raise Refused('native operation must use epr flow')
            if kind == 'native':
                native_argv(step, 'revision-read-at-dispatch')
            if kind == 'lifecycle' and not (argv[:3] == ['just', 'mesh', 'storage-restart']
                                            and len(argv) > 3
                                            and set(argv[3:]) <= {'matthew', 'jessica', 'james'}):
                raise Refused('host lifecycle permits only declared household storage restarts')
        elif kind == 'gate' and step.get('project') not in spec['verification']:
            raise Refused('gate must name a manifest-selected verification project')
        if kind in {'household', 'ci'} and not isinstance(step.get('proof'), str):
            raise Refused('household/CI operations require a declared structured oracle proof')
        if kind == 'household' and (not step.get('oracle') or not step.get('binary')):
            raise Refused('household operation requires an oracle and built/running binary binding')
    if not spec['steps']:
        raise Refused('no operations declared')


def op_dir(directory, step):
    return directory / 'operations' / step['id']


def receipt_history(directory, contract, step):
    operation = op_dir(directory, step)
    paths = list(operation.glob('receipt-*.json'))
    if (operation / 'receipt.json').exists():
        paths.append(operation / 'receipt.json')
    values = {}
    for path in paths:
        value = read(path)
        if value['operation'] != step['id'] or value['contract_sha256'] != digest(directory / 'contract.json'):
            raise Refused('operation receipt belongs to another contract')
        for source in value['evidence']:
            if digest(under(contract['worktree'], source['path'])) != source['sha256']:
                raise Refused(f"changed receipt evidence: {source['path']}")
        attempt = value.get('attempt')
        if attempt is None:
            # Read the original receipt format, without rewriting its bytes.
            logs = [re.search(r'(?:host|claude|codex)-(\d+)\.(?:log|jsonl)$', source['path'])
                    for source in value['evidence']]
            ids = {int(match[1]) for match in logs if match}
            attempt = ids.pop() if len(ids) == 1 else 1
        if type(attempt) is not int or attempt < 1 or attempt in values:
            raise Refused('ambiguous operation receipt attempt')
        if path.name != 'receipt.json' and path.name != f'receipt-{attempt}.json':
            raise Refused('receipt filename differs from its attempt')
        values[attempt] = {**value, 'attempt': attempt}
    return [values[attempt] for attempt in sorted(values)]


def receipts(directory, contract):
    # Validate all history, but only the latest attempt can authorize a next step.
    values = []
    for step in contract['steps']:
        history = receipt_history(directory, contract, step)
        if history:
            values.append(history[-1])
    return values


def save_receipt(operation, result):
    path = operation / 'receipt.json'
    if path.exists():
        path = operation / f"receipt-{result['attempt']}.json"
    atomic(path, result, exclusive=True)


def retryable(step):
    return step['kind'] in {'agent', 'gate'} or (step['kind'] == 'measure' and step.get('readonly') is True)


def retry_budget(contract, attempt):
    limit = contract['objective'].get('budget', {}).get('iterations')
    if type(limit) is not int or limit < 1 or attempt > limit:
        raise Refused('frozen objective iteration budget exhausted or missing')


def objective_history(directory, contract):
    return [{**row, 'objective': True} for step in contract['steps'] if step.get('objective') is True
            for row in receipt_history(directory, contract, step)]


def needs_stability(directory, contract, delivered):
    try:
        require_measurements(contract, objective_history(directory, contract), delivered)
        return False
    except Refused:
        return True


def stability_retry(directory, contract, delivered):
    candidates = []
    for step in contract['steps']:
        if step.get('objective') is True and step.get('readonly') is True:
            history = receipt_history(directory, contract, step)
            if history and history[-1]['exit'] == 0:
                candidates.append((history[-1], step))
    if not candidates or not needs_stability(directory, contract, delivered):
        return None
    latest, step = min(candidates, key=lambda pair: pair[0]['finished'])
    retry_budget(contract, latest['attempt'] + 1)
    return step


def next_operation(directory, contract, checkpoint):
    """Select only a new operation, proven completion, or bounded safe retry."""
    authority(directory, contract, checkpoint)
    current = revision(contract['worktree'])
    for step in contract['steps']:
        if step['kind'] in {'land', 'push'}:
            repeat = stability_retry(directory, contract, current)
            if repeat:
                return repeat
        history = receipt_history(directory, contract, step)
        latest = history[-1] if history else None
        operation = op_dir(directory, step)
        pending = read(operation / 'pending.json') if (operation / 'pending.json').exists() else None
        if latest and pending is None:
            raise Refused('operation receipt has no durable pending identity')
        if latest and pending and latest['attempt'] > pending['attempt']:
            raise Refused('receipt is newer than durable operation identity')
        if pending and (latest is None or pending['attempt'] > latest['attempt']):
            if not (operation / f"dispatch-{pending['attempt']}.json").exists():
                return step
            if step['kind'] == 'agent':
                retry_budget(contract, pending['attempt'] + 1)
                reconcile_operation(directory, contract, step, checkpoint)
                return step
            reconcile_operation(directory, contract, step, checkpoint)
            latest = receipt_history(directory, contract, step)[-1]
        if latest is None:
            return step
        stale = latest['revision'] != current and (step['kind'] == 'gate' or
                                                 (step['kind'] == 'measure' and step.get('readonly') is True))
        if latest['exit'] != 0 or stale:
            if not retryable(step):
                raise Refused(f"operation failed: {step['id']}; side effects require independent inspection")
            retry_budget(contract, latest['attempt'] + 1)
            return step
    return stability_retry(directory, contract, current)


def require_delivery_evidence(directory, contract, delivered, *, published=False):
    require_acceptance(contract, delivered)
    evidence = [r for r in receipts(directory, contract)
                if r['exit'] == 0 and r['revision'] == delivered and r['clean']]
    projects = {r.get('project') for r in evidence if r['kind'] == 'gate'}
    if not set(contract['verification']) <= projects:
        raise Refused('required gate receipts missing for exact delivered revision')
    kinds = {r['kind'] for r in evidence}
    if 'household' not in kinds:
        raise Refused('matching household receipt missing')
    require_measurements(contract, objective_history(directory, contract), delivered)
    for step in contract['steps']:
        if step['kind'] == 'household':
            require_binary_evidence(directory, step, delivered)
    if published and not {'land', 'push', 'ci'} <= kinds:
        raise Refused('authorized publication or downstream CI receipt missing')


def observer_paths(contract):
    return contract.get('observer_paths', [])


def clean(root, contract=None):
    allowed = observer_paths(contract or {})
    result = subprocess.run(['git', 'status', '--porcelain', '--ignore-submodules=dirty'],
                            cwd=root, text=True, capture_output=True, check=True)
    return all(row.startswith(' M ') and row[3:] in allowed for row in result.stdout.splitlines())


def scoped_changes(contract):
    root = contract['worktree']
    paths = run(['git', 'diff', '--name-only', contract['baseline_revision']], root).splitlines()
    paths += run(['git', 'ls-files', '--others', '--exclude-standard'], root).splitlines()
    observers = set(observer_paths(contract))
    staged = run(['git', 'diff', '--cached', '--name-only'], root).splitlines()
    if observers.intersection(staged):
        raise Refused('generated observer state is staged; preserve it outside the product commit')
    paths = [p for p in paths if p not in observers]
    scopes = contract['objective']['scope']['paths']
    outside = [p for p in paths if not any(fnmatch.fnmatchcase(p, scope) for scope in scopes)]
    if outside:
        raise Refused(f'changes outside frozen scope: {outside}')
    return sorted(set(paths))


def host_argv(directory, contract, state, step):
    kind, root = step['kind'], contract['worktree']
    authority(directory, contract, state, publication=kind in {'land', 'push'})
    if kind == 'lifecycle':
        grant = read(under(root, contract['authority']['path']))
        if grant.get('household_lifecycle') is not True:
            raise Refused('household lifecycle authorization absent')
    if kind == 'gate':
        return ['just', 'gate', step['project']]
    if kind == 'measure' and step.get('objective') is True:
        return ['sh', '-c', contract['objective']['measure']['run']]
    if kind == 'native':
        return native_argv(step, revision(root))
    if kind == 'commit':
        paths = scoped_changes(contract)
        if not paths:
            raise Refused('commit operation has no scoped changes')
        staged = set(run(['git', 'diff', '--cached', '--name-only'], root).splitlines())
        if not staged <= set(paths):
            raise Refused('index contains paths outside this scoped commit')
        run(['git', 'add', '--', *paths], root)
        return ['git', 'commit', '-m', step['message'], '-m', f"Shift-Operation: {contract['id']}/{step['id']}"]
    if kind in {'land', 'push'}:
        delivered = revision(root)
        scoped_changes(contract)
        if not clean(root, contract):
            raise Refused('publication requires a clean scoped revision')
        require_delivery_evidence(directory, contract, delivered)
        # Normal ff-only update in the owning target checkout; never reset another session's WIP.
        landing = Path(contract['landing_worktree']).resolve()
        if not clean(landing):
            raise Refused('landing checkout has existing edits; preserve them')
        if run(['git', 'branch', '--show-current'], landing) != 'dev':
            raise Refused('landing checkout is not on dev')
        if kind == 'land':
            return ['git', '-C', str(landing), 'merge', '--ff-only', delivered]
        if revision(landing) != delivered:
            raise Refused('dev does not name the accepted delivered revision')
        return ['git', '-C', str(landing), 'push', 'origin', 'dev:dev']
    return step['argv']


def reconcile_operation(directory, contract, step, checkpoint=None):
    """Recognize narrowly inspectable side effects. Never re-run arbitrary shell on a guess."""
    operation = op_dir(directory, step)
    pending = read(operation / 'pending.json')
    checkpoint = checkpoint or load(directory)[1]
    authority(directory, contract, checkpoint, publication=step['kind'] in {'land', 'push'})
    if pending['contract_sha256'] != digest(directory / 'contract.json'):
        raise Refused('pending operation contract changed')
    if step['kind'] == 'agent':
        runtime = pending['runtime']
        session = session_from(runtime, operation / pending['log']) or pending.get('resume_session')
        if session and pending.get('session') and session != pending['session']:
            raise Refused('agent session evidence conflicts with saved exact session')
        if session:
            return session
        raise Refused('agent dispatch uncertain with no durable session id; inspect runtime records')
    dispatch_path = operation / f"dispatch-{pending['attempt']}.json"
    dispatch = read(dispatch_path)
    if step['kind'] not in {'commit', 'land', 'push'} and not (
            step['kind'] == 'native' and dispatch.get('argv', [])[:3] == ['epr', 'flow', 'fulfill']):
        raise Refused(f"unresolved side effect: {step['id']}; inspect evidence before any retry")
    if step['kind'] in {'land', 'push'}:
        if revision(contract['worktree']) != pending['revision']:
            raise Refused('publication recovery cannot bless a different revision')
        expected = host_argv(directory, contract, checkpoint, step)
        if dispatch.get('argv') != expected:
            raise Refused('publication dispatch changed')
    if step['kind'] == 'commit':
        expected = ['git', 'commit', '-m', step['message'], '-m', f"Shift-Operation: {contract['id']}/{step['id']}"]
        if dispatch.get('argv') != expected or not clean(contract['worktree'], contract):
            raise Refused('commit dispatch changed or source tree is dirty')
        scoped_changes(contract)
    witness = observe_effect(directory, contract, checkpoint, step, pending, dispatch)
    sources = [dispatch_path, witness]
    if step['kind'] == 'native':
        report = dispatch['native_binding']['report']
        original = under(contract['worktree'], report['path'])
        if digest(original) != report['sha256']:
            raise Refused('original fulfillment report changed before recovery receipt')
        sources.append(original)
    if (operation / pending['log']).exists():
        sources.append(operation / pending['log'])
    delivered = revision(contract['worktree']) if step['kind'] == 'commit' else pending['revision']
    result = {'operation': step['id'], 'kind': step['kind'], 'project': step.get('project'),
              'contract_sha256': pending['contract_sha256'], 'attempt': pending['attempt'],
              'exit': 0, 'revision': delivered, 'clean': clean(contract['worktree'], contract),
              'started': pending['prepared_at'], 'finished': time.time(), 'session': None,
              'recovered': True, 'evidence': [
                  {'path': str(path.relative_to(contract['worktree'])), 'sha256': digest(path)} for path in sources]}
    save_receipt(operation, result)
    return 'receipt'


def execute(directory, step_id):
    """Run as a guardian-owned worker, independent of the controller's lifetime."""
    contract, state = load(directory)
    step = next(s for s in contract['steps'] if s['id'] == step_id)
    operation = op_dir(directory, step)
    pending = read(operation / 'pending.json')
    if pending['contract_sha256'] != digest(directory / 'contract.json'):
        raise Refused('pending operation contract changed')
    before = revision(contract['worktree'])
    is_clean = clean(contract['worktree'], contract)
    if step['kind'] == 'agent':
        argv = command(pending['runtime'], session=pending.get('resume_session'),
                       fresh_id=pending.get('session'), tools=contract['allowed_tools'],
                       readonly=step.get('readonly', False))
        text = prompt(contract, step, directory)
    else:
        argv = host_argv(directory, contract, state, step)
        text = None
    # Persist dispatch intent BEFORE Popen. A crash in the following window is explicitly uncertain.
    dispatch = {
        'argv': argv, 'operation': step_id, 'revision': before, 'time': time.time(),
        'contract_sha256': pending['contract_sha256'], 'attempt': pending['attempt'],
        'native_binding': dispatch_binding(contract, step, argv),
    }
    dispatch_path = operation / f"dispatch-{pending['attempt']}.json"
    atomic(dispatch_path, dispatch, exclusive=True)
    with open(operation / pending['log'], 'ab', buffering=0) as output:
        from shift_supervisor import spawn_owned
        environment = {**os.environ, 'SHIFT_CONTRACT_SHA256': pending['contract_sha256'],
                       'SHIFT_OPERATION': step_id, 'SHIFT_ATTEMPT': str(pending['attempt']),
                       'SHIFT_PREPARED_AT': str(pending['prepared_at']), 'SHIFT_REVISION': before}
        if step.get('proof'):
            environment['SHIFT_PROOF_PATH'] = str(under(contract['worktree'], step['proof']))
        process = spawn_owned(argv, cwd=contract['worktree'], stdin=subprocess.PIPE,
                              stdout=output, stderr=subprocess.STDOUT, env=environment)
        process.communicate(text.encode() if text is not None else None)
        os.fsync(output.fileno())
    after = revision(contract['worktree'])
    session = session_from(pending['runtime'], operation / pending['log']) if step['kind'] == 'agent' else None
    exit_code = process.returncode
    if step['kind'] == 'agent' and exit_code == 0 and not completed(pending['runtime'], operation / pending['log']):
        exit_code = 125
    result = {
        'operation': step_id, 'kind': step['kind'], 'project': step.get('project'),
        'contract_sha256': pending['contract_sha256'], 'attempt': pending['attempt'], 'exit': exit_code,
        'objective': step.get('objective') is True,
        'revision': after, 'clean': is_clean and clean(contract['worktree'], contract) and before == after,
        'started': pending['prepared_at'], 'finished': time.time(), 'session': session,
        'evidence': [{'path': str((operation / pending['log']).relative_to(contract['worktree'])),
                      'sha256': digest(operation / pending['log'])}],
    }
    if step.get('artifact') and exit_code == 0:
        result['artifact'] = {'path': step['artifact'], 'sha256': digest(step['artifact'])}
    if dispatch['native_binding'] is not None and exit_code == 0:
        try:
            effect, context = native_effect(contract, dispatch)
            snapshot = operation / f"native-{pending['attempt']}.json"
            atomic(snapshot, {'effect': effect, 'native_context': context}, exclusive=True)
            if digest(under(contract['worktree'], effect['report']['path'])) != effect['report']['sha256']:
                raise Refused('original fulfillment report changed before receipt')
            for source in [dispatch_path, snapshot, under(contract['worktree'], effect['report']['path'])]:
                result['evidence'].append({'path': str(source.relative_to(contract['worktree'])),
                                           'sha256': digest(source)})
        except (Refused, OSError, ValueError, KeyError) as error:
            result.update(exit=125, refusal=str(error))
    if step.get('objective') is True and exit_code == 0:
        try:
            value = float((operation / pending['log']).read_text().strip())
            if not satisfies(value, contract['objective']['baseline']):
                raise ValueError('objective regression floor violated')
            result['measurement'] = value
            if not satisfies(value, contract['objective']['target']):
                raise ValueError('objective target not met')
        except (ValueError, KeyError) as error:
            result.update(exit=126, refusal=str(error))
    if step['kind'] in {'household', 'ci'} and exit_code == 0:
        try:
            proof = oracle_proof(contract, step, after, pending)
            snapshot = operation / f"proof-{pending['attempt']}.json"
            atomic(snapshot, proof, exclusive=True)
            result['evidence'].append({'path': str(snapshot.relative_to(contract['worktree'])), 'sha256': digest(snapshot)})
            result['evidence'].extend(proof['evidence'])
        except (Refused, OSError, ValueError, KeyError) as error:
            result.update(exit=125, refusal=str(error))
    save_receipt(operation, result)


def prepare(directory, contract, state, step):
    authority(directory, contract, state)
    operation = op_dir(directory, step)
    operation.mkdir(parents=True, exist_ok=True)
    pending_file = operation / 'pending.json'
    prior = read(pending_file) if pending_file.exists() else None
    session = None
    attempt = 1
    if prior:
        if prior['contract_sha256'] != digest(directory / 'contract.json'):
            raise Refused('pending operation contract changed')
        if not (operation / f"dispatch-{prior['attempt']}.json").exists():
            # Preparation is not dispatch. A controller can die before a worker ever
            # reaches Popen; do not resume a Claude UUID that was only reserved locally.
            if prior['runtime'] == state['runtime']:
                if prior['attempt'] > 1:
                    retry_budget(contract, prior['attempt'])
                return prior
            attempt = prior['attempt'] + 1
            archive = operation / f"preparation-{prior['attempt']}.json"
            if archive.exists():
                if read(archive) != prior:
                    raise Refused('preparation history conflicts with pending operation')
            else:
                atomic(archive, prior, exclusive=True)
            prior = None
    if prior:
        attempt = prior['attempt'] + 1
        retry_budget(contract, attempt)
        history = receipt_history(directory, contract, step)
        completed_attempt = next((r for r in history if r['attempt'] == prior['attempt']), None)
        if completed_attempt:
            stale = completed_attempt['revision'] != revision(contract['worktree']) and step['kind'] != 'agent'
            repeat = (step.get('objective') is True and step.get('readonly') is True
                      and needs_stability(directory, contract, revision(contract['worktree'])))
            if not retryable(step) or (completed_attempt['exit'] == 0 and not stale and not repeat):
                raise Refused('completed operation cannot be replayed')
        elif step['kind'] != 'agent':
            reconcile_operation(directory, contract, step, state)
            raise Refused('operation reconciled; select the next operation without dispatching it again')
        if step['kind'] == 'agent':
            previous_session = reconcile_operation(directory, contract, step, state)
            if prior['runtime'] == state['runtime']:
                session = previous_session
        # Cross-runtime recovery deliberately starts a fresh conversation, same evidence.
    if attempt > 1:
        retry_budget(contract, attempt)
    runtime = state['runtime']
    pending = {
        'operation': step['id'], 'contract_sha256': digest(directory / 'contract.json'),
        'attempt': attempt, 'runtime': runtime, 'resume_session': session,
        'session': session or (str(uuid.uuid4()) if runtime == 'claude' else None),
        'revision': revision(contract['worktree']), 'prepared_at': time.time(),
        'log': f'{runtime}-{attempt}.jsonl' if step['kind'] == 'agent' else f'host-{attempt}.log',
    }
    if prior:
        archive = operation / f'prior-{attempt - 1}.json'
        if archive.exists():
            if read(archive) != prior:
                raise Refused('attempt history conflicts with pending operation')
        else:
            atomic(archive, prior, exclusive=True)
    atomic(pending_file, pending)
    return pending
