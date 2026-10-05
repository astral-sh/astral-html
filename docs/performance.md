# Performance

The benchmark measures `astral-html` parsing, extraction of the HTML fields
consumed by uv, and destruction of the output. Input creation, URL handling,
package validation, network requests, and resolution are excluded.

## Running benchmarks

```console
cargo bench -p astral-html --bench parse --locked
```

The Criterion harness runs all 58 cases by default using the system allocator.
The inputs include captured and generated package indexes, all 32 pinned uv HTML
fixtures, many attributes, long text, entities, names, comments, scripts, and
repeated small text-mode elements. Entity cases cover uncommon and unknown
references and long text prefixes.

Pass a name filter and Criterion options after `--` to select cases and adjust
sampling. Warmup and measurement times are in seconds:

```console
cargo bench -p astral-html --bench parse --locked -- root-10000-generated --sample-size 20 --warm-up-time 1 --measurement-time 3
```

Criterion reports timings and throughput, saves results under
`target/criterion`, and compares against the previous run.

Pin the executable to one available CPU with `taskset -c <cpu>` on Linux and
repeat runs with the same binary. Record the source revision, compiler, CPU,
allocator, and settings alongside results. Cases run in fixed order; generated
inputs are stress tests, not production captures.

The harness does more extraction than uv in some cases: root indexes also
extract base/status fields and decode hrefs, and project indexes read both
metadata attributes instead of short-circuiting the fallback. This benchmark
does not measure uv-client end to end.

## CodSpeed

The [benchmark workflow](../.github/workflows/benchmarks.yml) follows
[Ruff's CodSpeed setup](https://github.com/astral-sh/ruff/blob/ca53cef36054b5a9322e6897fe860e8ed5e00c1d/.github/workflows/ci.yaml).
It runs all cases in one job on pull requests, pushes to `main`, and manual
dispatches, recording CPU simulation and memory measurements. The same `parse`
harness runs locally with Criterion and in CI with CodSpeed instrumentation.
Input construction happens outside the measured region.

The workflow uses the system allocator and the existing `profiling` Cargo
profile for symbolized results. Enable `astral-sh/astral-html` in the CodSpeed
GitHub app to receive reports. Uploads use GitHub OIDC; no `CODSPEED_TOKEN`
secret is needed.

To build and check the instrumented benchmarks locally:

```console
cargo install cargo-codspeed --version 5.0.1 --locked
cargo codspeed build -m simulation -m memory --profile profiling -p astral-html --bench parse --locked
cargo codspeed run --bench parse
```

`cargo codspeed run` validates the instrumented benchmarks locally; collection
and upload happen inside the CodSpeed action in CI. Append a name filter to run
selected cases, for example
`cargo codspeed run --bench parse root-10000-generated`.

## Profiling

Select one case and run it for approximately 10 seconds without statistical
analysis or saving results:

```console
cargo bench -p astral-html --bench parse --profile profiling --locked -- root-10000-generated --profile-time 10
```

Attach a profiler to the compiled executable with
`--bench root-10000-generated --profile-time 10`. The `profiling` Cargo profile
includes debug symbols.

Running the executable without `--bench`, or with `--test`, parses and extracts
each selected input once without collecting timings. This is also how
`cargo test --all-targets` runs it.
