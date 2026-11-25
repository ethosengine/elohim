// Build-context COPY sources of a Dockerfile, read the way scripts/ci/closure-input-digest.sh reads
// them (its `closure_sources` heredoc): the same continuation join, COPY match, `--from` exclusion,
// valued-flag strip and JSON-or-space split. That script is an embedded node heredoc and cannot
// import this module, so storage-build-inputs.test.mjs asserts each regex below appears verbatim in
// it; change one only together with the other.
//
// A bare flag (`--link`, `--parents`) is not stripped there: it stays a source token and the digest
// then refuses it ("cannot resolve '--link' in HEAD"). This module keeps it as a token the same way.

export const CONTINUATION = /\\\r?\n/g;
export const COPY_LINE = /^\s*COPY\s+(.+)$/i;
export const FROM_FLAG = /--from(?:=|\s)/;
export const VALUED_FLAG = /--[\w-]+=\S+\s*/g;
// Stage boundaries (the digest has no notion of stages; Docker's FROM is case-insensitive and may
// be indented or carry --platform=…).
const FROM_LINE = /^\s*FROM\s+(?:--[\w-]+=\S+\s+)*(\S+)(?:\s+AS\s+(\S+))?/i;

/**
 * Stages in order (the first, named null, holds anything before the first FROM), each
 * `{ name, parent, copies }`; a copy is `{ sources, dest }` for a build-context
 * COPY, or `{ fromStage: true }` for a `--from` copy (never a context source).
 */
export function dockerfileStages(text) {
  // A COPY before any FROM is invalid Docker, but the digest would still count it: keep it.
  const stages = [];
  let stage = { name: null, parent: null, copies: [] };
  stages.push(stage);
  for (const line of text.replace(CONTINUATION, ' ').split('\n')) {
    const from = line.match(FROM_LINE);
    if (from) {
      stage = { name: from[2] ?? `stage${stages.length - 1}`, parent: from[1], copies: [] };
      stages.push(stage);
      continue;
    }
    const copy = line.match(COPY_LINE);
    if (!copy) continue;
    if (FROM_FLAG.test(copy[1])) {
      stage.copies.push({ fromStage: true });
      continue;
    }
    const args = copy[1].replace(VALUED_FLAG, '').trim();
    const parts = args.startsWith('[') ? JSON.parse(args) : args.split(/\s+/);
    if (parts.length < 2) throw new Error(`unparsed COPY: ${line.trim()}`);
    stage.copies.push({ sources: parts.slice(0, -1), dest: parts.at(-1) });
  }
  return stages;
}

/** Every build-context COPY source, in file order (the digest's set, before path resolution). */
export function contextCopySources(text) {
  return dockerfileStages(text).flatMap(s => s.copies.flatMap(c => c.sources ?? []));
}
