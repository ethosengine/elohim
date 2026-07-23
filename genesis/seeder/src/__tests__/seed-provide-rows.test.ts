import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  deriveActiveHumanIds,
  loadActiveHumanIdsFromDeployments,
  resolveTargetHumanIds,
  type DeploymentsRegistryFile,
} from '../seed-provide-rows.js';

describe('deriveActiveHumanIds', () => {
  it('keeps humans with no suspended flag (absent = active)', () => {
    const data: DeploymentsRegistryFile = {
      humans: [{ humanId: 'human-adam-firstman' }, { humanId: 'human-matthew-manager' }],
    };
    expect(deriveActiveHumanIds(data)).toEqual(['human-adam-firstman', 'human-matthew-manager']);
  });

  it('keeps humans with suspended: false', () => {
    const data: DeploymentsRegistryFile = {
      humans: [{ humanId: 'human-eve-firstwoman', suspended: false }],
    };
    expect(deriveActiveHumanIds(data)).toEqual(['human-eve-firstwoman']);
  });

  it('excludes humans with suspended: true', () => {
    const data: DeploymentsRegistryFile = {
      humans: [
        { humanId: 'human-adam-firstman' },
        { humanId: 'human-pete-pastor', suspended: true },
      ],
    };
    expect(deriveActiveHumanIds(data)).toEqual(['human-adam-firstman']);
  });

  it('mirrors the live deployments.json topology (7 active of 14)', () => {
    // Regression pin: today's deployments.json yields exactly this active set.
    // If this test needs updating, the derivation logic is doing its job —
    // the topology changed and the fallback-trio comment/EXPECTED set should
    // be re-checked too.
    const data: DeploymentsRegistryFile = {
      humans: [
        { humanId: 'human-adam-firstman' },
        { humanId: 'human-matthew-manager' },
        { humanId: 'human-jessica-spouse' },
        { humanId: 'human-james-son' },
        { humanId: 'human-pete-pastor', suspended: true },
        { humanId: 'human-terrance-tutor', suspended: true },
        { humanId: 'human-frank-farmer', suspended: true },
        { humanId: 'human-gertrude-grandma' },
        { humanId: 'human-susan-household' },
        { humanId: 'human-caleb-spouse', suspended: true },
        { humanId: 'human-daniel-brother', suspended: true },
        { humanId: 'human-emma-spouse', suspended: true },
        { humanId: 'human-eve-firstwoman' },
        { humanId: 'human-nancy-neighbor', suspended: true },
      ],
    };
    const active = deriveActiveHumanIds(data);
    expect(active).toEqual([
      'human-adam-firstman',
      'human-matthew-manager',
      'human-jessica-spouse',
      'human-james-son',
      'human-gertrude-grandma',
      'human-susan-household',
      'human-eve-firstwoman',
    ]);
    expect(active).toContain('human-adam-firstman');
    expect(active).toContain('human-matthew-manager');
    expect(active).toContain('human-jessica-spouse');
  });

  it('skips malformed entries (missing/non-string humanId) without throwing', () => {
    const data = {
      humans: [
        { humanId: 'human-adam-firstman' },
        { suspended: false },
        { humanId: 42 },
        null,
      ],
    } as unknown as DeploymentsRegistryFile;
    expect(deriveActiveHumanIds(data)).toEqual(['human-adam-firstman']);
  });

  it('returns an empty array when humans is missing or not an array', () => {
    expect(deriveActiveHumanIds({})).toEqual([]);
    expect(deriveActiveHumanIds({ humans: undefined })).toEqual([]);
    expect(deriveActiveHumanIds({ humans: 'nope' } as unknown as DeploymentsRegistryFile)).toEqual([]);
  });
});

describe('loadActiveHumanIdsFromDeployments', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'provide-rows-deployments-'));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it('reads and derives from a real file', () => {
    const path = join(tmpDir, 'deployments.json');
    writeFileSync(
      path,
      JSON.stringify({
        humans: [
          { humanId: 'human-adam-firstman' },
          { humanId: 'human-pete-pastor', suspended: true },
        ],
      })
    );
    expect(loadActiveHumanIdsFromDeployments(path)).toEqual(['human-adam-firstman']);
  });

  it('throws when the file does not exist', () => {
    expect(() => loadActiveHumanIdsFromDeployments(join(tmpDir, 'nope.json'))).toThrow();
  });

  it('throws when the file is not valid JSON', () => {
    const path = join(tmpDir, 'deployments.json');
    writeFileSync(path, '{ not json');
    expect(() => loadActiveHumanIdsFromDeployments(path)).toThrow();
  });
});

describe('resolveTargetHumanIds', () => {
  let tmpDir: string;
  const originalEnv = process.env.PROVIDE_HUMAN_IDS;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'provide-rows-resolve-'));
    delete process.env.PROVIDE_HUMAN_IDS;
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(console, 'log').mockImplementation(() => {});
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
    if (originalEnv === undefined) {
      delete process.env.PROVIDE_HUMAN_IDS;
    } else {
      process.env.PROVIDE_HUMAN_IDS = originalEnv;
    }
    vi.restoreAllMocks();
  });

  it('PROVIDE_HUMAN_IDS env overrides everything, even a valid deployments.json', () => {
    const path = join(tmpDir, 'deployments.json');
    writeFileSync(path, JSON.stringify({ humans: [{ humanId: 'human-adam-firstman' }] }));
    process.env.PROVIDE_HUMAN_IDS = 'human-eve-firstwoman, human-gertrude-grandma';
    expect(resolveTargetHumanIds(path)).toEqual(['human-eve-firstwoman', 'human-gertrude-grandma']);
  });

  it('derives from deployments.json when no env override is set', () => {
    const path = join(tmpDir, 'deployments.json');
    writeFileSync(
      path,
      JSON.stringify({
        humans: [
          { humanId: 'human-adam-firstman' },
          { humanId: 'human-matthew-manager' },
          { humanId: 'human-jessica-spouse' },
          { humanId: 'human-pete-pastor', suspended: true },
        ],
      })
    );
    expect(resolveTargetHumanIds(path)).toEqual([
      'human-adam-firstman',
      'human-matthew-manager',
      'human-jessica-spouse',
    ]);
  });

  it('falls back to the hardcoded trio with a WARN when deployments.json is unreadable', () => {
    const warnSpy = vi.spyOn(console, 'warn');
    const result = resolveTargetHumanIds(join(tmpDir, 'does-not-exist.json'));
    expect(result).toEqual(['human-matthew-manager', 'human-adam-firstman', 'human-jessica-spouse']);
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('WARN'));
  });

  it('falls back to the hardcoded trio with a WARN when deployments.json yields zero active humans', () => {
    const path = join(tmpDir, 'deployments.json');
    writeFileSync(
      path,
      JSON.stringify({ humans: [{ humanId: 'human-pete-pastor', suspended: true }] })
    );
    const warnSpy = vi.spyOn(console, 'warn');
    const result = resolveTargetHumanIds(path);
    expect(result).toEqual(['human-matthew-manager', 'human-adam-firstman', 'human-jessica-spouse']);
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('WARN'));
  });

  it('WARNs (but does not fall back) when an expected active human is missing from the derived set', () => {
    const path = join(tmpDir, 'deployments.json');
    writeFileSync(path, JSON.stringify({ humans: [{ humanId: 'human-eve-firstwoman' }] }));
    const warnSpy = vi.spyOn(console, 'warn');
    const result = resolveTargetHumanIds(path);
    expect(result).toEqual(['human-eve-firstwoman']);
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('human-adam-firstman'));
  });
});
