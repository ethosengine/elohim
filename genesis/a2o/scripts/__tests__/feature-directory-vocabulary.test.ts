import { strict as assert } from 'node:assert';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

const FEATURES_DIR = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', 'features');

void describe('system-scoped feature directories', () => {
  const systemDirectories = ['lms', 'rms', 'wms'];
  const retiredPillarDirectories = ['lamad', 'shefa', 'avodah'];

  for (const directory of systemDirectories) {
    void it(`${directory} exists with directory-local governance`, () => {
      assert.equal(existsSync(resolve(FEATURES_DIR, directory)), true);
      assert.equal(existsSync(resolve(FEATURES_DIR, directory, '.epr-meta')), true);
    });
  }

  for (const directory of retiredPillarDirectories) {
    void it(`does not recreate features/${directory}`, () => {
      assert.equal(existsSync(resolve(FEATURES_DIR, directory)), false);
    });
  }
});
