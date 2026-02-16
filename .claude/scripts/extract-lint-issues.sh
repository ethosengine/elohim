#!/bin/bash
# Extract lint issues into a manifest for parallel fixing
#
# Usage:
#   ./extract-lint-issues.sh                        # elohim-app (default)
#   ./extract-lint-issues.sh --project doorway      # doorway clippy issues
#   ./extract-lint-issues.sh --project doorway-app  # doorway-app ESLint
#   ./extract-lint-issues.sh --project sophia       # sophia ESLint
#   ./extract-lint-issues.sh --project all          # all projects combined

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$PROJECT_DIR/.claude"

# Parse args
TARGET_PROJECT="elohim-app"
while [[ $# -gt 0 ]]; do
  case $1 in
    --project) TARGET_PROJECT="$2"; shift 2 ;;
    *) echo "Unknown arg: $1. Usage: $0 [--project elohim-app|doorway|doorway-app|sophia|all]"; exit 1 ;;
  esac
done

MANIFEST_FILE="$OUTPUT_DIR/lint-manifest.json"

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
  "sonarjs/no-identical-functions": "judgment",
  "@typescript-eslint/no-unnecessary-type-assertion": "mechanical",
  "@typescript-eslint/await-thenable": "mechanical",
  "@typescript-eslint/max-params": "contextual",
  "unicorn/prefer-set-has": "contextual",
  "unicorn/no-zero-fractions": "mechanical",
  "unicorn/prefer-number-properties": "mechanical",
  "unicorn/prefer-code-point": "mechanical",
  "unicorn/prefer-array-index-of": "mechanical",
  "unicorn/no-typeof-undefined": "mechanical",
  "unicorn/prefer-export-from": "mechanical",
  "clippy::unnecessary_clone": "mechanical",
  "clippy::needless_return": "mechanical",
  "clippy::manual_map": "mechanical",
  "clippy::needless_borrow": "mechanical",
  "clippy::redundant_closure": "mechanical",
  "clippy::single_match": "mechanical",
  "clippy::match_single_binding": "mechanical",
  "clippy::len_zero": "mechanical",
  "clippy::cognitive_complexity": "contextual",
  "clippy::too_many_arguments": "contextual",
  "clippy::missing_docs": "judgment",
  "clippy::missing_safety_doc": "judgment"
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
  "color-function-notation": "Use modern syntax: rgb(255 255 255) not rgba(255, 255, 255, 1)",
  "@typescript-eslint/no-unnecessary-type-assertion": "Remove redundant `as Type` or `!` assertion - TypeScript already knows the type",
  "@typescript-eslint/await-thenable": "Remove `await` - the value is not a Promise/Thenable",
  "@typescript-eslint/max-params": "Extract parameters into an options object",
  "unicorn/prefer-set-has": "Convert array to `new Set([...])` and use `.has()` instead of `.includes()`",
  "unicorn/no-zero-fractions": "Replace `0.0` with `0`, `1.0` with `1`, etc.",
  "unicorn/prefer-number-properties": "Use `Number.isNaN()` instead of `isNaN()`, `Number.parseInt()` instead of `parseInt()`",
  "unicorn/prefer-code-point": "Use `String.fromCodePoint()` instead of `String.fromCharCode()`",
  "unicorn/prefer-array-index-of": "Use `.indexOf()` instead of `.findIndex()` for simple values",
  "unicorn/no-typeof-undefined": "Use `=== undefined` instead of `typeof x === 'undefined'`",
  "unicorn/prefer-export-from": "Use `export { Foo } from './bar'` instead of importing then re-exporting",
  "clippy::unnecessary_clone": "Remove .clone() when ownership can transfer or use a reference",
  "clippy::needless_return": "Remove explicit return at end of function",
  "clippy::manual_map": "Replace manual match with .map()",
  "clippy::needless_borrow": "Remove & when value is already a reference",
  "clippy::redundant_closure": "Replace |x| foo(x) with foo",
  "clippy::cognitive_complexity": "Extract complex logic into smaller helper functions",
  "clippy::too_many_arguments": "Group parameters into a config struct"
}
HINTS_EOF
)

# ============================================================================
# Extract functions per project
# ============================================================================

extract_elohim_app() {
  local APP_DIR="$PROJECT_DIR/elohim-app"
  cd "$APP_DIR"
  echo "Extracting lint issues from elohim-app..."

  # ESLint
  echo "  Running ESLint..."
  ESLINT_OUTPUT=$(npx eslint src --ext .ts,.html -f json 2>/dev/null || true)

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
       source: "eslint",
       project: "elohim-app"
     }]
  ' > /tmp/elohim-app-eslint.json

  # Stylelint
  echo "  Running Stylelint..."
  STYLELINT_OUTPUT=$(npx stylelint "src/**/*.{css,scss}" -f json 2>/dev/null || true)

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
       source: "stylelint",
       project: "elohim-app"
     }]
  ' > /tmp/elohim-app-stylelint.json

  jq -s 'add' /tmp/elohim-app-eslint.json /tmp/elohim-app-stylelint.json
}

extract_doorway() {
  echo "Extracting clippy issues from doorway..."
  cd "$PROJECT_DIR/doorway"

  # Run clippy with JSON output
  CLIPPY_OUTPUT=$(RUSTFLAGS="" cargo clippy --message-format=json 2>/dev/null || true)

  echo "$CLIPPY_OUTPUT" | jq --argjson hints "$FIX_HINTS" --argjson tiers "$TIER_MAP" '
    [inputs |
     select(.reason == "compiler-message") |
     .message |
     select(.level == "warning" or .level == "error") |
     select(.code != null) |
     .spans[0] as $span |
     select($span != null) |
     {
       file: ($span.file_name | if startswith("/") then . else ("'"$PROJECT_DIR/doorway/"'" + .) end),
       line: $span.line_start,
       column: $span.column_start,
       ruleId: ("clippy::" + (.code.code | split("::") | last)),
       message: .message,
       severity: .level,
       tier: ($tiers["clippy::" + (.code.code | split("::") | last)] // "contextual"),
       fixHint: ($hints["clippy::" + (.code.code | split("::") | last)] // "Review clippy suggestion and fix accordingly"),
       status: "pending",
       source: "clippy",
       project: "doorway"
     }]
  ' 2>/dev/null || echo '[]'
}

extract_doorway_app() {
  echo "Extracting ESLint issues from doorway-app..."
  cd "$PROJECT_DIR/doorway-app"

  ESLINT_OUTPUT=$(npx eslint src --ext .ts,.html -f json 2>/dev/null || true)

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
       source: "eslint",
       project: "doorway-app"
     }]
  ' 2>/dev/null || echo '[]'
}

extract_sophia() {
  echo "Extracting ESLint issues from sophia..."
  cd "$PROJECT_DIR/sophia"

  ESLINT_OUTPUT=$(pnpm lint -- --format json 2>/dev/null || true)

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
       source: "eslint",
       project: "sophia"
     }]
  ' 2>/dev/null || echo '[]'
}

# ============================================================================
# Main execution
# ============================================================================

ALL_ISSUES="[]"

case "$TARGET_PROJECT" in
  elohim-app)
    ALL_ISSUES=$(extract_elohim_app)
    ;;
  doorway)
    ALL_ISSUES=$(extract_doorway)
    ;;
  doorway-app)
    ALL_ISSUES=$(extract_doorway_app)
    ;;
  sophia)
    ALL_ISSUES=$(extract_sophia)
    ;;
  all)
    echo "Extracting from all projects..."
    ELOHIM=$(extract_elohim_app)
    DOORWAY=$(extract_doorway)
    DOORWAY_APP=$(extract_doorway_app)
    SOPHIA=$(extract_sophia)
    ALL_ISSUES=$(echo "$ELOHIM" "$DOORWAY" "$DOORWAY_APP" "$SOPHIA" | jq -s 'add')
    ;;
  *)
    echo "Unknown project: $TARGET_PROJECT"
    echo "Valid: elohim-app, doorway, doorway-app, sophia, all"
    exit 1
    ;;
esac

# Add unique IDs and write manifest
echo "$ALL_ISSUES" | jq '
  to_entries | map(.value + {id: ("lint-" + (.key + 1 | tostring | if length < 4 then ("0" * (4 - length)) + . else . end))})
' > "$MANIFEST_FILE"

# Summary
TOTAL=$(jq 'length' "$MANIFEST_FILE")
MECHANICAL_COUNT=$(jq '[.[] | select(.tier == "mechanical")] | length' "$MANIFEST_FILE")
CONTEXTUAL_COUNT=$(jq '[.[] | select(.tier == "contextual")] | length' "$MANIFEST_FILE")
JUDGMENT_COUNT=$(jq '[.[] | select(.tier == "judgment")] | length' "$MANIFEST_FILE")

echo ""
echo "=== Lint Manifest Generated ==="
echo "Location: $MANIFEST_FILE"
echo "Project: $TARGET_PROJECT"
echo "Total issues: $TOTAL"

# Per-source breakdown
for src in eslint stylelint clippy; do
  COUNT=$(jq "[.[] | select(.source == \"$src\")] | length" "$MANIFEST_FILE")
  if [ "$COUNT" -gt 0 ]; then
    echo "  ${src}: $COUNT"
  fi
done

echo ""
echo "Tier breakdown:"
echo "  Mechanical:  $MECHANICAL_COUNT"
echo "  Contextual:  $CONTEXTUAL_COUNT"
echo "  Judgment:    $JUDGMENT_COUNT"
echo "  Other:       $((TOTAL - MECHANICAL_COUNT - CONTEXTUAL_COUNT - JUDGMENT_COUNT))"

if [ "$TARGET_PROJECT" = "all" ]; then
  echo ""
  echo "Per-project breakdown:"
  for proj in elohim-app doorway doorway-app sophia; do
    COUNT=$(jq "[.[] | select(.project == \"$proj\")] | length" "$MANIFEST_FILE")
    if [ "$COUNT" -gt 0 ]; then
      echo "  ${proj}: $COUNT"
    fi
  done
fi

echo ""
echo "Top rules:"
jq -r '[.[] | .ruleId] | group_by(.) | map({rule: .[0], count: length}) | sort_by(-.count) | .[0:10] | .[] | "  \(.count)\t\(.rule)"' "$MANIFEST_FILE"
