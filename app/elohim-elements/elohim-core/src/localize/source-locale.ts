import { msg, str } from '@lit/localize';

/**
 * Smoke-test source string — proves the localize pipeline is wired and provides
 * one entry for the xliff seed.
 */
export const smokeGreeting = (name: string) => msg(str`Hello, ${name}`);
