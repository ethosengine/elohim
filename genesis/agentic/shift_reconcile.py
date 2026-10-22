"""Observe narrow completed effects without repeating a mutation or altering native stock."""
import base64
import json
from pathlib import Path
import time

from shift_state import Refused, atomic, authority, digest, native_context, read, revision, run, under


def native_argv(step, delivered):
    argv = list(step['argv'])
    if step.get('commit_revision') is True:
        if argv[:3] != ['epr', 'flow', 'fulfill'] or '--commit' in argv:
            raise Refused('commit_revision requires fulfillment without explicit commit arguments')
        argv += ['--commit', delivered]
    return argv


def cid_text(value):
    # Native serde stores CID bytes in records; context exposes their base32 text.
    if isinstance(value, list):
        return 'b' + base64.b32encode(bytes(value)).decode().lower().rstrip('=')
    return value


def report_identity(contract, path):
    report = under(contract['worktree'], path)
    before = digest(report)
    context = json.loads(run(['epr', 'flow', 'context', str(report), '--json'], contract['worktree']))
    if digest(report) != before:
        raise Refused('fulfillment report changed while its native identity was inspected')
    return {'path': str(report.relative_to(contract['worktree'])), 'sha256': before,
            'cid': context['identity']['cid']}


def dispatch_binding(contract, step, argv):
    """Pin the existing native report before a fulfillment can have a side effect."""
    if step['kind'] != 'native' or argv[:3] != ['epr', 'flow', 'fulfill']:
        return None
    options, commits = {}, []
    index = 3
    while index < len(argv):
        flag = argv[index]
        if flag == '--json':
            index += 1
            continue
        if flag not in {'--on', '--report', '--status', '--as', '--session', '--commit'} or index + 1 == len(argv):
            raise Refused('recoverable fulfillment requires explicit bounded native arguments')
        value = argv[index + 1]
        if flag == '--commit':
            commits.append(value)
        elif flag in options:
            raise Refused('duplicate native fulfillment argument')
        else:
            options[flag] = value
        index += 2
    if options.get('--on') != contract['commitment'] or not options.get('--as') or not options.get('--report'):
        raise Refused('fulfillment must name the exact original commitment, report and actor')
    status = options.get('--status', '').strip().upper().replace('-', '_')
    if status not in {'DONE', 'DONE_WITH_CONCERNS'}:
        raise Refused('fulfillment has no discharging native status')
    native_context(contract)
    return {'commitment': contract['commitment'], 'scope_cid': contract['scope_cid'],
            'actor': options['--as'], 'status': status, 'commits': commits,
            'report': report_identity(contract, options['--report'])}


def native_effect(contract, dispatch):
    binding = dispatch.get('native_binding')
    if not binding or binding.get('commitment') != contract['commitment'] or binding.get('scope_cid') != contract['scope_cid']:
        raise Refused('native fulfillment has no original pinned dispatch evidence')
    if report_identity(contract, binding['report']['path']) != binding['report']:
        raise Refused('original fulfillment report bytes or native identity changed')
    context, promise = native_context(contract)
    rows = {}
    for line in under(contract['worktree'], '.eprfs/status/flows.jsonl').read_text().splitlines():
        row = json.loads(line)
        if row['cid'] in promise['fulfillments']:
            if row['cid'] in rows and rows[row['cid']] != row:
                raise Refused('conflicting native fulfillment records')
            rows[row['cid']] = row
    matches = []
    for row in rows.values():
        record = row['record']
        slots = record.get('classifiedAs', [])
        if (record.get('kind') == 'event' and record.get('action') == 'produce'
                and [cid_text(cid) for cid in record.get('fulfills', [])] == [binding['commitment']]
                and cid_text(record.get('inScopeOf')) == binding['scope_cid']
                and cid_text(record.get('resource')) == binding['report']['cid']
                and record.get('provider') == binding['actor']
                and record.get('quantity') == {'Count': {'value': 1.0, 'unit': 'task-report'}}
                and [s for s in slots if s.startswith('report:')] == ['report:' + binding['status']]
                and [s for s in slots if s.startswith('evidence:')] == ['evidence:' + binding['report']['cid']]
                and [s.removeprefix('commit:') for s in slots if s.startswith('commit:')] == binding['commits']):
            matches.append(row)
    if len(matches) != 1:
        raise Refused('exact native fulfillment is missing or ambiguous; no replay authorized')
    return {'record': matches[0], 'report': binding['report']}, context


def observe_effect(directory, contract, checkpoint, step, pending, dispatch):
    """Return a new immutable observation, never a synthetic native acceptance."""
    authority(directory, contract, checkpoint, publication=step['kind'] in {'land', 'push'})
    if (dispatch.get('operation') != step['id'] or dispatch.get('revision') != pending['revision']
            or dispatch.get('contract_sha256') != pending['contract_sha256']
            or dispatch.get('attempt') != pending['attempt']):
        raise Refused('dispatch identity missing or changed; side effect remains unresolved')
    root = contract['worktree']
    kind = step['kind']
    context = None
    if kind == 'commit':
        expected = f"{contract['id']}/{step['id']}"
        candidates = []
        for sha in run(['git', 'rev-list', f"{contract['baseline_revision']}..HEAD"], root).splitlines():
            trailers = run(['git', 'show', '-s', '--format=%(trailers:key=Shift-Operation,valueonly)', sha], root).splitlines()
            if expected in trailers:
                if trailers != [expected]:
                    raise Refused('commit operation trailer is ambiguous')
                candidates.append(sha)
        if len(candidates) != 1 or candidates[0] != revision(root):
            raise Refused('unique operation commit at current HEAD is unestablished')
        parents = run(['git', 'rev-list', '--parents', '-n', '1', candidates[0]], root).split()
        if parents != [candidates[0], pending['revision']]:
            raise Refused('operation commit does not descend directly from its dispatched revision')
        effect = {'commit': candidates[0], 'parent': pending['revision'], 'trailer': expected}
    elif kind in {'land', 'push'}:
        expected = pending['revision']
        if revision(root) != expected or revision(contract['landing_worktree']) != expected:
            raise Refused('publication differs from the originally dispatched revision')
        # Caller also checks current acceptance, required gates, household and target cleanliness.
        effect = {'revision': expected, 'landing_worktree': contract['landing_worktree']}
        if kind == 'push':
            rows = run(['git', 'ls-remote', 'origin', 'refs/heads/dev'], contract['landing_worktree']).splitlines()
            if len(rows) != 1 or rows[0].split() != [expected, 'refs/heads/dev']:
                raise Refused('remote dev does not name the originally accepted revision')
            effect['remote'] = rows[0]
    elif kind == 'native' and dispatch.get('argv', [])[:3] == ['epr', 'flow', 'fulfill']:
        if dispatch['argv'] != native_argv(step, pending['revision']) or dispatch_binding(contract, step, dispatch['argv']) != dispatch.get('native_binding'):
            raise Refused('native fulfillment dispatch or original report changed')
        effect, context = native_effect(contract, dispatch)
    else:
        raise Refused(f"unresolved side effect: {step['id']}; inspect task/build/commit/flow evidence before any retry")
    observation = {'operation': step['id'], 'contract_sha256': pending['contract_sha256'],
                   'attempt': pending['attempt'], 'effect': effect, 'observed_at': time.time()}
    if context is not None:
        observation['native_context'] = context
    path = directory / 'operations' / step['id'] / f"recovery-{pending['attempt']}.json"
    if path.exists():
        prior = read(path)
        if any(prior.get(key) != observation[key] for key in ['operation', 'contract_sha256', 'attempt', 'effect']):
            raise Refused('immutable recovery observation conflicts with current evidence')
    else:
        atomic(path, observation, exclusive=True)
    return path
