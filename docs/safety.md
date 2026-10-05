# Safety and resource use

The parser crate forbids unsafe Rust, performs no I/O, and executes no scripts.
It is not an HTML sanitizer. Allocator selection belongs to the application.

Tokenization and document construction are iterative. Flat node storage avoids
recursive parsing, traversal, and destruction. Attribute deduplication and
open-element lookup use bounded linear scans before switching to hash indexes.
Character references use bounded name lookup and saturating numeric
accumulation.

## Limits

`Document::parse` uses these defaults:

| Resource                              |   Default |
| ------------------------------------- | --------: |
| Input                                 |   128 MiB |
| Retained element and text nodes       | 4,000,000 |
| Simultaneously open non-void elements |       256 |

`Document::parse_with_limits` accepts caller-supplied limits. Input size is
checked before tokenization.

Node and depth limits are checked before retaining the next node. The current
token's strings and attributes may already have been allocated. Limits do not
account for vector capacity, decoded strings, application output, or
process-wide allocation failure; standard Rust allocation behavior applies.

`Tokenizer` and `Reader` impose no input or token-size limits. Even without a
document, a single token can contain a large attribute list or comment. Repeated
subtree queries revisit their descendants; prefer one traversal when it
suffices.

## Application bounds

Bound response bytes and decompression before constructing the input string.
Choose parser limits together with response concurrency, and measure peak memory
on representative large indexes. Parser limits cannot bound memory already used
by the HTTP client or retained by the application.

The [conformance tests](conformance.md) and [fuzz targets](fuzzing.md) check
recovery, resource thresholds, and traversal. They do not prove resource bounds
for every input.
