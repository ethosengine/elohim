#!/usr/bin/env npx tsx
/**
 * Migration script: Jasmine/Karma → Vitest
 *
 * Transforms Jasmine-specific patterns in .spec.ts files to Vitest equivalents.
 * Run: npx tsx scripts/migrate-to-vitest.ts [--dry-run]
 */
import * as fs from 'fs';
import * as path from 'path';

const DRY_RUN = process.argv.includes('--dry-run');
const VERBOSE = process.argv.includes('--verbose');

let filesProcessed = 0;
let filesModified = 0;
const errors: string[] = [];

function findSpecFiles(dir: string): string[] {
  const results: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory() && entry.name !== 'node_modules' && entry.name !== 'dist') {
      results.push(...findSpecFiles(fullPath));
    } else if (entry.isFile() && entry.name.endsWith('.spec.ts') && !entry.name.endsWith('.vitest.spec.ts')) {
      results.push(fullPath);
    }
  }
  return results;
}

function migrateFile(filePath: string): boolean {
  const original = fs.readFileSync(filePath, 'utf-8');
  let content = original;

  // ── 1. Replace jasmine.createSpyObj ──────────────────────────────────
  // Pattern: jasmine.createSpyObj<Type>('Name', ['m1', 'm2', ...])
  // Also:    jasmine.createSpyObj('Name', ['m1', 'm2', ...])
  content = content.replace(
    /jasmine\.createSpyObj(?:<[^>]+>)?\(\s*'[^']*'\s*,\s*\[([\s\S]*?)\]\s*\)/g,
    (_match, methodsStr: string) => {
      const methods = methodsStr
        .split(',')
        .map(m => m.trim().replace(/^['"]|['"]$/g, ''))
        .filter(m => m.length > 0);
      if (methods.length === 0) return '{}';
      return '{\n    ' + methods.map(m => `${m}: vi.fn()`).join(',\n    ') + ',\n  }';
    }
  );

  // ── 2. Replace jasmine.createSpy('name') ─────────────────────────────
  content = content.replace(/jasmine\.createSpy\([^)]*\)/g, 'vi.fn()');

  // ── 3. Replace jasmine.SpyObj<T> type annotations ────────────────────
  // let/const foo: jasmine.SpyObj<SomeType> → let/const foo: any
  content = content.replace(/:\s*jasmine\.SpyObj<[^>]+>/g, ': any');
  // jasmine.Spy type → Mock
  content = content.replace(/:\s*jasmine\.Spy\b/g, ': Mock');

  // ── 4. Replace .and.returnValue( → .mockReturnValue( ─────────────────
  content = content.replace(/\.and\.returnValue\(/g, '.mockReturnValue(');

  // ── 5. Replace .and.callFake( → .mockImplementation( ──────────────────
  content = content.replace(/\.and\.callFake\(/g, '.mockImplementation(');

  // ── 6. Replace .and.throwError( → .mockImplementation(() => { throw  ──
  // This one is tricky: .and.throwError(new Error('msg'))
  // → .mockImplementation(() => { throw new Error('msg'); })
  content = content.replace(
    /\.and\.throwError\(([^)]+)\)/g,
    '.mockImplementation(() => { throw $1; })'
  );

  // ── 7. Replace .and.resolveTo( → .mockResolvedValue( ─────────────────
  content = content.replace(/\.and\.resolveTo\(/g, '.mockResolvedValue(');

  // ── 8. Replace .and.rejectWith( → .mockRejectedValue( ────────────────
  content = content.replace(/\.and\.rejectWith\(/g, '.mockRejectedValue(');

  // ── 9. Replace .calls.reset() → .mockClear() ─────────────────────────
  content = content.replace(/\.calls\.reset\(\)/g, '.mockClear()');

  // ── 10. Replace .calls.count() → .mock.calls.length ───────────────────
  content = content.replace(/\.calls\.count\(\)/g, '.mock.calls.length');

  // ── 11. Replace .calls.mostRecent() → .mock.lastCall ──────────────────
  // .calls.mostRecent().args → .mock.lastCall
  content = content.replace(/\.calls\.mostRecent\(\)\.args/g, '.mock.lastCall');
  content = content.replace(/\.calls\.mostRecent\(\)/g, '.mock.lastCall');

  // ── 12. Replace .calls.allArgs() → .mock.calls ────────────────────────
  content = content.replace(/\.calls\.allArgs\(\)/g, '.mock.calls');

  // ── 13. Replace .calls.argsFor(n) → .mock.calls[n] ────────────────────
  content = content.replace(/\.calls\.argsFor\((\d+)\)/g, '.mock.calls[$1]');

  // ── 14. Replace .calls.all() patterns ──────────────────────────────────
  // .calls.all().length → .mock.calls.length
  content = content.replace(/\.calls\.all\(\)\.length/g, '.mock.calls.length');
  // .calls.all()[n].args → .mock.calls[n]
  content = content.replace(/\.calls\.all\(\)\[(\d+)\]\.args/g, '.mock.calls[$1]');
  content = content.replace(/\.calls\.all\(\)/g, '.mock.calls.map(args => ({ args }))');

  // ── 15. Replace .toBeTrue() → .toBe(true) ─────────────────────────────
  content = content.replace(/\.toBeTrue\(\)/g, '.toBe(true)');
  content = content.replace(/\.toBeFalse\(\)/g, '.toBe(false)');

  // ── 16. Replace standalone fail() ──────────────────────────────────────
  // fail('message') → throw new Error('message')
  // Be careful not to match done.fail or expect().fail
  content = content.replace(/(?<![.\w])fail\((['"`])(.*?)\1\)/g, "throw new Error($1$2$1)");
  // fail() with no args
  content = content.replace(/(?<![.\w])fail\(\)/g, "throw new Error('Test should not reach this point')");

  // ── 17. Replace HttpClientTestingModule import + usage ─────────────────
  if (content.includes('HttpClientTestingModule')) {
    // Replace the import
    content = content.replace(
      /import\s*\{([^}]*)\bHttpClientTestingModule\b([^}]*)\}\s*from\s*'@angular\/common\/http\/testing'/g,
      (_match, before: string, after: string) => {
        // Keep other imports from the testing module (like HttpTestingController)
        const otherImports = (before + after)
          .split(',')
          .map(s => s.trim())
          .filter(s => s.length > 0 && s !== 'HttpClientTestingModule');

        const testingImports = otherImports.length > 0
          ? `import { provideHttpClientTesting, ${otherImports.join(', ')} } from '@angular/common/http/testing'`
          : `import { provideHttpClientTesting } from '@angular/common/http/testing'`;

        return testingImports;
      }
    );

    // Add provideHttpClient import if not present
    if (!content.includes('provideHttpClient')) {
      // Find the HttpClient testing import line and add provideHttpClient import after it
      content = content.replace(
        /(import\s*\{[^}]*provideHttpClientTesting[^}]*\}\s*from\s*'@angular\/common\/http\/testing')/,
        "import { provideHttpClient } from '@angular/common/http';\n$1"
      );
    }

    // Replace usage in TestBed: imports: [HttpClientTestingModule] → providers: [provideHttpClient(), provideHttpClientTesting()]
    // Case 1: HttpClientTestingModule is the only import
    content = content.replace(
      /imports:\s*\[\s*HttpClientTestingModule\s*\]/g,
      'providers: [provideHttpClient(), provideHttpClientTesting()]'
    );
    // Case 2: HttpClientTestingModule is among other imports - remove it and add providers
    content = content.replace(
      /HttpClientTestingModule,?\s*/g,
      ''
    );

    // If we just moved it to providers, make sure we don't have duplicate providers arrays
    // This is a best-effort transform; complex cases will need manual fixup
  }

  // ── 18. Add vi import if needed ────────────────────────────────────────
  const needsVi = content.includes('vi.fn()') || content.includes('vi.spyOn');
  const hasViImport = /import\s*\{[^}]*\bvi\b[^}]*\}\s*from\s*'vitest'/.test(content);
  const hasMockType = content.includes(': Mock');

  if (needsVi && !hasViImport) {
    if (hasMockType) {
      content = addImport(content, "import { type Mock, vi } from 'vitest';");
    } else {
      content = addImport(content, "import { vi } from 'vitest';");
    }
  } else if (hasMockType && !hasViImport) {
    content = addImport(content, "import { type Mock } from 'vitest';");
  }

  // ── 19. Clean up empty imports arrays ──────────────────────────────────
  content = content.replace(/imports:\s*\[\s*\],?\s*\n?/g, '');

  // ── 20. Remove @types/jasmine reference if present ─────────────────────
  content = content.replace(/\/\/\/\s*<reference\s+types="jasmine"\s*\/>\s*\n?/g, '');

  // Check if anything changed
  if (content === original) return false;

  if (!DRY_RUN) {
    fs.writeFileSync(filePath, content, 'utf-8');
  }
  return true;
}

function addImport(content: string, importLine: string): string {
  // Add after the last import statement
  const lines = content.split('\n');
  let lastImportIndex = -1;
  for (let i = 0; i < lines.length; i++) {
    if (/^\s*import\s/.test(lines[i])) {
      // Find the end of this import (may span multiple lines)
      let j = i;
      while (j < lines.length && !lines[j].includes(';')) j++;
      lastImportIndex = j;
    }
  }

  if (lastImportIndex >= 0) {
    lines.splice(lastImportIndex + 1, 0, importLine);
  } else {
    // No imports found, add at top
    lines.unshift(importLine);
  }
  return lines.join('\n');
}

// ── Main ──────────────────────────────────────────────────────────────────
const srcDir = path.resolve(__dirname, '../src');
const specFiles = findSpecFiles(srcDir);

console.log(`Found ${specFiles.length} spec files`);
if (DRY_RUN) console.log('DRY RUN — no files will be modified\n');

for (const file of specFiles) {
  filesProcessed++;
  try {
    const modified = migrateFile(file);
    if (modified) {
      filesModified++;
      if (VERBOSE || DRY_RUN) {
        console.log(`  ✓ ${path.relative(srcDir, file)}`);
      }
    }
  } catch (err) {
    const msg = `  ✗ ${path.relative(srcDir, file)}: ${err}`;
    errors.push(msg);
    console.error(msg);
  }
}

console.log(`\nProcessed: ${filesProcessed} files`);
console.log(`Modified:  ${filesModified} files`);
if (errors.length > 0) {
  console.log(`Errors:    ${errors.length} files`);
}
