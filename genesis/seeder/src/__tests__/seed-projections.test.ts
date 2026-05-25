import { describe, it, expect } from 'vitest';
import { buildProjectionCommitmentBody, type ProjectionSpec } from '../seed-projections.js';

describe('buildProjectionCommitmentBody', () => {
  const baseSpec: ProjectionSpec = {
    stewardHumanId: 'human-matthew-manager',
    stewardArchetype: 'desktop',
    doorwayId: 'alpha-elohim-host',
    eprId: 'lamad-spa',
    urlPath: '/lamad',
    mode: 'cached',
    reach: 'commons',
    baseHref: '/lamad/',
    entryFile: 'index.html',
    redirectsFrom: [],
    previewEprRef: null,
    gateHints: [],
    deadEnd: false,
    stewardDirectEndpoint: null,
  };

  it('builds a commons-reach lamad projection at /lamad', () => {
    const body = buildProjectionCommitmentBody(baseSpec);
    expect(body.action).toBe('project-epr');
    expect(body.inScopeOf).toContain('doorway:alpha-elohim-host');
    expect(body.inScopeOf).toContain('epr:lamad-spa');
    const meta = JSON.parse(body.metadataJson);
    expect(meta.urlPath).toBe('/lamad');
    expect(meta.mode).toBe('cached');
    expect(meta.reach).toBe('commons');
  });

  it('produces deterministic id for same spec (idempotent re-seed)', () => {
    const a = buildProjectionCommitmentBody(baseSpec);
    const b = buildProjectionCommitmentBody(baseSpec);
    expect(a.id).toBe(b.id);
  });

  it('produces different ids for different (doorway, epr) pairs', () => {
    const a = buildProjectionCommitmentBody(baseSpec);
    const b = buildProjectionCommitmentBody({ ...baseSpec, doorwayId: 'elohim-host' });
    const c = buildProjectionCommitmentBody({ ...baseSpec, eprId: 'elohim-host-landing' });
    expect(a.id).not.toBe(b.id);
    expect(a.id).not.toBe(c.id);
    expect(b.id).not.toBe(c.id);
  });
});
