import fs from 'fs';
import path from 'path';

function findSpecs(dir) {
  let results = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const f = path.join(dir, e.name);
    if (e.isDirectory()) results.push(...findSpecs(f));
    else if (e.name.endsWith('.spec.ts')) results.push(f);
  }
  return results;
}

let fixed = 0;
for (const file of findSpecs('src')) {
  let c = fs.readFileSync(file, 'utf-8');
  const orig = c;

  // 1. jasmine.any(Type) → expect.any(Type)
  c = c.replace(/jasmine\.any\(/g, 'expect.any(');

  // 2. jasmine.createSpyObj('Name', ['method1', 'method2'])
  // → { method1: vi.fn(), method2: vi.fn() }
  c = c.replace(
    /jasmine\.createSpyObj\(\s*'[^']*'\s*,\s*\[([^\]]*)\]\s*\)/g,
    (match, methods) => {
      const methodList = methods.split(',').map(s => s.trim().replace(/^['"]|['"]$/g, '')).filter(Boolean);
      if (methodList.length === 0) return '{}';
      const entries = methodList.map(m => `${m}: vi.fn()`).join(', ');
      return `{ ${entries} }`;
    }
  );

  // Also handle jasmine.createSpyObj<Type>('Name', ['method1', ...])
  c = c.replace(
    /jasmine\.createSpyObj<[^>]+>\(\s*'[^']*'\s*,\s*\[([^\]]*)\]\s*\)/g,
    (match, methods) => {
      const methodList = methods.split(',').map(s => s.trim().replace(/^['"]|['"]$/g, '')).filter(Boolean);
      if (methodList.length === 0) return '{}';
      const entries = methodList.map(m => `${m}: vi.fn()`).join(', ');
      return `{ ${entries} }`;
    }
  );

  // 3. jasmine.Spy → Mock (vitest type)
  // as jasmine.Spy → as Mock
  c = c.replace(/as\s+jasmine\.Spy\b/g, 'as Mock');
  // jasmine.Spy type annotations in variable declarations
  c = c.replace(/:\s*jasmine\.Spy\b/g, ': Mock');

  // 4. jasmine.SpyObj<Type> → Record<string, Mock> or just the type
  c = c.replace(/jasmine\.SpyObj<([^>]+)>/g, '{ [K in keyof $1]?: Mock }');
  // Simpler version for variable declarations
  c = c.replace(/jasmine\.SpyObj\b/g, 'Record<string, Mock>');

  // 5. jasmine.clock().install() → vi.useFakeTimers()
  c = c.replace(/jasmine\.clock\(\)\.install\(\)/g, 'vi.useFakeTimers()');
  // jasmine.clock().uninstall() → vi.useRealTimers()
  c = c.replace(/jasmine\.clock\(\)\.uninstall\(\)/g, 'vi.useRealTimers()');
  // jasmine.clock().tick(ms) → vi.advanceTimersByTime(ms)
  c = c.replace(/jasmine\.clock\(\)\.tick\(([^)]+)\)/g, 'vi.advanceTimersByTime($1)');

  // 6. remaining spyOnProperty(obj, 'prop') → vi.spyOn(obj, 'prop', 'get')
  c = c.replace(/(?<![.\w])spyOnProperty\(([^,]+),\s*([^,)]+)\)/g, "vi.spyOn($1, $2, 'get')");

  // 7. remaining bare spyOn
  c = c.replace(/(?<![.\w])(?<!vi\.)spyOn\(/g, 'vi.spyOn(');

  // 8. expectAsync patterns
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeResolved\(\)/g, 'await expect($1).resolves.toBeDefined()');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeRejected\(\)/g, 'await expect($1).rejects.toThrow()');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeRejectedWith\(([^)]+)\)/g, 'await expect($1).rejects.toThrow($2)');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeResolvedTo\(([^)]+)\)/g, 'await expect($1).resolves.toEqual($2)');
  // expectAsync(x).toBeRejectedWithError(msg) → expect(x).rejects.toThrow(msg)
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeRejectedWithError\(([^)]*)\)/g, 'await expect($1).rejects.toThrow($2)');

  // 9. Ensure Mock type is imported if used
  if (/\bMock\b/.test(c) && !/import.*\bMock\b.*from\s*'vitest'/.test(c)) {
    const vitestMatch = c.match(/(import\s*\{([^}]*)\}\s*from\s*'vitest'\s*;)/);
    if (vitestMatch) {
      const imports = vitestMatch[2].split(',').map(s => s.trim()).filter(Boolean);
      if (!imports.includes('Mock')) {
        imports.push('Mock');
        c = c.replace(vitestMatch[1], `import { ${imports.join(', ')} } from 'vitest';`);
      }
    }
  }

  // 10. Ensure vi import if vi.fn/vi.spyOn used
  if (/\bvi\.(fn|spyOn|mock|mocked|useFakeTimers|useRealTimers|advanceTimersByTime)\b/.test(c)) {
    const hasViImport = /import\s*\{[^}]*\bvi\b[^}]*\}\s*from\s*'vitest'/.test(c);
    if (!hasViImport) {
      const vitestMatch = c.match(/(import\s*\{([^}]*)\}\s*from\s*'vitest'\s*;)/);
      if (vitestMatch) {
        const imports = vitestMatch[2].split(',').map(s => s.trim()).filter(Boolean);
        if (!imports.includes('vi')) {
          imports.push('vi');
          c = c.replace(vitestMatch[1], `import { ${imports.join(', ')} } from 'vitest';`);
        }
      } else {
        c = "import { vi } from 'vitest';\n" + c;
      }
    }
  }

  // 11. .and.returnValue → .mockReturnValue (any remaining)
  c = c.replace(/\.and\.returnValue\(/g, '.mockReturnValue(');
  c = c.replace(/\.and\.callFake\(/g, '.mockImplementation(');
  c = c.replace(/\.and\.resolveTo\(/g, '.mockResolvedValue(');
  c = c.replace(/\.and\.rejectWith\(/g, '.mockRejectedValue(');

  // 12. .calls.reset() → .mockReset() (any remaining)
  c = c.replace(/\.calls\.reset\(\)/g, '.mockReset()');
  c = c.replace(/\.calls\.count\(\)/g, '.mock.calls.length');

  if (c !== orig) {
    fs.writeFileSync(file, c);
    fixed++;
    console.log('Fixed:', path.relative('.', file));
  }
}
console.log(`\nFixed ${fixed} files`);
