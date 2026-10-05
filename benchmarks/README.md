# HTML benchmarks

A comparison workspace inspired by [string-rosetta-rs](https://github.com/rosetta-rs/string-rosetta-rs), formerly epage's string-benchmarks-rs. It uses common inputs and Criterion reports, with separate results for each operation and document. Competitor dependencies and their lockfile live here, outside the parser crate's workspace.

## Workloads

| Workload | Timed work | Implementations |
| --- | --- | --- |
| `extract-links` | Parse the input and return owned link records, then drop parser state and output. | astral-html Reader and Document, html5gum, tl, scraper, lol_html |
| `parse-document` | Construct and drop each parser's native retained document. | astral-html Document, tl, scraper |

A link record contains decoded `href`, `title`, and `rel` attributes for every `a` element with an `href`, in document order. Empty URLs are included. Missing optional attributes are `None`; boolean and explicitly empty attributes are `Some("")`. Values are decoded once. The task does not resolve URLs, parse `rel` tokens, collapse whitespace, or extract link text.

Input loading, SHA-256 verification, selector compilation, and correctness checks happen outside the timed iterations. Parser construction, required decoding, output allocation, and destruction happen inside. Every implementation uses the system allocator. The release profile uses thin LTO and one codegen unit, without native-CPU compiler flags by default.

Each iteration parses the same in-memory input after warmup. These are warm-input throughput measurements; they do not measure network or disk latency, cold starts, peak memory, or browser rendering.

Native-document results measure different representations and semantics. In particular, astral-html retains lexical scopes while scraper performs HTML tree construction. A matching link extraction result does not imply identical trees or full HTML conformance. Tokenizers and streaming rewriters are excluded from this workload because they do not retain a document.

## Implementations

Versions are pinned in [Cargo.toml](Cargo.toml), with the complete dependency graph in [Cargo.lock](Cargo.lock).

| Implementation | Adapter and relevant behavior |
| --- | --- |
| astral-html Reader | Source-order events; disables scripting for text-state selection. |
| astral-html Document | Retains lexical element scopes and borrows unchanged input. |
| html5gum 0.8.4 | Selective callback emitter drops unused text/comments, with scanning acceleration enabled; selects the same HTML text states as Reader. |
| tl 0.7.8 | Upstream lightweight retained document with default features; raw attribute values require decoding. |
| scraper 0.27.0 / html5ever 0.39.0 | Browser-style tree construction with scripting disabled; precompiled `a[href]` selector. |
| lol_html 3.0.1 | Streaming `a[href]` handler with an empty serialization sink; retains the library's native scripting policy. |

The upstream tl crate has no default features. Its optional `simd` feature requires nightly Rust and is disabled here.

The tl and lol_html adapters decode raw attributes using HTML character-reference tables from html5ever. This required conversion is timed. They do not use astral-html's decoder. Focused tests have independent expected records for attribute presence, references, duplicate attributes, and text contexts. Known native semantic differences are represented in those tests, including upstream tl's case-sensitive attribute names, boolean-attribute handling, and text states. These differences do not occur in the measured corpus; all five pages produce identical records.

Before timing a corpus pair, the suite compares its complete owned records against scraper's output. Any mismatch remains in `eligibility.json` and the report with an explanation, and that pair is not timed. The same gate applies to native-document results. Inspect the eligibility matrix alongside timings; a skipped result is not a fast result. The reference is a practical extraction oracle, not a proof of parser conformance.

## Corpus

The five [pinned documents](fixtures/README.md) cover a package index, a Rust Book chapter, a long article, a table index, and a dashboard example. They range from 6 KiB to 409 KiB. The fixture manifest records immutable source links, exact byte sizes, SHA-256 hashes, categories, and license notices.

All inputs are available offline. Referenced scripts and stylesheets are not loaded or executed. These pages cover different structures but are not a representative sample of all websites. The dashboard is an upstream example with sample data.

## Run

From the repository root, with Python 3.11+ and the pinned Rust toolchain installed:

```console
cargo test --manifest-path benchmarks/Cargo.toml --locked
python3 -m unittest discover -s benchmarks/scripts -p 'test_*.py'
python3 benchmarks/scripts/run.py --output benchmarks/runs/my-machine --cpu 0
```

Choose a CPU in the machine's allowed affinity set; omit `--cpu` on platforms without affinity support. Fixture loading and compilation happen before measurement affinity is applied.

The default run starts three independent Criterion processes, each with 100 samples, a one-second warmup and three-second measurement target per pair. Each process uses a different recorded seed to vary fixture, operation, and implementation order. Criterion may extend measurement time to collect enough samples.

For a short validation run:

```console
python3 benchmarks/scripts/run.py --output benchmarks/runs/smoke --smoke --runs 1
```

Smoke reports are labeled as unsuitable for performance claims. `--filter extract-links` measures just the extraction workload. `--seed` controls ordering, and `--toolchain` selects another installed compiler. Local Ohm runs can use `--toolchain ohm --no-ohm-defaults` to disable experimental Cargo defaults; the actual compiler and flags are recorded.

Use `--notes` to record environmental conditions, such as shared virtualization, power settings, or background load. The notes appear above the results alongside compiler, CPU, operating system, and measurement affinity.

The runner refuses to overwrite an existing output directory. It keeps:

- `metadata.json`: source revision and hashes, dirty state, compiler, CPU, affinity, flags, commands, settings, and run status.
- `eligibility.json`: every supported workload/fixture/parser pair, including mismatches.
- `criterion/session-*/`: full Criterion HTML/SVG reports, estimates, and raw samples.
- `report.md` and `index.html`: per-session tables with mean, 95% confidence interval, median, and throughput.
- The lockfile and logs for validation, compilation, and each session.

## Publishing results

Publish the complete run directory so chart and detail links work and readers can inspect raw data. The generated `index.html` is the entry point for a static host; no deployment happens automatically. Include this workspace and its licensed corpus at the recorded revision so others can rerun it.

Run on a quiet machine with fixed settings and disclose virtualization, power policy, and any remaining environmental limits. Preserve all independent sessions. Confidence intervals describe uncertainty within a Criterion session; they do not account for machine-to-machine differences or turn shared-host measurements into controlled hardware measurements.

Results stay separate by operation, document, and session. The report does not pool samples or calculate an overall winner. Review variation across sessions and avoid comparing timings across different eligible corpora. Report dirty-source runs and any source changes during measurement; the report flags those as unsuitable for publication.

The repository README shows extraction from the PEP index, the largest fixture. Its [source measurements](results/README.md) include all five documents, all sessions, and the raw Criterion samples. Regenerate the light and dark SVGs with:

```console
python3 benchmarks/scripts/plot.py benchmarks/results/epyc-vm-58a28da.json
```

The plot uses the median of session means and shows Reader for astral-html; Document remains in the detailed results. The generator uses Python's standard library to update [Ruff's original README SVG](scripts/templates/README.md), preserving its layout and light/dark styling.

CI runs a smoke test to validate adapters, corpus integrity, report generation, and benchmark execution. Its timings are not release performance results.
