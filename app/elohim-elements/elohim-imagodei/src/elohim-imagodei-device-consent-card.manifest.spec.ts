import { expect } from '@open-wc/testing';

interface CemDeclaration {
  name: string;
  tagName?: string;
  customElement?: boolean;
  members?: { kind: string; name: string; type?: { text: string } }[];
  events?: { name: string }[];
  capabilityContract?: Record<string, unknown>;
}

interface CemModule {
  declarations?: CemDeclaration[];
}

interface CemManifest {
  modules: CemModule[];
}

const TAG = 'elohim-imagodei-device-consent-card';

let decl: CemDeclaration;
let contract: Record<string, unknown>;

before(async () => {
  const res = await fetch('/dist/custom-elements.json');
  if (!res.ok) {
    throw new Error(
      `Failed to fetch custom-elements.json (${res.status}). ` +
        `Run \`pnpm --filter elohim-imagodei run build\`.`
    );
  }
  const manifest = (await res.json()) as CemManifest;
  const found = manifest.modules
    .flatMap(mod => mod.declarations ?? [])
    .find(d => d.tagName === TAG);
  if (!found) {
    throw new Error(
      `${TAG} declaration not found in custom-elements.json. ` +
        'Run `pnpm --filter elohim-imagodei run analyze`.'
    );
  }
  decl = found;
  contract = (found.capabilityContract as Record<string, unknown>) ?? {};
});

describe(`${TAG} custom-elements-manifest`, () => {
  it('declares the tag and class', () => {
    expect(decl.tagName).to.equal(TAG);
    expect(decl.name).to.equal('ElohimImagodeiDeviceConsentCard');
  });

  for (const name of [
    'request',
    'phase',
    'signer',
    'personLabel',
    'hostLabel',
    'code',
    'expiresAt',
    'refusalCode',
  ]) {
    it(`declares the ${name} property`, () => {
      const prop = decl.members?.find(m => m.kind === 'field' && m.name === name);
      expect(prop).to.exist;
    });
  }

  it('declares the four intent events', () => {
    const names = (decl.events ?? []).map(e => e.name);
    for (const ev of ['approve', 'decline', 'code-copied', 'expired']) {
      expect(names).to.include(ev);
    }
  });
});

describe(`<${TAG}> — capabilityContract manifest`, () => {
  it('declares the precondition gate fields', () => {
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
});
