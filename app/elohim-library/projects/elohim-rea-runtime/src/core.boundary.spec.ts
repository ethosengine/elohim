/*
 * Boundary test for the framework-free `/core` entrypoint.
 *
 * Canon (elohim-sdk.md §4.1): the `./core` subpath export is framework-free
 * ONLY — zero `@angular/*` anywhere in its transitive import closure. This
 * spec enforces that rule mechanically: it walks core.ts's transitive import
 * closure on disk (node:fs only — no resolver dependencies) and fails on any
 * `@angular/*` module specifier.
 *
 * If this test fails after you touched core.ts, you re-exported a module that
 * (transitively) imports Angular. Such modules belong on the root entry
 * (public-api.ts), not on /core. This gate is what Phase 4's CommitmentService
 * lands behind.
 */
import { readFileSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = dirname(fileURLToPath(import.meta.url));

/** Extract every static module specifier from a TS source text. */
function moduleSpecifiers(source: string): string[] {
  const specifiers: string[] = [];
  // `import ... from '...'` / `export ... from '...'` (incl. multi-line clauses)
  const fromRe = /(?:^|\n)\s*(?:import|export)\s[^;]*?from\s*['"]([^'"]+)['"]/g;
  // bare side-effect imports: `import '...';`
  const bareRe = /(?:^|\n)\s*import\s*['"]([^'"]+)['"]/g;
  // dynamic imports: `import('...')`
  const dynamicRe = /import\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
  for (const re of [fromRe, bareRe, dynamicRe]) {
    for (let m = re.exec(source); m !== null; m = re.exec(source)) {
      specifiers.push(m[1]);
    }
  }
  return specifiers;
}

/** Resolve a relative specifier to a .ts file on disk (ESM `.js` → `.ts` aware). */
function resolveRelative(fromFile: string, specifier: string): string {
  const base = resolve(dirname(fromFile), specifier);
  const candidates = [
    base,
    `${base}.ts`,
    `${base}.tsx`,
    base.replace(/\.js$/, '.ts'),
    resolve(base, 'index.ts'),
  ];
  const hit = candidates.find(c => c.endsWith('.ts') && existsSync(c));
  if (!hit) {
    throw new Error(
      `core boundary walk: cannot resolve relative import '${specifier}' from ${fromFile}`
    );
  }
  return hit;
}

interface ClosureResult {
  /** Absolute paths of every .ts file reachable from the entry. */
  visited: Set<string>;
  /** Bare (non-relative) specifiers found, mapped to the files importing them. */
  bareImports: Map<string, string[]>;
}

/** Walk the transitive static-import closure of a TS entry file. */
function walkClosure(entryFile: string): ClosureResult {
  const visited = new Set<string>();
  const bareImports = new Map<string, string[]>();
  const queue = [resolve(entryFile)];
  while (queue.length > 0) {
    const file = queue.pop() as string;
    if (visited.has(file)) {
      continue;
    }
    visited.add(file);
    const source = readFileSync(file, 'utf8');
    for (const spec of moduleSpecifiers(source)) {
      if (spec.startsWith('.')) {
        queue.push(resolveRelative(file, spec));
      } else {
        const importers = bareImports.get(spec) ?? [];
        importers.push(file);
        bareImports.set(spec, importers);
      }
    }
  }
  return { visited, bareImports };
}

const angularHits = (closure: ClosureResult): string[] =>
  [...closure.bareImports.entries()]
    .filter(([spec]) => spec === '@angular' || spec.startsWith('@angular/'))
    .map(([spec, importers]) => `${spec} (imported by: ${importers.join(', ')})`);

describe('@elohim/rea-runtime/core boundary (elohim-sdk.md §4.1)', () => {
  const coreEntry = resolve(HERE, 'core.ts');

  it('walks a non-trivial closure (walker sanity)', () => {
    const closure = walkClosure(coreEntry);
    const visitedNames = [...closure.visited];
    // core.ts itself plus every re-exported framework-free module.
    expect(visitedNames.some(f => f.endsWith('/core.ts'))).toBe(true);
    expect(visitedNames.some(f => f.endsWith('/lib/rea-action-types.ts'))).toBe(true);
    expect(visitedNames.some(f => f.endsWith('/lib/protocol-event-types.ts'))).toBe(true);
    expect(visitedNames.some(f => f.endsWith('/lib/lamad-event-intent.types.ts'))).toBe(true);
    expect(visitedNames.some(f => f.endsWith('/lib/signal-emit-types.ts'))).toBe(true);
    expect(closure.visited.size).toBeGreaterThanOrEqual(5);
  });

  it('has ZERO @angular/* imports in its transitive closure', () => {
    const closure = walkClosure(coreEntry);
    expect(angularHits(closure)).toEqual([]);
  });

  it('detector control: the Angular root entry (public-api.ts) DOES trip the detector', () => {
    // Proves the boundary check can actually detect Angular imports: the root
    // entry re-exports event.service (and four other services + the shefa DI
    // tokens), all of which import @angular/core.
    const closure = walkClosure(resolve(HERE, 'public-api.ts'));
    expect(angularHits(closure).length).toBeGreaterThan(0);
  });
});
