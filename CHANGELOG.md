# Changelog

## 0.0.3

Released on 2026-10-05.

### Performance

- Speed parsing after tags with many attributes by discarding oversized
  deduplication tables instead of repeatedly clearing them
  ([astral-sh/astral-html#72](https://github.com/astral-sh/astral-html/pull/72))

### Other changes

- Make parser performance easier to compare with benchmark tables covering link
  extraction and document construction across five workloads
  ([astral-sh/astral-html#71](https://github.com/astral-sh/astral-html/pull/71))

## 0.0.2

Released on 2026-10-05.

### Other changes

- Show the full repository README, including examples and benchmarks, on
  crates.io
  ([astral-sh/astral-html#69](https://github.com/astral-sh/astral-html/pull/69))

## 0.0.1

Released on 2026-10-05.

This is the first release of astral-html, a high-performance HTML parser
designed for document traversal.

### Enhancements

- Add read-only document and element views with attribute lookup, descendant
  iteration, text extraction, and configurable resource limits
  ([astral-sh/astral-html#8](https://github.com/astral-sh/astral-html/pull/8))
- Add a streaming event reader that automatically selects the correct text mode
  for HTML elements
  ([astral-sh/astral-html#7](https://github.com/astral-sh/astral-html/pull/7))
- Add a UTF-8 tokenizer that borrows unchanged input and supports data, RCDATA,
  raw-text, script, plaintext, and CDATA states
  ([astral-sh/astral-html#5](https://github.com/astral-sh/astral-html/pull/5))
- Decode named and numeric HTML character references according to their text or
  attribute context
  ([astral-sh/astral-html#3](https://github.com/astral-sh/astral-html/pull/3))
- Allow documents with any number of attributes while retaining input, node, and
  nesting limits
  ([astral-sh/astral-html#34](https://github.com/astral-sh/astral-html/pull/34))
- Add the initial `astral-html` crate without unsafe code or a mandated
  allocator
  ([astral-sh/astral-html#2](https://github.com/astral-sh/astral-html/pull/2))

### Bug fixes

- Treat text-mode closing tags with non-letter names as text, matching HTML
  tokenizer rules
  ([astral-sh/astral-html#10](https://github.com/astral-sh/astral-html/pull/10))

### Performance

- Reduce routine parsing allocations by reusing attribute storage and borrowing
  unchanged comments, decoded strings, and common normalized names
  ([astral-sh/astral-html#16](https://github.com/astral-sh/astral-html/pull/16))
  ([astral-sh/astral-html#21](https://github.com/astral-sh/astral-html/pull/21))
- Make unknown and unterminated named references substantially faster with early
  attribute-context rejection and a compact trie
  ([astral-sh/astral-html#45](https://github.com/astral-sh/astral-html/pull/45))
  ([astral-sh/astral-html#52](https://github.com/astral-sh/astral-html/pull/52))
- Borrow unchanged punctuation-heavy comments instead of allocating strings for
  them
  ([astral-sh/astral-html#55](https://github.com/astral-sh/astral-html/pull/55))
- Normalize newlines and NUL characters in script, raw-text, and plaintext
  content in a single pass
  ([astral-sh/astral-html#54](https://github.com/astral-sh/astral-html/pull/54))
- Coalesce runs of literal `<` characters, sharply reducing memory use for
  adversarial text
  ([astral-sh/astral-html#24](https://github.com/astral-sh/astral-html/pull/24))
- Reuse owned buffers when replacing NUL characters, reducing peak memory use
  for large tokens
  ([astral-sh/astral-html#30](https://github.com/astral-sh/astral-html/pull/30))
- Normalize comment newlines while scanning to avoid a second full-buffer
  allocation
  ([astral-sh/astral-html#28](https://github.com/astral-sh/astral-html/pull/28))
- Release excessive capacity after character-reference decoding so small results
  do not retain input-sized allocations
  ([astral-sh/astral-html#27](https://github.com/astral-sh/astral-html/pull/27))
- Avoid indexing balanced open elements until malformed nesting requires
  recovery
  ([astral-sh/astral-html#53](https://github.com/astral-sh/astral-html/pull/53))
- Scan comment and script content in bulk
  ([astral-sh/astral-html#36](https://github.com/astral-sh/astral-html/pull/36))
- Delay attribute-name indexing until it is beneficial and reuse its allocation
  between tags
  ([astral-sh/astral-html#38](https://github.com/astral-sh/astral-html/pull/38))
  ([astral-sh/astral-html#46](https://github.com/astral-sh/astral-html/pull/46))
- Speed named-reference decoding with direct handling for common names and
  narrower searches for uncommon names
  ([astral-sh/astral-html#39](https://github.com/astral-sh/astral-html/pull/39))
  ([astral-sh/astral-html#44](https://github.com/astral-sh/astral-html/pull/44))
- Reuse the tokenizer’s first character-reference offset instead of rescanning
  unchanged text prefixes
  ([astral-sh/astral-html#40](https://github.com/astral-sh/astral-html/pull/40))
- Avoid rescanning tag, attribute, and doctype names already marked for
  normalization
  ([astral-sh/astral-html#56](https://github.com/astral-sh/astral-html/pull/56))
- Speed element and attribute queries for short normalized names
  ([astral-sh/astral-html#58](https://github.com/astral-sh/astral-html/pull/58))
- Reduce buffered token-dispatch overhead in `Reader` and `Tokenizer`
  ([astral-sh/astral-html#57](https://github.com/astral-sh/astral-html/pull/57))
- Use compact parent indices to reduce each document node by eight bytes on
  64-bit platforms
  ([astral-sh/astral-html#42](https://github.com/astral-sh/astral-html/pull/42))
- Avoid allocating names when entering built-in script, raw-text, and RCDATA
  modes
  ([astral-sh/astral-html#43](https://github.com/astral-sh/astral-html/pull/43))
