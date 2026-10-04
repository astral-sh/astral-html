# astral-html

A read-only HTML parser for package indexes, written in Rust.

astral-html provides a borrowed event reader and an immutable document view for the HTML reads used by uv. The parser handles HTML tokenization and recovery, character references, case-insensitive names, raw-text elements, and read-only element traversal. It does not build a browser DOM or provide mutation and serialization APIs.

```rust
use astral_html::Document;

let source = r#"<a href="demo-1.0.tar.gz" data-requires-python="&gt;=3.12">demo</a>"#;
let document = Document::parse(source)?;

for element in document.elements().filter(|element| element.is("a")) {
    if let Some(href) = element.attribute("href") {
        println!("{}: {}", element.text(), href.value());
    }
}
```

`Reader` emits events without retaining a document. `Tokenizer` accepts an explicit text mode for callers that supply fragment context. `Document::parse_with_limits` configures input size, node count, and nesting limits. Unchanged strings borrow from the input, and the embedding application chooses the allocator.

## Validation

- **Conformance:** all 7,045 supported html5lib tokenizer runs pass, with exact counts for the eight excluded non-scalar and XML-coercion cases. See the [conformance bar](docs/conformance.md).
- **Compatibility:** all 32 pinned uv HTML inputs match `astral-tl` 0.8.0's extracted fields. See the [uv migration notes](docs/uv.md).
- **Fuzzing:** five targets exercise tokenizer states, entity decoding, the event reader, document traversal and limits, and an independent tokenizer oracle. Campaigns begin with the pinned conformance and uv inputs. See [fuzzing](docs/fuzzing.md) and [safety and resource use](docs/safety.md).
- **Performance:** the [benchmark report](docs/performance.md) compares equivalent parse-and-extract work with `astral-tl`, using both the system allocator and uv's Linux allocator. It includes raw results, source hashes, and the cases that remain slower.

The library is pre-release. The parser boundary is tested; replacing uv's dependency still requires its adapter changes and upstream integration tests. Browser tree construction, encoding detection, and script execution are outside the conformance target.

## Development

```console
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo bench -p astral-html --bench parse --features benchmark-jemalloc
```

CI tests Linux with the minimum supported Rust version and stable Rust. It also checks formatting, documentation, packaging, and AddressSanitizer fuzz smoke runs. On an Ohm development checkout, use `cargo +ohm` and keep a separate target directory for the checkout and shared build directory for the toolchain.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
