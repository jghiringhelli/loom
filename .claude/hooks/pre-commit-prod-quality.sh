#!/bin/bash
STAGED=$(git diff --cached --name-only --diff-filter=ACM)
SOURCE_FILES=$(echo "$STAGED" | grep -E '\.(py|ts|tsx|js|jsx|rs)$' | grep -vE '(test_|\.test\.|\.spec\.|__tests__|tests/|fixtures/|mock|conftest|_test\.rs)')
if [ -z "$SOURCE_FILES" ]; then exit 0; fi
VIOLATIONS=0
WARNINGS=0
# Check if a file is covered by a hook exception in .forgecraft/exceptions.json
# Usage: is_excepted "layer-boundary" "src/migrations/001.ts"
# Add entries to .forgecraft/exceptions.json to record known false positives.
is_excepted() {
  local hook_name="$1"
  local file_path="$2"
  if [ ! -f ".forgecraft/exceptions.json" ]; then return 1; fi
  node -e "
    const fs = require('fs');
    const data = JSON.parse(fs.readFileSync('.forgecraft/exceptions.json', 'utf-8'));
    const exc = (data.exceptions || []).find(e => {
      if (e.hook !== '$hook_name') return false;
      const pat = e.pattern.replace(/\\/g, '/').replace(/\./g, '\\\\.').replace(/\*\*/g, '<<<D>>>').replace(/\*/g, '[^/]*').replace(/<<<D>>>/g, '.*');
      return new RegExp('^' + pat + '$').test('$file_path'.replace(/\\\\/g, '/'));
    });
    if (exc) { console.log('EXCEPTED: ' + exc.reason); process.exit(0); }
    process.exit(1);
  " 2>/dev/null
}
echo "🔍 Scanning for production code anti-patterns..."
for file in $SOURCE_FILES; do
  if echo "$file" | grep -vqE '(config|settings|\.env)'; then
    if grep -nE '(localhost|127\.0\.0\.1|0\.0\.0\.0)' "$file" | grep -vE '(#|//|""")' > /tmp/violations 2>/dev/null; then
      if [ -s /tmp/violations ]; then
        echo "  ❌ $file — hardcoded URL/host"
        VIOLATIONS=$((VIOLATIONS + 1))
      fi
    fi
  fi
  if ! is_excepted "anti-pattern/mock-data" "$file"; then
    if grep -nEi '\b(mock_data|fake_data|dummy_data|stub_response)' "$file" > /tmp/violations 2>/dev/null; then
      if [ -s /tmp/violations ]; then
        echo "  ❌ $file — mock/stub data in production code"
        VIOLATIONS=$((VIOLATIONS + 1))
      fi
    fi
  fi
  # Layer boundary: no direct DB/ORM imports from route handlers / controllers
  if echo "$file" | grep -qE '(routes|controllers|handlers|endpoints)'; then
    if ! is_excepted "layer-boundary" "$file"; then
      if grep -nE '\b(prisma\.|knex\(|mongoose\.|sequelize\.|db\.query|pool\.query)' "$file" > /tmp/violations 2>/dev/null; then
        if [ -s /tmp/violations ]; then
          echo "  ❌ $file — direct DB call in route/controller (layer violation)"
          VIOLATIONS=$((VIOLATIONS + 1))
        fi
      fi
    fi
  fi
  # Bare Error throws in business logic (not test files)
  if ! is_excepted "error-hierarchy" "$file"; then
    if grep -nE 'throw new Error\(' "$file" > /tmp/violations 2>/dev/null; then
      if [ -s /tmp/violations ]; then
        echo "  ⚠️  $file — bare 'throw new Error()' found — use custom error hierarchy"
        WARNINGS=$((WARNINGS + 1))
      fi
    fi
  fi
  LINE_COUNT=$(wc -l < "$file")
  MAX_LINES=300
  # Codegen files are legitimately larger (emitter tables, not business logic)
  if echo "$file" | grep -qE 'src/codegen/'; then MAX_LINES=2500; fi
  if [ "$LINE_COUNT" -gt "$MAX_LINES" ]; then
    echo "  ⚠️  $file — $LINE_COUNT lines (max $MAX_LINES)"
    WARNINGS=$((WARNINGS + 1))
  fi
  # Rust-specific anti-patterns
  if echo "$file" | grep -q '\.rs$'; then
    if ! is_excepted "rust/unwrap" "$file"; then
      if grep -nE '\.unwrap\(\)' "$file" > /tmp/violations 2>/dev/null; then
        if [ -s /tmp/violations ]; then
          echo "  ⚠️  $file — .unwrap() in production code — use ? or explicit error handling"
          WARNINGS=$((WARNINGS + 1))
        fi
      fi
    fi
    if ! is_excepted "rust/todo" "$file"; then
      # Exclude codegen files: todo!() appears as string literals in generated output
      if echo "$file" | grep -qE 'src/codegen/'; then
        : # codegen emits todo!() as string content — skip check
      elif grep -nE '\btodo!\(|\bunimplemented!\(' "$file" > /tmp/violations 2>/dev/null; then
        if [ -s /tmp/violations ]; then
          echo "  ❌ $file — todo!/unimplemented! in production code"
          VIOLATIONS=$((VIOLATIONS + 1))
        fi
      fi
    fi
    if grep -nE '^[[:space:]]*#\[allow\(dead_code\)\]' "$file" > /tmp/violations 2>/dev/null; then
      if [ -s /tmp/violations ]; then
        echo "  ⚠️  $file — #[allow(dead_code)] suppression — delete orphaned code instead"
        WARNINGS=$((WARNINGS + 1))
      fi
    fi
    if grep -nE '^[[:space:]]*unsafe[[:space:]]*\{' "$file" > /tmp/violations 2>/dev/null; then
      if [ -s /tmp/violations ]; then
        echo "  ⚠️  $file — unsafe block present — requires explicit justification comment"
        WARNINGS=$((WARNINGS + 1))
      fi
    fi
  fi
done
rm -f /tmp/violations
if [ $VIOLATIONS -gt 0 ]; then
  echo "❌ $VIOLATIONS violation(s) found — commit blocked."
  exit 1
fi
if [ $WARNINGS -gt 0 ]; then
  echo "⚠️  $WARNINGS warning(s) found — review recommended."
fi
echo "🔍 Production quality scan passed"
