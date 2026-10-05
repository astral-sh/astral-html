# astral-html

[![Crates.io](https://img.shields.io/crates/v/astral-html.svg)](https://crates.io/crates/astral-html)

A high-performance HTML parser designed for document traversal.

<p align="center">
  <picture align="center">
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/extraction-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="docs/assets/extraction-light.svg">
    <img alt="Link extraction from the PEP index: astral-html (Reader) 1.12 ms, tl 1.77 ms, lol_html 2.86 ms, html5gum 2.91 ms, and scraper 11.35 ms. Lower is better." src="docs/assets/extraction-light.svg">
  </picture>
</p>

<p align="center">
  <i>Extracting 2,000 anchor links from a Simple API-compatible index</i>
</p>

> [!WARNING]
>
> This README was written by a human, but all code changes, PR summaries, and
> additional documentation were authored entirely by GPT-6 Astra in Codex.

## Example usage

Use `Document` to traverse elements and extract text and attributes, or `Reader`
to process HTML as a stream of tokens.

For example, to extract links from a document:

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

To extract link URLs without building a document, use `Reader`:

```rust
use astral_html::{Reader, Token};

let source = r#"<a href="/guide">Read the guide</a>"#;

for token in Reader::new(source) {
    if let Token::StartTag(tag) = token
        && tag.name == "a"
        && let Some(href) = tag.attributes.iter().find(|attr| attr.name == "href")
    {
        println!("{}", href.value());
    }
}
```

## License

astral-html is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in astral-html by you, as defined in the Apache-2.0 license, shall
be dually licensed as above, without any additional terms or conditions.

<div align="center">
  <a target="_blank" href="https://astral.sh" style="background:none">
    <img src="https://raw.githubusercontent.com/astral-sh/uv/main/assets/svg/Astral.svg" alt="Made by Astral">
  </a>
</div>
