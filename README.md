# astral-html

An HTML parser written in Rust.

astral-html provides a borrowed event reader and immutable document views. It implements HTML tokenization and recovery without browser tree construction, mutation, or serialization.

```rust
use astral_html::Document;

let source = r#"<a href="/guide">Read the guide</a>"#;
let document = Document::parse(source)?;

for element in document.elements().filter(|element| element.is("a")) {
    if let Some(href) = element.attribute("href") {
        println!("{}: {}", element.text(), href.value());
    }
}
```

`Reader` selects HTML text modes automatically; `Tokenizer` accepts explicit context. `Document::parse_with_limits` bounds input bytes, nodes, nesting, and parsed attributes. Unchanged strings borrow from the input. The application chooses the allocator.

- [Conformance](docs/conformance.md): tokenizer and document semantics.
- [Resource limits](docs/safety.md): allocation and input bounds.
- [uv integration](docs/uv.md): compatibility tests and adapter.
- [Fuzzing](docs/fuzzing.md): targets, oracles, and commands.
- [Benchmarks](docs/performance.md): equivalent parsing and extraction against astral-tl.

The library is pre-release. Adoption in uv still requires its broader index and resolver integration tests.

## Development

```console
cargo test --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo bench -p astral-html --bench parse --features benchmark-jemalloc --locked
```

CI uses Namespace's Ubuntu 24.04 runners to test AMD64 and ARM64 with the minimum supported Rust version and stable Rust. It also checks formatting, documentation, packaging, uv compatibility, and AddressSanitizer fuzz targets.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
