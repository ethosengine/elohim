#!/usr/bin/env node
/**
 * Elohim Content MCP Server
 *
 * Provides tools for transforming raw docs into structured content graph seed data.
 * This is a prototype of the capabilities Elohim agents will provide to users.
 *
 * Pipeline: docs/content/ → Claude + MCP → data/lamad/ → seeder → Holochain
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';

// Tool implementations
import { readDoc, listDocs, searchDocs } from './tools/source-tools.js';
import { listSeeds, readSeed, writeSeed, deleteSeed, validateSeed } from './tools/seed-tools.js';
import { createConcept, createRelationship, queryGraph, getRelated, updateConcept, deleteConcept } from './tools/graph-tools.js';
import { createPath, createChapter, createModule, createSection, addToPath, removeFromPath, reorderPath, generatePath } from './tools/path-tools.js';
import { createQuiz, createAssessment, updateAssessment } from './tools/assessment-tools.js';

// Configuration - all genesis project content is in /genesis
const DOCS_DIR = process.env.DOCS_DIR || '/projects/elohim/genesis/docs/content';
const DATA_DIR = process.env.DATA_DIR || '/projects/elohim/genesis/data/lamad';

const server = new Server(
  {
    name: 'elohim-content',
    version: '1.0.0',
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

// Tool definitions
const tools = [
  // Source Reading Tools
  {
    name: 'read_doc',
    description: 'Read markdown/gherkin content from docs/content/. Returns file content with frontmatter parsed.',
    inputSchema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: 'Relative path within docs/content/ (e.g., "elohim-protocol/governance/epic.md")' },
      },
      required: ['path'],
    },
  },
  {
    name: 'list_docs',
    description: 'List available source documents. Can filter by epic, type, or pattern.',
    inputSchema: {
      type: 'object',
      properties: {
        epic: { type: 'string', description: 'Filter by epic (governance, value_scanner, etc.)' },
        pattern: { type: 'string', description: 'Glob pattern (e.g., "**/*.feature")' },
      },
    },
  },
  {
    name: 'search_docs',
    description: 'Search source documents for concepts/keywords.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search query' },
        epic: { type: 'string', description: 'Limit search to specific epic' },
      },
      required: ['query'],
    },
  },

  // Seed Data CRUD Tools
  {
    name: 'list_seeds',
    description: 'List existing seed data files (concepts, paths, assessments).',
    inputSchema: {
      type: 'object',
      properties: {
        type: { type: 'string', enum: ['concepts', 'paths', 'assessments', 'all'], description: 'Type of seed data to list' },
      },
    },
  },
  {
    name: 'read_seed',
    description: 'Read a specific seed file.',
    inputSchema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: 'Relative path within data/lamad/ (e.g., "paths/governance-intro.json")' },
      },
      required: ['path'],
    },
  },
  {
    name: 'write_seed',
    description: 'Write/update structured JSON to data/lamad/.',
    inputSchema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: 'Relative path within data/lamad/' },
        content: { type: 'object', description: 'JSON content to write' },
      },
      required: ['path', 'content'],
    },
  },
  {
    name: 'delete_seed',
    description: 'Remove a seed file.',
    inputSchema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: 'Relative path within data/lamad/' },
      },
      required: ['path'],
    },
  },
  {
    name: 'validate_seed',
    description: 'Validate JSON against Holochain entry schemas.',
    inputSchema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: 'Relative path to seed file' },
        schema: { type: 'string', enum: ['content', 'path', 'assessment', 'concept'], description: 'Schema to validate against' },
      },
      required: ['path', 'schema'],
    },
  },

  // Content Graph Tools
  {
    name: 'create_concept',
    description: 'Extract/create atomic concept from docs. Creates a node in the content graph.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string', description: 'Unique concept ID (kebab-case)' },
        title: { type: 'string', description: 'Display title' },
        content: { type: 'string', description: 'Concept content (markdown)' },
        sourceDoc: { type: 'string', description: 'Source document path this was derived from' },
        tags: { type: 'array', items: { type: 'string' }, description: 'Classification tags' },
      },
      required: ['id', 'title', 'content'],
    },
  },
  {
    name: 'create_relationship',
    description: 'Link concepts in the graph (prereq, related, extends, etc.).',
    inputSchema: {
      type: 'object',
      properties: {
        source: { type: 'string', description: 'Source concept ID' },
        target: { type: 'string', description: 'Target concept ID' },
        type: { type: 'string', enum: ['prereq', 'related', 'extends', 'exemplifies', 'contrasts'], description: 'Relationship type' },
      },
      required: ['source', 'target', 'type'],
    },
  },
  {
    name: 'query_graph',
    description: 'Find concepts by relationship, tags, or type.',
    inputSchema: {
      type: 'object',
      properties: {
        tags: { type: 'array', items: { type: 'string' }, description: 'Filter by tags' },
        hasRelationship: { type: 'string', description: 'Filter by relationship type' },
        relatedTo: { type: 'string', description: 'Find concepts related to this ID' },
      },
    },
  },
  {
    name: 'get_related',
    description: 'Get related concepts for a given node.',
    inputSchema: {
      type: 'object',
      properties: {
        conceptId: { type: 'string', description: 'Concept ID to find relations for' },
        relationshipType: { type: 'string', description: 'Filter by relationship type' },
        depth: { type: 'number', description: 'Traversal depth (default 1)' },
      },
      required: ['conceptId'],
    },
  },

  // Path Authoring Tools
  {
    name: 'create_path',
    description: 'Create ordered traversal through the content graph.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string', description: 'Path ID (kebab-case)' },
        title: { type: 'string', description: 'Display title' },
        description: { type: 'string', description: 'Path description' },
        difficulty: { type: 'string', enum: ['beginner', 'intermediate', 'advanced'] },
        conceptIds: { type: 'array', items: { type: 'string' }, description: 'Ordered list of concept IDs' },
      },
      required: ['id', 'title'],
    },
  },
  {
    name: 'generate_path',
    description: 'Auto-generate a path from a region of the content graph.',
    inputSchema: {
      type: 'object',
      properties: {
        startConcept: { type: 'string', description: 'Starting concept ID' },
        tags: { type: 'array', items: { type: 'string' }, description: 'Include concepts with these tags' },
        maxSteps: { type: 'number', description: 'Maximum path length' },
        followRelationships: { type: 'array', items: { type: 'string' }, description: 'Relationship types to follow' },
      },
      required: ['startConcept'],
    },
  },

  // Assessment Tools
  {
    name: 'create_quiz',
    description: 'Generate quiz from content concepts.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string', description: 'Quiz ID' },
        title: { type: 'string', description: 'Quiz title' },
        conceptIds: { type: 'array', items: { type: 'string' }, description: 'Concepts to quiz on' },
        questionCount: { type: 'number', description: 'Number of questions to generate' },
      },
      required: ['id', 'title', 'conceptIds'],
    },
  },
  {
    name: 'create_assessment',
    description: 'Build assessment instrument from content.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string', description: 'Assessment ID' },
        title: { type: 'string', description: 'Assessment title' },
        type: { type: 'string', enum: ['diagnostic', 'formative', 'summative'] },
        conceptIds: { type: 'array', items: { type: 'string' }, description: 'Concepts to assess' },
      },
      required: ['id', 'title', 'type'],
    },
  },
];

// Handle tool listing
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return { tools };
});

// Type helper for extracting args
type ToolArgs = Record<string, unknown>;

function getString(args: ToolArgs | undefined, key: string): string {
  const value = args?.[key];
  if (typeof value !== 'string') {
    throw new Error(`Missing or invalid argument: ${key}`);
  }
  return value;
}

function getOptionalString(args: ToolArgs | undefined, key: string): string | undefined {
  const value = args?.[key];
  return typeof value === 'string' ? value : undefined;
}

function getOptionalNumber(args: ToolArgs | undefined, key: string): number | undefined {
  const value = args?.[key];
  return typeof value === 'number' ? value : undefined;
}

function getStringArray(args: ToolArgs | undefined, key: string): string[] | undefined {
  const value = args?.[key];
  return Array.isArray(value) ? value.filter((v): v is string => typeof v === 'string') : undefined;
}

// Handle tool calls
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    let result;
    switch (name) {
      // Source tools
      case 'read_doc':
        result = await readDoc(DOCS_DIR, getString(args, 'path'));
        break;
      case 'list_docs':
        result = await listDocs(DOCS_DIR, getOptionalString(args, 'epic'), getOptionalString(args, 'pattern'));
        break;
      case 'search_docs':
        result = await searchDocs(DOCS_DIR, getString(args, 'query'), getOptionalString(args, 'epic'));
        break;

      // Seed tools
      case 'list_seeds':
        result = await listSeeds(DATA_DIR, getOptionalString(args, 'type'));
        break;
      case 'read_seed':
        result = await readSeed(DATA_DIR, getString(args, 'path'));
        break;
      case 'write_seed':
        result = await writeSeed(DATA_DIR, getString(args, 'path'), args?.content);
        break;
      case 'delete_seed':
        result = await deleteSeed(DATA_DIR, getString(args, 'path'));
        break;
      case 'validate_seed':
        result = await validateSeed(DATA_DIR, getString(args, 'path'), getString(args, 'schema'));
        break;

      // Graph tools
      case 'create_concept':
        result = await createConcept(DATA_DIR, {
          id: getString(args, 'id'),
          title: getString(args, 'title'),
          content: getString(args, 'content'),
          sourceDoc: getOptionalString(args, 'sourceDoc'),
          tags: getStringArray(args, 'tags'),
        });
        break;
      case 'create_relationship':
        result = await createRelationship(
          DATA_DIR,
          getString(args, 'source'),
          getString(args, 'target'),
          getString(args, 'type') as 'prereq' | 'related' | 'extends' | 'exemplifies' | 'contrasts'
        );
        break;
      case 'query_graph':
        result = await queryGraph(DATA_DIR, {
          tags: getStringArray(args, 'tags'),
          hasRelationship: getOptionalString(args, 'hasRelationship'),
          relatedTo: getOptionalString(args, 'relatedTo'),
        });
        break;
      case 'get_related':
        result = await getRelated(
          DATA_DIR,
          getString(args, 'conceptId'),
          getOptionalString(args, 'relationshipType'),
          getOptionalNumber(args, 'depth')
        );
        break;
      case 'update_concept':
        result = await updateConcept(DATA_DIR, getString(args, 'id'), {
          title: getOptionalString(args, 'title'),
          content: getOptionalString(args, 'content'),
          sourceDoc: getOptionalString(args, 'sourceDoc'),
          tags: getStringArray(args, 'tags'),
        });
        break;
      case 'delete_concept':
        result = await deleteConcept(DATA_DIR, getString(args, 'id'));
        break;

      // Path tools
      case 'create_path':
        result = await createPath(DATA_DIR, {
          id: getString(args, 'id'),
          title: getString(args, 'title'),
          description: getOptionalString(args, 'description'),
          difficulty: getOptionalString(args, 'difficulty') as 'beginner' | 'intermediate' | 'advanced' | undefined,
          conceptIds: getStringArray(args, 'conceptIds'),
        });
        break;
      case 'create_chapter':
        result = await createChapter(DATA_DIR, {
          id: getString(args, 'id'),
          title: getString(args, 'title'),
          description: getOptionalString(args, 'description'),
          pathId: getOptionalString(args, 'pathId'),
        });
        break;
      case 'create_module':
        result = await createModule(DATA_DIR, {
          id: getString(args, 'id'),
          title: getString(args, 'title'),
          description: getOptionalString(args, 'description'),
          pathId: getOptionalString(args, 'pathId'),
          chapterId: getOptionalString(args, 'chapterId'),
        });
        break;
      case 'create_section':
        result = await createSection(DATA_DIR, {
          id: getString(args, 'id'),
          title: getString(args, 'title'),
          description: getOptionalString(args, 'description'),
          conceptIds: getStringArray(args, 'conceptIds'),
          pathId: getOptionalString(args, 'pathId'),
          chapterId: getOptionalString(args, 'chapterId'),
          moduleId: getOptionalString(args, 'moduleId'),
        });
        break;
      case 'add_to_path':
        result = await addToPath(
          DATA_DIR,
          getString(args, 'pathId'),
          getString(args, 'conceptId'),
          getOptionalNumber(args, 'position')
        );
        break;
      case 'remove_from_path':
        result = await removeFromPath(DATA_DIR, getString(args, 'pathId'), getString(args, 'conceptId'));
        break;
      case 'reorder_path':
        result = await reorderPath(
          DATA_DIR,
          getString(args, 'pathId'),
          getStringArray(args, 'conceptIds') || []
        );
        break;
      case 'generate_path':
        result = await generatePath(DATA_DIR, {
          startConcept: getString(args, 'startConcept'),
          tags: getStringArray(args, 'tags'),
          maxSteps: getOptionalNumber(args, 'maxSteps'),
          followRelationships: getStringArray(args, 'followRelationships'),
        });
        break;

      // Assessment tools
      case 'create_quiz':
        result = await createQuiz(DATA_DIR, {
          id: getString(args, 'id'),
          title: getString(args, 'title'),
          conceptIds: getStringArray(args, 'conceptIds') || [],
          questionCount: getOptionalNumber(args, 'questionCount'),
        });
        break;
      case 'create_assessment':
        result = await createAssessment(DATA_DIR, {
          id: getString(args, 'id'),
          title: getString(args, 'title'),
          type: getString(args, 'type') as 'diagnostic' | 'formative' | 'summative',
          conceptIds: getStringArray(args, 'conceptIds'),
          description: getOptionalString(args, 'description'),
          passingScore: getOptionalNumber(args, 'passingScore'),
          timeLimit: getOptionalNumber(args, 'timeLimit'),
        });
        break;
      case 'update_assessment':
        result = await updateAssessment(DATA_DIR, getString(args, 'id'), {
          title: getOptionalString(args, 'title'),
          description: getOptionalString(args, 'description'),
          type: getOptionalString(args, 'type') as 'diagnostic' | 'formative' | 'summative' | undefined,
          conceptIds: getStringArray(args, 'conceptIds'),
          passingScore: getOptionalNumber(args, 'passingScore'),
          timeLimit: getOptionalNumber(args, 'timeLimit'),
        });
        break;

      default:
        throw new Error(`Unknown tool: ${name}`);
    }

    return {
      content: [{ type: 'text', text: JSON.stringify(result, null, 2) }],
    };
  } catch (error) {
    return {
      content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : String(error)}` }],
      isError: true,
    };
  }
});

// Start server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('Elohim Content MCP server running on stdio');
}

main().catch(console.error);
