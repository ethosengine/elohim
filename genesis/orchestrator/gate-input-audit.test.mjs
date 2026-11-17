import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'fs';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import { auditProject, declarationScopes, declarationsIn, exhaustInputs, globCovers, isReadPattern, readCovers, readSet } from './gate-input-audit.mjs';
import { GATE_EXHAUST } from './gate-evidence.mjs';
import { NAMED_READ_SET_PROJECTS } from './gate-preflight.mjs';
import { loadGateRegistry } from './pipeline-registry.mjs';

const root = '/fixture';
const file = 'crates/example/tests/reads.rs';
const project = { dir: 'crates/example' };

function fixture(text, sources = ['crates/example/**'], extra = {}) {
  const files = { 'crates/example/Cargo.toml': '[package]', [file]: text, ...extra };
  return {
    root, project, sources,
    listFiles: () => Object.keys(files),
    readFile: path => {
      const rel = path.slice(root.length + 1);
      if (Object.hasOwn(files, rel)) return files[rel];
      throw Object.assign(new Error(`missing ${path}`), { code: 'ENOENT' });
    },
  };
}

function audit(...args) {
  return auditProject(fixture(...args)).undeclared;
}

test('manifest concat includes normalize parent segments from the nearest crate', () => {
  assert.deepEqual(audit('include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/data.json"));'), [
    { file, line: 1, path: 'shared/data.json', how: 'include_str' },
  ]);
});

test('concat with a variable resolves the directory of its literal prefix', () => {
  for (const expression of [
    'concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/", $path)',
    'concat!(env!("CARGO_MANIFEST_DIR"), "/../", "../shared/schema-", $path, ".json")',
    'concat!("../../../shared/", $path)',
  ]) {
    assert.deepEqual(readSet(fixture(`include_str!(${expression});`)), { paths: ['shared'], unresolved: [] });
    assert.deepEqual(audit(`include_str!(${expression});`), [{ file, line: 1, path: 'shared', how: 'include_str' }]);
  }
});

test('readSet includes declared and undeclared files and directory reads, sorted and deduplicated', () => {
  assert.deepEqual(readSet(fixture(`include_str!("../../../shared/undeclared.json");
include_bytes!("../../../shared/declared.json");
include_str!("../../../shared/undeclared.json");
include_str!("../fixtures/local.json");
std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/dir"));
Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shared/constructed-dir");`,
  ['crates/example/**', 'shared/declared.json'])), {
    paths: ['crates/example/fixtures/local.json', 'shared/constructed-dir', 'shared/declared.json', 'shared/dir', 'shared/undeclared.json'],
    unresolved: [],
  });
});

test('readSet preserves unresolved source locations and expressions', () => {
  assert.deepEqual(readSet(fixture('\ninclude_bytes!(unknown!());')), {
    paths: [], unresolved: [{ file, line: 2, expr: 'include_bytes!(unknown!())' }],
  });
});

test('concat directory prefixes resolve manifest paths but transformations and repository roots stay unresolved', () => {
  const prefix = 'Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/", $path))';
  assert.deepEqual(readSet(fixture(`std::fs::read_dir(${prefix});`)), { paths: ['shared'], unresolved: [] });
  for (const expression of [`${prefix}.parent()`, `${prefix}.join("../other")`,
    'include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $path))']) {
    const result = readSet(fixture(`${expression};`));
    assert.deepEqual(result.paths, []);
    assert.equal(result.unresolved.length, 1);
  }
});

test('a declared input passes', () => {
  assert.deepEqual(audit('include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/data.json"));',
    ['crates/example/**', 'shared/data.json']), []);
});

test('an undeclared include reports its source file and line', () => {
  assert.deepEqual(audit('// comment\n\nconst DATA: &[u8] = include_bytes!("../../../shared/data.bin");'), [
    { file, line: 3, path: 'shared/data.bin', how: 'include_bytes' },
  ]);
});

test('a terminal ** covers nested paths and the directory, with a component boundary', () => {
  assert.equal(globCovers('a/b/**', 'a/b/c/d.json'), true);
  assert.equal(globCovers('a/b/**', 'a/b'), true);
  assert.equal(globCovers('a/b/**', 'a/b-sibling/c.json'), false);
});

test('an exact glob does not cover a sibling or a descendant', () => {
  assert.equal(globCovers('a/b.json', 'a/c.json'), false);
  assert.equal(globCovers('a/b.json', 'a/b.json/child'), false);
  assert.equal(globCovers('a/b.json', 'a/b.json'), true);
  assert.equal(globCovers('**/build-manifest.json', 'a/build-manifest.json'), false);
});

test('a join chain with a trailing variable resolves to its literal directory', () => {
  for (const constructor of ['Path::new', 'std::path::PathBuf::from']) {
    const text = `${constructor}(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name);`;
    assert.deepEqual(readSet(fixture(text)), { paths: ['crates/example/fixtures'], unresolved: [] });
  }
  assert.deepEqual(audit('Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shared").join(name);',
    ['crates/example/**', 'shared/**']), []);
});

test('parent and normalized literal joins precede a variable filename', () => {
  assert.deepEqual(readSet(fixture(`PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent().unwrap().join("../shared/unused/..").join(name);`)), {
    paths: ['shared'], unresolved: [],
  });
});

test('transformations after a variable join remain unresolved', () => {
  for (const suffix of ['.parent().unwrap()', '.join("../elsewhere")', '.replace_extension(ext)']) {
    const result = readSet(fixture(`PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)${suffix};`));
    assert.deepEqual(result.paths, []);
    assert.equal(result.unresolved.length, 1);
  }
});

test('a one-level local helper resolves its callers without keeping its root constructor', () => {
  assert.deepEqual(readSet(fixture(`fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
  }
  fn read() {
    std::fs::read(repo().join("shared/data.json"));
    repo().join("shared/fixtures").join(name);
  }`)), { paths: ['shared/data.json', 'shared/fixtures'], unresolved: [] });
});

test('helpers with statements, arguments or ambiguous names are not inferred', () => {
  for (const definition of [
    'fn repo() -> PathBuf { touch(); PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") }',
    'fn repo(name: &str) -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(name) }',
    `mod a { fn repo() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") } }
     mod b { fn repo() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") } }`,
  ]) {
    const result = readSet(fixture(`${definition}\nrepo().join("shared/data.json");`));
    assert.ok(result.unresolved.length > 0);
    assert.ok(!result.paths.includes('shared/data.json'));
  }
});

test('helper expansion stops at one level', () => {
  const result = readSet(fixture(`fn base() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../shared")
  }
  fn indirect() -> PathBuf { base().join("fixtures") }
  std::fs::read(indirect().join("data.json"));`));
  assert.deepEqual(result, { paths: ['shared/fixtures'], unresolved: [] });
});

test('methods, qualified names and shadowed callables do not borrow a local helper body', () => {
  for (const invocation of ['other.repo()', 'other::repo()', 'repo()']) {
    const result = readSet(fixture(`fn repo() -> PathBuf {
      PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
    fn use_other() {
      let repo = temporary_root;
      ${invocation}.join("shared/data.json");
    }`));
    assert.deepEqual(result.paths, []);
    assert.equal(result.unresolved.length, 1);
  }
});

test('a gate-reads comment within 15 lines resolves an otherwise unresolved walk', () => {
  for (const distance of [1, 15, 16]) {
    const result = readSet(fixture(`// gate-reads: shared/fixtures\n${'\n'.repeat(distance - 1)}Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_root);`));
    assert.deepEqual(result.paths, distance <= 15 ? ['shared/fixtures'] : []);
    assert.equal(result.unresolved.length, distance <= 15 ? 0 : 1);
  }
});

test('stacked gate-reads comments resolve the walk of the statement they sit above', () => {
  const result = readSet(fixture(`fn walk() {
    // gate-reads: shared/a.json
    // gate-reads: shared/b.json
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_root);
  }`));
  assert.deepEqual(result, { paths: ['shared/a.json', 'shared/b.json'], unresolved: [] });
});

test('a declaration away from its statement resolves nothing, before or after it', () => {
  for (const body of [
    `// gate-reads: shared/a.json\n    let unrelated = 1;\n    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_root);`,
    `Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_root);\n    // gate-reads: shared/a.json`,
  ]) {
    const result = readSet(fixture(`fn walk() {\n    ${body}\n  }`));
    assert.deepEqual(result.paths, []);
    assert.equal(result.unresolved.length, 1);
  }
});

test('a fixed-read declaration cannot mask a data-derived walk in the same function (row 76)', () => {
  const result = readSet(fixture(`fn fixture() {
    // gate-reads: shared/fixed.json
    let fixed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let copy = fs::read(fixed.join("shared/fixed.json"));
    for entry in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() { visit(entry); }
  }`));
  assert.deepEqual(result.paths, ['shared/fixed.json']);
  assert.equal(result.unresolved.length, 1);
  assert.equal(result.unresolved[0].line, 5);
});

test('a declaration covers the reads derived from its construction through bindings', () => {
  const result = readSet(fixture(`fn copy() {
    // gate-reads: shared/data.json
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let name = pick();
    let bytes = fs::read(repo.join(name));
  }`));
  assert.deepEqual(result, { paths: ['shared/data.json'], unresolved: [] });
});

test('a **/<basename> declaration names one file kind across the repository, and no other wildcard is valid', () => {
  const walk = decl => readSet(fixture(`fn walk() {
    // gate-reads: ${decl}
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_root);
  }`));
  assert.deepEqual(walk('**/.epr-meta'), { paths: ['**/.epr-meta'], unresolved: [] });
  for (const invalid of ['shared/*.json', '**/a/b', '**/*', '*', 'shared/**']) {
    const result = walk(invalid);
    assert.deepEqual(result.paths, [], invalid);
    assert.equal(result.unresolved.length, 1, invalid);
  }
});

// Backlog steward-device-arrival-open-items row 7: the eprfs live-corpus legs keyed the workspace
// gate on the whole documentation tree. A scoped declaration gives a read to one gate project.
test('a gate-reads(<project>) declaration names its read for that project alone', () => {
  const text = `fn repo_root() -> PathBuf {
    // gate-reads(live-corpus): genesis/docs
    // gate-reads(live-corpus): **/CLAUDE.md
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
  }`;
  const as = name => readSet({ ...fixture(text), project: { ...project, name } });
  assert.deepEqual(as('live-corpus'), { paths: ['**/CLAUDE.md', 'genesis/docs'], unresolved: [] });
  assert.deepEqual(as('workspace'), { paths: [], unresolved: [] },
    'another project neither keys on the read nor reports the claimed walk unresolved');
  assert.deepEqual(readSet(fixture(text)), { paths: ['**/CLAUDE.md', 'genesis/docs'], unresolved: [] },
    'a scan that names no project (reverse-dependency narrowing) sees every declared read');
  assert.deepEqual(auditProject({ ...fixture(text), project: { ...project, name: 'workspace' } }).undeclared, []);
});

test('an unscoped declaration beside a scoped one still keys every project, and a scope covers only its statement', () => {
  const mixed = `fn walk() {
    // gate-reads(live-corpus): genesis/docs
    // gate-reads: shared/fixtures
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_root);
  }`;
  const as = (text, name) => readSet({ ...fixture(text), project: { ...project, name } });
  assert.deepEqual(as(mixed, 'workspace'), { paths: ['shared/fixtures'], unresolved: [] });
  assert.deepEqual(as(mixed, 'live-corpus').paths, ['genesis/docs', 'shared/fixtures']);
  const beside = `fn walk() {
    // gate-reads(live-corpus): genesis/docs
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_root);
    let other = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_other);
  }`;
  const result = as(beside, 'workspace');
  assert.deepEqual(result.paths, []);
  assert.equal(result.unresolved.length, 1, 'the next statement is not claimed by the scoped line above its neighbour');
  for (const malformed of ['gate-reads(): genesis/docs', 'gate-reads(a b): genesis/docs', 'gate-reads (x): genesis/docs']) {
    const r = as(`fn walk() {\n    // ${malformed}\n    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().find_map(find_root);\n  }`, 'workspace');
    assert.deepEqual(r.paths, [], malformed);
    assert.equal(r.unresolved.length, 1, `${malformed} is not a declaration`);
  }
  assert.deepEqual(declarationScopes(mixed), [{ project: 'live-corpus', path: 'genesis/docs', line: 2 }]);
});

test('readCovers matches a basename read at the root and any depth only; source globs stay literal', () => {
  assert.ok(readCovers('**/.epr-meta', '.epr-meta'));
  assert.ok(readCovers('**/.epr-meta', 'a/b/.epr-meta'));
  assert.ok(!readCovers('**/.epr-meta', 'a/x.epr-meta'));
  assert.ok(!readCovers('**/.epr-meta', '.epr-meta/elohim/policy.json'));
  assert.ok(!globCovers('**/.epr-meta', 'a/b/.epr-meta'), 'manifest source globs do not gain basename patterns');
  assert.ok(readCovers('.', 'any/file'));
  assert.ok(readCovers('shared', 'shared/x') && !readCovers('shared', 'shared-other/x'));
  assert.ok(isReadPattern('**/.epr-meta') && !isReadPattern('**/a/b') && !isReadPattern('shared/**'));
});

test('gate-reads dot explicitly names the whole repository', () => {
  assert.deepEqual(readSet(fixture(`fn walk() {
    // gate-reads: .
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  }`)), { paths: ['.'], unresolved: [] });
});

test('declarations in strings, block comments and other functions do not resolve a walk', () => {
  for (const decoy of [
    'const TEXT: &str = r#"\n// gate-reads: shared\n"#;',
    '/*\n// gate-reads: shared\n*/',
    'fn unrelated() {\n// gate-reads: shared\n}',
  ]) {
    const result = readSet(fixture(`${decoy}\nfn walk() { Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."); }`));
    assert.deepEqual(result.paths, []);
    assert.equal(result.unresolved.length, 1);
  }
});

test('declarations cannot name paths outside the repository or hide resolved reads', () => {
  for (const path of ['/outside', '../outside']) {
    const result = readSet(fixture(`// gate-reads: ${path}\ninclude_bytes!(unknown!());`));
    assert.deepEqual(result.paths, []);
    assert.equal(result.unresolved.length, 1);
  }
  assert.deepEqual(readSet(fixture(`// gate-reads: shared/decoy
    include_str!("../../../shared/actual.json");`)), {
    paths: ['shared/actual.json'], unresolved: [],
  });
});

test('literal join chains, parents and local string constants resolve', () => {
  const rows = audit(`const NAME: &str = "data.json";
Path::new(env!("CARGO_MANIFEST_DIR"))
  .parent().unwrap().parent().expect("root").join("shared").join(NAME);`);
  assert.deepEqual(rows, [{ file, line: 2, path: 'shared/data.json', how: 'CARGO_MANIFEST_DIR' }]);
});

test('direct fs reads and File::open accept manifest-built paths', () => {
  for (const read of ['std::fs::read', 'std::fs::read_to_string', 'read_dir', 'std::fs::File::open']) {
    const rows = audit(`${read}(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/data"));`);
    assert.deepEqual(rows, [{ file, line: 1, path: 'shared/data', how: 'CARGO_MANIFEST_DIR' }]);
  }
});

test('a local base followed by a literal read declares the concrete file', () => {
  const rows = audit(`fn test() {
let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shared");
let path = base.join("data.json");
std::fs::read_to_string(&path).unwrap();
}`);
  assert.deepEqual(rows, [{ file, line: 4, path: 'shared/data.json', how: 'CARGO_MANIFEST_DIR' }]);
});

test('project tree covers includes even when sources name only the Rust file', () => {
  assert.deepEqual(audit('include_str!("../fixtures/data.json");', [file]), []);
});

test('the nearest Cargo.toml wins inside a nested workspace', () => {
  const nested = 'crates/example/member/src/lib.rs';
  const rows = audit('', ['crates/example/**'], {
    'crates/example/member/Cargo.toml': '[package]',
    [nested]: 'include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../shared/data.json"));',
  });
  assert.deepEqual(rows, [{ file: nested, line: 1, path: 'shared/data.json', how: 'include_str' }]);
});

test('comments and embedded source strings are not scanned as code', () => {
  assert.deepEqual(audit(`// include_str!("../../../missing")
/* nested /* include_bytes!("missing") */ comment */
let example = r###"include_str!("../../../missing")"###;
let path = "Path::new(env!(\\"CARGO_MANIFEST_DIR\\")).join(\\"missing\\")";
include_str!(r#"../fixtures/real.json"#);`), []);
});

test('whole-repository roots and ancestor searches stay unresolved', () => {
  for (const expr of ['.join("../..")', '.ancestors().nth(2)', '.ancestors().find_map(find_root)']) {
    const rows = audit(`Path::new(env!("CARGO_MANIFEST_DIR"))${expr};`);
    assert.equal(rows.length, 1);
    assert.equal(rows[0].how, 'unresolved');
  }
});

test('dynamic include expressions are reported', () => {
  const rows = audit('include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), unknown!()));');
  assert.equal(rows.length, 1);
  assert.equal(rows[0].how, 'unresolved');
});

test('unresolved concatenation and mutable ancestor searches are never dropped', () => {
  for (const expression of [
    'Path::new(concat!(env!("CARGO_MANIFEST_DIR"), unknown!()))',
    'Path::new(env!("CARGO_MANIFEST_DIR"))',
  ]) {
    const rows = audit(`let mut path = ${expression}; path.pop();`);
    assert.equal(rows.length, 1);
    assert.equal(rows[0].how, 'unresolved');
  }
});

test('reading a directory and a descendant still requires declaring the directory', () => {
  const rows = audit(`let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shared");
std::fs::read_dir(&base);
std::fs::read(base.join("data.json"));`, ['crates/example/**', 'shared/data.json']);
  assert.ok(rows.some(row => row.path === 'shared' && row.line === 2));
});

test('an unrelated descendant read cannot hide a directory constructor', () => {
  const rows = audit(`fn corpus() -> PathBuf {
PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../shared")
}
std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/data.json"));`,
  ['crates/example/**', 'shared/data.json']);
  assert.deepEqual(rows, [{ file, line: 2, path: 'shared', how: 'CARGO_MANIFEST_DIR' }]);
});

test('a shadowed local binding does not turn a fixture read into a repo read', () => {
  const rows = audit(`fn test() {
let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("local");
{ let path = temporary.path(); std::fs::read_to_string(path.join("fixture")); }
}`);
  assert.deepEqual(rows, []);
});

test('Rust files outside the declared sources are not scanned', () => {
  assert.deepEqual(audit('', ['crates/example/**'], {
    'another/src/lib.rs': 'include_str!("../../undeclared.json");',
  }), []);
});

function realReadSet(name) {
  const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const project = loadGateRegistry(repo).get(name);
  const manifest = JSON.parse(readFileSync(resolve(repo, project.manifestPath), 'utf8'));
  return readSet({
    root: repo, project,
    sources: [...new Set(project.steps.flatMap(step => manifest.steps[step].inputs.sources))],
  });
}

test('REAL eprfs names its governance walk as the .epr-meta file kind, not the whole repository', () => {
  const result = realReadSet('eprfs');
  assert.deepEqual(result.unresolved, []);
  assert.ok(result.paths.includes('genesis/agentic/agent-runtimes.json'));
  // The walk is a live-corpus leg: eprfs-live-corpus keys on it, the workspace gate skips it.
  const live = realReadSet('eprfs-live-corpus');
  assert.deepEqual(live.unresolved, []);
  assert.ok(live.paths.includes('**/.epr-meta'), 'the governance walk must remain visible as its file kind');
  assert.ok(!result.paths.includes('**/.epr-meta'), 'the workspace gate skips the live walk and must not key on it');
  for (const r of [result, live]) assert.ok(!r.paths.includes('.'), 'no eprfs read may name the whole repository');
  assert.ok(!result.unresolved.some(row => row.file.endsWith('/live_manifest_integrity.rs')));
  assert.ok(!result.unresolved.some(row => row.file.includes('/flow/memory/recall/')));
  for (const name of ['recall-contract', 'recall-semantic-index', 'recall-lexical-index']) {
    assert.ok(result.paths.includes(`.epr-meta/elohim/algorithms/${name}.json`));
  }
});

test('exhaust promise rejects sources, recipe references and audited reads, including future files', () => {
  for (const input of ['.claude/data/future.jsonl', '.claude/data/**', '.claude/**', '.claude/**/*.jsonl', '**', '.']) {
    assert.deepEqual(exhaustInputs({ exhaust: GATE_EXHAUST, sources: [input] }), [input]);
  }
  assert.deepEqual(exhaustInputs({ exhaust: GATE_EXHAUST, reads: ['**/future.jsonl'] }), ['**/future.jsonl']);
  assert.deepEqual(exhaustInputs({ exhaust: GATE_EXHAUST, recipe: 'cat .claude/data/future.jsonl' }), ['.claude/data/future.jsonl']);
  assert.deepEqual(exhaustInputs({ exhaust: GATE_EXHAUST, reads: ['.claude/data/runtime-cursor.json'] }), ['.claude/data/runtime-cursor.json']);
  assert.deepEqual(exhaustInputs({ exhaust: GATE_EXHAUST, sources: ['**/*.rs', '.claude/data/failure-taxonomy.json'],
    recipe: '# explains .claude/data/foo.jsonl\nnode tests.mjs' }), []);
});

test('REAL every gate keeps declared sources, recipes and audited reads disjoint from exhaust', () => {
  const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const files = readdirSync(resolve(repo, '.claude/data')).map(name => `.claude/data/${name}`);
  const failures = [];
  for (const project of loadGateRegistry(repo).values()) {
    const manifest = JSON.parse(readFileSync(resolve(repo, project.manifestPath), 'utf8'));
    const sources = [...new Set(project.steps.flatMap(step => manifest.steps[step].inputs?.sources || []))];
    const reads = readSet({ root: repo, sources }).paths;
    let recipe = '';
    if (project.run.kind !== 'attested') {
      const local = resolve(repo, project.dir, 'justfile');
      recipe = readFileSync(project.run.kind !== 'root-just' && existsSync(local) ? local : resolve(repo, 'justfile'), 'utf8');
    }
    const conflicts = exhaustInputs({ exhaust: GATE_EXHAUST, files, sources, reads, recipe });
    if (conflicts.length) failures.push({ project: project.name, conflicts });
  }
  assert.deepEqual(failures, [], 'remove a real input from GATE_EXHAUST; never silently ignore it');
});

// One list with the preflight that runs this same assertion before the gates (gate-preflight.mjs).
for (const name of NAMED_READ_SET_PROJECTS) {
  test(`REAL ${name} read set has no unresolved reads or repository-wide path`, () => {
    const result = realReadSet(name);
    assert.deepEqual(result.unresolved, []);
    assert.ok(!result.paths.includes('.'));
    assert.ok(result.paths.includes('elohim/sdk/schemas/v1/feedback-signals'));
    // The shared-crate reads below belong to the three runtime services; elohim-epr and eprfs
    // read neither the render fixtures nor the canon-lift sources through those crates.
    if (!['elohim-epr', 'eprfs'].includes(name)) {
      assert.ok(result.paths.includes('.claude/epr-meta/policies.yaml'));
      assert.ok(result.paths.includes('.claude/epr-meta/concerns.yaml'));
      assert.ok(result.paths.includes('elohim/elohim-render/fixtures'));
    }
    if (name === 'elohim-storage') {
      for (const path of ['elohim/sdk/schemas/v1', 'elohim/sdk/domains',
        'genesis/orchestrator/manifests/runtime', 'genesis/orchestrator/data/deployments.json',
        'scripts/ci/conductor-split-budget.sh']) assert.ok(result.paths.includes(path), path);
    }
  });
}

// A habit delta re-projects genesis/manifests/habits.yaml. While eprfs keyed on the whole
// directory, every delta in the repository re-ran the eprfs workspace gate (1323 s on 9be1cb048).
test('eprfs-live-corpus keys on each cluster-state manifest and never on the habits projection', () => {
  const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const { paths, unresolved } = realReadSet('eprfs-live-corpus');
  assert.deepEqual(unresolved, []);
  assert.equal(readSet({ root: ROOT, sources: ['elohim/eprfs/**'] }).paths.some(read => readCovers(read, 'genesis/manifests/habits.yaml')),
    false, 'no eprfs scan, scoped or not, may cover the habits projection');
  const covered = file => paths.some(read => readCovers(read, file));
  assert.equal(covered('genesis/manifests/habits.yaml'), false,
    'a read covers genesis/manifests/habits.yaml: name the files read, not the directory');
  const manifests = readdirSync(resolve(ROOT, 'genesis/manifests'))
    .filter(name => /^cluster-state.*\.yaml$/.test(name));
  assert.ok(manifests.length >= 2, 'the cluster-state manifests are absent: the listing is broken');
  for (const name of manifests) {
    assert.ok(covered(`genesis/manifests/${name}`),
      `genesis/manifests/${name} is unnamed: add its gate-reads line in epr-cli/tests/flow_scope.rs`);
  }
});

// Row 7's receipt on the real tree: a documentation edit falls outside every eprfs workspace read
// and inside the live-corpus project's, so only eprfs-live-corpus re-keys for it.
test('REAL a documentation edit re-keys eprfs-live-corpus and never the eprfs workspace gate', () => {
  const workspace = realReadSet('eprfs');
  const live = realReadSet('eprfs-live-corpus');
  const covers = (reads, file) => reads.paths.some(read => readCovers(read, file));
  for (const file of [
    'genesis/docs/superpowers/sprints/2026-11-17-scratch-sprint-journal.md',
    'genesis/docs/content/elohim-protocol/scratch.md',
    '.claude/memory/scratch-note.md',
    'elohim/sdk/CLAUDE.md',
    'genesis/a2o/features/devflow/scratch.feature',
    'genesis/research/scratch.md',
    'elohim/eprfs/.epr-meta',
  ]) {
    assert.ok(!covers(workspace, file), `the eprfs workspace key reads ${file}`);
    assert.ok(covers(live, file), `eprfs-live-corpus does not key on ${file}`);
  }
});

test('REAL every gate-reads(<project>) scope names a registered gate project whose sources scan the file', () => {
  const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const registry = loadGateRegistry(repo);
  const scanned = name => {
    const project = registry.get(name);
    const manifest = JSON.parse(readFileSync(resolve(repo, project.manifestPath), 'utf8'));
    return [...new Set(project.steps.flatMap(step => manifest.steps[step].inputs?.sources || []))];
  };
  const failures = [];
  let scopes = 0;
  const walk = dir => {
    for (const entry of readdirSync(resolve(repo, dir), { withFileTypes: true })) {
      const rel = `${dir}/${entry.name}`;
      if (entry.isDirectory()) { if (!['target', 'node_modules', 'fixtures'].includes(entry.name)) walk(rel); continue; }
      if (!entry.name.endsWith('.rs')) continue;
      for (const { project, line } of declarationScopes(readFileSync(resolve(repo, rel), 'utf8'))) {
        scopes += 1;
        // A misspelt scope would drop the read from every key: refuse it.
        if (!registry.has(project)) failures.push(`${rel}:${line} names unknown gate project ${project}`);
        else if (!scanned(project).some(glob => globCovers(glob, rel))) failures.push(`${rel}:${line}: ${project} does not scan this file`);
      }
    }
  };
  walk('elohim/eprfs');
  assert.ok(scopes >= 20, `only ${scopes} scoped declarations found: the live-corpus legs lost their scope`);
  assert.deepEqual(failures, []);
});

// The Rust side keeps its own reader of these lines (epr-cli tests/common/mod.rs
// assert_declared_reads); the scoped form once reached the audit and not it (S2 red, 2026-11-17).
// One fixture pins the grammar both must accept; flow_cites.rs reads the same rows.
test('the gate-reads grammar fixture shared with the Rust reader parses as declared', () => {
  const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const rows = readFileSync(resolve(repo, 'elohim/eprfs/epr-cli/tests/fixtures/gate-reads-grammar.tsv'), 'utf8')
    .split('\n').filter(row => row && !row.startsWith('#'));
  assert.ok(rows.length >= 10, 'the grammar fixture lost its rows');
  for (const row of rows) {
    const [verdict, path, ...rest] = row.split('\t');
    const line = rest.join('\t');
    const parsed = declarationsIn(`${line}\n`).map(d => d.path);
    if (verdict === 'declares') assert.deepEqual(parsed, [path], line);
    else {
      assert.equal(verdict, 'none', row);
      assert.deepEqual(parsed, [], line);
    }
  }
});
