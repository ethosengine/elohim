"""Frozen repair edges derived from immutable failed verification receipts."""
import hashlib
import json
import re
from pathlib import Path

from shift_state import Refused, authority, digest, native_context, read, revision, run, under


def eligible(step):
    return step['kind'] == 'gate' or (step['kind'] == 'measure' and
                                      step.get('objective') is True and step.get('readonly') is True)


def validate_spec(contract):
    repair = contract.get('repair')
    if repair is None:
        return
    steps = {step['id']: step for step in contract['steps']}
    if (not isinstance(repair, dict) or set(repair) != {'agent', 'commit', 'prompt'}
            or not all(isinstance(repair[key], str) and repair[key].strip() for key in repair)
            or steps.get(repair['agent'], {}).get('kind') != 'agent'
            or steps.get(repair['agent'], {}).get('readonly') is True
            or steps.get(repair['commit'], {}).get('kind') != 'commit'):
        raise Refused('repair requires an existing writable agent, host commit and bounded prompt')
    order = list(steps)
    if order.index(repair['agent']) >= order.index(repair['commit']):
        raise Refused('repair agent must precede its declared host commit')


def receipt_path(directory, step, attempt):
    operation = directory / 'operations' / step['id']
    first = operation / 'receipt.json'
    return first if first.exists() and read(first).get('attempt', 1) == attempt else operation / f'receipt-{attempt}.json'


def reference(directory, contract, step, receipt):
    path = receipt_path(directory, step, receipt['attempt'])
    return {'operation': step['id'], 'attempt': receipt['attempt'], 'revision': receipt['revision'],
            'path': str(path.relative_to(contract['worktree'])), 'sha256': digest(path)}


def validate_reference(directory, contract, target):
    validate_spec(contract)
    if not contract.get('repair') or not isinstance(target, dict) or set(target) != {'operation', 'attempt', 'revision', 'path', 'sha256'}:
        raise Refused('repair has no frozen edge or exact failed receipt identity')
    step = next((s for s in contract['steps'] if s['id'] == target['operation']), None)
    if not step or not eligible(step) or type(target['attempt']) is not int or target['attempt'] < 1:
        raise Refused('repair target is not a failed host verification')
    path = receipt_path(directory, step, target['attempt'])
    if under(contract['worktree'], target['path']) != path or digest(path) != target['sha256']:
        raise Refused('failed repair receipt missing or changed')
    receipt = read(path)
    if (receipt.get('operation') != step['id'] or receipt.get('attempt') != target['attempt']
            or receipt.get('revision') != target['revision'] or receipt.get('exit') == 0
            or receipt.get('verification_failure') is not True
            or receipt.get('contract_sha256') != digest(directory / 'contract.json')):
        raise Refused('repair target lacks an executed failed verification receipt')
    if not receipt.get('evidence'):
        raise Refused('repair failure has no retained raw evidence')
    for source in receipt['evidence']:
        if digest(under(contract['worktree'], source['path'])) != source['sha256']:
            raise Refused('repair failure raw evidence changed')
    return receipt


def require_open(directory, contract, checkpoint):
    authority(directory, contract, checkpoint)
    _, promise = native_context(contract)
    if promise.get('fulfillments'):
        raise Refused('repair closed after native fulfillment; preserve original delivery evidence')
    for step in contract['steps']:
        if step['kind'] in {'land', 'push', 'household', 'lifecycle'} or step.get('artifact') or (
                step['kind'] == 'native' and step.get('argv', [])[:3] == ['epr', 'flow', 'fulfill']):
            operation = directory / 'operations' / step['id']
            if list(operation.glob('dispatch-*.json')) or list(operation.glob('receipt*.json')):
                raise Refused('repair closed after household/artifact/fulfillment/publication dispatch, including uncertain effects')


def fingerprint(contract):
    from shift_ops import scoped_changes
    paths = scoped_changes(contract)
    root = contract['worktree']
    data = [run(['git', 'diff', '--binary', 'HEAD', '--', *paths], root) if paths else '']
    for name in run(['git', 'ls-files', '--others', '--exclude-standard'], root).splitlines():
        if name not in contract.get('observer_paths', []):
            data.append([name, digest(under(root, name))])
    return hashlib.sha256(json.dumps(data, sort_keys=True).encode()).hexdigest()


def validate_pending(directory, contract, checkpoint, step, pending, *, prepared=True):
    target = pending.get('repair_for')
    if target is None:
        return
    validate_reference(directory, contract, target)
    require_open(directory, contract, checkpoint)
    if step['id'] not in (contract['repair']['agent'], contract['repair']['commit']):
        raise Refused('operation is outside the frozen repair edge')
    if prepared and step['kind'] == 'agent' and (not isinstance(pending.get('source_before'), str)
                                                or not re.fullmatch(r'[0-9a-f]{64}', pending['source_before'])):
        raise Refused('repair source_before requires its prepared source digest before dispatch')


def next_repair(directory, contract, checkpoint):
    """Return a declared agent/commit with a validated repair edge, or normal scheduling."""
    if not contract.get('repair'):
        return None
    from shift_ops import receipt_history, reconcile_operation, retry_budget
    validate_spec(contract)
    steps = {step['id']: step for step in contract['steps']}
    agent, commit = (steps[contract['repair'][kind]] for kind in ['agent', 'commit'])
    histories = {step['id']: receipt_history(directory, contract, step) for step in contract['steps']}
    for failed in contract['steps']:
        history = histories[failed['id']]
        if not eligible(failed) or not history or history[-1]['exit'] == 0:
            continue
        failure = history[-1]
        if failure.get('verification_failure') is not True:
            raise Refused('verification refusal is not an authorized repair signal')
        target = reference(directory, contract, failed, failure)
        if contract['steps'].index(agent) >= contract['steps'].index(failed):
            raise Refused('repair agent must precede its failed verification')
        validate_reference(directory, contract, target)
        associated = [r for r in histories[agent['id']] if r.get('repair_for') == target]
        # A later externally delivered revision is measured before inventing another repair.
        if failure['revision'] != revision(contract['worktree']) and not associated:
            continue
        require_open(directory, contract, checkpoint)
        if not associated or associated[-1]['exit'] != 0:
            retry_budget(contract, (histories[agent['id']][-1]['attempt'] if histories[agent['id']] else 0) + 1)
            return {**agent, 'repair_for': target}
        repaired = associated[-1]
        if repaired.get('changed_source') is not True:
            raise Refused('repair agent produced no source change; unresolved verification frontier')
        if repaired['revision'] != failure['revision']:
            raise Refused('repair agent changed revision; only the declared host commit may publish repairs')
        prior_commits = histories[commit['id']]
        if prior_commits:
            matched = [r for r in prior_commits if r.get('repair_for') == target]
            if matched:
                if matched[-1]['exit'] != 0:
                    raise Refused('repair commit failed; side effect requires independent inspection')
                continue
            operation = directory / 'operations' / commit['id']
            pending = read(operation / 'pending.json')
            if pending.get('repair_for') == target and pending['attempt'] > prior_commits[-1]['attempt']:
                validate_pending(directory, contract, checkpoint, commit, pending)
                if (operation / f"dispatch-{pending['attempt']}.json").exists():
                    reconcile_operation(directory, contract, commit, checkpoint)
                    return next_repair(directory, contract, checkpoint)
            retry_budget(contract, prior_commits[-1]['attempt'] + 1)
            return {**commit, 'repair_for': target}
    return None
