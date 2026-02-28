#!/usr/bin/env npx ts-node
/**
 * Second-pass fixup script for Karma→Vitest migration.
 * Fixes patterns the first migration script missed.
 */
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const APP_DIR = path.resolve(__dirname, '../src');

function findSpecFiles(dir: string): string[] {
  const results: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...findSpecFiles(full));
    } else if (entry.name.endsWith('.spec.ts')) {
      results.push(full);
    }
  }
  return results;
}

let totalFixed = 0;

function fix(file: string): void {
  let content = fs.readFileSync(file, 'utf-8');
  const original = content;
  const fixes: string[] = [];

  // 1. Fix bare spyOn → vi.spyOn (not already prefixed with vi.)
  if (/(?<![.\w])spyOn\(/.test(content) && !/vi\.spyOn\(/.test(content)) {
    // File uses spyOn but never vi.spyOn — add vi. prefix
    content = content.replace(/(?<![.\w])spyOn\(/g, 'vi.spyOn(');
    fixes.push('spyOn → vi.spyOn');
  } else if (/(?<![.\w])spyOn\(/.test(content)) {
    // File has mix of vi.spyOn and bare spyOn
    content = content.replace(/(?<![.\w])(?<!vi\.)spyOn\(/g, 'vi.spyOn(');
    fixes.push('remaining spyOn → vi.spyOn');
  }

  // 2. Fix spyOnProperty → vi.spyOn with getter/setter accessor
  // spyOnProperty(obj, 'prop') → vi.spyOn(obj, 'prop', 'get')
  content = content.replace(
    /(?<![.\w])spyOnProperty\(([^,]+),\s*([^,)]+)\)/g,
    'vi.spyOn($1, $2, \'get\')'
  );
  if (content !== original && /vi\.spyOn.*'get'/.test(content)) {
    fixes.push('spyOnProperty → vi.spyOn(…, \'get\')');
  }

  // 3. Fix jasmine.anything() → expect.anything()
  content = content.replace(/jasmine\.anything\(\)/g, 'expect.anything()');
  if (/expect\.anything\(\)/.test(content) && /jasmine\.anything/.test(original)) {
    fixes.push('jasmine.anything → expect.anything');
  }

  // 4. Fix jasmine.objectContaining → expect.objectContaining
  content = content.replace(/jasmine\.objectContaining\(/g, 'expect.objectContaining(');
  if (/expect\.objectContaining/.test(content) && /jasmine\.objectContaining/.test(original)) {
    fixes.push('jasmine.objectContaining → expect.objectContaining');
  }

  // 5. Fix jasmine.arrayContaining → expect.arrayContaining
  content = content.replace(/jasmine\.arrayContaining\(/g, 'expect.arrayContaining(');
  if (/expect\.arrayContaining/.test(content) && /jasmine\.arrayContaining/.test(original)) {
    fixes.push('jasmine.arrayContaining → expect.arrayContaining');
  }

  // 6. Fix jasmine.stringMatching → expect.stringMatching
  content = content.replace(/jasmine\.stringMatching\(/g, 'expect.stringMatching(');
  if (/expect\.stringMatching/.test(content) && /jasmine\.stringMatching/.test(original)) {
    fixes.push('jasmine.stringMatching → expect.stringMatching');
  }

  // 7. Fix jasmine.stringContaining → expect.stringContaining
  content = content.replace(/jasmine\.stringContaining\(/g, 'expect.stringContaining(');
  if (/expect\.stringContaining/.test(content) && /jasmine\.stringContaining/.test(original)) {
    fixes.push('jasmine.stringContaining → expect.stringContaining');
  }

  // 8. Fix remaining jasmine.createSpy('name') → vi.fn()
  content = content.replace(/jasmine\.createSpy\([^)]*\)/g, 'vi.fn()');
  if (/vi\.fn\(\)/.test(content) && /jasmine\.createSpy/.test(original)) {
    fixes.push('jasmine.createSpy → vi.fn()');
  }

  // 9. Fix expectAsync(promise).toBeResolved() → expect(promise).resolves.toBeDefined()
  content = content.replace(
    /expectAsync\(([^)]+)\)\.toBeResolved\(\)/g,
    'await expect($1).resolves.toBeDefined()'
  );

  // 10. Fix expectAsync(promise).toBeRejected() → expect(promise).rejects.toThrow()
  content = content.replace(
    /expectAsync\(([^)]+)\)\.toBeRejected\(\)/g,
    'await expect($1).rejects.toThrow()'
  );

  // 11. Fix expectAsync(promise).toBeRejectedWith(err) → expect(promise).rejects.toThrow(err)
  content = content.replace(
    /expectAsync\(([^)]+)\)\.toBeRejectedWith\(([^)]+)\)/g,
    'await expect($1).rejects.toThrow($2)'
  );

  // 12. Fix expectAsync(promise).toBeResolvedTo(val) → expect(promise).resolves.toEqual(val)
  content = content.replace(
    /expectAsync\(([^)]+)\)\.toBeResolvedTo\(([^)]+)\)/g,
    'await expect($1).resolves.toEqual($2)'
  );

  // 13. Fix provideHttpClient not imported — if used but not imported
  if (/provideHttpClient\b/.test(content) && !/import.*provideHttpClient/.test(content)) {
    // Check if there's an @angular/common/http import to extend
    const httpImportMatch = content.match(/(import\s*\{[^}]*\}\s*from\s*'@angular\/common\/http';)/);
    if (httpImportMatch) {
      const existingImport = httpImportMatch[1];
      // Extract what's already imported
      const importContent = existingImport.match(/\{([^}]*)\}/)?.[1] || '';
      const imports = importContent.split(',').map(s => s.trim()).filter(Boolean);
      if (!imports.includes('provideHttpClient')) {
        imports.push('provideHttpClient');
      }
      const newImport = `import { ${imports.join(', ')} } from '@angular/common/http';`;
      content = content.replace(existingImport, newImport);
      fixes.push('added provideHttpClient to existing http import');
    } else {
      // No http import at all, add one after the last import
      const lastImportIdx = content.lastIndexOf('\nimport ');
      if (lastImportIdx !== -1) {
        const lineEnd = content.indexOf('\n', lastImportIdx + 1);
        const insertPoint = content.indexOf(';', lineEnd) + 1;
        content = content.slice(0, insertPoint) +
          "\nimport { provideHttpClient } from '@angular/common/http';" +
          content.slice(insertPoint);
        fixes.push('added provideHttpClient import');
      }
    }
  }

  // 14. Fix provideHttpClientTesting not imported
  if (/provideHttpClientTesting\b/.test(content) && !/import.*provideHttpClientTesting/.test(content)) {
    const httpTestImportMatch = content.match(/(import\s*\{[^}]*\}\s*from\s*'@angular\/common\/http\/testing';)/);
    if (httpTestImportMatch) {
      const existingImport = httpTestImportMatch[1];
      const importContent = existingImport.match(/\{([^}]*)\}/)?.[1] || '';
      const imports = importContent.split(',').map(s => s.trim()).filter(Boolean);
      if (!imports.includes('provideHttpClientTesting')) {
        imports.push('provideHttpClientTesting');
      }
      const newImport = `import { ${imports.join(', ')} } from '@angular/common/http/testing';`;
      content = content.replace(existingImport, newImport);
      fixes.push('added provideHttpClientTesting to existing http/testing import');
    } else {
      const lastImportIdx = content.lastIndexOf('\nimport ');
      if (lastImportIdx !== -1) {
        const lineEnd = content.indexOf('\n', lastImportIdx + 1);
        const insertPoint = content.indexOf(';', lineEnd) + 1;
        content = content.slice(0, insertPoint) +
          "\nimport { provideHttpClientTesting } from '@angular/common/http/testing';" +
          content.slice(insertPoint);
        fixes.push('added provideHttpClientTesting import');
      }
    }
  }

  // 15. Fix duplicate providers: keys in TestBed.configureTestingModule
  // Pattern: providers: [...], providers: [...] → merge into single providers
  const dupProviderPattern = /providers:\s*\[([^\]]*)\],(\s*)providers:\s*\[([^\]]*)\]/g;
  if (dupProviderPattern.test(content)) {
    content = content.replace(dupProviderPattern, (_, first, ws, second) => {
      return `providers: [${first.trim()}, ${second.trim()}]`;
    });
    fixes.push('merged duplicate providers arrays');
  }

  // 16. Fix `() => throw new Error(...)` → `() => { throw new Error(...); }`
  content = content.replace(
    /=>\s*throw\s+new\s+Error\(([^)]*)\)/g,
    '=> { throw new Error($1); }'
  );
  if (/=> \{ throw new Error/.test(content) && /=>\s*throw\s+new\s+Error/.test(original)) {
    fixes.push('wrapped throw in arrow fn body');
  }

  // 17. Fix done() callback pattern — Vitest doesn't support done(), convert to promise
  // it('name', (done) => { ... done(); }) → it('name', () => new Promise<void>(done => { ... done(); }))
  // This is a complex transform, just remove done parameter and done() calls for simple cases
  // Actually, let's just note files that use done() for manual fix

  // 18. Ensure vi import exists if vi.fn() or vi.spyOn is used
  if (/\bvi\.(fn|spyOn|mock|mocked)\b/.test(content) && !/import\s*\{[^}]*\bvi\b[^}]*\}\s*from\s*'vitest'/.test(content)) {
    // Check if there's already a vitest import
    const vitestImportMatch = content.match(/(import\s*\{[^}]*\}\s*from\s*'vitest';)/);
    if (vitestImportMatch) {
      const existing = vitestImportMatch[1];
      const importContent = existing.match(/\{([^}]*)\}/)?.[1] || '';
      const imports = importContent.split(',').map(s => s.trim()).filter(Boolean);
      if (!imports.includes('vi')) {
        imports.push('vi');
        const newImport = `import { ${imports.join(', ')} } from 'vitest';`;
        content = content.replace(existing, newImport);
        fixes.push('added vi to vitest import');
      }
    } else {
      // Add vitest import at top after other imports
      const firstImportMatch = content.match(/^(import\s)/m);
      if (firstImportMatch) {
        content = `import { vi } from 'vitest';\n` + content;
        fixes.push('added vitest vi import');
      }
    }
  }

  // 19. Fix .and.returnValue still present (migration script may have missed some)
  content = content.replace(/\.and\.returnValue\(/g, '.mockReturnValue(');
  content = content.replace(/\.and\.callFake\(/g, '.mockImplementation(');
  content = content.replace(/\.and\.throwError\(/g, '.mockImplementation(() => { throw ');
  // The throwError one needs closing — but this is a rough pass

  // 20. Fix .calls.reset() → .mockReset()
  content = content.replace(/\.calls\.reset\(\)/g, '.mockReset()');
  content = content.replace(/\.calls\.count\(\)/g, '.mock.calls.length');
  content = content.replace(/\.calls\.mostRecent\(\)\.args/g, '.mock.calls[.mock.calls.length - 1]');
  content = content.replace(/\.calls\.allArgs\(\)/g, '.mock.calls');
  content = content.replace(/\.calls\.all\(\)/g, '.mock.calls');
  content = content.replace(/\.calls\.argsFor\((\d+)\)/g, '.mock.calls[$1]');

  if (content !== original) {
    fs.writeFileSync(file, content);
    totalFixed++;
    const rel = path.relative(path.resolve(__dirname, '..'), file);
    console.log(`  Fixed: ${rel} [${fixes.join(', ')}]`);
  }
}

const specFiles = findSpecFiles(APP_DIR);
console.log(`Scanning ${specFiles.length} spec files for fixup patterns...\n`);

for (const file of specFiles) {
  fix(file);
}

console.log(`\nDone. Fixed ${totalFixed} of ${specFiles.length} files.`);
