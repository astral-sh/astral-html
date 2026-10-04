//! Reader-token API contracts beyond the upstream conformance corpus.

use std::borrow::Cow;

use astral_html::{State, Token, Tokenizer};

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
        Some(Token::Text(Cow::Borrowed("a & <b>"))).map(|token| match token {
            Token::Text(value) => Token::Text(Cow::Owned(value.into_owned())),
            other => other,
        })
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
