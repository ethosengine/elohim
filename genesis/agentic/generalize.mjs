/**
 * Cluster near-duplicate palette entries and propose broader patterns
 * under the safety taxonomy. Pure function: returns proposals, never
 * writes to disk. Caller (skill invocation) is responsible for applying
 * approved proposals to settings.json.
 */

const BASH_PATTERN = /^Bash\((.*)\)$/;

function parseEntry(entry) {
  const m = entry.match(BASH_PATTERN);
  if (!m) return null;
  const body = m[1];
  const tokens = body.trim().split(/\s+/);
  // Strip leading VAR=value env assignments; keep the first real command.
  let i = 0;
  while (i < tokens.length && /^[A-Z_][A-Z0-9_]*=/.test(tokens[i])) i++;
  const cmd = tokens[i];
  if (!cmd) return null;
  const envPrefix = tokens.slice(0, i).join(' ');
  const subcommand = tokens[i + 1] ?? null;
  return { entry, body, cmd, subcommand, envPrefix };
}

function classifyCommand(cmd, taxonomy) {
  if (taxonomy.broadly_safe.includes(cmd)) return { tier: 'broadly_safe' };
  const sub = taxonomy.subcommand_scoped.find((e) => e.command === cmd);
  if (sub) return { tier: 'subcommand_scoped', rule: sub };
  if (taxonomy.never_wildcard.find((e) => e.command === cmd)) {
    return { tier: 'never_wildcard' };
  }
  return { tier: 'unknown' };
}

function clusterKey(parsed, classification) {
  const { cmd, subcommand, envPrefix } = parsed;
  const envKey = envPrefix || '';
  if (classification.tier === 'broadly_safe') {
    // All subcommands of a broadly-safe cmd can share one cluster per env prefix.
    return `${envKey}|${cmd}|*`;
  }
  if (classification.tier === 'subcommand_scoped') {
    const safe = classification.rule.safe_subcommands;
    // Only cluster within the safe-subcommand set. The cluster key pins the
    // (cmd, subcommand) pair — generalize args under it with `*`.
    if (!subcommand) return null;
    // Some "subcommands" are multi-token (e.g. 'pr list'); pick the longest prefix match.
    const match = [...safe]
      .sort((a, b) => b.length - a.length)
      .find((s) => {
        const parts = s.split(' ');
        const body = parsed.body
          .split(/\s+/)
          .slice(envKey ? envKey.split(/\s+/).length : 0);
        return parts.every((p, idx) => body[idx + 1] === p || body[idx] === p);
      });
    if (!match) return null;
    return `${envKey}|${cmd}|${match}|*`;
  }
  return null; // never_wildcard + unknown produce no cluster
}

function proposedPattern(clusterKey, parsed, classification) {
  const [envKey, cmd, rest] = clusterKey.split('|');
  const envStr = envKey ? `${envKey} ` : '';
  if (classification.tier === 'broadly_safe') {
    return `Bash(${envStr}${cmd} *)`;
  }
  // subcommand_scoped
  const subcmd = clusterKey.split('|').slice(2, -1).join(' ');
  return `Bash(${envStr}${cmd} ${subcmd} *)`;
}

export function clusterAndPropose(entries, taxonomy) {
  const clusters = new Map();

  for (const entry of entries) {
    const parsed = parseEntry(entry);
    if (!parsed) continue;
    const classification = classifyCommand(parsed.cmd, taxonomy);
    if (classification.tier === 'never_wildcard') continue;
    if (classification.tier === 'unknown') continue;
    const key = clusterKey(parsed, classification);
    if (!key) continue;
    if (!clusters.has(key)) {
      clusters.set(key, { key, classification, parsed, absorbs: [] });
    }
    clusters.get(key).absorbs.push(entry);
  }

  const proposals = [];
  for (const cluster of clusters.values()) {
    if (cluster.absorbs.length < 2) continue; // require at least 2 to justify
    proposals.push({
      proposed: proposedPattern(cluster.key, cluster.parsed, cluster.classification),
      absorbs: cluster.absorbs,
      safety: cluster.classification.tier,
    });
  }
  return proposals;
}
