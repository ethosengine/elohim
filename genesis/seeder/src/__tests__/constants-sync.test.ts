/**
 * Constants Sync Tests
 *
 * Validates that content format constants stay in sync across:
 * - Rust healing.rs (source of truth)
 * - Generated TypeScript schema-enums.ts
 * - Actual content JSON files in genesis/data/lamad/content/
 *
 * Catches drift between layers that caused the "sophia format rejected" bug.
 */
import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

import { CONTENT_FORMATS, CONTENT_TYPES } from '../validation-constants.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SEEDER_DIR = path.resolve(__dirname, '..', '..');
const GENESIS_DIR = path.resolve(SEEDER_DIR, '..');
const CONTENT_DIR = path.join(GENESIS_DIR, 'data', 'lamad', 'content');
const APP_GENERATED = path.resolve(
  GENESIS_DIR,
  '..',
  'app',
  'elohim-app',
  'src',
  'app',
  'generated',
  'schema-enums.ts',
);
const SEEDER_GENERATED = path.resolve(SEEDER_DIR, 'src', 'generated', 'schema-enums.ts');
const HEALING_RS = path.resolve(
  GENESIS_DIR,
  '..',
  'elohim',
  'holochain',
  'dna',
  'elohim',
  'zomes',
  'content_store_integrity',
  'src',
  'healing.rs',
);

/** Normalization mappings from seed-sqlite.ts that map variant formats to canonical */
const FORMAT_MAPPINGS: Record<string, string> = {
  'perseus-quiz-json': 'perseus',
  'perseus-quiz': 'perseus',
  'quiz-json': 'perseus',
  'sophia-moment-json': 'sophia',
  'sophia-quiz-json': 'sophia',
  'sophia-mastery': 'sophia',
  'sophia-discovery': 'sophia',
  md: 'markdown',
  htm: 'html',
  txt: 'text',
};

/** Parse a Rust const array from healing.rs source */
function parseRustConstArray(source: string, constName: string): string[] {
  const pattern = new RegExp(
    `pub const ${constName}:\\s*&\\[&str\\]\\s*=\\s*&\\[([\\s\\S]*?)\\];`,
  );
  const match = source.match(pattern);
  if (!match) return [];
  return [...match[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
}

/** Collect unique contentFormat values from all content JSON files */
function collectContentFormats(): Set<string> {
  const formats = new Set<string>();
  if (!fs.existsSync(CONTENT_DIR)) return formats;

  const files = fs.readdirSync(CONTENT_DIR).filter((f) => f.endsWith('.json'));
  for (const file of files) {
    try {
      const data = JSON.parse(
        fs.readFileSync(path.join(CONTENT_DIR, file), 'utf8'),
      );
      if (data.contentFormat) {
        formats.add(data.contentFormat);
      }
    } catch {
      // Skip malformed files
    }
  }
  return formats;
}

describe('Constants Sync: healing.rs ↔ schema-enums.ts', () => {
  it('generated CONTENT_FORMATS should match healing.rs', () => {
    if (!fs.existsSync(HEALING_RS)) {
      console.warn('healing.rs not found, skipping Rust sync check');
      return;
    }

    const rustSource = fs.readFileSync(HEALING_RS, 'utf8');
    const rustFormats = parseRustConstArray(rustSource, 'CONTENT_FORMATS');

    expect(rustFormats.length).toBeGreaterThan(0);

    // Every Rust format should be in the generated TypeScript
    for (const format of rustFormats) {
      expect(
        (CONTENT_FORMATS as readonly string[]).includes(format),
      ).toBe(true);
    }

    // Every TypeScript format should be in Rust
    for (const format of CONTENT_FORMATS) {
      expect(rustFormats).toContain(format);
    }
  });

  it('CONTENT_FORMATS should include sophia assessment formats', () => {
    expect(CONTENT_FORMATS).toContain('sophia');
    expect(CONTENT_FORMATS).toContain('sophia-quiz-json');
  });

  it('CONTENT_FORMATS should include perseus formats', () => {
    expect(CONTENT_FORMATS).toContain('perseus');
    expect(CONTENT_FORMATS).toContain('perseus-json');
    expect(CONTENT_FORMATS).toContain('perseus-quiz-json');
  });
});

describe('Constants Sync: content JSONs ↔ CONTENT_FORMATS', () => {
  it('every contentFormat in seed data should be recognized', () => {
    const usedFormats = collectContentFormats();
    if (usedFormats.size === 0) {
      console.warn('No content files found, skipping');
      return;
    }

    const allFormats = CONTENT_FORMATS as readonly string[];
    const unrecognized: string[] = [];

    for (const format of usedFormats) {
      const normalized = format.toLowerCase();
      const isKnown =
        allFormats.includes(normalized) || normalized in FORMAT_MAPPINGS;
      if (!isKnown) {
        unrecognized.push(format);
      }
    }

    expect(
      unrecognized,
    ).toEqual([]);
  });

  it('every contentType in seed data should be recognized', () => {
    const usedTypes = new Set<string>();
    if (!fs.existsSync(CONTENT_DIR)) return;

    const files = fs.readdirSync(CONTENT_DIR).filter((f) => f.endsWith('.json'));
    for (const file of files) {
      try {
        const data = JSON.parse(
          fs.readFileSync(path.join(CONTENT_DIR, file), 'utf8'),
        );
        if (data.contentType) usedTypes.add(data.contentType);
      } catch {
        // Skip
      }
    }

    const allTypes = CONTENT_TYPES as readonly string[];
    const unrecognized: string[] = [];
    for (const ct of usedTypes) {
      if (!allTypes.includes(ct.toLowerCase())) {
        unrecognized.push(ct);
      }
    }

    expect(
      unrecognized,
    ).toEqual([]);
  });
});

describe('Constants Sync: seeder generated ↔ app generated', () => {
  it('app-side schema-enums.ts should match seeder-side schema-enums.ts', () => {
    if (!fs.existsSync(APP_GENERATED)) {
      throw new Error(
        `App-side generated file not found at ${APP_GENERATED}. ` +
          'Run: cd genesis/seeder && npx tsx src/generate-schema-types.ts',
      );
    }
    if (!fs.existsSync(SEEDER_GENERATED)) {
      throw new Error(`Seeder-side generated file not found at ${SEEDER_GENERATED}`);
    }

    const appContent = fs.readFileSync(APP_GENERATED, 'utf8');
    const seederContent = fs.readFileSync(SEEDER_GENERATED, 'utf8');

    // Strip timestamp line (differs per run) before comparing
    const stripTimestamp = (s: string) => s.replace(/^\/\/ Generated at:.*$/m, '');
    expect(stripTimestamp(appContent)).toEqual(stripTimestamp(seederContent));
  });
});
