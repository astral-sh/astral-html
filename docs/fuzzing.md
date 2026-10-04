# Fuzzing

The fuzz workspace exercises the parser as a library. It has no network access
or filesystem operations in its targets, and it does not execute embedded code.
Inputs are UTF-8 because the public parser accepts `&str`. Byte decoding and
encoding detection belong to the caller.

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
| `document` | Input, node, and depth limits; element traversal, attributes, and descendant text |
| `differential` | Token values against html5gum 0.8.4 in all six supported initial states |

The differential target joins adjacent text tokens, sorts attributes, and ignores
the reference tokenizer's parse-error reports. It compares names, attribute
values, self-closing flags, comments, and doctype identifiers and quirks flags.
It excludes inputs containing `<?`: html5gum 0.8.4 predates WHATWG processing
instructions and emits bogus comments for that syntax. The pinned conformance
tests cover processing instructions separately. This exclusion applies only to
the differential target; the parser, reader, and document targets still exercise
those inputs.

The entity target rejects `<`, `"`, NUL, and CR so that the reference tokenizer
isolates character-reference decoding from markup and input normalization. The
other targets cover these characters. Parser errors are recovered internally;
only document resource-limit failures are exposed as errors.

## Running locally

Install `cargo-fuzz` and a nightly toolchain, then run from the repository root:

```console
cargo install cargo-fuzz --version 0.13.2 --locked
cargo +nightly fuzz run differential -- -dict=fuzz/html.dict -max_len=16384 -max_total_time=300 -timeout=5 -rss_limit_mb=2048 -print_final_stats=1
```

On an Ohm development checkout, use `cargo +ohm` for both commands and retain the
checkout's configured target and shared build directories. Sanitizer builds need
an Ohm toolchain with the corresponding Rust standard-library components.

Keep generated corpus entries separate from the committed seeds:

```console
python3 fuzz/seed_corpus.py differential
cargo +nightly fuzz run differential fuzz/generated/differential fuzz/corpus/differential -- -dict=fuzz/html.dict -max_len=16384 -max_total_time=900 -timeout=5 -rss_limit_mb=2048 -print_final_stats=1
```

`cargo-fuzz` uses AddressSanitizer by default. Run all five targets; agreement
with another tokenizer does not substitute for traversal and resource-limit
testing. Each failure must be minimized, explained, and retained as a regression
test. Do not discard a differential mismatch merely because the other parser
disagrees with the expected result.

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
memory ceiling. The document target also varies much smaller parser limits to
exercise rejection paths. These limits make regressions reproducible; they do
not establish a bound for every possible input accepted by the public API.

The workflow is an ongoing testing mechanism. A smoke run is not evidence of
sustained coverage or production readiness. Record completed campaigns, compiler
versions, total executions, seeds, failures, and remaining exclusions alongside
the conformance and performance results before making release claims.

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
and individual results. The source hashes matched the checkout after completion.

The compiler was Ohm 1.98.1-dev (`f6270311094c`, LLVM 22.1.8) on x86-64 Linux.
Its AddressSanitizer runtime archive was absent, so these local runs used
`--sanitizer none`. Coverage instrumentation, debug assertions, and overflow
checks remained enabled. This is evidence for the tested assertions and oracle
comparisons, not an AddressSanitizer result. Linux CI runs the same targets with
AddressSanitizer on its pinned nightly compiler.
