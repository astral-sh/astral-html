# astral-html

A read-only HTML parser for package indexes, written in Rust.

astral-html provides a borrowed event reader and immutable document views for uv's package indexes. It implements HTML tokenization and recovery without browser tree construction, mutation, or serialization.

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

`Reader` selects HTML text modes automatically; `Tokenizer` accepts explicit context. `Document::parse_with_limits` bounds input bytes, nodes, nesting, and parsed attributes. Unchanged strings borrow from the input. The application chooses the allocator.

## Validation

- [Conformance](docs/conformance.md): 7,045 supported html5lib tokenizer runs pass; eight excluded cases are documented.
- [uv compatibility](docs/uv.md): all 32 pinned inputs match astral-tl's extracted fields, and all 31 original uv HTML tests pass.
- [Fuzzing](docs/fuzzing.md): five targets cover tokens, entities, reader state, document structure, and limits, including an independent tokenizer oracle.
- [Performance](docs/performance.md): equivalent parse-and-extract benchmarks against astral-tl, including slower cases and source hashes.

See [CI results](docs/ci.md), [resource limits](docs/safety.md), and the [hardening review](docs/hardening.md) for evidence and remaining work.

The library is pre-release. Adoption in uv still requires its broader index and resolver integration tests.

## Development

```console
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo bench -p astral-html --bench parse --features benchmark-jemalloc
```

CI uses Namespace's Ubuntu 24.04 runners and tests AMD64 and ARM64 with the minimum supported Rust version and stable Rust. It also checks formatting, documentation, packaging, and AddressSanitizer fuzz smoke runs. On an Ohm development checkout, use `cargo +ohm` and keep a separate target directory for the checkout and shared build directory for the toolchain.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
