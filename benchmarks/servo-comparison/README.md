# Servo full-document comparison

This benchmark compares three parse-and-destroy operations:

1. `Document::parse` from the unmodified astral-html revision `b363c5e` in this checkout.
2. `Document::parse_recovering` from a neighboring resilient astral-html checkout.
3. Servo's [`html5ever`](https://github.com/servo/html5ever) 0.39.0 full HTML tree builder with [`markup5ever_rcdom`](https://docs.rs/markup5ever_rcdom/0.39.0+unofficial/markup5ever_rcdom/) as its reference-counted tree sink.
4. On inputs where the default baseline fails, an additional `Document::parse_with_limits` measurement using `max_depth=10,000`. The default failure remains visible; the extra row supports a successful parse speed comparison.

The standalone html5ever parser is developed as part of Servo; Servo's full browser DOM is not used here. The `RcDom` sink is maintained for testing and explicitly disclaims production use. Version 0.39.0 is pinned for compatibility with the available RcDom 0.39.0. The parser constructs browser-style HTML trees, including implied elements, formatting reconstruction, and foster parenting; astral-html intentionally stores source-order nodes and borrowed input slices. Resulting memory use and document models are therefore different.

## Run

Prepare the public corpus with the neighboring `astral-html-corpus` checkout (see `corpus/README.md` there); public pages are stored under its ignored `.corpus/pages` directory. A captured 41-page corpus includes 24 live public sites, nine parse5 pages, and eight WPT snippets. Its `corpus/manifest.json` records the URL, capture time, size, SHA-256, and license or source for each file. The benchmark adds eight deterministic, in-memory cases: a package index, implicit `li`/`p` closure, misnested formatting, table foster parenting, unmatched end tags, entity attributes, script data, and a document exceeding the default nesting limit. Downloaded HTML stays out of Git.

From the baseline checkout:

```sh
BENCHMARK_CPU=3 CARGO_HOME="$HOME/.cache/astral-html-servo-cargo" sh benchmarks/servo-comparison/run.sh \
  /path/to/astral-html-resilient \
  /path/to/astral-html-corpus/.corpus/pages \
  /tmp/astral-html-servo-results
```

Both checkouts must retain their original package name and be built separately. The script runs three structural checks against each implementation, compiles both optimized binaries with the same locked Servo dependencies, then runs three repetitions in alternating order. Results include `environment.txt` with revision and machine information, six raw TSV files, and a textual summary. It builds under Cargo's release profile **only to measure optimized parser speed**; the crate is never published. A writable Cargo home can be needed in managed environments with read-only global caches.
`BENCHMARK_CPU` is optional and uses Linux `taskset` to pin parser runs to one allowed logical CPU; omit it on systems without `taskset`.

Each input is read once and decoded with UTF-8 replacement before timing. Each parser performs a successful warmup, calibrates to at least 30 ms or 100,000 iterations, then collects five per-iteration elapsed-time samples and reports their median. Each input is timed independently; `summarize.py` reports the median across three process repetitions for each input (six for Servo, which runs in both binaries). Parsing, allocation, and destruction are timed; file reads, UTF-8 decoding, program startup, and extraction of application-specific fields are excluded. Servo's conversion of `&str` into its owned tendril is included in parse time. A resource-limit error is reported as a failure and excluded from the default-limit shared-success aggregate throughput. A second aggregate uses the baseline's raised-depth measurement only where its default parse fails, allowing a matched successful parse comparison for those inputs. The throughput aggregate adds per-page median times on the common success set, then divides total decoded bytes by total time; it does not weight every page equally. The raw per-page timings show performance distributions and regressions hidden by a single aggregate.

This is a same-machine performance comparison, not a conformance test or memory benchmark. CPU scheduling, cache state, allocations, JIT-free compilation, and other concurrent jobs affect local measurements. The source-order document and Servo DOM expose different semantics; query work would need an explicit common extraction task for an application-level performance comparison. The corpus is finite and UTF-8-decoded; script execution, character encoding sniffing, and browser networking are outside scope. The fixture corpus and its original source copyright should be retained separately rather than committed here.

## Observed performance, 2026-10-19

On a 16-vCPU AMD EPYC-Milan Linux host with Rust 1.97.1, `BENCHMARK_CPU=3` pinned each optimized binary to the same CPU. The baseline crate remained at `b363c5e`; recovery used commit `59818c0`, which contains the recovery implementation at `7cb6e6e`. Raw TSV files and machine metadata are stored outside Git. The reported speed sums **per-page medians** across three process repetitions (six for Servo), rather than treating one noisy pass as representative.

| Common successful inputs | Decoded bytes | Baseline MiB/s | Recovery MiB/s | Servo/RcDom MiB/s |
| --- | ---: | ---: | ---: | ---: |
| Default limits: 46/49 | 8,031,192 | 458.92 | 430.21 | 52.56 |
| Baseline depth raised only on failures: 48/49 | 8,977,171 | 394.81 | 373.04 | 49.37 |

Recovery took 6.7% more time than the baseline over the 46 common default-limit cases. Servo/RcDom took about 8.7 times as long as the baseline; this is a comparison of different full-document models, and does not imply a similar cost for Servo's tokenizer or an application-specific optimized tree sink. On the captured 907,953-byte deeply malformed parse5 page, the default baseline returned `DepthLimit`; at depth 10,000 it took 4.385 ms, versus 4.736 ms for recovery at its default limit and 24.956 ms for Servo/RcDom. The generated optional-`li`/`p` input likewise exceeded the default baseline limit; with raised depth it took 0.610 ms, versus 0.411 ms for recovery at its default limit. A deliberately 300-level input still exceeds recovery's default 256-depth limit; Servo accepts it.

Among successful common inputs, the largest repeatable recovery costs were generated table markup (1.19 times the baseline latency) and the pinned parse5 WHATWG page (1.16 times). Servo's smaller apparent throughput is partly the cost of the independent `RcDom` representation and owned input, while astral-html borrows source text. The three structural checks confirm that recovery agrees with Servo on implicit paragraph and list-item closure; table foster parenting still differs, and astral-html does not store SVG namespaces.

Single-pass throughput ranged from 366.90 to 464.40 MiB/s for baseline and from 352.78 to 434.95 MiB/s for recovery, despite affinity. Nearly all of this variation came from the 949,601-byte `ruff-rules.html`: one baseline repetition took 7.722 ms versus its 3.505/3.706 ms repeats, and one recovery repetition took 7.735 ms versus 3.690/3.759 ms repeats. Servo's six full-corpus repetitions ranged from 51.69 to 53.49 MiB/s. On the other 45 common cases, recovery took 8.2% more median time. These observations support a small, measurable recovery cost on this machine; single-run aggregate comparisons are unreliable in this environment, and results should be repeated on dedicated hardware before making a strict performance guarantee.
