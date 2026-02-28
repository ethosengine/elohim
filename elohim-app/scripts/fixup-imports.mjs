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

  // Fix: provideHttpClient used but not imported from @angular/common/http
  const usesProvideHttpClient = /provideHttpClient\b/.test(c);
  const hasProvideHttpClientImport = /import[^;]*provideHttpClient[^;]*from\s*'@angular\/common\/http'\s*;/.test(c);
  if (usesProvideHttpClient && !hasProvideHttpClientImport) {
    const httpMatch = c.match(/(import\s*\{([^}]*)\}\s*from\s*'@angular\/common\/http'\s*;)/);
    if (httpMatch) {
      const imports = httpMatch[2].split(',').map(s => s.trim()).filter(Boolean);
      if (!imports.includes('provideHttpClient')) {
        imports.push('provideHttpClient');
        c = c.replace(httpMatch[1], `import { ${imports.join(', ')} } from '@angular/common/http';`);
      }
    } else {
      // Find last import statement and add after it
      const lines = c.split('\n');
      let lastImportLine = -1;
      for (let i = 0; i < lines.length; i++) {
        if (/^\s*import\s/.test(lines[i])) lastImportLine = i;
      }
      if (lastImportLine >= 0) {
        lines.splice(lastImportLine + 1, 0, "import { provideHttpClient } from '@angular/common/http';");
        c = lines.join('\n');
      }
    }
  }

  // Fix: provideHttpClientTesting used but not imported from @angular/common/http/testing
  const usesProvideHttpClientTesting = /provideHttpClientTesting\b/.test(c);
  const hasProvideHttpClientTestingImport = /import[^;]*provideHttpClientTesting[^;]*from\s*'@angular\/common\/http\/testing'\s*;/.test(c);
  if (usesProvideHttpClientTesting && !hasProvideHttpClientTestingImport) {
    const httpTestMatch = c.match(/(import\s*\{([^}]*)\}\s*from\s*'@angular\/common\/http\/testing'\s*;)/);
    if (httpTestMatch) {
      const imports = httpTestMatch[2].split(',').map(s => s.trim()).filter(Boolean);
      if (!imports.includes('provideHttpClientTesting')) {
        imports.push('provideHttpClientTesting');
        c = c.replace(httpTestMatch[1], `import { ${imports.join(', ')} } from '@angular/common/http/testing';`);
      }
    } else {
      const lines = c.split('\n');
      let lastImportLine = -1;
      for (let i = 0; i < lines.length; i++) {
        if (/^\s*import\s/.test(lines[i])) lastImportLine = i;
      }
      if (lastImportLine >= 0) {
        lines.splice(lastImportLine + 1, 0, "import { provideHttpClientTesting } from '@angular/common/http/testing';");
        c = lines.join('\n');
      }
    }
  }

  // Fix: xdescribe → describe.skip, xit → it.skip
  c = c.replace(/\bxdescribe\(/g, 'describe.skip(');
  c = c.replace(/\bxit\(/g, 'it.skip(');

  // Fix: remaining jasmine references
  c = c.replace(/jasmine\.anything\(\)/g, 'expect.anything()');
  c = c.replace(/jasmine\.objectContaining\(/g, 'expect.objectContaining(');
  c = c.replace(/jasmine\.arrayContaining\(/g, 'expect.arrayContaining(');
  c = c.replace(/jasmine\.stringMatching\(/g, 'expect.stringMatching(');
  c = c.replace(/jasmine\.stringContaining\(/g, 'expect.stringContaining(');
  c = c.replace(/jasmine\.createSpy\([^)]*\)/g, 'vi.fn()');

  // Fix: remaining spyOnProperty(obj, 'prop') → vi.spyOn(obj, 'prop', 'get')
  c = c.replace(/(?<![.\w])spyOnProperty\(([^,]+),\s*([^,)]+)\)/g, "vi.spyOn($1, $2, 'get')");

  // Fix: remaining bare spyOn → vi.spyOn
  c = c.replace(/(?<![.\w])(?<!vi\.)spyOn\(/g, 'vi.spyOn(');

  // Fix: expectAsync patterns
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeResolved\(\)/g, 'await expect($1).resolves.toBeDefined()');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeRejected\(\)/g, 'await expect($1).rejects.toThrow()');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeRejectedWith\(([^)]+)\)/g, 'await expect($1).rejects.toThrow($2)');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeResolvedTo\(([^)]+)\)/g, 'await expect($1).resolves.toEqual($2)');

  // Fix: done() callback → return new Promise pattern
  // Simple pattern: it('name', (done) => { ...done()... })
  // We'll just remove done param and done() calls for sync tests
  // For async observable tests, wrap in firstValueFrom or similar

  // Ensure vi import if needed
  if (/\bvi\.(fn|spyOn|mock|mocked)\b/.test(c)) {
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

  if (c !== orig) {
    fs.writeFileSync(file, c);
    fixed++;
    console.log('Fixed:', path.relative('.', file));
  }
}
console.log(`\nFixed ${fixed} files`);
