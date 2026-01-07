#!/usr/bin/env npx tsx
/**
 * Generate TypeScript schema types from Rust source
 *
 * Parses healing.rs to extract validation constants and generates
 * schema-enums.ts for use by the seeder. This keeps the Rust source
 * as the single source of truth without requiring the Rust binary.
 */

import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const RUST_SOURCE = path.resolve(
  __dirname,
  '../../../holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs'
);

const OUTPUT_FILE = path.resolve(__dirname, 'generated/schema-enums.ts');

interface ParsedConstant {
  name: string;
  values: string[];
  comments: string[];
}

function parseRustConstants(source: string): ParsedConstant[] {
  const constants: ParsedConstant[] = [];

  // Match pub const NAME: &[&str] = &[...];
  const constRegex = /(?:\/\/\/?\s*(.+?)\n)*pub\s+const\s+(\w+):\s*&\[&str\]\s*=\s*&\[([\s\S]*?)\];/g;

  let match;
  while ((match = constRegex.exec(source)) !== null) {
    const comments: string[] = [];
    const name = match[2];
    const valuesBlock = match[3];

    // Extract doc comments from the full match
    const fullMatch = match[0];
    const docCommentRegex = /\/\/\/?\s*(.+)/g;
    let docMatch;
    const beforeConst = fullMatch.split('pub const')[0];
    while ((docMatch = docCommentRegex.exec(beforeConst)) !== null) {
      comments.push(docMatch[1].trim());
    }

    // Parse values, handling comments
    const values: string[] = [];
    const valueRegex = /"([^"]+)"/g;
    let valueMatch;
    while ((valueMatch = valueRegex.exec(valuesBlock)) !== null) {
      values.push(valueMatch[1]);
    }

    if (values.length > 0) {
      constants.push({ name, values, comments });
    }
  }

  return constants;
}

function generateTypeScript(constants: ParsedConstant[]): string {
  const lines: string[] = [
    '// AUTO-GENERATED from healing.rs - DO NOT EDIT',
    `// Generated at: ${new Date().toISOString()}`,
    '// Source: holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs',
    '',
  ];

  for (const constant of constants) {
    // Add doc comments
    if (constant.comments.length > 0) {
      lines.push('/**');
      for (const comment of constant.comments) {
        lines.push(` * ${comment}`);
      }
      lines.push(' */');
    }

    // Generate const array
    lines.push(`export const ${constant.name} = [`);
    for (const value of constant.values) {
      lines.push(`  '${value}',`);
    }
    lines.push('] as const;');
    lines.push('');

    // Generate type
    const typeName = constant.name.replace(/_/g, ' ')
      .replace(/\b\w/g, c => c.toUpperCase())
      .replace(/\s/g, '')
      .replace(/s$/, ''); // Remove trailing 's' for singular type name
    lines.push(`export type ${typeName} = typeof ${constant.name}[number];`);
    lines.push('');
  }

  return lines.join('\n');
}

function main() {
  // Check if Rust source exists
  if (!fs.existsSync(RUST_SOURCE)) {
    console.error(`ERROR: Rust source not found at ${RUST_SOURCE}`);
    process.exit(1);
  }

  console.log(`📖 Reading Rust source: ${RUST_SOURCE}`);
  const rustSource = fs.readFileSync(RUST_SOURCE, 'utf-8');

  console.log('🔍 Parsing constants...');
  const constants = parseRustConstants(rustSource);

  if (constants.length === 0) {
    console.error('ERROR: No constants found in Rust source');
    process.exit(1);
  }

  console.log(`   Found ${constants.length} constants:`);
  for (const c of constants) {
    console.log(`   - ${c.name}: ${c.values.length} values`);
  }

  console.log('📝 Generating TypeScript...');
  const typescript = generateTypeScript(constants);

  // Ensure output directory exists
  const outputDir = path.dirname(OUTPUT_FILE);
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  fs.writeFileSync(OUTPUT_FILE, typescript);
  console.log(`✅ Generated: ${OUTPUT_FILE}`);
}

main();
