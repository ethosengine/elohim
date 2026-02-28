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

  // 1. Fix remaining jasmine.createSpy(...) (multiline) → vi.fn()
  // Find all jasmine.createSpy occurrences
  let idx = 0;
  while (true) {
    const pos = c.indexOf('jasmine.createSpy(', idx);
    if (pos === -1) break;
    // Find matching close paren
    let depth = 0;
    let end = -1;
    for (let i = pos + 'jasmine.createSpy'.length; i < c.length; i++) {
      if (c[i] === '(') depth++;
      else if (c[i] === ')') {
        depth--;
        if (depth === 0) { end = i + 1; break; }
      }
    }
    if (end === -1) { idx = pos + 1; continue; }
    c = c.substring(0, pos) + 'vi.fn()' + c.substring(end);
    idx = pos + 'vi.fn()'.length;
  }

  // 2. Fix remaining spyOnProperty → vi.spyOn (3-arg version too)
  c = c.replace(/(?<![.\w])spyOnProperty\(/g, 'vi.spyOn(');

  // 3. Fix expectAsync patterns (multiline safe)
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeResolved\(\)/g, 'await expect($1).resolves.toBeDefined()');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeRejected\(\)/g, 'await expect($1).rejects.toThrow()');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeRejectedWith\(([^)]+)\)/g, 'await expect($1).rejects.toThrow($2)');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeResolvedTo\(([^)]+)\)/g, 'await expect($1).resolves.toEqual($2)');
  c = c.replace(/expectAsync\(([^)]+)\)\.toBeRejectedWithError\(([^)]*)\)/g, 'await expect($1).rejects.toThrow($2)');

  // 4. Fix remaining bare spyOn
  c = c.replace(/(?<![.\w])(?<!vi\.)spyOn\(/g, 'vi.spyOn(');

  // 5. Fix .and.returnValue / .and.callFake / .and.resolveTo / .and.rejectWith
  c = c.replace(/\.and\.returnValue\(/g, '.mockReturnValue(');
  c = c.replace(/\.and\.callFake\(/g, '.mockImplementation(');
  c = c.replace(/\.and\.resolveTo\(/g, '.mockResolvedValue(');
  c = c.replace(/\.and\.rejectWith\(/g, '.mockRejectedValue(');
  c = c.replace(/\.and\.callThrough\(\)/g, '.mockRestore()');

  // 6. Fix .calls.reset() etc
  c = c.replace(/\.calls\.reset\(\)/g, '.mockReset()');
  c = c.replace(/\.calls\.count\(\)/g, '.mock.calls.length');
  c = c.replace(/\.calls\.all\(\)/g, '.mock.calls');
  c = c.replace(/\.calls\.allArgs\(\)/g, '.mock.calls');
  c = c.replace(/\.calls\.mostRecent\(\)\.args/g, '.mock.calls[.mock.calls.length - 1]');
  c = c.replace(/\.calls\.argsFor\((\d+)\)/g, '.mock.calls[$1]');

  // 7. Ensure vi import
  if (/\bvi\.(fn|spyOn)\b/.test(c) && !/import\s*\{[^}]*\bvi\b[^}]*\}\s*from\s*'vitest'/.test(c)) {
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

  if (c !== orig) {
    fs.writeFileSync(file, c);
    fixed++;
    console.log('Fixed:', path.relative('.', file));
  }
}
console.log(`\nFixed ${fixed} files`);

// Verification
let remaining = 0;
for (const file of findSpecs('src')) {
  const c = fs.readFileSync(file, 'utf-8');
  const lines = c.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (/\/\//.test(line)) continue; // skip comments
    if (/import/.test(line)) continue;
    if (/jasmine\./.test(line) || /(?<![.\w])spyOnProperty\(/.test(line) || /\bexpectAsync\(/.test(line)) {
      remaining++;
      if (remaining <= 10) console.log(`Remaining: ${path.relative('.', file)}:${i + 1}: ${line.trim()}`);
    }
  }
}
console.log(`Total remaining references: ${remaining}`);
