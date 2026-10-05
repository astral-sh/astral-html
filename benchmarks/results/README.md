# Benchmark results

[epyc-vm-58a28da.json](epyc-vm-58a28da.json) preserves the completed run at
[revision 58a28da](https://github.com/viarius-experiments/astral-html/tree/58a28da8420cc71f1f829372769ae4d1d036e85e/benchmarks).
It contains the original run metadata and eligibility matrix, plus all 135
Criterion measurements: benchmark identifiers, statistical estimates, and raw
iteration counts and times. Times in the source data are nanoseconds. The run
started from a clean checkout and recorded no source changes.

The README chart selects the PEP index because it is the largest of the five
checked-in fixtures: 418,744 bytes and 2,099 extracted links. All six adapters
produced the same link records. The chart shows Reader for astral-html alongside
the four other crates; Document remains in the detailed results. Each bar is the
median of three independent session means. Individual session estimates and
confidence intervals are preserved in the source data. The complete corpus is
summarized below to make the selection explicit.

## Link extraction

Median of three session means, in **microseconds**; lower is better. Each session
collected 100 samples per measurement, with a one-second warmup and a three-second
measurement target. Sessions used different recorded order seeds. These values
summarize sessions without pooling their samples.

| Fixture | astral-html Reader | astral-html Document | tl | html5gum | lol_html | scraper |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| iniconfig | 5.70 | 6.60 | 14.75 | 14.99 | 17.08 | 66.81 |
| Bootstrap dashboard | 31.20 | 35.83 | 52.31 | 95.35 | 65.32 | 399.16 |
| Rust Book | 146.30 | 165.77 | 219.28 | 394.40 | 284.96 | 1,589.29 |
| PEP 8 | 301.01 | 366.70 | 454.03 | 870.56 | 527.90 | 3,193.92 |
| PEP index | 1,116.75 | 1,297.77 | 1,766.79 | 2,914.33 | 2,857.74 | 11,346.75 |

The workload parses resident UTF-8 input, returns owned `href`, `title`, and `rel`
records, and drops parser state and output. Input loading and correctness checks
are outside timing. See the [methodology](../README.md) for adapter semantics and
the separate native-document workload, whose representations differ.

## Environment and provenance

These measurements came from a shared KVM VM on an AMD EPYC-Milan processor,
running Linux 6.8.0 with glibc 2.39. Measurement affinity was pinned to guest
vCPU 0. Physical-core exclusivity, host scheduling, CPU frequency/power policy,
and neighboring workloads were not controlled. Inputs were repeatedly measured
from memory. Five fixtures on this machine do not establish performance across
all websites or hardware; compare the independent sessions and their raw samples
when assessing variability.

The comparison uses upstream `tl` 0.7.8 with default features; its optional
nightly-only `simd` feature is disabled. The complete link records match across
all five documents for every implementation. Focused tests retain native
semantic differences outside this corpus.

The compiler was `rustc 1.98.1-dev (f62703110 2026-09-08)` from
`ohm-1.98.1-1`, with Ohm's experimental Cargo defaults disabled. The release
profile used thin LTO and one codegen unit, with the system allocator. Exact
compiler information, commands, settings, environment flags, hashes, and shared-VM
disclosures are retained in the JSON metadata.

The pinned [fixture manifest](https://github.com/viarius-experiments/astral-html/blob/58a28da8420cc71f1f829372769ae4d1d036e85e/benchmarks/fixtures/manifest.json)
records source URLs and revisions, byte counts, SHA-256 hashes, and licenses.
The pinned [Cargo.lock](https://github.com/viarius-experiments/astral-html/blob/58a28da8420cc71f1f829372769ae4d1d036e85e/benchmarks/Cargo.lock)
records the dependency graph. The PEP index is from
[python/peps revision 73849c8](https://github.com/python/peps/blob/73849c82abc9b9061664b2e2a064019f80c496db/pep-0000/index.html).
