/**
 * Generate chunked content for large files that exceed Kuzu WASM limits.
 *
 * Strategy:
 * - Content <= 15KB: Store as single ContentNode (handled by existing scripts)
 * - Content > 15KB: Split into multiple ContentChunk nodes + parent ContentNode with metadata
 *
 * This allows ALL content to live in Kuzu without JSON fallback.
 */
const fs = require('fs');
const path = require('path');

const contentDir = 'src/assets/lamad-data/content';

// Kuzu WASM-safe limit (conservative)
const MAX_CHUNK_SIZE = 12000; // 12KB per chunk (leaves room for escaping)
const SIZE_THRESHOLD = 15000; // Content over this gets chunked

function escapeCypher(str) {
  if (typeof str !== 'string') return '';
  // Use placeholders instead of escape sequences because Kuzu WASM
  // mangles \n escape sequences (strips backslash, leaves just 'n')
  // Also escape double quotes to avoid Cypher parsing issues
  return str
    .replace(/\\/g, '{{BACKSLASH}}')
    .replace(/'/g, '{{QUOTE}}')
    .replace(/"/g, '{{DQUOTE}}')
    .replace(/\n/g, '{{NEWLINE}}')
    .replace(/\r/g, '');
}

// Split text into chunks at paragraph/line boundaries when possible
function splitIntoChunks(text, maxSize) {
  const chunks = [];
  let remaining = text;
  let chunkIndex = 0;

  while (remaining.length > 0) {
    if (remaining.length <= maxSize) {
      chunks.push({ index: chunkIndex, content: remaining });
      break;
    }

    // Find a good split point (paragraph or line break)
    let splitAt = maxSize;

    // Try to split at paragraph boundary
    const lastPara = remaining.lastIndexOf('\n\n', maxSize);
    if (lastPara > maxSize * 0.5) {
      splitAt = lastPara + 2; // Include the double newline
    } else {
      // Try single newline
      const lastLine = remaining.lastIndexOf('\n', maxSize);
      if (lastLine > maxSize * 0.5) {
        splitAt = lastLine + 1;
      }
    }

    chunks.push({ index: chunkIndex, content: remaining.substring(0, splitAt) });
    remaining = remaining.substring(splitAt);
    chunkIndex++;
  }

  return chunks;
}

function generateChunkCypher(contentId, chunks) {
  const statements = [];

  // Create ContentChunk nodes
  for (const chunk of chunks) {
    const chunkId = `${contentId}__chunk_${chunk.index}`;
    const escapedContent = escapeCypher(chunk.content);

    statements.push(
      `CREATE (:ContentChunk {id: '${chunkId}', parentId: '${escapeCypher(contentId)}', chunkIndex: ${chunk.index}, totalChunks: ${chunks.length}, content: '${escapedContent}'});`
    );
  }

  return statements;
}

function analyzeAndChunk() {
  console.log('=== Analyzing Large Content for Chunking ===\n');

  const files = fs.readdirSync(contentDir).filter(f => f.endsWith('.json'));
  const largeFiles = [];

  for (const file of files) {
    try {
      const json = JSON.parse(fs.readFileSync(path.join(contentDir, file), 'utf8'));
      let contentLength = 0;
      let content = '';

      if (typeof json.content === 'string') {
        content = json.content;
        contentLength = content.length;
      } else if (json.content && typeof json.content === 'object') {
        content = JSON.stringify(json.content);
        contentLength = content.length;
      }

      if (contentLength > SIZE_THRESHOLD) {
        largeFiles.push({
          id: json.id || path.basename(file, '.json'),
          file,
          contentLength,
          content,
          json
        });
      }
    } catch (e) {
      console.error(`Error processing ${file}:`, e.message);
    }
  }

  console.log(`Found ${largeFiles.length} files over ${SIZE_THRESHOLD / 1000}KB threshold:\n`);

  // Generate chunk statements
  const allStatements = [];

  // First, create ContentChunk table schema if not exists
  allStatements.push(`CREATE NODE TABLE IF NOT EXISTS ContentChunk (
  id STRING PRIMARY KEY,
  parentId STRING,
  chunkIndex INT32,
  totalChunks INT32,
  content STRING
);`);

  allStatements.push(`CREATE REL TABLE IF NOT EXISTS HAS_CHUNK (
  FROM ContentNode TO ContentChunk
);`);

  for (const item of largeFiles) {
    const chunks = splitIntoChunks(item.content, MAX_CHUNK_SIZE);
    console.log(`  ${item.id}: ${(item.contentLength / 1024).toFixed(1)}KB -> ${chunks.length} chunks`);

    const chunkStatements = generateChunkCypher(item.id, chunks);
    allStatements.push(`\n// Chunks for: ${item.id} (${chunks.length} chunks)`);
    allStatements.push(...chunkStatements);

    // Create relationship statements
    for (let i = 0; i < chunks.length; i++) {
      const chunkId = `${item.id}__chunk_${i}`;
      allStatements.push(
        `MATCH (c:ContentNode {id: '${escapeCypher(item.id)}'}), (ch:ContentChunk {id: '${chunkId}'}) CREATE (c)-[:HAS_CHUNK]->(ch);`
      );
    }
  }

  // Write chunk seed file
  const chunkSeedFile = 'src/assets/lamad-data/content-chunks.cypher';
  const header = `// Content Chunks for Large Files
// Generated: ${new Date().toISOString()}
// These chunks store content that exceeds Kuzu WASM string literal limits
// The KuzuDataService will automatically reassemble chunks when loading content
`;

  fs.writeFileSync(chunkSeedFile, header + '\n' + allStatements.join('\n'));
  console.log(`\nWrote ${allStatements.length} statements to ${chunkSeedFile}`);

  return { largeFiles, statements: allStatements };
}

analyzeAndChunk();
