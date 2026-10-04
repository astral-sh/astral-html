//! Reader-token API contracts beyond the upstream conformance corpus.

use std::borrow::Cow;

use astral_html::{State, Token, Tokenizer};

#[test]
fn owned_null_replacement_preserves_token_normalization() {
    for state in [
        State::Rcdata,
        State::Rawtext,
        State::ScriptData,
        State::Plaintext,
    ] {
        assert_eq!(
            Tokenizer::with_state("é\r\n\0🦀", state, None).next(),
            Some(Token::Text(Cow::Borrowed("é\n�🦀")))
        );
    }
    let Some(Token::StartTag(tag)) = Tokenizer::new("<A\0É B\0🦀=value>").next() else {
        panic!("expected start tag");
    };
    assert_eq!(tag.name, "a�É");
    assert_eq!(tag.attributes[0].name, "b�🦀");
    let Some(Token::Doctype(doctype)) =
        Tokenizer::new("<!DOCTYPE A\0É PUBLIC 'é\r\n\0🦀' 'x\0\r'>").next()
    else {
        panic!("expected doctype");
    };
    assert_eq!(doctype.name.as_deref(), Some("a�É"));
    assert_eq!(doctype.public_id.as_deref(), Some("é\n�🦀"));
    assert_eq!(doctype.system_id.as_deref(), Some("x�\n"));
    assert_eq!(
        Tokenizer::new("<!é\r\0🦀>").next(),
        Some(Token::Comment(Cow::Borrowed("é\n�🦀")))
    );
}

#[test]
fn distinguishes_boolean_and_empty_attributes() {
    let Token::StartTag(tag) = Tokenizer::new("<a boolean empty='' unquoted=value missing=>")
        .next()
        .unwrap()
    else {
        panic!("expected start tag")
    };
    assert_eq!(tag.attributes[0].value(), "");
    assert_eq!(tag.attributes[0].raw_value(), None);
    assert_eq!(tag.attributes[1].raw_value(), Some(""));
    assert_eq!(tag.attributes[2].raw_value(), Some("value"));
    assert_eq!(tag.attributes[3].raw_value(), Some(""));
}

#[test]
fn literal_less_than_runs_stop_at_markup() {
    let prefix = "<<0 &amp;\r\n<é";
    for suffix in [
        "<a href=value>",
        "</a>",
        "<!--comment-->",
        "<!DOCTYPE html>",
        "<?target data>",
        "</0>",
        "</>tail",
        "<?unfinished",
        "</",
        "",
    ] {
        let source = format!("{prefix}{suffix}");
        let mut tokenizer = Tokenizer::new(&source);
        assert_eq!(
            tokenizer.next(),
            Some(Token::Text(Cow::Borrowed("<<0 &\n<é"))),
            "{source:?}"
        );
        assert_eq!(tokenizer.position(), prefix.len());
        assert_eq!(
            tokenizer.collect::<Vec<_>>(),
            Tokenizer::new(suffix).collect::<Vec<_>>(),
            "{source:?}"
        );
    }
    let source = "<".repeat(65_536);
    let mut tokenizer = Tokenizer::new(&source);
    assert!(matches!(tokenizer.next(), Some(Token::Text(Cow::Borrowed(text))) if text == source));
    assert!(tokenizer.next().is_none());
}

#[test]
fn borrows_ordinary_source_fields() {
    let mut tokenizer = Tokenizer::new("<a href='package.whl'>package</a>");
    let Token::StartTag(tag) = tokenizer.next().unwrap() else {
        panic!("expected start tag")
    };
    assert!(matches!(tag.name, Cow::Borrowed("a")));
    assert!(matches!(tag.attributes[0].name, Cow::Borrowed("href")));
    assert!(matches!(
        tag.attributes[0].value,
        Cow::Borrowed("package.whl")
    ));
    assert!(matches!(
        tokenizer.next(),
        Some(Token::Text(Cow::Borrowed("package")))
    ));
}

#[test]
fn comment_newlines_preserve_recovery_and_source_positions() {
    for newline in ["\r", "\r\n", "\n"] {
        for prefix in ["", "-", "x", "<", "<!", "<!-", "<!--", "x-", "--", "--!"] {
            let comment = format!("<!--{prefix}{newline}\0-->");
            let source = format!("{comment}<p>after");
            let mut tokenizer = Tokenizer::new(&source);
            assert_eq!(
                tokenizer.next(),
                Some(Token::Comment(Cow::Owned(format!("{prefix}\n�")))),
                "{source:?}"
            );
            assert_eq!(tokenizer.position(), comment.len());
            assert!(matches!(tokenizer.next(), Some(Token::StartTag(tag)) if tag.name == "p"));
            assert_eq!(tokenizer.next(), Some(Token::Text(Cow::Borrowed("after"))));
            assert_eq!(tokenizer.next(), None);
        }
    }
}

#[test]
fn preserves_first_attributes_across_deduplication_threshold() {
    for count in [0, 1, 7, 8, 9, 64, 1024] {
        let mut source = String::from("<a");
        for index in 0..count {
            source.push_str(&format!(" attr{index}='{index}'"));
        }
        for index in 0..count {
            source.push_str(&format!(" ATTR{index}='duplicate'"));
        }
        source.push('>');
        let Token::StartTag(tag) = Tokenizer::new(&source).next().unwrap() else {
            panic!("expected start tag")
        };
        assert_eq!(tag.attributes.len(), count);
        for (index, attribute) in tag.attributes.iter().enumerate() {
            assert_eq!(attribute.name, format!("attr{index}"));
            assert_eq!(attribute.value, index.to_string());
        }
    }
}

#[test]
fn positions_are_utf8_boundaries_and_exhaustion_is_permanent() {
    for source in [
        "",
        "é<α>𝄞</α>",
        "<a href='é&copy;'>𝄞</a>",
        "<?unfinished",
        "<!--unfinished-",
        "<a attr='unfinished",
        "<script>x</script>",
    ] {
        for state in [
            State::Data,
            State::Rcdata,
            State::Rawtext,
            State::ScriptData,
            State::Plaintext,
            State::Cdata,
        ] {
            let mut tokenizer = Tokenizer::with_state(source, state, Some("script"));
            let mut previous = 0;
            while tokenizer.next().is_some() {
                assert!(tokenizer.position() > previous);
                assert!(source.is_char_boundary(tokenizer.position()));
                previous = tokenizer.position();
            }
            assert_eq!(tokenizer.position(), source.len());
            assert_eq!(tokenizer.next(), None);
        }
    }
}

#[test]
fn caller_can_select_text_context_after_a_start_tag() {
    let mut tokenizer = Tokenizer::new("<title>a &amp; <b></title><p>");
    assert!(matches!(tokenizer.next(), Some(Token::StartTag(_))));
    tokenizer.set_state(State::Rcdata, Some("title"));
    assert_eq!(
        tokenizer.next(),
        Some(Token::Text(Cow::Borrowed("a & <b>")))
    );
    let Some(Token::EndTag(tag)) = tokenizer.next() else {
        panic!("expected title end tag")
    };
    assert_eq!(tag.name, "title");
    let Some(Token::StartTag(tag)) = tokenizer.next() else {
        panic!("expected p start tag")
    };
    assert_eq!(tag.name, "p");
}

#[test]
fn text_modes_only_recognize_ascii_letter_end_tag_names() {
    for state in [State::Rcdata, State::Rawtext, State::ScriptData] {
        for name in ["", "h1", "custom-element", "é", "a/b"] {
            let source = format!("before</{name}>after");
            for mut tokenizer in [Tokenizer::with_state(&source, state, Some(name)), {
                let mut tokenizer = Tokenizer::new(&source);
                tokenizer.set_state(state, Some(name));
                tokenizer
            }] {
                assert_eq!(
                    tokenizer.next(),
                    Some(Token::Text(Cow::Borrowed(&source))),
                    "{state:?}, {name:?}"
                );
                assert_eq!(tokenizer.next(), None);
            }
        }
        let mut tokenizer = Tokenizer::with_state("before</CUSTOM>after", state, Some("custom"));
        assert_eq!(tokenizer.next(), Some(Token::Text(Cow::Borrowed("before"))));
        assert!(matches!(tokenizer.next(), Some(Token::EndTag(tag)) if tag.name == "custom"));
        assert_eq!(tokenizer.next(), Some(Token::Text(Cow::Borrowed("after"))));
    }
}
