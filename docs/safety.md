# Safety and resource use

The parser crate forbids unsafe Rust. Input scanning uses checked slices and the `memchr` crate; unchanged strings borrow from the caller's UTF-8 input. The library does not select a global allocator, access the network, open files, execute scripts, or load external resources.

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

Validation includes the complete supported tokenizer corpus, uv field comparisons, document-limit boundary tests, deep traversal and destruction, and independent differential fuzzing. [fuzzing.md](fuzzing.md) records the targets, exclusions, campaign limits, and completed runs. Linux CI runs tests, documentation, formatting, Clippy, packaging, and AddressSanitizer fuzz smoke tests; scheduled runs extend the fuzz campaigns after the workflow reaches the default branch.

Safe Rust and successful campaigns reduce specific risks. They do not prove termination or resource bounds for every possible input, establish semantic equivalence to a browser, or replace continued fuzzing and upstream uv integration testing.
