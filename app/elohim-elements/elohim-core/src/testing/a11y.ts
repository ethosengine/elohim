/**
 * A11y precondition-gate helpers — usable by every element's spec.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §8.1
 */

/* eslint-disable import/no-extraneous-dependencies */
import { aTimeout } from '@open-wc/testing';
import axe from 'axe-core';

import type { Result as AxeResultType } from 'axe-core';
/* eslint-enable import/no-extraneous-dependencies */

export interface AxeResult {
  violations: AxeResultType[];
}

/**
 * Scans the element subtree with axe-core. Returns violations.
 * Fail your test on `violations.length > 0`.
 */
export async function axeScan(element: Element): Promise<AxeResult> {
  const result = await axe.run(element as unknown as HTMLElement);
  return { violations: result.violations };
}

/**
 * Asserts the element is keyboard-focusable. Either the host receives focus
 * directly or focus delegates to a descendant via delegatesFocus.
 * Throws if focus does not land on the host or any of its shadow descendants.
 */
export async function expectKeyboardFocusable(element: HTMLElement): Promise<void> {
  element.focus();
  await aTimeout(0);
  const active = document.activeElement;
  const inside =
    active === element ||
    // eslint-disable-next-line @typescript-eslint/prefer-nullish-coalescing
    (active && element.shadowRoot?.contains(active as Node)) ||
    (active && element.contains(active as Node));
  if (!inside) {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-type-assertion
    const activeEl = active as Element | null;
    throw new Error(
      `expectKeyboardFocusable: focus did not land on or within <${element.tagName.toLowerCase()}>. ` +
        `activeElement was ${activeEl?.tagName.toLowerCase() ?? 'null'}.`
    );
  }
}
