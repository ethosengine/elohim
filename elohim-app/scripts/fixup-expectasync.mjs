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

function findMatchingParen(content, openPos) {
  let depth = 0;
  for (let i = openPos; i < content.length; i++) {
    if (content[i] === '(') depth++;
    else if (content[i] === ')') {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

let fixed = 0;
for (const file of findSpecs('src')) {
  let c = fs.readFileSync(file, 'utf-8');
  if (!c.includes('expectAsync(')) continue;
  const orig = c;

  // Find and replace all expectAsync(expr).toBeXxx(args) patterns
  let idx = 0;
  while (true) {
    const pos = c.indexOf('expectAsync(', idx);
    if (pos === -1) break;

    const openParen = pos + 'expectAsync'.length;
    const closeParen = findMatchingParen(c, openParen);
    if (closeParen === -1) { idx = pos + 1; continue; }

    const expr = c.substring(openParen + 1, closeParen).trim();
    const afterClose = c.substring(closeParen + 1).trimStart();

    // Match .toBeResolved(), .toBeRejected(), .toBeRejectedWith(...), .toBeResolvedTo(...), .toBeRejectedWithError(...)
    let replacement = null;
    let endPos = closeParen + 1;

    // Adjust endPos to skip whitespace
    let dotPos = closeParen + 1;
    while (dotPos < c.length && /\s/.test(c[dotPos])) dotPos++;

    if (c[dotPos] !== '.') { idx = pos + 1; continue; }

    const rest = c.substring(dotPos + 1);

    if (rest.startsWith('toBeResolved()')) {
      replacement = `await expect(${expr}).resolves.toBeDefined()`;
      endPos = dotPos + 1 + 'toBeResolved()'.length;
    } else if (rest.startsWith('toBeRejected()')) {
      replacement = `await expect(${expr}).rejects.toThrow()`;
      endPos = dotPos + 1 + 'toBeRejected()'.length;
    } else if (rest.startsWith('toBeRejectedWith(')) {
      const argOpen = dotPos + 1 + 'toBeRejectedWith'.length;
      const argClose = findMatchingParen(c, argOpen);
      if (argClose === -1) { idx = pos + 1; continue; }
      const args = c.substring(argOpen + 1, argClose).trim();
      replacement = `await expect(${expr}).rejects.toThrow(${args})`;
      endPos = argClose + 1;
    } else if (rest.startsWith('toBeResolvedTo(')) {
      const argOpen = dotPos + 1 + 'toBeResolvedTo'.length;
      const argClose = findMatchingParen(c, argOpen);
      if (argClose === -1) { idx = pos + 1; continue; }
      const args = c.substring(argOpen + 1, argClose).trim();
      replacement = `await expect(${expr}).resolves.toEqual(${args})`;
      endPos = argClose + 1;
    } else if (rest.startsWith('toBeRejectedWithError(')) {
      const argOpen = dotPos + 1 + 'toBeRejectedWithError'.length;
      const argClose = findMatchingParen(c, argOpen);
      if (argClose === -1) { idx = pos + 1; continue; }
      const args = c.substring(argOpen + 1, argClose).trim();
      replacement = `await expect(${expr}).rejects.toThrow(${args})`;
      endPos = argClose + 1;
    }

    if (replacement) {
      c = c.substring(0, pos) + replacement + c.substring(endPos);
      idx = pos + replacement.length;
    } else {
      idx = pos + 1;
    }
  }

  if (c !== orig) {
    fs.writeFileSync(file, c);
    fixed++;
    console.log('Fixed:', path.relative('.', file));
  }
}
console.log(`\nFixed ${fixed} files`);

// Verify
let remaining = 0;
for (const file of findSpecs('src')) {
  const c = fs.readFileSync(file, 'utf-8');
  const lines = c.split('\n');
  for (let i = 0; i < lines.length; i++) {
    if (/expectAsync\(/.test(lines[i]) && !/\/\//.test(lines[i].split('expectAsync')[0])) {
      remaining++;
    }
  }
}
console.log(`Remaining expectAsync: ${remaining}`);
