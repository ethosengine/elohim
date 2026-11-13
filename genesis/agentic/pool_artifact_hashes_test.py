"""Both Cargo layouts: python3 genesis/agentic/pool_artifact_hashes_test.py."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

LIB = Path(__file__).resolve().parent / 'bin/pool-lib.sh'


class PoolArtifactHashesTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix='pool hashes ')
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.serial = 0

    def age(self, path, hours):
        stamp = time.time() - hours * 3600
        for child in [*path.rglob('*'), path] if path.is_dir() else [path]:
            os.utime(child, (stamp, stamp))

    def unit(self, kind='lib-widget', hours=20, size=2**20 + 1,
             package='widget', profile=1, features='[]', target=1, source=1):
        self.serial += 1
        unit = self.root / 'slot/debug/build' / package / f'{self.serial:016x}'
        (unit / 'fingerprint').mkdir(parents=True)
        (unit / 'out').mkdir()
        (unit / 'fingerprint' / f'{kind}.json').write_text(json.dumps({
            'target': target, 'path': source, 'features': features, 'profile': profile}))
        (unit / 'fingerprint' / kind).write_text('0123456789abcdef')
        artifact = unit / 'out' / f'widget-{self.serial:016x}'
        with artifact.open('wb') as stream:
            stream.truncate(size)
        self.age(unit, hours)
        return unit

    def legacy(self, stem='libwidget', ext='.rlib', hours=20, source=None):
        self.serial += 1
        deps = self.root / 'old slot/debug/deps'
        deps.mkdir(parents=True, exist_ok=True)
        artifact = deps / f'{stem}-{self.serial:016x}{ext}'
        with artifact.open('wb') as stream:
            stream.truncate(2**20 + 1)
        if source:
            artifact.with_name(artifact.name + '.d').write_text(f'{artifact}: {source}\n')
        self.age(artifact, hours)
        return artifact

    def invoke(self, function='list_stale_artifact_hashes', keep=1, min_mb=1, apply=0):
        result = subprocess.run([
            'bash', '-c', 'source "$1"; "$2" "$3" "$4" "$5" 6 "$6"',
            '_', str(LIB), function, str(self.root), str(keep), str(min_mb), str(apply)],
            capture_output=True, text=True, check=True)
        return result.stdout.splitlines()

    def listed(self, **kwargs):
        return {Path(line.split('\t')[1]) for line in self.invoke(**kwargs)}

    def test_both_layouts_keep_newest_and_dry_run_is_read_only(self):
        old = self.unit(hours=30)
        middle = self.unit(hours=20)
        newest = self.unit(hours=10)
        legacy_old = self.legacy(hours=30)
        self.legacy(hours=10)
        self.assertEqual(self.listed(), {old, middle, legacy_old})
        self.assertEqual(self.listed(keep=2), {old})
        lines = self.invoke('prune_stale_artifact_hashes')
        self.assertEqual({Path(line.split('\t')[2]) for line in lines},
                         {old, middle, legacy_old})
        self.assertTrue(all(line.startswith('would-remove\t') for line in lines))
        self.assertTrue(all(p.exists() for p in (old, middle, newest, legacy_old)))
        expected_bytes = sum(int(line.split('\t')[0]) for line in self.invoke())
        self.assertEqual(int(self.invoke('stale_artifact_hash_bytes')[0]), expected_bytes)

    def test_target_feature_profile_and_package_groups_are_independent(self):
        variants = [
            {}, {'kind': 'test-lib-widget'}, {'kind': 'test-bin-widget'},
            {'kind': 'build-script-build-script-build'}, {'kind': 'lib-macro'},
            {'kind': 'test-integration-test-widget'}, {'features': '["extra"]'},
            {'profile': 2}, {'package': 'other'}, {'target': 2}, {'source': 2},
        ]
        originals = [self.unit(**variant) for variant in variants]
        self.assertEqual(self.listed(keep=0), set())
        self.unit(hours=10)
        self.assertEqual(self.listed(), {originals[0]})

    def test_age_size_hardlinks_and_unclassifiable_units_are_protected(self):
        old = self.unit(hours=30)
        young = self.unit(hours=1)
        self.unit(hours=0)
        small = self.unit(package='small', size=1)
        self.unit(package='small', hours=10)
        linked = self.unit(package='linked')
        os.link(next((linked / 'out').iterdir()), self.root / 'uplifted')
        self.unit(package='linked', hours=10)
        incomplete = self.unit(package='incomplete')
        next((incomplete / 'fingerprint').glob('*.json')).write_text('{')
        self.age(incomplete, 20)
        self.unit(package='incomplete', hours=10)
        run = self.unit(kind='run-build-script-build-script-build')
        self.unit(kind='run-build-script-build-script-build', hours=10)
        self.assertEqual(self.listed(), {old})
        self.assertTrue(all(p.exists() for p in (young, small, linked, incomplete, run)))

    def test_recent_descendant_and_symlink_are_protected(self):
        old = self.unit()
        self.unit(hours=0)
        # A rebuild can refresh contents without changing the hash directory mtime.
        self.age(next((old / 'out').iterdir()), 1)
        linked = self.unit(package='symlink')
        (linked / 'out/elsewhere').symlink_to(self.root / 'slot', target_is_directory=True)
        self.age(linked, 20)
        self.unit(package='symlink', hours=10)
        self.assertEqual(self.listed(), set())

    def test_legacy_executable_source_and_hardlink_rules_survive(self):
        old = self.legacy('widget', '', source='src/main.rs')
        self.legacy('widget', '', hours=10, source='src/main.rs')
        self.legacy('widget', '', source='src/lib.rs')
        self.legacy('widget', '')  # No dep-info: unclassifiable.
        linked = self.legacy('liblinked')
        os.link(linked, self.root / 'uplifted')
        self.legacy('liblinked', hours=10)
        self.assertEqual(self.listed(), {old})

    def test_apply_removes_whole_new_unit_but_only_legacy_artifact(self):
        # This fixture is the only place apply=1 is exercised; never the real pool.
        old = self.unit()
        new = self.unit(hours=10)
        legacy_old = self.legacy()
        legacy_new = self.legacy(hours=10)
        self.invoke('prune_stale_artifact_hashes', apply=1)
        self.assertFalse(old.exists())
        self.assertFalse(legacy_old.exists())
        self.assertTrue(new.exists())
        self.assertTrue(legacy_new.exists())


if __name__ == '__main__':
    unittest.main()
