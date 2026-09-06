import type { CellId } from '@holochain/client';

/** role = provisioned cell; role.name or role.N = enabled clone name/id. */
export function selectSeedCell(cellInfo: Record<string, unknown[]>, target?: string): CellId {
  if (target !== undefined && !/^[^.\s]+(?:\.[^.\s]+)?$/.test(target)) {
    throw new Error(`Invalid cell target '${target}': expected role or role.clone`);
  }
  const [role, clone] = target?.split('.') ?? [];
  const matches: CellId[] = [];
  for (const [name, cells] of Object.entries(cellInfo)) {
    if (role !== undefined && name !== role) continue;
    for (const raw of target === undefined ? cells.slice(0, 1) : cells) {
      const cell = raw as Record<string, unknown>;
      const kind = clone === undefined ? 'provisioned' : 'cloned';
      const value = (cell.type === kind ? cell.value : cell[kind]) as
        | Record<string, unknown>
        | undefined;
      if (!value) continue;
      if (
        clone !== undefined &&
        (value.enabled !== true || (value.name !== clone && value.clone_id !== target))
      )
        continue;
      if (!Array.isArray(value.cell_id) || value.cell_id.length !== 2) {
        throw new Error(`Malformed cell id for '${target ?? name}'`);
      }
      matches.push(value.cell_id as CellId);
      if (target === undefined) return matches[0];
    }
  }
  if (matches.length !== 1) {
    throw new Error(
      `Cell target '${target ?? '(provisioned)'}': expected one enabled cell, found ${matches.length}; no fallback`,
    );
  }
  return matches[0];
}

export function seedCellTarget(
  args: string[],
  env = process.env.SEED_CELL_TARGET,
): string | undefined {
  const index = args.indexOf('--cell');
  if (index < 0) return env;
  const target = args[index + 1];
  if (!target || target.startsWith('--')) throw new Error('--cell requires role or role.clone');
  return target;
}
