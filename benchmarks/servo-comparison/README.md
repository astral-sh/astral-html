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
CARGO_HOME="$HOME/.cache/astral-html-servo-cargo" sh benchmarks/servo-comparison/run.sh \
  /path/to/astral-html-resilient \
  /path/to/astral-html-corpus/.corpus/pages \
  /tmp/astral-html-servo-results
```

Both checkouts must retain their original package name and be built separately. The script runs three structural checks against each implementation, compiles both optimized binaries with the same locked Servo dependencies, then runs three repetitions in alternating order. Results include `environment.txt` with revision and machine information, six raw TSV files, and a textual summary. It builds under Cargo's release profile **only to measure optimized parser speed**; the crate is never published. A writable Cargo home can be needed in managed environments with read-only global caches.

Each input is read once and decoded with UTF-8 replacement before timing. Each parser performs a successful warmup, calibrates to at least 30 ms or 100,000 iterations, then collects five per-iteration elapsed-time samples and reports their median. Each input is timed independently; `summarize.py` reports the median across three process repetitions for each input (six for Servo, which runs in both binaries). Parsing, allocation, and destruction are timed; file reads, UTF-8 decoding, program startup, and extraction of application-specific fields are excluded. Servo's conversion of `&str` into its owned tendril is included in parse time. A resource-limit error is reported as a failure and excluded from the default-limit shared-success aggregate throughput. A second aggregate uses the baseline's raised-depth measurement only where its default parse fails, allowing a matched successful parse comparison for those inputs. The throughput aggregate adds per-page median times on the common success set, then divides total decoded bytes by total time; it does not weight every page equally. The raw per-page timings show performance distributions and regressions hidden by a single aggregate.

This is a same-machine performance comparison, not a conformance test or memory benchmark. CPU scheduling, cache state, allocations, JIT-free compilation, and other concurrent jobs affect local measurements. The source-order document and Servo DOM expose different semantics; query work would need an explicit common extraction task for an application-level performance comparison. The corpus is finite and UTF-8-decoded; script execution, character encoding sniffing, and browser networking are outside scope. The fixture corpus and its original source copyright should be retained separately rather than committed here.
