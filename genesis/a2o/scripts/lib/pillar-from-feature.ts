const AUTH_IS_IMAGODEI = new Map<string, string>([['auth', 'imagodei']]);

export function pillarFromFeature(uri: string): string {
  const match = uri.match(/features\/([^/]+)\/[^/]+\.feature$/);
  if (!match) return 'unknown';
  const raw = match[1];
  return AUTH_IS_IMAGODEI.get(raw) ?? raw;
}
