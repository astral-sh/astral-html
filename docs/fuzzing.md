# Fuzzing

Targets accept UTF-8 input, matching the library's `&str` APIs.

`fuzz/seed_corpus.py` imports every UTF-8 scalar input from all 14 pinned html5lib
tokenizer files, the pinned uv HTML fixtures, and the handwritten seeds. It skips
the four inputs containing surrogate code points and inputs larger than 16 KiB,
deduplicates by SHA-256, and records fixture hashes and counts. XML-coercion
expectations are not a conformance goal, but their input strings still seed fuzzing.
Every target receives the complete corpus, including those inputs its oracle
intentionally filters out. Generated cases remain outside version control.

| Target | Contract exercised |
| --- | --- |
| `tokenize` | Progress and permanent EOF in all six states, including arbitrary fragment tag contexts |
| `entities` | Character references in text and quoted attributes, compared with html5gum 0.8.4 |
| `reader` | Automatic raw-text state transitions, progress, and permanent EOF |
| `document` | Exact input, node, and depth limits; attribute budgets; parent/child relationships, descendants, and text against a separate model |
| `differential` | Token values against html5gum 0.8.4 in all six supported initial states |

The differential target joins adjacent text tokens, sorts attributes, and ignores
the reference tokenizer's parse-error reports. It compares names, attribute
values, self-closing flags, comments, and doctype identifiers and quirks flags.
It excludes inputs containing `<?`: html5gum 0.8.4 predates WHATWG processing
instructions and emits bogus comments for that syntax. The pinned conformance
tests cover processing instructions separately. This exclusion applies only to
the differential target; the parser, reader, and document targets still exercise
those inputs.

The entity target rejects `<`, `"`, NUL, and CR to isolate reference decoding
from markup and input normalization. The other targets cover these characters.

## Running locally

Install `cargo-fuzz` and a nightly toolchain, then seed and run a target from
the repository root:

```console
cargo install cargo-fuzz --version 0.13.2 --locked
python3 fuzz/seed_corpus.py differential
cargo +nightly fuzz run differential fuzz/generated/differential fuzz/corpus/differential -- -dict=fuzz/html.dict -max_len=16384 -max_total_time=900 -timeout=5 -rss_limit_mb=2048 -print_final_stats=1
```

On an Ohm development checkout, use `cargo +ohm` for Cargo commands and retain the
checkout's configured target and shared build directories. Sanitizer builds need
an Ohm toolchain with the corresponding Rust standard-library components.

`cargo-fuzz` uses AddressSanitizer by default. Run all five targets, keeping
generated inputs separate from committed seeds. Investigate differential
mismatches and retain minimized failures as regression tests.

To reproduce a saved failure and minimize it:

```console
cargo +nightly fuzz run differential fuzz/artifacts/differential/crash-HASH
cargo +nightly fuzz tmin differential fuzz/artifacts/differential/crash-HASH
```

## Continuous runs and evidence

Linux pull requests run each target for 30 seconds. Scheduled and manually
dispatched runs use 15 minutes per target. The workflow pins `cargo-fuzz`, the
nightly compiler, and the independent tokenizer. The fuzz lockfile is committed.
CI imports the complete pinned fixture corpus before each campaign.
Successful default-branch runs retain the generated corpus for subsequent runs.
Every job uploads compiler and revision metadata, corpus hashes, logs, generated
inputs, and failure artifacts for 30 days, including when the campaign fails.

Each input is limited to 16 KiB, with a five-second timeout and a 2 GiB resident
memory ceiling. The document target checks inputs up to 4 KiB with an independent
model of reader events. It compares every tag and exact resource thresholds;
repeated subtree and ancestor comparisons are limited to documents with at most
128 elements. Larger inputs up to 16 KiB still exercise parsing and a bounded
set of traversal and text queries. The model shares the event reader;
tokenization is checked by the conformance corpus and differential target.
The document target also varies the total parsed-attribute budget.

Smoke runs do not establish sustained coverage or resource bounds beyond the
tested inputs. Campaign records identify the tested compiler, source, seeds,
limits, and results.

## Recorded local campaign

On 2026-10-04, all five targets completed 181 seconds each, starting from a fresh
corpus of 6,747 distinct inputs per target. There were no crashes, timeouts, or
differential mismatches. The targets ran concurrently with seed `1`, a 16 KiB
input ceiling, and input-length growth enabled immediately (`-len_control=0`).

| Target | Executions | Peak resident memory | Raw log |
| --- | ---: | ---: | --- |
| `tokenize` | 966,585 | 38 MiB | [log](../fuzz/evidence/2026-10-04/tokenize.log.gz) |
| `entities` | 1,029,927 | 35 MiB | [log](../fuzz/evidence/2026-10-04/entities.log.gz) |
| `reader` | 3,221,627 | 39 MiB | [log](../fuzz/evidence/2026-10-04/reader.log.gz) |
| `document` | 2,634,860 | 41 MiB | [log](../fuzz/evidence/2026-10-04/document.log.gz) |
| `differential` | 412,456 | 46 MiB | [log](../fuzz/evidence/2026-10-04/differential.log.gz) |

These are libFuzzer execution counts, including inputs rejected for invalid UTF-8
or filtered by an oracle. The [metadata](../fuzz/evidence/2026-10-04/metadata.json)
records exact commands, source and binary hashes, fixture provenance, the compiler,
and individual results.

The compiler was Ohm 1.98.1-dev (`f6270311094c`, LLVM 22.1.8) on x86-64 Linux.
Its AddressSanitizer runtime archive was absent, so these local runs used
`--sanitizer none`. Coverage instrumentation, debug assertions, and overflow
checks remained enabled. See [CI results](ci.md) for AddressSanitizer runs.
