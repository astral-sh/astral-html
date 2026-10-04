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
| Match an attribute name without ASCII case sensitivity | `Element::attribute` |
| Distinguish absent and boolean attributes | `Element::has_attribute`, `Attribute::raw_value` field |
| Read the first `head` element's metadata | `Element::descendants` |
| Extract a root index anchor's project name | `Element::text` |

Package pages use the first `base` element before an `a` or `link`, the first
`head` element's project-status metadata, and every `a` element with a nonempty
`href`. uv reads `data-requires-python`, `data-core-metadata`,
`data-dist-info-metadata`, `data-yanked`, `data-size`, and `data-upload-time`.
Root index pages also read anchor text. uv performs package-name normalization,
URL resolution, percent decoding, hash validation, and result sorting itself.

## Entity decoding

`Attribute::value` and `Element::text` return decoded text. Remove uv's
`html_escape::decode_html_entities` calls when using these APIs:
decoding `&amp;lt;` twice would incorrectly produce `<` instead of `&lt;`.
The `Attribute::raw_value` field retains the input spelling when a caller needs
it and is `None` for a boolean attribute. An absent attribute is represented by
`Element::attribute` returning `None`.

There are deliberate differences from astral-tl outside the compatibility
fixtures. HTML character references are decoded in the correct text or
attribute context, element and attribute names are matched without ASCII case
sensitivity, and raw-text elements do not expose embedded markup as links.
See the [conformance contract](conformance.md) for recovery behavior.

## Compatibility evidence

`crates/astral-html/tests/uv.rs` compares extracted fields against astral-tl
0.8.0 for all 32 HTML inputs from the upstream file's 31 tests. The fixtures
retain their upstream test names, revision, and licenses. The comparison covers
base URLs, project metadata, link attributes, boolean attributes, and root index
text. Additional assertions check the decoded values uv consumes.

## Pinned uv integration

The [adapter patch](uv-integration.patch) replaces the parser in the pinned uv
revision above. It uses borrowed `Document` and `Element` reads, removes the
second character-reference decoding step, and removes uv-client's dependency on
`html-escape`. All 31 upstream HTML test bodies and their assertions remain
byte-for-byte unchanged.

The [Linux integration workflow](../.github/workflows/uv.yml) checks out that
exact uv revision into `uv-source`, applies the patch, verifies the unchanged
test module, and runs:

```console
cargo test --manifest-path uv-source/Cargo.toml -p uv-client html::tests --lib --locked
```

All **31 original uv-client HTML tests passed**, with zero failures, against
uv revision `46b84fd0bfec23b72f29e8e2185ba68a65052f48` on Ubuntu 24.04 and Rust
1.97.1 in [integration run 37208040489](https://github.com/viarius-experiments/astral-html/actions/runs/37208040489/job/111453271707).
The [recorded result](uv-integration-results.json) includes the tested parser
revision, source hashes, and all 31 passing test names. CI requires the same
31-test success result on each run. The workflow resolves/builds the adapted
dependency graph before testing with `--locked`, and retains the resulting
lockfile, checksum, and test output.

The patch expects `uv-source` immediately below this repository, so its path
dependency resolves to `../crates/astral-html`. The wider uv index and resolver
integration suite remains a separate adoption check.

## Performance

The [benchmark report](performance.md) compares equivalent parsing and field
extraction with astral-tl, including allocations and destruction. It excludes
uv's HTTP, URL validation, packaging, and resolver work.

Allocator selection belongs to the application. uv uses jemalloc on supported
Linux architectures and mimalloc on Windows. The benchmark defaults to the
system allocator; `--features benchmark-jemalloc` selects jemalloc for both parsers.
