# Safety and resource use

The parser crate forbids unsafe Rust and scans UTF-8 input with checked slices and `memchr`. It performs no I/O, executes no scripts, and leaves allocator selection to the application.

Tokenization and document construction are iterative. The document stores nodes and subtree endpoints in a flat vector, so nested input does not produce recursive parsing, traversal, or destruction. Tests cover a depth of 10,000, repeated element names across deep-scope indexing, and long sequences of unmatched end tags.

Small attribute lists use bounded duplicate-name scans. Larger lists use the standard library's randomized hash set. Documents similarly index open names after a small fixed depth, so unmatched end tags do not repeatedly scan an unbounded stack. Named references have bounded lookahead; numeric references accumulate with saturation. Caller-supplied text contexts are validated once before scanning.

## Limits

`Document::parse` uses these defaults:

| Resource | Default |
| --- | ---: |
| Input | 128 MiB |
| Retained element and text nodes | 4,000,000 |
| Simultaneously open non-void elements | 256 |
| Parsed attribute occurrences | 1,000,000 |

`Document::parse_with_limits` lets the caller choose different limits. Input size is checked before tokenization. The attribute budget counts every occurrence across the input, including duplicate names, end-tag attributes, and attributes in incomplete tags. Each occurrence is checked before normalizing its name or decoding its value. Markup-like text inside comments and text elements does not consume the attribute budget.

Node and depth limits are checked before retaining the next node. The current token's strings and its attributes within the budget may already have been allocated by that point. These checks are not exact memory accounting or fallible allocation. Standard allocation failure retains Rust's process-level behavior.

`Tokenizer` and `Reader` impose no input or token-size limit. Callers using them on untrusted responses should bound response bytes before parsing. They do not retain a document, but a single token can still contain a large attribute list or comment.

## Evidence

Tests cover tokenizer conformance, uv fields, limit boundaries, and deep traversal and destruction. See the [fuzz targets and campaigns](fuzzing.md), [Linux CI results](ci.md), and [remaining hardening work](hardening.md#remaining-work-before-adoption). These checks do not prove resource bounds for all inputs.
