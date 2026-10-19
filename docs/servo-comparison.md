# Servo comparison

Servo's [`html5ever`](https://github.com/servo/html5ever) provides HTML5 tree
construction through a caller-supplied tree sink. The comparison uses
`html5ever` 0.39.0 with `markup5ever_rcdom` 0.39.0 as that sink. Unlike the
source-order `Document`, this builds browser-style implied elements, repairs
misnested formatting, and moves table content where required. The reference
counted DOM is a test implementation, not a production DOM. Neither parser
executes JavaScript or detects response encodings in this comparison.

The [pinned baseline benchmark branch](https://github.com/astral-sh/astral-html/tree/konsti/servo-comparison/benchmarks/servo-comparison)
contains the standalone Rust benchmark, three structural checks, dependencies
and the reproduction instructions. It compares unmodified `b363c5e`, this
branch's `Document::parse_recovering`, and Servo on the same
[41-page corpus](../corpus/README.md) plus eight generated cases. Reads and
decoding happen before timing; parsing, allocations and destruction are timed.
The baseline's depth-limit failures are recorded, with a separate successful
parse comparison using a raised depth limit. Parser implementations retain
different data structures, so these results measure parse-and-drop workloads,
not interchangeable DOMs or equivalent query behavior. Memory use was not
measured.

## Measured parse speed

The 2026-10-19 local run used a single pinned CPU on an AMD EPYC Milan host,
optimized Rust builds, three alternating process repetitions, and per-input
medians of five timed samples. Throughput divides the sum of decoded input
bytes by the sum of per-page median parse-and-drop times. The raw run output is
generated outside Git; use the benchmark branch's `run.sh` and
`summarize.py` to collect it again.

| Common successful inputs | Unmodified astral-html | Recovery mode | Servo + RcDom |
| --- | ---: | ---: | ---: |
| 46 of 49 cases, default limits (8,031,192 bytes) | 458.92 MiB/s | 430.21 MiB/s | 52.56 MiB/s |
| 48 of 49 cases, baseline depth raised for its failures (8,977,171 bytes) | 394.81 MiB/s | 373.04 MiB/s | 49.37 MiB/s |

Recovery took about **6.7% longer** than the original on common inputs at
default limits. Servo with RcDom took about **8.7 times** as long as the
original. The retained trees and amount of browser repair differ, so this is
evidence about these specific parse-and-drop implementations, not a general
ranking of parser libraries. The deliberately over-depth case still fails in
recovery mode under the configured limit, while Servo parses it. The large
malformed page and an optional-end-tag stress case fail with the baseline's
default depth; recovery accepts both. On the large page, baseline with a
10,000-level limit took 4.385 ms, recovery with the default limit 4.736 ms,
and Servo 24.956 ms.

Recovery cost varies with input: the generated table case took about 19% longer
and the saved WHATWG page about 16% longer than baseline. One page had an
uncontrolled scheduling outlier in an individual process repetition despite
CPU affinity; treating that repetition as typical reverses the aggregate
result. The figures above use per-page medians across repetitions, and the
6–8% cost is supported by the other stable page timings. Repeat the run on the
target workload before setting a production latency budget.

`parse_recovering` closes common optional tags in source order. Servo also
inserts missing `html`, `head`, `body` and table container elements, applies
table foster parenting and formatting reconstruction, and handles foreign
content. This branch intentionally does not implement those transformations.
Its tokenizer/reader choose HTML-only text states, so markup inside SVG or
MathML can still differ from browser output. The recovery mode does not make
HTML safe for rendering.

## Python binding experiment

The separate [Servo-backed Python prototype](https://github.com/konsti-openai/astral-html-python/tree/konsti/servo-parser-prototype)
uses html5ever to build an HTML5 DOM, then converts it into the package's
compact tree for its existing CSS selector API. The original lexical builder
remains selectable in that branch. It disables scripting so `<noscript>`
fallback content remains queryable. Its
[benchmark report](https://github.com/konsti-openai/astral-html-python/blob/konsti/servo-parser-prototype/benchmarks/SERVO_PROTOTYPE.md)
records the exact replay procedure and checks link output against an
independent Lexbor parser.

On the 40 of 41 captured pages accepted by both Python parsers at their
default limits, all **9,425** ordered anchor `(href, text)` pairs matched.
Servo parsed the additional malformed page that exceeds the lexical builder's
depth limit. Across the 40 shared pages, the sum of per-page median parse times
was **17.559 ms** for lexical versus **130.075 ms** for Servo (7.41 times
longer). Parsing plus `a[href]` extraction was **20.920 ms** versus
**134.395 ms** (6.42 times longer). This is a separate Python extension
experiment; the numbers are not comparable directly to the Rust MiB/s table.

The intermediate `RcDom` allocates before the compact tree's node and depth
limits can be checked and doubles the tree representation during conversion.
Its maintainers explicitly describe it as a test DOM. A production HTML5
backend would need an appropriately bounded tree sink and another correctness,
peak-memory and speed evaluation.
