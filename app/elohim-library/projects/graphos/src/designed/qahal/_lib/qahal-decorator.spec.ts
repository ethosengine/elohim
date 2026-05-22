import { describe, expect, it } from 'vitest';
import { html, render, type TemplateResult } from 'lit';
import {
  qahalLightDecorator,
  qahalDarkDecorator,
  qahalHighContrastDecorator,
} from './qahal-decorator';

function renderDecoratorToDom(decoratorFn: (story: () => TemplateResult) => TemplateResult) {
  const wrapper = decoratorFn(() => html`<elohim-qahal-test-child></elohim-qahal-test-child>`);
  const host = document.createElement('div');
  render(wrapper, host);
  return host;
}

describe('qahal-decorator', () => {
  describe('qahalLightDecorator', () => {
    it('emits a wrapper div with the EL_TOKENS brand block', () => {
      const host = renderDecoratorToDom(qahalLightDecorator);
      const wrapperStyle = host.querySelector('div')?.getAttribute('style') ?? '';
      expect(wrapperStyle).toContain('--el-cream');
      expect(wrapperStyle).toContain('--el-stone');
      expect(wrapperStyle).toContain('--el-green-deep');
    });

    it("renders the user's story content inside the wrapper", () => {
      const host = renderDecoratorToDom(qahalLightDecorator);
      expect(host.querySelector('elohim-qahal-test-child')).toBeTruthy();
    });

    it('sets the wrapper background to cream (light mode)', () => {
      const host = renderDecoratorToDom(qahalLightDecorator);
      const wrapperStyle = host.querySelector('div')?.getAttribute('style') ?? '';
      expect(wrapperStyle).toMatch(/background:\s*var\(--el-cream\)/);
    });
  });

  describe('qahalDarkDecorator', () => {
    it('sets the wrapper background to night (dark mode)', () => {
      const host = renderDecoratorToDom(qahalDarkDecorator);
      const wrapperStyle = host.querySelector('div')?.getAttribute('style') ?? '';
      expect(wrapperStyle).toMatch(/background:\s*var\(--el-night\)/);
    });
  });

  describe('qahalHighContrastDecorator', () => {
    it('emits explicit border + night-on-cream for max contrast', () => {
      const host = renderDecoratorToDom(qahalHighContrastDecorator);
      const wrapperStyle = host.querySelector('div')?.getAttribute('style') ?? '';
      expect(wrapperStyle).toContain('--el-night');
      expect(wrapperStyle).toMatch(/--elohim-qahal-.+-border:\s*2px solid/);
    });
  });
});
