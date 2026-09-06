import { describe, expect, it } from 'vitest';
import { selectSeedCell, seedCellTarget } from './cell-target.js';

const base = [new Uint8Array([1]), new Uint8Array([2])];
const clone = [new Uint8Array([3]), new Uint8Array([2])];
for (const format of ['tagged', 'keyed']) {
  const wrap = (kind: string, value: object) =>
    format === 'tagged' ? { type: kind, value } : { [kind]: value };
  const cloned = (enabled = true) =>
    wrap('cloned', {
      cell_id: clone,
      clone_id: 'lamad.0',
      name: 'fixtures',
      enabled,
    });
  const provisioned = wrap('provisioned', { cell_id: base });
  describe(format, () => {
    it('preserves the default and selects clone by name or id regardless of order', () => {
      const info = { lamad: [provisioned, cloned()] };
      expect(selectSeedCell(info)).toBe(base);
      expect(selectSeedCell(info, 'lamad')).toBe(base);
      expect(selectSeedCell(info, 'lamad.fixtures')).toBe(clone);
      expect(selectSeedCell(info, 'lamad.0')).toBe(clone);
    });
    it('refuses missing, disabled, wrong-role and ambiguous clones', () => {
      for (const cells of [
        [provisioned],
        [provisioned, cloned(false)],
        [provisioned, cloned(), cloned()],
      ]) {
        expect(() => selectSeedCell({ lamad: cells }, 'lamad.fixtures')).toThrow('no fallback');
      }
      expect(() => selectSeedCell({ imagodei: [cloned()] }, 'lamad.fixtures')).toThrow(
        'no fallback',
      );
    });
  });
}
it('rejects malformed selectors and missing flag values; CLI overrides env', () => {
  for (const target of ['', '.fixtures', 'lamad.', 'lamad.a.b', 'lamad. fixtures']) {
    expect(() => selectSeedCell({}, target)).toThrow('Invalid cell target');
  }
  expect(() => seedCellTarget(['--cell'], 'lamad.fixtures')).toThrow();
  expect(seedCellTarget(['--cell', 'lamad.0'], 'lamad.fixtures')).toBe('lamad.0');
  expect(seedCellTarget([], 'lamad.fixtures')).toBe('lamad.fixtures');
});
