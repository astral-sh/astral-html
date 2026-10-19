#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
    echo "usage: $0 RESILIENT_CHECKOUT CORPUS_DIRECTORY RESULTS_DIRECTORY" >&2
    exit 2
fi

benchmark_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
resilient_checkout=$(CDPATH= cd -- "$1" && pwd -P)
corpus_directory=$(CDPATH= cd -- "$2" && pwd -P)
results_directory=$3
mkdir -p "$results_directory"
results_directory=$(CDPATH= cd -- "$results_directory" && pwd -P)

if [ -z "${CARGO_HOME:-}" ]; then
    CARGO_HOME="${XDG_CACHE_HOME:-$HOME/.cache}/astral-html-servo-cargo"
    export CARGO_HOME
fi

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT HUP INT TERM
mkdir "$temporary_directory/src"
cp "$benchmark_dir/src/main.rs" "$temporary_directory/src/main.rs"
cp "$benchmark_dir/Cargo.lock" "$temporary_directory/Cargo.lock"

cat > "$temporary_directory/Cargo.toml" <<EOF
[package]
name = "astral-servo-comparison"
version = "0.1.0"
edition = "2024"
publish = false

[workspace]

[features]
resilient = []

[dependencies]
astral-html = { path = '$resilient_checkout/crates/astral-html' }
html5ever = "=0.39.0"
markup5ever_rcdom = "=0.39.0"
EOF

{
    echo "baseline_commit=$(git -C "$benchmark_dir" rev-parse HEAD)"
    echo "resilient_commit=$(git -C "$resilient_checkout" rev-parse HEAD)"
    echo "rustc=$(rustc --version)"
    echo "cargo=$(cargo --version)"
    echo "corpus_directory=$corpus_directory"
    echo "benchmark_cpu=${BENCHMARK_CPU:-all available CPUs}"
    echo "timestamp_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    uname -a
    lscpu | grep -E '^Model name:|^CPU\(s\):|^Architecture:' || true
} > "$results_directory/environment.txt"

cargo test --manifest-path "$benchmark_dir/Cargo.toml" --locked
cargo test --manifest-path "$temporary_directory/Cargo.toml" --locked --features resilient
if [ "${BENCHMARK_TEST_ONLY:-}" = 1 ]; then
    exit 0
fi
cargo build --manifest-path "$benchmark_dir/Cargo.toml" --release --locked
cargo build --manifest-path "$temporary_directory/Cargo.toml" --release --locked --features resilient

baseline_binary="$benchmark_dir/target/release/astral-servo-comparison"
resilient_binary="$temporary_directory/target/release/astral-servo-comparison"

benchmark_binary() {
    if [ -n "${BENCHMARK_CPU:-}" ]; then
        taskset -c "$BENCHMARK_CPU" "$@"
    else
        "$@"
    fi
}

benchmark_binary "$baseline_binary" "$corpus_directory" > "$results_directory/baseline-1.tsv"
benchmark_binary "$resilient_binary" "$corpus_directory" > "$results_directory/recovering-1.tsv"
benchmark_binary "$resilient_binary" "$corpus_directory" > "$results_directory/recovering-2.tsv"
benchmark_binary "$baseline_binary" "$corpus_directory" > "$results_directory/baseline-2.tsv"
benchmark_binary "$baseline_binary" "$corpus_directory" > "$results_directory/baseline-3.tsv"
benchmark_binary "$resilient_binary" "$corpus_directory" > "$results_directory/recovering-3.tsv"

python3 "$benchmark_dir/summarize.py" "$results_directory" | tee "$results_directory/summary.txt"
