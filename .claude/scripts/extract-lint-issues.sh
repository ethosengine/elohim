#!/bin/bash
# Extract lint issues from ESLint and Stylelint into a manifest for parallel fixing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
APP_DIR="$PROJECT_DIR/elohim-app"
OUTPUT_DIR="$PROJECT_DIR/.claude"
MANIFEST_FILE="$OUTPUT_DIR/lint-manifest.json"

cd "$APP_DIR"

echo "Extracting lint issues from elohim-app..."

# Tier classification for smart dispatch
TIER_MAP=$(cat <<'TIER_EOF'
{
  "@typescript-eslint/no-floating-promises": "mechanical",
  "@typescript-eslint/no-unused-vars": "mechanical",
  "@typescript-eslint/require-await": "mechanical",
  "@typescript-eslint/no-empty-function": "mechanical",
  "@typescript-eslint/prefer-nullish-coalescing": "mechanical",
  "@typescript-eslint/prefer-optional-chain": "mechanical",
  "@typescript-eslint/prefer-readonly": "mechanical",
  "@angular-eslint/template/button-has-type": "mechanical",
  "@angular-eslint/template/no-negated-async": "mechanical",
  "@angular-eslint/template/eqeqeq": "mechanical",
  "@angular-eslint/template/no-duplicate-attributes": "mechanical",
  "import/order": "mechanical",
  "import/no-duplicates": "mechanical",
  "no-console": "mechanical",
  "prefer-const": "mechanical",
  "eqeqeq": "mechanical",
  "a11y/no-outline-none": "mechanical",
  "a11y/media-prefers-reduced-motion": "mechanical",
  "declaration-block-no-duplicate-properties": "mechanical",
  "color-function-notation": "mechanical",
  "alpha-value-notation": "mechanical",
  "prettier/prettier": "mechanical",
  "@typescript-eslint/no-explicit-any": "contextual",
  "@typescript-eslint/no-unsafe-assignment": "contextual",
  "@typescript-eslint/no-unsafe-member-access": "contextual",
  "@typescript-eslint/no-unsafe-call": "contextual",
  "@typescript-eslint/no-unsafe-return": "contextual",
  "@typescript-eslint/no-unsafe-argument": "contextual",
  "sonarjs/cognitive-complexity": "contextual",
  "sonarjs/no-duplicate-string": "contextual",
  "sonarjs/no-nested-conditional": "contextual",
  "sonarjs/todo-tag": "judgment",
  "sonarjs/no-identical-functions": "judgment"
}
TIER_EOF
)

# Fix hints for common rules
FIX_HINTS=$(cat <<'HINTS_EOF'
{
  "@typescript-eslint/no-floating-promises": "Add `void` prefix before the promise expression, or add `await` if in async function",
  "@typescript-eslint/no-explicit-any": "Replace `any` with a specific type, `unknown`, or a generic type parameter",
  "@typescript-eslint/no-unused-vars": "Remove the unused variable, or prefix with `_` if intentionally unused",
  "@typescript-eslint/no-unsafe-assignment": "Add proper type annotation or cast to the specific expected type",
  "@typescript-eslint/no-unsafe-member-access": "Add type guard or proper type annotation before accessing the member",
  "@typescript-eslint/no-unsafe-call": "Add proper type annotation to ensure the value is callable",
  "@typescript-eslint/no-unsafe-return": "Add explicit return type annotation to the function",
  "@typescript-eslint/no-unsafe-argument": "Cast the argument to the expected type or add proper typing",
  "@typescript-eslint/require-await": "Remove `async` keyword if no await needed, or add await to a promise",
  "@typescript-eslint/no-empty-function": "Add a comment explaining why empty, or remove the function",
  "@typescript-eslint/prefer-nullish-coalescing": "Replace `||` with `??` for null/undefined checks",
  "@typescript-eslint/prefer-optional-chain": "Replace `&&` chain with `?.` optional chaining",
  "@typescript-eslint/prefer-readonly": "Add `readonly` modifier to the property",
  "@angular-eslint/template/button-has-type": "Add `type=\"button\"` attribute to the <button> element",
  "@angular-eslint/template/no-negated-async": "Replace `!asyncValue` with `asyncValue === false`",
  "@angular-eslint/template/eqeqeq": "Replace `==` with `===` for strict equality",
  "import/order": "Reorder imports: builtin, external, internal (@app/), parent, sibling, index, type",
  "import/no-duplicates": "Merge duplicate imports from the same module into one import statement",
  "no-console": "Remove console.log or replace with proper logging service",
  "prefer-const": "Change `let` to `const` since variable is never reassigned",
  "eqeqeq": "Replace `==` with `===` and `!=` with `!==`",
  "sonarjs/cognitive-complexity": "Extract complex logic into smaller helper functions",
  "sonarjs/no-duplicate-string": "Extract repeated string into a constant",
  "sonarjs/no-identical-functions": "Extract common logic into a shared function",
  "alpha-value-notation": "Use percentage for alpha: rgb(0 0 0 / 50%) not rgb(0 0 0 / 0.5)",
  "color-function-notation": "Use modern syntax: rgb(255 255 255) not rgba(255, 255, 255, 1)"
}
HINTS_EOF
)

# Extract ESLint issues
echo "Running ESLint..."
ESLINT_OUTPUT=$(npx eslint src --ext .ts,.html -f json 2>/dev/null || true)

# Extract Stylelint issues
echo "Running Stylelint..."
STYLELINT_OUTPUT=$(npx stylelint "src/**/*.{css,scss}" -f json 2>/dev/null || true)

# Convert ESLint issues
echo "Building manifest..."
echo "$ESLINT_OUTPUT" | jq --argjson hints "$FIX_HINTS" --argjson tiers "$TIER_MAP" '
  [.[] | select(.errorCount > 0 or .warningCount > 0) |
   .filePath as $file | .messages[] |
   select(.ruleId != null) |
   {
     file: $file,
     line: .line,
     column: .column,
     ruleId: .ruleId,
     message: .message,
     severity: (if .severity == 2 then "error" else "warning" end),
     tier: ($tiers[.ruleId] // "sonnet"),
     fixHint: ($hints[.ruleId] // "Review the rule documentation and fix accordingly"),
     status: "pending",
     source: "eslint"
   }]
' > /tmp/eslint-issues.json

# Convert Stylelint issues
echo "$STYLELINT_OUTPUT" | jq --argjson hints "$FIX_HINTS" --argjson tiers "$TIER_MAP" '
  [.[] | select(.warnings | length > 0) |
   .source as $file | .warnings[] |
   {
     file: $file,
     line: .line,
     column: .column,
     ruleId: .rule,
     message: .text,
     severity: (if .severity == "error" then "error" else "warning" end),
     tier: ($tiers[.rule] // "sonnet"),
     fixHint: ($hints[.rule] // "Review the rule documentation and fix accordingly"),
     status: "pending",
     source: "stylelint"
   }]
' > /tmp/stylelint-issues.json

# Merge and add unique IDs
jq -s '
  add | to_entries | map(.value + {id: ("lint-" + (.key + 1 | tostring | ("0" * (4 - length)) + .))})
' /tmp/eslint-issues.json /tmp/stylelint-issues.json > "$MANIFEST_FILE"

# Summary
TOTAL=$(jq 'length' "$MANIFEST_FILE")
ESLINT_COUNT=$(jq '[.[] | select(.source == "eslint")] | length' "$MANIFEST_FILE")
STYLELINT_COUNT=$(jq '[.[] | select(.source == "stylelint")] | length' "$MANIFEST_FILE")
MECHANICAL_COUNT=$(jq '[.[] | select(.tier == "mechanical")] | length' "$MANIFEST_FILE")
CONTEXTUAL_COUNT=$(jq '[.[] | select(.tier == "contextual")] | length' "$MANIFEST_FILE")
JUDGMENT_COUNT=$(jq '[.[] | select(.tier == "judgment")] | length' "$MANIFEST_FILE")

echo ""
echo "=== Lint Manifest Generated ==="
echo "Location: $MANIFEST_FILE"
echo "Total issues: $TOTAL"
echo "  ESLint:    $ESLINT_COUNT"
echo "  Stylelint: $STYLELINT_COUNT"
echo ""
echo "Tier breakdown:"
echo "  Mechanical:  $MECHANICAL_COUNT"
echo "  Contextual:  $CONTEXTUAL_COUNT"
echo "  Judgment:    $JUDGMENT_COUNT"
echo "  Other:       $((TOTAL - MECHANICAL_COUNT - CONTEXTUAL_COUNT - JUDGMENT_COUNT))"
echo ""
echo "Top rules:"
jq -r '[.[] | .ruleId] | group_by(.) | map({rule: .[0], count: length}) | sort_by(-.count) | .[0:10] | .[] | "  \(.count)\t\(.rule)"' "$MANIFEST_FILE"
