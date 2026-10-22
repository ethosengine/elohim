"""Bind household and downstream observations to the source that was delivered."""
from pathlib import Path
import math
import time

from shift_state import Refused, atomic, ci_policy, digest, finite_number, incarnation, read, revision, under


CI_EXECUTION_STATES = {'SUCCESS': 'completed', 'FAILURE': 'completed', 'UNSTABLE': 'completed',
                       'ABORTED': 'interrupted', 'NOT_BUILT': 'not_executed'}


def require_ci_findings(build):
    if build.get('result') == 'SUCCESS':
        return
    obligations = build.get('remaining_obligations')
    if (not isinstance(build.get('diagnosis'), str) or not build['diagnosis'].strip()
            or not isinstance(obligations, list) or not obligations
            or not all(isinstance(item, str) and item.strip() for item in obligations)
            or not isinstance(build.get('diagnosis_evidence'), list) or not build['diagnosis_evidence']):
        raise Refused('non-SUCCESS downstream inspection requires diagnosis, evidence and remaining obligations')


def satisfies(value, predicate):
    if not isinstance(value, (float, int)) or not math.isfinite(value):
        return False
    target = predicate['value']
    return {'==': value == target, '>=': value >= target, '<=': value <= target,
            '>': value > target, '<': value < target}[predicate['predicate']]


def require_measurements(contract, receipts, delivered):
    # Every attempt is an observation. A failed retry resets stability even when
    # a later receipt supersedes it for operation scheduling.
    measured = [r for r in receipts if r.get('objective') or 'measurement' in r]
    if any(not finite_number(r.get('finished')) for r in measured):
        raise Refused('objective observations lack finite completion times')
    measured.sort(key=lambda r: r['finished'])
    count = contract['objective']['target']['stability']['consecutive']
    if len(measured) < count or not all(r['revision'] == delivered and r['clean'] and r.get('exit') == 0
                                       and satisfies(r.get('measurement'), contract['objective']['target'])
                                       for r in measured[-count:]):
        raise Refused('objective target lacks its declared consecutive measurements at the delivered revision')


def oracle_proof(contract, step, delivered, pending):
    proof = read(under(contract['worktree'], step['proof']))
    if proof.get('revision') != delivered or proof.get('success') is not True:
        raise Refused('oracle proof does not establish success at the delivered revision')
    identity = {'contract_sha256': pending['contract_sha256'], 'operation': step['id'], 'attempt': pending['attempt']}
    if any(proof.get(key) != value for key, value in identity.items()):
        raise Refused('oracle proof belongs to another dispatch')
    observed = proof.get('observed_at')
    if not isinstance(observed, (float, int)) or not pending['prepared_at'] <= observed <= time.time():
        raise Refused('oracle proof is stale or future-dated')
    if not proof.get('observations') or not proof.get('evidence'):
        raise Refused('oracle proof requires observations and raw evidence paths')
    for source in proof['evidence']:
        if digest(under(contract['worktree'], source['path'])) != source['sha256']:
            raise Refused('oracle raw evidence changed')
    if step['kind'] == 'household':
        counts = proof.get('scenarios', {})
        if (counts.get('passed', 0) < 1 or counts.get('failed') != 0
                or counts.get('skipped') != 0 or proof.get('oracle') != step['oracle']):
            raise Refused('household oracle is absent, failed, skipped, or differs from declaration')
    else:
        policy = ci_policy(contract)
        if proof.get('ci_policy') != policy:
            raise Refused('CI proof policy differs from frozen contract')
        builds = proof.get('builds', [])
        required = set(contract['downstream'])
        selected = {name: [b for b in builds if b.get('pipeline') == name] for name in required}
        if not required or len(builds) != len(required) or any(len(rows) != 1 for rows in selected.values()):
            raise Refused('downstream proof has missing or conflicting selected builds')
        if not all(rows[0].get('revision') == delivered and rows[0].get('building') is False
                   and rows[0].get('result') in CI_EXECUTION_STATES
                   and rows[0].get('execution_state') == CI_EXECUTION_STATES[rows[0]['result']]
                   and rows[0].get('url', '').startswith('https://')
                   and type(rows[0].get('number')) is int and rows[0]['number'] > 0
                   for rows in selected.values()):
            raise Refused('terminal downstream build and execution state at the delivered revision are unestablished')
        all_succeeded = all(build['result'] == 'SUCCESS' for build in builds)
        if proof.get('all_succeeded') is not all_succeeded:
            raise Refused('CI proof aggregate conflicts with actual build outcomes')
        for build in builds:
            require_ci_findings(build)
            sources = [build.get('evidence'), *build.get('diagnosis_evidence', [])]
            if any(not isinstance(source, dict) or source not in proof['evidence'] for source in sources):
                raise Refused('downstream build or diagnosis lacks retained raw evidence')
        if policy == 'success' and not all_succeeded:
            raise Refused('downstream success at the delivered revision is unestablished')
    return proof


def binary_observation(directory, contract, step, position):
    """Called only by the host guardian, before/after the contained household oracle."""
    binary = step['binary']
    build = read(directory / 'operations' / binary['build_operation'] / 'receipt.json')
    artifact = build.get('artifact', {})
    delivered = revision(contract['worktree'])
    if (build.get('exit') != 0 or not build.get('clean') or build.get('revision') != delivered
            or build.get('contract_sha256') != digest(directory / 'contract.json')
            or not artifact.get('sha256')):
        raise Refused('household binary has no successful build receipt at the delivered revision')
    fields = Path(binary['pid_file']).read_text().split()
    if len(fields) != 2:
        raise Refused('household PID record must contain host PID and start ticks')
    pid = int(fields[0])
    before = incarnation(pid)
    if before is None or before['start'] != fields[1]:
        raise Refused('household PID record names a stale process incarnation')
    observed = digest(f'/proc/{pid}/exe')
    if before is None or incarnation(pid) != before or observed != artifact['sha256']:
        raise Refused('household process does not run the exact built storage binary')
    operation = directory / 'operations' / step['id']
    witness = {'revision': delivered, 'process': before, 'sha256': observed,
               'build_operation': binary['build_operation'], 'observed_at': time.time()}
    path = operation / f'binary-{position}.json'
    if path.exists():
        prior = read(path)
        if any(prior[key] != witness[key] for key in ['revision', 'process', 'sha256', 'build_operation']):
            raise Refused('existing household binary witness conflicts with current process')
        return prior
    atomic(path, witness, exclusive=True)
    return witness


def require_binary_evidence(directory, step, delivered):
    operation = directory / 'operations' / step['id']
    before, after = (read(operation / f'binary-{p}.json') for p in ['before', 'after'])
    for key in ['revision', 'process', 'sha256', 'build_operation']:
        if before[key] != after[key]:
            raise Refused('household binary/process changed during its oracle')
    if after['revision'] != delivered or after['observed_at'] < before['observed_at']:
        raise Refused('household binary witness is stale or its clock regressed')
