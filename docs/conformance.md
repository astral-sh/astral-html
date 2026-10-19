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

## Additional upstream coverage

The pinned
[html5ever regression fixtures](../crates/astral-html/tests/fixtures/html5ever/README.md)
add **14 cases / 20 state-expanded runs** to the same token-output harness, with
no exclusions. These are html5ever's custom tests, separate from its vendored
html5lib suite.

[Selected Web Platform Tests](../crates/astral-html/tests/fixtures/wpt/README.md)
add **2,252 tokenizer input cases**: all 2,231 named character references, plus
ambiguous ampersands in text and attributes, newline normalization, doctype
fields, and truncated declarations. The Rust adaptations check tokenizer
behavior; they do not run WPT's browser harness or assert browser tree
construction. Their provenance documents each adaptation and its upstream
source.

[Nine saved parse5 pages](../crates/astral-html/tests/fixtures/parse5/README.md),
ranging from 7,639 to 907,953 bytes, run in full through `Tokenizer`, `Reader`,
and `Document`. Tokenizer and reader output are compared with html5gum 0.8.4;
document element names and decoded attributes are compared with the independent
reader's start tags. An explicit child-graph model built from those tokens also
checks every element's parent and ordered children. Descendant order and text
are checked for all shallow elements, selected deeper levels, and every leaf,
covering every element on eight pages while bounding repeated subtree work on
the largest page. The oracle uses the same HTML-only, scripting-disabled context
as `Reader`. Every page must run, with no expected failures or truncation. The
largest page asserts rejection at the default depth limit, then runs with an
explicit 10,000-level limit; the other pages use the defaults. These fixtures
provide whole-page differential coverage, not browser DOM conformance.

All imported suites are pinned by revision and retain upstream licenses. They
run offline as ordinary Cargo integration tests.

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

`Document::parse_recovering` and `Document::parse_recovering_with_limits` offer
source-order recovery for common optional end tags. A new paragraph or block
start closes an open paragraph; successive list items, description items,
headings, options, option groups, table cells, rows, and table sections close
their corresponding open scopes. A `body` start closes an open `head`.
Nested lists, tables, and selects do not close items in outer containers.
Recovery happens before checking the depth limit for the incoming tag, so pages
with omitted optional end tags can stay within their configured depth. The
lexical `parse` APIs keep their original source-scope behavior.

Recovery never inserts or moves nodes. Browser insertion modes, table foster
parenting and implicit table containers, formatting reconstruction, and foreign
content remain unsupported. In particular, a `td` without a `tr` stays where it
appears in source order, and malformed table text stays inside its lexical
parent. An unmatched end tag is still ignored. This mode is useful for extracting
data from imperfect pages, not for reproducing the browser DOM or sanitizing HTML.

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
