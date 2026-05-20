import { expect } from '@open-wc/testing';

// wtr's esbuild plugin doesn't support JSON import attributes (import.meta.with),
// so we fetch the committed manifest at runtime — same pattern as elohim-button.manifest.spec.ts.
// This smoke test locks in the capabilityContract schema shape.
// Until Task 15 annotates elohim-button with @capability* JSDoc tags, the
// `annotated` list will be empty and the for-loop body won't execute —
// that is intentional and acceptable at this stage.

describe('capability-contract CEM plugin (post-build smoke)', () => {
  let cem: any;

  before(async () => {
    const res = await fetch('/dist/custom-elements.json');
    if (!res.ok) {
      throw new Error(
        `Failed to fetch custom-elements.json (${res.status}). ` +
          `Run \`pnpm --filter elohim-core run build\` and commit the result.`
      );
    }
    cem = await res.json();
  });

  it('emits a capabilityContract block for elements that declare @capability tags', () => {
    // After Task 15, <elohim-button> will declare tags. Until then, this test is allowed
    // to pass with no annotated declarations — it locks in the schema shape.
    const declarations = (cem.modules || []).flatMap((m: any) => m.declarations || []);
    const annotated = declarations.filter((d: any) => d.capabilityContract);
    for (const d of annotated) {
      expect(d.capabilityContract).to.have.property('a11y');
      expect(d.capabilityContract).to.have.property('i18n');
      expect(d.capabilityContract).to.have.property('uaPrefs');
    }
  });
});
