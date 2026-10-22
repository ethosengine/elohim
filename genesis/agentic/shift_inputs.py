"""Pins for declared private execution dependencies, separate from editable source."""
from pathlib import Path
import re
import shlex

from shift_state import Refused, digest, under


def private_path(root, value, *, relative=False):
    root = Path(root).resolve()
    if not isinstance(value, str) or (relative and Path(value).is_absolute()):
        raise Refused('private input paths must be worktree-relative strings')
    path = under(root, value)
    if not path.relative_to(root).as_posix().startswith('.claude/shifts/'):
        raise Refused('input_files only pins private files under .claude/shifts')
    if not path.is_file():
        raise Refused(f'private execution input missing: {value}')
    return path


def normalize_inputs(root, values):
    if not isinstance(values, list):
        raise Refused('input_files must be a list of private file paths')
    pins, seen = [], set()
    for value in values:
        path = private_path(root, value, relative=True)
        label = path.relative_to(root).as_posix()
        if label in seen:
            raise Refused(f'duplicate private execution input: {label}')
        seen.add(label)
        pins.append({'path': label, 'sha256': digest(path)})
    return pins


def shell_targets(command, depth=0):
    if depth > 4:
        raise Refused('private execution dependency inspection exceeded shell nesting limit')
    try:
        lexer = shlex.shlex(command, posix=True, punctuation_chars=';&|')
        lexer.whitespace_split = True
        argv = []
        for word in lexer:
            if word and all(char in ';&|' for char in word):
                yield from executable_targets(argv, depth)
                argv = []
            else:
                argv.append(word)
        yield from executable_targets(argv, depth)
    except ValueError as error:
        raise Refused(f'invalid objective shell declaration: {error}') from error


def executable_targets(argv, depth=0):
    """Inspect direct command/script operands, never output or native report arguments."""
    words = list(argv)
    if words and Path(words[0]).name == 'env':
        words.pop(0)
        while words and words[0].startswith('-'):
            flag = words.pop(0)
            if flag in {'-u', '--unset'} and words:
                words.pop(0)
    while words and re.match(r'^[A-Za-z_][A-Za-z0-9_]*=', words[0]):
        words.pop(0)
    if not words:
        return
    command, *args = words
    yield command
    name = Path(command).name
    shell = name in {'sh', 'bash', 'dash', 'zsh'}
    interpreter = shell or name in {'node', 'nodejs', 'source', '.'} or re.fullmatch(r'python(?:\d+(?:\.\d+)?)?', name)
    if not interpreter:
        return
    index = 0
    while index < len(args):
        arg = args[index]
        if arg in {'-c', '-e', '--eval', '--command'}:
            if shell and index + 1 < len(args):
                yield from shell_targets(args[index + 1], depth + 1)
            return
        if arg == '-m':
            return  # Python module names are not private script file operands.
        if arg in {'-W', '-X', '--require', '-r', '--import'}:
            index += 2
            continue
        if arg == '--':
            index += 1
            if index < len(args):
                yield args[index]
            return
        if not arg.startswith('-'):
            yield arg
            return
        index += 1


def verify_inputs(contract):
    root = Path(contract['worktree']).resolve()
    pins = contract.get('input_files', [])
    if not isinstance(pins, list):
        raise Refused('private execution input pins are missing or corrupt')
    pinned = set()
    for pin in pins:
        if not isinstance(pin, dict) or set(pin) != {'path', 'sha256'}:
            raise Refused('private execution input pin is corrupt')
        path = private_path(root, pin['path'], relative=True)
        label = path.relative_to(root).as_posix()
        if label in pinned or pin['path'] != label:
            raise Refused('private execution input pins are duplicate or noncanonical')
        pinned.add(label)
        if digest(path) != pin['sha256']:
            raise Refused(f'private execution input changed: {label}')
    measure = contract.get('objective', {}).get('measure', {}).get('run')
    targets = list(shell_targets(measure)) if isinstance(measure, str) else []
    for step in contract.get('steps', []):
        targets.extend(executable_targets(step.get('argv', [])))
    for target in targets:
        resolved = (root / target).resolve()
        if not resolved.is_relative_to(root / '.claude/shifts') and '.claude/shifts/' not in target:
            continue
        path = private_path(root, target)
        label = path.relative_to(root).as_posix()
        if label not in pinned:
            raise Refused(f'private executable dependency absent from input_files: {label}')
