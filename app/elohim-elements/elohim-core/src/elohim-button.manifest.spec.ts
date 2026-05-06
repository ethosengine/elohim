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
}

interface CemModule {
  declarations?: CemDeclaration[];
}

interface CemManifest {
  modules: CemModule[];
}

describe('elohim-button custom-elements-manifest', () => {
  let declaration: CemDeclaration;

  before(async () => {
    // wtr's esbuild plugin doesn't rewrite JSON import attributes for the
    // browser, so we fetch the committed manifest at runtime. Task 7's
    // freshness check guarantees this file matches what `cem analyze`
    // would produce from the current source.
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
      .find(d => d.tagName === 'elohim-button');
    if (!found) {
      throw new Error(
        'elohim-button declaration not found in custom-elements.json. ' +
          'Run `pnpm --filter elohim-core run analyze` and commit dist/custom-elements.json.'
      );
    }
    declaration = found;
  });

  it('declares the elohim-button tag', () => {
    expect(declaration.tagName).to.equal('elohim-button');
    expect(declaration.name).to.equal('ElohimButton');
  });

  it('is flagged as a custom element', () => {
    expect(declaration.customElement).to.be.true;
  });

  it('exposes variant and disabled properties', () => {
    const fields = declaration.members?.filter(member => member.kind === 'field') ?? [];
    const propNames = fields.map(field => field.name);
    expect(propNames).to.include('variant');
    expect(propNames).to.include('disabled');
  });

  it('declares the default slot', () => {
    const slotNames = (declaration.slots ?? []).map(slot => slot.name);
    expect(slotNames).to.include('');
  });

  it('declares the four --elohim-button-* CSS properties', () => {
    const cssPropNames = (declaration.cssProperties ?? []).map(p => p.name);
    expect(cssPropNames).to.include('--elohim-button-bg');
    expect(cssPropNames).to.include('--elohim-button-fg');
    expect(cssPropNames).to.include('--elohim-button-border');
    expect(cssPropNames).to.include('--elohim-button-radius');
  });

  it('declares the button CSS part', () => {
    const partNames = (declaration.cssParts ?? []).map(p => p.name);
    expect(partNames).to.include('button');
  });

  it('declares the click event from @fires JSDoc', () => {
    const eventNames = (declaration.events ?? []).map(e => e.name);
    expect(eventNames).to.include('click');
  });
});
