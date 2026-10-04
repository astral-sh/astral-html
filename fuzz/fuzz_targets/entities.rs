#![no_main]

use astral_html::decode;
use html5gum::{Token, Tokenizer};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return;
    };
    // Restrict only characters interpreted as markup or normalized by the
    // tokenizer. The oracle then isolates character-reference decoding.
    if source.contains(['<', '"', '\0', '\r']) {
        return;
    }

    let mut text = Vec::new();
    for token in Tokenizer::new(source) {
        match token.unwrap() {
            Token::String(value) => text.extend_from_slice(&value),
            Token::Error(_) => {}
            token => panic!("unexpected token in text-only oracle: {token:?}"),
        }
    }
    assert_eq!(decode(source, false).as_bytes(), text);

    let markup = format!("<a value=\"{source}\">");
    let tag = Tokenizer::new(markup.as_str())
        .filter_map(|token| match token.unwrap() {
            Token::StartTag(tag) => Some(tag),
            _ => None,
        })
        .next()
        .unwrap();
    let value = tag.attributes.values().next().unwrap();
    assert_eq!(decode(source, true).as_bytes(), value.as_ref());
});
