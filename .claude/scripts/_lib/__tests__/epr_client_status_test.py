"""Clients consume the native bin-status view without deriving filesystem layouts."""
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from _lib import epr_client


def test_native_status_supplies_exact_path_and_cid():
    value = '{"resolved":"/device/cid/epr","resolvedCid":"bafkreifixture"}'
    with patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, value, '')) as run:
        assert epr_client.resolve({'BIN_STORE': '/private/store', 'PATH': '/device/bin'}) == ('/device/cid/epr', 'bafkreifixture')
    assert run.call_args.args[0] == ['epr', 'bin', 'status', '--tool', 'epr', '--json']
    assert run.call_args.kwargs['env']['BIN_STORE'] == '/private/store'


def test_a_multi_binary_release_resolves_by_its_manifest_cid():
    # epr + bin-launch releases are addressed by their DAG-CBOR sums manifest (bafyrei…). Accepting
    # only bafkrei… left every hook on such a device with no epr at all.
    value = '{"resolved":"/device/cid/epr","resolvedCid":"bafyreifixture"}'
    with patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, value, '')):
        assert epr_client.resolve({}) == ('/device/cid/epr', 'bafyreifixture')


def test_absent_or_legacy_identity_is_absence():
    for value in ['{}', '{"resolved":"/x","resolvedCid":"sha256:abc"}']:
        with patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, value, '')):
            assert epr_client.resolve({}) == (None, None)


def _launcher(tmp_path, name):
    target = tmp_path / name
    target.write_text('#!/bin/sh\n')
    target.chmod(0o755)
    bindir = tmp_path / 'bin'
    bindir.mkdir()
    (bindir / 'epr').symlink_to(target)
    return str(bindir)


def test_pre_native_status_queues_detached_bootstrap_and_names_fallback(tmp_path):
    legacy = subprocess.CompletedProcess([], 2, '', 'unknown command bin')
    path = _launcher(tmp_path, 'epr_store.py')
    with patch.object(subprocess, 'run', return_value=legacy), patch.object(subprocess, 'Popen') as spawn:
        assert epr_client.resolve({'PATH': path}) == (None, None)
        assert spawn.call_args.kwargs['start_new_session'] is True
        assert spawn.call_args.kwargs['stdout'] == subprocess.DEVNULL
        assert 'native-epr-bootstrap.mjs' in spawn.call_args.args[0][1]
        assert 'native evaluator bootstrapping' in epr_client.DEGRADED_NOTICE


def test_a_release_older_than_bin_behind_the_native_launcher_is_absence_not_arrival(tmp_path):
    old_pin = subprocess.CompletedProcess([], 2, '', 'unknown command `bin`')
    path = _launcher(tmp_path, 'bin-launch')
    with patch.object(subprocess, 'run', return_value=old_pin), patch.object(subprocess, 'Popen') as spawn:
        assert epr_client.resolve({'PATH': path}) == (None, None)
    spawn.assert_not_called()
    unresolved = subprocess.CompletedProcess([], 127, '', 'head-absent')
    with patch.object(subprocess, 'run', return_value=unresolved), patch.object(subprocess, 'Popen') as spawn:
        assert epr_client.resolve({'PATH': path}) == (None, None)
    spawn.assert_called_once()


def test_an_explicit_candidate_is_the_answer_and_never_runs_or_bootstraps(tmp_path):
    candidate = tmp_path / 'epr'
    candidate.write_bytes(b'candidate bytes')
    candidate.chmod(0o755)
    with patch.object(subprocess, 'run') as run, patch.object(subprocess, 'Popen') as spawn:
        path, cid = epr_client.resolve({'EPR_BIN': str(candidate)})
        assert epr_client.resolve({'EPR_BIN': str(tmp_path / 'absent')}) == (None, None)
    run.assert_not_called()
    spawn.assert_not_called()
    assert path == str(candidate.resolve())
    # `epr bin status` reports this CID for the same bytes as a candidate (native BlobCid raw).
    assert cid == 'bafkreidtfucy7lozbrypejbjej5llwohjemsc4ez57ttpksgqnoohjqiky'
