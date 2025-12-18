#!/usr/bin/env npx tsx
/**
 * Holochain Snapshot Manager
 *
 * Creates and restores pre-seeded conductor snapshots for fast startup.
 *
 * Commands:
 *   create  - Seed fresh conductor and create snapshot archive
 *   restore - Restore conductor from snapshot
 *   clean   - Remove conductor data (fresh start)
 *   status  - Show current conductor state
 */

import * as fs from 'fs';
import * as path from 'path';
import { execSync, spawn } from 'child_process';

// Configuration
const LOCAL_DEV_DIR = process.env.LOCAL_DEV_DIR || '/projects/elohim/holochain/local-dev';
const CONDUCTOR_DATA_DIR = path.join(LOCAL_DEV_DIR, 'conductor-data');
const SNAPSHOTS_DIR = path.join(LOCAL_DEV_DIR, 'snapshots');
const SEED_REPORT_PATH = path.join(LOCAL_DEV_DIR, 'last-seed-report.json');
const HAPP_PATH = '/projects/elohim/holochain/dna/lamad-spike/workdir/lamad-spike.happ';
const APP_ID = 'lamad-spike';

interface SeedReport {
  timestamp: string;
  totalDurationMs: number;
  totalDurationFormatted: string;
  results: {
    conceptsCreated: number;
    conceptErrors: number;
    pathsCreated: number;
    pathErrors: number;
  };
  phases: Record<string, { durationMs: number; percentage: number }>;
  operations: Record<string, { count: number; totalMs: number; avgMs: number }>;
  skippedFiles: {
    total: number;
    byReason: Record<string, string[]>;
  };
}

interface SnapshotMetadata {
  createdAt: string;
  conceptCount: number;
  pathCount: number;
  holochainVersion: string;
  seedDuration: string;
  note?: string;
  seedReport?: SeedReport;
}

function ensureDir(dir: string): void {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
    console.log(`   Created directory: ${dir}`);
  }
}

function getHolochainVersion(): string {
  try {
    const output = execSync('hc --version 2>&1', { encoding: 'utf-8' });
    return output.trim();
  } catch {
    return 'unknown';
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function getDirSize(dir: string): number {
  if (!fs.existsSync(dir)) return 0;
  let size = 0;
  const walk = (d: string) => {
    const entries = fs.readdirSync(d, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(d, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
      } else {
        size += fs.statSync(fullPath).size;
      }
    }
  };
  walk(dir);
  return size;
}

async function runCommand(cmd: string, args: string[], options?: { cwd?: string }): Promise<{ code: number; stdout: string; stderr: string }> {
  return new Promise((resolve) => {
    const proc = spawn(cmd, args, {
      cwd: options?.cwd || process.cwd(),
      stdio: ['inherit', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';

    proc.stdout?.on('data', (data) => {
      stdout += data.toString();
      process.stdout.write(data);
    });

    proc.stderr?.on('data', (data) => {
      stderr += data.toString();
      process.stderr.write(data);
    });

    proc.on('close', (code) => {
      resolve({ code: code || 0, stdout, stderr });
    });
  });
}

// ========================================
// COMMANDS
// ========================================

async function createSnapshot(name?: string): Promise<void> {
  const snapshotName = name || `snapshot-${new Date().toISOString().split('T')[0]}`;
  const snapshotPath = path.join(SNAPSHOTS_DIR, `${snapshotName}.tar.gz`);
  const metadataPath = path.join(SNAPSHOTS_DIR, `${snapshotName}.json`);

  console.log('========================================');
  console.log('   HOLOCHAIN SNAPSHOT CREATOR');
  console.log('========================================\n');

  // Check if happ exists
  if (!fs.existsSync(HAPP_PATH)) {
    console.error(`   hApp not found: ${HAPP_PATH}`);
    console.error('   Run: cd holochain/dna/lamad-spike && hc app pack workdir');
    process.exit(1);
  }

  // Step 1: Clean existing conductor data
  console.log('Step 1: Cleaning existing conductor data...');
  if (fs.existsSync(CONDUCTOR_DATA_DIR)) {
    fs.rmSync(CONDUCTOR_DATA_DIR, { recursive: true });
    console.log('   Removed existing conductor data');
  }
  ensureDir(CONDUCTOR_DATA_DIR);
  ensureDir(SNAPSHOTS_DIR);

  // Step 2: Generate fresh sandbox with persistent root
  console.log('\nStep 2: Generating fresh sandbox...');
  const generateResult = await runCommand('hc', [
    'sandbox',
    'generate',
    '--root', CONDUCTOR_DATA_DIR,
    '--app-id', APP_ID,
    '--in-process-lair',
    '-r=4445',
    HAPP_PATH,
  ]);

  if (generateResult.code !== 0) {
    console.error('   Failed to generate sandbox');
    process.exit(1);
  }
  console.log('   Sandbox generated');

  // Step 3: Start conductor in background
  console.log('\nStep 3: Starting conductor...');
  const conductorProc = spawn('hc', [
    'sandbox',
    'run',
    '--root', CONDUCTOR_DATA_DIR,
    '-p=4445',
  ], {
    stdio: ['ignore', 'pipe', 'pipe'],
    detached: true,
  });

  // Wait for conductor to be ready
  let adminPort: number | null = null;
  await new Promise<void>((resolve) => {
    const timeout = setTimeout(() => {
      console.error('   Timeout waiting for conductor');
      process.exit(1);
    }, 30000);

    conductorProc.stdout?.on('data', (data) => {
      const output = data.toString();
      // Look for admin port in output
      const match = output.match(/admin_port[=:\s]+(\d+)/i) || output.match(/Conductor admin interface running at.*:(\d+)/i);
      if (match) {
        adminPort = parseInt(match[1], 10);
      }
      if (output.includes('Conductor ready') || output.includes('running') || adminPort) {
        clearTimeout(timeout);
        setTimeout(resolve, 2000); // Give it a moment to stabilize
      }
    });
  });

  // Write ports file for seeder
  const portsContent = `admin_port=${adminPort || 4444}\napp_port=4445`;
  fs.writeFileSync(path.join(LOCAL_DEV_DIR, '.hc_ports'), portsContent);
  console.log(`   Conductor running (admin: ${adminPort}, app: 4445)`);

  // Step 4: Run seeder
  console.log('\nStep 4: Running seeder...');
  const seedStart = Date.now();
  const seedResult = await runCommand('npx', ['tsx', 'src/seed.ts'], {
    cwd: '/projects/elohim/holochain/seeder',
  });
  const seedDuration = Date.now() - seedStart;

  // Parse seed results from output
  const conceptMatch = seedResult.stdout.match(/Success: (\d+)/);
  const pathMatch = seedResult.stdout.match(/Paths: (\d+)/);
  const conceptCount = conceptMatch ? parseInt(conceptMatch[1], 10) : 0;
  const pathCount = pathMatch ? parseInt(pathMatch[1], 10) : 0;

  // Step 5: Stop conductor
  console.log('\nStep 5: Stopping conductor...');
  conductorProc.kill('SIGTERM');
  await new Promise((resolve) => setTimeout(resolve, 2000));
  console.log('   Conductor stopped');

  // Step 6: Create snapshot archive
  console.log('\nStep 6: Creating snapshot archive...');
  execSync(`tar -czvf "${snapshotPath}" -C "${LOCAL_DEV_DIR}" conductor-data`, {
    stdio: 'inherit',
  });

  const snapshotSize = fs.statSync(snapshotPath).size;
  console.log(`   Snapshot created: ${snapshotPath}`);
  console.log(`   Size: ${formatBytes(snapshotSize)}`);

  // Step 7: Save metadata
  const metadata: SnapshotMetadata = {
    createdAt: new Date().toISOString(),
    conceptCount,
    pathCount,
    holochainVersion: getHolochainVersion(),
    seedDuration: `${(seedDuration / 1000).toFixed(1)}s`,
  };
  fs.writeFileSync(metadataPath, JSON.stringify(metadata, null, 2));

  console.log('\n========================================');
  console.log('   SNAPSHOT CREATED SUCCESSFULLY');
  console.log('========================================');
  console.log(`   Name: ${snapshotName}`);
  console.log(`   Concepts: ${conceptCount}`);
  console.log(`   Paths: ${pathCount}`);
  console.log(`   Seed time: ${metadata.seedDuration}`);
  console.log(`   Archive size: ${formatBytes(snapshotSize)}`);
  console.log('\n   To restore: npm run snapshot:restore');
}

async function restoreSnapshot(name?: string): Promise<void> {
  console.log('========================================');
  console.log('   HOLOCHAIN SNAPSHOT RESTORE');
  console.log('========================================\n');

  ensureDir(SNAPSHOTS_DIR);

  // Find snapshot to restore
  let snapshotPath: string;
  let metadataPath: string;

  if (name) {
    snapshotPath = path.join(SNAPSHOTS_DIR, `${name}.tar.gz`);
    metadataPath = path.join(SNAPSHOTS_DIR, `${name}.json`);
  } else {
    // Find most recent snapshot
    const snapshots = fs.readdirSync(SNAPSHOTS_DIR)
      .filter(f => f.endsWith('.tar.gz'))
      .map(f => ({
        name: f,
        path: path.join(SNAPSHOTS_DIR, f),
        mtime: fs.statSync(path.join(SNAPSHOTS_DIR, f)).mtime,
      }))
      .sort((a, b) => b.mtime.getTime() - a.mtime.getTime());

    if (snapshots.length === 0) {
      console.error('   No snapshots found. Run: npm run snapshot:create');
      process.exit(1);
    }

    snapshotPath = snapshots[0].path;
    metadataPath = snapshotPath.replace('.tar.gz', '.json');
    console.log(`   Using most recent: ${snapshots[0].name}`);
  }

  if (!fs.existsSync(snapshotPath)) {
    console.error(`   Snapshot not found: ${snapshotPath}`);
    process.exit(1);
  }

  // Load metadata if available
  let metadata: SnapshotMetadata | null = null;
  if (fs.existsSync(metadataPath)) {
    metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf-8'));
    console.log(`   Snapshot from: ${metadata?.createdAt}`);
    console.log(`   Contains: ${metadata?.conceptCount} concepts, ${metadata?.pathCount} paths`);
  }

  // Clean existing conductor data
  console.log('\nStep 1: Cleaning existing conductor data...');
  if (fs.existsSync(CONDUCTOR_DATA_DIR)) {
    fs.rmSync(CONDUCTOR_DATA_DIR, { recursive: true });
    console.log('   Removed existing data');
  }

  // Extract snapshot
  console.log('\nStep 2: Extracting snapshot...');
  const startTime = Date.now();
  execSync(`tar -xzf "${snapshotPath}" -C "${LOCAL_DEV_DIR}"`, {
    stdio: 'inherit',
  });
  const extractTime = Date.now() - startTime;

  // Check what was actually restored
  const restoredSize = getDirSize(CONDUCTOR_DATA_DIR);
  const snapshotSize = fs.statSync(snapshotPath).size;

  console.log('\n========================================');
  console.log('   SNAPSHOT RESTORED');
  console.log('========================================');
  console.log(`   Restore time: ${extractTime}ms`);
  console.log(`   Data restored: ${formatBytes(restoredSize)}`);
  console.log(`   Location: ${CONDUCTOR_DATA_DIR}`);

  // Warn if snapshot appears empty or minimal
  if (restoredSize < 100 * 1024) { // Less than 100KB
    console.log('\n   ⚠️  WARNING: This snapshot appears to be empty or minimal!');
    console.log('   The conductor-data was likely saved before seeding.');
    if (metadata?.conceptCount === 0) {
      console.log('   Metadata shows 0 concepts - you may need to seed again.');
    }
    console.log('\n   Recommended: Run the full pipeline instead:');
    console.log('      npm run hc:start    # Start fresh conductor');
    console.log('      npm run hc:seed     # Seed content');
    console.log('      npm run snapshot:save  # Save the seeded state');
  } else {
    console.log('\n   ✅ Snapshot contains data');
    if (metadata) {
      console.log(`   Content: ${metadata.conceptCount} concepts, ${metadata.pathCount} paths`);
      if (metadata.seedDuration) {
        console.log(`   Original seed time: ${metadata.seedDuration}`);
      }
      if (metadata.seedReport?.skippedFiles?.total > 0) {
        console.log(`   Skipped files: ${metadata.seedReport.skippedFiles.total}`);
      }
    }
    console.log('\n   To start conductor: npm run hc:start:snapshot');
  }
}

async function cleanConductor(): Promise<void> {
  console.log('Cleaning conductor data...');

  if (fs.existsSync(CONDUCTOR_DATA_DIR)) {
    fs.rmSync(CONDUCTOR_DATA_DIR, { recursive: true });
    console.log('   Removed: ' + CONDUCTOR_DATA_DIR);
  }

  // Also clean temp sandbox directories
  const hcFile = path.join(LOCAL_DEV_DIR, '.hc');
  if (fs.existsSync(hcFile)) {
    const dirs = fs.readFileSync(hcFile, 'utf-8').trim().split('\n');
    for (const dir of dirs) {
      if (dir.startsWith('/tmp/') && fs.existsSync(dir)) {
        fs.rmSync(dir, { recursive: true });
        console.log(`   Removed temp: ${dir}`);
      }
    }
    fs.writeFileSync(hcFile, '');
  }

  console.log('   Done! Conductor data cleaned.');
}

function showStatus(): void {
  console.log('========================================');
  console.log('   HOLOCHAIN CONDUCTOR STATUS');
  console.log('========================================\n');

  // Conductor data
  console.log('Conductor Data:');
  if (fs.existsSync(CONDUCTOR_DATA_DIR)) {
    const size = getDirSize(CONDUCTOR_DATA_DIR);
    console.log(`   Path: ${CONDUCTOR_DATA_DIR}`);
    console.log(`   Size: ${formatBytes(size)}`);
    console.log('   Status: READY');
  } else {
    console.log('   Status: NOT INITIALIZED');
    console.log('   Run: npm run snapshot:restore (or snapshot:create)');
  }

  // Snapshots
  console.log('\nSnapshots:');
  ensureDir(SNAPSHOTS_DIR);
  const snapshots = fs.readdirSync(SNAPSHOTS_DIR)
    .filter(f => f.endsWith('.tar.gz'))
    .map(f => {
      const snapshotPath = path.join(SNAPSHOTS_DIR, f);
      const metadataPath = snapshotPath.replace('.tar.gz', '.json');
      let metadata: SnapshotMetadata | null = null;
      if (fs.existsSync(metadataPath)) {
        metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf-8'));
      }
      return {
        name: f.replace('.tar.gz', ''),
        size: fs.statSync(snapshotPath).size,
        metadata,
      };
    });

  if (snapshots.length === 0) {
    console.log('   No snapshots found');
    console.log('   Run: npm run snapshot:create');
  } else {
    for (const snap of snapshots) {
      console.log(`\n   ${snap.name}`);
      console.log(`      Size: ${formatBytes(snap.size)}`);
      if (snap.metadata) {
        console.log(`      Created: ${snap.metadata.createdAt}`);
        console.log(`      Content: ${snap.metadata.conceptCount} concepts, ${snap.metadata.pathCount} paths`);
        if (snap.metadata.seedDuration && snap.metadata.seedDuration !== 'N/A') {
          console.log(`      Seed time: ${snap.metadata.seedDuration}`);
        }
        if (snap.metadata.seedReport?.results) {
          const r = snap.metadata.seedReport.results;
          if (r.conceptErrors > 0 || r.pathErrors > 0) {
            console.log(`      Errors: ${r.conceptErrors} concept, ${r.pathErrors} path`);
          }
        }
        if (snap.metadata.seedReport?.skippedFiles?.total > 0) {
          console.log(`      Skipped: ${snap.metadata.seedReport.skippedFiles.total} files`);
        }
      }
    }
  }

  // Holochain version
  console.log(`\nHolochain: ${getHolochainVersion()}`);
}

/**
 * Save current conductor state as snapshot (without re-seeding)
 * Use this when you already have a running/stopped conductor with data you want to keep
 */
async function saveSnapshot(name?: string): Promise<void> {
  const snapshotName = name || `snapshot-${new Date().toISOString().split('T')[0]}`;
  const snapshotPath = path.join(SNAPSHOTS_DIR, `${snapshotName}.tar.gz`);
  const metadataPath = path.join(SNAPSHOTS_DIR, `${snapshotName}.json`);

  console.log('========================================');
  console.log('   SAVE CURRENT CONDUCTOR STATE');
  console.log('========================================\n');

  // Check conductor data exists
  if (!fs.existsSync(CONDUCTOR_DATA_DIR)) {
    console.error(`   No conductor data found at: ${CONDUCTOR_DATA_DIR}`);
    console.error('   Run hc:start first, or use snapshot:create for fresh seed');
    process.exit(1);
  }

  const dataSize = getDirSize(CONDUCTOR_DATA_DIR);
  console.log(`   Conductor data: ${formatBytes(dataSize)}`);

  // Warn if conductor-data appears empty
  if (dataSize < 100 * 1024) { // Less than 100KB
    console.log('\n   ⚠️  WARNING: Conductor data appears empty or minimal!');
    console.log('   You may be saving an unseeded state.');
    console.log('   Make sure you ran: npm run hc:seed');
    console.log('\n   Continuing anyway...');
  }

  ensureDir(SNAPSHOTS_DIR);

  // Check if snapshot already exists
  if (fs.existsSync(snapshotPath)) {
    console.log(`\n   ⚠️  Snapshot "${snapshotName}" already exists`);
    console.log(`   Overwriting...`);
  }

  // Create snapshot archive
  console.log('\nCreating snapshot archive...');
  execSync(`tar -czvf "${snapshotPath}" -C "${LOCAL_DEV_DIR}" conductor-data`, {
    stdio: 'inherit',
  });

  const snapshotSize = fs.statSync(snapshotPath).size;
  console.log(`   Snapshot created: ${snapshotPath}`);
  console.log(`   Size: ${formatBytes(snapshotSize)}`);

  // Try to read seed report from last seed run
  let seedReport: SeedReport | undefined;
  let conceptCount = 0;
  let pathCount = 0;
  let seedDuration = 'N/A';

  if (fs.existsSync(SEED_REPORT_PATH)) {
    try {
      seedReport = JSON.parse(fs.readFileSync(SEED_REPORT_PATH, 'utf-8'));
      conceptCount = seedReport?.results?.conceptsCreated || 0;
      pathCount = seedReport?.results?.pathsCreated || 0;
      seedDuration = seedReport?.totalDurationFormatted || 'N/A';

      console.log(`\n   📊 Found seed report from: ${seedReport?.timestamp}`);
      console.log(`      Concepts: ${conceptCount} created, ${seedReport?.results?.conceptErrors || 0} errors`);
      console.log(`      Paths: ${pathCount} created, ${seedReport?.results?.pathErrors || 0} errors`);
      console.log(`      Duration: ${seedDuration}`);

      if (seedReport?.skippedFiles?.total > 0) {
        console.log(`      Skipped files: ${seedReport.skippedFiles.total}`);
      }
    } catch (err) {
      console.log(`\n   ⚠️  Could not read seed report: ${err}`);
    }
  } else {
    console.log(`\n   ⚠️  No seed report found at ${SEED_REPORT_PATH}`);
    console.log('      Run npm run hc:seed first to generate the report');
  }

  // Save metadata with seed report
  const metadata: SnapshotMetadata = {
    createdAt: new Date().toISOString(),
    conceptCount,
    pathCount,
    holochainVersion: getHolochainVersion(),
    seedDuration,
    seedReport,
  };
  fs.writeFileSync(metadataPath, JSON.stringify(metadata, null, 2));

  console.log('\n========================================');
  console.log('   SNAPSHOT SAVED SUCCESSFULLY');
  console.log('========================================');
  console.log(`   Name: ${snapshotName}`);
  console.log(`   Archive size: ${formatBytes(snapshotSize)}`);
  console.log(`   Original data: ${formatBytes(dataSize)}`);
  console.log(`   Compression: ${((1 - snapshotSize / dataSize) * 100).toFixed(1)}%`);
  console.log('\n   To restore: npm run snapshot:restore');
}

function listSnapshots(): void {
  ensureDir(SNAPSHOTS_DIR);
  const snapshots = fs.readdirSync(SNAPSHOTS_DIR).filter(f => f.endsWith('.tar.gz'));

  if (snapshots.length === 0) {
    console.log('No snapshots found. Run: npm run snapshot:create');
    return;
  }

  console.log('Available snapshots:\n');
  for (const snap of snapshots) {
    const snapshotPath = path.join(SNAPSHOTS_DIR, snap);
    const metadataPath = snapshotPath.replace('.tar.gz', '.json');
    const size = fs.statSync(snapshotPath).size;
    const name = snap.replace('.tar.gz', '');

    let info = `  ${name} (${formatBytes(size)})`;
    if (fs.existsSync(metadataPath)) {
      const metadata: SnapshotMetadata = JSON.parse(fs.readFileSync(metadataPath, 'utf-8'));
      info += ` - ${metadata.conceptCount} concepts, ${metadata.pathCount} paths`;
    }
    console.log(info);
  }
}

// ========================================
// CLI
// ========================================

const command = process.argv[2];
const arg = process.argv[3];

switch (command) {
  case 'create':
    createSnapshot(arg).catch(console.error);
    break;
  case 'save':
    saveSnapshot(arg).catch(console.error);
    break;
  case 'restore':
    restoreSnapshot(arg).catch(console.error);
    break;
  case 'clean':
    cleanConductor().catch(console.error);
    break;
  case 'status':
    showStatus();
    break;
  case 'list':
    listSnapshots();
    break;
  default:
    console.log(`
Holochain Snapshot Manager

Commands:
  save [name]     Save current conductor state as snapshot (use after seeding)
  create [name]   Full pipeline: fresh conductor → seed → snapshot
  restore [name]  Restore from snapshot (uses latest if no name)
  clean           Remove conductor data
  status          Show conductor and snapshot status
  list            List available snapshots

Typical workflow:
  1. npm run hc:start        # Start fresh conductor
  2. npm run hc:seed         # Seed with content
  3. npm run snapshot:save   # Save the seeded state  <-- YOU ARE HERE

  Later:
  4. npm run snapshot:restore  # Restore saved state
  5. npm run hc:start:snapshot # Start from restored state

Examples:
  npm run snapshot:save
  npm run snapshot:save -- my-test-data
  npm run snapshot:restore
`);
}
