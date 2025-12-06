/**
 * Elohim Service - Content Intelligence Module
 *
 * Provides autonomous content import, analysis, and path generation
 * for the Elohim Protocol lamad learning platform.
 *
 * Main capabilities:
 * - Import: Transform raw markdown/gherkin into ContentNode models
 * - Analyze: Assess content quality and identify gaps
 * - Generate: Create learning paths from content graph
 *
 * Architecture:
 * - Source layer: Raw imported files preserved with provenance
 * - Knowledge layer: Derived concepts, paths, relationships
 *
 * Entry points:
 * - CLI: `npx ts-node src/cli/import.ts import`
 * - Skill: `executeSkill({ action: 'import' })`
 * - Programmatic: `runImportPipeline(options)`
 */

// Models
export * from './models/content-node.model';
export * from './models/path-metadata.model';
export * from './models/import-context.model';
export * from './models/manifest.model';

// Parsers
export * from './parsers/path-metadata-parser';
export * from './parsers/markdown-parser';
export * from './parsers/gherkin-parser';

// Transformers
export * from './transformers';

// Services
export * from './services/relationship-extractor.service';
export * from './services/manifest.service';
export * from './services/import-pipeline.service';

// Database
export * from './db';

// Re-export main functions for convenience
export { runImportPipeline, importContent } from './services/import-pipeline.service';
