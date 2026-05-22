import { expect } from '@open-wc/testing';

interface CemDeclaration {
  name: string;
  tagName?: string;
  customElement?: boolean;
  members?: { kind: string; name: string; type?: { text: string } }[];
  capabilityContract?: Record<string, unknown>;
}

interface CemModule {
  declarations?: CemDeclaration[];
}

interface CemManifest {
  modules: CemModule[];
}

// Shared manifest fetch — both describe blocks consume the same parsed result.
let decl: CemDeclaration;
let contract: Record<string, unknown>;

before(async () => {
  const res = await fetch('/dist/custom-elements.json');
  if (!res.ok) {
    throw new Error(
      `Failed to fetch custom-elements.json (${res.status}). ` +
        `Run \`pnpm --filter elohim-imagodei run build\` and commit the result.`
    );
  }
  const manifest = (await res.json()) as CemManifest;
  const found = manifest.modules
    .flatMap(mod => mod.declarations ?? [])
    .find(d => d.tagName === 'elohim-imagodei-protected-tier-marker');
  if (!found) {
    throw new Error(
      'elohim-imagodei-protected-tier-marker declaration not found in custom-elements.json. ' +
        'Run `pnpm --filter elohim-imagodei run analyze` and commit dist/custom-elements.json.'
    );
  }
  decl = found;
  contract = (found.capabilityContract as Record<string, unknown>) ?? {};
});

describe('elohim-imagodei-protected-tier-marker custom-elements-manifest', () => {
  it('declares the tag', () => {
    expect(decl.tagName).to.equal('elohim-imagodei-protected-tier-marker');
    expect(decl.name).to.equal('ElohimImagodeiProtectedTierMarker');
  });

  it('declares the tier property', () => {
    const prop = decl.members?.find(m => m.kind === 'field' && m.name === 'tier');
    expect(prop).to.exist;
  });
});

describe('<elohim-imagodei-protected-tier-marker> — capabilityContract manifest', () => {
  it('declares the precondition gate fields', () => {
    expect(contract).to.exist;
    expect(contract).to.have.property('a11y');
    expect(contract).to.have.property('i18n');
    expect(contract).to.have.property('uaPrefs');
  });

  it('claims maxLens=standard', () => {
    expect(contract.maxLens).to.equal('standard');
  });

  it('claims maxStimulus=still', () => {
    expect(contract.maxStimulus).to.equal('still');
  });

  it('claims both themes (light and dark)', () => {
    expect(contract.themes).to.deep.equal(['light', 'dark']);
  });

  it('claims contentCertainty=observed-private', () => {
    expect(contract.contentCertainty).to.equal('observed-private');
  });
});
