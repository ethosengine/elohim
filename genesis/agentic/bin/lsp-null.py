#!/usr/bin/env python3
"""An empty LSP endpoint keeps edits usable when the evaluator is unavailable."""
import json
import sys


def read_message(stream):
    headers = {}
    while True:
        line = stream.readline()
        if not line:
            return None
        if line in (b'\r\n', b'\n'):
            break
        name, value = line.decode('ascii').split(':', 1)
        headers[name.lower()] = value.strip()
    length = int(headers['content-length'])
    if length < 0:
        return None
    body = stream.read(length)
    return json.loads(body) if len(body) == length else None


def send_message(message):
    body = json.dumps({'jsonrpc': '2.0', **message}, ensure_ascii=False).encode('utf-8')
    sys.stdout.buffer.write(f'Content-Length: {len(body)}\r\n\r\n'.encode('ascii') + body)
    sys.stdout.buffer.flush()


def main(reason):
    diagnostic_sent = False
    while (message := read_message(sys.stdin.buffer)) is not None:
        method = message.get('method')
        if method == 'initialize':
            send_message({'id': message['id'], 'result': {'capabilities': {}}})
            send_message({'method': 'window/showMessage',
                          'params': {'type': 2, 'message': f'no diagnostics: {reason}'}})
        elif method in ('textDocument/didOpen', 'textDocument/didChange', 'textDocument/didSave'):
            uri = message.get('params', {}).get('textDocument', {}).get('uri')
            if not diagnostic_sent and isinstance(uri, str) and uri:
                send_message({'method': 'textDocument/publishDiagnostics',
                              'params': {'uri': uri, 'diagnostics': [{
                                  'range': {'start': {'line': 0, 'character': 0},
                                            'end': {'line': 0, 'character': 0}},
                                  'severity': 3,
                                  'source': 'epr-lsp',
                                  'message': f'no diagnostics: {reason}',
                              }]}})
                diagnostic_sent = True
        elif method == 'shutdown':
            send_message({'id': message['id'], 'result': None})
        elif method == 'exit':
            return
        elif 'id' in message and method:
            send_message({'id': message['id'], 'error': {'code': -32601, 'message': 'Method not found'}})


if __name__ == '__main__':
    try:
        main(sys.argv[1] if len(sys.argv) > 1 else 'epr unresolved')
    except (OSError, ValueError, KeyError, TypeError, AttributeError):
        pass
