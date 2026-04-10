import { describe, it, expect } from 'vitest';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  validateCollectivesFile,
  validateReferentialIntegrity,
  type CollectiveValidationResult,
} from '../validate-collectives.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const COLLECTIVES_FILE = resolve(__dirname, '../../../data/collectives/collectives.json');

describe('validateCollectivesFile()', () => {
  it('should validate the actual collectives.json without errors', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    if (result.errors.length > 0) {
      console.error('Validation errors:', result.errors);
    }
    expect(result.errors).toEqual([]);
    expect(result.collectiveCount).toBeGreaterThanOrEqual(46);
  });

  it('should validate all reach values are protocol-canonical', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    expect(result.errors.filter(e => e.includes('reach'))).toEqual([]);
  });

  it('should validate all governanceLayer values are canonical', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    expect(result.errors.filter(e => e.includes('governanceLayer'))).toEqual([]);
  });
});

describe('validateReferentialIntegrity()', () => {
  it('should have no dangling constitutionalParentId references', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    const integrityErrors = validateReferentialIntegrity(result);
    const parentErrors = integrityErrors.filter(e => e.includes('constitutionalParentId'));
    if (parentErrors.length > 0) {
      console.error('Dangling parent refs:', parentErrors);
    }
    expect(parentErrors).toEqual([]);
  });

  it('should have no dangling relationship from/to references', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    const integrityErrors = validateReferentialIntegrity(result);
    const relErrors = integrityErrors.filter(e => e.includes('relationship'));
    if (relErrors.length > 0) {
      console.error('Dangling relationship refs:', relErrors);
    }
    expect(relErrors).toEqual([]);
  });

  it('should have no circular constitutionalParentId chains', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    const integrityErrors = validateReferentialIntegrity(result);
    const circularErrors = integrityErrors.filter(e => e.includes('circular'));
    expect(circularErrors).toEqual([]);
  });

  it('should require domainOverlap on peers-with relationships', () => {
    const result = validateCollectivesFile(COLLECTIVES_FILE);
    const integrityErrors = validateReferentialIntegrity(result);
    const peerErrors = integrityErrors.filter(e => e.includes('domainOverlap'));
    expect(peerErrors).toEqual([]);
  });
});
