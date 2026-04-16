import { readFileSync, existsSync } from 'node:fs';
import picomatch from 'picomatch';

const BASH_PATTERN = /^Bash\((.*)\)$/;

function extractBashPattern(entry) {
  const m = entry.match(BASH_PATTERN);
  return m ? m[1] : null;
}

function toGlob(palettePattern) {
  if (!palettePattern || typeof palettePattern !== 'string') return null;
  const trimmed = palettePattern.trim();
  if (!trimmed) return null;
  return trimmed.replaceAll(':*', '*');
}

export function matchesPalette(command, paletteEntries) {
  // Reject anything that isn't a clean single-line string — newlines or
  // control characters would let a matched prefix smuggle a second command
  // through a shell that interprets line breaks as separators.
  if (!command || typeof command !== 'string') return false;
  if (/[\n\r\x00]/.test(command)) return false;

  const trimmed = command.trim();
  for (const entry of paletteEntries) {
    if (typeof entry !== 'string') continue;
    const bashBody = extractBashPattern(entry);
    const candidate = bashBody ?? entry;
    const glob = toGlob(candidate);
    if (!glob) continue;
    try {
      if (picomatch.isMatch(trimmed, glob, { dot: true })) return true;
    } catch {
      // Malformed glob pattern — skip this entry, never fail-open.
      continue;
    }
  }
  return false;
}

export function loadPalette({ durablePath, localPath }) {
  const out = [];
  for (const path of [durablePath, localPath]) {
    if (!existsSync(path)) continue;
    let contents;
    try {
      contents = JSON.parse(readFileSync(path, 'utf8'));
    } catch {
      continue; // malformed JSON → skip file, never fail-open
    }
    const allow = contents?.permissions?.allow;
    if (!Array.isArray(allow)) continue;
    for (const entry of allow) if (typeof entry === 'string') out.push(entry);
  }
  return out;
}
