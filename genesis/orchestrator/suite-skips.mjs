// A node --test reporter for the orchestrator suite's named half (E11): one JSON line per skipped
// or todo test, so suite-run.mjs can turn a skip the list does not allow into a red.
export default async function* skips(source) {
  for await (const event of source) {
    if (event.type !== 'test:pass' && event.type !== 'test:fail') continue;
    const { skip, todo, file, name } = event.data;
    if ((skip === undefined || skip === false) && (todo === undefined || todo === false)) continue;
    const reason = typeof skip === 'string' ? skip : typeof todo === 'string' ? todo : skip ? 'skip' : 'todo';
    yield `${JSON.stringify({ file: file || '', name, reason })}\n`;
  }
}
