# Performance

The benchmark measures `astral-html` parsing, extraction of the HTML fields
consumed by uv, and destruction of the output. Input creation, URL handling,
package validation, network requests, and resolution are excluded.

## Running benchmarks

```console
cargo bench -p astral-html --bench parse --locked
cargo bench -p astral-html --bench parse --locked --features benchmark-jemalloc
ASTRAL_HTML_BENCH_SUITE=uv cargo bench -p astral-html --bench parse --locked --features benchmark-jemalloc
```

The default suite includes captured and generated package indexes, many
attributes, long text, and entities. The `uv` suite covers all 32 pinned HTML
fixtures. The `scanning` suite covers names, comments, scripts, and repeated
small text-mode elements. The `entity-scanning` suite covers uncommon and
unknown references and long text prefixes. The jemalloc feature requires Linux
on x86_64 or aarch64. Otherwise, run without the feature to use the system
allocator.

| Variable                      | Default   | Purpose                                                          |
| ----------------------------- | --------- | ---------------------------------------------------------------- |
| `ASTRAL_HTML_BENCH_SUITE`     | `default` | Select `default`, `uv`, `scanning`, or `entity-scanning` inputs. |
| `ASTRAL_HTML_BENCH_CASE`      | All cases | Filter case names by substring.                                  |
| `ASTRAL_HTML_BENCH_SAMPLES`   | `21`      | Samples per case; at least 3.                                    |
| `ASTRAL_HTML_BENCH_SAMPLE_MS` | `100`     | Warmup duration in milliseconds.                                 |

Warmup chooses an iteration count for each case. CSV output contains median,
10th percentile, and 90th percentile runtimes and throughput; the percentiles
describe sample variation, not confidence intervals.

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
It runs all four suites on pull requests, pushes to `main`, and manual
dispatches, recording CPU simulation and memory measurements. The
`parse_codspeed` target shares inputs with `parse` and includes parsing, field
extraction, and output destruction in each measurement. Input construction
happens outside the measured region.

The workflow uses the system allocator and the existing `profiling` Cargo
profile for symbolized results. Enable `astral-sh/astral-html` in the CodSpeed
GitHub app to receive reports. Uploads use GitHub OIDC; no `CODSPEED_TOKEN`
secret is needed.

To build and check the instrumented benchmarks locally:

```console
cargo install cargo-codspeed --version 5.0.1 --locked
cargo codspeed build -m simulation -m memory --profile profiling -p astral-html --bench parse_codspeed --locked
cargo codspeed run
```

`cargo codspeed run` validates the instrumented benchmarks locally; collection
and upload happen inside the CodSpeed action in CI. `ASTRAL_HTML_BENCH_SUITE`
and `ASTRAL_HTML_BENCH_CASE` select inputs for both targets. The custom sampling
and profiling variables apply only to `parse`. For local Criterion timings:

```console
cargo bench -p astral-html --bench parse_codspeed --locked
```

## Profiling

Select one case for a fixed iteration count:

```console
ASTRAL_HTML_BENCH_CASE=root-10000-generated ASTRAL_HTML_BENCH_PROFILE=1 ASTRAL_HTML_BENCH_PROFILE_ITERATIONS=5 cargo bench -p astral-html --bench parse --locked
```

The iteration count defaults to 100,000. Profiling skips warmup and sampling.
Attach a profiler to the compiled executable with `--bench`; `measure` bounds
parsing, extraction, and output destruction.

Running the executable without `--bench`, or with `--test`, parses and extracts
each selected input once without collecting timings. This is also how
`cargo test --all-targets` runs it.
