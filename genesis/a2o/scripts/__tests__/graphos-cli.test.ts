import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { parseGraphosArgs } from '../graphos.js';

void describe('parseGraphosArgs', () => {
  void it('parses list with optional filter', () => {
    const cmd = parseGraphosArgs(['list', 'compute']);
    assert.equal(cmd.verb, 'list');
    assert.equal(cmd.arg, 'compute');
    assert.equal(cmd.base, 'https://storybook.elohim.host');
  });
  void it('parses list with no filter', () => {
    assert.equal(parseGraphosArgs(['list']).arg, undefined);
  });
  void it('requires an argument for story and sheet', () => {
    assert.throws(() => parseGraphosArgs(['story']));
    assert.throws(() => parseGraphosArgs(['sheet']));
  });
  void it('parses story flags', () => {
    const cmd = parseGraphosArgs([
      'story', 'designed-core-elohim-compute-tile--standard',
      '--docs', '--base', 'http://localhost:6006/', '--out', 'my-slug',
      '--viewport', '800x600',
    ]);
    assert.equal(cmd.verb, 'story');
    assert.equal(cmd.docs, true);
    assert.equal(cmd.base, 'http://localhost:6006'); // trailing slash stripped
    assert.equal(cmd.out, 'my-slug');
    assert.deepEqual(cmd.viewport, { width: 800, height: 600 });
  });
  void it('parses sheet flags with defaults', () => {
    const cmd = parseGraphosArgs(['sheet', 'elohim-compute-tile']);
    assert.deepEqual(cmd.cell, { width: 420, height: 320 });
    assert.equal(cmd.cols, 3);
    assert.equal(cmd.family, undefined);
  });
  void it('parses --family and validates it', () => {
    assert.equal(
      parseGraphosArgs(['sheet', 'x', '--family', 'designed']).family,
      'designed'
    );
    assert.throws(() => parseGraphosArgs(['sheet', 'x', '--family', 'bogus']));
  });
  void it('parses --cell and --cols', () => {
    const cmd = parseGraphosArgs(['sheet', 'x', '--cell', '300x200', '--cols', '4']);
    assert.deepEqual(cmd.cell, { width: 300, height: 200 });
    assert.equal(cmd.cols, 4);
  });
  void it('rejects unknown verbs and flags', () => {
    assert.throws(() => parseGraphosArgs(['render', 'x']));
    assert.throws(() => parseGraphosArgs(['list', '--bogus']));
  });
});
