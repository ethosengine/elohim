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


def test_absent_or_legacy_identity_is_absence():
    for value in ['{}', '{"resolved":"/x","resolvedCid":"sha256:abc"}']:
        with patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, value, '')):
            assert epr_client.resolve({}) == (None, None)
