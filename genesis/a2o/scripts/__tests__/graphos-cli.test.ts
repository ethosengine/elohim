import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { containedSlug, groupRows, parseGraphosArgs } from '../graphos.js';
import { type StoryEntry } from '../lib/graphos-stories.js';

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
  void it('throws when --out has no value', () => {
    assert.throws(() => parseGraphosArgs(['story', 'x', '--out']));
  });
  void it('throws when --cols has no value', () => {
    assert.throws(() => parseGraphosArgs(['sheet', 'x', '--cols']));
  });
});

void describe('containedSlug', () => {
  void it('accepts plain slugs', () => {
    assert.equal(containedSlug('sheet-elohim-compute-tile'), 'sheet-elohim-compute-tile');
  });
  void it('rejects traversal and absolute slugs', () => {
    assert.throws(() => containedSlug('../../etc'));
    assert.throws(() => containedSlug('/tmp/exfil'));
  });
});

void describe('groupRows', () => {
  const e = (id: string, title: string): StoryEntry => ({ id, title, name: 'X', type: 'story' });
  void it('yields ceil(rows) per family', () => {
    const entries = [
      e('default-a--1', 'Default/A'), e('default-a--2', 'Default/A'), e('default-a--3', 'Default/A'),
      e('default-a--4', 'Default/A'),
      e('designed-a--1', 'Designed/A'),
    ];
    assert.deepEqual([...groupRows(entries, { cols: 3 })], [2, 1]);
  });
  void it('handles a single family with even division', () => {
    const entries = [e('x--1', 'Default/X'), e('x--2', 'Default/X'), e('x--3', 'Default/X')];
    assert.deepEqual([...groupRows(entries, { cols: 3 })], [1]);
  });
});
