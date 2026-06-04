/**
 * Referential-integrity check for vocabulary.quiltPolicies references
 * (tiered-quilt §4, amended 2026-06-04).
 *
 * JSON Schema cannot express cross-key references, and the staged-intents
 * substrate documented exactly where that bites: a typo'd name "passes
 * manifest validation and fails later at runtime lookup". This check is the
 * loader-enforced rule that closes that trap for quilt policies: every
 * contentTypes.<type>.quiltPolicy and vocabulary.quiltPolicyDefault MUST name
 * a declared vocabulary.quiltPolicies key.
 *
 * Called by test-manifest-quilt-policy.mjs AND codegen-manifest.mjs (fails
 * codegen loud on a dangling reference).
 *
 * @param {object} manifest - parsed app manifest
 * @returns {string[]} human-readable errors; empty array = clean
 */
export function validateQuiltPolicyRefs(manifest) {
  const errors = [];
  const vocab = manifest?.vocabulary ?? {};
  const declared = new Set(Object.keys(vocab.quiltPolicies ?? {}));
  const dangling = (ref) => !declared.has(ref);

  if (vocab.quiltPolicyDefault !== undefined && dangling(vocab.quiltPolicyDefault)) {
    errors.push(
      `vocabulary.quiltPolicyDefault "${vocab.quiltPolicyDefault}" references no declared vocabulary.quiltPolicies entry`,
    );
  }
  for (const [typeName, decl] of Object.entries(vocab.contentTypes ?? {})) {
    const ref = decl?.quiltPolicy;
    if (ref !== undefined && dangling(ref)) {
      errors.push(
        `vocabulary.contentTypes.${typeName}.quiltPolicy "${ref}" references no declared vocabulary.quiltPolicies entry`,
      );
    }
  }
  return errors;
}
