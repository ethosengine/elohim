#!/usr/bin/env python3
"""Local archive checker/encoder. No network and no writes to source dist."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile


def package(dist, output, aliases=None, reject_name=None):
    source = Path(dist).resolve()
    output = Path(output).resolve()
    if not source.is_dir():
        raise ValueError(f'build output directory missing: {source}; build the app first')
    if output.is_relative_to(source):
        raise ValueError('archive output must be outside source dist')
    files = {}
    modes = {}
    for path in sorted(source.rglob('*')):
        if path.is_symlink():
            raise ValueError(f'symlink is not a bundle file: {path}')
        if path.is_file():
            name = path.relative_to(source).as_posix()
            # A dot-directory (or dotfile) anywhere under the dist is never a
            # build output. The one known producer: a doorway's bundle-heads
            # reconciler (doorway-service/src/render/registry.rs) materializes
            # every adopted server bundle into `.reconcile/<slug>/<hash16>/`
            # under whatever directory SSR_BUNDLE_PATH names — if that path is
            # ever pointed AT this dist (the household server-bundle feedback
            # loop), the scratch lands here and the next package ships it back
            # out as the app. Refuse loudly instead of silently excluding it:
            # a silent exclusion would hide the misconfiguration.
            parts = name.split('/')
            dotted_at = next((i for i, part in enumerate(parts) if part.startswith('.')), None)
            if dotted_at is not None:
                offender = source.joinpath(*parts[: dotted_at + 1])
                raise ValueError(
                    f"dist contains a hidden path ({'/'.join(parts[: dotted_at + 1])}): a "
                    'build output must never contain a dot-file or dot-directory (usually a '
                    'doorway that materialized scratch into this dist because SSR_BUNDLE_PATH '
                    f'pointed here); remove {offender} and point SSR_BUNDLE_PATH elsewhere'
                )
            if path.name == reject_name:
                raise ValueError(f'stale package archive in source dist: {path}')
            if '\\' in name or ':' in name:
                raise ValueError(f'nonportable archive path: {name}')
            files[name] = path.read_bytes()
            modes[name] = 0o755 if path.stat().st_mode & 0o111 else 0o644
    for target, fallback in (aliases or {}).items():
        if not isinstance(target, str) or not isinstance(fallback, str) or target.startswith('/') or '..' in Path(target).parts or '\\' in target or ':' in target:
            raise ValueError(f'unsafe archive alias: {target}')
        if target not in files and fallback in files:
            files[target] = files[fallback]
            modes[target] = modes[fallback]
    output.parent.mkdir(parents=True, exist_ok=True)
    # Sorted paths, fixed timestamps/permissions and no host metadata. Identical
    # input bytes produce the same archive/hash independently of build mtimes.
    with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, body in sorted(files.items()):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o100000 | modes[name]) << 16
            archive.writestr(info, body)
    return {'path': str(output), 'hash': 'sha256-' + hashlib.sha256(output.read_bytes()).hexdigest(), 'size': output.stat().st_size}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dist', required=True)
    parser.add_argument('--out', required=True)
    parser.add_argument('--aliases', default='{}')
    parser.add_argument('--reject-name')
    args = parser.parse_args()
    try:
        print(json.dumps(package(args.dist, args.out, json.loads(args.aliases), args.reject_name)))
    except (ValueError, OSError, KeyError) as error:
        parser.exit(2, f'Package refused: {error}\n')
