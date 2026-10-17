/**
 * Pure comparison for the apex-transition sibling browser step, kept apart from
 * the step file so a unit test can load it without cucumber.
 */

export interface HttpError {
  url: string;
  status: number;
}

/** The URL's path, or the raw string when it does not parse as an absolute URL. */
function pathOf(url: string): string {
  try {
    return new URL(url).pathname;
  } catch {
    return url;
  }
}

/**
 * The corpus-gap class: a 404 on `/epr-head/<slug>`, which every household
 * doorway returns alike for the landing's unseeded epic links
 * (genesis/data/timeline/backlog/household-landing-links-unseeded-epics.md).
 * The only error classes forgiven for either browser: the `/epr-head/`
 * 404s above, and a 404 on exactly `/assets/config.json` (see below).
 */
export function isCorpusGap(e: HttpError): boolean {
  if (e.status !== 404) return false;
  const path = pathOf(e.url);
  // `/assets/config.json` is supplied on alpha by a ConfigMap mount; the household stages the
  // dist as built, so every household doorway 404s it alike. ConfigService treats that 404 as
  // designed (catchError -> DEFAULT_PROD_CONFIG), so it is a household fixture gap, not a
  // failover fact, so both browsers are forgiven it whether or not a given capture recorded it.
  return path.startsWith('/epr-head/') || path === '/assets/config.json';
}

/**
 * The primary's own errors outside the corpus-gap class, as paths. The primary
 * reading is the baseline the sibling is held to, so it must itself be clean
 * but for corpus gaps — anything else is a broken baseline, not a failover fact.
 */
export function primaryErrorsOutsideCorpusGap(primary: HttpError[]): HttpError[] {
  return primary.filter(e => !isCorpusGap(e)).map(e => ({ status: e.status, url: pathOf(e.url) }));
}

/**
 * HTTP errors the sibling's browser saw outside the corpus-gap class, as paths. One rule
 * (isCorpusGap) decides for both browsers. The sibling is forgiven the declared class whether or
 * not the primary's capture recorded it: the primary may legitimately lack a corpus-gap 404 (the
 * browser can serve /assets/config.json from cache on a repeat visit), and that absence is not a
 * failover fact. Every other 404 and every 5xx still fails, even one the primary shared.
 */
export function siblingErrorsOutsideCorpusGap(sibling: HttpError[]): HttpError[] {
  return sibling.filter(e => !isCorpusGap(e)).map(e => ({ status: e.status, url: pathOf(e.url) }));
}
