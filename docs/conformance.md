# Conformance

The tokenizer targets the
[WHATWG HTML tokenizer](https://html.spec.whatwg.org/multipage/parsing.html#tokenization)
for decoded UTF-8 input. We test data, RCDATA, raw text, script data, plaintext,
and CDATA entry states. The caller supplies the last start tag when a fragment
needs that context.

The pinned
[html5lib corpus](../crates/astral-html/tests/fixtures/html5lib/README.md) runs
**7,045 state-expanded cases**. Every supported case must pass; there is no
expected-failure list. The harness asserts the exact run and exclusion counts,
compares all emitted token values, and coalesces adjacent character tokens as
the fixture format requires.

The exclusions are:

- Four inputs contain unpaired UTF-16 surrogates. Rust's `&str` cannot represent
  them. Numeric references to invalid Unicode scalars are representable inputs
  and are tested.
- Four cases exercise optional coercion into an XML infoset, which this library
  does not perform.
- The corpus's parse-error diagnostics and positions are not compared. The
  tokenizer applies recovery rules but does not expose diagnostics.

The tested cases cover malformed tags and attributes, duplicate attributes,
comment recovery, document type identifiers and quirks flags, named and numeric
references, script escaping and double escaping, and processing instructions.
The independent fuzz oracle predates processing instructions; its narrower
comparison is documented in [fuzzing.md](fuzzing.md).

## Reader and document semantics

`Tokenizer` emits tokens without selecting tree-construction insertion modes.
`Reader` selects text modes for HTML `title`, `textarea`, `style`, `script`,
`xmp`, `iframe`, `noembed`, `noframes`, and `plaintext` elements. It assumes
HTML context with scripting disabled. `noscript` therefore contains ordinary
markup. It does not select SVG/MathML namespace transitions or foreign-content
integration points. A caller with that context can select tokenizer states
explicitly.

`Document` retains source-order elements and text. Each start tag opens a scope,
except HTML void elements. The self-closing flag on other HTML elements is
ignored. A matching end tag closes its element and any open descendants. An
unmatched end tag is ignored; remaining scopes close at EOF. No elements are
inserted, moved, or cloned. Comments, doctypes, and processing instructions
remain available in the token APIs but are not retained in the document.

For example, `<p>one<p>two` contains nested source scopes; a browser tree
builder would close the first paragraph. Tables are not repaired, formatting
elements are not reconstructed, and absent `html`, `head`, and `body` elements
are not inserted. Encoding detection, CSS selectors, script execution, mutation,
and serialization are out of scope.

Names use ASCII case folding. Duplicate attributes keep the first normalized
name. Attribute values are decoded exactly once; `raw_value` preserves their
input spelling and distinguishes boolean attributes from explicit empty values.
Text queries concatenate decoded descendant text without inserting layout
whitespace; script and style text remains present.

## API coverage

Document and reader tests use Insta inline snapshots of public API results. They
cover lexical scopes, traversal, attributes, decoded text, text modes, and
malformed-input recovery.

Direct assertions also check resource-limit boundaries, borrowing, source
positions, permanent exhaustion, attribute deduplication, and deep traversal.
The tokenizer corpus remains the token-output conformance check.
