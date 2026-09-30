/**
 * Steward scripts — the repo side: resolve an id to its authored JSON, and expand a
 * path into the closure of items it names.
 *
 * Shared by scripts/steward-publish.ts (publish commons content from a steward's own
 * peer) and scripts/steward-grade.ts (widen a steward's own rows to their authored
 * reach), so both walk exactly the same closure.
 */
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import type { ConceptJson, PathJson } from '../../../seeder/src/content-input.js';

export interface RepoItem {
  id: string;
  kind: 'content' | 'path';
  file: string;
  json: ConceptJson & PathJson & { visibility?: string };
}

export type Loaded = { item: RepoItem; error?: never } | { item?: never; error: string };

/** Resolve `<dataDir>/content/<id>.json` or `<dataDir>/paths/<id>.json` (exactly one). */
export function loadRepoItem(dataDir: string, id: string): Loaded {
  const contentFile = join(dataDir, 'content', `${id}.json`);
  const pathFile = join(dataDir, 'paths', `${id}.json`);
  const hits = [contentFile, pathFile].filter(f => existsSync(f));
  if (hits.length === 0) {
    return { error: `${id}: no content/${id}.json or paths/${id}.json under ${dataDir}` };
  }
  if (hits.length === 2)
    return { error: `${id}: ambiguous — both content/ and paths/ hold ${id}.json` };
  const file = hits[0];
  const json = JSON.parse(readFileSync(file, 'utf8')) as RepoItem['json'];
  if (json.id !== id) return { error: `${id}: ${file} declares id "${json.id}"` };
  return { item: { id, kind: file === pathFile ? 'path' : 'content', file, json } };
}

/** Targets of the typed edges a content item authors (top-level or legacy metadata list). */
export function edgeTargets(json: Record<string, unknown>): string[] {
  const lists = [
    json.relationships,
    (json.metadata as Record<string, unknown> | undefined)?.relationships,
  ];
  const out = new Set<string>();
  for (const list of lists) {
    if (!Array.isArray(list)) continue;
    for (const edge of list as Record<string, unknown>[]) {
      const target = edge?.target ?? edge?.targetId ?? edge?.target_id;
      if (typeof target === 'string' && target && target !== json.id) out.add(target);
    }
  }
  return [...out];
}

/**
 * Every item a path's walk names (chapter steps, module steps, section concept ids),
 * so a closure carries a path together with everything it points at.
 */
export function pathReferences(json: Record<string, unknown>): string[] {
  const out = new Set<string>();
  const visit = (node: unknown): void => {
    if (Array.isArray(node)) {
      node.forEach(visit);
      return;
    }
    if (!node || typeof node !== 'object') return;
    const obj = node as Record<string, unknown>;
    if (typeof obj.resourceId === 'string' && obj.resourceId) out.add(obj.resourceId);
    if (Array.isArray(obj.conceptIds)) {
      for (const c of obj.conceptIds) if (typeof c === 'string' && c) out.add(c);
    }
    for (const key of ['chapters', 'modules', 'sections', 'steps']) visit(obj[key]);
  };
  visit(json.chapters);
  return [...out];
}

/**
 * The closure of the named ids: each named path's walk, then one hop along every typed
 * edge the resulting content items author (a lesson's scripture, story, practice…), so
 * nothing in the set points at an item outside it. Order: the given ids first, then
 * additions in discovery order; no duplicates.
 */
export function expandClosure(dataDir: string, given: string[]): string[] {
  const ids = [...new Set(given)];
  for (const id of [...ids]) {
    const { item } = loadRepoItem(dataDir, id);
    if (item?.kind !== 'path') continue;
    for (const ref of pathReferences(item.json as unknown as Record<string, unknown>)) {
      if (!ids.includes(ref)) ids.push(ref);
    }
  }
  for (const id of [...ids]) {
    const { item } = loadRepoItem(dataDir, id);
    if (item?.kind !== 'content') continue;
    for (const target of edgeTargets(item.json as unknown as Record<string, unknown>)) {
      if (!ids.includes(target)) ids.push(target);
    }
  }
  return ids;
}

/** Publish the leaves before their course composition; reject cyclic path composition. */
export function publicationOrder(items: RepoItem[]): RepoItem[] {
  const byId = new Map(items.map(item => [item.id, item]));
  const emitted = new Set<string>();
  const visiting = new Set<string>();
  const ordered: RepoItem[] = [];
  const visit = (item: RepoItem): void => {
    if (emitted.has(item.id)) return;
    if (visiting.has(item.id)) throw new Error(`cyclic course composition at ${item.id}`);
    visiting.add(item.id);
    if (item.kind === 'path') {
      for (const id of pathReferences(item.json as unknown as Record<string, unknown>)) {
        const dependency = byId.get(id);
        if (dependency) visit(dependency);
      }
    }
    visiting.delete(item.id);
    emitted.add(item.id);
    ordered.push(item);
  };
  items.filter(item => item.kind === 'content').forEach(visit);
  items.filter(item => item.kind === 'path').forEach(visit);
  return ordered;
}
