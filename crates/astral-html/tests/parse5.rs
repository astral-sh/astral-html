//! Whole-page replay against an independent tokenizer, without browser tree construction.

use std::collections::BTreeMap;

use astral_html::{Attribute, Doctype, Document, Error, Limits, Reader, Tag, Token, Tokenizer};

#[path = "support/tokens.rs"]
mod tokens;

/// Convert the oracle's UTF-8 bytes without hiding invalid output behind replacement characters.
fn string(value: html5gum::HtmlString) -> String {
    String::from_utf8(value.into()).expect("UTF-8 fixture must produce UTF-8 tokens")
}

/// Read independent tokens, optionally selecting the reader's HTML text contexts.
fn oracle(source: &str, select_text_states: bool) -> Vec<Token<'static>> {
    // This oracle predates processing instructions. Fail on unsupported fixtures
    // instead of silently skipping any page or changing its contents.
    assert!(!source.contains("<?"), "fixture needs a newer oracle");
    let mut tokenizer = html5gum::Tokenizer::new(source);
    let mut result = Vec::new();
    while let Some(token) = tokenizer.next() {
        let token = match token.unwrap() {
            html5gum::Token::StartTag(tag) => {
                if select_text_states {
                    // html5gum assumes scripting is enabled and spells `noframes`
                    // as `noframe`. Match our documented HTML-only reader policy.
                    let state = match tag.name.as_ref() {
                        b"noscript" | b"noframe" => None,
                        b"noframes" => Some(html5gum::State::RawText),
                        name => html5gum::naive_next_state(name),
                    };
                    if let Some(state) = state {
                        tokenizer.set_state(state);
                    }
                }
                Token::StartTag(Tag {
                    name: string(tag.name).into(),
                    attributes: tag
                        .attributes
                        .into_iter()
                        .map(|(name, value)| Attribute {
                            name: string(name).into(),
                            value: string(value.value).into(),
                            // Raw source spelling is not part of this oracle's output.
                            raw_value: None,
                        })
                        .collect(),
                    self_closing: tag.self_closing,
                })
            }
            html5gum::Token::EndTag(tag) => Token::EndTag(Tag {
                name: string(tag.name).into(),
                attributes: Vec::new(),
                self_closing: false,
            }),
            html5gum::Token::String(text) => Token::Text(string(text.value).into()),
            html5gum::Token::Comment(text) => Token::Comment(string(text.value).into()),
            html5gum::Token::Doctype(doctype) => {
                let doctype = doctype.value;
                Token::Doctype(Doctype {
                    name: (!doctype.name.is_empty()).then(|| string(doctype.name).into()),
                    public_id: doctype.public_identifier.map(|id| string(id).into()),
                    system_id: doctype.system_identifier.map(|id| string(id).into()),
                    force_quirks: doctype.force_quirks,
                })
            }
            html5gum::Token::Error(_) => continue,
        };
        result.push(token);
    }
    result
}

/// Compare one token at a time so failures identify a position without dumping a whole page.
fn assert_tokens(actual: Vec<serde_json::Value>, expected: Vec<serde_json::Value>) {
    assert_eq!(actual.len(), expected.len(), "token count");
    for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(actual, expected, "token {index}");
    }
}

/// Replay the complete page through explicit tokenization, automatic reading, and document views.
fn replay(source: &str, bytes: usize, limits: Limits) {
    assert_eq!(
        source.len(),
        bytes,
        "the pinned page must be present in full"
    );
    assert_tokens(
        tokens::output(Tokenizer::new(source)),
        tokens::output(oracle(source, false)),
    );

    let expected = oracle(source, true);
    let mut reader = Reader::new(source);
    let mut previous = 0;
    let actual = tokens::output(std::iter::from_fn(|| {
        let token = reader.next()?;
        let position = reader.position();
        assert!(position > previous, "reader must make progress");
        assert!(source.is_char_boundary(position));
        previous = position;
        Some(token)
    }));
    assert_eq!(reader.position(), source.len());
    assert!(reader.next().is_none());

    let document =
        Document::parse_with_limits(source, limits).expect("page fits the replay limits");
    let mut elements = document.elements();
    for tag in expected.iter().filter_map(|token| match token {
        Token::StartTag(tag) => Some(tag),
        _ => None,
    }) {
        let element = elements
            .next()
            .expect("document must retain every start tag");
        assert_eq!(element.name(), tag.name);
        assert_eq!(element.attributes().count(), tag.attributes.len());
        let actual: BTreeMap<_, _> = element
            .attributes()
            .map(|attr| (attr.name.as_ref(), attr.value()))
            .collect();
        let expected: BTreeMap<_, _> = tag
            .attributes
            .iter()
            .map(|attr| (attr.name.as_ref(), attr.value()))
            .collect();
        assert_eq!(actual, expected, "attributes of {}", tag.name);
    }
    assert!(
        elements.next().is_none(),
        "document must not invent elements"
    );
    assert_tokens(actual, tokens::output(expected));
}

macro_rules! pages {
    ($($name:ident => ($file:literal, $bytes:literal)),+ $(,)?) => {
        $(#[test]
        fn $name() {
            replay(include_str!(concat!("fixtures/parse5/", $file, ".html")), $bytes, Limits::default());
        })+
    };
}

pages! {
    cern => ("cern", 18_262),
    dx => ("dx", 99_086),
    github_parse5 => ("github-parse5", 45_233),
    whatwg_html => ("whatwg-html", 143_378),
    wiki_42 => ("wiki-42", 222_328),
    lhc => ("lhc", 34_018),
    nodejsorg => ("nodejsorg", 7_639),
    npmorg => ("npmorg", 10_774),
}

#[test]
fn huge_page() {
    let source = include_str!("fixtures/parse5/huge-page.html");
    // Lexical scopes nest much deeper than a browser's repaired tree here.
    assert!(matches!(Document::parse(source), Err(Error::DepthLimit)));
    replay(
        source,
        907_953,
        Limits {
            max_depth: 10_000,
            ..Limits::default()
        },
    );
}
