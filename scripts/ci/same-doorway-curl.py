#!/usr/bin/env python3
"""The small curl surface used by serving probes, with same-origin redirects only."""
import sys
import urllib.error
import urllib.parse
import urllib.request


def origin(url):
    parsed = urllib.parse.urlsplit(url)
    return parsed.scheme, parsed.hostname, parsed.port or (443 if parsed.scheme == 'https' else 80)


class SameDoorway(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, newurl):
        if origin(request.full_url) != origin(newurl):
            raise ValueError(f'cross-doorway redirect refused: {request.full_url} -> {newurl}')
        return super().redirect_request(request, fp, code, msg, headers, newurl)


def main(args):
    output = headers = write_format = None
    timeout = 15
    url = None
    while args:
        arg = args.pop(0)
        if arg == '-o': output = args.pop(0)
        elif arg == '-D': headers = args.pop(0)
        elif arg == '-w': write_format = args.pop(0)
        elif arg == '-m': timeout = float(args.pop(0))
        elif arg in ('-sS', '-s', '-L'): pass
        elif arg.startswith('-'): raise ValueError(f'unsupported probe option: {arg}')
        else: url = arg
    if not url: raise ValueError('URL required')
    opener = urllib.request.build_opener(SameDoorway())
    try:
        response = opener.open(url, timeout=timeout)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        body = response.read()
        if output:
            with open(output, 'wb') as target: target.write(body)
        else: sys.stdout.buffer.write(body)
        if headers:
            with open(headers, 'w') as target:
                target.write(f'HTTP/1.1 {response.status}\r\n{response.headers}\r\n')
        if write_format:
            sys.stdout.write(write_format.replace('%{http_code}', str(response.status)))


if __name__ == '__main__':
    try: main(sys.argv[1:])
    except Exception as error:
        print(f'serving probe: {error}', file=sys.stderr)
        sys.exit(1)
