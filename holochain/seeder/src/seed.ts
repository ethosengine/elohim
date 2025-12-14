/**
 * Holochain Content Seeder
 *
 * Seeds content from /data/content directory into Holochain.
 * Reads markdown files, extracts metadata, and creates Content entries.
 */

import { AdminWebsocket, AppWebsocket, encodeHashToBase64, CellId, decodeHashFromBase64 } from '@holochain/client';
import * as fs from 'fs';
import * as path from 'path';
import matter from 'gray-matter';
import { createHash } from 'crypto';

// Configuration
const CONTENT_DIR = process.env.CONTENT_DIR || '/projects/elohim/data/content';
const HC_PORTS_FILE = process.env.HC_PORTS_FILE || '/projects/elohim/holochain/local-dev/.hc_ports';
const APP_ID = 'lamad-spike';
const ROLE_NAME = 'lamad';
const ZOME_NAME = 'content_store';

/**
 * Read Holochain ports from .hc_ports file
 */
function readHcPorts(): { adminPort: number; appPort: number } {
  try {
    const content = fs.readFileSync(HC_PORTS_FILE, 'utf-8');
    const adminMatch = content.match(/admin_port=(\d+)/);
    const appMatch = content.match(/app_port=(\d+)/);

    if (!adminMatch || !appMatch) {
      throw new Error('Could not parse .hc_ports file');
    }

    return {
      adminPort: parseInt(adminMatch[1], 10),
      appPort: parseInt(appMatch[1], 10),
    };
  } catch (error) {
    console.error(`❌ Could not read ${HC_PORTS_FILE}:`, error);
    console.log('   Falling back to default ports (4444, 4445)');
    return { adminPort: 4444, appPort: 4445 };
  }
}

// Read ports from file or env
const ports = readHcPorts();
const ADMIN_WS_URL = process.env.HOLOCHAIN_ADMIN_URL || `ws://localhost:${ports.adminPort}`;
// APP_WS_URL is computed dynamically for remote connections (see resolveAppUrl function)
const DEFAULT_APP_WS_URL = process.env.HOLOCHAIN_APP_URL || `ws://localhost:${ports.appPort}`;

/**
 * Resolve app WebSocket URL based on admin URL and dynamic port.
 * For remote connections (via proxy), use /app/:port path routing.
 * For local connections, use direct localhost.
 */
function resolveAppUrl(adminUrl: string, port: number): string {
  // If admin URL is remote (not localhost), route through proxy
  if (!adminUrl.includes('localhost') && !adminUrl.includes('127.0.0.1')) {
    // Extract base URL (remove query params) and add /app/:port path
    const url = new URL(adminUrl);
    const baseUrl = `${url.protocol}//${url.host}`;
    const apiKey = url.searchParams.get('apiKey');
    const apiKeyParam = apiKey ? `?apiKey=${encodeURIComponent(apiKey)}` : '';
    return `${baseUrl}/app/${port}${apiKeyParam}`;
  }
  // Local: use direct connection
  return `ws://localhost:${port}`;
}

// Types matching the Holochain zome
interface CreateContentInput {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  reach: string;
  metadata_json: string;
}

interface BulkCreateContentInput {
  import_id: string;
  contents: CreateContentInput[];
}

interface ContentOutput {
  action_hash: Uint8Array;
  entry_hash: Uint8Array;
  content: any;
}

interface BulkCreateContentOutput {
  import_id: string;
  created_count: number;
  action_hashes: Uint8Array[];
  errors: string[];
}

// Learning Path types
interface CreatePathInput {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose: string | null;
  difficulty: string;
  estimated_duration: string | null;
  visibility: string;
  path_type: string;
  tags: string[];
}

interface AddPathStepInput {
  path_id: string;
  order_index: number;
  step_type: string;
  resource_id: string;
  step_title: string | null;
  step_narrative: string | null;
  is_optional: boolean;
}

// Sample paths to seed
// Steps use id_pattern to match content IDs (substring match)
const SAMPLE_PATHS = [
  {
    id: 'elohim-protocol-overview',
    version: '1.0.0',
    title: 'Elohim Protocol Overview',
    description: 'An introduction to the Elohim Protocol - decentralized governance for human flourishing',
    purpose: 'Understand the core concepts and pillars of the Elohim Protocol',
    difficulty: 'beginner',
    estimated_duration: '30 minutes',
    visibility: 'public',
    path_type: 'introduction',
    tags: ['elohim', 'protocol', 'overview', 'beginner'],
    steps: [
      { id_pattern: 'manifesto', step_type: 'read', title: 'Read the Manifesto' },
      { id_pattern: 'governance-layers-architecture', step_type: 'read', title: 'Understand the Architecture' },
      { id_pattern: 'global-orchestra', step_type: 'explore', title: 'Explore the Global Orchestra' },
    ],
  },
  {
    id: 'lamad-learning-path',
    version: '1.0.0',
    title: 'Lamad Learning System',
    description: 'Learn how the Lamad pillar enables decentralized, permissionless learning',
    purpose: 'Master the Lamad learning path system and content model',
    difficulty: 'intermediate',
    estimated_duration: '1 hour',
    visibility: 'public',
    path_type: 'deep-dive',
    tags: ['lamad', 'learning', 'content', 'paths'],
    steps: [
      { id_pattern: 'lamad', step_type: 'read', title: 'Introduction to Lamad' },
      { id_pattern: 'Module-1', step_type: 'read', title: 'The Church Dilemma' },
      { id_pattern: 'Module-2', step_type: 'read', title: 'Systems Thinking' },
    ],
  },
];

/**
 * Extract metadata from file path
 */
function extractPathMetadata(filePath: string): {
  domain: string;
  epic: string | null;
  userType: string | null;
  contentCategory: string | null;
} {
  const relativePath = filePath.replace(CONTENT_DIR + '/', '');
  const parts = relativePath.split('/');

  return {
    domain: parts[0] || 'unknown',
    epic: parts[1] || null,
    userType: parts[2] || null,
    contentCategory: parts.length > 3 ? parts.slice(3, -1).join('/') : null,
  };
}

/**
 * Determine content type from file path and content
 */
function determineContentType(filePath: string, frontmatter: any): string {
  const fileName = path.basename(filePath, '.md').toLowerCase();

  if (fileName === 'epic' || fileName.startsWith('epic-')) return 'epic';
  if (fileName === 'readme') return 'documentation';
  if (fileName.includes('manifesto')) return 'manifesto';
  if (fileName.includes('feature')) return 'feature';
  if (fileName.includes('module')) return 'module';
  if (fileName.includes('claude')) return 'guidance';

  // Check frontmatter for type hints
  if (frontmatter?.type) return frontmatter.type;
  if (frontmatter?.contentType) return frontmatter.contentType;

  return 'article';
}

/**
 * Generate a stable ID from file path
 */
function generateContentId(filePath: string): string {
  const relativePath = filePath.replace(CONTENT_DIR + '/', '');
  const hash = createHash('sha256').update(relativePath).digest('hex').slice(0, 12);
  const cleanPath = relativePath
    .replace(/\.md$/, '')
    .replace(/[^a-zA-Z0-9-_]/g, '-')
    .replace(/-+/g, '-')
    .slice(0, 50);
  return `${cleanPath}-${hash}`;
}

/**
 * Extract tags from content and frontmatter
 */
function extractTags(frontmatter: any, pathMeta: ReturnType<typeof extractPathMetadata>): string[] {
  const tags: Set<string> = new Set();

  // Add domain as tag
  if (pathMeta.domain) tags.add(pathMeta.domain);

  // Add epic as tag
  if (pathMeta.epic) tags.add(pathMeta.epic);

  // Add frontmatter tags
  if (frontmatter?.tags) {
    if (Array.isArray(frontmatter.tags)) {
      frontmatter.tags.forEach((t: string) => tags.add(t));
    }
  }

  return Array.from(tags);
}

/**
 * Extract title from content
 */
function extractTitle(content: string, filePath: string): string {
  // Try to extract from first H1
  const h1Match = content.match(/^#\s+(.+)$/m);
  if (h1Match) return h1Match[1].trim();

  // Fall back to filename
  const fileName = path.basename(filePath, '.md');
  return fileName.replace(/[-_]/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
}

/**
 * Extract description from content
 */
function extractDescription(content: string, frontmatter: any): string {
  if (frontmatter?.description) return frontmatter.description;

  // Try to get first paragraph after title
  const lines = content.split('\n');
  let inParagraph = false;
  let paragraph = '';

  for (const line of lines) {
    if (line.startsWith('#')) continue;
    if (line.startsWith('---')) continue;
    if (line.trim() === '') {
      if (inParagraph && paragraph) break;
      continue;
    }
    inParagraph = true;
    paragraph += line + ' ';
  }

  return paragraph.trim().slice(0, 500) || 'No description available';
}

/**
 * Find all markdown files in directory
 */
function findMarkdownFiles(dir: string, limit?: number): string[] {
  const files: string[] = [];

  function walk(currentDir: string) {
    if (limit && files.length >= limit) return;

    const entries = fs.readdirSync(currentDir, { withFileTypes: true });

    for (const entry of entries) {
      if (limit && files.length >= limit) return;

      const fullPath = path.join(currentDir, entry.name);

      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (entry.name.endsWith('.md')) {
        files.push(fullPath);
      }
    }
  }

  walk(dir);
  return files;
}

/**
 * Parse a markdown file into CreateContentInput
 */
function parseMarkdownFile(filePath: string): CreateContentInput {
  const fileContent = fs.readFileSync(filePath, 'utf-8');
  const { data: frontmatter, content } = matter(fileContent);
  const pathMeta = extractPathMetadata(filePath);

  return {
    id: generateContentId(filePath),
    content_type: determineContentType(filePath, frontmatter),
    title: extractTitle(content, filePath),
    description: extractDescription(content, frontmatter),
    content: content,
    content_format: 'markdown',
    tags: extractTags(frontmatter, pathMeta),
    source_path: filePath.replace(CONTENT_DIR + '/', ''),
    related_node_ids: [],
    reach: 'public',
    metadata_json: JSON.stringify({
      domain: pathMeta.domain,
      epic: pathMeta.epic,
      userType: pathMeta.userType,
      contentCategory: pathMeta.contentCategory,
      frontmatter,
    }),
  };
}

/**
 * Main seeding function
 */
async function seed() {
  const args = process.argv.slice(2);
  const limitArg = args.find((a) => a.startsWith('--limit'));
  const limit = limitArg ? parseInt(limitArg.split('=')[1] || args[args.indexOf('--limit') + 1]) : undefined;

  console.log('🌱 Holochain Content Seeder');
  console.log(`📁 Content directory: ${CONTENT_DIR}`);
  console.log(`🔌 Admin WebSocket: ${ADMIN_WS_URL}`);
  if (limit) console.log(`📊 Limit: ${limit} files`);

  // Connect to admin websocket
  console.log('\n📡 Connecting to Holochain admin...');
  let adminWs: AdminWebsocket;
  try {
    adminWs = await AdminWebsocket.connect({
      url: new URL(ADMIN_WS_URL),
      wsClientOptions: { origin: 'http://localhost' },
    });
    console.log('✅ Connected to admin WebSocket');
  } catch (error) {
    console.error('❌ Failed to connect to admin WebSocket:', error);
    process.exit(1);
  }

  // Get app info to find cell ID
  console.log('\n📱 Getting app info...');
  const apps = await adminWs.listApps({});
  const app = apps.find((a) => a.installed_app_id === APP_ID);

  if (!app) {
    console.error(`❌ App "${APP_ID}" not found. Available apps:`, apps.map((a) => a.installed_app_id));
    process.exit(1);
  }

  const cellInfo = app.cell_info[ROLE_NAME];
  if (!cellInfo || cellInfo.length === 0) {
    console.error(`❌ Role "${ROLE_NAME}" not found in app`);
    process.exit(1);
  }

  const provisionedCell = cellInfo.find((c: any) => c.type === 'provisioned');
  if (!provisionedCell) {
    console.error('❌ No provisioned cell found');
    process.exit(1);
  }

  // Cell ID is an array: [dna_hash, agent_pub_key]
  // Each element can be a Buffer-like object with {type: 'Buffer', data: [...]} or a Uint8Array
  const rawCellId = (provisionedCell as any).value.cell_id;

  function toUint8Array(val: any): Uint8Array {
    if (val instanceof Uint8Array) return val;
    if (val?.type === 'Buffer' && Array.isArray(val.data)) {
      return new Uint8Array(val.data);
    }
    if (ArrayBuffer.isView(val)) return new Uint8Array(val.buffer);
    throw new Error(`Cannot convert to Uint8Array: ${JSON.stringify(val)}`);
  }

  const cellId: CellId = [toUint8Array(rawCellId[0]), toUint8Array(rawCellId[1])];
  console.log(`✅ Found cell: ${encodeHashToBase64(cellId[0]).slice(0, 20)}...`);

  // Get app auth token
  console.log('\n🔑 Getting app auth token...');
  const token = await adminWs.issueAppAuthenticationToken({
    installed_app_id: APP_ID,
    single_use: false,
    expiry_seconds: 3600, // 1 hour
  });
  console.log('✅ Got auth token');

  // Authorize signing credentials via admin websocket
  console.log('\n🔏 Authorizing signing credentials...');
  await adminWs.authorizeSigningCredentials(cellId);
  console.log('✅ Signing credentials authorized');

  // Attach or get existing app interface
  console.log('\n🔌 Setting up app interface...');
  let appPort: number;
  const existingInterfaces = await adminWs.listAppInterfaces();
  if (existingInterfaces.length > 0) {
    appPort = existingInterfaces[0].port;
    console.log(`✅ Using existing app interface on port ${appPort}`);
  } else {
    const { port } = await adminWs.attachAppInterface({ allowed_origins: '*' });
    appPort = port;
    console.log(`✅ Created app interface on port ${appPort}`);
  }

  // Resolve app URL (uses proxy routing for remote, direct for local)
  const appWsUrl = process.env.HOLOCHAIN_APP_URL || resolveAppUrl(ADMIN_WS_URL, appPort);
  console.log(`🔌 App WebSocket: ${appWsUrl}`);

  // Connect to app websocket
  console.log('\n📡 Connecting to Holochain app...');
  let appWs: AppWebsocket;
  try {
    appWs = await AppWebsocket.connect({
      url: new URL(appWsUrl),
      wsClientOptions: { origin: 'http://localhost' },
      token: token.token,
    });
    console.log('✅ Connected to app WebSocket');
  } catch (error) {
    console.error('❌ Failed to connect to app WebSocket:', error);
    process.exit(1);
  }

  // Find markdown files
  console.log('\n📂 Scanning for markdown files...');
  const files = findMarkdownFiles(CONTENT_DIR, limit);
  console.log(`✅ Found ${files.length} markdown files`);

  // Seed content
  console.log('\n🌱 Seeding content to Holochain...');
  let successCount = 0;
  let errorCount = 0;

  for (const file of files) {
    try {
      const input = parseMarkdownFile(file);
      console.log(`  📄 ${input.id.slice(0, 50)}...`);

      // Check if content already exists
      const existingContent = await appWs.callZome({
        cell_id: cellId,
        zome_name: ZOME_NAME,
        fn_name: 'get_content_by_id',
        payload: { id: input.id },
      });

      if (existingContent) {
        successCount++; // Count as success (idempotent)
        console.log(`     ⏭️  Already exists, skipping`);
        continue;
      }

      const result = await appWs.callZome({
        cell_id: cellId,
        zome_name: ZOME_NAME,
        fn_name: 'create_content',
        payload: input,
      });

      successCount++;
      console.log(`     ✅ Created: ${encodeHashToBase64((result as ContentOutput).action_hash).slice(0, 15)}...`);
    } catch (error: any) {
      errorCount++;
      console.error(`     ❌ Error: ${error.message || error}`);
    }
  }

  // Summary for content
  console.log('\n📊 Content Seeding Complete!');
  console.log(`   ✅ Success: ${successCount}`);
  console.log(`   ❌ Errors: ${errorCount}`);
  console.log(`   📁 Total files: ${files.length}`);

  // Seed learning paths
  console.log('\n📚 Seeding learning paths...');
  let pathSuccessCount = 0;
  let pathErrorCount = 0;

  // First, get all created content to build an index for pattern matching
  const contentStats: any = await appWs.callZome({
    cell_id: cellId,
    zome_name: ZOME_NAME,
    fn_name: 'get_content_stats',
    payload: null,
  });
  console.log(`   📦 Available content: ${contentStats.total_count} items`);

  // Build content index by fetching all content (for ID pattern matching)
  // We use get_my_content since all content was created by this agent
  const allContent = await appWs.callZome({
    cell_id: cellId,
    zome_name: ZOME_NAME,
    fn_name: 'get_my_content',
    payload: null,
  }) as ContentOutput[];
  console.log(`   📦 Content index built: ${allContent.length} items`);

  // Helper to find content by ID pattern
  const findContentByPattern = (pattern: string): string | null => {
    const match = allContent.find(c =>
      c.content.id.toLowerCase().includes(pattern.toLowerCase())
    );
    if (!match) {
      console.log(`      ❓ No match for pattern "${pattern}" in ${allContent.length} items`);
      // Show some sample IDs for debugging
      if (allContent.length > 0) {
        console.log(`         Sample IDs: ${allContent.slice(0, 3).map(c => c.content.id).join(', ')}`);
      }
    }
    return match ? match.content.id : null;
  };

  for (const pathDef of SAMPLE_PATHS) {
    try {
      console.log(`\n   📖 Processing path: ${pathDef.title}`);

      // Check if path already exists
      const existingPath = await appWs.callZome({
        cell_id: cellId,
        zome_name: ZOME_NAME,
        fn_name: 'get_path_with_steps',
        payload: pathDef.id,
      }) as any;

      if (existingPath) {
        // Check if path has placeholder resource IDs - if so, delete and recreate
        const hasPlaceholders = existingPath.steps?.some((s: any) =>
          s.step.resource_id?.startsWith('placeholder-')
        );

        if (hasPlaceholders) {
          console.log(`      🔄 Deleting path with placeholder IDs...`);
          try {
            await appWs.callZome({
              cell_id: cellId,
              zome_name: ZOME_NAME,
              fn_name: 'delete_path',
              payload: pathDef.id,
            });
            console.log(`      ✅ Deleted old path, recreating...`);
            // Continue to create new path below
          } catch (error: any) {
            console.log(`      ⚠️  Could not delete path: ${error.message}`);
            pathErrorCount++;
            continue;
          }
        } else {
          console.log(`      ⏭️  Path already exists with valid IDs, skipping`);
          pathSuccessCount++; // Count as success (idempotent)
          continue;
        }
      }

      // Create the path
      const pathInput: CreatePathInput = {
        id: pathDef.id,
        version: pathDef.version,
        title: pathDef.title,
        description: pathDef.description,
        purpose: pathDef.purpose,
        difficulty: pathDef.difficulty,
        estimated_duration: pathDef.estimated_duration,
        visibility: pathDef.visibility,
        path_type: pathDef.path_type,
        tags: pathDef.tags,
      };

      const pathResult = await appWs.callZome({
        cell_id: cellId,
        zome_name: ZOME_NAME,
        fn_name: 'create_path',
        payload: pathInput,
      });
      console.log(`      ✅ Path created: ${encodeHashToBase64(pathResult as Uint8Array).slice(0, 15)}...`);

      // Add steps - find content that matches the ID pattern
      for (let i = 0; i < pathDef.steps.length; i++) {
        const stepDef = pathDef.steps[i];

        // Find content by ID pattern match
        const resourceId = findContentByPattern(stepDef.id_pattern)
          ?? `placeholder-${stepDef.id_pattern}`;

        const stepInput: AddPathStepInput = {
          path_id: pathDef.id,
          order_index: i,
          step_type: stepDef.step_type,
          resource_id: resourceId,
          step_title: stepDef.title,
          step_narrative: null,
          is_optional: false,
        };

        await appWs.callZome({
          cell_id: cellId,
          zome_name: ZOME_NAME,
          fn_name: 'add_path_step',
          payload: stepInput,
        });
        console.log(`      📝 Step ${i + 1}: ${stepDef.title} → ${resourceId.slice(0, 30)}...`);
      }

      pathSuccessCount++;
    } catch (error: any) {
      pathErrorCount++;
      console.error(`      ❌ Error creating path: ${error.message || error}`);
    }
  }

  // Final summary
  console.log('\n📊 Path Seeding Complete!');
  console.log(`   ✅ Paths created: ${pathSuccessCount}`);
  console.log(`   ❌ Errors: ${pathErrorCount}`);

  // Verify paths were created by querying them back
  console.log('\n🔍 Verifying paths...');
  try {
    const allPaths = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_all_paths',
      payload: null,
    }) as any;
    console.log(`   📊 get_all_paths returns: ${allPaths.total_count} paths`);

    // Also try to get a specific path
    const pathWithSteps = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_path_with_steps',
      payload: 'elohim-protocol-overview',
    });
    console.log(`   📖 get_path_with_steps('elohim-protocol-overview'):`, pathWithSteps ? 'FOUND' : 'NOT FOUND');
  } catch (error: any) {
    console.error(`   ❌ Verification failed:`, error.message);
  }

  // Get stats
  console.log('\n📈 Fetching content stats...');
  try {
    const stats = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_content_stats',
      payload: null,
    });
    console.log('Content Stats:', stats);
  } catch (error) {
    console.log('Could not fetch stats:', error);
  }

  await adminWs.client.close();
  await appWs.client.close();

  console.log('\n✨ Done!');
}

seed().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
