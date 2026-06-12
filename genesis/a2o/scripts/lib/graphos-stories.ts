/**
 * Pure functions over a Storybook v5 index — story-id conventions, component
 * matching, grouping, and composite-sheet HTML generation for `graphos`.
 *
 * Story ids follow `<family>-<group>-<component>--<cell>`; family is the
 * first `title` segment lowercased (default | designed | i..iv | foundations).
 * Spec: genesis/docs/superpowers/specs/2026-06-11-graphos-look-design.md
 */

export interface StoryEntry {
  id: string;
  title: string;
  name: string;
  type: 'story' | 'docs';
}

export interface StorybookIndex {
  v: number;
  entries: Record<string, StoryEntry>;
}

/** Id prefix before `--`, e.g. `designed-core-elohim-compute-tile`. */
export function componentPrefix(id: string): string {
  const i = id.indexOf('--');
  return i === -1 ? id : id.slice(0, i);
}

/**
 * Segment-aligned: prefix === component or (multi-segment) prefix ends with `-<component>`.
 * Multi-segment names may match as any segment-aligned suffix (e.g. `core-elohim-compute-tile`
 * also matches `designed-core-elohim-compute-tile`); single-segment names require exact prefix equality.
 */
export function matchesComponent(id: string, component: string): boolean {
  const prefix = componentPrefix(id);
  if (prefix === component) return true;

  // Multi-segment components (containing dashes) can match as a suffix
  if (component.includes('-')) {
    return prefix.endsWith(`-${component}`);
  }

  // Single-segment components must match exactly (already checked above)
  return false;
}

export function familyOf(entry: StoryEntry): string {
  return (entry.title.split('/')[0] ?? '').toLowerCase();
}

export function listStories(index: StorybookIndex, filter?: string): StoryEntry[] {
  const all = Object.values(index.entries);
  if (!filter) return all;
  const f = filter.toLowerCase();
  return all.filter(e => e.id.includes(f) || e.title.toLowerCase().includes(f));
}

export function groupByComponent(entries: StoryEntry[]): Map<string, StoryEntry[]> {
  const groups = new Map<string, StoryEntry[]>();
  for (const e of entries) {
    const key = componentPrefix(e.id);
    const list = groups.get(key);
    if (list) list.push(e);
    else groups.set(key, [e]);
  }
  return groups;
}

export function suggestComponents(index: StorybookIndex, name: string, limit = 5): string[] {
  const n = name.toLowerCase();
  const all = Object.values(index.entries).map(e => componentPrefix(e.id));
  return [...new Set(all.filter(p => p.toLowerCase().includes(n)))].slice(0, limit);
}

export function storiesForSheet(
  index: StorybookIndex,
  component: string,
  family?: string
): StoryEntry[] {
  return Object.values(index.entries).filter(
    e =>
      e.type === 'story' && matchesComponent(e.id, component) && (!family || familyOf(e) === family)
  );
}

/** A named viewport archetype band for the sheet's viewport dimension. */
export interface SheetViewport {
  name: string;
  width: number;
  height: number;
}

export interface SheetOptions {
  component: string;
  base: string;
  entries: StoryEntry[];
  cell: { width: number; height: number };
  cols: number;
  /**
   * Optional viewport dimension: when present, each family section renders
   * one BAND per archetype, with iframes at the archetype's TRUE pixel size
   * (so vw-relative CSS and breakpoints resolve as on a real device), scaled
   * down via CSS transform to fit the cell width.
   */
  viewports?: SheetViewport[];
}

/** Scale factor that fits an archetype's true width into the cell width. */
export function viewportScale(cellWidth: number, vp: SheetViewport): number {
  return Math.min(1, cellWidth / vp.width);
}

/**
 * Self-contained iframe-grid page; rendered via file:// in one full-page
 * screenshot. A file:// parent loading http(s) iframes is permitted in
 * Chromium (mixed-content rules block the reverse case).
 * Precondition: caller validates `entries` is non-empty (the CLI errors before calling).
 */
export function sheetHtml(opts: SheetOptions): string {
  const { component, base, entries, cell, cols, viewports } = opts;
  const byFamily = new Map<string, StoryEntry[]>();
  for (const e of entries) {
    const fam = familyOf(e);
    const list = byFamily.get(fam);
    if (list) list.push(e);
    else byFamily.set(fam, [e]);
  }
  const storyIframe = (e: StoryEntry, width: number, height: number, extra = ''): string =>
    `<iframe src="${escapeHtml(base)}/iframe.html?id=${encodeURIComponent(e.id)}&amp;viewMode=story" width="${width}" height="${height}" loading="eager"${extra}></iframe>`;
  const sections = [...byFamily.entries()]
    .map(([family, stories]) => {
      if (viewports && viewports.length > 0) {
        // Viewport dimension: one band per archetype; iframes at TRUE
        // archetype size (vw units + breakpoints resolve as on-device),
        // transform-scaled to fit the cell width.
        const bands = viewports
          .map(vp => {
            const scale = viewportScale(cell.width, vp);
            const cellW = Math.round(vp.width * scale);
            const cellH = Math.round(vp.height * scale);
            const cells = stories
              .map(
                e => `<figure class="cell">
  <figcaption>${escapeHtml(e.name)}</figcaption>
  <div class="vp-frame" style="width: ${cellW}px; height: ${cellH}px;">
    ${storyIframe(e, vp.width, vp.height, ` style="transform: scale(${scale}); transform-origin: top left;"`)}
  </div>
</figure>`
              )
              .join('\n');
            return `<div class="band">\n<h3>${escapeHtml(vp.name)} (${vp.width}×${vp.height})</h3>\n<div class="grid" style="grid-template-columns: repeat(${cols}, ${cellW}px);">\n${cells}\n</div>\n</div>`;
          })
          .join('\n');
        return `<section>\n<h2>${escapeHtml(family)}</h2>\n${bands}\n</section>`;
      }
      const cells = stories
        .map(
          e => `<figure class="cell">
  <figcaption>${escapeHtml(e.name)}</figcaption>
  ${storyIframe(e, cell.width, cell.height)}
</figure>`
        )
        .join('\n');
      return `<section>\n<h2>${escapeHtml(family)}</h2>\n<div class="grid">\n${cells}\n</div>\n</section>`;
    })
    .join('\n');
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>graphos sheet: ${escapeHtml(component)}</title>
<style>
  body { margin: 0; padding: 8px; font-family: system-ui, sans-serif; background: #fff; }
  h1 { font-size: 16px; margin: 4px 0; }
  h2 { margin: 12px 0 4px; font-size: 14px; }
  h3 { margin: 8px 0 2px; font-size: 12px; color: #333; }
  .grid { display: grid; grid-template-columns: repeat(${cols}, ${cell.width}px); gap: 8px; }
  .cell { margin: 0; }
  .cell figcaption { font-size: 11px; color: #555; padding: 2px 0; }
  .cell iframe { border: 1px solid #ddd; display: block; }
  .vp-frame { overflow: hidden; border: 1px solid #ddd; }
  .vp-frame iframe { border: 0; }
</style>
</head>
<body>
<h1>${escapeHtml(component)}</h1>
${sections}
</body>
</html>`;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
