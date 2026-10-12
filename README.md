# astral-html

[![Crates.io](https://img.shields.io/crates/v/astral-html.svg)](https://crates.io/crates/astral-html)

A high-performance HTML parser designed for document traversal.

> [!WARNING]
>
> This README was written by a human, but all code changes, PR summaries, and
> additional documentation were authored entirely by GPT-6 Astra in Codex.

## Highlights

- A single runtime dependency: `memchr`.
- No unsafe code, beyond `memchr`.
- Tested against the html5lib tokenizer conformance suite.

## Benchmarks

### Link extraction

Parse HTML, collect owned `href`, `title`, and `rel` attributes from links, and
drop the output (`astral_html::Reader`).

| Parser      | iniconfig | Bootstrap dashboard | Rust Book | PEP 8 | PEP index |
| ----------- | --------: | ------------------: | --------: | ----: | --------: |
| astral-html |       5.7 |                31.2 |       146 |   301 |     1,116 |
| tl          |      14.7 |                52.3 |       219 |   454 |     1,766 |
| html5gum    |      14.9 |                  95 |       394 |   870 |     2,914 |
| lol_html    |      17.0 |                65.3 |       284 |   527 |     2,857 |
| scraper     |      66.8 |                 399 |     1,589 | 3,193 |    11,346 |

<sub>Times in microseconds (µs);
[lower is better](https://github.com/astral-sh/astral-html/blob/7ea072add99590685672de232f2a3637baceeba2/benchmarks/results/README.md).</sub>

### Document construction

Parse HTML into each library's native document and drop it
(`astral_html::Document`).

| Parser      | iniconfig | Bootstrap dashboard | Rust Book | PEP 8 | PEP index |
| ----------- | --------: | ------------------: | --------: | ----: | --------: |
| astral-html |      5.46 |                  35 |       147 |   325 |     1,024 |
| tl          |       8.2 |                  48 |       192 |   394 |     1,124 |
| scraper     |      63.1 |                 378 |     1,570 | 3,090 |     9,958 |

<sub>Times in microseconds (µs);
[lower is better](https://github.com/astral-sh/astral-html/blob/7ea072add99590685672de232f2a3637baceeba2/benchmarks/results/README.md).</sub>

## Example usage

Use `Document` to traverse elements and extract text and attributes, or `Reader`
to process HTML as a stream of tokens.

Do not use their output to approve HTML for rendering. See
[safety and resource use](docs/safety.md) for details.

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
