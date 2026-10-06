//! Tokenizer-level adaptations of selected, pinned Web Platform Tests.
//!
//! See `fixtures/wpt/README.md` for provenance and the browser behavior outside
//! these tests' scope.

use std::borrow::Cow;
use std::collections::BTreeMap;

use astral_html::{Token, Tokenizer};

fn text(source: &str) -> String {
    let mut result = String::new();
    for token in Tokenizer::new(source) {
        let Token::Text(value) = token else {
            panic!("expected text for {source:?}, got {token:?}");
        };
        result.push_str(&value);
    }
    result
}

fn only_token(source: &str) -> Token<'_> {
    let mut tokenizer = Tokenizer::new(source);
    let token = tokenizer.next().expect("expected a token");
    assert_eq!(tokenizer.next(), None, "{source:?}");
    token
}

#[test]
fn named_character_references() {
    let references: BTreeMap<String, String> =
        serde_json::from_str(include_str!("fixtures/wpt/named-character-references.json")).unwrap();
    assert_eq!(
        references.len(),
        2231,
        "the complete WPT map must be present"
    );
    for (source, expected) in references {
        assert_eq!(text(&source), expected, "{source:?}");
    }
}

#[test]
fn ambiguous_ampersand_in_text_and_attribute() {
    let source = "?a=b&c=d&a0b=c&copy=1&noti=n&not=in&notin=&notin;&not;&;& &";
    assert_eq!(
        text(&format!("Text: {source}")),
        "Text: ?a=b&c=d&a0b=c©=1¬i=n¬=in¬in=∉¬&;& &"
    );

    let markup = format!("<a href='{source}'>");
    let Token::StartTag(tag) = only_token(&markup) else {
        panic!("expected an anchor");
    };
    assert_eq!(tag.name, "a");
    assert_eq!(tag.attributes.len(), 1);
    assert_eq!(tag.attributes[0].name, "href");
    // WPT reads a.href, which additionally URL-encodes the Unicode and space.
    assert_eq!(
        tag.attributes[0].value,
        "?a=b&c=d&a0b=c&copy=1&noti=n&not=in&notin=∉¬&;& &"
    );
}

#[test]
fn newline_normalization_in_text_and_quoted_attributes() {
    let mut runs = 0;
    for (source, expected) in [
        ("before\rABCDEF\nafter", "before\nABCDEF\nafter"),
        ("before\r\nafter", "before\nafter"),
    ] {
        assert_eq!(text(source), expected, "{source:?}");
        runs += 1;
        for quote in ['\'', '"'] {
            let markup = format!("<input value={quote}{source}{quote}>");
            let Token::StartTag(tag) = only_token(&markup) else {
                panic!("expected an input for {markup:?}");
            };
            assert_eq!(tag.name, "input");
            assert_eq!(tag.attributes.len(), 1);
            assert_eq!(tag.attributes[0].name, "value");
            assert_eq!(tag.attributes[0].value, expected, "{markup:?}");
            runs += 1;
        }
    }
    assert_eq!(runs, 6);
}

#[test]
fn doctype_identifiers_preserve_missing_empty_and_nonempty_values() {
    let cases = [
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Frameset//\" \"\">",
            "-//W3C//DTD HTML 4.01 Frameset//",
            Some(""),
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Frameset//\">",
            "-//W3C//DTD HTML 4.01 Frameset//",
            None,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Frameset//\" \"http://www.w3.org/TR/html4/frameset.dtd\">",
            "-//W3C//DTD HTML 4.01 Frameset//",
            Some("http://www.w3.org/TR/html4/frameset.dtd"),
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//\" \"\">",
            "-//W3C//DTD HTML 4.01 Transitional//",
            Some(""),
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//\">",
            "-//W3C//DTD HTML 4.01 Transitional//",
            None,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//\" \"http://www.w3.org/TR/html4/loose.dtd\">",
            "-//W3C//DTD HTML 4.01 Transitional//",
            Some("http://www.w3.org/TR/html4/loose.dtd"),
        ),
        ("<!doctype html PUBLIC \"\" \"\">", "", Some("")),
    ];
    assert_eq!(cases.len(), 7);
    for (source, public_id, system_id) in cases {
        let Token::Doctype(doctype) = only_token(source) else {
            panic!("expected a doctype for {source:?}");
        };
        assert_eq!(doctype.name.as_deref(), Some("html"), "{source:?}");
        assert_eq!(doctype.public_id.as_deref(), Some(public_id), "{source:?}");
        assert_eq!(doctype.system_id.as_deref(), system_id, "{source:?}");
        // The tokenizer flag is separate from the tree builder's quirks mode.
        assert!(!doctype.force_quirks, "{source:?}");
    }
}

#[test]
fn absent_doctype_name_stays_absent_in_the_token() {
    let cases = ["<!doctype>", "<!DOCTYPE >", "<!DOCTYPE\n>", "<!DOCTYPE"];
    assert_eq!(cases.len(), 4);
    for source in cases {
        let Token::Doctype(doctype) = only_token(source) else {
            panic!("expected a doctype for {source:?}");
        };
        assert_eq!(doctype.name, None, "{source:?}");
        assert_eq!(doctype.public_id, None, "{source:?}");
        assert_eq!(doctype.system_id, None, "{source:?}");
        assert!(doctype.force_quirks, "{source:?}");
    }
}

#[test]
fn truncated_markup_declarations() {
    assert_eq!(
        Tokenizer::new("Abc<!d>Hi").collect::<Vec<_>>(),
        [
            Token::Text(Cow::Borrowed("Abc")),
            Token::Comment(Cow::Borrowed("d")),
            Token::Text(Cow::Borrowed("Hi")),
        ]
    );
    let Token::Doctype(doctype) = only_token("<!DOCTYPE html pu") else {
        panic!("expected a doctype");
    };
    assert_eq!(doctype.name.as_deref(), Some("html"));
    assert_eq!(doctype.public_id, None);
    assert_eq!(doctype.system_id, None);
    assert!(doctype.force_quirks);
}
