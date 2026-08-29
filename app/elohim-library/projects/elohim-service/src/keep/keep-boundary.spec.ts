/*
 * Boundary test for `src/keep/` — the framework-free half of elohim-service.
 *
 * The rule (elohim-sdk.md §4.1, and the same one `@elohim/identity/core`
 * already enforces at `elohim-identity/src/core.boundary.spec.ts`): a
 * framework-free surface has ZERO `@angular/*` anywhere in its transitive
 * import closure. Keep is meant to be usable from a Tauri shell, a Node
 * script, an a2o step and a browser SPA alike, so an Angular import anywhere
 * under `keep/` silently narrows it to one of those.
 *
 * It matters here more than usual because the thing this directory replaces —
 * doorway-app's `DoorwayFederationService` — is an `@Injectable` that reaches
 * for `HttpClient`. Reproducing that shape would also make the register
 * re-enter the interceptor it exists to answer for, since the interceptor
 * rewrites exactly the `/api/` path the register loads from.
 *
 * If this fails after you touched keep/, you imported Angular (or a module that
 * transitively does). The Angular-facing wiring belongs in `src/angular/`.
 */
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = dirname(fileURLToPath(import.meta.url));

/** Extract every static module specifier from a TS source text. */
function moduleSpecifiers(source: string): string[] {
  const specifiers: string[] = [];
  const fromRe = /(?:^|\n)\s*(?:import|export)\s[^;]*?from\s*['"]([^'"]+)['"]/g;
  const bareRe = /(?:^|\n)\s*import\s*['"]([^'"]+)['"]/g;
  const dynamicRe = /import\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
  for (const re of [fromRe, bareRe, dynamicRe]) {
    for (let m = re.exec(source); m !== null; m = re.exec(source)) {
      specifiers.push(m[1]);
    }
  }
  return specifiers;
}

function resolveRelative(fromFile: string, specifier: string): string {
  const base = resolve(dirname(fromFile), specifier);
  const candidates = [base, `${base}.ts`, base.replace(/\.js$/, '.ts'), resolve(base, 'index.ts')];
  const hit = candidates.find(c => c.endsWith('.ts') && existsSync(c));
  if (!hit) {
    throw new Error(`keep boundary walk: cannot resolve '${specifier}' from ${fromFile}`);
  }
  return hit;
}

interface ClosureResult {
  visited: Set<string>;
  bareImports: Map<string, string[]>;
}

function walkClosure(entryFile: string): ClosureResult {
  const visited = new Set<string>();
  const bareImports = new Map<string, string[]>();
  const queue = [resolve(entryFile)];
  while (queue.length > 0) {
    const file = queue.pop() as string;
    if (visited.has(file)) continue;
    visited.add(file);
    for (const spec of moduleSpecifiers(readFileSync(file, 'utf8'))) {
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

describe('keep/ framework boundary', () => {
  const keepEntry = resolve(HERE, 'index.ts');

  it('walks a non-trivial closure (walker sanity)', () => {
    const closure = walkClosure(keepEntry);
    // The register plus the resolver seam it implements, at minimum — so a
    // green result cannot come from a walk that visited nothing.
    expect([...closure.visited].some(f => f.endsWith('/keep/peer-register.ts'))).toBe(true);
    expect([...closure.visited].some(f => f.endsWith('/client/doorway-address-resolver.ts'))).toBe(
      true
    );
    expect(closure.visited.size).toBeGreaterThanOrEqual(3);
  });

  it('has ZERO @angular/* imports in its transitive closure', () => {
    expect(angularHits(walkClosure(keepEntry))).toEqual([]);
  });

  it('detector control: an Angular-importing entry DOES trip the detector', () => {
    // Without this, a walker that silently resolved nothing would report the
    // same empty list as a genuinely clean closure.
    const angularEntry = resolve(HERE, '..', 'client', 'angular-provider.ts');
    expect(existsSync(angularEntry)).toBe(true);
    expect(angularHits(walkClosure(angularEntry)).length).toBeGreaterThan(0);
  });

  it('every .ts under keep/ is reachable from the barrel', () => {
    // A file nobody re-exports is a file the boundary check never walks — which
    // is exactly how an Angular import would slip past a green suite.
    const closure = walkClosure(keepEntry);
    const onDisk = readdirSync(HERE)
      .filter(f => f.endsWith('.ts') && !f.endsWith('.spec.ts') && f !== 'index.ts')
      .map(f => join(HERE, f));
    const unreachable = onDisk.filter(f => !closure.visited.has(f));
    expect(unreachable).toEqual([]);
  });
});
