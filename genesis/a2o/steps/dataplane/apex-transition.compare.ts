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
 * The only error class the sibling is forgiven for sharing with the primary.
 */
export function isCorpusGap(e: HttpError): boolean {
  return e.status === 404 && pathOf(e.url).startsWith('/epr-head/');
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
 * HTTP errors the sibling's browser saw that are not a corpus gap the primary's
 * browser also had, compared by URL path + status so the two doorways' differing
 * origins do not count as a difference. Any other sibling error fails, even one
 * the primary shared. Returned URLs are paths.
 */
export function siblingWorseThanPrimary(primary: HttpError[], sibling: HttpError[]): HttpError[] {
  const key = (e: HttpError): string => `${e.status} ${pathOf(e.url)}`;
  const forgiven = new Set(primary.filter(isCorpusGap).map(key));
  return sibling
    .filter(e => !(isCorpusGap(e) && forgiven.has(key(e))))
    .map(e => ({ status: e.status, url: pathOf(e.url) }));
}
