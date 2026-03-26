/**
 * Lamad Manifest Content Validation Tests
 *
 * Validates the Lamad app manifest's CONTENT — not just structural validity,
 * but that the manifest declares all expected content types, relationships,
 * signals, formats, and renderers for the learning pillar.
 *
 * Manifest: app/lamad/manifest.json
 */
import { describe, it, expect, beforeAll } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const MANIFEST_PATH = path.resolve(__dirname, '../../../../app/lamad/manifest.json');

interface Manifest {
  id: string;
  name: string;
  version: string;
  description: string;
  vocabulary: {
    contentTypes: Record<string, {
      description: string;
      coupling: {
        knowledge?: { relationships: Record<string, string[]> };
        value: Record<string, unknown>;
        governance: Record<string, unknown>;
      };
    }>;
    contentFormats: Record<string, {
      description: string;
      renderer?: string;
      mimeTypes?: string[];
      extensions?: string[];
      aliases?: string[];
    }>;
    relationships: Record<string, {
      description: string;
      sourceTypes: string[];
      targetTypes: string[];
      inverse?: string;
    }>;
    signals: Record<string, {
      description: string;
      substrateSignal: string;
      economicAction: string;
      resourceType: string;
    }>;
  };
  rendering: Record<string, {
    component: string;
    formats: string[];
    platform: string;
  }>;
}

let manifest: Manifest;

beforeAll(() => {
  const raw = fs.readFileSync(MANIFEST_PATH, 'utf8');
  manifest = JSON.parse(raw) as Manifest;
});

describe('Lamad manifest: app identity', () => {
  it('has correct id', () => {
    expect(manifest.id).toBe('manifest-lamad');
  });

  it('has correct name', () => {
    expect(manifest.name).toBe('lamad');
  });

  it('version follows semver', () => {
    const semverPattern = /^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$/;
    expect(manifest.version).toMatch(semverPattern);
  });

  it('has a description', () => {
    expect(manifest.description).toBeTruthy();
    expect(typeof manifest.description).toBe('string');
  });
});

describe('Lamad manifest: core content types', () => {
  const CORE_CONTENT_TYPES = [
    'lesson',
    'assessment',
    'path',
    'concept',
    'exercise',
    'reflection',
    'epic',
    'scenario',
    'discovery-assessment',
    'instrument',
  ];

  it.each(CORE_CONTENT_TYPES)('declares core content type: %s', (typeName) => {
    expect(manifest.vocabulary.contentTypes).toHaveProperty(typeName);
  });

  it('every content type has a description', () => {
    const missing: string[] = [];
    for (const [name, type] of Object.entries(manifest.vocabulary.contentTypes)) {
      if (!type.description || typeof type.description !== 'string') {
        missing.push(name);
      }
    }
    expect(missing, `Content types missing descriptions: ${missing.join(', ')}`).toEqual([]);
  });
});

describe('Lamad manifest: three-leg coupling', () => {
  it('every content type has coupling.value', () => {
    const missing: string[] = [];
    for (const [name, type] of Object.entries(manifest.vocabulary.contentTypes)) {
      if (!type.coupling || !('value' in type.coupling)) {
        missing.push(name);
      }
    }
    expect(missing, `Content types missing coupling.value: ${missing.join(', ')}`).toEqual([]);
  });

  it('every content type has coupling.governance', () => {
    const missing: string[] = [];
    for (const [name, type] of Object.entries(manifest.vocabulary.contentTypes)) {
      if (!type.coupling || !('governance' in type.coupling)) {
        missing.push(name);
      }
    }
    expect(missing, `Content types missing coupling.governance: ${missing.join(', ')}`).toEqual([]);
  });

  it('governance sections have a governanceModel', () => {
    const missing: string[] = [];
    for (const [name, type] of Object.entries(manifest.vocabulary.contentTypes)) {
      const gov = type.coupling?.governance as Record<string, unknown> | undefined;
      if (gov && !gov.governanceModel) {
        missing.push(name);
      }
    }
    expect(missing, `Content types with governance but no governanceModel: ${missing.join(', ')}`).toEqual([]);
  });
});

describe('Lamad manifest: content formats and renderers', () => {
  it('declares sophia-quiz-json format', () => {
    expect(manifest.vocabulary.contentFormats).toHaveProperty('sophia-quiz-json');
  });

  it('declares markdown format', () => {
    expect(manifest.vocabulary.contentFormats).toHaveProperty('markdown');
  });

  it('every format with a renderer references a valid renderer name', () => {
    const rendererNames = Object.keys(manifest.rendering);
    const invalid: string[] = [];

    for (const [formatName, format] of Object.entries(manifest.vocabulary.contentFormats)) {
      if (format.renderer && !rendererNames.includes(format.renderer)) {
        invalid.push(`${formatName} -> ${format.renderer}`);
      }
    }

    expect(
      invalid,
      `Formats reference non-existent renderers: ${invalid.join(', ')}`,
    ).toEqual([]);
  });

  it('every renderer lists at least one format', () => {
    const empty: string[] = [];
    for (const [name, renderer] of Object.entries(manifest.rendering)) {
      if (!renderer.formats || renderer.formats.length === 0) {
        empty.push(name);
      }
    }
    expect(empty, `Renderers with no formats: ${empty.join(', ')}`).toEqual([]);
  });

  it('every format listed in a renderer exists in contentFormats', () => {
    const formatNames = Object.keys(manifest.vocabulary.contentFormats);
    const invalid: string[] = [];

    for (const [rendererName, renderer] of Object.entries(manifest.rendering)) {
      for (const fmt of renderer.formats) {
        if (!formatNames.includes(fmt)) {
          invalid.push(`${rendererName} lists unknown format: ${fmt}`);
        }
      }
    }

    expect(
      invalid,
      `Renderers reference non-existent formats: ${invalid.join(', ')}`,
    ).toEqual([]);
  });
});

describe('Lamad manifest: signals', () => {
  const VALID_SUBSTRATE_SIGNALS = ['attention', 'compute', 'storage', 'bandwidth', 'resource'];

  it('every signal maps to a valid substrate signal', () => {
    const invalid: string[] = [];
    for (const [name, signal] of Object.entries(manifest.vocabulary.signals)) {
      if (!VALID_SUBSTRATE_SIGNALS.includes(signal.substrateSignal)) {
        invalid.push(`${name} -> ${signal.substrateSignal}`);
      }
    }
    expect(
      invalid,
      `Signals with invalid substrateSignal: ${invalid.join(', ')}`,
    ).toEqual([]);
  });

  it('every signal has an economicAction', () => {
    const missing: string[] = [];
    for (const [name, signal] of Object.entries(manifest.vocabulary.signals)) {
      if (!signal.economicAction) {
        missing.push(name);
      }
    }
    expect(missing, `Signals missing economicAction: ${missing.join(', ')}`).toEqual([]);
  });

  it('every signal has a resourceType', () => {
    const missing: string[] = [];
    for (const [name, signal] of Object.entries(manifest.vocabulary.signals)) {
      if (!signal.resourceType) {
        missing.push(name);
      }
    }
    expect(missing, `Signals missing resourceType: ${missing.join(', ')}`).toEqual([]);
  });
});

describe('Lamad manifest: relationships', () => {
  it('declares CONTAINS relationship', () => {
    expect(manifest.vocabulary.relationships).toHaveProperty('CONTAINS');
  });

  it('declares REFERENCES relationship', () => {
    expect(manifest.vocabulary.relationships).toHaveProperty('REFERENCES');
  });

  it('every relationship has sourceTypes and targetTypes', () => {
    const invalid: string[] = [];
    for (const [name, rel] of Object.entries(manifest.vocabulary.relationships)) {
      if (!Array.isArray(rel.sourceTypes) || rel.sourceTypes.length === 0) {
        invalid.push(`${name} missing sourceTypes`);
      }
      if (!Array.isArray(rel.targetTypes) || rel.targetTypes.length === 0) {
        invalid.push(`${name} missing targetTypes`);
      }
    }
    expect(invalid, `Relationships with missing type arrays: ${invalid.join(', ')}`).toEqual([]);
  });

  it('inverse relationships are symmetric', () => {
    const broken: string[] = [];
    for (const [name, rel] of Object.entries(manifest.vocabulary.relationships)) {
      if (rel.inverse) {
        const inverseRel = manifest.vocabulary.relationships[rel.inverse];
        if (!inverseRel) {
          broken.push(`${name}.inverse = ${rel.inverse}, but ${rel.inverse} does not exist`);
        } else if (inverseRel.inverse && inverseRel.inverse !== name) {
          broken.push(
            `${name}.inverse = ${rel.inverse}, but ${rel.inverse}.inverse = ${inverseRel.inverse} (expected ${name})`,
          );
        }
      }
    }
    expect(broken, `Broken inverse relationships: ${broken.join('; ')}`).toEqual([]);
  });
});
