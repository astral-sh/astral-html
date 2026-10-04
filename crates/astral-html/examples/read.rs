//! Read file links and root-index names without mutable DOM operations.

use astral_html::{Document, Reader, Token};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"<a href="demo-1.0.tar.gz" data-requires-python="&gt;=3.12">demo</a>"#;

    let document = Document::parse(source)?;
    for element in document.elements().filter(|element| element.is("a")) {
        if let Some(href) = element.attribute("href") {
            println!("{}: {}", element.text(), href.value());
        }
    }

    // An event reader avoids retaining a document when only start tags matter.
    for token in Reader::new(source) {
        if let Token::StartTag(tag) = token {
            println!("{}: {} attributes", tag.name, tag.attributes.len());
        }
    }
    Ok(())
}
