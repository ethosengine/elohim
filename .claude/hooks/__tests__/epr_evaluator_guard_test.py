"""The remaining Python adapter fails open; custody and installation belong to Rust."""
import importlib.util
import subprocess
from pathlib import Path
from unittest.mock import patch

HOOK = Path(__file__).resolve().parents[1] / 'epr-evaluator-guard.py'


def load():
    spec = importlib.util.spec_from_file_location('guard_a3', HOOK)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_registry_initialization_is_fail_open(tmp_path, capsys):
    module = load()
    with patch.object(module, 'REPO', tmp_path):
        module.main()
    assert 'initialization error (ignored)' in capsys.readouterr().out


def test_bootstrap_refusal_never_installs(tmp_path, capsys):
    module = load()
    registry = tmp_path / 'genesis/agentic/bin-tools.json'
    registry.parent.mkdir(parents=True)
    registry.write_text('{"tools":{"epr":{}}}')
    calls = []

    def run(tool, *args):
        calls.append((tool, args))
        return subprocess.CompletedProcess([], 2, '', 'REFUSED bootstrap-dirty')

    with patch.object(module, 'REPO', tmp_path), patch.object(module, '_run', run):
        module.main()
    assert calls == [('epr', ('bootstrap', '--repo', str(tmp_path)))]
    assert 'launcher left untouched' in capsys.readouterr().out


def test_follow_failure_keeps_install_and_status_native(tmp_path):
    module = load()
    registry = tmp_path / 'genesis/agentic/bin-tools.json'
    registry.parent.mkdir(parents=True)
    registry.write_text('{"tools":{"epr":{}}}')
    calls = []

    def run(tool, *args):
        calls.append(args[0])
        if args[0] == 'follow':
            return subprocess.CompletedProcess([], 2, '', 'REFUSED unpublished')
        return subprocess.CompletedProcess([], 0, '{"head":"bafkreifixture","source":"current"}', '')

    with patch.object(module, 'REPO', tmp_path), patch.object(module, '_run', run):
        module.main()
    assert calls == ['bootstrap', 'follow', 'install-launcher', 'status']
