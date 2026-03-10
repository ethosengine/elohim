/**
 * Seed Data CRUD Tools
 *
 * Tools for managing structured JSON in data/lamad/
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import { glob } from 'glob';
import { contentSchema, pathSchema, assessmentSchema, conceptSchema } from '../schemas/index.js';

export interface SeedListItem {
  path: string;
  type: string;
  id?: string;
  title?: string;
}

/**
 * List seed data files
 */
export async function listSeeds(
  dataDir: string,
  type?: string
): Promise<SeedListItem[]> {
  let patterns: string[];

  switch (type) {
    case 'concepts':
      patterns = ['concepts/**/*.json', 'content/**/*.json'];
      break;
    case 'paths':
      patterns = ['paths/**/*.json'];
      break;
    case 'assessments':
      patterns = ['assessments/**/*.json'];
      break;
    default:
      patterns = ['**/*.json'];
  }

  const results: SeedListItem[] = [];

  for (const pattern of patterns) {
    const files = await glob(pattern, {
      cwd: dataDir,
      nodir: true,
    });

    for (const file of files) {
      try {
        const fullPath = path.join(dataDir, file);
        const content = await fs.readFile(fullPath, 'utf-8');
        const data = JSON.parse(content);

        // Determine type from path
        let seedType = 'unknown';
        if (file.startsWith('concepts/') || file.startsWith('content/')) {
          seedType = 'concept';
        } else if (file.startsWith('paths/')) {
          seedType = 'path';
        } else if (file.startsWith('assessments/')) {
          seedType = 'assessment';
        }

        results.push({
          path: file,
          type: seedType,
          id: data.id,
          title: data.title,
        });
      } catch {
        results.push({ path: file, type: 'unknown' });
      }
    }
  }

  return results;
}

/**
 * Read a seed file
 */
export async function readSeed(
  dataDir: string,
  relativePath: string
): Promise<unknown> {
  const fullPath = path.join(dataDir, relativePath);

  // Security check
  const resolved = path.resolve(fullPath);
  if (!resolved.startsWith(path.resolve(dataDir))) {
    throw new Error('Path traversal not allowed');
  }

  const content = await fs.readFile(fullPath, 'utf-8');
  return JSON.parse(content);
}

/**
 * Write a seed file
 */
export async function writeSeed(
  dataDir: string,
  relativePath: string,
  content: unknown
): Promise<{ success: boolean; path: string }> {
  const fullPath = path.join(dataDir, relativePath);

  // Security check
  const resolved = path.resolve(fullPath);
  if (!resolved.startsWith(path.resolve(dataDir))) {
    throw new Error('Path traversal not allowed');
  }

  // Ensure directory exists
  await fs.mkdir(path.dirname(fullPath), { recursive: true });

  // Write with pretty formatting
  await fs.writeFile(fullPath, JSON.stringify(content, null, 2), 'utf-8');

  return { success: true, path: relativePath };
}

/**
 * Delete a seed file
 */
export async function deleteSeed(
  dataDir: string,
  relativePath: string
): Promise<{ success: boolean; path: string }> {
  const fullPath = path.join(dataDir, relativePath);

  // Security check
  const resolved = path.resolve(fullPath);
  if (!resolved.startsWith(path.resolve(dataDir))) {
    throw new Error('Path traversal not allowed');
  }

  await fs.unlink(fullPath);

  return { success: true, path: relativePath };
}

/**
 * Validate a seed file against schema
 */
export async function validateSeed(
  dataDir: string,
  relativePath: string,
  schemaType: string
): Promise<{ valid: boolean; errors?: string[] }> {
  const content = await readSeed(dataDir, relativePath);

  let schema;
  switch (schemaType) {
    case 'content':
      schema = contentSchema;
      break;
    case 'path':
      schema = pathSchema;
      break;
    case 'assessment':
      schema = assessmentSchema;
      break;
    case 'concept':
      schema = conceptSchema;
      break;
    default:
      throw new Error(`Unknown schema type: ${schemaType}`);
  }

  const result = schema.safeParse(content);

  if (result.success) {
    return { valid: true };
  } else {
    return {
      valid: false,
      errors: result.error.errors.map(e => `${e.path.join('.')}: ${e.message}`),
    };
  }
}
