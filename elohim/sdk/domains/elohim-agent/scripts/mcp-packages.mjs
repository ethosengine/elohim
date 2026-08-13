// mcp-packages.mjs - reusable MCP definitions + explicit project activation.
//
// A server package owns connection identity, never credentials. A profile owns
// activation. Runtime adapters lower those two canonical shapes into Claude's
// project .mcp.json and Codex's trusted-project .codex/config.toml.

export const MCP_SERVER_KIND = 'McpServerPackage';
export const MCP_PROFILE_KIND = 'McpProfilePackage';

function tomlString(value) {
  return JSON.stringify(value);
}

function tomlArray(values) {
  return `[${values.map(tomlString).join(', ')}]`;
}

function tomlScalar(value) {
  if (Array.isArray(value)) return tomlArray(value);
  if (typeof value === 'string') return tomlString(value);
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  throw new Error(`unsupported TOML runtime-setting value: ${JSON.stringify(value)}`);
}

// Top-level (non-table) Codex settings carried by a profile package, e.g.
// `project_doc_max_bytes` — the combined AGENTS.md instruction budget Codex
// applies root-to-cwd (its own default is 32 KiB = 32768 B, which truncates our
// 42,327-byte root gospel mid-line). Sizing rule: measured chain bytes + >=50%
// headroom, rounded UP to a power-of-two KiB. 42327 * 1.5 = 63490.5 -> 65536
// (64 KiB) = 1.548x the measured root, leaving 23,209 B for the subtree
// AGENTS.md files the restructure will add. Keys are emitted sorted, BEFORE the
// first `[mcp_servers.*]` table (TOML: a bare key after a table header would be
// parsed into that table). Absent `runtimeSettings` projects byte-identically to
// a profile that never declared one.
function codexRuntimeSettings(profile) {
  const settings = profile.runtimeSettings?.codex;
  if (!settings) return '';
  const keys = Object.keys(settings).sort();
  if (!keys.length) return '';
  return `${keys.map((key) => `${key} = ${tomlScalar(settings[key])}`).join('\n')}\n\n`;
}

function claudeEntry(pkg) {
  const connection = pkg.connection;
  if (connection.transport === 'stdio') {
    const entry = { command: connection.command };
    if (connection.args?.length) entry.args = connection.args;
    if (connection.envVars?.length) {
      entry.env = Object.fromEntries(connection.envVars.map((name) => [name, `\${${name}}`]));
    }
    return entry;
  }

  const entry = {
    type: connection.transport === 'streamable-http' ? 'http' : 'sse',
    url: connection.url,
  };
  if (connection.auth.mode === 'bearer-env') {
    entry.headers = { Authorization: `Bearer \${${connection.auth.envVar}}` };
  }
  return entry;
}

function codexTable(pkg) {
  const connection = pkg.connection;
  if (connection.transport === 'sse') {
    throw new Error(
      `${pkg.metadata.id}: Codex does not support legacy SSE MCP transport; provide a streamable-http endpoint or remove codex from runtimeTargets`,
    );
  }

  const lines = [`[mcp_servers.${pkg.metadata.id}]`];
  if (connection.transport === 'stdio') {
    lines.push(`command = ${tomlString(connection.command)}`);
    if (connection.args?.length) lines.push(`args = ${tomlArray(connection.args)}`);
    if (connection.envVars?.length) lines.push(`env_vars = ${tomlArray(connection.envVars)}`);
  } else {
    lines.push(`url = ${tomlString(connection.url)}`);
    if (connection.auth.mode === 'bearer-env') {
      lines.push(`bearer_token_env_var = ${tomlString(connection.auth.envVar)}`);
    }
  }

  const policy = pkg.policy ?? {};
  if (policy.required !== undefined) lines.push(`required = ${policy.required}`);
  if (policy.startupTimeoutSec !== undefined) {
    lines.push(`startup_timeout_sec = ${policy.startupTimeoutSec}`);
  }
  if (policy.toolTimeoutSec !== undefined) {
    lines.push(`tool_timeout_sec = ${policy.toolTimeoutSec}`);
  }
  if (policy.enabledTools?.length) lines.push(`enabled_tools = ${tomlArray(policy.enabledTools)}`);
  if (policy.disabledTools?.length) {
    lines.push(`disabled_tools = ${tomlArray(policy.disabledTools)}`);
  }
  return `${lines.join('\n')}\n`;
}

export function projectMcpServer(pkg, runtime) {
  if (!pkg.metadata.runtimeTargets.includes(runtime)) {
    throw new Error(`${pkg.metadata.id}: runtime ${runtime} is not declared in runtimeTargets`);
  }
  if (runtime === 'claude') {
    return `${JSON.stringify({ mcpServers: { [pkg.metadata.id]: claudeEntry(pkg) } }, null, 2)}\n`;
  }
  if (runtime === 'codex') return codexTable(pkg);
  throw new Error(`${pkg.metadata.id}: unsupported MCP runtime ${runtime}`);
}

export function resolveProfileServers(profile, packages) {
  const byId = new Map(
    packages.filter((pkg) => pkg.kind === MCP_SERVER_KIND).map((pkg) => [pkg.metadata.id, pkg]),
  );
  return profile.serverRefs.map((ref) => {
    const server = byId.get(ref);
    if (!server) throw new Error(`${profile.metadata.id}: unresolved MCP server ref ${ref}`);
    return server;
  });
}

export function projectMcpProfile(profile, packages, runtime) {
  const servers = resolveProfileServers(profile, packages);
  for (const server of servers) {
    if (!server.metadata.runtimeTargets.includes(runtime)) {
      throw new Error(
        `${profile.metadata.id}: MCP server ${server.metadata.id} does not support runtime ${runtime}`,
      );
    }
  }

  if (runtime === 'claude') {
    const mcpServers = Object.fromEntries(
      servers.map((server) => [server.metadata.id, claudeEntry(server)]),
    );
    return `${JSON.stringify({ mcpServers }, null, 2)}\n`;
  }
  if (runtime === 'codex') {
    const header =
      `# Generated from McpProfilePackage:${profile.metadata.id} by package-projections.mjs.\n` +
      '# Edit .epr-meta/elohim/packages/mcp-{servers,profiles}, not this file.\n\n';
    return header + codexRuntimeSettings(profile) + servers.map(codexTable).join('\n');
  }
  throw new Error(`${profile.metadata.id}: unsupported MCP runtime ${runtime}`);
}

export function verifyMcpServerPackage(pkg, { assert }) {
  assert(pkg.metadata.id === pkg.metadata.name, `${pkg.metadata.id} MCP server name/id round-trip`);
  assert(
    Boolean(pkg.metadata.governance?.eprRef),
    `${pkg.metadata.id} MCP server has metadata.governance.eprRef`,
  );
  const projectedRuntimes = Object.keys(pkg.projections).sort();
  const runtimeTargets = [...pkg.metadata.runtimeTargets].sort();
  assert(
    JSON.stringify(projectedRuntimes) === JSON.stringify(runtimeTargets),
    `${pkg.metadata.id} MCP projection runtimes match runtimeTargets`,
  );
  for (const runtime of runtimeTargets) {
    try {
      const projected = projectMcpServer(pkg, runtime);
      assert(projected.length > 0, `${pkg.metadata.id} MCP ${runtime} projection is derivable`);
    } catch (error) {
      assert(false, error.message);
    }
  }
}

export function verifyMcpProfilePackage(pkg, packages, { assert }) {
  assert(
    pkg.metadata.id === pkg.metadata.name,
    `${pkg.metadata.id} MCP profile name/id round-trip`,
  );
  assert(
    Boolean(pkg.metadata.governance?.eprRef),
    `${pkg.metadata.id} MCP profile has metadata.governance.eprRef`,
  );
  const projectedRuntimes = Object.keys(pkg.projections).sort();
  const runtimeTargets = [...pkg.metadata.runtimeTargets].sort();
  assert(
    JSON.stringify(projectedRuntimes) === JSON.stringify(runtimeTargets),
    `${pkg.metadata.id} MCP profile projection runtimes match runtimeTargets`,
  );
  for (const runtime of runtimeTargets) {
    try {
      const projected = projectMcpProfile(pkg, packages, runtime);
      assert(projected.length > 0, `${pkg.metadata.id} MCP ${runtime} profile is derivable`);
    } catch (error) {
      assert(false, error.message);
    }
  }
}
