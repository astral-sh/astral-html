# Performance

The `parse` benchmark measures HTML parsing, extraction of the fields used by
uv, and output destruction. Inputs are prepared before timing. Cases cover
captured package indexes and generated workloads for text, attributes, entities,
comments, and scripts. The harness uses the system allocator.

## Local benchmarks

Run all cases with Criterion:

```console
cargo bench -p astral-html --bench parse --locked
```

Append a name filter to select cases:

```console
cargo bench -p astral-html --bench parse --locked -- root-10000-generated
```

Criterion saves timings and throughput under `target/criterion` and compares
against the previous run. For a ten-second profiling run with debug symbols:

```console
cargo bench -p astral-html --bench parse --profile profiling --locked -- root-10000-generated --profile-time 10
```

## CodSpeed

The [benchmark workflow](../.github/workflows/benchmarks.yml) runs the same
harness on pull requests and pushes to `main`, collecting CPU simulation and
memory measurements.

To check the instrumented benchmarks locally:

```console
cargo install cargo-codspeed --version 5.0.1 --locked
cargo codspeed build -m simulation -m memory --profile profiling -p astral-html --bench parse --locked
cargo codspeed run --bench parse
```

Local runs validate the cases; CodSpeed collects performance measurements in CI.
