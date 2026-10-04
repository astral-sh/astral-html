# Conformance

Our conformance bar is token output from the [WHATWG HTML tokenizer](https://html.spec.whatwg.org/multipage/parsing.html#tokenization) for already decoded UTF-8 input. We test data, RCDATA, raw text, script data, plaintext, and CDATA entry states. The caller supplies the last start tag when a fragment needs that context.

The pinned [html5lib corpus](../crates/astral-html/tests/fixtures/html5lib/README.md) runs **7,045 state-expanded cases**. Every supported case must pass; there is no expected-failure list. The harness asserts the exact run and exclusion counts, compares all emitted token values, and coalesces adjacent character tokens as the fixture format requires.

The exclusions are:

- Four inputs contain unpaired UTF-16 surrogates. Rust's `&str` cannot represent them. Numeric references to invalid Unicode scalars are representable inputs and are tested.
- Four cases exercise optional coercion into an XML infoset, which this library does not perform.
- The corpus's parse-error diagnostics and positions are not compared. The tokenizer applies recovery rules but does not expose diagnostics.

This includes malformed tags and attributes, duplicate attributes, comment recovery, document type identifiers and quirks flags, all named and numeric references, script escaping and double escaping, and processing instructions from the October 2026 standard. The independent fuzz oracle predates processing instructions; its narrower comparison is documented in [fuzzing.md](fuzzing.md).

## Reader and document semantics

`Tokenizer` emits tokens without selecting tree-construction insertion modes. `Reader` selects text modes for HTML `title`, `textarea`, `style`, `script`, `xmp`, `iframe`, `noembed`, `noframes`, and `plaintext` elements. It assumes HTML context with scripting disabled. `noscript` therefore contains ordinary markup. It does not select SVG/MathML namespace transitions or foreign-content integration points. A caller with that context can select tokenizer states explicitly.

`Document` retains source-order elements and text. Each start tag opens a scope, except HTML void elements. The self-closing flag on other HTML elements is ignored. A matching end tag closes its element and any open descendants. An unmatched end tag is ignored; remaining scopes close at EOF. No elements are inserted, moved, or cloned. Comments, doctypes, and processing instructions remain available in the token APIs but are not retained in the document.

This is deliberately not a browser DOM. For example, `<p>one<p>two` contains nested source scopes; the browser tree builder would implicitly close the first paragraph. Tables are not repaired, formatting elements are not reconstructed, and absent `html`, `head`, and `body` elements are not inserted. There is no CSS selector engine, script execution, mutation, or HTML serialization.

Names use ASCII case folding. Duplicate attributes keep the first normalized name. Attribute values are decoded exactly once; `raw_value` preserves their input spelling and distinguishes boolean attributes from explicit empty values. Text queries concatenate decoded descendant text without inserting layout whitespace; script and style text remains present.

## Compatibility bar

All **32 pinned uv HTML inputs** must produce the same extracted fields as `astral-tl` 0.8.0. The comparison includes project metadata, base URLs, file attributes, boolean values, and root-index text. Additional tests vary name casing, whitespace, quoting, attribute order, and nesting. [uv.md](uv.md) describes the adapter changes and the upstream integration tests needed before replacing uv's dependency.

Malformed input follows the stated HTML tokenization and lexical-scope rules, rather than preserving every behavior of `astral-tl`. Neither conformance corpus completion nor agreement with another parser establishes browser tree equivalence.
