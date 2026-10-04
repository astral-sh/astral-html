# Performance

The benchmark compares parsing and uv's HTML field extraction with
`astral-tl` 0.8.0. It verifies equal extracted output before measuring each
input. Both implementations run in the same executable and use the same
allocator. URL handling, package validation, network requests, and resolution
are outside the measured work.

## Current results

With jemalloc, the six uv index workloads measured **1.40–3.23×** the performance
of astral-tl across two campaigns. All 32 pinned uv HTML fixtures also measured
faster in both campaigns, at **1.06–1.47×**. Jemalloc is the primary comparison
because uv uses it on supported Linux architectures. These results cover the
recorded inputs and host; they do not establish a speedup for every HTML input.

The table shows medians from the first jemalloc campaign. A speedup below 1
means astral-html took longer.

| Input | astral-html (µs) | astral-tl (µs) | Speedup |
| --- | ---: | ---: | ---: |
| PyPI iniconfig (captured) | 8.99 | 18.54 | 2.06× |
| CodeArtifact (uv fixture) | 2.24 | 3.15 | 1.41× |
| Flat index (uv fixture) | 1.28 | 1.84 | 1.43× |
| Project index, 1,000 links (generated) | 718.71 | 1,379.31 | 1.92× |
| Project index, 10,000 links (generated) | 7,686.59 | 22,053.60 | 2.87× |
| Root index, 10,000 links (generated) | 2,786.96 | 8,949.94 | 3.21× |
| 64 extra attributes per link (generated) | 689.55 | 869.38 | 1.26× |
| 1 MiB text node (generated) | 19.44 | 15.13 | 0.78× |
| 1,000 links with entities (generated) | 1,057.97 | 1,356.71 | 1.28× |

The generated 1 MiB text case remains slower: **0.78–0.96× with jemalloc and
0.77–0.85× with the system allocator** across the two campaigns. This workload
consists mostly of one long text span, unlike the link and attribute processing
in the index fixtures.

The system allocator measured **1.06–1.94×** across the six index workloads.
Three small uv fixtures remained slower in both campaigns. Three others crossed
parity between campaigns. These are all the fixtures that measured slower in
either run:

| uv fixture | First campaign | Second campaign |
| --- | ---: | ---: |
| `parse_missing_href` | 0.947× | 0.959× |
| `parse_simple_detail_with_project_status_and_emoji_reason` | 0.980× | 0.971× |
| `parse_simple_detail_with_project_status_and_reason` | 0.996× | 1.001× |
| `parse_simple_detail_with_unknown_project_status` | 0.969× | 0.973× |
| `parse_simple_html_case_insensitively_2` | 1.076× | 0.981× |
| `parse_simple_index_empty_href` | 0.980× | 1.018× |

Both repeats are retained separately: jemalloc
[default 1](benchmarks/2026-10-04-optimized-jemalloc-default-1.csv),
[default 2](benchmarks/2026-10-04-optimized-jemalloc-default-2.csv),
[uv 1](benchmarks/2026-10-04-optimized-jemalloc-uv-1.csv), and
[uv 2](benchmarks/2026-10-04-optimized-jemalloc-uv-2.csv); system allocator
[default 1](benchmarks/2026-10-04-optimized-system-default-1.csv),
[default 2](benchmarks/2026-10-04-optimized-system-default-2.csv),
[uv 1](benchmarks/2026-10-04-optimized-system-uv-1.csv), and
[uv 2](benchmarks/2026-10-04-optimized-system-uv-2.csv).
The [environment record](benchmarks/2026-10-04-optimized-environment.json)
identifies the source files, binaries, commands, and run times.

## What changed

Profiles identified repeated attribute allocation and expensive token dispatch.
Documents now keep attributes in flat storage and recycle a tag buffer, reducing
allocation and destruction costs. Attribute exhaustion is checked through
internal budget state, avoiding a nested result in each token return. Unchanged
text and ordinary comments borrow the input; common normalized HTML names borrow
static spellings. An explicit byte tag simplifies token dispatch.

Callgrind recorded five parse/extract/drop iterations of the generated
10,000-link root index with the system allocator:

| Profile measurement | Before | After |
| --- | ---: | ---: |
| Executed instructions | 189.70 million | 151.01 million |
| Document parsing, inclusive instructions | 114.63 million | 89.20 million |
| Document destruction, inclusive instructions | 14.96 million | 2.35 million |
| Attribute-vector growth calls | 50,000 | 10 |
| `free` calls during document destruction | 50,005 | 10 |

The [profile record](benchmarks/2026-10-04-root-profile.json) identifies the
commands, binaries, and [before](benchmarks/2026-10-04-before-root.callgrind.gz)
and [after](benchmarks/2026-10-04-optimized-root.callgrind.gz) profiles. These
counts explain where work was removed; native measurements establish elapsed
time. Parsing and destruction are subsets of the complete workload, and the
allocation counts exclude application output.

## Earlier results

The [hardening campaign](benchmarks/2026-10-04-hardening-jemalloc.csv)
measured 0.90–2.33× across the six index workloads with jemalloc. CodeArtifact
and the flat-index fixture took about 11% longer then; the current runs measure
the changes made to address those costs. Its
[environment and source hashes](benchmarks/2026-10-04-hardening-environment.json)
remain available.

Before hardening, the six workloads measured 1.04–2.43× with jemalloc. The system
allocator regressed on CodeArtifact and the root index, and the 1 MiB text case
was slower with both allocators. The [system results](benchmarks/2026-10-04-system.csv),
[jemalloc results](benchmarks/2026-10-04-jemalloc.csv), and
[source hashes](benchmarks/2026-10-04-environment.json) retain that baseline.
These historical campaigns used different source revisions and are not repeats
of the current measurements.

## Method

The runs used an AMD EPYC-Milan Linux host, pinned to CPU 24, with Rust
1.98.1-dev (`ohm-1.98.1-1`), ThinLTO, one codegen unit, and Ohm's experimental
defaults disabled. Both parsers use the same build settings. The jemalloc
executable uses `tikv-jemallocator` 0.6.1, from the same dependency series uv
uses on supported Linux architectures. The other executable uses the system
allocator.

Each case warms both implementations together to choose a common iteration
count, then takes 31 samples per implementation. Combined warmup is 100 ms for
the default suite and 50 ms for the 32-fixture uv suite. Measurement order
alternates each sample. Two complete campaigns use the same binaries and
settings. The table reports the first campaign's medians; repeat results are
retained separately rather than pooled or selected for the fastest result.

CSV files also retain 10th and 90th percentile sample runtimes. These describe
variation between samples, not confidence intervals. Input creation and output
comparison happen outside the timed loop. Parsing, lookup, entity decoding,
owned field extraction, and destruction happen inside it.

Cases run in fixed order within each suite. The captured iniconfig response and
uv test inputs have pinned provenance and licenses. Larger indexes,
attribute-heavy inputs, the long text node, and entity-heavy input are generated
by the harness; they are not production traffic captures. The 32-fixture suite
uses the same extraction modes as the compatibility tests.

Results apply to the recorded source hashes, host, inputs, and allocator.

## Reproduce

```console
ASTRAL_HTML_BENCH_SAMPLES=31 cargo bench -p astral-html --bench parse --locked
ASTRAL_HTML_BENCH_SAMPLES=31 cargo bench -p astral-html --bench parse --locked --features benchmark-jemalloc
ASTRAL_HTML_BENCH_SUITE=uv ASTRAL_HTML_BENCH_SAMPLES=31 ASTRAL_HTML_BENCH_SAMPLE_MS=50 cargo bench -p astral-html --bench parse --locked --features benchmark-jemalloc
```

`ASTRAL_HTML_BENCH_SUITE=uv` selects the 32 pinned uv fixtures; omitting it or
using `default` selects the nine captured/generated cases.
`ASTRAL_HTML_BENCH_CASE` filters case names by substring. Set
`ASTRAL_HTML_BENCH_SAMPLE_MS` to change the combined warmup duration; its default
is 100 ms. On Linux, use `taskset -c <cpu>` to pin the executable to an available
logical CPU. Repeat each suite with the same binary to compare campaigns.
When using Ohm locally, use `cargo +ohm -Zohm-defaults=no bench` and unset the
procedural-macro and native-tool trust overrides for the measurement build.

For a bounded profiler run, select exactly one case and one implementation:

```console
ASTRAL_HTML_BENCH_CASE=root-10000-generated ASTRAL_HTML_BENCH_PROFILE=astral ASTRAL_HTML_BENCH_PROFILE_ITERATIONS=5 cargo bench -p astral-html --bench parse --locked
```

Use `ASTRAL_HTML_BENCH_PROFILE=baseline` for astral-tl. Profiling bypasses warmup,
sample collection, and output comparison, so run the regular correctness check
first. Attach the profiler to the compiled executable with `--bench`; the
`measure` function bounds parsing, extraction, and output destruction. Profiler
instruction counts and instrumented elapsed times are not native timings.

The executable measures timings only with `--bench`, which `cargo bench` passes
automatically. Running it without that argument, or with `--test`, checks all
outputs and skips timing. Set `ASTRAL_HTML_BENCH_SUITE=uv` to check that suite as
well. CI checks correctness without imposing a wall-clock threshold on shared
runners.
