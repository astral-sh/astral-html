# Public HTML corpus for document recovery

This optional evaluation compares `Document::parse` (lexical scopes) with
`Document::parse_recovering` (the browser-like recovery option) on exactly the
same decoded bytes. The pages themselves are intentionally absent from Git.

## Provenance and scope

[`sources.json`](sources.json) lists all URLs. [`manifest.json`](manifest.json)
records the response's effective URL, fetch time, media type, byte count,
SHA-256, and UTF-8 replacement count for each page. The capture on
**2026-10-19 20:47:10–20:47:14 UTC** contains **41 pages / 8,542,490 bytes**:

| Source | Pages | Bytes | Provenance |
| --- | ---: | ---: | --- |
| Live HTML | 24 | 7,047,995 | Astral, GitHub, crates.io, Node.js, Go, OpenAI, ChatGPT, and Buildkite pages; individual URLs and UTC retrieval times in the manifest. Public pages are not redistributable by default. |
| [parse5 source data](https://github.com/inikulin/parse5/tree/e65eae9a9dc27f1b7eb71868d245ca83070ccc4a/test/data) | 9 | 1,488,671 | Fixed `e65eae9a9dc27f1b7eb71868d245ca83070ccc4a`; MIT. These are historical captured pages, including a deeply malformed HTML Standard page. |
| [Web Platform Tests](https://github.com/web-platform-tests/wpt/tree/3d8a779128cf89fff4732953eff7bbac682b3063/html/syntax/parsing) | 8 | 5,824 | Fixed `3d8a779128cf89fff4732953eff7bbac682b3063`; BSD-3-Clause. These small HTML files include intentionally malformed syntax and test harnesses; they are not eight representative production websites. |

The source sites span documentation, package indexes, repository front pages,
product pages, legacy captures, long documents, and conformance fixtures. This
is a purposive sample, not a random measure of the web. The parser receives
UTF-8 text; the harness uses `String::from_utf8_lossy` on the captured HTTP
body. This capture had **zero** replacement characters. No cookies or logged-in
credentials are sent.

## Reproduce

From the repository root, with Python 3, Rust, Cargo, and network access:

```sh
python3 corpus/fetch.py download
python3 corpus/fetch.py verify
python3 corpus/evaluate.py /path/to/unmodified/astral-html baseline
python3 corpus/evaluate.py /path/to/recovery-checkout recovering
```

The downloader writes `.corpus/pages/*.html`, which Git ignores. `download`
checks each SHA-256 against the recorded manifest. Pinned repository pages can
be recovered from their revision; **live pages may have changed**, in which
case `download` reports content drift instead of claiming reproduction of these
exact results. An already populated `.corpus/pages` directory permits exact
offline replay. To measure a new current snapshot, run
`python3 corpus/fetch.py capture`, review the updated manifest, then rerun the
evaluations. The evaluator builds a separate temporary Cargo package under
`.corpus/evaluation` with a path dependency on the checkout supplied on the
command line. It does not edit that checkout. Set `CARGO_HOME` explicitly to
reuse a writable existing Cargo cache; otherwise a private ignored cache is
used. The evaluator writes `.corpus/{baseline,recovering}-results.json`.

The default resource limits are checked first. If parsing rejects a page, the
evaluator retries with a **10,000-element nesting limit** and records the
default error separately. This allows a structural comparison for a baseline
rejection without concealing that rejection. Other resource limits remain at
their defaults. Counts reflect start-tag elements in source order, plus
observable public API metrics: parent names, roots, attributes on links,
paragraph/list nesting, and title text. For identical name sequences, the
evaluator compares each element's direct parent; it does **not** claim an
independent browser DOM oracle or evaluate text equality for every subtree.

## 2026-10-19 comparison

Both modes retained **102,136 elements**, the same element names in source
order, and the same anchor and `href` counts on **all 41 pages**. Recovering
changed direct parentage on **3 pages**, with **9,719 elements** assigned a
different direct parent overall:

| Page | Changed parents | Notable observation |
| --- | ---: | --- |
| Live Node.js download | 4 | Three spans escaped open paragraphs; a div escaped an open noscript element. |
| Pinned parse5 npm capture | 1 | Body became a direct child of html instead of head. |
| Pinned parse5 huge page | 9,714 | Malformed repeated list and paragraph tags no longer nested within the previous `li`/`p`. Directly nested `li` count went **372 → 0**; directly nested `p` count **383 → 0**. |

The other **38 pages** had identical element names and direct parent names.
The lexical parser rejected the huge page at the default depth limit; its
recovering counterpart accepted it at that limit. For its parent comparison,
the lexical parse used the documented extended 10,000-depth limit. The one
large deeply malformed page accounts for virtually all changed parent links,
so the totals are not a web-wide recovery rate. The WPT cases exercise
tokenizer edge cases and did not provoke tree-scope changes in this sample.
Many browser repairs, including implied html/head/body insertion, table foster
parenting, foreign-content namespaces, and formatting-element reconstruction,
remain outside this flat source-order document model. Runtime and memory
performance should be measured separately with warmed binaries, representative
input classes, and the same byte decoding and resource-limit settings.
