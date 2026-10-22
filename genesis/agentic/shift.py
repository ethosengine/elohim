#!/usr/bin/env python3
"""Shared Claude/Codex shift launcher. See shift-contract.example.json and SHIFT.md."""
import argparse
import contextlib
import datetime as dt
import fcntl
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time

from shift_ops import execute, validate_steps
from shift_inputs import normalize_inputs, verify_inputs
from shift_runtime import command, completed, session_from
from shift_state import (Refused, alive, atomic, authority, check_clock, digest, finite_number, home, incarnation,
                         load, lock, native_context, read, revision, run, signal_handle,
                         timestamp, under)
from shift_supervisor import contained, controller, guardian, lease, namespace_init, spawn_owned


def bounded(argv, root, timeout, log, text=None, *, env=None, check=None):
    if not finite_number(timeout) or timeout <= 0:
        raise Refused('readiness operation has no finite remaining time')
    expires = time.monotonic() + timeout
    def remaining():
        live = check() if check is not None else timeout
        if not finite_number(live) or live <= 0:
            raise Refused('readiness authority has no finite remaining time')
        left = min(live, expires - time.monotonic())
        if left <= 0:
            raise Refused(f'readiness operation exceeded remaining time: {log}')
        return left
    remaining()  # Never spawn after a stop, revocation or original deadline.
    with open(log, 'wb', buffering=0) as output:
        process = spawn_owned(contained(argv), cwd=root, stdin=subprocess.PIPE,
                              stdout=output, stderr=subprocess.STDOUT, env={**os.environ, **(env or {})})
        try:
            pending_input = text.encode() if text else None
            while True:
                try:
                    process.communicate(pending_input, timeout=min(0.2, remaining()))
                except subprocess.TimeoutExpired:
                    # communicate retains partially written stdin across timeouts.
                    pending_input = None
                    continue
                remaining()  # A completed child cannot hide concurrent revocation.
                break
        except BaseException:
            if process.poll() is None:
                process.kill()  # unshare's kill-child tears down PID 1 and its descendants.
            process.wait()
            raise
        finally:
            os.fsync(output.fileno())
    if process.returncode:
        raise Refused(f'readiness failed ({process.returncode}): {log}')


def preflight(spec, directory, *, checks=None):
    validate_steps(spec)
    root = Path(spec['worktree'])
    if not (root / '.git').is_file():
        raise Refused('isolated retained git worktree required')
    required = ['epr', 'node', 'just', 'claude', 'codex', 'unshare', 'setpriv', 'git']
    for binary in required:
        if shutil.which(binary) is None:
            raise Refused(f'required tool missing: {binary}')
    if not Path(spec['ark']).is_file():
        raise Refused('ark binary missing; build the manifest-selected elohim-ark gate')
    remaining = lambda: initialization_remaining(directory)
    if remaining() <= 0:
        raise Refused('deadline expired before readiness')
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    checks = checks or directory / 'readiness'
    checks.mkdir(parents=True, exist_ok=True)
    environment = spec.get('workload_env', {})
    bounded(['true'], root, min(10, remaining()), checks / 'namespace.log', env=environment, check=remaining)
    for runtime, argv in [('claude', ['claude', 'auth', 'status']), ('codex', ['codex', 'login', 'status'])]:
        bounded(argv, root, min(20, remaining()), checks / f'{runtime}-auth.log', env=environment, check=remaining)
        bounded(command(runtime, tools=spec['allowed_tools']), root, min(90, remaining()),
                checks / f'{runtime}-canary.jsonl',
                'Read-only launch canary. Execute pwd using the shell, then reply SHIFT_CANARY_OK. Do not edit files.',
                env=environment, check=remaining)
        transcript = checks / f'{runtime}-canary.jsonl'
        if not session_from(runtime, transcript) or not completed(runtime, transcript, canary_root=root):
            raise Refused(f'{runtime} canary lacks completed session and successful tool evidence')
    gates = run(['node', 'genesis/orchestrator/gate-runner.mjs', '--list'], root)
    available = {line.split('\t')[0] for line in gates.splitlines()}
    if not spec['verification'] or not set(spec['verification']) <= available:
        raise Refused('required gates absent from manifest-selected registry')
    measurement_resource = spec.get('measurement_resource')
    for resource in ({s.get('resource') for s in spec['steps']} | {measurement_resource}) - {None}:
        holder = run([str(root / 'genesis/agentic/bin/berth'), 'who', resource], root)
        if holder and holder != 'null':
            raise Refused(f'resource lease unavailable: {resource}: {holder}')
    remaining()
    lease(spec, measurement_resource)
    try:
        bounded(['node', 'genesis/agentic/readiness.mjs', '--objective', str(directory / 'objective.json'),
                 '--worktree', str(root), '--deadline', spec['deadline'], '--preserve-observers'],
                 root, remaining(), checks / 'baseline.json', env=environment, check=remaining)
    finally:
        lease(spec, measurement_resource, release=True)
    baseline = read(checks / 'baseline.json')
    if not baseline.get('ready'):
        raise Refused('baseline measurement or clean-worktree readiness failed')
    native_context(spec)
    remaining()
    return {'checks': str(checks), 'measured': baseline['baseline'], 'revision': revision(root)}


def utc_now():
    return dt.datetime.now(dt.timezone.utc).isoformat().replace('+00:00', 'Z')


def initialization_state(directory):
    frozen = read(directory / 'initialization.json')
    state = read(directory / 'initialization-checkpoint.json')
    if (frozen.get('version') != 1 or state.get('version') != 1
            or state.get('initialization_sha256') != digest(directory / 'initialization.json')
            or state.get('phase') not in {'preflight', 'publishing'}
            or type(state.get('attempt')) is not int or state['attempt'] < 0
            or not finite_number(state.get('last_wall'))):
        raise Refused('missing/corrupt frozen initialization state')
    return frozen, state


def initialization_remaining(directory, **updates):
    """Validate the original authority and advance, never reset, readiness's clock floor."""
    with lock(directory / 'state.lock'):
        frozen, state = initialization_state(directory)
        spec = frozen['spec']
        if state['phase'] != 'preflight':
            raise Refused('execution publication already began; incomplete state requires reconciliation')
        if (directory / 'stop.json').exists():
            raise Refused('explicit stop marker; initialization cannot resume')
        if digest(directory / 'objective.json') != spec['objective_sha256']:
            raise Refused('frozen objective changed during initialization')
        root = spec['worktree']
        if digest(under(root, spec['objective_path'])) != frozen['objective_source_sha256']:
            raise Refused('original objective source changed during initialization')
        verify_inputs(spec)
        authority_path = under(root, spec['authority']['path'])
        if digest(authority_path) != spec['authority']['sha256']:
            raise Refused('original authorization missing or changed during initialization')
        grant = read(authority_path)
        revocation = read(under(root, spec['authority']['revocation']))
        if (grant.get('build') is not True or not grant.get('source')
                or revocation.get('revoked') is not False or grant.get('deadline') != spec['deadline']):
            raise Refused('original initialization authorization revoked or invalid')
        now = time.time()
        remaining = check_clock(spec, state, now)
        state.update(updates)
        state['last_wall'] = now
        atomic(directory / 'initialization-checkpoint.json', state)
        return remaining


def start(path, runtime):
    # Contract declarations accept the same JSON/YAML parser as objectives.
    declaration_sha256 = digest(path)
    raw = run(['node', '--input-type=module', '-e',
               "import fs from 'node:fs'; import YAML from 'yaml'; "
               "console.log(JSON.stringify(YAML.parse(fs.readFileSync(process.argv[1], 'utf8'), {uniqueKeys:true})))", str(path)], Path.cwd())
    spec = json.loads(raw)
    root = Path(spec['worktree']).resolve()
    directory = home(root, spec['id'])
    objective_path = under(root, spec['objective_path'])
    objective_source_sha256 = digest(objective_path)
    objective = json.loads(run(['node', str(root / 'genesis/agentic/objective.mjs'), str(objective_path)], root))
    if digest(path) != declaration_sha256 or digest(objective_path) != objective_source_sha256:
        raise Refused('initialization input changed while being read')
    spec.update(version=1, worktree=str(root), objective=objective)
    spec['input_files'] = normalize_inputs(root, spec.get('input_files', []))
    verify_inputs(spec)
    # These five generated hook ledgers are operational state, never a source-clean
    # exemption selected by an agent. Pin their exact set into the frozen contract.
    spec['observer_paths'] = read(Path(__file__).with_name('shift-observer-paths.json'))
    if spec.get('mode') not in {'delivery', 'rehearsal'}:
        raise Refused('declare delivery or rehearsal mode')
    if spec['mode'] == 'rehearsal' and any(s['kind'] not in {'agent', 'measure', 'gate'} for s in spec['steps']):
        raise Refused('rehearsal cannot fulfill, publish, or touch the household')
    validate_steps(spec)
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    with lock(directory / 'execution.lock', blocking=False) as ownership:
        # Check again under execution ownership. A prior publication is never
        # reconstructed from initialization, even if both execution files vanished.
        if (directory / 'contract.json').exists() or (directory / 'checkpoint.json').exists():
            raise Refused('shift already has execution state; use resume, never replace it')
        prior = None
        if (directory / 'initialization.json').exists():
            prior, initial_state = initialization_state(directory)
            if initial_state['phase'] != 'preflight':
                raise Refused('execution publication already began; missing state requires reconciliation')
            spec.setdefault('started_at', prior['spec']['started_at'])
        else:
            if any((directory / name).exists() for name in
                   ('objective.json', 'initialization-checkpoint.json', 'driver', 'operations', 'readiness')):
                raise Refused('legacy or incomplete initialization lacks immutable authority; inspection only')
            spec.setdefault('started_at', utc_now())
        duration = timestamp(spec['deadline']) - timestamp(spec['started_at'])
        if not 0 < duration <= objective['budget']['wall_clock_min'] * 60:
            raise Refused('deadline exceeds the original objective budget')
        frozen = directory / 'objective.json'
        if prior is None:
            atomic(frozen, objective, exclusive=True)
        spec['objective_sha256'] = prior['spec']['objective_sha256'] if prior else digest(frozen)
        spec['authority']['sha256'] = digest(under(root, spec['authority']['path']))
        initialization = {'version': 1, 'declaration_sha256': declaration_sha256,
                          'objective_source_sha256': objective_source_sha256, 'spec': spec}
        if prior is not None:
            if initialization != prior:
                raise Refused('initialization declaration, scope, objective, authority or deadline changed')
        else:
            atomic(directory / 'initialization.json', initialization, exclusive=True)
            atomic(directory / 'initialization-checkpoint.json', {
                'version': 1, 'initialization_sha256': digest(directory / 'initialization.json'),
                'last_wall': time.time(), 'phase': 'preflight', 'attempt': 0,
            }, exclusive=True)
        initialization_remaining(directory)
        _, initial_state = initialization_state(directory)
        attempt = initial_state['attempt'] + 1
        initialization_remaining(directory, attempt=attempt)
        checks = directory / 'readiness' / f'attempt-{attempt:04d}'
        checks.mkdir(parents=True, exist_ok=False)
        try:
            spec['baseline'] = preflight(spec, directory, checks=checks)
            initialization_remaining(directory)
        except Exception as error:
            atomic(checks / 'refusal.json', {'reason': str(error), 'at': time.time()}, exclusive=True)
            # Keep the highest observed clock even after a failed command. A
            # rollback cannot manufacture more readiness time on the next start.
            with lock(directory / 'state.lock'):
                _, failed = initialization_state(directory)
                observed = time.time()
                if finite_number(observed):
                    failed['last_wall'] = max(failed['last_wall'], observed)
                atomic(directory / 'initialization-checkpoint.json', failed)
            raise
        spec.setdefault('baseline_revision', spec['baseline']['revision'])
        initialization_remaining(directory, phase='publishing')
        driver = directory / 'driver'
        driver.mkdir()
        for source in Path(__file__).parent.glob('shift*.py'):
            if not source.name.endswith('_test.py'):
                shutil.copyfile(source, driver / source.name)
        spec['driver'] = {p.name: digest(p) for p in driver.iterdir()}
        atomic(directory / 'contract.json', spec, exclusive=True)
        atomic(directory / 'checkpoint.json', {
            'version': 1, 'contract_sha256': digest(directory / 'contract.json'),
            'runtime': runtime, 'last_wall': time.time(), 'phase': 'ready',
            'pending': None, 'guardian': None, 'runs': [],
        }, exclusive=True)
        return launch(directory, ownership)


def launch(directory, ownership):
    contract, state = load(directory)
    for name, pin in contract['driver'].items():
        if digest(directory / 'driver' / name) != pin:
            raise Refused('frozen launcher bytes changed')
    with open(directory / 'guardian.log', 'ab', buffering=0) as output:
        process = subprocess.Popen([sys.executable, str(directory / 'driver/shift.py'), '_guardian',
                                    str(directory), str(ownership)], pass_fds=(ownership,),
                                   start_new_session=True, stdin=subprocess.DEVNULL, stdout=output, stderr=output)
    started = time.monotonic()
    while time.monotonic() - started < 25:
        if process.poll() is not None:
            raise Refused(f'guardian refused launch ({process.returncode}); inspect {directory}')
        ready = directory / 'guardian-ready.json'
        if ready.exists() and read(ready)['guardian'] == incarnation(process.pid):
            return {'shift': contract['id'], 'guardian': incarnation(process.pid), 'deadline': contract['deadline']}
        time.sleep(0.1)
    process.terminate()
    process.wait(timeout=5)
    raise Refused('guardian/controller launch handshake did not complete')


def resume(directory, runtime=None):
    if not directory.is_dir():
        raise Refused('legacy or missing shift; no complete execution contract')
    with lock(directory / 'execution.lock', blocking=False) as ownership:
        with lock(directory / 'state.lock'):
            contract, state = load(directory)
            if state['phase'] in {'complete', 'rehearsal-complete'}:
                # Re-read acceptance even when a cached checkpoint says done.
                if contract['mode'] == 'delivery':
                    from shift_ops import require_delivery_evidence
                    require_delivery_evidence(directory, contract, revision(contract['worktree']), published=True)
                return {'shift': contract['id'], 'phase': state['phase']}
            if runtime:
                state['runtime'] = runtime
            state['runs'].append({'runtime': state['runtime'], 'at': utc_now()})
            now = time.time()
            check_clock(contract, state, now)
            state['last_wall'] = now
            atomic(directory / 'checkpoint.json', state)
        return launch(directory, ownership)


def stop(directory):
    if not directory.is_dir():
        raise Refused('shift directory does not exist')
    with lock(directory / 'state.lock'):
        marker = directory / 'stop.json'
        if not marker.exists():
            atomic(marker, {'stopped_at': utc_now(), 'reason': 'explicit operator stop'}, exclusive=True)
        # Stopping remains possible with a corrupt checkpoint, using the independent handshake record.
        try:
            handle = read(directory / 'guardian-ready.json')['guardian']
        except Refused:
            handle = None
        signal_handle(handle, signal.SIGTERM)
    return {'stopped': True, 'automatic_recovery': 'refused'}


def status(directory):
    if not (directory / 'contract.json').exists():
        legacy = directory.parent / (directory.name + '.journal.md')
        return {'phase': 'legacy-inspection-only', 'journal': str(legacy), 'launchable': False}
    contract, state = read(directory / 'contract.json'), read(directory / 'checkpoint.json')
    result = {'shift': contract['id'], 'deadline': contract['deadline'], 'checkpoint': state,
              'guardian_alive': alive(state.get('guardian')),
              'refusals': [read(p) for p in sorted(directory.glob('refusal-*.json'))[-3:]]}
    try:
        authority(directory, contract, state)
        result['native'] = native_context(contract)[1]
    except (Refused, OSError, ValueError, KeyError) as error:
        result['refused'] = str(error)
    return result


def recover(root):
    """Git's existing worktree registry is the discovery source; legacy journals never launch."""
    output = run(['git', 'worktree', 'list', '--porcelain', '-z'], root)
    results = []
    for field in output.split('\0'):
        if not field.startswith('worktree '):
            continue
        worktree = Path(field.removeprefix('worktree '))
        for path in sorted((worktree / '.claude/shifts').glob('*/contract.json')):
            try:
                results.append(resume(path.parent))
            except (Refused, OSError, ValueError, KeyError) as error:
                results.append({'worktree': str(worktree), 'shift': path.parent.name, 'refused': str(error)})
    return results


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv and argv[0].startswith('_'):
        name, *args = argv
        if name == '_init':
            return namespace_init(args)
        if name == '_controller':
            return controller(Path(args[0]), args[1], args[2])
        if name == '_execute':
            execute(Path(args[0]), args[1])
            return 0
        if name == '_guardian':
            # The launcher transfers the already-held flock descriptor to its detached guardian.
            fd = int(args[1])
            supplied, expected = os.fstat(fd), os.stat(Path(args[0]) / 'execution.lock')
            if (supplied.st_dev, supplied.st_ino) != (expected.st_dev, expected.st_ino):
                raise Refused('execution ownership handoff failed')
            # Re-lock the inherited open description. An independently opened,
            # unlocked fd for the same inode cannot impersonate the current owner.
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            from shift_supervisor import _guard
            with os.fdopen(fd, 'rb'):
                return _guard(Path(args[0]))
        raise Refused('unknown internal operation')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['start', 'resume', 'status', 'stop', 'recover'])
    parser.add_argument('subject', nargs='?')
    parser.add_argument('--runtime', choices=['claude', 'codex'])
    options = parser.parse_args(argv)
    root = Path(run(['git', 'rev-parse', '--show-toplevel'], Path.cwd()))
    if options.action == 'recover':
        result = recover(root)
    elif options.action == 'start':
        if not options.subject:
            raise Refused('start requires a contract declaration path')
        result = start(Path(options.subject).resolve(), options.runtime or 'codex')
    else:
        directory = home(root, options.subject or '')
        if options.action == 'resume':
            result = resume(directory, options.runtime)
        elif options.action == 'stop':
            result = stop(directory)
        else:
            result = status(directory)
    print(json.dumps(result, indent=2))
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (Refused, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f'shift REFUSED: {error}', file=sys.stderr)
        sys.exit(3)
