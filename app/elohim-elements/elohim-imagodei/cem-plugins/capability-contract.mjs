/**
 * CEM analyzer plugin: reads @capability* JSDoc tags on Lit element classes
 * and writes them into a `capabilityContract` field on the declaration.
 *
 * Recognized tags:
 *   @capabilityMaxLens <Lens>
 *   @capabilityThemes <Theme,...>
 *   @capabilityContrast <Contrast,...>
 *   @capabilityLocales <Locale,...>
 *   @capabilityMaxStimulus <Stimulus>
 *   @capabilityTextuality <Textuality,...>
 *   @capabilityRequiredStandings <DSL>
 *   @capabilityOptionalStandings <DSL>
 *   @capabilityContentCertainty observed | not-observed
 *   @capabilityStates <name:status, ...>
 *
 * The gate fields (a11y, i18n, uaPrefs) are NOT read from tags; they are written
 * by the test runner. The plugin emits "unknown" for those three fields so they
 * are visible in the contract even before tests run.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §7
 */

const SIMPLE_LIST_TAGS = new Set([
  'capabilityThemes',
  'capabilityContrast',
  'capabilityLocales',
  'capabilityTextuality',
]);

function parseList(raw) {
  return raw
    .split(',')
    .map(t => t.trim())
    .filter(t => t.length > 0);
}

function parseStandingDsl(raw) {
  // each comma-separated entry is one AND-group; | within entries is OR (preserved as string)
  return parseList(raw);
}

function parseStates(raw) {
  // "empty:designed, error:not-yet, contested:n/a"
  const out = {};
  for (const entry of parseList(raw)) {
    const [name, status] = entry.split(':').map(t => t.trim());
    if (name && status) out[name] = status;
  }
  return out;
}

export default function capabilityContractPlugin() {
  return {
    name: 'capability-contract',
    analyzePhase({ ts, node, moduleDoc }) {
      // Only operate on class declarations
      if (!ts.isClassDeclaration(node) || !node.name) return;
      const className = node.name.text;
      const decl = moduleDoc.declarations?.find(
        d => d.kind === 'class' && d.name === className
      );
      if (!decl) return;

      const jsdoc = ts.getJSDocTags(node);
      if (!jsdoc || jsdoc.length === 0) return;

      const contract = {
        a11y: 'unknown',
        i18n: 'unknown',
        uaPrefs: 'unknown',
      };

      for (const tag of jsdoc) {
        const tagName = tag.tagName.text;
        const text = typeof tag.comment === 'string' ? tag.comment : '';
        const raw = text.replace(/\/\/.*$/, '').trim();
        if (!raw) continue;

        if (tagName === 'capabilityMaxLens') contract.maxLens = raw;
        else if (tagName === 'capabilityMaxStimulus') contract.maxStimulus = raw;
        else if (tagName === 'capabilityContentCertainty') contract.contentCertainty = raw;
        else if (SIMPLE_LIST_TAGS.has(tagName)) {
          const key = tagName === 'capabilityThemes' ? 'themes'
            : tagName === 'capabilityContrast' ? 'contrast'
            : tagName === 'capabilityLocales' ? 'locales'
            : 'textuality';
          contract[key] = parseList(raw);
        } else if (tagName === 'capabilityRequiredStandings') {
          contract.standings = contract.standings || {};
          contract.standings.required = parseStandingDsl(raw);
        } else if (tagName === 'capabilityOptionalStandings') {
          contract.standings = contract.standings || {};
          contract.standings.optional = parseStandingDsl(raw);
        } else if (tagName === 'capabilityStates') {
          contract.states = parseStates(raw);
        }
      }

      // Only attach if any capability tag fired (besides the always-present gate stubs)
      const keys = Object.keys(contract);
      if (keys.length > 3) {
        decl.capabilityContract = contract;
      }
    },
  };
}
