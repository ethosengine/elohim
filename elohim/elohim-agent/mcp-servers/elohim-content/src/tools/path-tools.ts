/**
 * Path Authoring Tools
 *
 * Tools for creating paths as ordered traversals/projections over the content graph.
 * Paths are views that organize concepts into hierarchies (chapters, modules, sections).
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import { glob } from 'glob';
import {
  pathSchema,
  chapterSchema,
  moduleSchema,
  sectionSchema,
  conceptSchema,
  type Path,
  type Chapter,
  type Module,
  type Section,
  type Concept,
} from '../schemas/index.js';

const PATHS_DIR = 'paths';
const CONCEPTS_DIR = 'content';

interface PathInput {
  id: string;
  title: string;
  description?: string;
  difficulty?: 'beginner' | 'intermediate' | 'advanced';
  conceptIds?: string[];
}

interface ChapterInput {
  id: string;
  title: string;
  description?: string;
  order?: number;
}

interface ModuleInput {
  id: string;
  title: string;
  description?: string;
  order?: number;
}

interface SectionInput {
  id: string;
  title: string;
  description?: string;
  conceptIds?: string[];
  order?: number;
}

interface GeneratePathOptions {
  startConcept: string;
  tags?: string[];
  maxSteps?: number;
  followRelationships?: string[];
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
 * Load a path from disk
 */
async function loadPath(dataDir: string, pathId: string): Promise<Path> {
  const pathFile = path.join(dataDir, PATHS_DIR, `${pathId}.json`);

  try {
    const content = await fs.readFile(pathFile, 'utf-8');
    return JSON.parse(content);
  } catch {
    throw new Error(`Path not found: ${pathId}`);
  }
}

/**
 * Save a path to disk
 */
async function savePath(dataDir: string, pathData: Path): Promise<string> {
  const relativePath = path.join(PATHS_DIR, `${pathData.id}.json`);
  const fullPath = path.join(dataDir, relativePath);

  await fs.mkdir(path.dirname(fullPath), { recursive: true });
  await fs.writeFile(fullPath, JSON.stringify(pathData, null, 2), 'utf-8');

  return relativePath;
}

/**
 * Create a new learning path
 */
export async function createPath(
  dataDir: string,
  input: PathInput
): Promise<{ success: boolean; path: string; data: Path }> {
  const pathData: Path = {
    id: input.id,
    title: input.title,
    description: input.description,
    difficulty: input.difficulty,
    chapters: [],
    conceptIds: input.conceptIds,
    tags: [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  // Validate
  const result = pathSchema.safeParse(pathData);
  if (!result.success) {
    throw new Error(`Invalid path: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  const relativePath = await savePath(dataDir, pathData);
  return { success: true, path: relativePath, data: pathData };
}

/**
 * Create a chapter and optionally add it to a path
 */
export async function createChapter(
  dataDir: string,
  input: ChapterInput & { pathId?: string }
): Promise<{ success: boolean; chapter: Chapter }> {
  const chapter: Chapter = {
    id: input.id,
    title: input.title,
    description: input.description,
    modules: [],
    order: input.order,
  };

  // Validate
  const result = chapterSchema.safeParse(chapter);
  if (!result.success) {
    throw new Error(`Invalid chapter: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  // If pathId provided, add to path
  if (input.pathId) {
    const pathData = await loadPath(dataDir, input.pathId);
    pathData.chapters.push(chapter);
    pathData.updatedAt = new Date().toISOString();
    await savePath(dataDir, pathData);
  }

  return { success: true, chapter };
}

/**
 * Create a module and optionally add it to a chapter
 */
export async function createModule(
  dataDir: string,
  input: ModuleInput & { pathId?: string; chapterId?: string }
): Promise<{ success: boolean; module: Module }> {
  const module: Module = {
    id: input.id,
    title: input.title,
    description: input.description,
    sections: [],
    order: input.order,
  };

  // Validate
  const result = moduleSchema.safeParse(module);
  if (!result.success) {
    throw new Error(`Invalid module: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  // If pathId and chapterId provided, add to chapter
  if (input.pathId && input.chapterId) {
    const pathData = await loadPath(dataDir, input.pathId);
    const chapter = pathData.chapters.find(c => c.id === input.chapterId);
    if (!chapter) {
      throw new Error(`Chapter not found: ${input.chapterId}`);
    }
    chapter.modules.push(module);
    pathData.updatedAt = new Date().toISOString();
    await savePath(dataDir, pathData);
  }

  return { success: true, module };
}

/**
 * Create a section and optionally add it to a module
 */
export async function createSection(
  dataDir: string,
  input: SectionInput & { pathId?: string; chapterId?: string; moduleId?: string }
): Promise<{ success: boolean; section: Section }> {
  const section: Section = {
    id: input.id,
    title: input.title,
    description: input.description,
    conceptIds: input.conceptIds || [],
    order: input.order,
  };

  // Validate
  const result = sectionSchema.safeParse(section);
  if (!result.success) {
    throw new Error(`Invalid section: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  // If all IDs provided, add to module
  if (input.pathId && input.chapterId && input.moduleId) {
    const pathData = await loadPath(dataDir, input.pathId);
    const chapter = pathData.chapters.find(c => c.id === input.chapterId);
    if (!chapter) {
      throw new Error(`Chapter not found: ${input.chapterId}`);
    }
    const module = chapter.modules.find(m => m.id === input.moduleId);
    if (!module) {
      throw new Error(`Module not found: ${input.moduleId}`);
    }
    module.sections.push(section);
    pathData.updatedAt = new Date().toISOString();
    await savePath(dataDir, pathData);
  }

  return { success: true, section };
}

/**
 * Add a concept to a path (flat list)
 */
export async function addToPath(
  dataDir: string,
  pathId: string,
  conceptId: string,
  position?: number
): Promise<{ success: boolean; path: Path }> {
  const pathData = await loadPath(dataDir, pathId);

  if (!pathData.conceptIds) {
    pathData.conceptIds = [];
  }

  // Verify concept exists
  const conceptPath = path.join(dataDir, CONCEPTS_DIR, `${conceptId}.json`);
  try {
    await fs.access(conceptPath);
  } catch {
    throw new Error(`Concept not found: ${conceptId}`);
  }

  // Add at position or end
  if (position !== undefined && position >= 0 && position < pathData.conceptIds.length) {
    pathData.conceptIds.splice(position, 0, conceptId);
  } else {
    pathData.conceptIds.push(conceptId);
  }

  pathData.updatedAt = new Date().toISOString();
  await savePath(dataDir, pathData);

  return { success: true, path: pathData };
}

/**
 * Remove a concept from a path
 */
export async function removeFromPath(
  dataDir: string,
  pathId: string,
  conceptId: string
): Promise<{ success: boolean; path: Path }> {
  const pathData = await loadPath(dataDir, pathId);

  if (!pathData.conceptIds) {
    return { success: true, path: pathData };
  }

  pathData.conceptIds = pathData.conceptIds.filter(id => id !== conceptId);
  pathData.updatedAt = new Date().toISOString();
  await savePath(dataDir, pathData);

  return { success: true, path: pathData };
}

/**
 * Reorder concepts in a path
 */
export async function reorderPath(
  dataDir: string,
  pathId: string,
  conceptIds: string[]
): Promise<{ success: boolean; path: Path }> {
  const pathData = await loadPath(dataDir, pathId);

  pathData.conceptIds = conceptIds;
  pathData.updatedAt = new Date().toISOString();
  await savePath(dataDir, pathData);

  return { success: true, path: pathData };
}

/**
 * Auto-generate a path from a region of the content graph
 */
export async function generatePath(
  dataDir: string,
  options: GeneratePathOptions
): Promise<{ success: boolean; path: Path; conceptIds: string[] }> {
  const concepts = await loadAllConcepts(dataDir);
  const startConcept = concepts.get(options.startConcept);

  if (!startConcept) {
    throw new Error(`Start concept not found: ${options.startConcept}`);
  }

  const maxSteps = options.maxSteps || 10;
  const followTypes = options.followRelationships || ['prereq', 'related', 'extends'];
  const tagFilter = new Set(options.tags || []);

  const orderedIds: string[] = [options.startConcept];
  const visited = new Set<string>([options.startConcept]);

  // BFS to find related concepts
  const queue: string[] = [options.startConcept];

  while (queue.length > 0 && orderedIds.length < maxSteps) {
    const currentId = queue.shift()!;
    const current = concepts.get(currentId);

    if (!current) continue;

    // Follow relationships
    for (const rel of current.relationships || []) {
      if (!followTypes.includes(rel.type)) continue;
      if (visited.has(rel.target)) continue;

      const targetConcept = concepts.get(rel.target);
      if (!targetConcept) continue;

      // Check tag filter
      if (tagFilter.size > 0) {
        const hasMatchingTag = targetConcept.tags?.some(t => tagFilter.has(t));
        if (!hasMatchingTag) continue;
      }

      visited.add(rel.target);
      orderedIds.push(rel.target);
      queue.push(rel.target);

      if (orderedIds.length >= maxSteps) break;
    }
  }

  // Create a generated path
  const generatedPath: Path = {
    id: `generated-${Date.now()}`,
    title: `Path from ${startConcept.title}`,
    description: `Auto-generated path starting from "${startConcept.title}"`,
    conceptIds: orderedIds,
    chapters: [],
    tags: options.tags || [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  const relativePath = await savePath(dataDir, generatedPath);

  return {
    success: true,
    path: generatedPath,
    conceptIds: orderedIds,
  };
}
