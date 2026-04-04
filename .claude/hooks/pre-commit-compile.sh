#!/bin/bash
echo "🔨 Running build check..."
if [ -f "pyproject.toml" ] || [ -f "setup.py" ] || [ -f "requirements.txt" ]; then
  STAGED_PY=$(git diff --cached --name-only --diff-filter=ACM | grep '\.py$')
  if [ -n "$STAGED_PY" ]; then
    for file in $STAGED_PY; do
      python -m py_compile "$file" 2>&1
      if [ $? -ne 0 ]; then
        echo "❌ Syntax error in $file"
        exit 1
      fi
    done
    echo "  ✅ Python syntax OK"
  fi
fi
if [ -f "tsconfig.json" ]; then
  npx tsc --noEmit 2>&1
  if [ $? -ne 0 ]; then
    echo "❌ TypeScript compilation failed."
    exit 1
  fi
  echo "  ✅ TypeScript compilation OK"
fi
if [ -f "Cargo.toml" ]; then
  STAGED_RS=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$')
  if [ -n "$STAGED_RS" ]; then
    cargo check --quiet 2>&1
    if [ $? -ne 0 ]; then
      echo "❌ Rust cargo check failed."
      exit 1
    fi
    echo "  ✅ Rust cargo check OK"
  fi
fi
echo "🔨 Build check passed"
