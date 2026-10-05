# Using astral-html in uv

The compatibility target is `uv-client` at
[`46b84fd0bfec23b72f29e8e2185ba68a65052f48`](https://github.com/astral-sh/uv/blob/46b84fd0bfec23b72f29e8e2185ba68a65052f48/crates/uv-client/src/html.rs),
which uses astral-tl 0.8.0. Its HTML reads serve package and root indexes,
remote flat indexes, and local `--find-links` files.

## Required reads

| uv read | astral-html API |
| --- | --- |
| Parse a UTF-8 response | `Document::parse` |
| Iterate elements in source order | `Document::elements` |
| Match an HTML element name | `Element::is` |
| Match an attribute name | `Element::attribute` |
| Distinguish absent, boolean, and empty attributes | `Element::attribute`, `Attribute::raw_value` |
| Read the first `head` element's metadata | `Element::descendants` |
| Extract a root index anchor's project name | `Element::text` |

Name matching ignores ASCII case. uv retains responsibility for URL resolution,
percent decoding, package-name normalization, hash validation, and sorting.

`Attribute::value` and `Element::text` return decoded text. Remove uv's
`html_escape::decode_html_entities` calls when using these APIs: decoding
`&amp;lt;` twice would produce `<` instead of `&lt;`. `raw_value` preserves the
input spelling and is `None` for boolean attributes; `Element::attribute`
returns `None` for absent attributes.

Outside the compatibility fixtures, astral-html can differ from astral-tl.
Character references follow their text or attribute context, names ignore
ASCII case, and raw-text elements do not expose embedded markup as links.
See [conformance](conformance.md) for recovery and document semantics.

## Compatibility tests

Compatibility is checked by running uv's 31 upstream HTML tests against the
adapter. The [32 pinned inputs](../crates/astral-html/tests/fixtures/uv/README.md)
from those tests remain available as benchmark fixtures. The crate's own tests
cover generic parser behavior with snapshots and direct assertions.

The [adapter patch](uv-integration.patch) replaces the parser in that uv
revision and removes the second entity-decoding step and uv-client's
`html-escape` dependency. The patch expects a `uv-source` checkout immediately
below this repository so its path dependency resolves to this crate.

The [integration workflow](../.github/workflows/uv.yml) applies the patch,
verifies that the upstream test module is unchanged, resolves and builds the
adapted dependency graph, then runs:

```console
cargo test --manifest-path uv-source/Cargo.toml -p uv-client html::tests --lib --locked
```

CI requires all 31 tests to pass and retains the adapted lockfile, checksum,
and test output. Adoption still requires uv's broader index and resolver tests,
including HTTP, caching, and URL-resolution interactions. Set response-byte and
decompression bounds before parsing and choose [parser limits](safety.md) for
the expected response concurrency.

The [benchmarks](performance.md) measure HTML extraction with the
system allocator or jemalloc. URL validation, packaging, HTTP, and resolver work
are outside their scope. Allocator selection remains the application's choice.
