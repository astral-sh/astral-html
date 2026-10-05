# Performance

The benchmark compares `astral-html` with `astral-tl` using the same allocator
and a shared representation of the HTML fields consumed by uv. It checks equal
output before timing parsing, field extraction, and destruction. Input creation,
URL handling, package validation, network requests, and resolution are excluded.

## Running benchmarks

```console
cargo bench -p astral-html --bench parse --locked
cargo bench -p astral-html --bench parse --locked --features benchmark-jemalloc
ASTRAL_HTML_BENCH_SUITE=uv cargo bench -p astral-html --bench parse --locked --features benchmark-jemalloc
```

The default suite includes captured and generated package indexes, many
attributes, long text, and entities. The `uv` suite covers all 32 pinned HTML
fixtures. The `scanning` suite covers names, comments, scripts, and repeated
small text-mode elements. The `entity-scanning` suite covers uncommon and unknown
references and long text/attribute prefixes.
The optional jemalloc feature matches uv's allocator on supported
Linux architectures; otherwise the benchmark uses the system allocator.

| Variable | Default | Purpose |
| --- | --- | --- |
| `ASTRAL_HTML_BENCH_SUITE` | `default` | Select `default`, `uv`, `scanning`, or `entity-scanning` inputs. |
| `ASTRAL_HTML_BENCH_CASE` | All cases | Filter case names by substring. |
| `ASTRAL_HTML_BENCH_SAMPLES` | `21` | Samples per parser and case; at least 3. |
| `ASTRAL_HTML_BENCH_SAMPLE_MS` | `100` | Combined warmup duration in milliseconds. |

Warmup chooses a common iteration count for both parsers. Measurement order
alternates between samples. CSV output contains median, 10th percentile, and
90th percentile runtimes; the percentiles describe sample variation, not
confidence intervals. Speedup is astral-tl runtime divided by astral-html runtime.

Pin the executable to one available CPU with `taskset -c <cpu>` on Linux and
repeat runs with the same binary. Record the source revision, compiler, CPU,
allocator, and settings alongside results. Cases run in fixed order; generated
inputs are stress tests, not production captures.

The harness does more extraction than uv in some cases: root indexes also
extract base/status fields and decode hrefs, and project indexes read both
metadata attributes instead of short-circuiting the fallback. These operations
run for both parsers; this benchmark does not measure uv-client end to end.

## Profiling

Select one case and parser for a fixed iteration count:

```console
ASTRAL_HTML_BENCH_CASE=root-10000-generated ASTRAL_HTML_BENCH_PROFILE=astral ASTRAL_HTML_BENCH_PROFILE_ITERATIONS=5 cargo bench -p astral-html --bench parse --locked
```

Use `ASTRAL_HTML_BENCH_PROFILE=baseline` for astral-tl. The iteration count
defaults to 100,000. Profiling skips warmup and output comparison, so run the
regular benchmark first. Attach a profiler to the compiled executable with
`--bench`; `measure` bounds parsing, extraction, and output destruction.

Running the executable without `--bench`, or with `--test`, checks output equality
without collecting timings. This is also how `cargo test --all-targets` runs it.
