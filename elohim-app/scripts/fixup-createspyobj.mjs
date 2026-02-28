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

/**
 * Replace jasmine.createSpyObj calls (including multiline) with vi.fn() objects.
 * Handles these patterns:
 *   jasmine.createSpyObj('Name', ['m1', 'm2'])
 *   jasmine.createSpyObj<Type>('Name', ['m1', 'm2'])
 *   jasmine.createSpyObj('Name', ['m1'], { prop1: value })
 *   jasmine.createSpyObj(\n  'Name',\n  ['m1', 'm2']\n)
 */
function replaceCreateSpyObj(content) {
  // Find all jasmine.createSpyObj occurrences and their positions
  const pattern = /jasmine\.createSpyObj(?:<[^>]+>)?\s*\(/g;
  let match;
  const replacements = [];

  while ((match = pattern.exec(content)) !== null) {
    const start = match.index;
    // Find the matching closing paren
    let depth = 0;
    let i = start + match[0].length - 1; // position of the opening paren
    let end = -1;
    for (; i < content.length; i++) {
      if (content[i] === '(') depth++;
      else if (content[i] === ')') {
        depth--;
        if (depth === 0) {
          end = i + 1;
          break;
        }
      }
    }
    if (end === -1) continue;

    const fullCall = content.substring(start, end);
    // Extract the arguments (everything between outer parens)
    const argsStart = fullCall.indexOf('(') + 1;
    const argsStr = fullCall.substring(argsStart, fullCall.length - 1).trim();

    // Parse the array of method names from the arguments
    // The args are: 'Name', ['m1', 'm2'], optionalPropsObj
    const arrayMatch = argsStr.match(/\[\s*([^\]]*)\s*\]/);
    if (arrayMatch) {
      const methods = arrayMatch[1]
        .split(',')
        .map(s => s.trim().replace(/^['"]|['"]$/g, ''))
        .filter(Boolean);

      // Check for third argument (properties object)
      const afterArray = argsStr.substring(argsStr.indexOf(']') + 1).trim();
      let propsStr = '';
      if (afterArray.startsWith(',')) {
        propsStr = afterArray.substring(1).trim();
      }

      let replacement;
      if (methods.length === 0 && !propsStr) {
        replacement = '{}';
      } else {
        const entries = methods.map(m => `${m}: vi.fn()`);
        if (propsStr && propsStr.startsWith('{')) {
          // Merge properties — extract key:value pairs from props object
          const propsInner = propsStr.slice(1, propsStr.lastIndexOf('}')).trim();
          if (propsInner) {
            entries.push(propsInner);
          }
        }
        replacement = `{ ${entries.join(', ')} }`;
      }

      replacements.push({ start, end, replacement });
    }
  }

  // Apply replacements in reverse order to preserve positions
  for (let i = replacements.length - 1; i >= 0; i--) {
    const r = replacements[i];
    content = content.substring(0, r.start) + r.replacement + content.substring(r.end);
  }

  return content;
}

let fixed = 0;
for (const file of findSpecs('src')) {
  let c = fs.readFileSync(file, 'utf-8');
  if (!c.includes('jasmine.createSpyObj')) continue;

  const orig = c;
  c = replaceCreateSpyObj(c);

  // Ensure vi import
  if (/\bvi\.fn\b/.test(c) && !/import\s*\{[^}]*\bvi\b[^}]*\}\s*from\s*'vitest'/.test(c)) {
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
