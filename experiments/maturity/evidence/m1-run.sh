#!/usr/bin/env bash
# M1 — does emitted Rust compile?  Usage: run from repo root after `cargo build --bin loom`.
set -u; LOOM=target/debug/loom; OUT=${1:-/tmp/m1}; mkdir -p "$OUT"; : > "$OUT/results.tsv"
for f in $(git ls-files 'corpus/*.loom' 'corpus/**/*.loom' 'examples/*.loom' 'examples/**/*.loom' | sort -u); do
  n=$(echo "${f%.loom}" | tr '/-' '__'); rs="$OUT/$n.rs"
  $LOOM compile "$f" -o "$rs" >"$OUT/$n.cerr" 2>&1 || { printf "%s\tLOOM_FAIL\t\n" "$f" >> "$OUT/results.tsv"; continue; }
  if rustc --edition 2021 --crate-type lib --crate-name m1 -A warnings "$rs" -o "$OUT/$n.rlib" >"$OUT/$n.rerr" 2>&1
  then printf "%s\tOK\t\n" "$f" >> "$OUT/results.tsv"
  else printf "%s\tRUSTC_FAIL\t%s\n" "$f" "$(grep -m1 '^error' "$OUT/$n.rerr" | head -c 150)" >> "$OUT/results.tsv"; fi
done
echo "ok=$(grep -c -P '\tOK\t' $OUT/results.tsv) total=$(wc -l < $OUT/results.tsv)"
