import { readFileSync, existsSync } from 'node:fs';
import picomatch from 'picomatch';

const BASH_PATTERN = /^Bash\((.*)\)$/;

function extractBashPattern(entry) {
  const m = entry.match(BASH_PATTERN);
  return m ? m[1] : null;
}

function toGlob(palettePattern) {
  // Claude-style patterns use `*` as a wildcard, `:*` as "any trailing args".
  // Convert to picomatch glob: both become `*` spanning any chars.
  return palettePattern.replaceAll(':*', '*');
}

export function matchesPalette(command, paletteEntries) {
  const trimmed = command.trim();
  for (const entry of paletteEntries) {
    // Accept both Bash(...) and bare MCP tool names (mcp__foo__*).
    const bashBody = extractBashPattern(entry);
    const candidate = bashBody ?? entry;
    const glob = toGlob(candidate);
    if (picomatch.isMatch(trimmed, glob, { dot: true })) return true;
  }
  return false;
}

export function loadPalette({ durablePath, localPath }) {
  const out = [];
  for (const path of [durablePath, localPath]) {
    if (!existsSync(path)) continue;
    const contents = JSON.parse(readFileSync(path, 'utf8'));
    const allow = contents?.permissions?.allow ?? [];
    for (const entry of allow) if (typeof entry === 'string') out.push(entry);
  }
  return out;
}
