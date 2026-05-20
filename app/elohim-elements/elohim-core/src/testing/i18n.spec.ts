import { expect, fixture, html } from '@open-wc/testing';
import { LitElement } from 'lit';
import { renderInLocale, scanForHardcodedStrings, requiresLogicalProperties } from './i18n.js';

class HardcodedThing extends LitElement {
  override render() {
    return html`
      <span aria-label="Save">Save</span>
    `;
  }
}
customElements.define('hardcoded-thing', HardcodedThing);

describe('i18n harness', () => {
  describe('renderInLocale', () => {
    it('renders the element with the document direction set for he-IL', async () => {
      const el = await renderInLocale(
        'he-IL',
        html`
          <span></span>
        `
      );
      expect(el.ownerDocument.documentElement.getAttribute('dir')).to.equal('rtl');
    });

    it('restores LTR direction after rendering ends', async () => {
      await renderInLocale(
        'he-IL',
        html`
          <span></span>
        `
      );
      // Within the same test, a subsequent en call sets dir=ltr.
      // Cross-test cleanup happens via afterEach (see i18n.ts).
      const el = await renderInLocale(
        'en',
        html`
          <span></span>
        `
      );
      expect(el.ownerDocument.documentElement.getAttribute('dir')).to.equal('ltr');
    });

    it('cleans up dir after the test (via afterEach)', async () => {
      document.documentElement.removeAttribute('dir');
      await renderInLocale(
        'he-IL',
        html`
          <span></span>
        `
      );
      expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
      // After this test finishes, afterEach fires and restores dir.
      // The next test in the same file can verify.
    });

    it('confirms dir is restored after a renderInLocale test', () => {
      // Runs after the previous test's afterEach.
      // dir should NOT still be 'rtl'.
      expect(document.documentElement.getAttribute('dir')).to.not.equal('rtl');
    });
  });

  describe('scanForHardcodedStrings', () => {
    it('flags element render output with hardcoded text content', async () => {
      const el = await fixture<HardcodedThing>(html`
        <hardcoded-thing></hardcoded-thing>
      `);
      const findings = scanForHardcodedStrings(el.shadowRoot!.innerHTML);
      expect(findings.length).to.be.greaterThan(0);
    });

    it('returns empty findings when content uses placeholders only', () => {
      const findings = scanForHardcodedStrings('<span aria-label="{{label}}">{{text}}</span>');
      expect(findings).to.deep.equal([]);
    });

    it('flags hardcoded text that appears after whitespace (prettified HTML)', () => {
      const findings = scanForHardcodedStrings('<span>\n  Save\n</span>');
      expect(findings).to.include('Save');
    });
  });

  describe('requiresLogicalProperties', () => {
    it('flags physical-property CSS rules', () => {
      const findings = requiresLogicalProperties('.x { margin-left: 8px; padding-right: 4px; }');
      expect(findings).to.have.lengthOf(2);
      expect(findings[0]).to.contain('margin-left');
      expect(findings[1]).to.contain('padding-right');
    });

    it('does not flag logical properties', () => {
      const findings = requiresLogicalProperties('.x { margin-inline-start: 8px; }');
      expect(findings).to.have.lengthOf(0);
    });
  });
});
