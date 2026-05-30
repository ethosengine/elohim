import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { parseArgs } from '../look.js';

describe('parseArgs', () => {
  it('parses a bare url', () => {
    const o = parseArgs(['https://example.test/path']);
    assert.equal(o.url, 'https://example.test/path');
    assert.equal(o.as, undefined);
  });

  it('parses all flags', () => {
    const o = parseArgs([
      'https://example.test',
      '--as',
      'Matthew',
      '--doorway',
      'https://doorway.test',
      '--wait-testid',
      'app-root',
      '--out',
      'my-slug',
      '--viewport',
      '800x600',
    ]);
    assert.equal(o.as, 'Matthew');
    assert.equal(o.doorway, 'https://doorway.test');
    assert.equal(o.waitTestid, 'app-root');
    assert.equal(o.out, 'my-slug');
    assert.deepEqual(o.viewport, { width: 800, height: 600 });
  });

  it('throws when url is missing', () => {
    assert.throws(() => parseArgs([]), /Usage: look/);
    assert.throws(() => parseArgs(['--as', 'Matthew']), /Usage: look/);
  });

  it('throws on a bad --viewport', () => {
    assert.throws(() => parseArgs(['https://x.test', '--viewport', 'huge']), /--viewport expects WxH/);
  });

  it('throws on an unknown flag', () => {
    assert.throws(() => parseArgs(['https://x.test', '--nope', 'v']), /Unknown flag: --nope/);
  });
});
