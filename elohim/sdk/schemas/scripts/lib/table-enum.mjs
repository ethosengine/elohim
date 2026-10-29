// Shared reader for `_table` vocabulary enums (elohim/sdk/schemas/v1/enums/*.schema.json
// carrying a `_table` marker): one projection of the per-value annotations, consumed by
// codegen-ts.mjs (schema-enums.ts tables) and codegen-rs.mjs (lamad-types generated_vocab.rs).
// Kept side-effect free: codegen-ts.mjs creates directories at import time.

/**
 * Per-value annotation rows of a `_table` enum, in `enum` order. Every `enum` value
 * must have exactly one `oneOf` entry (`{ const, x-... }`) and vice versa, and every
 * annotation the `_table` maps must be present on every entry — a value with a
 * missing annotation fails generation loud rather than emitting `undefined`.
 */
export function tableRows(schema) {
  const name = schema.title;
  const annotations = Object.keys(schema._table.annotations || {});
  const entries = new Map((schema.oneOf || []).map((e) => [e.const, e]));
  const values = schema.enum;
  if (entries.size !== values.length || values.some((v) => !entries.has(v))) {
    throw new Error(`${name}: _table oneOf consts must match enum values exactly`);
  }
  return values.map((value) => {
    const entry = entries.get(value);
    const row = { value };
    for (const key of annotations) {
      if (!(key in entry)) throw new Error(`${name}: value '${value}' lacks annotation ${key}`);
      row[key] = entry[key];
    }
    return row;
  });
}

