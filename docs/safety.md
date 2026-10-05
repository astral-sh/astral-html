# Safety and resource use

The parser crate forbids unsafe Rust, performs no I/O, and executes no scripts.
It is not an HTML sanitizer. Allocator selection belongs to the application.

For `<svg><style><img src=x onerror=alert(1)>`, `Reader` and `Document` return
the `img` markup as style text; a browser can create an `img` with an `onerror`
handler. Do not use parser output to approve HTML for rendering. Decoded
attributes remain untrusted: callers must validate URLs and escape values for
the output format.

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
| Attributes encountered in one tag     |     1,024 |
| Total attributes encountered          | 1,000,000 |

`Document::parse_with_limits` accepts caller-supplied limits. Input size is
checked before tokenization.

Both attribute limits count duplicates and attributes on end tags or tags
discarded at EOF. Each is checked before reading the next name or value. Set
either limit to `usize::MAX` to make it nonbinding.

Node and depth limits are checked before retaining the next node. The current
token's strings and permitted attributes may already have been allocated.
Individual names and values can still allocate in proportion to
`max_input_bytes`. Limits do not account for vector capacity, decoded strings,
application output, or process-wide allocation failure; standard Rust allocation
behavior applies.

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
