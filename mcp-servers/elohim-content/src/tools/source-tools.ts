/**
 * Source Reading Tools
 *
 * Tools for reading raw docs from docs/content/
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import matter from 'gray-matter';
import { glob } from 'glob';

export interface DocContent {
  path: string;
  frontmatter: Record<string, unknown>;
  content: string;
  raw: string;
}

export interface DocListItem {
  path: string;
  title?: string;
  epic?: string;
  type?: string;
}

export interface SearchResult {
  path: string;
  title?: string;
  matches: string[];
  score: number;
}

/**
 * Read a single document from docs/content/
 */
export async function readDoc(docsDir: string, relativePath: string): Promise<DocContent> {
  const fullPath = path.join(docsDir, relativePath);

  // Security check - ensure path is within docsDir
  const resolved = path.resolve(fullPath);
  if (!resolved.startsWith(path.resolve(docsDir))) {
    throw new Error('Path traversal not allowed');
  }

  const raw = await fs.readFile(fullPath, 'utf-8');
  const { data: frontmatter, content } = matter(raw);

  return {
    path: relativePath,
    frontmatter,
    content,
    raw,
  };
}

/**
 * List documents in docs/content/
 */
export async function listDocs(
  docsDir: string,
  epic?: string,
  pattern?: string
): Promise<DocListItem[]> {
  // Build glob pattern
  let searchPattern = pattern || '**/*.{md,feature}';
  if (epic) {
    searchPattern = `elohim-protocol/${epic}/**/*.{md,feature}`;
  }

  const files = await glob(searchPattern, {
    cwd: docsDir,
    nodir: true,
  });

  const results: DocListItem[] = [];

  for (const file of files) {
    try {
      const fullPath = path.join(docsDir, file);
      const raw = await fs.readFile(fullPath, 'utf-8');
      const { data: frontmatter } = matter(raw);

      // Extract title from frontmatter or first heading
      let title = frontmatter.title as string | undefined;
      if (!title) {
        const headingMatch = raw.match(/^#\s+(.+)$/m);
        title = headingMatch?.[1];
      }

      // Extract epic from path
      const pathParts = file.split('/');
      const fileEpic = pathParts[1]; // e.g., elohim-protocol/governance/...

      results.push({
        path: file,
        title,
        epic: fileEpic,
        type: path.extname(file) === '.feature' ? 'scenario' : 'markdown',
      });
    } catch {
      // Skip files that can't be read
      results.push({ path: file });
    }
  }

  return results;
}

/**
 * Search documents for keywords/concepts
 */
export async function searchDocs(
  docsDir: string,
  query: string,
  epic?: string
): Promise<SearchResult[]> {
  // Get all docs
  const docs = await listDocs(docsDir, epic);

  const results: SearchResult[] = [];
  const queryLower = query.toLowerCase();
  const queryTerms = queryLower.split(/\s+/);

  for (const doc of docs) {
    try {
      const { content, frontmatter } = await readDoc(docsDir, doc.path);
      const contentLower = content.toLowerCase();

      // Find matches
      const matches: string[] = [];
      let score = 0;

      for (const term of queryTerms) {
        // Check title
        if (doc.title?.toLowerCase().includes(term)) {
          score += 10;
          matches.push(`Title: "${doc.title}"`);
        }

        // Check frontmatter tags
        const tags = (frontmatter.tags as string[]) || [];
        for (const tag of tags) {
          if (tag.toLowerCase().includes(term)) {
            score += 5;
            matches.push(`Tag: ${tag}`);
          }
        }

        // Check content (find context around matches)
        let pos = 0;
        while ((pos = contentLower.indexOf(term, pos)) !== -1) {
          score += 1;
          // Extract context (50 chars before and after)
          const start = Math.max(0, pos - 50);
          const end = Math.min(content.length, pos + term.length + 50);
          const context = content.slice(start, end).replace(/\n/g, ' ').trim();
          if (matches.length < 5) { // Limit matches shown
            matches.push(`...${context}...`);
          }
          pos += term.length;
        }
      }

      if (score > 0) {
        results.push({
          path: doc.path,
          title: doc.title,
          matches,
          score,
        });
      }
    } catch {
      // Skip files that can't be read
    }
  }

  // Sort by score descending
  results.sort((a, b) => b.score - a.score);

  return results.slice(0, 20); // Return top 20
}
