import { expect } from '@open-wc/testing';

interface CemDeclaration {
  name: string;
  tagName?: string;
  members?: { kind: string; name: string }[];
  slots?: { name: string }[];
  cssProperties?: { name: string }[];
  cssParts?: { name: string }[];
  events?: { name: string }[];
}

interface CemModule {
  declarations?: CemDeclaration[];
}

interface CemManifest {
  modules: CemModule[];
}

// JSON import attributes (`with { type: 'json' }`) aren't currently rewritten
// by web-test-runner's esbuild plugin in this setup, so we fetch the manifest
// from the dev-server-served `dist/` instead. The freshness check (Task 7)
// guarantees this committed file matches the analyzer output.
let manifest: CemManifest;
let declaration: CemDeclaration | undefined;

before(async () => {
  const res = await fetch('/dist/custom-elements.json');
  if (!res.ok) {
    throw new Error(`Failed to load custom-elements.json: ${res.status}`);
  }
  manifest = (await res.json()) as CemManifest;
  declaration = manifest.modules
    .flatMap(mod => mod.declarations ?? [])
    .find(d => d.tagName === 'elohim-button');
});

describe('elohim-button custom-elements-manifest', () => {
  it('declares the elohim-button tag', () => {
    expect(declaration).to.exist;
    expect(declaration!.name).to.equal('ElohimButton');
  });

  it('exposes variant and disabled properties', () => {
    const fields = declaration!.members?.filter(member => member.kind === 'field') ?? [];
    const propNames = fields.map(field => field.name);
    expect(propNames).to.include('variant');
    expect(propNames).to.include('disabled');
  });

  it('declares the default slot', () => {
    const slotNames = (declaration!.slots ?? []).map(slot => slot.name);
    expect(slotNames).to.include('');
  });

  it('declares the four --elohim-button-* CSS properties', () => {
    const cssPropNames = (declaration!.cssProperties ?? []).map(p => p.name);
    expect(cssPropNames).to.include('--elohim-button-bg');
    expect(cssPropNames).to.include('--elohim-button-fg');
    expect(cssPropNames).to.include('--elohim-button-border');
    expect(cssPropNames).to.include('--elohim-button-radius');
  });

  it('declares the button CSS part', () => {
    const partNames = (declaration!.cssParts ?? []).map(p => p.name);
    expect(partNames).to.include('button');
  });
});
