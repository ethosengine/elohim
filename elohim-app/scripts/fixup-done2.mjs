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
  // Match both (done) => and done =>
  if (!/(?:\(done(?:\s*:\s*\w+)?\)|done)\s*=>/.test(c)) continue;
  const orig = c;

  const lines = c.split('\n');
  const result = [];
  let i = 0;
  let changed = false;

  while (i < lines.length) {
    const line = lines[i];

    // Match patterns:
    // it('name', (done) => {
    // it('name', done => {
    // it('name', (done: DoneFn) => {
    // beforeEach((done) => {
    // Also with fakeAsync wrapping removed in some cases
    const doneMatch = line.match(
      /^(\s*)((?:it|beforeEach|afterEach|beforeAll|afterAll)\s*\([^,]*,\s*)(?:\(done(?:\s*:\s*\w+)?\)|done)\s*=>\s*\{(.*)$/
    );

    if (doneMatch) {
      const indent = doneMatch[1];
      const prefix = doneMatch[2];
      const restOfLine = doneMatch[3];

      // Count braces to find the end of this block
      let depth = 1;
      for (const ch of restOfLine) {
        if (ch === '{') depth++;
        else if (ch === '}') depth--;
      }

      const bodyLines = [restOfLine];
      let j = i + 1;

      while (j < lines.length && depth > 0) {
        const bodyLine = lines[j];
        for (const ch of bodyLine) {
          if (ch === '{') depth++;
          else if (ch === '}') depth--;
        }
        bodyLines.push(bodyLine);
        j++;
      }

      // Emit modified opening
      result.push(`${indent}${prefix}() => new Promise<void>(done => {${restOfLine}`);

      // Emit body lines except last
      for (let k = 1; k < bodyLines.length - 1; k++) {
        result.push(bodyLines[k]);
      }

      // Fix last line: }); → }));
      if (bodyLines.length > 1) {
        const lastLine = bodyLines[bodyLines.length - 1];
        // Add closing paren for Promise
        result.push(lastLine.replace(/\}\);/, '}));'));
      }

      changed = true;
      i = j;
    } else {
      result.push(line);
      i++;
    }
  }

  if (changed) {
    c = result.join('\n');
    fs.writeFileSync(file, c);
    fixed++;
    console.log('Fixed:', path.relative('.', file));
  }
}
console.log(`\nFixed ${fixed} files`);

// Verify remaining
let remaining = 0;
for (const file of findSpecs('src')) {
  const c = fs.readFileSync(file, 'utf-8');
  const lines = c.split('\n');
  for (const line of lines) {
    if (/\/\//.test(line.split('=>')[0])) continue;
    if (/(?:it|beforeEach|afterEach)\s*\([^,]*,\s*(?:\(done|done\s*=>)/.test(line)) {
      remaining++;
    }
  }
}
console.log(`Remaining done callback test blocks: ${remaining}`);
