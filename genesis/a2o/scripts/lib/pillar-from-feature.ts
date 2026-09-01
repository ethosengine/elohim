const DIRECTORY_TO_PILLAR = new Map<string, string>([
  ['auth', 'imagodei'],
  ['lms', 'lamad'],
  ['rms', 'shefa'],
  ['wms', 'avodah'],
]);
const FEATURE_RE = /features\/([^/]+)\/[^/]+\.feature$/;

export function pillarFromFeature(uri: string): string {
  const match = FEATURE_RE.exec(uri);
  if (!match) return 'unknown';
  const raw = match[1];
  return DIRECTORY_TO_PILLAR.get(raw) ?? raw;
}
