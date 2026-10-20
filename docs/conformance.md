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
the largest page. The oracle selects HTML-only text modes with scripting
disabled; these pinned pages do not exercise SVG cases that require different
text modes or closing rules. Focused reader and document tests cover those
cases. Every page must run, with no expected failures or truncation. The largest
page asserts rejection at the default depth limit, then runs with an explicit
10,000-level limit; the other pages use the defaults. These fixtures provide
whole-page differential coverage, not browser DOM conformance.

All imported suites are pinned by revision and retain upstream licenses. They
run offline as ordinary Cargo integration tests.

## Reader and document semantics

`Tokenizer` emits tokens without selecting tree-construction insertion modes.
`Reader` selects text modes for HTML `title`, `textarea`, `style`, `script`,
`xmp`, `iframe`, `noembed`, `noframes`, and `plaintext` elements. It assumes
scripting is disabled, so `noscript` contains ordinary markup. SVG elements stay
in the data state, including SVG `title`, `script`, and `style`. Children of SVG
`foreignObject`, `title`, and `desc` use HTML text modes; nested `svg` elements
return to SVG context. Self-closing SVG tags do not open a context. Matching SVG
boundary end tags close intervening integration points, and HTML breakout tags
resume HTML processing even after an unclosed SVG element. Outer HTML ancestors
are not tracked: in malformed markup such as `<div><svg></div>`, the outer HTML
end tag leaves the reader in SVG context. This can change text handling after
that end tag compared with a browser.

CDATA sections are recognized when the current element is SVG, including
integration points; HTML children of integration points use HTML declaration
rules. This context tracking retains open element names inside SVG, including
HTML descendants, without browser tree repair. MathML and namespace and
attribute adjustments remain unsupported. A caller with that context can select
tokenizer states explicitly. Context storage grows with open elements inside SVG
and any owned normalized names; HTML-only input needs no context allocation.

`Document` retains source-order elements and text. Each start tag opens a scope,
except HTML void elements and self-closing SVG elements. The self-closing flag
on other HTML elements is ignored. HTML breakout changes text handling without
closing lexical document scopes. A matching end tag closes its element and any
open descendants. An unmatched end tag is ignored; remaining scopes close at
EOF. No elements are inserted, moved, or cloned. Comments, doctypes, and
processing instructions remain available in the token APIs but are not retained
in the document.

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
