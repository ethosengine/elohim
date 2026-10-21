// Rail R-T1: @elohim/eprfs stays pure. No source file imports @elohim/storage-client, a DOM or
// Angular module, or names a browser global; tsconfig's lib carries no DOM.
//
// Imports are read with the TypeScript compiler's own scanner (`ts.preProcessFile`), so every
// form counts: `from '…'` and `from "…"`, a bare `import '…'`, `export … from`, and a dynamic
// `import(…)`. Globals are matched as identifiers, including through `globalThis.`/`self.`.
import { readdirSync, readFileSync } from 'node:fs';

import ts from 'typescript';
import { describe, expect, it } from 'vitest';

const src = new URL('../src/', import.meta.url);

const IMPURE_IMPORT = (spec: string) =>
  spec.startsWith('@elohim/storage-client') ||
  spec.startsWith('@angular/') ||
  spec === 'rxjs' ||
  spec.startsWith('rxjs/') ||
  spec.startsWith('node:');

const GLOBALS = ['window', 'document', 'navigator', 'caches', 'localStorage', 'sessionStorage', 'indexedDB', 'fetch', 'XMLHttpRequest', 'WebSocket'];

function importsOf(text: string): string[] {
  const info = ts.preProcessFile(text, true, true);
  return info.importedFiles.map((f) => f.fileName);
}

function namedGlobals(text: string): string[] {
  const found: string[] = [];
  for (const g of GLOBALS) {
    // A use of the global, bare or through globalThis/self, but not a property of something else
    // (`budget.fetch`) or part of a longer name (`prefetch`, `fetchBlobs`).
    const use = new RegExp(`(?:^|[^.\\w$])(?:(?:globalThis|self)\\s*\\.\\s*)?${g}\\b(?!\\s*:)`, 'm');
    const bracket = new RegExp(`(?:globalThis|self)\\s*\\[\\s*['"\`]${g}['"\`]\\s*\\]`);
    if (use.test(text) || bracket.test(text)) found.push(g);
  }
  return found;
}

describe('rail R-T1', () => {
  const files = readdirSync(src).filter((f) => f.endsWith('.ts'));

  it('scans every source file', () => {
    expect(files.length).toBeGreaterThanOrEqual(6);
  });

  it('reads every import form, so none slips past', () => {
    const sample = [
      `import a from "@angular/core";`,
      `import '@elohim/storage-client';`,
      `export { b } from 'rxjs/operators';`,
      `const c = await import('node:fs');`,
    ].join('\n');
    expect(importsOf(sample).filter(IMPURE_IMPORT)).toHaveLength(4);
    expect(namedGlobals('const r = globalThis.fetch(url);')).toEqual(['fetch']);
    expect(namedGlobals("const r = self['fetch'](url);")).toEqual(['fetch']);
    expect(namedGlobals('await tier.fetchBlobs(root); budget.fetch(1); prefetch();')).toEqual([]);
  });

  it('imports nothing impure and names no browser global', () => {
    for (const file of files) {
      const text = readFileSync(new URL(file, src), 'utf8');
      for (const spec of importsOf(text)) {
        expect(IMPURE_IMPORT(spec), `${file} imports ${spec}`).toBe(false);
      }
      // Comments may name a global in prose; the scan reads code only.
      const code = text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:])\/\/.*$/gm, '$1');
      expect(namedGlobals(code), `${file} names a browser global`).toEqual([]);
    }
  });

  it('compiles without the DOM library', () => {
    const tsconfig = JSON.parse(readFileSync(new URL('../tsconfig.json', import.meta.url), 'utf8'));
    expect(tsconfig.compilerOptions.lib).not.toContain('DOM');
  });
});
