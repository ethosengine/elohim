import { expect } from '@open-wc/testing';

interface CemDeclaration {
  name: string;
  tagName?: string;
  customElement?: boolean;
  members?: { kind: string; name: string; type?: { text: string } }[];
  slots?: { name: string }[];
  cssProperties?: { name: string }[];
  cssParts?: { name: string }[];
  events?: { name: string }[];
  // Emitted by the capability-contract CEM plugin
  capabilityContract?: Record<string, unknown>;
}

interface CemModule {
  declarations?: CemDeclaration[];
}

interface CemManifest {
  modules: CemModule[];
}

describe('elohim-compute-tile custom-elements-manifest', () => {
  let declaration: CemDeclaration;

  before(async () => {
    const res = await fetch('/dist/custom-elements.json');
    if (!res.ok) {
      throw new Error(
        `Failed to fetch custom-elements.json (${res.status}). ` +
          `Run \`pnpm --filter elohim-core run build\` and commit the result.`
      );
    }
    const manifest = (await res.json()) as CemManifest;
    const found = manifest.modules
      .flatMap(mod => mod.declarations ?? [])
      .find(d => d.tagName === 'elohim-compute-tile');
    if (!found) {
      throw new Error(
        'elohim-compute-tile declaration not found in custom-elements.json. ' +
          'Run `pnpm --filter elohim-core run build` and commit dist/custom-elements.json.'
      );
    }
    declaration = found;
  });

  it('declares the elohim-compute-tile tag', () => {
    expect(declaration.tagName).to.equal('elohim-compute-tile');
    expect(declaration.name).to.equal('ElohimComputeTile');
  });

  it('is flagged as a custom element', () => {
    expect(declaration.customElement).to.be.true;
  });

  it('exposes value and tileState properties', () => {
    const fields = declaration.members?.filter(m => m.kind === 'field') ?? [];
    const propNames = fields.map(f => f.name);
    expect(propNames).to.include('value');
    expect(propNames).to.include('tileState');
  });

  it('exposes lastSampleAt and sourceRoute properties', () => {
    const fields = declaration.members?.filter(m => m.kind === 'field') ?? [];
    const propNames = fields.map(f => f.name);
    expect(propNames).to.include('lastSampleAt');
    expect(propNames).to.include('sourceRoute');
  });

  it('declares the label named slot', () => {
    const slotNames = (declaration.slots ?? []).map(s => s.name);
    expect(slotNames).to.include('label');
  });

  it('declares CSS custom properties for the override surface', () => {
    const cssPropNames = (declaration.cssProperties ?? []).map(p => p.name);
    expect(cssPropNames).to.include('--elohim-compute-tile-bg');
    expect(cssPropNames).to.include('--elohim-compute-tile-fg');
    expect(cssPropNames).to.include('--elohim-compute-tile-border');
    expect(cssPropNames).to.include('--elohim-compute-tile-radius');
    expect(cssPropNames).to.include('--elohim-compute-tile-padding');
    expect(cssPropNames).to.include('--elohim-compute-tile-gauge-fill');
    expect(cssPropNames).to.include('--elohim-compute-tile-meta-color');
  });

  it('declares the container CSS part', () => {
    const partNames = (declaration.cssParts ?? []).map(p => p.name);
    expect(partNames).to.include('container');
    expect(partNames).to.include('header');
    expect(partNames).to.include('body');
    expect(partNames).to.include('value-primary');
    expect(partNames).to.include('gauge');
    expect(partNames).to.include('meta');
  });
});

describe('<elohim-compute-tile> — capabilityContract manifest', () => {
  let contract: Record<string, unknown>;

  before(async () => {
    const response = await fetch('/dist/custom-elements.json');
    const cem = (await response.json()) as CemManifest;
    const decl = cem.modules
      .flatMap(m => m.declarations || [])
      .find(d => d.name === 'ElohimComputeTile');
    contract = (decl?.capabilityContract as Record<string, unknown>) ?? {};
  });

  it('has the three precondition gate fields', () => {
    expect(contract).to.exist;
    expect(contract).to.have.property('a11y');
    expect(contract).to.have.property('i18n');
    expect(contract).to.have.property('uaPrefs');
  });

  it('claims maxLens=trace (full triptych + debug + source-route)', () => {
    expect(contract.maxLens).to.equal('trace');
  });

  it('claims maxStimulus=still (compute tile never animates beyond forced-colors safe)', () => {
    expect(contract.maxStimulus).to.equal('still');
  });

  it('claims both themes (light and dark)', () => {
    expect(contract.themes).to.deep.equal(['light', 'dark']);
  });

  it('claims both contrast tiers (normal and high)', () => {
    expect(contract.contrast).to.deep.equal(['normal', 'high']);
  });

  it('claims contentCertainty=observed (compute sample freshness matters)', () => {
    expect(contract.contentCertainty).to.equal('observed');
  });

  it('marks all designed content states correctly', () => {
    const states = contract.states as Record<string, string>;
    expect(states).to.exist;
    expect(states['empty']).to.equal('designed');
    expect(states['loading']).to.equal('designed');
    expect(states['error']).to.equal('designed');
    expect(states['stale']).to.equal('designed');
    expect(states['offline']).to.equal('designed');
    expect(states['contested']).to.equal('n/a');
    expect(states['unauthorized']).to.equal('n/a');
  });

  it('requires pilot | steward | elohim-support standing', () => {
    const standings = contract.standings as Record<string, string[]>;
    expect(standings).to.exist;
    expect(standings.required).to.deep.equal(['pilot | steward | elohim-support']);
  });

  it('claims both textuality modes (textual and symbolic)', () => {
    expect(contract.textuality).to.deep.equal(['textual', 'symbolic']);
  });
});
