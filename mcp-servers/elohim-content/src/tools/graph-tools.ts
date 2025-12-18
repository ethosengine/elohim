/**
 * Content Graph Tools
 *
 * Tools for managing the underlying knowledge graph of concepts and relationships.
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import { glob } from 'glob';
import { conceptSchema, type Concept, type Relationship } from '../schemas/index.js';

const CONCEPTS_DIR = 'content';

interface ConceptInput {
  id: string;
  title: string;
  content: string;
  sourceDoc?: string;
  tags?: string[];
  contentFormat?: 'markdown' | 'html' | 'plain';
}

interface QueryOptions {
  tags?: string[];
  hasRelationship?: string;
  relatedTo?: string;
}

interface RelatedResult {
  concept: Concept;
  relationship: string;
  depth: number;
}

/**
 * Load all concepts from disk
 */
async function loadAllConcepts(dataDir: string): Promise<Map<string, Concept>> {
  const conceptsPath = path.join(dataDir, CONCEPTS_DIR);
  const concepts = new Map<string, Concept>();

  try {
    const files = await glob('**/*.json', { cwd: conceptsPath, nodir: true });

    for (const file of files) {
      try {
        const fullPath = path.join(conceptsPath, file);
        const content = await fs.readFile(fullPath, 'utf-8');
        const data = JSON.parse(content);
        const parsed = conceptSchema.safeParse(data);
        if (parsed.success) {
          concepts.set(parsed.data.id, parsed.data);
        }
      } catch {
        // Skip invalid files
      }
    }
  } catch {
    // Directory doesn't exist yet
  }

  return concepts;
}

/**
 * Create a new concept in the content graph
 */
export async function createConcept(
  dataDir: string,
  input: ConceptInput
): Promise<{ success: boolean; path: string; concept: Concept }> {
  const concept: Concept = {
    id: input.id,
    title: input.title,
    content: input.content,
    contentFormat: input.contentFormat || 'markdown',
    sourceDoc: input.sourceDoc,
    tags: input.tags || [],
    relationships: [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  // Validate
  const result = conceptSchema.safeParse(concept);
  if (!result.success) {
    throw new Error(`Invalid concept: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  // Write to disk
  const relativePath = path.join(CONCEPTS_DIR, `${input.id}.json`);
  const fullPath = path.join(dataDir, relativePath);

  await fs.mkdir(path.dirname(fullPath), { recursive: true });
  await fs.writeFile(fullPath, JSON.stringify(concept, null, 2), 'utf-8');

  return { success: true, path: relativePath, concept };
}

/**
 * Create a relationship between two concepts
 */
export async function createRelationship(
  dataDir: string,
  source: string,
  target: string,
  type: Relationship['type']
): Promise<{ success: boolean; relationship: Relationship }> {
  // Load source concept
  const sourcePath = path.join(dataDir, CONCEPTS_DIR, `${source}.json`);

  let concept: Concept;
  try {
    const content = await fs.readFile(sourcePath, 'utf-8');
    concept = JSON.parse(content);
  } catch {
    throw new Error(`Source concept not found: ${source}`);
  }

  // Verify target exists
  const targetPath = path.join(dataDir, CONCEPTS_DIR, `${target}.json`);
  try {
    await fs.access(targetPath);
  } catch {
    throw new Error(`Target concept not found: ${target}`);
  }

  // Check if relationship already exists
  const existing = concept.relationships?.find(
    r => r.target === target && r.type === type
  );
  if (existing) {
    return { success: true, relationship: { source, target, type } };
  }

  // Add relationship
  if (!concept.relationships) {
    concept.relationships = [];
  }
  concept.relationships.push({ target, type });
  concept.updatedAt = new Date().toISOString();

  // Write back
  await fs.writeFile(sourcePath, JSON.stringify(concept, null, 2), 'utf-8');

  return { success: true, relationship: { source, target, type } };
}

/**
 * Query the content graph
 */
export async function queryGraph(
  dataDir: string,
  options: QueryOptions
): Promise<Concept[]> {
  const concepts = await loadAllConcepts(dataDir);
  let results = Array.from(concepts.values());

  // Filter by tags
  if (options.tags && options.tags.length > 0) {
    results = results.filter(c =>
      options.tags!.some(tag => c.tags?.includes(tag))
    );
  }

  // Filter by relationship type
  if (options.hasRelationship) {
    results = results.filter(c =>
      c.relationships?.some(r => r.type === options.hasRelationship)
    );
  }

  // Filter by related to specific concept
  if (options.relatedTo) {
    const relatedIds = new Set<string>();

    // Find outgoing relationships from the target
    const targetConcept = concepts.get(options.relatedTo);
    if (targetConcept) {
      targetConcept.relationships?.forEach(r => relatedIds.add(r.target));
    }

    // Find incoming relationships to the target
    for (const concept of concepts.values()) {
      if (concept.relationships?.some(r => r.target === options.relatedTo)) {
        relatedIds.add(concept.id);
      }
    }

    results = results.filter(c => relatedIds.has(c.id));
  }

  return results;
}

/**
 * Get related concepts for a given node
 */
export async function getRelated(
  dataDir: string,
  conceptId: string,
  relationshipType?: string,
  depth: number = 1
): Promise<RelatedResult[]> {
  const concepts = await loadAllConcepts(dataDir);
  const results: RelatedResult[] = [];
  const visited = new Set<string>();

  function traverse(id: string, currentDepth: number) {
    if (currentDepth > depth || visited.has(id)) return;
    visited.add(id);

    const concept = concepts.get(id);
    if (!concept) return;

    // Outgoing relationships
    for (const rel of concept.relationships || []) {
      if (relationshipType && rel.type !== relationshipType) continue;

      const targetConcept = concepts.get(rel.target);
      if (targetConcept && !visited.has(rel.target)) {
        results.push({
          concept: targetConcept,
          relationship: rel.type,
          depth: currentDepth,
        });
        traverse(rel.target, currentDepth + 1);
      }
    }

    // Incoming relationships
    for (const [otherId, otherConcept] of concepts) {
      if (visited.has(otherId)) continue;

      const incomingRel = otherConcept.relationships?.find(
        r => r.target === id && (!relationshipType || r.type === relationshipType)
      );

      if (incomingRel) {
        results.push({
          concept: otherConcept,
          relationship: `inverse:${incomingRel.type}`,
          depth: currentDepth,
        });
        traverse(otherId, currentDepth + 1);
      }
    }
  }

  traverse(conceptId, 1);
  return results;
}

/**
 * Update an existing concept
 */
export async function updateConcept(
  dataDir: string,
  id: string,
  updates: Partial<ConceptInput>
): Promise<{ success: boolean; concept: Concept }> {
  const conceptPath = path.join(dataDir, CONCEPTS_DIR, `${id}.json`);

  let concept: Concept;
  try {
    const content = await fs.readFile(conceptPath, 'utf-8');
    concept = JSON.parse(content);
  } catch {
    throw new Error(`Concept not found: ${id}`);
  }

  // Apply updates
  if (updates.title !== undefined) concept.title = updates.title;
  if (updates.content !== undefined) concept.content = updates.content;
  if (updates.contentFormat !== undefined) concept.contentFormat = updates.contentFormat;
  if (updates.sourceDoc !== undefined) concept.sourceDoc = updates.sourceDoc;
  if (updates.tags !== undefined) concept.tags = updates.tags;
  concept.updatedAt = new Date().toISOString();

  // Validate
  const result = conceptSchema.safeParse(concept);
  if (!result.success) {
    throw new Error(`Invalid concept: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  // Write back
  await fs.writeFile(conceptPath, JSON.stringify(concept, null, 2), 'utf-8');

  return { success: true, concept };
}

/**
 * Delete a concept and its relationships
 */
export async function deleteConcept(
  dataDir: string,
  id: string
): Promise<{ success: boolean; removedRelationships: number }> {
  const conceptPath = path.join(dataDir, CONCEPTS_DIR, `${id}.json`);

  // Verify it exists
  try {
    await fs.access(conceptPath);
  } catch {
    throw new Error(`Concept not found: ${id}`);
  }

  // Remove incoming relationships from other concepts
  const concepts = await loadAllConcepts(dataDir);
  let removedRelationships = 0;

  for (const [otherId, otherConcept] of concepts) {
    if (otherId === id) continue;

    const originalLength = otherConcept.relationships?.length || 0;
    otherConcept.relationships = otherConcept.relationships?.filter(
      r => r.target !== id
    ) || [];

    if (otherConcept.relationships.length < originalLength) {
      removedRelationships += originalLength - otherConcept.relationships.length;
      otherConcept.updatedAt = new Date().toISOString();

      const otherPath = path.join(dataDir, CONCEPTS_DIR, `${otherId}.json`);
      await fs.writeFile(otherPath, JSON.stringify(otherConcept, null, 2), 'utf-8');
    }
  }

  // Delete the concept file
  await fs.unlink(conceptPath);

  return { success: true, removedRelationships };
}
