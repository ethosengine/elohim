import { describe, it, expect } from 'vitest';
import {
  buildProjectionCommitmentBody,
  defaultProjectionSeeds,
  type ProjectionSpec,
} from '../seed-projections.js';

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
    routeClaims: null,
    redirectTemplates: [],
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

describe('defaultProjectionSeeds', () => {
  it('default seed set has 6 commitments total (landing × 2 + lamad × 2 + portal × 2)', () => {
    expect(defaultProjectionSeeds().length).toBe(6);
  });

  it('grants lamad routeClaims + the legacy resource redirect template', () => {
    const lamad = defaultProjectionSeeds().find(
      (s) => s.eprId === 'lamad-spa' && s.doorwayId === 'alpha-elohim-host',
    )!;
    const meta = JSON.parse(buildProjectionCommitmentBody(lamad).metadataJson);
    expect(meta.routeClaims.schemaVersion).toBe(1);
    expect(meta.routeClaims.claims).toEqual([
      { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
    ]);
    expect(meta.redirectTemplates).toEqual([{ from: '/lamad/resource/{id}', to: '/epr/{id}' }]);
  });

  it('default seed set includes /auth/portal projections on both doorways', () => {
    const seeds = defaultProjectionSeeds();
    const portalSeeds = seeds.filter((s) => s.eprId === 'imagodei-portal');
    expect(portalSeeds.length).toBe(2);
    expect(portalSeeds.every((s) => s.urlPath === '/auth/portal')).toBe(true);
    expect(portalSeeds.every((s) => s.baseHref === '/auth/portal/')).toBe(true);
    expect(portalSeeds.map((s) => s.doorwayId).sort()).toEqual([
      'alpha-elohim-host',
      'apex-elohim-host',
    ]);
  });
});
