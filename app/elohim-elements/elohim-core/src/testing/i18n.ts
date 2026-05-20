/**
 * I18n precondition-gate helpers.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §8.2
 */

import type { TemplateResult } from 'lit';

/* eslint-disable-next-line import/no-extraneous-dependencies */
import { fixture } from '@open-wc/testing';

import { setLocale } from '../localize/runtime.js';

const RTL_LOCALES = ['he', 'ar', 'fa', 'ur'] as const;

function isRtlLocale(locale: string): boolean {
  const lang = locale.split('-')[0]!.toLowerCase();
  return (RTL_LOCALES as readonly string[]).includes(lang);
}

/**
 * Renders a Lit template under a specific locale, setting document direction
 * and the localize runtime. Restores prior state afterward.
 */
export async function renderInLocale<T extends Element = HTMLElement>(
  locale: string,
  template: TemplateResult
): Promise<T> {
  const root = document.documentElement;
  const priorDir = root.getAttribute('dir');
  const priorLang = root.getAttribute('lang');

  root.setAttribute('lang', locale);
  root.setAttribute('dir', isRtlLocale(locale) ? 'rtl' : 'ltr');

  const baseLocale = locale.split('-')[0]!.toLowerCase();
  if (baseLocale !== 'en') {
    try {
      await setLocale(baseLocale);
    } catch {
      // locale bundle absent — leave at source
    }
  }

  try {
    return await fixture<T>(template);
  } finally {
    if (priorDir === null) {
      root.removeAttribute('dir');
    } else {
      root.setAttribute('dir', priorDir);
    }
    if (priorLang === null) {
      root.removeAttribute('lang');
    } else {
      root.setAttribute('lang', priorLang);
    }
  }
}

/**
 * Scans rendered HTML for likely hardcoded user-facing strings.
 * Heuristic: visible text or aria-* attribute values that are not pure
 * whitespace, not numeric, and not template placeholder syntax.
 *
 * Returns the offending fragments.
 */
export function scanForHardcodedStrings(renderedHtml: string): string[] {
  const findings: string[] = [];
  // text content between tags
  const textMatches = renderedHtml.matchAll(/>([^<>{}]*?)</gu);
  for (const m of textMatches) {
    const text = m[1]?.trim() ?? '';
    if (text.length > 0 && !/^[\d\s.,:;%-]+$/u.test(text)) findings.push(text);
  }
  // aria-* attribute values
  const ariaMatches = renderedHtml.matchAll(/aria-[a-z]+="([^"{}]+)"/gu);
  for (const m of ariaMatches) {
    const value = m[1]?.trim() ?? '';
    if (value.length > 0 && !/^[\d\s.,:;%-]+$/u.test(value)) findings.push(value);
  }
  return findings;
}

/**
 * Physical CSS property patterns that should be replaced with logical
 * equivalents for RTL/LTR support.
 * Each entry is a regex that matches the physical property as a standalone
 * declaration (not as a substring of another property name).
 */
const PHYSICAL_PROP_PATTERNS: { label: string; re: RegExp }[] = [
  { label: 'margin-left', re: /\bmargin-left\s*:/u },
  { label: 'margin-right', re: /\bmargin-right\s*:/u },
  { label: 'padding-left', re: /\bpadding-left\s*:/u },
  { label: 'padding-right', re: /\bpadding-right\s*:/u },
  { label: 'left', re: /(?<![a-z-])left\s*:/u },
  { label: 'right', re: /(?<![a-z-])right\s*:/u },
  { label: 'border-left', re: /\bborder-left\s*:/u },
  { label: 'border-right', re: /\bborder-right\s*:/u },
  { label: 'text-align: left', re: /text-align\s*:\s*left/u },
  { label: 'text-align: right', re: /text-align\s*:\s*right/u },
];

/**
 * Returns CSS property labels that should have been logical properties.
 */
export function requiresLogicalProperties(css: string): string[] {
  return PHYSICAL_PROP_PATTERNS.filter(({ re }) => re.test(css)).map(({ label }) => label);
}
