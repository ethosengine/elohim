#!/usr/bin/env python3
"""Angular package checks against the exact archived bytes; no JavaScript execution."""
import argparse
import json
from html.parser import HTMLParser
from pathlib import Path
import re
from urllib.parse import unquote, urljoin, urlsplit
import zipfile

class Shell(HTMLParser):
    def __init__(self):
        super().__init__()
        self.base = '/'
        self.assets = []
        self.scripts = 0

    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if tag == 'base' and 'href' in attrs:
            self.base = attrs['href']
        if tag == 'script' and attrs.get('src'):
            self.assets.append(attrs['src'])
            self.scripts += 1
        if tag == 'link' and attrs.get('rel', '').lower() == 'stylesheet' and attrs.get('href'):
            self.assets.append(attrs['href'])


def check_browser(files):
    if 'index.html' not in files and 'index.csr.html' in files:
        files['index.html'] = files['index.csr.html']
    if 'index.html' not in files:
        raise ValueError('browser bundle needs index.html or index.csr.html')
    stamp = json.loads(files.get('version.json', b'null'))
    if not isinstance(stamp, dict) or not stamp.get('commit'):
        raise ValueError('browser bundle needs version.json with a nonempty commit')
    shell = Shell()
    shell.feed(files['index.html'].decode('utf-8'))
    if not shell.scripts:
        raise ValueError('browser shell names no executable script')
    base = urljoin('https://bundle.invalid/', shell.base)
    prefix = urlsplit(base).path
    for asset in shell.assets:
        url = urlsplit(urljoin(base, asset))
        if url.netloc != 'bundle.invalid':
            raise ValueError(f'asset must resolve through the same doorway: {asset}')
        path = unquote(url.path)
        relative = path[len(prefix):] if path.startswith(prefix) else path.lstrip('/')
        if relative not in files:
            raise ValueError(f'browser asset is missing: {asset} (bundle path {relative})')


def check_server(files):
    stamp = json.loads(files.get('version.json', b'null'))
    if not isinstance(stamp, dict) or not stamp.get('commit'):
        raise ValueError('SSR bundle needs version.json with a nonempty commit')
    source = files.get('main.server.mjs', b'').decode('utf-8')
    if not source:
        raise ValueError('SSR bundle needs main.server.mjs')
    # Accept named declarations and esbuild export lists. Source conformance is
    # also checked by lint-ssr-entry.mjs; actual rendering remains the mesh gate.
    for name in ('default', 'renderApplication'):
        declaration = rf'export\s+(?:async\s+)?(?:function\s+|const\s+)?{name}\b'
        export_list = rf'export\s*\{{[^}}]*\b{name}\b[^}}]*\}}'
        if not (re.search(declaration, source) or re.search(export_list, source)):
            raise ValueError(f'SSR entry does not export {name}')



if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', required=True)
    parser.add_argument('--kind', choices=['browser', 'server'], required=True)
    parser.add_argument('--version')
    args = parser.parse_args()
    try:
        with zipfile.ZipFile(args.archive) as archive:
            files = {name: archive.read(name) for name in archive.namelist()}
        for name in files:
            if Path(name).name in ('spa-bundle.zip', 'browser.zip', 'server.zip'):
                raise ValueError(f'stale package archive in source dist: {name}')
        if args.version:
            if 'version.json' not in files:
                raise ValueError('SSR build stamp missing; rebuild and stamp browser/server together')
            if json.loads(files['version.json']) != json.loads(Path(args.version).read_bytes()):
                raise ValueError('browser and SSR version.json disagree')
        (check_browser if args.kind == 'browser' else check_server)(files)
    except (ValueError, OSError, KeyError, zipfile.BadZipFile) as error:
        parser.exit(2, f'Package refused: {error}\n')
