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
  if (!c.includes('(done)') && !c.includes('(done:') && !c.includes('( done)')) continue;
  const orig = c;

  // Strategy: Replace (done) parameter with () and wrap body in new Promise
  // Pattern 1: it('name', (done) => { body with done() })
  // → it('name', () => new Promise<void>(done => { body with done() }))
  //
  // Pattern 2: it('name', (done: DoneFn) => { body })
  // → it('name', () => new Promise<void>(done => { body }))
  //
  // We also handle beforeEach/afterEach with done

  // Replace (done) => { ... } and (done: DoneFn) => { ... } in it/beforeEach/afterEach
  // with () => new Promise<void>(done => { ... })
  const testFnPattern = /((?:it|beforeEach|afterEach|beforeAll|afterAll)\s*\([^,]*,\s*)\(done(?:\s*:\s*\w+)?\)\s*=>\s*\{/g;

  c = c.replace(testFnPattern, (match, prefix) => {
    return `${prefix}() => new Promise<void>(done => {`;
  });

  // Also need to add closing ) for the Promise before the final });
  // This is tricky — we need to find the matching } for each converted block.
  // Simpler approach: just add a closing paren before the last }) of each block.
  // But that's error-prone. Let's use a different approach:
  // Count opens and closes to find the right spot.

  // Actually, let's take a simpler approach that works for most cases:
  // The function keyword version: function(done) { ... }
  const testFnPattern2 = /((?:it|beforeEach|afterEach|beforeAll|afterAll)\s*\([^,]*,\s*)function\s*\(done(?:\s*:\s*\w+)?\)\s*\{/g;
  c = c.replace(testFnPattern2, (match, prefix) => {
    return `${prefix}function() { return new Promise<void>(done => {`;
  });

  // For the arrow function version, we replaced (done) => { with () => new Promise<void>(done => {
  // We need to add one extra }) at the end of each test block.
  // Since the test body ends with `});` (closing the it()), we need to insert `)` before the last `}`
  // The test block structure is:
  //   it('name', () => new Promise<void>(done => {
  //     ...test body...
  //     done();
  //   }));
  // So we need the Promise closing paren before the test function's closing brace.

  // This is too complex for regex. Let's use a different strategy:
  // Since we know the number of replacements, we can try to be smarter.
  // For now, let's count how many replacements we made and then
  // for each done() call inside these blocks, the last done() is followed by
  // }); which closes the subscribe callback and the it() — we need to add `)` to close the Promise.

  // Actually the simplest reliable approach: search for `done();\n  });` patterns
  // and replace with `done();\n  }));` — adding the Promise close paren.
  // But this won't work for all indentation levels.

  // Better approach: after converting the opening, find all done() calls
  // and the corresponding test block end, then add the Promise close.

  // Let's just do a more targeted conversion:
  // Find test blocks that contain done() and wrap the entire body.

  // RESET — let's try a completely different approach that's more reliable.
  c = orig; // reset

  // For each it/beforeEach/afterEach with (done), find the full block and wrap it
  const lines = c.split('\n');
  const result = [];
  let i = 0;
  let changed = false;

  while (i < lines.length) {
    const line = lines[i];
    // Check if this line starts a test with done callback
    const doneMatch = line.match(/^(\s*)((?:it|beforeEach|afterEach|beforeAll|afterAll)\s*\([^,]*,\s*)\(done(?:\s*:\s*\w+)?\)\s*=>\s*\{(.*)$/);
    const doneFnMatch = !doneMatch && line.match(/^(\s*)((?:it|beforeEach|afterEach|beforeAll|afterAll)\s*\([^,]*,\s*)function\s*\(done(?:\s*:\s*\w+)?\)\s*\{(.*)$/);

    if (doneMatch || doneFnMatch) {
      const m = doneMatch || doneFnMatch;
      const indent = m[1];
      const prefix = m[2];
      const restOfLine = m[3];

      // Find the closing of this block by counting braces
      let depth = 1; // we've seen the opening {
      // Count braces in rest of line
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

      // Now we have all lines from i+1 to j-1 that form the body
      // The last line (j-1) should end with }); which closes the arrow fn and the it()
      // We need: () => new Promise<void>(done => { body }));

      // Emit the modified opening
      result.push(`${indent}${prefix}() => new Promise<void>(done => {${restOfLine}`);

      // Emit body lines (all except the last one)
      for (let k = 1; k < bodyLines.length - 1; k++) {
        result.push(bodyLines[k]);
      }

      // The last body line closes the function. We need to add `)` before the final `);`
      // Typically it's: `  });` → `  }));`
      if (bodyLines.length > 1) {
        const lastLine = bodyLines[bodyLines.length - 1];
        // Add closing paren for Promise before the closing of it()
        // Pattern: indent + }); → indent + }));
        const closingMatch = lastLine.match(/^(\s*)\}\);(.*)$/);
        if (closingMatch) {
          result.push(`${closingMatch[1]}}));${closingMatch[2]}`);
        } else {
          // Fallback: just add ) before the last );
          result.push(lastLine.replace(/\}\);/, '}));'));
        }
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

// Verify
let remaining = 0;
for (const file of findSpecs('src')) {
  const c = fs.readFileSync(file, 'utf-8');
  if (/\(done\)\s*=>/.test(c) || /\(done:\s*\w+\)\s*=>/.test(c)) {
    remaining++;
  }
}
console.log(`Files with remaining done callbacks: ${remaining}`);
