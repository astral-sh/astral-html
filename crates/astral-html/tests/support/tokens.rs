use std::collections::BTreeMap;

use astral_html::Token;
use serde_json::{Value, json};

/// Convert tokens to the fixture representation, coalescing adjacent text.
pub(crate) fn output<'a>(tokens: impl IntoIterator<Item = Token<'a>>) -> Vec<Value> {
    let mut result = Vec::<Value>::new();
    for token in tokens {
        let value = match token {
            Token::Text(text) => {
                if text.is_empty() {
                    continue;
                }
                if let Some(last) = result.last_mut().filter(|last| last[0] == "Character") {
                    let mut merged = last[1].as_str().unwrap().to_owned();
                    merged.push_str(&text);
                    last[1] = Value::String(merged);
                    continue;
                }
                json!(["Character", text])
            }
            Token::StartTag(tag) => {
                let count = tag.attributes.len();
                let attributes: BTreeMap<_, _> = tag
                    .attributes
                    .into_iter()
                    .map(|attribute| (attribute.name.into_owned(), attribute.value.into_owned()))
                    .collect();
                assert_eq!(attributes.len(), count, "duplicate attribute names");
                if tag.self_closing {
                    json!(["StartTag", tag.name, attributes, true])
                } else {
                    json!(["StartTag", tag.name, attributes])
                }
            }
            Token::EndTag(tag) => json!(["EndTag", tag.name]),
            Token::Comment(comment) => json!(["Comment", comment]),
            Token::Doctype(doctype) => json!([
                "DOCTYPE",
                doctype.name,
                doctype.public_id,
                doctype.system_id,
                !doctype.force_quirks
            ]),
            Token::ProcessingInstruction { target, data } => {
                json!(["ProcessingInstruction", target, data])
            }
        };
        result.push(value);
    }
    result
}
