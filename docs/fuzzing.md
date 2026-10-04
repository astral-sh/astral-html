# Fuzzing

Five targets accept UTF-8 input, matching the library's `&str` APIs:

| Target | Checks |
| --- | --- |
| `tokenize` | Progress and permanent EOF in all six states, including arbitrary fragment tag contexts |
| `entities` | Character references in text and quoted attributes against html5gum 0.8.4 |
| `reader` | Automatic text-mode transitions, progress, and permanent EOF |
| `document` | Resource thresholds, parents, children, descendants, and text against a separate tree model |
| `differential` | Token values against html5gum 0.8.4 in all six initial states |

`fuzz/seed_corpus.py` seeds every target from all 14 pinned html5lib tokenizer files, the uv fixtures, and handwritten seeds. It skips unpaired surrogates and inputs over 16 KiB, deduplicates by SHA-256, and records input hashes and counts. XML-coercion inputs also seed fuzzing, though their expected output is outside the conformance target. Generated inputs stay outside version control.

## Oracle scope

The differential target joins adjacent text tokens, sorts attributes, and ignores parse-error reports. It compares names, attribute values, self-closing flags, comments, and doctype identifiers and quirks flags. Inputs containing `<?` are excluded because html5gum 0.8.4 predates WHATWG processing instructions. The pinned conformance corpus and other fuzz targets cover that syntax, but it has no independent differential oracle.

The entity target excludes `<`, `"`, NUL, and CR to isolate reference decoding from markup and input normalization. Other targets exercise those characters.

The document model uses explicit child links and shares `Reader`, so it checks document structure rather than tokenizer correctness. It compares all tags and exact input, node, and depth thresholds for inputs up to 4 KiB, and varies the attribute budget. Repeated ancestor, subtree, and text comparisons are limited to 128 elements. Larger inputs up to 16 KiB still exercise parsing and bounded traversal and text queries.

## Run locally

Install `cargo-fuzz` and a nightly toolchain, then run from the repository root:

```console
cargo install cargo-fuzz --version 0.13.2 --locked
python3 fuzz/seed_corpus.py differential
cargo +nightly fuzz run differential fuzz/generated/differential fuzz/corpus/differential -- -dict=fuzz/html.dict -max_len=16384 -len_control=0 -max_total_time=900 -timeout=5 -rss_limit_mb=2048 -print_final_stats=1
```

`cargo-fuzz` enables AddressSanitizer by default. Repeat for each target. Reproduce and minimize a saved failure with:

```console
cargo +nightly fuzz run differential fuzz/artifacts/differential/crash-HASH
cargo +nightly fuzz tmin differential fuzz/artifacts/differential/crash-HASH
```

Retain minimized failures as regression tests.

## CI

The [Linux workflow](../.github/workflows/fuzz.yml) runs each target for 30 seconds on pull requests and pushes, or 15 minutes on scheduled and manual runs. It pins the compiler and `cargo-fuzz`, uses the committed fuzz lockfile, and imports the fixture corpus before each campaign. Successful default-branch runs cache generated inputs for later campaigns.

Inputs are limited to 16 KiB, with a five-second timeout and 2 GiB resident-memory ceiling. Every job retains compiler and revision metadata, input hashes, logs, generated inputs, and failure artifacts for 30 days. Short campaigns do not establish sustained coverage; larger inputs need separate validation.
