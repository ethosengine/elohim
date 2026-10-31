import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import Ajv2020 from 'ajv/dist/2020.js';

const root = fileURLToPath(new URL('../../../../../', import.meta.url));
const readJson = async (path) => JSON.parse(await readFile(resolve(root, path), 'utf8'));
const agent = await readJson('.epr-meta/elohim/packages/agents/stasis-keeper.json');
const skill = await readJson('.epr-meta/elohim/packages/skills/stasis-keeper-brief.json');
const runtimes = (await readJson('genesis/agentic/agent-runtimes.json')).runtimes;

test('keeper packages are born native and validate against their package schemas', async () => {
  for (const [kind, pkg] of [['agent', agent], ['skill', skill]]) {
    const schema = await readJson(`elohim/sdk/domains/elohim-agent/schemas/${kind}-package.schema.json`);
    const validate = new Ajv2020({ allErrors: true }).compile(schema);
    assert(validate(pkg), JSON.stringify(validate.errors));
    assert.equal(pkg.metadata.sourceRuntime, 'elohim-agent');
    assert.equal(pkg.metadata.master, 'package');
    assert.equal(pkg.metadata.composedBy, 'gpt-6-astra (codex)');
  }
  assert.deepEqual(skill.metadata.runtimeTargets, ['claude', 'codex', 'antigravity']);
});

test('keeper projection fixtures expose only read-only tools', async () => {
  assert.deepEqual(agent.metadata.toolRefs, ['Read', 'Grep', 'Glob']);
  assert.deepEqual(agent.metadata.mcpServerRefs, []);
  assert.deepEqual(agent.metadata.capabilityProfile, {
    level: 'C1', effort: 'low', authority: 'read-only-nonprompting', independence: 'same-family',
  });
  // Fixtures can be checked before runtime installation; verify owns freshness.
  for (const runtime of Object.keys(agent.projections)) {
    const fixture = await readFile(
      resolve(root, `.epr-meta/elohim/projections/${runtime}/agents/stasis-keeper.md`), 'utf8',
    );
    const frontmatter = fixture.split('---\n')[1];
    assert.match(frontmatter, /^name: stasis-keeper$/m);
    assert.match(frontmatter, /^tools: Read, Grep, Glob$/m);
    assert.equal((frontmatter.match(/^tools:/gm) ?? []).length, 1);
    assert.doesNotMatch(frontmatter, /^mcpServers:/m);
    if (runtime === 'claude') {
      const binding = runtimes.find((entry) => entry.id === 'claude-code').capabilityBindings.C1;
      assert.equal(binding.model, 'haiku');
      assert.match(frontmatter, /^model: haiku$/m);
      assert.match(frontmatter, /^effort: low$/m);
    } else {
      assert.doesNotMatch(frontmatter, /^(model|model_reasoning_effort):/m);
    }
  }
});

test('declared keeper invocation preserves the nonprompting boundary without calling a model', () => {
  const invocation = runtimes.find((entry) => entry.id === 'claude-code').keeperInvocation;
  assert.equal(invocation.status, 'probed');
  assert.equal(invocation.env.STASIS_KEEPER, '1');
  assert(invocation.argv.includes('--restricted'));
  assert.equal(invocation.argv[invocation.argv.indexOf('--permission-prompts') + 1], 'none');
  assert.equal(invocation.argv[invocation.argv.indexOf('--tools') + 1], 'Read,Grep,Glob');
  // This checks the declared contract; no live prompt/incidence measurement is claimed.
});
