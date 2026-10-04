# Hardening review

Review base: `73e6d5b37b0897b22dfd6052b44ec0848f869cf5`.
The review covered allocation, UTF-8 boundaries, EOF handling, scope indexing,
and error paths, plus the uv adapter, fuzz assertions, and Linux workflows.

## Findings addressed

- **Attribute allocation preceded node and depth checks.** Added
  `Limits::max_attributes`, enforced before name normalization or value decoding.
  It counts all parsed occurrences, including duplicates and unfinished tags.
  See [limits](safety.md#limits) for defaults and allocation boundaries.
- **Unchanged strings were copied.** Entity decoding now allocates at the first
  transformation, preserving its borrowing contract for literal ampersands.
  Comments reuse their buffer when newline normalization does nothing.
- **Document fuzzing checked counts but few query results and ignored errors.**
  A separate structural model now checks parents, children, descendants, text,
  and exact resource thresholds. It shares `Reader`; tokenizer output is checked
  separately. See [fuzzing](fuzzing.md) for model bounds and oracle exclusions.
- **Map conversion could hide duplicate attributes in conformance tests.**
  The harness now asserts name uniqueness before conversion.
- **The adapted uv dependency graph was not retained.** CI now saves the lockfile
  and its checksum, then tests with `--locked` after the initial resolution/build.

Tokenizer and reader fuzzing also check byte-position progress, UTF-8 boundaries,
and complete input consumption. No additional concrete token-value, termination,
UTF-8-boundary, or lifetime defect was identified by static review.

## Remaining work before adoption

- Set response-byte and decompression limits in uv before constructing the input
  string. Parser limits cannot bound memory already used by the HTTP client.
  Choose parser limits together with the number of concurrent responses.
- Measure peak memory as well as timing on representative large package indexes.
  Parser limits do not account for vector capacity, decoded strings, application
  output, or process-wide allocation failure.
- Keep direct `Reader` and `Tokenizer` users behind an input-size bound. Their
  public iterator APIs remain unbounded; the new attribute budget belongs to
  document construction.
- Run uv's broader index and resolver integration tests with the adapter. The
  31 original HTML tests do not cover all HTTP, caching, URL-resolution, and
  dependency-resolution interactions.
- Track the remaining system-allocator and long-text costs. The current
  [benchmarks](performance.md) are faster across all uv fixtures with jemalloc,
  but a few small fixtures remain slower with the system allocator and the
  generated 1 MiB text case is slower with both allocators.
- Continue the scheduled ASan campaigns and retain minimized regressions.
  The 16 KiB fuzz ceiling and short pull-request campaigns leave larger inputs
  and sustained coverage as separate validation work. Processing instructions
  still lack an independent differential oracle because html5gum predates them.

Repeated subtree queries revisit their descendants; consumers should avoid
querying every subtree when one traversal would suffice. Tokenizer conformance
also does not make this library an HTML sanitizer or a browser DOM parser.

Recorded campaigns and benchmarks apply to their listed source hashes.
