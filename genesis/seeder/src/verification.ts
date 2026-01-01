/**
 * Seeding Verification Module
 *
 * Provides pre-flight and post-flight verification to ensure seeding actually works.
 *
 * ## Pre-flight Checks
 * 1. Conductor connectivity - can we call zome functions?
 * 2. Cell discovery - is the cell_id configured correctly?
 * 3. Write test - can we actually create entries?
 * 4. Existing content count - what's already in the conductor?
 *
 * ## Post-flight Verification
 * 1. Content count increased by expected amount
 * 2. Sample verification of specific content IDs
 * 3. Path count verification
 *
 * ## Usage
 * ```typescript
 * const verification = new SeedingVerification(appWs, cellId);
 * const preflight = await verification.runPreflightChecks();
 * if (!preflight.canProceed) {
 *   console.error('Preflight failed:', preflight.errors);
 *   process.exit(1);
 * }
 *
 * // ... run seeding ...
 *
 * const postflight = await verification.runPostflightVerification(expectedCounts, sampleIds);
 * if (!postflight.success) {
 *   console.error('Verification failed:', postflight.errors);
 * }
 * ```
 */

import type { AppClient, CellId } from '@holochain/client';

// =============================================================================
// Types
// =============================================================================

export interface ContentStats {
  total_count: number;
  by_type: Record<string, number>;
}

export interface PathSummary {
  id: string;
  title: string;
  step_count: number;
}

export interface PathIndex {
  paths: PathSummary[];
  total_count: number;
}

export interface PreflightResult {
  canProceed: boolean;
  checks: PreflightCheck[];
  errors: string[];
  warnings: string[];
  existingCounts: {
    content: number;
    paths: number;
  };
}

export interface PreflightCheck {
  name: string;
  status: 'pass' | 'fail' | 'warn';
  message: string;
  details?: string;
}

export interface PostflightResult {
  success: boolean;
  checks: PostflightCheck[];
  errors: string[];
  warnings: string[];
  finalCounts: {
    content: number;
    paths: number;
  };
  delta: {
    content: number;
    paths: number;
  };
}

export interface PostflightCheck {
  name: string;
  status: 'pass' | 'fail' | 'warn';
  message: string;
  expected?: number;
  actual?: number;
  details?: string;
}

export interface ExpectedCounts {
  content: number;
  paths: number;
}

// =============================================================================
// Verification Class
// =============================================================================

export class SeedingVerification {
  private appWs: AppClient;
  private cellId: CellId;
  private zomeName: string;
  private preflightCounts: { content: number; paths: number } | null = null;

  constructor(appWs: AppClient, cellId: CellId, zomeName = 'content_store') {
    this.appWs = appWs;
    this.cellId = cellId;
    this.zomeName = zomeName;
  }

  // ===========================================================================
  // Pre-flight Checks
  // ===========================================================================

  async runPreflightChecks(): Promise<PreflightResult> {
    const checks: PreflightCheck[] = [];
    const errors: string[] = [];
    const warnings: string[] = [];

    console.log('\n' + '═'.repeat(70));
    console.log('🔍 PRE-FLIGHT VERIFICATION');
    console.log('═'.repeat(70));

    // Check 1: Conductor connectivity
    console.log('\n1. Testing conductor connectivity...');
    const connectivityCheck = await this.checkConnectivity();
    checks.push(connectivityCheck);
    this.logCheck(connectivityCheck);

    if (connectivityCheck.status === 'fail') {
      errors.push(connectivityCheck.message);
      return {
        canProceed: false,
        checks,
        errors,
        warnings,
        existingCounts: { content: 0, paths: 0 },
      };
    }

    // Check 2: Get existing content stats
    console.log('\n2. Querying existing content...');
    const contentStatsCheck = await this.checkContentStats();
    checks.push(contentStatsCheck);
    this.logCheck(contentStatsCheck);

    let existingContent = 0;
    if (contentStatsCheck.status === 'pass' && contentStatsCheck.details) {
      try {
        const stats = JSON.parse(contentStatsCheck.details);
        existingContent = stats.total_count;
      } catch {
        // ignore parse error
      }
    } else if (contentStatsCheck.status === 'fail') {
      errors.push(contentStatsCheck.message);
    }

    // Check 3: Get existing paths
    console.log('\n3. Querying existing paths...');
    const pathsCheck = await this.checkPaths();
    checks.push(pathsCheck);
    this.logCheck(pathsCheck);

    let existingPaths = 0;
    if (pathsCheck.status === 'pass' && pathsCheck.details) {
      try {
        const index = JSON.parse(pathsCheck.details);
        existingPaths = index.total_count;
      } catch {
        // ignore parse error
      }
    } else if (pathsCheck.status === 'fail') {
      errors.push(pathsCheck.message);
    }

    // Check 4: Write test (create and verify a test entry)
    console.log('\n4. Testing write capability...');
    const writeCheck = await this.checkWriteCapability();
    checks.push(writeCheck);
    this.logCheck(writeCheck);

    if (writeCheck.status === 'fail') {
      errors.push(writeCheck.message);
    } else if (writeCheck.status === 'warn') {
      warnings.push(writeCheck.message);
    }

    // Store counts for post-flight comparison
    this.preflightCounts = { content: existingContent, paths: existingPaths };

    // Summary
    console.log('\n' + '─'.repeat(70));
    console.log('📊 Pre-flight Summary:');
    console.log(`   Existing content: ${existingContent}`);
    console.log(`   Existing paths:   ${existingPaths}`);
    console.log(`   Checks passed:    ${checks.filter(c => c.status === 'pass').length}/${checks.length}`);

    const canProceed = errors.length === 0 && writeCheck.status !== 'fail';

    if (canProceed) {
      console.log('   ✅ Pre-flight PASSED - ready to seed');
    } else {
      console.log('   ❌ Pre-flight FAILED - cannot proceed');
      errors.forEach(e => console.log(`      - ${e}`));
    }

    if (warnings.length > 0) {
      console.log('   ⚠️  Warnings:');
      warnings.forEach(w => console.log(`      - ${w}`));
    }

    console.log('═'.repeat(70) + '\n');

    return {
      canProceed,
      checks,
      errors,
      warnings,
      existingCounts: { content: existingContent, paths: existingPaths },
    };
  }

  private async checkConnectivity(): Promise<PreflightCheck> {
    try {
      // Try to call a simple zome function
      await this.appWs.callZome({
        cell_id: this.cellId,
        zome_name: this.zomeName,
        fn_name: 'get_content_stats',
        payload: null,
      });

      return {
        name: 'conductor_connectivity',
        status: 'pass',
        message: 'Successfully connected to conductor and called zome function',
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);

      // Check for specific error types
      if (message.includes('cell_id') || message.includes('Cell not found')) {
        return {
          name: 'conductor_connectivity',
          status: 'fail',
          message: 'Cell not found - check cell_id configuration',
          details: message,
        };
      }

      if (message.includes('connection') || message.includes('WebSocket')) {
        return {
          name: 'conductor_connectivity',
          status: 'fail',
          message: 'Cannot connect to conductor - check HOLOCHAIN_APP_URL',
          details: message,
        };
      }

      return {
        name: 'conductor_connectivity',
        status: 'fail',
        message: `Conductor call failed: ${message}`,
        details: message,
      };
    }
  }

  private async checkContentStats(): Promise<PreflightCheck> {
    try {
      const stats = await this.appWs.callZome({
        cell_id: this.cellId,
        zome_name: this.zomeName,
        fn_name: 'get_content_stats',
        payload: null,
      }) as ContentStats;

      return {
        name: 'content_stats',
        status: 'pass',
        message: `Found ${stats.total_count} existing content entries`,
        details: JSON.stringify(stats),
      };
    } catch (error) {
      return {
        name: 'content_stats',
        status: 'fail',
        message: `Failed to get content stats: ${error instanceof Error ? error.message : error}`,
      };
    }
  }

  private async checkPaths(): Promise<PreflightCheck> {
    try {
      const index = await this.appWs.callZome({
        cell_id: this.cellId,
        zome_name: this.zomeName,
        fn_name: 'get_all_paths',
        payload: null,
      }) as PathIndex;

      return {
        name: 'paths',
        status: 'pass',
        message: `Found ${index.total_count} existing paths`,
        details: JSON.stringify(index),
      };
    } catch (error) {
      return {
        name: 'paths',
        status: 'fail',
        message: `Failed to get paths: ${error instanceof Error ? error.message : error}`,
      };
    }
  }

  private async checkWriteCapability(): Promise<PreflightCheck> {
    const testId = `__preflight_test_${Date.now()}`;

    try {
      // Try to create a test content entry
      await this.appWs.callZome({
        cell_id: this.cellId,
        zome_name: this.zomeName,
        fn_name: 'create_content',
        payload: {
          id: testId,
          content_type: '__preflight_test',
          title: 'Preflight Test Entry',
          description: 'This entry tests write capability. Safe to delete.',
          summary: '',
          content: '{}',
          content_format: 'json',
          tags: ['__preflight'],
          source_path: '',
          related_node_ids: [],
          reach: 'private',
          estimated_minutes: 0,
          thumbnail_url: null,
          metadata_json: '{}',
        },
      });

      // Verify we can read it back
      const readBack = await this.appWs.callZome({
        cell_id: this.cellId,
        zome_name: this.zomeName,
        fn_name: 'get_content_by_id',
        payload: { id: testId },
      });

      if (readBack) {
        return {
          name: 'write_capability',
          status: 'pass',
          message: 'Successfully created and verified test entry',
          details: `Created test entry: ${testId}`,
        };
      } else {
        return {
          name: 'write_capability',
          status: 'warn',
          message: 'Entry created but could not be read back immediately (DHT propagation delay)',
          details: testId,
        };
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);

      // If it already exists, that's actually fine for our purposes
      if (message.includes('already exists')) {
        return {
          name: 'write_capability',
          status: 'pass',
          message: 'Write capability confirmed (test entry already exists)',
        };
      }

      return {
        name: 'write_capability',
        status: 'fail',
        message: `Cannot write to conductor: ${message}`,
        details: message,
      };
    }
  }

  // ===========================================================================
  // Post-flight Verification
  // ===========================================================================

  async runPostflightVerification(
    expected: ExpectedCounts,
    sampleContentIds?: string[]
  ): Promise<PostflightResult> {
    const checks: PostflightCheck[] = [];
    const errors: string[] = [];
    const warnings: string[] = [];

    console.log('\n' + '═'.repeat(70));
    console.log('🔍 POST-FLIGHT VERIFICATION');
    console.log('═'.repeat(70));

    // Get final counts
    console.log('\n1. Querying final content count...');
    let finalContent = 0;
    let finalPaths = 0;

    try {
      const stats = await this.appWs.callZome({
        cell_id: this.cellId,
        zome_name: this.zomeName,
        fn_name: 'get_content_stats',
        payload: null,
      }) as ContentStats;
      finalContent = stats.total_count;
      console.log(`   Found ${finalContent} content entries`);
    } catch (error) {
      errors.push(`Failed to get final content stats: ${error}`);
    }

    console.log('\n2. Querying final path count...');
    try {
      const index = await this.appWs.callZome({
        cell_id: this.cellId,
        zome_name: this.zomeName,
        fn_name: 'get_all_paths',
        payload: null,
      }) as PathIndex;
      finalPaths = index.total_count;
      console.log(`   Found ${finalPaths} paths`);
    } catch (error) {
      errors.push(`Failed to get final paths: ${error}`);
    }

    // Calculate deltas
    const preContent = this.preflightCounts?.content ?? 0;
    const prePaths = this.preflightCounts?.paths ?? 0;
    const deltaContent = finalContent - preContent;
    const deltaPaths = finalPaths - prePaths;

    // Check content delta
    console.log('\n3. Verifying content was written...');
    const contentCheck: PostflightCheck = {
      name: 'content_count',
      status: 'pass',
      message: '',
      expected: expected.content,
      actual: deltaContent,
    };

    if (deltaContent === 0 && expected.content > 0) {
      contentCheck.status = 'fail';
      contentCheck.message = `No content was written! Expected ${expected.content}, got 0 new entries`;
      errors.push(contentCheck.message);
    } else if (deltaContent < expected.content * 0.9) {
      // Less than 90% of expected
      contentCheck.status = 'warn';
      contentCheck.message = `Only ${deltaContent}/${expected.content} content entries written (${((deltaContent / expected.content) * 100).toFixed(1)}%)`;
      warnings.push(contentCheck.message);
    } else if (deltaContent >= expected.content) {
      contentCheck.status = 'pass';
      contentCheck.message = `✓ ${deltaContent} content entries written (expected ${expected.content})`;
    } else {
      contentCheck.status = 'pass';
      contentCheck.message = `${deltaContent} content entries written`;
    }
    checks.push(contentCheck);
    this.logCheck(contentCheck);

    // Check paths delta
    console.log('\n4. Verifying paths were written...');
    const pathsCheck: PostflightCheck = {
      name: 'paths_count',
      status: 'pass',
      message: '',
      expected: expected.paths,
      actual: deltaPaths,
    };

    if (deltaPaths === 0 && expected.paths > 0) {
      pathsCheck.status = 'fail';
      pathsCheck.message = `No paths were written! Expected ${expected.paths}, got 0 new paths`;
      errors.push(pathsCheck.message);
    } else if (deltaPaths < expected.paths) {
      pathsCheck.status = 'warn';
      pathsCheck.message = `Only ${deltaPaths}/${expected.paths} paths written`;
      warnings.push(pathsCheck.message);
    } else {
      pathsCheck.status = 'pass';
      pathsCheck.message = `✓ ${deltaPaths} paths written (expected ${expected.paths})`;
    }
    checks.push(pathsCheck);
    this.logCheck(pathsCheck);

    // Sample verification
    if (sampleContentIds && sampleContentIds.length > 0) {
      console.log(`\n5. Verifying ${sampleContentIds.length} sample content entries...`);
      const sampleCheck = await this.verifySampleContent(sampleContentIds);
      checks.push(sampleCheck);
      this.logCheck(sampleCheck);

      if (sampleCheck.status === 'fail') {
        errors.push(sampleCheck.message);
      } else if (sampleCheck.status === 'warn') {
        warnings.push(sampleCheck.message);
      }
    }

    // Summary
    const success = errors.length === 0;

    console.log('\n' + '─'.repeat(70));
    console.log('📊 Post-flight Summary:');
    console.log(`   Content: ${preContent} → ${finalContent} (Δ ${deltaContent >= 0 ? '+' : ''}${deltaContent})`);
    console.log(`   Paths:   ${prePaths} → ${finalPaths} (Δ ${deltaPaths >= 0 ? '+' : ''}${deltaPaths})`);
    console.log(`   Expected: ${expected.content} content, ${expected.paths} paths`);

    if (success) {
      console.log('   ✅ Post-flight PASSED - seeding verified');
    } else {
      console.log('   ❌ Post-flight FAILED - seeding may have failed');
      errors.forEach(e => console.log(`      - ${e}`));
    }

    if (warnings.length > 0) {
      console.log('   ⚠️  Warnings:');
      warnings.forEach(w => console.log(`      - ${w}`));
    }

    console.log('═'.repeat(70) + '\n');

    return {
      success,
      checks,
      errors,
      warnings,
      finalCounts: { content: finalContent, paths: finalPaths },
      delta: { content: deltaContent, paths: deltaPaths },
    };
  }

  private async verifySampleContent(ids: string[]): Promise<PostflightCheck> {
    let found = 0;
    let missing: string[] = [];

    for (const id of ids.slice(0, 10)) {
      // Check up to 10 samples
      try {
        const result = await this.appWs.callZome({
          cell_id: this.cellId,
          zome_name: this.zomeName,
          fn_name: 'get_content_by_id',
          payload: { id },
        });

        if (result) {
          found++;
        } else {
          missing.push(id);
        }
      } catch {
        missing.push(id);
      }
    }

    const checked = Math.min(ids.length, 10);

    if (found === checked) {
      return {
        name: 'sample_verification',
        status: 'pass',
        message: `All ${found} sample entries verified`,
        expected: checked,
        actual: found,
      };
    } else if (found > 0) {
      return {
        name: 'sample_verification',
        status: 'warn',
        message: `${found}/${checked} sample entries found, missing: ${missing.slice(0, 3).join(', ')}${missing.length > 3 ? '...' : ''}`,
        expected: checked,
        actual: found,
      };
    } else {
      return {
        name: 'sample_verification',
        status: 'fail',
        message: `None of the ${checked} sample entries found - content may not have been written`,
        expected: checked,
        actual: 0,
      };
    }
  }

  // ===========================================================================
  // Helpers
  // ===========================================================================

  private logCheck(check: PreflightCheck | PostflightCheck): void {
    const icon = check.status === 'pass' ? '✓' : check.status === 'fail' ? '✗' : '⚠';
    const color = check.status === 'pass' ? '' : check.status === 'fail' ? '' : '';
    console.log(`   ${icon} ${check.message}`);
    if (check.status === 'fail' && 'details' in check && check.details) {
      console.log(`     Details: ${check.details.slice(0, 100)}...`);
    }
  }
}

// =============================================================================
// Standalone Functions
// =============================================================================

/**
 * Quick check if conductor is reachable and cell is configured
 */
export async function quickConnectivityCheck(
  appWs: AppClient,
  cellId: CellId,
  zomeName = 'content_store'
): Promise<{ ok: boolean; error?: string; stats?: ContentStats }> {
  try {
    const stats = await appWs.callZome({
      cell_id: cellId,
      zome_name: zomeName,
      fn_name: 'get_content_stats',
      payload: null,
    }) as ContentStats;

    return { ok: true, stats };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * Check if specific content IDs exist
 */
export async function checkContentExists(
  appWs: AppClient,
  cellId: CellId,
  contentIds: string[],
  zomeName = 'content_store'
): Promise<{ exists: string[]; missing: string[] }> {
  const exists: string[] = [];
  const missing: string[] = [];

  for (const id of contentIds) {
    try {
      const result = await appWs.callZome({
        cell_id: cellId,
        zome_name: zomeName,
        fn_name: 'get_content_by_id',
        payload: { id },
      });

      if (result) {
        exists.push(id);
      } else {
        missing.push(id);
      }
    } catch {
      missing.push(id);
    }
  }

  return { exists, missing };
}
