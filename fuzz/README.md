# cargo-fuzz targets (M3)

Targets: `lex`, `parse`, `compile`. Requires nightly + libFuzzer (Linux/CI):

    cargo install cargo-fuzz
    for t in lex parse compile; do mkdir -p fuzz/corpus/$t; find corpus examples -name '*.loom' -exec cp {} fuzz/corpus/$t/ \; ; done
    cargo +nightly fuzz run compile -- -max_total_time=60 -timeout=1

The deterministic, toolchain-independent stress harness is `tests/fuzz_smoke_test.rs`.
