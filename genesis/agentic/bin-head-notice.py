#!/usr/bin/env python3
"""Bounded prompt observation: current head and the named session's pin per tool."""
import base64
import hashlib
import json
import os
from pathlib import Path


def pin_name(session):
    """The native pin_path encoding (raw CID only for names exceeding 200 bytes)."""
    safe = b'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.-'
    name = ''.join(chr(b) if b in safe else f'%{b:02x}' for b in session.encode())
    if len(name) > 200:
        name = 'b' + base64.b32encode(b'\x01\x55\x12\x20' + hashlib.sha256(session.encode()).digest()).decode().lower().rstrip('=')
    return name.replace('.', '%2e') if name in ('.', '..') else name


def notices(store, session, tools=('epr', 'brit')):
    """Notice each landing once, including a new landing returning to an older CID."""
    if not isinstance(session, str) or not session:
        return []
    lines = []
    name = pin_name(session)
    for tool_name in tools:
        tool = Path(store) / tool_name
        try:
            current = json.loads((tool / 'current.json').read_text())
            pin = json.loads((tool / 'sessions' / name).read_text())
            if pin.get('session') != session:
                continue
            old, new = pin['cid'], current['cid']
            if old == new:
                continue
            transition = json.dumps([new, current['landedAt']])
            marker = tool / 'notices' / name
            if marker.exists() and marker.read_text() == transition:
                continue
            marker.parent.mkdir(parents=True, exist_ok=True)
            temporary = marker.with_name(marker.name + f'.{os.getpid()}.tmp')
            temporary.write_text(transition)
            os.replace(temporary, marker)
            subject = str(current.get('subject') or current.get('commit') or '').replace('\n', ' ')
            lines.append(f'{tool.name}: pinned {old}; landed {new} ({subject}). Switch: /clear or epr bin pin --tool {tool.name} --cid {new}')
        except (OSError, ValueError, KeyError, TypeError):
            continue
    return lines
