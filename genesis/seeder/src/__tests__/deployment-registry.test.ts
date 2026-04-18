import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { loadDeploymentRegistry } from '../deployment-registry.js';

describe('loadDeploymentRegistry', () => {
  let tmpDir: string;
  let registryPath: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'dep-reg-'));
    registryPath = join(tmpDir, 'deployments.json');
    writeFileSync(registryPath, JSON.stringify({
      humans: [
        { humanId: 'human-adam-firstman', name: 'adam' },
        { humanId: 'human-matthew-manager', name: 'matthew' },
      ],
    }));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it('reads humanIds from a registry file', () => {
    const reg = loadDeploymentRegistry({ registryPath });
    expect(reg.source).toBe('file');
    expect(reg.path).toBe(registryPath);
    expect(reg.deployedHumanIds.has('human-adam-firstman')).toBe(true);
    expect(reg.deployedHumanIds.has('human-matthew-manager')).toBe(true);
    expect(reg.deployedHumanIds.size).toBe(2);
  });

  it('explicit deployedHumans list overrides registryPath', () => {
    const reg = loadDeploymentRegistry({
      registryPath,
      deployedHumans: ['human-eve-firstwoman'],
    });
    expect(reg.source).toBe('flag');
    expect(reg.deployedHumanIds.has('human-eve-firstwoman')).toBe(true);
    expect(reg.deployedHumanIds.has('human-adam-firstman')).toBe(false);
  });

  it('rejects short names in deployedHumans with a clear error', () => {
    expect(() =>
      loadDeploymentRegistry({ deployedHumans: ['adam'] })
    ).toThrow(/full humanId/i);
  });

  it('throws if registryPath does not exist', () => {
    expect(() =>
      loadDeploymentRegistry({ registryPath: '/nonexistent/path.json' })
    ).toThrow(/deployment registry/i);
  });

  it('throws if registry JSON has no humans array', () => {
    writeFileSync(registryPath, JSON.stringify({}));
    expect(() =>
      loadDeploymentRegistry({ registryPath })
    ).toThrow(/humans/i);
  });
});
