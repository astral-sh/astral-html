# Hardening review

This review started from `73e6d5b37b0897b22dfd6052b44ec0848f869cf5`.
It covered the tokenizer, entity decoder, reader, document storage and queries,
uv adapter, fuzz assertions, and Linux workflows. Reviewers traced allocation,
UTF-8 boundaries, EOF handling, scope indexing, and error paths in the source.
The changes below have focused regression tests and run through the existing
conformance, uv integration, and AddressSanitizer workflows.

## Findings addressed

### Attribute allocation happened before document limits

Node and depth limits apply to completed tokens. They did not bound the
attribute metadata accumulated while reading one token. The input-byte limit
provided a separate ceiling, but callers could not directly limit this work.

`Limits::max_attributes` now bounds attribute occurrences across the document.
The parser checks and spends this budget before normalizing each attribute name
or decoding its value. Duplicate attributes, attributes on end tags, and
attributes in unfinished tags count, even when their token is not retained.
The default is 1,000,000; exceeding it returns `Error::AttributeLimit`.
Small boundary tests cover exact budgets and the recovery paths above.

### Some unchanged strings were copied

The entity decoder allocated at the first ampersand, including when no reference
was consumed. This contradicted its borrowing contract and copied ordinary
query strings unnecessarily. Allocation now begins at the first actual change.
The comment parser also copied its completed buffer when newline normalization
did nothing; it now reuses that buffer.

### Document fuzzing did not verify document structure

The previous target checked counts and called traversal methods, but did not
check parent relationships, child membership, descendant order, or the contents
of text queries. It also ignored every parse error. Passing such a target would
not distinguish several kinds of incorrect document from a correct one.

The target now compares bounded documents with a reference model built from
reader events. The model uses explicit relationships and accumulated text,
independent of the parser's subtree intervals and open-name index. Separate
checks exercise exact resource thresholds. Tokenizer and reader fuzzing also
assert byte-position progress, UTF-8 boundaries, and complete input consumption.

The document model deliberately shares `Reader`: it checks document construction
and queries, not tokenization. The html5lib tests and html5gum differential target
provide the separate token-output checks.

The html5lib harness also checks attribute-name uniqueness before comparing its
map representation, so duplicate names cannot be hidden by map insertion. The
uv workflow retains the adapted lockfile and its checksum, and runs tests with
`--locked` after the adapter's initial dependency resolution and build.

## Remaining work before adoption

- Set response-byte and decompression limits in uv before constructing the input
  string. Parser limits cannot bound memory already used by the HTTP client.
  Choose parser limits together with the number of concurrent responses.
- Measure peak memory as well as timing on representative large package indexes.
  The limits count input bytes, nodes, open elements, and parsed attributes;
  they are not exact allocation accounting. Decoded strings, vector capacity,
  and application output consume additional memory. Allocation failure retains
  Rust's process-level behavior.
- Keep direct `Reader` and `Tokenizer` users behind an input-size bound. Their
  public iterator APIs remain unbounded; the new attribute budget belongs to
  document construction.
- Run uv's broader index and resolver integration tests with the adapter. The
  31 original HTML tests do not cover all HTTP, caching, URL-resolution, and
  dependency-resolution interactions.
- Recover the performance margin on small indexes without removing resource
  checks. The [fresh benchmark run](performance.md) found two small uv fixtures
  about 11% slower than astral-tl after hardening; four other uv workloads remain
  faster. Earlier performance results do not describe this implementation.
- Continue the scheduled ASan campaigns and retain minimized regressions.
  The 16 KiB fuzz ceiling and short pull-request campaigns leave larger inputs
  and sustained coverage as separate validation work. Processing instructions
  still lack an independent differential oracle because html5gum predates them.

Repeated subtree queries revisit their descendants; consumers should avoid
querying every subtree when one traversal would suffice. Tokenizer conformance
also does not make this library an HTML sanitizer or a browser DOM parser.

No additional concrete token-value, termination, UTF-8-boundary, or lifetime
defect was identified by static review. That is a review result, not a proof of
absence. Recorded performance and fuzz campaigns remain evidence for their
listed source hashes; changes require fresh validation.
