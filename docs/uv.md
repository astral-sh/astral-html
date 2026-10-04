# Using astral-html in uv

The compatibility target is `uv-client` at
[`46b84fd0bfec23b72f29e8e2185ba68a65052f48`](https://github.com/astral-sh/uv/blob/46b84fd0bfec23b72f29e8e2185ba68a65052f48/crates/uv-client/src/html.rs).
That revision depends on `astral-tl` 0.8.0. All parser calls are in
`crates/uv-client/src/html.rs`; the parsed fields feed package index pages,
root index pages, remote flat indexes, and local `--find-links` HTML files.

## Required reads

| uv read | astral-html API |
| --- | --- |
| Parse a UTF-8 response | `Document::parse` |
| Iterate elements in source order | `Document::elements` |
| Match an HTML element name | `Element::is` |
| Find an attribute without case-sensitive spelling assumptions | `Element::attribute` |
| Distinguish absent and boolean attributes | `Element::has_attribute`, `Attribute::raw_value` field |
| Read the first `head` element's metadata | `Element::descendants` |
| Extract a root index anchor's project name | `Element::text` |

Package pages use the first `base` element before an `a` or `link`, the first
`head` element's project-status metadata, and every `a` element with a nonempty
`href`. uv reads `data-requires-python`, `data-core-metadata`,
`data-dist-info-metadata`, `data-yanked`, `data-size`, and `data-upload-time`.
Root index pages also read anchor text. uv performs package-name normalization,
URL resolution, percent decoding, hash validation, and result sorting itself.

No mutation, selector engine, ID index, class index, or HTML serialization is
needed for these call sites.

## Entity decoding

`Attribute::value` and `Element::text` return decoded HTML text. The existing uv
adapter explicitly calls `html_escape::decode_html_entities` after reading most
attributes from astral-tl. Remove those calls when using the decoded API:
decoding `&amp;lt;` twice would incorrectly produce `<` instead of `&lt;`.
`Attribute::raw_value` field retains the input spelling when a caller needs it, and
returns `None` for a boolean attribute. An absent attribute is represented by
`Element::attribute` returning `None`.

There are deliberate differences from astral-tl outside the compatibility
fixtures. HTML character references are decoded in the correct text or
attribute context, element and attribute names are matched without ASCII case
sensitivity, and raw-text elements do not expose embedded markup as links.
The parser's conformance documentation defines the supported HTML behavior;
astral-tl's behavior on malformed input is not the specification.

## Compatibility evidence

`crates/astral-html/tests/uv.rs` compares extracted fields against astral-tl
0.8.0 for all 32 HTML inputs from the upstream file's 31 tests. The fixtures
retain their upstream test names, revision, and licenses. The comparison covers
base URLs, project metadata, link attributes, boolean attributes, and root index
text. Additional assertions check the decoded values uv consumes.

This validates the parser boundary. It does not run uv's resolver, HTTP client,
URL validation, or packaging logic, and does not constitute an upstream uv
integration test. Before replacing the dependency in uv, update the adapter and
run uv-client's tests and uv's index integration tests.

## Performance

`cargo bench -p astral-html --bench parse` compares parse plus uv field extraction
with astral-tl 0.8.0. Both implementations use the same process, compiler,
profile, allocator, inputs, and owned output type. Every input's complete output
is checked for equality before measuring. Timings include parsing, field lookup,
entity decoding where uv needs it, output allocation, and destruction.

The harness alternates measurement order, warms both implementations, and
reports the median and 10th/90th percentile sample timings. It includes a
captured PyPI response, uv's CodeArtifact and flat-index fixtures, and generated
indexes with 1,000 or 10,000 links, 64 extra attributes per link, a 1 MiB text
node, and frequent character references. Generated results must not be described as
captured production traffic. Set `ASTRAL_HTML_BENCH_SAMPLES` and
`ASTRAL_HTML_BENCH_SAMPLE_MS` to control sampling.

The library does not install a global allocator. The default benchmark uses the
system allocator for both implementations. Add `--features benchmark-jemalloc`
to run both implementations with jemalloc in the benchmark executable.
uv installs jemalloc on its supported
Linux architectures and mimalloc on Windows in a separate application crate;
allocator selection should remain with the embedding application. Results from
this harness do not establish performance for uv's full network and resolver workload.
