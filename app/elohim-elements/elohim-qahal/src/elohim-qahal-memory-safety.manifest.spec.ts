import { expect } from '@open-wc/testing';

interface CemDeclaration {
  name: string;
  tagName?: string;
  customElement?: boolean;
  members?: { kind: string; name: string; type?: { text: string } }[];
  cssProperties?: { name: string }[];
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
        `Run \`pnpm --filter elohim-qahal run build\` and commit the result.`
    );
  }
  const manifest = (await res.json()) as CemManifest;
  const found = manifest.modules
    .flatMap(mod => mod.declarations ?? [])
    .find(d => d.tagName === 'elohim-qahal-memory-safety');
  if (!found) {
    throw new Error(
      'elohim-qahal-memory-safety declaration not found in custom-elements.json. ' +
        'Run `pnpm --filter elohim-qahal run analyze` and commit dist/custom-elements.json.'
    );
  }
  decl = found;
  contract = (found.capabilityContract as Record<string, unknown>) ?? {};
});

describe('elohim-qahal-memory-safety custom-elements-manifest', () => {
  it('declares the tag', () => {
    expect(decl.tagName).to.equal('elohim-qahal-memory-safety');
    expect(decl.name).to.equal('ElohimQahalMemorySafety');
  });

  it('declares the feltStatus property', () => {
    const prop = decl.members?.find(m => m.kind === 'field' && m.name === 'feltStatus');
    expect(prop).to.exist;
  });

  it('declares the contentLabel property', () => {
    const prop = decl.members?.find(m => m.kind === 'field' && m.name === 'contentLabel');
    expect(prop).to.exist;
  });

  it('declares the --elohim-memory-safety-bg CSS property', () => {
    const prop = decl.cssProperties?.find(p => p.name === '--elohim-memory-safety-bg');
    expect(prop).to.exist;
  });

  it('declares the --elohim-memory-safety-fg CSS property', () => {
    const prop = decl.cssProperties?.find(p => p.name === '--elohim-memory-safety-fg');
    expect(prop).to.exist;
  });

  it('declares the --elohim-memory-safety-status-not-yet-seen-color CSS property', () => {
    const prop = decl.cssProperties?.find(
      p => p.name === '--elohim-memory-safety-status-not-yet-seen-color'
    );
    expect(prop).to.exist;
  });
});

describe('<elohim-qahal-memory-safety> — capabilityContract manifest', () => {
  it('declares the precondition gate fields', () => {
    expect(contract).to.exist;
    expect(contract).to.have.property('a11y');
    expect(contract).to.have.property('i18n');
    expect(contract).to.have.property('uaPrefs');
  });

  it('claims maxLens=detail', () => {
    expect(contract.maxLens).to.equal('detail');
  });

  it('claims maxStimulus=still', () => {
    expect(contract.maxStimulus).to.equal('still');
  });

  it('claims both themes (light and dark)', () => {
    expect(contract.themes).to.deep.equal(['light', 'dark']);
  });

  it('claims contentCertainty=observed', () => {
    expect(contract.contentCertainty).to.equal('observed');
  });

  it('claims textuality=textual', () => {
    expect(contract.textuality).to.deep.equal(['textual']);
  });
});
