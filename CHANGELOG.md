# Changelog

## 0.0.1

Released on 2026-10-05.

This is the first release of astral-html, a high-performance HTML parser
designed for document traversal.

### Enhancements

- Adds read-only document and element views with attribute lookup, descendant
  iteration, text extraction, and configurable resource limits
  ([astral-sh/astral-html#8](https://github.com/astral-sh/astral-html/pull/8))
- Adds a streaming event reader that automatically selects the correct text mode
  for HTML elements
  ([astral-sh/astral-html#7](https://github.com/astral-sh/astral-html/pull/7))
- Adds a UTF-8 tokenizer that borrows unchanged input and supports data, RCDATA,
  raw-text, script, plaintext, and CDATA states
  ([astral-sh/astral-html#5](https://github.com/astral-sh/astral-html/pull/5))
- Decodes named and numeric HTML character references according to their text or
  attribute context
  ([astral-sh/astral-html#3](https://github.com/astral-sh/astral-html/pull/3))
- Allows documents with any number of attributes while retaining input, node,
  and nesting limits
  ([astral-sh/astral-html#34](https://github.com/astral-sh/astral-html/pull/34))
- Adds the initial `astral-html` crate without unsafe code or a mandated
  allocator
  ([astral-sh/astral-html#2](https://github.com/astral-sh/astral-html/pull/2))

### Bug fixes

- Treats text-mode closing tags with non-letter names as text, matching HTML
  tokenizer rules
  ([astral-sh/astral-html#10](https://github.com/astral-sh/astral-html/pull/10))

### Performance

- Reduces routine parsing allocations by reusing attribute storage and borrowing
  unchanged comments, decoded strings, and common normalized names
  ([astral-sh/astral-html#16](https://github.com/astral-sh/astral-html/pull/16))
  ([astral-sh/astral-html#21](https://github.com/astral-sh/astral-html/pull/21))
- Makes unknown and unterminated named references substantially faster with
  early attribute-context rejection and a compact trie
  ([astral-sh/astral-html#45](https://github.com/astral-sh/astral-html/pull/45))
  ([astral-sh/astral-html#52](https://github.com/astral-sh/astral-html/pull/52))
- Borrows unchanged punctuation-heavy comments instead of allocating strings for
  them
  ([astral-sh/astral-html#55](https://github.com/astral-sh/astral-html/pull/55))
- Normalizes newlines and NUL characters in script, raw-text, and plaintext
  content in a single pass
  ([astral-sh/astral-html#54](https://github.com/astral-sh/astral-html/pull/54))
- Coalesces runs of literal `<` characters, sharply reducing memory use for
  adversarial text
  ([astral-sh/astral-html#24](https://github.com/astral-sh/astral-html/pull/24))
- Reuses owned buffers when replacing NUL characters, reducing peak memory use
  for large tokens
  ([astral-sh/astral-html#30](https://github.com/astral-sh/astral-html/pull/30))
- Normalizes comment newlines while scanning to avoid a second full-buffer
  allocation
  ([astral-sh/astral-html#28](https://github.com/astral-sh/astral-html/pull/28))
- Releases excessive capacity after character-reference decoding so small
  results do not retain input-sized allocations
  ([astral-sh/astral-html#27](https://github.com/astral-sh/astral-html/pull/27))
- Avoids indexing balanced open elements until malformed nesting requires
  recovery
  ([astral-sh/astral-html#53](https://github.com/astral-sh/astral-html/pull/53))
- Scans comment and script content in bulk
  ([astral-sh/astral-html#36](https://github.com/astral-sh/astral-html/pull/36))
- Delays attribute-name indexing until it is beneficial and reuses its
  allocation between tags
  ([astral-sh/astral-html#38](https://github.com/astral-sh/astral-html/pull/38))
  ([astral-sh/astral-html#46](https://github.com/astral-sh/astral-html/pull/46))
- Speeds named-reference decoding with direct handling for common names and
  narrower searches for uncommon names
  ([astral-sh/astral-html#39](https://github.com/astral-sh/astral-html/pull/39))
  ([astral-sh/astral-html#44](https://github.com/astral-sh/astral-html/pull/44))
- Reuses the tokenizer’s first character-reference offset instead of rescanning
  unchanged text prefixes
  ([astral-sh/astral-html#40](https://github.com/astral-sh/astral-html/pull/40))
- Avoids rescanning tag, attribute, and doctype names already marked for
  normalization
  ([astral-sh/astral-html#56](https://github.com/astral-sh/astral-html/pull/56))
- Speeds element and attribute queries for short normalized names
  ([astral-sh/astral-html#58](https://github.com/astral-sh/astral-html/pull/58))
- Reduces buffered token-dispatch overhead in `Reader` and `Tokenizer`
  ([astral-sh/astral-html#57](https://github.com/astral-sh/astral-html/pull/57))
- Uses compact parent indices to reduce each document node by eight bytes on
  64-bit platforms
  ([astral-sh/astral-html#42](https://github.com/astral-sh/astral-html/pull/42))
- Avoids allocating names when entering built-in script, raw-text, and RCDATA
  modes
  ([astral-sh/astral-html#43](https://github.com/astral-sh/astral-html/pull/43))

### Documentation

- Documents parser semantics, conformance boundaries, resource limits,
  borrowing, and packaged licenses
  ([astral-sh/astral-html#14](https://github.com/astral-sh/astral-html/pull/14))
  ([astral-sh/astral-html#18](https://github.com/astral-sh/astral-html/pull/18))
- Adds introductory examples for the `Document` and `Reader` APIs
  ([astral-sh/astral-html#63](https://github.com/astral-sh/astral-html/pull/63))
- Clarifies that astral-html is a general-purpose HTML parser designed for
  document traversal
  ([astral-sh/astral-html#47](https://github.com/astral-sh/astral-html/pull/47))
  ([astral-sh/astral-html#60](https://github.com/astral-sh/astral-html/pull/60))
- Adds a benchmark chart comparing link extraction with other Rust HTML parsers
  ([astral-sh/astral-html#51](https://github.com/astral-sh/astral-html/pull/51))

### Other changes

- Generates named character-reference data directly from a pinned WHATWG source
  while preserving the generated lookup contents
  ([astral-sh/astral-html#65](https://github.com/astral-sh/astral-html/pull/65))
