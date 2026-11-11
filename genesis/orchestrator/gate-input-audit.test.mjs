import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'fs';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import { auditProject, exhaustInputs, globCovers, isReadPattern, readCovers, readSet } from './gate-input-audit.mjs';
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
  assert.ok(result.paths.includes('**/.epr-meta'), 'the governance walk must remain visible as its file kind');
  assert.ok(!result.paths.includes('.'), 'no eprfs read may name the whole repository');
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
