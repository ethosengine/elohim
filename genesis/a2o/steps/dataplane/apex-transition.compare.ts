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
 * HTTP errors the sibling's browser saw that the primary's browser did not,
 * compared by URL path + status so the two doorways' differing origins do not
 * count as a difference. Returned URLs are paths.
 */
export function siblingWorseThanPrimary(primary: HttpError[], sibling: HttpError[]): HttpError[] {
  const seen = new Set(primary.map(e => `${e.status} ${pathOf(e.url)}`));
  return sibling
    .map(e => ({ status: e.status, url: pathOf(e.url) }))
    .filter(e => !seen.has(`${e.status} ${e.url}`));
}
