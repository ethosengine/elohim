"""Read-only Cargo 1.100 unit GC candidates; stdout is bytes<TAB>hash-directory.

Called by pool-lib.sh alongside its legacy deps-file reader. Fingerprint names
identify target kinds (lib, test-lib, test-bin, build-script, etc.); Cargo's
target/path/features/profile fields keep distinct variants separate. Unknown or
incomplete fingerprints are retained, including run-build-script units: those
describe execution output rather than a compiled target and lack this identity.
"""
import json
import os
from pathlib import Path
import re
import stat
import sys
import time
from collections import defaultdict


def unit_key(unit):
    fingerprints = list((unit / 'fingerprint').glob('*.json'))
    if len(fingerprints) != 1 or not (unit / 'out').is_dir():
        return None
    fingerprint = fingerprints[0]
    if not re.match(r'^(lib-|bin-|test-|example-|bench-|build-script-)',
                    fingerprint.stem):
        return None
    data = json.loads(fingerprint.read_text())
    # A successful fingerprint has both the JSON and its hash marker.
    if not fingerprint.with_suffix('').is_file():
        return None
    identity = [data[key] for key in ('target', 'path', 'features', 'profile')]
    return (str(unit.parent), fingerprint.stem, json.dumps(identity, sort_keys=True))


def unit_stats(unit):
    """Apparent bytes like du -sb; protect hardlinks and any recent descendant."""
    size, newest, linked = 0, 0, False
    seen = set()

    def failed(error):
        raise error

    for directory, dirs, files in os.walk(unit, followlinks=False, onerror=failed):
        for path in [Path(directory), *(Path(directory) / name for name in files)]:
            info = path.lstat()
            if stat.S_ISLNK(info.st_mode):
                raise ValueError('symlink in unit')
            newest = max(newest, info.st_mtime)
            linked |= stat.S_ISREG(info.st_mode) and info.st_nlink > 1
            inode = (info.st_dev, info.st_ino)
            if inode not in seen:
                size += info.st_size
                seen.add(inode)
        if any((Path(directory) / name).is_symlink() for name in dirs):
            raise ValueError('symlink in unit')
    return size, newest, linked


def candidates(root, keep, min_mb, min_age_h):
    groups = defaultdict(list)
    # Prune traversal at a unit: generated output may itself contain build dirs.
    for directory, dirs, _ in os.walk(root, followlinks=False):
        unit = Path(directory)
        if unit.parent.parent.name != 'build' or not re.fullmatch(r'[0-9a-f]{16}', unit.name):
            continue
        dirs[:] = []
        try:
            key = unit_key(unit)
            if key is not None:
                size, newest, linked = unit_stats(unit)
                groups[key].append((newest, str(unit), size, linked))
        except (OSError, ValueError, KeyError, TypeError):
            # Concurrent or incomplete builds are never deletion candidates.
            continue
    cutoff = time.time() - min_age_h * 3600
    for units in groups.values():
        # Count all units before applying thresholds; even keep=0 retains one.
        for newest, path, size, linked in sorted(units, reverse=True)[max(1, keep):]:
            if not linked and newest < cutoff and size > min_mb * 1024 * 1024:
                yield size, path


if __name__ == '__main__':
    for size, path in candidates(sys.argv[1], *map(int, sys.argv[2:5])):
        print(f'{size}\t{path}')
