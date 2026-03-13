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
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Resolve repo root reliably — __dirname-based relative paths can break
// when tsx resolves import.meta.url differently based on CWD vs source location
function findRepoRoot(): string {
  try {
    return execSync('git rev-parse --show-toplevel', { encoding: 'utf-8' }).trim();
  } catch {
    // Fallback: walk up from __dirname looking for genesis/ directory
    let dir = __dirname;
    for (let i = 0; i < 10; i++) {
      if (fs.existsSync(path.join(dir, 'genesis')) && fs.existsSync(path.join(dir, 'elohim'))) {
        return dir;
      }
      dir = path.dirname(dir);
    }
    // Last resort: assume __dirname is genesis/seeder/src
    return path.resolve(__dirname, '..', '..', '..');
  }
}

const REPO_ROOT = findRepoRoot();

const RUST_SOURCE = path.join(
  REPO_ROOT,
  'elohim/holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs'
);

const OUTPUT_FILE = path.resolve(__dirname, 'generated/schema-enums.ts');
const APP_OUTPUT_FILE = path.join(
  REPO_ROOT,
  'app/elohim-app/src/app/generated/schema-enums.ts'
);

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
    '// Source: elohim/holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs',
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

    // Generate type name: CONTENT_TYPES -> ContentType (singular PascalCase)
    let typeName = constant.name
      .toLowerCase()
      .split('_')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join('');
    // Singularize: remove trailing 's', but handle 'ies' -> 'y'
    if (typeName.endsWith('ies')) {
      typeName = typeName.slice(0, -3) + 'y';
    } else if (typeName.endsWith('s')) {
      typeName = typeName.slice(0, -1);
    }
    lines.push(`export type ${typeName} = typeof ${constant.name}[number];`);
    lines.push('');
  }

  return lines.join('\n');
}

function main() {
  // Check if Rust source exists
  if (!fs.existsSync(RUST_SOURCE)) {
    console.error(`ERROR: Rust source not found at ${RUST_SOURCE}`);
    console.error(`  __dirname: ${__dirname}`);
    console.error(`  REPO_ROOT: ${REPO_ROOT}`);
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

  // Also write to elohim-app for direct import
  const appOutputDir = path.dirname(APP_OUTPUT_FILE);
  if (!fs.existsSync(appOutputDir)) {
    fs.mkdirSync(appOutputDir, { recursive: true });
  }
  fs.writeFileSync(APP_OUTPUT_FILE, typescript);
  console.log(`✅ Generated: ${APP_OUTPUT_FILE}`);
}

main();
