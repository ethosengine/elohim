import { describe, it, expect } from 'vitest';
import { validateRecord, type DeploymentRecord } from '../validate-deployments.js';

// The shared consolidated template all non-adam humans sed-render from. It
// exists in the repo, so validateRecord's existsSync check resolves it.
const TEMPLATE_REL = 'genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml';

const knownHumans = new Set(['human-testperson']);
const knownDevices = new Set(['device-family-node-base']);

function consolidated(overrides: Partial<DeploymentRecord> = {}): DeploymentRecord {
  return {
    name: 'testperson',
    role: 'tester',
    humanLabel: 'testperson',
    humanId: 'human-testperson',
    pattern: 'consolidated',
    template: TEMPLATE_REL,
    deviceArchetype: 'device-family-node-base',
    nodeTypes: ['edge'],
    ...overrides,
  } as DeploymentRecord;
}

describe('validateRecord — consolidated pattern accepts template OR manifest', () => {
  it('accepts a consolidated record that sed-renders from a template (the 13-of-14 convention)', () => {
    // RED before the fix: the validator hardcoded `consolidated → manifest`.
    expect(validateRecord(consolidated(), knownHumans, knownDevices)).toEqual([]);
  });

  it('still accepts a consolidated record with an explicit manifest (adam-style)', () => {
    const rec = consolidated({ template: undefined, manifest: TEMPLATE_REL });
    expect(validateRecord(rec, knownHumans, knownDevices)).toEqual([]);
  });

  it('rejects a consolidated record with neither manifest nor template', () => {
    const rec = consolidated({ template: undefined, manifest: undefined });
    const errors = validateRecord(rec, knownHumans, knownDevices);
    expect(errors.some(e => /requires 'manifest' or 'template'/.test(e))).toBe(true);
  });

  it('rejects a consolidated record whose deployment source file is missing', () => {
    const rec = consolidated({ template: 'genesis/orchestrator/manifests/humans/does-not-exist.yaml' });
    const errors = validateRecord(rec, knownHumans, knownDevices);
    expect(errors.some(e => /missing/.test(e))).toBe(true);
  });
});

describe('validateRecord — legacy pattern still requires template + sizing (regression)', () => {
  it('accepts a legacy record with template + resource sizing', () => {
    const rec = consolidated({
      pattern: 'legacy',
      manifest: undefined,
      template: TEMPLATE_REL,
      edgenodeMemoryRequest: '1Gi',
      edgenodeMemoryLimit: '2Gi',
      edgenodeCpuRequest: '250m',
      edgenodeCpuLimit: '2000m',
    });
    expect(validateRecord(rec, knownHumans, knownDevices)).toEqual([]);
  });
});
