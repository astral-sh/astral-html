# astral-html

[![Crates.io](https://img.shields.io/crates/v/astral-html.svg)](https://crates.io/crates/astral-html)

A high-performance HTML parser designed for document traversal.

## Benchmarks

Times are in **microseconds (µs)**; lower is better. Each value is the median of
three session means, measured with warm input and the system allocator on a
shared AMD EPYC-Milan VM. See the [methodology and raw results](https://github.com/astral-sh/astral-html/blob/7ea072add99590685672de232f2a3637baceeba2/benchmarks/results/README.md)
for the corpus, dependency versions, compiler, and reproduction instructions.

### Link extraction

Parse HTML, collect owned `href`, `title`, and `rel` attributes from links, and
drop the output. astral-html uses `Reader`.

| Parser | iniconfig | Bootstrap dashboard | Rust Book | PEP 8 | PEP index |
| --- | ---: | ---: | ---: | ---: | ---: |
| astral-html | 5.70 | 31.20 | 146.30 | 301.01 | 1,116.75 |
| tl | 14.75 | 52.31 | 219.28 | 454.03 | 1,766.79 |
| html5gum | 14.99 | 95.35 | 394.40 | 870.56 | 2,914.33 |
| lol_html | 17.08 | 65.32 | 284.96 | 527.90 | 2,857.74 |
| scraper | 66.81 | 399.16 | 1,589.29 | 3,193.92 | 11,346.75 |

### Document construction

Parse HTML into each library's native document and drop it. astral-html uses
`Document`. The representations differ: astral-html retains lexical scopes,
while scraper performs HTML tree construction.

| Parser | iniconfig | Bootstrap dashboard | Rust Book | PEP 8 | PEP index |
| --- | ---: | ---: | ---: | ---: | ---: |
| astral-html | 5.46 | 35.41 | 147.94 | 325.13 | 1,024.94 |
| tl | 8.23 | 48.86 | 192.51 | 394.16 | 1,124.35 |
| scraper | 63.11 | 378.35 | 1,570.03 | 3,090.82 | 9,958.86 |

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
