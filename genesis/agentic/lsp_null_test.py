"""Run: python3 -m unittest genesis.agentic.lsp_null_test"""
import io
import json
from pathlib import Path
import runpy
import subprocess
import sys
import unittest


SERVER = Path(__file__).parent / 'bin/lsp-null.py'
read_message = runpy.run_path(str(SERVER))['read_message']


def frame(message):
    body = json.dumps({'jsonrpc': '2.0', **message}).encode()
    return f'Content-Length: {len(body)}\r\n\r\n'.encode() + body


class NullServerTest(unittest.TestCase):
    def run_server(self, messages):
        result = subprocess.run([sys.executable, str(SERVER), 'epr unavailable — fixture'],
                                input=b''.join(frame(m) for m in messages),
                                capture_output=True, timeout=3)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, b'')
        stream = io.BytesIO(result.stdout)
        rows = []
        while (message := read_message(stream)) is not None:
            rows.append(message)
        return rows

    def test_initialize_warning_shutdown_and_exit(self):
        rows = self.run_server([
            {'id': 1, 'method': 'initialize', 'params': {}},
            {'method': 'initialized', 'params': {}},
            {'id': 'shutdown-id', 'method': 'shutdown'},
            {'method': 'exit'},
            {'id': 99, 'method': 'initialize'},
        ])
        self.assertEqual(rows, [
            {'jsonrpc': '2.0', 'id': 1, 'result': {'capabilities': {}}},
            {'jsonrpc': '2.0', 'method': 'window/showMessage',
             'params': {'type': 2, 'message': 'no diagnostics: epr unavailable — fixture'}},
            {'jsonrpc': '2.0', 'id': 'shutdown-id', 'result': None},
        ])

    def test_eof_after_initialize_or_shutdown(self):
        self.assertEqual(len(self.run_server([{'id': 'init', 'method': 'initialize'}])), 2)
        self.assertEqual(self.run_server([{'id': 0, 'method': 'shutdown'}])[0]['result'], None)
        self.assertEqual(self.run_server([]), [])

    def test_first_document_uri_gets_one_information_diagnostic(self):
        methods = ('textDocument/didOpen', 'textDocument/didChange', 'textDocument/didSave')
        uri = 'file:///workspace/src/first.rs'
        for method in methods:
            with self.subTest(first_notification=method):
                rows = self.run_server([
                    {'id': 1, 'method': 'initialize', 'params': {}},
                    {'method': 'initialized', 'params': {}},
                    {'method': 'textDocument/didOpen', 'params': {'textDocument': {}}},
                    {'method': method, 'params': {'textDocument': {'uri': uri}}},
                    {'method': method, 'params': {'textDocument': {'uri': uri}}},
                    *[{'method': later, 'params': {'textDocument': {
                        'uri': 'file:///workspace/src/second.rs'}}} for later in methods],
                    {'id': 2, 'method': 'shutdown'},
                    {'method': 'exit'},
                ])
                self.assertEqual(rows, [
                    {'jsonrpc': '2.0', 'id': 1, 'result': {'capabilities': {}}},
                    {'jsonrpc': '2.0', 'method': 'window/showMessage',
                     'params': {'type': 2, 'message': 'no diagnostics: epr unavailable — fixture'}},
                    {'jsonrpc': '2.0', 'method': 'textDocument/publishDiagnostics',
                     'params': {'uri': uri, 'diagnostics': [{
                         'range': {'start': {'line': 0, 'character': 0},
                                   'end': {'line': 0, 'character': 0}},
                         'severity': 3,
                         'source': 'epr-lsp',
                         'message': 'no diagnostics: epr unavailable — fixture',
                     }]}},
                    {'jsonrpc': '2.0', 'id': 2, 'result': None},
                ])

    def test_unknown_request_is_answered_and_document_without_uri_is_ignored(self):
        rows = self.run_server([{'method': 'textDocument/didOpen', 'params': {}},
                                {'id': 5, 'method': 'textDocument/hover'}])
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['id'], 5)
        self.assertEqual(rows[0]['error']['code'], -32601)


if __name__ == '__main__':
    unittest.main()
