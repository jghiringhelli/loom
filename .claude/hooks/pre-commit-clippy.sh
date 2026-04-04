#!/bin/bash
if [ ! -f "Cargo.toml" ]; then
  exit 0
fi
STAGED_RS=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$')
if [ -z "$STAGED_RS" ]; then
  exit 0
fi
echo "🦀 Running cargo clippy..."
cargo clippy --all-targets --all-features 2>&1
if [ $? -ne 0 ]; then
  echo "❌ cargo clippy failed — fix lint errors before committing."
  echo "   Run: cargo clippy --all-targets --all-features"
  exit 1
fi
echo "  ✅ cargo clippy passed"
