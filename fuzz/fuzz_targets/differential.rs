#![no_main]

use astral_html::{State, Token, Tokenizer};
use html5gum::{DefaultEmitter, Emitter};
use libfuzzer_sys::fuzz_target;

#[derive(Debug, PartialEq, Eq)]
enum Canonical {
    Start(String, Vec<(String, String)>, bool),
    End(String),
    Text(String),
    Comment(String),
    Doctype(String, Option<String>, Option<String>, bool),
}

/// Token boundaries in adjacent character data are not observable HTML semantics.
fn push(tokens: &mut Vec<Canonical>, token: Canonical) {
    if let Canonical::Text(text) = &token {
        if text.is_empty() {
            return;
        }
        if let Some(Canonical::Text(previous)) = tokens.last_mut() {
            previous.push_str(text);
            return;
        }
    }
    tokens.push(token);
}

fn own(source: &str, state: State, last_start_tag: Option<&str>) -> Vec<Canonical> {
    let mut tokens = Vec::new();
    for token in Tokenizer::with_state(source, state, last_start_tag) {
        let token = match token {
            Token::StartTag(tag) => {
                let mut attributes = tag
                    .attributes
                    .into_iter()
                    .map(|attribute| (attribute.name.into_owned(), attribute.value.into_owned()))
                    .collect::<Vec<_>>();
                attributes.sort_unstable();
                Canonical::Start(tag.name.into_owned(), attributes, tag.self_closing)
            }
            Token::EndTag(tag) => Canonical::End(tag.name.into_owned()),
            Token::Text(text) => Canonical::Text(text.into_owned()),
            Token::Comment(text) => Canonical::Comment(text.into_owned()),
            Token::Doctype(doctype) => Canonical::Doctype(
                doctype
                    .name
                    .map(|name| name.into_owned())
                    .unwrap_or_default(),
                doctype.public_id.map(|id| id.into_owned()),
                doctype.system_id.map(|id| id.into_owned()),
                doctype.force_quirks,
            ),
            Token::ProcessingInstruction { .. } => {
                unreachable!("processing instructions are excluded before comparison")
            }
        };
        push(&mut tokens, token);
    }
    tokens
}

fn oracle(source: &str, state: html5gum::State, last_start_tag: Option<&str>) -> Vec<Canonical> {
    let mut emitter = DefaultEmitter::default();
    emitter.set_last_start_tag(last_start_tag.map(str::as_bytes));
    let mut tokenizer = html5gum::Tokenizer::new_with_emitter(source, emitter);
    tokenizer.set_state(state);
    let mut tokens = Vec::new();
    for token in tokenizer {
        let token = match token.unwrap() {
            html5gum::Token::StartTag(tag) => Canonical::Start(
                String::from_utf8(tag.name.to_vec()).unwrap(),
                tag.attributes
                    .into_iter()
                    .map(|(name, value)| {
                        (
                            String::from_utf8(name.to_vec()).unwrap(),
                            String::from_utf8(value.to_vec()).unwrap(),
                        )
                    })
                    .collect(),
                tag.self_closing,
            ),
            html5gum::Token::EndTag(tag) => {
                Canonical::End(String::from_utf8(tag.name.to_vec()).unwrap())
            }
            html5gum::Token::String(text) => {
                Canonical::Text(String::from_utf8(text.to_vec()).unwrap())
            }
            html5gum::Token::Comment(text) => {
                Canonical::Comment(String::from_utf8(text.to_vec()).unwrap())
            }
            html5gum::Token::Doctype(doctype) => Canonical::Doctype(
                String::from_utf8(doctype.name.to_vec()).unwrap(),
                doctype
                    .public_identifier
                    .as_ref()
                    .map(|id| String::from_utf8(id.to_vec()).unwrap()),
                doctype
                    .system_identifier
                    .as_ref()
                    .map(|id| String::from_utf8(id.to_vec()).unwrap()),
                doctype.force_quirks,
            ),
            html5gum::Token::Error(_) => continue,
        };
        push(&mut tokens, token);
    }
    tokens
}

fuzz_target!(|bytes: &[u8]| {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return;
    };
    // html5gum 0.8.4 predates WHATWG processing instructions and treats them as
    // bogus comments. The pinned html5lib corpus covers our newer behavior.
    if source.contains("<?") {
        return;
    }
    for (state, reference_state, last_start_tag) in [
        (State::Data, html5gum::State::Data, None),
        (State::Rcdata, html5gum::State::RcData, Some("title")),
        (State::Rawtext, html5gum::State::RawText, Some("style")),
        (
            State::ScriptData,
            html5gum::State::ScriptData,
            Some("script"),
        ),
        (State::Plaintext, html5gum::State::PlainText, None),
        (State::Cdata, html5gum::State::CdataSection, None),
    ] {
        assert_eq!(
            own(source, state, last_start_tag),
            oracle(source, reference_state, last_start_tag),
            "state: {state:?}"
        );
    }
});
